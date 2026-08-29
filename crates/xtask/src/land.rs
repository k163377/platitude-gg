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

use crate::git_query;
use crate::seats::{SEAT_CLAIM, WorktreeBlock, worktree_blocks};

pub fn run(args: &[String]) -> Result<(), String> {
    let root = crate::workspace_root();
    let here = root.to_string_lossy().replace('\\', "/");
    let branch = match args {
        [] => current_branch(&here)?,
        [name] => name.clone(),
        more => return Err(format!("land takes one branch at most (got {more:?})")),
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
    let before = git_query(&here, &["rev-parse", "--short", "main"]).unwrap_or_default();
    let holder = trees.iter().find(|tree| tree.branch == "main");
    match holder {
        Some(tree) if tree.path == primary.path => merge_in(&tree.path.clone(), &branch)?,
        Some(tree) => {
            // A seat sitting on main would receive the merge into its own
            // working tree — nobody expects a seat to be main's window.
            return Err(format!(
                "main is checked out in {}, not in the primary checkout — \
                 free it (switch that tree to another branch) and land again",
                tree.path
            ));
        }
        None => forward_ref(&here, primary, &branch)?,
    }
    let after = git_query(&here, &["rev-parse", "--short", "main"]).unwrap_or_default();
    println!("landed {branch}: main {before} -> {after} ({ahead} commit(s)).");
    release_claim(&here, &trees, &branch);
    clear_the_board(&listing, &branch);
    Ok(())
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

/// Merge into the tree that has main checked out (the primary). A merge
/// that stops is walked back and reported — resolving conflicts on main
/// unattended is nobody's instruction.
fn merge_in(primary: &str, branch: &str) -> Result<(), String> {
    let mut command = std::process::Command::new("git");
    command.arg("-C").arg(primary).args(["merge", branch]);
    let output = crate::run_captured(&mut command)?;
    if output.status.success() {
        return Ok(());
    }
    let mut abort = std::process::Command::new("git");
    abort.arg("-C").arg(primary).args(["merge", "--abort"]);
    // run_captured only fails on a spawn error — the abort's own exit
    // code has to be read, or "walked back" is claimed over a primary
    // still standing mid-merge.
    let walked_back = crate::run_captured(&mut abort).is_ok_and(|out| out.status.success());
    Err(format!(
        "the merge into main stopped and was {}:\n{}{}\n\
         resolve it with the user — a conflict on main is not resolved unattended",
        if walked_back {
            "walked back"
        } else {
            "left standing (the abort failed too — the primary checkout is mid-merge)"
        },
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

/// Main is checked out nowhere, so the ref can move without leaving any
/// index behind — but only forward: without a working tree there is no
/// place for a real merge to happen.
fn forward_ref(here: &str, primary: &WorktreeBlock, branch: &str) -> Result<(), String> {
    if git_query(here, &["merge-base", "--is-ancestor", "main", branch]).is_none() {
        return Err(format!(
            "main and {branch} have diverged, and main is checked out nowhere \
             (the primary checkout sits on {}) — put the primary back on main \
             (`git switch main` there, with the user) and land again",
            head_name(primary)
        ));
    }
    git_query(here, &["fetch", ".", &format!("{branch}:main")])
        .ok_or("the fast-forward of refs/heads/main failed")?;
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

/// What to call the primary checkout's HEAD in a sentence.
fn head_name(tree: &WorktreeBlock) -> &str {
    if tree.branch.is_empty() {
        "a detached HEAD"
    } else {
        tree.branch.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::{head_name, landed_claim};
    use crate::seats::worktree_blocks;

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
        assert_eq!(head_name(&trees[0]), "a detached HEAD");
        assert_eq!(trees[1].branch, "worktree-a");
    }
}
