//! Whether the current branch's tip is held by anything besides that
//! branch.
//!
//! A history rewrite moves the branch off its tip: if another ref reaches
//! the old chain it stays drawn and the work is one cherry-pick away;
//! otherwise only the reflog has it (デザイン規約 §長押し).
//!
//! The tip alone is enough to ask about: the rewritten range is merge-free
//! (the sequencer refuses a merge), so every commit in it is an ancestor
//! of the tip.
//!
//! Two questions, for cost: a ref exactly on the tip is in the listing the
//! refresh already read (free, and covers tags); a descendant branch,
//! remote-tracking ref or stash needs git, and that walk leaves tags out —
//! on a tag-heavy repository they dominate it
//! (ci/baseline/head-reach-windows-x64.md). Missing one costs a hold mark
//! where a click would do: the safe direction.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};
use crate::refs::{HeadState, RefEntry};

/// True when a ref other than HEAD's branch sits on HEAD's commit
/// (annotated tags peeled). No process: reads the refresh's listing.
pub fn a_ref_sits_on_head(refs: &[RefEntry], head: &HeadState) -> bool {
    let Some(oid) = head.oid else {
        return false;
    };
    refs.iter()
        .any(|entry| !entry.is_head && entry.commit_oid() == oid)
}

/// True when a branch, a remote-tracking ref or the stash reaches `tip`
/// without `branch` (the short name of the branch about to be rewritten;
/// empty leaves nothing out).
pub async fn reached_without_branch(
    executor: &GitExecutor,
    workdir: &Path,
    tip: &str,
    branch: &str,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let out = executor.run(command(workdir, tip, branch), cancel).await?;
    // Anything printed is the tip, which no other ref reaches.
    Ok(out.stdout_utf8().trim().is_empty())
}

/// The walk behind [`reached_without_branch`], split out to be read in a
/// test without a repository.
///
/// `--exclude` takes the **short name** — `refs/heads/main` silently
/// excludes nothing and every branch reads "held". `--glob=refs/stash*`
/// keeps its `*`: the exact refname matches nothing, and a plain
/// `refs/stash` rev would need `--ignore-missing`, which also swallows a
/// bad `tip` (both in rules-refs/core.md).
fn command(workdir: &Path, tip: &str, branch: &str) -> GitCommand {
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .args(["rev-list", "--max-count=1", tip, "--not"]);
    if !branch.is_empty() {
        cmd = cmd.arg(format!("--exclude={branch}"));
    }
    cmd.args(["--branches", "--remotes", "--glob=refs/stash*"])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oid::Oid;
    use crate::refs::RefKind;

    fn entry(name: &str, kind: RefKind, oid: &str, is_head: bool) -> RefEntry {
        RefEntry {
            name: crate::Name::from(format!("refs/{name}")),
            short: crate::Name::from(name),
            kind,
            target: Oid::from_hex_str(oid).expect("test oid"),
            peeled: None,
            upstream: None,
            is_head,
            created_unix: 0,
            ahead: 0,
            behind: 0,
        }
    }

    fn head(oid: &str) -> HeadState {
        HeadState {
            branch: Some("main".to_string()),
            oid: Some(Oid::from_hex_str(oid).expect("test oid")),
            detached: false,
        }
    }

    const A: &str = "1111111111111111111111111111111111111111";
    const B: &str = "2222222222222222222222222222222222222222";

    #[test]
    fn the_branch_head_is_on_does_not_hold_its_own_tip() {
        let refs = vec![entry("heads/main", RefKind::LocalBranch, A, true)];
        assert!(!a_ref_sits_on_head(&refs, &head(A)));
    }

    #[test]
    fn another_ref_on_the_same_commit_holds_it() {
        for kind in [RefKind::LocalBranch, RefKind::RemoteBranch, RefKind::Tag] {
            let refs = vec![
                entry("heads/main", RefKind::LocalBranch, A, true),
                entry("other", kind, A, false),
            ];
            assert!(a_ref_sits_on_head(&refs, &head(A)), "{kind:?}");
        }
    }

    #[test]
    fn a_ref_somewhere_else_holds_nothing() {
        let refs = vec![
            entry("heads/main", RefKind::LocalBranch, A, true),
            entry("tags/v1", RefKind::Tag, B, false),
        ];
        assert!(!a_ref_sits_on_head(&refs, &head(A)));
    }

    #[test]
    fn an_annotated_tag_counts_by_the_commit_it_peels_to() {
        let mut tag = entry("tags/v1", RefKind::Tag, B, false);
        tag.peeled = Some(Oid::from_hex_str(A).expect("test oid"));
        let refs = vec![entry("heads/main", RefKind::LocalBranch, A, true), tag];
        assert!(a_ref_sits_on_head(&refs, &head(A)));
    }

    #[test]
    fn an_unborn_branch_has_no_tip_to_hold() {
        let state = HeadState {
            branch: Some("main".to_string()),
            oid: None,
            detached: false,
        };
        assert!(!a_ref_sits_on_head(&[], &state));
    }

    #[test]
    fn the_exclusion_drops_the_refs_heads_prefix() {
        let cmd = command(Path::new("."), "HEAD", "feature/login");
        let args = cmd.describe();
        assert!(args.contains("--exclude=feature/login"), "{args}");
        assert!(!args.contains("--exclude=refs/heads/"), "{args}");
    }

    #[test]
    fn nothing_is_excluded_without_a_branch_name() {
        let args = command(Path::new("."), "HEAD", "").describe();
        assert!(!args.contains("--exclude"), "{args}");
        assert!(args.contains("--branches"), "{args}");
    }
}
