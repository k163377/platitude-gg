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
    let trees = trees(&listing);
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
    Ok(())
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
    if crate::run_captured(&mut abort).is_err() {
        // The error below already tells the reader the merge stopped;
        // a failed abort leaves the same conflict for the same hands.
    }
    Err(format!(
        "the merge into main stopped and was walked back:\n{}{}\n\
         resolve it with the user — a conflict on main is not resolved unattended",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

/// Main is checked out nowhere, so the ref can move without leaving any
/// index behind — but only forward: without a working tree there is no
/// place for a real merge to happen.
fn forward_ref(here: &str, primary: &Tree, branch: &str) -> Result<(), String> {
    if git_query(here, &["merge-base", "--is-ancestor", "main", branch]).is_none() {
        return Err(format!(
            "main and {branch} have diverged, and main is checked out nowhere \
             (the primary checkout sits on {}) — put the primary back on main \
             (`git switch main` there, with the user) and land again",
            primary.head_name()
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
fn reattach(primary: &Tree) {
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

/// One tree of `git worktree list --porcelain`, first entry the primary.
struct Tree {
    path: String,
    /// Branch name, empty when detached.
    branch: String,
}

impl Tree {
    fn head_name(&self) -> &str {
        if self.branch.is_empty() {
            "a detached HEAD"
        } else {
            self.branch.as_str()
        }
    }
}

fn trees(listing: &str) -> Vec<Tree> {
    let mut trees = Vec::new();
    for block in listing.split("\n\n") {
        let mut path = None;
        let mut branch = String::new();
        for line in block.lines() {
            if let Some(rest) = line.strip_prefix("worktree ") {
                path = Some(rest.replace('\\', "/"));
            } else if let Some(rest) = line.strip_prefix("branch refs/heads/") {
                branch = rest.to_string();
            }
        }
        if let Some(path) = path {
            trees.push(Tree { path, branch });
        }
    }
    trees
}

#[cfg(test)]
mod tests {
    use super::trees;

    #[test]
    fn reads_the_primary_first_and_detachment_as_an_empty_branch() {
        let listing = "worktree C:/x/platitude-gg\nHEAD 1111\ndetached\n\n\
                       worktree C:/x/platitude-gg/.claude/worktrees/a\nHEAD 2222\nbranch refs/heads/worktree-a\n";
        let trees = trees(listing);
        assert_eq!(trees.len(), 2);
        assert_eq!(trees[0].path, "C:/x/platitude-gg");
        assert!(trees[0].branch.is_empty());
        assert_eq!(trees[0].head_name(), "a detached HEAD");
        assert_eq!(trees[1].branch, "worktree-a");
    }
}
