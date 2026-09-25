//! `cargo xtask land [<branch>]` — the one way a branch reaches main
//! (steps: internal-docs/反映前テストの機械化.md §land).
//!
//! A merge typed in the primary checkout inherits whatever HEAD is there:
//! a detached HEAD or a stray branch fast-forwards the wrong thing or
//! strands the commits. This verb reads where main actually is and picks
//! the safe move. Its fast-forward is what spends the permit
//! (hook/permit.rs).

use crate::command::{self, Permission, Where};
use crate::gate::Gated;
use crate::seats::{
    Held, Identity, Standing, WorktreeBlock, is_a_seat_claim, same_tree, standing, whose,
    worktree_blocks,
};
use crate::subprocess::git_query;

mod record;
use record::Phases;

pub(crate) static LAND: command::Command = command::Command {
    id: "land.branch",
    call: "land <branch>",
    purpose: "rebase, gate and fast-forward main onto a branch — the one sanctioned way",
    run_in: Where::Seat,
    needs: &["the branch checked out in a seat, with nothing uncommitted"],
    permission: Permission::Permit,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&LAND];

pub fn run(args: &[String]) -> Result<(), String> {
    let mut phases = Phases::start();
    let result = land(args, &mut phases);
    phases.finish(&result);
    result
}

fn land(args: &[String], phases: &mut Phases) -> Result<(), String> {
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
    phases.target(&root, "");
    let here = root.to_string_lossy().replace('\\', "/");
    let branch = match branch {
        Some(name) => name,
        None => current_branch(&here)?,
    };
    phases.target(&root, &branch);
    if branch == "main" {
        return Err("land moves a branch onto main; main itself is not one".into());
    }
    git_query(
        &here,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ],
    )
    .ok_or_else(|| format!("no local branch named {branch:?}"))?;
    let ahead = crate::seats::commits_in(&here, &format!("main..{branch}"))
        .ok_or("git could not count main..branch — is main a local branch here?")?;
    if ahead == 0 {
        return release_already_landed(&here, &branch);
    }
    let listing =
        git_query(&here, &["worktree", "list", "--porcelain"]).ok_or("git worktree list failed")?;
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
    let Some(seat) = trees.iter().find(|tree| tree.branch == branch) else {
        return Err(format!(
            "{branch} is checked out nowhere — the gate runs in the branch's own worktree. \
             Check it out in a seat and land again."
        ));
    };
    let seat_dir = std::path::Path::new(&seat.path);
    phases.target(seat_dir, &branch);
    settle_the_tree(&seat.path, &branch)?;
    // The landings' queue (`budget::Pool::turn`), held to the end. Taken
    // before the seat's tree, its gate and the machine's budget, so a
    // landing standing in line stands in nobody's way.
    let pool = crate::budget::Pool::of(&root, crate::budget::default_jobs())?;
    phases.mark("preflight");
    let turn = pool.turn(&crate::seats::slashed(seat_dir), &format!("land {branch}"));
    phases.mark("queue");
    let _turn = turn?;
    prepare_gate(&root, &seat.path)?;
    phases.mark("prepare");
    if git_query(&here, &["merge-base", "--is-ancestor", "main", &branch]).is_none() {
        println!("{branch} is behind main — rebasing it in {}", seat.path);
        rebase(&seat.path)?;
    }
    phases.mark("rebase");
    gate_in_the_seat(seat_dir, &branch)?;
    phases.mark("gate");
    let ahead = crate::seats::commits_in(&here, &format!("main..{branch}")).unwrap_or(ahead);
    let before = git_query(&here, &["rev-parse", "--short", "main"]).unwrap_or_default();
    if primary.branch == "main" {
        forward_in(&primary.path, &branch)?;
    } else {
        forward_ref(&here, primary, &branch)?;
    }
    phases.mark("fast-forward (verdict included)");
    let after = git_query(&here, &["rev-parse", "--short", "main"]).unwrap_or_default();
    // The permit is spent only here: a landing that stopped before this
    // line moved nothing.
    crate::hook::permit::landed(&here, &Identity::current(None).session);
    // Again, now that main moved: git runs a copy of the hook script, and
    // a landing that changed the script would leave the old copy answering.
    println!("{}", crate::gate::install(&root)?);
    println!("landed {branch}: main {before} -> {after} ({ahead} commit(s)).");
    // While the claim still stands, so nothing else is building here: the
    // rebase may have left a generation of build products behind.
    crate::sweep::at_a_tail(seat_dir, &crate::sweep::Tail::after_a_landing());
    phases.mark("sweep");
    // The claim comes off last: once it does, another session may take the
    // letter, and clearing the board after that would clear its runs.
    clear_the_board(&listing, &branch);
    release_claim(&here, &trees, &branch);
    phases.mark("hook, board and claim");
    Ok(())
}

/// What would land is what stands committed, so the seat must be clean —
/// except for a census a gate or verb run there rewrote, which the
/// landing commits as it does its own gate's rewrite (`gate_in_the_seat`).
fn settle_the_tree(seat: &str, branch: &str) -> Result<(), String> {
    let dirty = git_query(seat, &["status", "--porcelain"]).unwrap_or_default();
    if census_alone(&dirty) {
        commit_census(seat)?;
        println!(
            "{} was rewritten in {seat} before this landing: committed on {branch}",
            crate::gate::CENSUS_FILE
        );
        return Ok(());
    }
    if dirty.is_empty() {
        return Ok(());
    }
    Err(format!(
        "{seat} has uncommitted changes — what would land is not what is there. Commit or \
         stash first:\n{dirty}"
    ))
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
    println!("{}", crate::gate::install(root)?);
    Ok(())
}

/// A duration to the second, the way the gate says the wall clock a
/// landing's phases are compared against.
fn clock(took: std::time::Duration) -> String {
    let secs = took.as_secs();
    format!("{}m{:02}s", secs / 60, secs % 60)
}

/// What a landing leaves in a build slot it stepped out of and could not
/// delete: its own running image.
const INFLIGHT: &str = "xtask-inflight-";

/// Where cargo writes the binary the slot is linked to.
const DEPS: &str = "deps";

/// Frees the cargo build slot this process occupies, when the slot is one
/// of `trees`'.
///
/// The gate rebuilds the seat's task runner, and this process is
/// `<tree>/target/debug/xtask`: on Windows a running image cannot be
/// replaced (`os error 5`) but can be renamed, so this process moves out
/// of the name. Out of every name ([`names_of_this_image`]): cargo
/// hard-links the slot to `deps/xtask-<hash>`, and freeing one name alone
/// stops the gate's link step at the other (`LNK1104`), after the rebase.
///
/// What cannot be moved waits for the next landing's sweep. Silent when
/// this binary is in nobody's way; a move that fails says so.
fn step_out_of_the_build_slot(trees: &[&str]) -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let mine = build_slot_tree(&exe)?;
    if !trees.iter().any(|tree| same_tree(tree, &mine)) {
        return None;
    }
    let dir = exe.parent()?;
    sweep(dir);
    sweep(&dir.join(DEPS));
    let mut stood = Vec::new();
    let mut freed = Vec::new();
    for (at, name) in names_of_this_image(&exe, dir).iter().enumerate() {
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
        "stepped out of {} — the landing rebuilds the task runner there, and a running image \
         cannot be replaced.",
        freed.join(", ")
    ))
}

/// Every name in this tree's build directory that cargo writes this
/// binary under: the slot the run was started from, and the hashed names
/// in `deps/` the slot is linked to.
///
/// By name, not by inode: Windows offers one only through an unstable
/// interface. A hash not in use is an older build of this binary, which
/// cargo relinks anyway.
fn names_of_this_image(exe: &std::path::Path, dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut names = vec![exe.to_path_buf()];
    let Ok(entries) = std::fs::read_dir(dir.join(DEPS)) else {
        return names;
    };
    for entry in entries.flatten() {
        if cargo_writes_this_binary_at(&entry.file_name().to_string_lossy()) {
            names.push(entry.path());
        }
    }
    names
}

/// Whether a name in `deps/` is `xtask` or `xtask-<hash>` with the
/// executable suffix alone — not the `.d` / `.pdb` beside it, nor an
/// earlier landing's aside (already swept).
fn cargo_writes_this_binary_at(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(std::env::consts::EXE_SUFFIX) else {
        return false;
    };
    (stem == "xtask" || stem.starts_with("xtask-"))
        && !stem.contains('.')
        && !name.starts_with(INFLIGHT)
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

/// The gate in the seat. A census its verbs rewrote leaves the tree that
/// passed unstamped, so it is committed and the gate asked again (every
/// step cached: the census is no step's input). A second rewrite is a
/// census moving under the machine's timing, and is reported.
fn gate_in_the_seat(seat: &std::path::Path, branch: &str) -> Result<(), String> {
    if crate::gate::for_landing(seat, "main")? == Gated::Stamped {
        return Ok(());
    }
    commit_census(&crate::seats::slashed(seat))?;
    println!(
        "the land's gate rewrote {}: committed on {branch}, and the gate runs again",
        crate::gate::CENSUS_FILE
    );
    if crate::gate::for_landing(seat, "main")? == Gated::Stamped {
        return Ok(());
    }
    Err(format!(
        "the verbs rewrote {} again, on the run that was to stamp the commit holding the first \
         rewrite — the census is moving under the machine's timing. \
         Read the two rewrites' diffs before landing.",
        crate::gate::CENSUS_FILE
    ))
}

/// The message of every commit of the census the gate rewrote.
const CENSUS_COMMIT: &str = "chore(xtask): the verb census as the land's gate rewrote it";

/// Whether a seat's `status --porcelain` says the census, modified in the
/// working tree, is the whole of what stands uncommitted.
fn census_alone(dirty: &str) -> bool {
    dirty.trim() == format!("M {}", crate::gate::CENSUS_FILE)
}

/// Commits the census the gate's verbs rewrote, alone: a landing commits
/// only what the machine wrote, so anything else standing is refused.
fn commit_census(seat: &str) -> Result<(), String> {
    let census = crate::gate::CENSUS_FILE;
    let dirty = git_query(seat, &["status", "--porcelain"]).unwrap_or_default();
    if !census_alone(&dirty) {
        return Err(format!(
            "the gate left more than {census} changed in {seat}, and a landing commits the \
             census alone:\n{dirty}"
        ));
    }
    let mut command = std::process::Command::new("git");
    command
        .arg("-C")
        .arg(seat)
        .args(["commit", "-q", "-m", CENSUS_COMMIT, "--", census]);
    let output = crate::subprocess::run_captured(&mut command)?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "the census could not be committed in {seat}:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
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
/// its next edit or picture (`seats::held_by_this_run`).
fn release_claim(here: &str, trees: &[WorktreeBlock], branch: &str) {
    let Some(tree) = landed_claim(trees, branch) else {
        return;
    };
    let was = match standing(
        Some(tree.reason.clone()),
        &Identity::current(None),
        Held::BySession,
    ) {
        Standing::Ours => "this session's own".to_string(),
        _ => whose(&tree.reason),
    };
    hand_back(
        here,
        &tree.path,
        &format!(
            "it was {was}; this stretch of work landed, so the letter is back on the roster. \
             Going on in this tree claims it back at the next edit or picture, and `cargo \
             xtask seat` hands out another if somebody took the letter meanwhile."
        ),
    );
}

fn release_already_landed(here: &str, branch: &str) -> Result<(), String> {
    println!("{branch} has nothing main does not already have — nothing to land.");
    let listing =
        git_query(here, &["worktree", "list", "--porcelain"]).ok_or("git worktree list failed")?;
    let trees = worktree_blocks(&listing);
    if let Some(tree) = landed_claim(&trees, branch) {
        if crate::seats::dirty_lines(&tree.path) == Some(0) {
            release_claim(here, &trees, branch);
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
    use super::{build_slot_tree, clock, landed_claim};
    use crate::seats::worktree_blocks;

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
