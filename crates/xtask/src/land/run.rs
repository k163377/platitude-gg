//! The landing itself: what stands ahead of the turn in the landings'
//! queue, the turn, and everything past it up to the fast-forward and
//! the clearing after (the module's doc says why this is the one way).

use crate::seats::{
    Held, Identity, Standing, WorktreeBlock, is_a_seat_claim, same_tree, standing, whose,
    worktree_blocks,
};
use crate::subprocess::git_query;

use super::record::Phases;
use super::{approval, hold, leftovers, line, tags};

pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("line" | "wait" | "leave") => return line::run(args),
        Some("hold") => return hold::run(args),
        _ => {}
    }
    let mut phases = Phases::start();
    let result = land(args, &mut phases);
    phases.finish(&result);
    result
}

/// What the call names: the tree it is run against (`--dir`, else this
/// runner's own) and the branch, when one is named.
fn named(args: &[String]) -> Result<(std::path::PathBuf, Option<String>), String> {
    let mut root = crate::tree::workspace_root();
    let mut branch: Option<String> = None;
    let mut at = 0;
    while let Some(arg) = args.get(at) {
        match arg.as_str() {
            "--dir" => {
                at += 1;
                root = std::path::PathBuf::from(args.get(at).ok_or("--dir needs a path")?);
            }
            name if branch.is_none() && !name.starts_with('-') => branch = Some(name.to_string()),
            other => return Err(format!("land takes one branch at most (got {other:?})")),
        }
        at += 1;
    }
    Ok((root, branch))
}

/// A landing that stops once it has taken its turn — a rebase that
/// stopped, a red gate, runs awaiting approval, a closed permit, a refused
/// fast-forward — leaves the line of landings: what stopped it is the
/// user's to settle, and their next `/land` puts the seat back at the end
/// (`line::leave`) — unless the user's latest `/land` came after this
/// landing began: the place is that `/land`'s. A refusal ahead of the turn
/// — a dirty seat, a gate running in it, a branch checked out nowhere — is
/// the session's to put right and land again; a branch with nothing to land
/// ends its place too (`preflight`).
fn land(args: &[String], phases: &mut Phases) -> Result<(), String> {
    let (root, branch) = named(args)?;
    phases.target(&root, "");
    let here = root.to_string_lossy().replace('\\', "/");
    let me = Identity::current(None);
    let Some(ready) = preflight(&root, &here, branch, &me, phases)? else {
        return Ok(());
    };
    through(&root, &here, &ready, &me, phases).map_err(|why| {
        // A place the user's latest `/land` holds — behind another head, or
        // asked after this landing began — is that `/land`'s, not the
        // stopped landing's to give up.
        if !line::stands_behind(&here, &me)
            && said_since(&here, &me, ready.began.as_ref()).is_none()
            && line::leave(&here, &me.session, &why)
        {
            format!(
                "{why}\n— this seat left the line of landings; the user's next {} puts it back \
                 at the end.",
                crate::hook::permit::COMMAND
            )
        } else {
            why
        }
    })
}

/// A landing that has taken its turn: the branch, the trees as listed
/// (the primary first), which of them has the branch out, the user's
/// message it runs on, and the turn itself — held until the landing is
/// over.
struct Ready {
    branch: String,
    listing: String,
    trees: Vec<WorktreeBlock>,
    seat_at: usize,
    ahead: u32,
    began: Option<crate::hook::permit::Asked>,
    _turn: crate::budget::Admitted,
}

/// Everything ahead of the turn. A refusal here — a dirty seat, a gate
/// running in it, a branch checked out nowhere, a seat behind the head —
/// is the session's to put right and land again. None is a branch with
/// nothing to land: its place in the line ends there, with the permit
/// unspent.
fn preflight(
    root: &std::path::Path,
    here: &str,
    branch: Option<String>,
    me: &Identity,
    phases: &mut Phases,
) -> Result<Option<Ready>, String> {
    let branch = match branch {
        Some(name) => name,
        None => current_branch(here)?,
    };
    phases.target(root, &branch);
    if branch == "main" {
        return Err("land moves a branch onto main; main itself is not one".into());
    }
    git_query(
        here,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ],
    )
    .ok_or_else(|| format!("no local branch named {branch:?}"))?;
    let ahead = crate::seats::commits_in(here, &format!("main..{branch}"))
        .ok_or("git could not count main..branch — is main a local branch here?")?;
    if ahead == 0 {
        line::leave(here, &me.session, "nothing to land");
        release_already_landed(here, &branch, me)?;
        return Ok(None);
    }
    let listing =
        git_query(here, &["worktree", "list", "--porcelain"]).ok_or("git worktree list failed")?;
    let trees = worktree_blocks(&listing);
    let Some(primary) = trees.first() else {
        return Err("git worktree list answered with no trees at all".into());
    };
    if let Some(tree) = trees.iter().find(|tree| tree.branch == "main")
        && tree.path != primary.path
    {
        // A seat sitting on main would receive the merge into its own
        // working tree.
        return Err(format!(
            "main is checked out in {} — \
             free it (switch that tree to another branch) and land again",
            tree.path
        ));
    }
    let Some(seat_at) = trees.iter().position(|tree| tree.branch == branch) else {
        return Err(format!(
            "{branch} is checked out nowhere — the gate runs in the branch's own worktree. \
             Check it out in a seat and land again."
        ));
    };
    let seat_dir = std::path::Path::new(&trees[seat_at].path);
    phases.target(seat_dir, &branch);
    crate::hook::codex_land::sync(here)?;
    // The message this landing runs on; one after it says more for it
    // (`still_the_ask`).
    let began = crate::hook::permit::asked(here, &me.session);
    // A landing that waits moves nothing.
    approval::awaited(here, &listing, &branch)?;
    // The line of landings: only its head lands (`line`).
    line::must_be_head(here, me)?;
    settle_the_tree(&trees[seat_at].path)?;
    no_gate_running(&trees[seat_at].path)?;
    // The landings' queue (`budget::Pool::turn`), held to the end. Taken
    // before the seat's tree, its gate and the machine's budget, so a
    // landing standing in line stands in nobody's way.
    let pool = crate::budget::Pool::of(root, crate::budget::default_jobs())?;
    phases.mark("preflight");
    let turn = pool.turn(&crate::seats::slashed(seat_dir), &format!("land {branch}"));
    phases.mark("queue");
    let turn = turn?;
    // Again with the turn: a gate started in the seat while the landing
    // waited for it.
    no_gate_running(&trees[seat_at].path)?;
    Ok(Some(Ready {
        branch,
        listing,
        trees,
        seat_at,
        ahead,
        began,
        _turn: turn,
    }))
}

/// From the turn on. What stops here leaves the line of landings
/// (`land`): what stopped it is the user's to settle.
fn through(
    root: &std::path::Path,
    here: &str,
    ready: &Ready,
    me: &Identity,
    phases: &mut Phases,
) -> Result<(), String> {
    let Ready {
        branch,
        listing,
        trees,
        seat_at,
        ahead,
        began,
        ..
    } = ready;
    let (seat, primary) = (&trees[*seat_at], &trees[0]);
    let seat_dir = std::path::Path::new(&seat.path);
    still_the_ask(here, me, began.as_ref())?;
    prepare_gate(root, &seat.path)?;
    phases.mark("prepare");
    // What the branch held before any rebase: a copy taken from it before
    // its history was rewritten holds nothing this does not (`leftovers`).
    let unrebased = git_query(here, &["rev-parse", &format!("refs/heads/{branch}")]);
    if git_query(here, &["merge-base", "--is-ancestor", "main", branch]).is_none() {
        println!("{branch} is behind main — rebasing it in {}", seat.path);
        rebase(&seat.path)?;
    }
    phases.mark("rebase");
    gate_in_the_seat(seat_dir)?;
    phases.mark("gate");
    // Again, for a run that went up while the gate ran (a verify-ui left in
    // the background): once main moves, the board is cleared of it unseen.
    approval::awaited(here, listing, branch)?;
    // And the permit: a landing in the background outlives the user's next
    // message — nothing moves on a closed permit, nor on one a later
    // `/land` said more on.
    still_the_ask(here, me, began.as_ref())?;
    // And the place: a landing in the background outlives it. A message
    // and a `/land` after it put the seat at the end behind a head already
    // preparing, and a landing that went red and left the line may be run
    // again (`line::must_be_head`, as at the turn).
    line::must_be_head(here, me)?;
    let ahead = crate::seats::commits_in(here, &format!("main..{branch}")).unwrap_or(*ahead);
    let before = git_query(here, &["rev-parse", "--short", "main"]).unwrap_or_default();
    if primary.branch == "main" {
        forward_in(&primary.path, branch)?;
    } else {
        forward_ref(here, primary, branch)?;
    }
    phases.mark("fast-forward (verdict included)");
    let after = git_query(here, &["rev-parse", "--short", "main"]).unwrap_or_default();
    // The permit is spent only here: a landing that stopped before this
    // line moved nothing. Only the one this landing began on — a `/land`
    // after its last look keeps its own.
    crate::hook::permit::landed(here, &me.session, began.as_ref());
    if let Some(more) = said_since(here, me, began.as_ref()) {
        println!(
            "the user said more after this landing's last look: {} — main moved without it; its \
             permit stays open and the seat in the line, for the landing that takes it in.",
            more.said()
        );
    }
    // Again, now that main moved: git runs a copy of the hook script, and
    // a landing that changed the script would leave the old copy answering.
    println!("{}", crate::gate::hooks::install(root)?);
    println!("landed {branch}: main {before} -> {after} ({ahead} commit(s)).");
    if tags::bracket(here, &before, &after, ahead) {
        phases.mark("version tags");
    }
    // While the claim still stands, so nothing else is building here: the
    // rebase may have left a generation of build products behind.
    crate::sweep::at_a_tail(seat_dir, &crate::sweep::Tail::after_a_landing());
    phases.mark("sweep");
    // Still in the landings' queue, so no other landing sweeps beside it.
    leftovers::clear(here, unrebased.as_deref(), &seat.path);
    phases.mark("leftovers");
    // The claim comes off last: once it does, another session may take the
    // letter, and clearing the board after that would clear its runs.
    clear_the_board(listing, branch);
    release_claim(here, trees, branch, me);
    phases.mark("hook, board and claim");
    Ok(())
}

/// The permit read again: a landing that runs in the background outlives
/// the user's next message. One that is not `/land` closes the permit. A
/// `/land` after the one this landing `began` on says more — the landing
/// stops short of main, so that the session takes it in and lands again,
/// and the seat stays in the line where that `/land` put it: at its place
/// when the seat stood there as it was said, at the end after any other
/// message (`line::enter`). A session with no permit on record — a hand
/// run — is not asked.
fn still_the_ask(
    here: &str,
    me: &Identity,
    began: Option<&crate::hook::permit::Asked>,
) -> Result<(), String> {
    if std::env::var("CODEX_THREAD_ID").ok().as_deref() == Some(&me.session) {
        crate::hook::codex_land::sync(here)?;
    }
    if me.session.is_empty() {
        return Ok(());
    }
    let Some(asked) = crate::hook::permit::asked(here, &me.session) else {
        return Ok(());
    };
    if !crate::hook::permit::is_open(here, &me.session) {
        return Err(format!(
            "the permit is no longer open, so this landing moves nothing: the user's latest \
             message is {}, and main moves once on their own {} — their next {} lands the branch",
            asked.said(),
            crate::hook::permit::COMMAND,
            crate::hook::permit::COMMAND
        ));
    }
    if let Some(more) = said_since(here, me, began) {
        return Err(format!(
            "the user said more since this landing began: {} — nothing moved, and the seat stays \
             in the line where that /land put it. Take that in, and land again.",
            more.said()
        ));
    }
    Ok(())
}

/// The user's `/land` after the one this landing `began` on, while the
/// permit stands open: more for the same landing.
fn said_since(
    here: &str,
    me: &Identity,
    began: Option<&crate::hook::permit::Asked>,
) -> Option<crate::hook::permit::Asked> {
    let began = began?;
    let asked = crate::hook::permit::asked(here, &me.session)?;
    (crate::hook::permit::is_open(here, &me.session) && !began.is(&asked)).then_some(asked)
}

/// What would land is what stands committed, so the seat must be clean:
/// no run writes the tree, so anything standing is somebody's.
fn settle_the_tree(seat: &str) -> Result<(), String> {
    let dirty = git_query(seat, &["status", "--porcelain"]).unwrap_or_default();
    if dirty.is_empty() {
        return Ok(());
    }
    Err(format!(
        "{seat} has uncommitted changes — what would land is not what is there. Commit or \
         stash first:\n{dirty}"
    ))
}

/// A gate still running in the seat — one that went to the background and
/// looked finished — starts the task runner from the seat's build slot by
/// name and reads its tree: stepping out of the slot and rebasing under it
/// would turn it red for nothing. A lock that cannot be looked at moves
/// nothing either.
fn no_gate_running(seat: &str) -> Result<(), String> {
    match crate::gate::running_in(std::path::Path::new(seat)) {
        Ok(false) => Ok(()),
        Ok(true) => Err(format!(
            "a gate is running in {seat} — landing now would step out of the build slot its \
             tests start the task runner from, and rebase the tree it reads. Wait for its \
             verdict (the output of the command that started it) and land again."
        )),
        Err(why) => Err(format!(
            "{why} — whether a gate runs in {seat} is not known, so nothing moves"
        )),
    }
}

/// Free only the seat's build slot: the primary's verdict runs from
/// `target/hooks`. Install the main-ref guard before rebase and gate.
fn prepare_gate(root: &std::path::Path, seat: &str) -> Result<(), String> {
    if let Err(why) = crate::gate::preserve_reader() {
        eprintln!("graph cache: {why}");
    }
    if let Some(word) = step_out_of_the_build_slot(&[seat]) {
        println!("{word}");
    }
    println!("{}", crate::gate::hooks::install(root)?);
    Ok(())
}

/// A duration to the second, the way the gate says the wall clock a
/// landing's phases are compared against.
pub(super) fn clock(took: std::time::Duration) -> String {
    let secs = took.as_secs();
    format!("{}m{:02}s", secs / 60, secs % 60)
}

/// What a landing leaves in a build slot it stepped out of and could not
/// delete: its own running image.
const INFLIGHT: &str = "xtask-inflight-";

/// Where cargo writes the binary the slot is linked to: this package's
/// units, `build/xtask/<hash>/`, each with its `out/`.
const UNITS: [&str; 2] = ["build", "xtask"];

/// Frees the cargo build slot this process occupies, when the slot is one
/// of `trees`'.
///
/// The gate rebuilds the seat's task runner, and this process is
/// `<tree>/target/debug/xtask`: on Windows a running image cannot be
/// replaced (`os error 5`) but can be renamed, so this process moves out
/// of the name. Out of every name ([`names_of_this_image`]): cargo
/// hard-links the slot to the `xtask` in its unit's `out/`, and freeing
/// one name alone stops the gate's link step at the other (`LNK1104`),
/// after the rebase.
///
/// What cannot be moved waits for the next landing's sweep. Silent when
/// this binary is in nobody's way; a move that fails says so. The hold
/// (`hold`) steps out the same way, once no gate of its tree runs.
pub(super) fn step_out_of_the_build_slot(trees: &[&str]) -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let mine = build_slot_tree(&exe)?;
    if !trees.iter().any(|tree| same_tree(tree, &mine)) {
        return None;
    }
    let dir = exe.parent()?;
    let outs = unit_outs(dir);
    sweep(dir);
    for out in &outs {
        sweep(out);
    }
    let mut stood = Vec::new();
    let mut freed = Vec::new();
    for (at, name) in names_of_this_image(&exe, &outs).iter().enumerate() {
        match step_aside(name, at) {
            Ok(()) => freed.push(name.display().to_string()),
            Err(why) => stood.push(format!("{} ({why})", name.display())),
        }
    }
    if !stood.is_empty() {
        return Some(format!(
            "note: this task runner is still standing in {} — a step that has to rebuild it there \
             will stop at the running image.",
            stood.join(", ")
        ));
    }
    Some(format!(
        "stepped out of {} — the gate and the landing rebuild the task runner there, and a \
         running image cannot be replaced.",
        freed.join(", ")
    ))
}

/// The `out/` of every unit of this package in the profile directory
/// `dir`.
fn unit_outs(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let Ok(units) = std::fs::read_dir(dir.join(UNITS[0]).join(UNITS[1])) else {
        return Vec::new();
    };
    units
        .flatten()
        .map(|unit| unit.path().join("out"))
        .filter(|out| out.is_dir())
        .collect()
}

/// Every name in this tree's build directory that is this binary's image:
/// the slot the run was started from, and the name in a unit's `outs` the
/// slot is a hard link to.
///
/// Chosen by what the file is, not by what cargo calls it (user
/// decision): a name in a unit's `out/` is this image when it is the same
/// file as the slot ([`crate::sweep::same_file`]). So the images that are
/// not this one stay where cargo put them — another unit's build of this
/// binary, and the `--bin xtask` unit-test harness in a unit of its own.
fn names_of_this_image(
    exe: &std::path::Path,
    outs: &[std::path::PathBuf],
) -> Vec<std::path::PathBuf> {
    let mut names = vec![exe.to_path_buf()];
    for out in outs {
        let Ok(entries) = std::fs::read_dir(out) else {
            continue;
        };
        for entry in entries.flatten() {
            if crate::sweep::same_file(&entry.path(), exe) {
                names.push(entry.path());
            }
        }
    }
    names
}

/// Moves one name off the image. `at` keeps the asides of one image apart.
fn step_aside(name: &std::path::Path, at: usize) -> Result<(), std::io::Error> {
    let dir = name.parent().unwrap_or(name);
    let mut aside = dir.join(format!("{INFLIGHT}{}-{at}", std::process::id()));
    if let Some(extension) = name.extension() {
        aside.set_extension(extension);
    }
    std::fs::rename(name, &aside)?;
    let _ = std::fs::remove_file(&aside);
    Ok(())
}

/// Takes away what earlier landings left in a slot's directory —
/// whichever of them the system will part with now.
fn sweep(dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().starts_with(INFLIGHT) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// The checkout whose build slot `exe` is, when it is one:
/// `<tree>/target/<profile>/xtask[.exe]` is what cargo writes and runs.
fn build_slot_tree(exe: &std::path::Path) -> Option<String> {
    if exe.file_stem()? != "xtask" {
        return None;
    }
    let target = exe.parent()?.parent()?;
    if target.file_name()? != "target" {
        return None;
    }
    Some(target.parent()?.to_string_lossy().replace('\\', "/"))
}

/// The gate in the seat: green is the commit stamped, since no step
/// writes the tree.
fn gate_in_the_seat(seat: &std::path::Path) -> Result<(), String> {
    crate::gate::for_landing(seat, "main")
}

/// A rebase that stops is walked back and reported: resolving conflicts
/// unattended is nobody's instruction.
fn rebase(seat: &str) -> Result<(), String> {
    let mut command = std::process::Command::new("git");
    command
        .arg("-C")
        .arg(seat)
        .env("GIT_EDITOR", "true")
        .env("GIT_SEQUENCE_EDITOR", "true")
        .args(["rebase", "main"]);
    let output = crate::subprocess::run_captured(&mut command)?;
    if output.status.success() {
        return Ok(());
    }
    let mut abort = std::process::Command::new("git");
    abort.arg("-C").arg(seat).args(["rebase", "--abort"]);
    let walked_back =
        crate::subprocess::run_captured(&mut abort).is_ok_and(|out| out.status.success());
    Err(format!(
        "the rebase onto main stopped and was {}:\n{}{}\n\
         resolve it with the user — a conflict is nobody's to settle unattended",
        if walked_back {
            "walked back"
        } else {
            "left standing (the abort failed too — the seat is mid-rebase)"
        },
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

/// The seat's pictures go with its claim: their work is on main, and
/// spent pictures make the next session hunt for the current one. Only
/// this seat's — the others belong to seats still working. A sweep that
/// cannot happen is only said: the board is not what a land turns on.
fn clear_the_board(listing: &str, branch: &str) {
    let Some(seat) = crate::seats::seat_entries(listing)
        .into_iter()
        .find(|entry| entry.tree.branch == branch)
        .map(|entry| entry.seat)
    else {
        return;
    };
    match crate::shots::seat_freed(seat) {
        Ok((0, _)) => {}
        Ok((gone, page)) => println!(
            "board: took seat {seat}'s {gone} run(s) off {}",
            crate::shots::shown(&page)
        ),
        Err(message) => println!("board: seat {seat}'s runs were left on it ({message})"),
    }
}

/// The claim's release point (CLAUDE.md ビルド・テスト): the letter goes
/// back to the roster. Only the hooks' own kind of lock is lifted; a lock
/// a person wrote stays.
///
/// Whoever's claim it is comes off: a landing is the user's word that the
/// work is done, and a claim whose session is gone would never be handed
/// back (`seats::claim`). A session going on here takes the seat back at
/// its next edit; a picture takes nothing (`seats::may_picture`).
fn release_claim(here: &str, trees: &[WorktreeBlock], branch: &str, me: &Identity) {
    let Some(tree) = landed_claim(trees, branch) else {
        return;
    };
    let was = match standing(Some(tree.reason.clone()), me, Held::BySession) {
        Standing::Ours => "this session's own".to_string(),
        _ => whose(&tree.reason),
    };
    hand_back(
        here,
        &tree.path,
        &format!(
            "it was {was}; this stretch of work landed, so the letter is back on the roster. \
             New work in this tree claims it back at its first edit, and `cargo xtask seat` \
             hands out another if somebody took the letter meanwhile. A picture claims \
             nothing: the landed work's pictures do not go up again."
        ),
    );
}

fn release_already_landed(here: &str, branch: &str, me: &Identity) -> Result<(), String> {
    println!("{branch} has nothing main does not already have — nothing to land.");
    let listing =
        git_query(here, &["worktree", "list", "--porcelain"]).ok_or("git worktree list failed")?;
    let trees = worktree_blocks(&listing);
    if let Some(tree) = landed_claim(&trees, branch) {
        if crate::seats::dirty_lines(&tree.path) == Some(0) {
            release_claim(here, &trees, branch, me);
        } else {
            println!(
                "the seat claim stays: {} is dirty or could not be inspected",
                tree.path
            );
        }
    }
    Ok(())
}

/// The unlock itself, and what to do when git will not do it. `why` says
/// whose claim this was: when it was this session's own, the session may
/// still be sitting in the seat.
fn hand_back(here: &str, path: &str, why: &str) {
    if git_query(here, &["worktree", "unlock", path]).is_some() {
        println!("released the seat claim on {path} — {why}");
    } else {
        println!(
            "note: the seat claim on {path} did not release — \
             `git worktree unlock {path}` by hand."
        );
    }
}

/// The tree whose checked-out branch just landed, when a session's claim
/// holds it.
fn landed_claim<'a>(trees: &'a [WorktreeBlock], branch: &str) -> Option<&'a WorktreeBlock> {
    trees
        .iter()
        .find(|tree| tree.branch == branch && tree.locked && is_a_seat_claim(&tree.reason))
}

/// The branch under the tree this runs from, when it is a seat branch —
/// the usual call is bare `land` from the seat whose work is done.
fn current_branch(here: &str) -> Result<String, String> {
    let branch = git_query(here, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok_or("git could not name the current branch")?;
    if branch == "HEAD" {
        return Err("this tree is detached — name the branch to land".into());
    }
    if branch == "main" {
        return Err("this tree sits on main — name the branch to land".into());
    }
    Ok(branch)
}

/// Fast-forward main in the tree that has it checked out (the primary).
/// The branch was just rebased onto main, so anything but a
/// fast-forward means main moved meanwhile — land again.
fn forward_in(primary: &str, branch: &str) -> Result<(), String> {
    let mut command = std::process::Command::new("git");
    command
        .arg("-C")
        .arg(primary)
        .args(["merge", "--ff-only", branch]);
    let output = crate::subprocess::run_captured(&mut command)?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "the fast-forward of main in the primary checkout was refused:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

/// Main is checked out nowhere, so the ref can move without leaving any
/// index behind — forward only, which the rebase made it.
fn forward_ref(here: &str, primary: &WorktreeBlock, branch: &str) -> Result<(), String> {
    git_query(here, &["fetch", ".", &format!("{branch}:main")])
        .ok_or("the fast-forward of refs/heads/main was refused (see the hook's reason above)")?;
    reattach(primary);
    Ok(())
}

/// If the primary checkout was detached exactly at what main now is, put
/// it back on the branch — its working tree does not move, and the next
/// land finds main checked out where everyone expects it.
fn reattach(primary: &WorktreeBlock) {
    if !primary.branch.is_empty() {
        return;
    }
    let dir = primary.path.as_str();
    let at = git_query(dir, &["rev-parse", "HEAD"]);
    let main = git_query(dir, &["rev-parse", "main"]);
    let clean = git_query(dir, &["status", "--porcelain"]).is_some_and(|s| s.is_empty());
    if at.is_some() && at == main && clean && git_query(dir, &["switch", "main"]).is_some() {
        println!("the primary checkout was detached at that very commit — reattached to main.");
    } else {
        println!(
            "note: the primary checkout is detached and was left as it is \
             ({}) — its working tree does not show main.",
            at.as_deref().unwrap_or("unreadable")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{build_slot_tree, clock, landed_claim, still_the_ask};
    use crate::seats::{Identity, worktree_blocks};

    #[test]
    fn the_permit_is_read_again_and_only_the_open_one_it_began_on_lets_the_landing_on() {
        let yard = crate::yard::Yard::new("land-permit");
        let repo = crate::seats::slashed(&yard);
        assert!(crate::subprocess::git_query(&repo, &["init", "-q", "-b", "main"]).is_some());
        let me = |session: &str| Identity {
            session: session.into(),
            pid: None,
            host: None,
        };
        assert!(still_the_ask(&repo, &me(""), None).is_ok(), "a hand run");
        assert!(
            still_the_ask(&repo, &me("s"), None).is_ok(),
            "no permit on record"
        );
        let permits = yard.join(".permits");
        std::fs::create_dir_all(&permits).expect("permits");
        std::fs::write(permits.join("s.land"), "open\t1\t/land\n").expect("permit");
        let began = crate::hook::permit::asked(&repo, "s");
        assert!(still_the_ask(&repo, &me("s"), began.as_ref()).is_ok());
        for standing in ["closed\t2\t見た", "spent\t1\t/land", "waiting\t1\t/land"] {
            std::fs::write(permits.join("s.land"), format!("{standing}\n")).expect("permit");
            let refused = still_the_ask(&repo, &me("s"), began.as_ref()).expect_err(standing);
            assert!(refused.contains("no longer open"), "{refused}");
        }
        assert!(
            still_the_ask(&repo, &me("s"), began.as_ref())
                .expect_err("waiting")
                .contains("「/land」")
        );
        std::fs::write(permits.join("s.land"), "open\t2\t/land push\n").expect("permit");
        let more = still_the_ask(&repo, &me("s"), began.as_ref()).expect_err("said more");
        assert!(
            more.contains("said more") && more.contains("「/land push」"),
            "{more}"
        );
        // A landing past its last look spends only the message it began on.
        crate::hook::permit::landed(&repo, "s", began.as_ref());
        assert!(
            crate::hook::permit::is_open(&repo, "s"),
            "the later /land's"
        );
        let later = crate::hook::permit::asked(&repo, "s");
        crate::hook::permit::landed(&repo, "s", later.as_ref());
        assert!(!crate::hook::permit::is_open(&repo, "s"));
    }

    /// A tree root spelled the way the running system spells one.
    fn tree() -> &'static str {
        if cfg!(windows) {
            "C:/x/seat"
        } else {
            "/x/seat"
        }
    }

    fn binary(stem: &str) -> String {
        format!("{stem}{}", std::env::consts::EXE_SUFFIX)
    }

    #[test]
    fn the_build_slot_is_the_task_runner_under_a_tree_s_target() {
        let root = std::path::Path::new(tree());
        let slot = root.join("target").join("debug").join(binary("xtask"));
        assert_eq!(build_slot_tree(&slot).as_deref(), Some(tree()));
        let release = root.join("target").join("release").join(binary("xtask"));
        assert_eq!(build_slot_tree(&release).as_deref(), Some(tree()));
        assert_eq!(
            build_slot_tree(
                &root
                    .join("target")
                    .join("debug")
                    .join(binary("platitude-gg"))
            ),
            None,
            "the app shares the directory and is nobody's task runner"
        );
        assert_eq!(
            build_slot_tree(&root.join("tools").join(binary("xtask"))),
            None,
            "a copy outside target/ is not what cargo writes"
        );
    }

    #[test]
    fn releases_only_a_landed_tree_held_by_a_session_claim() {
        let listing = "worktree C:/x/platitude-gg\nHEAD 1111\nbranch refs/heads/main\n\n\
                       worktree C:/x/platitude-gg/.claude/worktrees/a\nHEAD 2222\nbranch refs/heads/worktree-a\nlocked claude-seat abc\n\n\
                       worktree C:/x/platitude-gg/.claude/worktrees/b\nHEAD 3333\nbranch refs/heads/worktree-b\nlocked parked by hand\n\n\
                       worktree C:/x/platitude-gg/.claude/worktrees/c\nHEAD 4444\nbranch refs/heads/worktree-c\n";
        let trees = worktree_blocks(listing);
        assert_eq!(
            landed_claim(&trees, "worktree-a").map(|tree| tree.path.as_str()),
            Some("C:/x/platitude-gg/.claude/worktrees/a")
        );
        assert!(
            landed_claim(&trees, "worktree-b").is_none(),
            "a lock a person wrote stays"
        );
        assert!(
            landed_claim(&trees, "worktree-c").is_none(),
            "no lock, nothing to release"
        );
        assert!(
            landed_claim(&trees, "worktree-x").is_none(),
            "a branch checked out nowhere has no seat to hand back"
        );
    }

    #[test]
    fn reads_the_primary_first_and_detachment_as_an_empty_branch() {
        let listing = "worktree C:/x/platitude-gg\nHEAD 1111\ndetached\n\n\
                       worktree C:/x/platitude-gg/.claude/worktrees/a\nHEAD 2222\nbranch refs/heads/worktree-a\n";
        let trees = worktree_blocks(listing);
        assert_eq!(trees.len(), 2);
        assert_eq!(trees[0].path, "C:/x/platitude-gg");
        assert!(trees[0].branch.is_empty());
        assert_eq!(trees[1].branch, "worktree-a");
    }

    #[test]
    fn a_phase_is_said_to_the_second_like_the_gates_wall_clock() {
        assert_eq!(clock(std::time::Duration::from_secs(0)), "0m00s");
        assert_eq!(clock(std::time::Duration::from_secs(7)), "0m07s");
        assert_eq!(clock(std::time::Duration::from_secs(244)), "4m04s");
        assert_eq!(clock(std::time::Duration::from_millis(59_900)), "0m59s");
    }
}
