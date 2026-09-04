//! `cargo xtask land [<branch>]` — the one way a branch reaches main.
//!
//! Sessions cannot be trusted to run the merge by hand: a worktree
//! session's git is fenced to its own tree, and a merge typed in the
//! primary checkout inherits whatever HEAD happens to be there — a
//! detached HEAD or a stray branch turns "merge into main" into a
//! fast-forward of the wrong thing, or strands the commits off every
//! branch (both observed). This verb reads where main actually is and
//! picks the safe move; the pre-shell hook still demands PG_ALLOW_MAIN
//! in front of it, so the transcript records that the user asked.
//!
//! The order is the gate's (internal-docs/反映前テストの機械化.md): a
//! branch behind main is rebased onto it in its own worktree first, then
//! gated there — every step its diff owes, cached by what each step
//! reads, so a run taken before the rebase is not paid twice — and only
//! then is main fast-forwarded, which the reference-transaction hook
//! allows onto a stamped commit and nothing else. History stays linear,
//! and a landed seat already stands at main's tip.

use crate::seats::{SEAT_CLAIM, WorktreeBlock, worktree_blocks};
use crate::subprocess::git_query;

pub fn run(args: &[String]) -> Result<(), String> {
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
    let here = root.to_string_lossy().replace('\\', "/");
    let branch = match branch {
        Some(name) => name,
        None => current_branch(&here)?,
    };
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
        println!("{branch} has nothing main does not already have — nothing to land.");
        return Ok(());
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
        // working tree — nobody expects a seat to be main's window.
        return Err(format!(
            "main is checked out in {}, not in the primary checkout — \
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
    let dirty = git_query(&seat.path, &["status", "--porcelain"]).unwrap_or_default();
    if !dirty.is_empty() {
        return Err(format!(
            "{} has uncommitted changes — what would land is not what is there. Commit or \
             stash first:\n{dirty}",
            seat.path
        ));
    }
    if let Some(word) = step_out_of_the_build_slot(&[&seat.path, &primary.path]) {
        println!("{word}");
    }
    // The hook that holds main to the stamp, in place before main moves.
    println!("{}", crate::gate::install(&root)?);
    if git_query(&here, &["merge-base", "--is-ancestor", "main", &branch]).is_none() {
        println!("{branch} is behind main — rebasing it in {}", seat.path);
        rebase(&seat.path)?;
    }
    crate::gate::for_landing(seat_dir, "main")?;
    let before = git_query(&here, &["rev-parse", "--short", "main"]).unwrap_or_default();
    if primary.branch == "main" {
        forward_in(&primary.path, &branch)?;
    } else {
        forward_ref(&here, primary, &branch)?;
    }
    let after = git_query(&here, &["rev-parse", "--short", "main"]).unwrap_or_default();
    // Again, now that main carries what it carries: git runs the copy
    // beside .git, and a landing that changed the script would otherwise
    // leave the old copy answering until some later session start.
    println!("{}", crate::gate::install(&root)?);
    println!("landed {branch}: main {before} -> {after} ({ahead} commit(s)).");
    release_claim(&here, &trees, &branch);
    clear_the_board(&listing, &branch);
    Ok(())
}

/// What a landing leaves in a build slot it stepped out of and could not
/// take away — its own image, still running (Windows).
const INFLIGHT: &str = "xtask-inflight-";

/// Frees the cargo build slot this process occupies, when the slot is one
/// of `trees`'.
///
/// Everything past here runs cargo in a tree this landing moves: the
/// gate's steps build the seat's task runner (`cargo run -p xtask`) over
/// the sources the rebase brought in, and the hook's verdict builds the
/// primary's over the ones the fast-forward checked out. `cargo xtask
/// land` is itself `<tree>/target/<profile>/xtask`, so on Windows that
/// build stops at `failed to remove file … (os error 5)` — a running
/// image cannot be replaced — and the landing dies before its first step.
/// Renaming one is allowed on both systems, so this process moves out of
/// the name and runs on from the copy beside it: Unix takes that copy at
/// once, Windows on the next landing's sweep. Silent when this binary is
/// in nobody's way; a move that fails says so rather than leaving the
/// cargo error it was meant to explain to arrive unannounced.
fn step_out_of_the_build_slot(trees: &[&str]) -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let mine = build_slot_tree(&exe)?;
    if !trees.iter().any(|tree| same_tree(tree, &mine)) {
        return None;
    }
    let dir = exe.parent()?;
    sweep(dir);
    let mut aside = dir.join(format!("{INFLIGHT}{}", std::process::id()));
    if let Some(extension) = exe.extension() {
        aside.set_extension(extension);
    }
    if let Err(why) = std::fs::rename(&exe, &aside) {
        return Some(format!(
            "note: this task runner is still standing in {} ({why}) — a step that has to rebuild \
             it there will stop at the running image.",
            exe.display()
        ));
    }
    let _ = std::fs::remove_file(&aside);
    Some(format!(
        "stepped out of {} — the landing rebuilds the task runner there, and a running image \
         cannot be replaced.",
        exe.display()
    ))
}

/// Takes away what earlier landings left in the slot's directory —
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
/// The tree is what tells the binary a step is about to rebuild from
/// another tree's, which this landing never touches.
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

/// Whether two paths name one tree. The filesystem answers it, because
/// the two are spelled by different mouths: git writes forward slashes
/// and the long name, while a process is handed the spelling it was
/// started with — an 8.3 short name, a junction, either case. A path the
/// filesystem cannot resolve falls back to its text.
fn same_tree(one: &str, other: &str) -> bool {
    match (std::fs::canonicalize(one), std::fs::canonicalize(other)) {
        (Ok(one), Ok(other)) => one == other,
        _ => tidy(one) == tidy(other),
    }
}

fn tidy(path: &str) -> String {
    let path = path.replace('\\', "/").trim_end_matches('/').to_string();
    if cfg!(windows) {
        path.to_lowercase()
    } else {
        path
    }
}

/// `git rebase main` in the seat, non-interactively. A rebase that stops
/// is walked back and reported — resolving conflicts unattended is
/// nobody's instruction.
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

/// The seat's pictures go with its claim (CLAUDE.md ビルド・テスト): the
/// work they were taken to show is on main and has been read, and a
/// board that keeps them makes the next session hunt through spent
/// evidence for the picture that is current. Only this seat's — the runs
/// beside them belong to seats still working.
///
/// The board is not what a land turns on, so a sweep that cannot happen
/// says so and leaves the merge reported as the success it was.
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

/// The claim's release point (CLAUDE.md ビルド・テスト): the reflection
/// instruction ends a seat's stretch of work, so the landed branch's
/// worktree is handed back the moment its commits are on main — whether
/// the session that worked it is still around or not. Further work there
/// claims the seat back at its first edit (the post-write hook). Only the
/// hooks' own kind of lock is lifted; a lock a person wrote stays.
fn release_claim(here: &str, trees: &[WorktreeBlock], branch: &str) {
    let Some(tree) = landed_claim(trees, branch) else {
        return;
    };
    if git_query(here, &["worktree", "unlock", &tree.path]).is_some() {
        println!(
            "released the seat claim on {} — the next edit there claims it back.",
            tree.path
        );
    } else {
        println!(
            "note: the seat claim on {} did not release — \
             `git worktree unlock {}` by hand.",
            tree.path, tree.path
        );
    }
}

/// The tree whose checked-out branch just landed, when a session's claim
/// (not a hand-written lock) holds it.
fn landed_claim<'a>(trees: &'a [WorktreeBlock], branch: &str) -> Option<&'a WorktreeBlock> {
    trees
        .iter()
        .find(|tree| tree.branch == branch && tree.locked && tree.reason.starts_with(SEAT_CLAIM))
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
    use super::{build_slot_tree, landed_claim, same_tree};
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
    fn one_tree_is_one_tree_however_it_is_spelled() {
        assert!(same_tree("C:/x/seat", "C:\\x\\seat"));
        assert!(same_tree("C:/x/seat/", "C:/x/seat"));
        assert!(!same_tree("C:/x/seat", "C:/x/seat-b"));
        assert_eq!(
            same_tree("C:/X/Seat", "C:/x/seat"),
            cfg!(windows),
            "the case is Windows' to ignore and nobody else's"
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
}
