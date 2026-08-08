//! Whether the current branch's tip is held by anything besides that
//! branch.
//!
//! A history rewrite moves the branch off its tip and replays what came
//! after the edit. Everything the old chain held on to goes with it — but
//! only as far as no other ref reaches it. If something else does, the old
//! commits stay drawn on the graph and the work is one cherry-pick away;
//! if nothing does, the reflog is all that is left (デザイン規約 §長押し).
//!
//! The answer is about the branch tip alone. The rewritten range is
//! merge-free (the sequencer refuses a range with a merge in it), so every
//! commit in it is an ancestor of the tip: whatever reaches the tip reaches
//! all of them, and if nothing reaches the tip then the tip itself is lost.
//!
//! Two questions in one, for cost. A ref sitting exactly on the tip is
//! already in the listing the refresh just read, so that half is free and
//! covers tags. The other half — a branch, a remote-tracking ref or a
//! stash that is a *descendant* of the tip — needs git, and deliberately
//! leaves tags out of the walk: on the benchmark repository they are 45,846
//! of the 53,672 refs and 478 ms of the 504 ms (ci/baseline). Missing one
//! costs a hold mark on a row that could have been a click, which is the
//! direction to be wrong in.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};
use crate::refs::{HeadState, RefEntry};

/// True when some ref other than the branch HEAD is on already sits on the
/// commit HEAD points at.
///
/// Reads the listing the refs refresh already has, so it costs no process.
/// Annotated tags count peeled: what matters is the commit, not the tag
/// object.
pub fn a_ref_sits_on_head(refs: &[RefEntry], head: &HeadState) -> bool {
    let Some(oid) = head.oid else {
        return false;
    };
    refs.iter()
        .any(|entry| !entry.is_head && entry.commit_oid() == oid)
}

/// True when a branch, a remote-tracking ref or the stash reaches `tip`
/// without `branch` — that is, when moving `branch` off `tip` would leave
/// the old commits still drawn.
///
/// `branch` is the short name of the branch to leave out (the one about to
/// be rewritten); an empty name leaves nothing out.
pub async fn reached_without_branch(
    executor: &GitExecutor,
    workdir: &Path,
    tip: &str,
    branch: &str,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let out = executor.run(command(workdir, tip, branch), cancel).await?;
    // Anything printed is a commit no other ref reaches — the tip itself,
    // since the walk starts there and stops at the first one.
    Ok(out.stdout_utf8().trim().is_empty())
}

/// The walk behind [`reached_without_branch`], split out to be read in a
/// test without a repository.
///
/// The exclusion pattern for `--branches` is given **without** the
/// `refs/heads/` prefix: git matches it against the part after the
/// namespace, so `--exclude=refs/heads/main` silently excludes nothing and
/// the walk then answers "held" for every branch there is (measured).
///
/// `--glob=refs/stash*` rather than `refs/stash`: a `--glob` pattern only
/// matches a hierarchy, so the exact refname alone matches nothing
/// (measured). Naming `refs/stash` as a plain rev would need
/// `--ignore-missing` for the repositories that never stashed, and that
/// flag would also swallow a bad `tip` — which would come back as "held",
/// the wrong way to fail.
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
            name: format!("refs/{name}"),
            short: name.to_string(),
            kind,
            target: Oid::from_hex_str(oid).expect("test oid"),
            peeled: None,
            upstream: None,
            is_head,
            created_unix: 0,
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
