//! Whether the current branch's tip is held by anything besides that
//! branch once a rewrite has moved the branch off it.
//!
//! A history rewrite moves the branch off its tip: if another ref reaches
//! the old chain it stays drawn and the work is one cherry-pick away;
//! otherwise only the reflog has it (デザイン規約 §回数押し).
//!
//! The tip alone is enough to ask about: the rewritten range is merge-free
//! (the sequencer refuses a merge), so every commit in it is an ancestor
//! of the tip.
//!
//! Whether a rewrite moves other branches besides HEAD's is the person's
//! own `rebase.updateRefs` (every rewrite here is a rebase that leaves the
//! flag to them — `integrate::rebase`): under it git takes the other local
//! branches on the tip along, so those hold nothing afterwards
//! ([`OnTip::branches`]) — except one another worktree has out, which git
//! leaves where it is (`sequencer::merge_todo`).
//!
//! Three questions, cheapest first: a ref exactly on the tip is in the
//! listing the refresh already read (free, and covers tags); the setting
//! is one `git config`, asked only where a local branch on the tip makes it
//! matter; a descendant branch, remote-tracking ref or stash needs a walk,
//! which leaves tags out — on a tag-heavy repository they dominate it
//! (ci/baseline/head-reach-windows-x64.md). Missing one costs a hold mark
//! where a click would do: the safe direction.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};
use crate::refs::{HeadState, RefEntry, RefKind};

/// What sits exactly on HEAD's commit besides HEAD's branch, read off the
/// refresh's listing (annotated tags peeled). No process.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OnTip {
    /// A ref no rewrite moves — a tag, a remote-tracking ref, or a branch
    /// another worktree has out.
    pub kept: bool,
    /// The other local branches, by short name: left on the old tip by
    /// git's default, taken to the new one under `rebase.updateRefs`.
    pub branches: Vec<String>,
}

impl OnTip {
    /// `out_elsewhere` names the branches other worktrees have out, by
    /// short name.
    pub fn of(refs: &[RefEntry], head: &HeadState, out_elsewhere: impl Fn(&str) -> bool) -> Self {
        let mut on_tip = Self::default();
        let Some(oid) = head.oid else {
            return on_tip;
        };
        for entry in refs
            .iter()
            .filter(|entry| !entry.is_head && entry.commit_oid() == oid)
        {
            match entry.kind {
                RefKind::LocalBranch if !out_elsewhere(&entry.short) => {
                    on_tip.branches.push(entry.short.to_string());
                }
                RefKind::LocalBranch | RefKind::RemoteBranch | RefKind::Tag => on_tip.kept = true,
            }
        }
        on_tip
    }
}

/// Whether the person's own `rebase.updateRefs` is on, so a rewrite takes
/// the branches on the tip along.
///
/// Anything but a readable true answers false: unset is git's default, and
/// a value git cannot read as a boolean stops the rebase itself
/// (`git_config_bool` dies on it), so nothing is rewritten under it.
pub async fn rewrites_take_branches_along(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new().cwd(workdir).answers_by_code(1).args([
        "config",
        "--type=bool",
        "--get",
        "rebase.updateRefs",
    ]);
    let out = executor.run_unchecked(cmd, cancel).await?;
    Ok(out.code == 0 && out.stdout.trim_ascii() == b"true")
}

/// True when a branch, a remote-tracking ref or the stash reaches `tip`
/// without the branches in `moving` — the short names of the ones the
/// rewrite takes off it (an empty name, HEAD's when detached, leaves
/// nothing out).
pub async fn reached_without(
    executor: &GitExecutor,
    workdir: &Path,
    tip: &str,
    moving: &[String],
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let out = executor.run(command(workdir, tip, moving), cancel).await?;
    // Anything printed is the tip, which no other ref reaches.
    Ok(out.stdout_utf8().trim().is_empty())
}

/// The walk behind [`reached_without`], split out to be read in a test
/// without a repository.
///
/// `--exclude` takes the **short name** — `refs/heads/main` silently
/// excludes nothing and every branch reads "held" — and a name is never a
/// glob (`check-ref-format` refuses `*`, `?` and `[`). Each one applies to
/// the `--branches` after it. `--glob=refs/stash*` keeps its `*`: the exact
/// refname matches nothing, and a plain `refs/stash` rev would need
/// `--ignore-missing`, which also swallows a bad `tip` (both in
/// rules-refs/core.md).
fn command(workdir: &Path, tip: &str, moving: &[String]) -> GitCommand {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["rev-list", "--max-count=1", tip, "--not"]);
    moving
        .iter()
        .filter(|branch| !branch.is_empty())
        .fold(cmd, |cmd, branch| cmd.arg(format!("--exclude={branch}")))
        .args(["--branches", "--remotes", "--glob=refs/stash*"])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oid::Oid;

    /// `name` under `refs/`; its short name drops the first part
    /// (`heads/feature/x` is `feature/x`).
    fn entry(name: &str, kind: RefKind, oid: &str, is_head: bool) -> RefEntry {
        RefEntry {
            name: crate::Name::from(format!("refs/{name}")),
            short: crate::Name::from(name.split_once('/').map_or(name, |(_, short)| short)),
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

    /// With no other worktree.
    fn on_tip(refs: &[RefEntry], head: &HeadState) -> OnTip {
        OnTip::of(refs, head, |_| false)
    }

    #[test]
    fn the_branch_head_is_on_does_not_hold_its_own_tip() {
        let refs = vec![entry("heads/main", RefKind::LocalBranch, A, true)];
        assert_eq!(on_tip(&refs, &head(A)), OnTip::default());
    }

    #[test]
    fn a_tag_or_a_remote_on_the_same_commit_is_kept() {
        for kind in [RefKind::RemoteBranch, RefKind::Tag] {
            let refs = vec![
                entry("heads/main", RefKind::LocalBranch, A, true),
                entry("other", kind, A, false),
            ];
            let found = on_tip(&refs, &head(A));
            assert!(found.kept, "{kind:?}");
            assert!(found.branches.is_empty(), "{kind:?}");
        }
    }

    #[test]
    fn a_branch_on_the_same_commit_is_named_for_the_setting_to_decide() {
        let refs = vec![
            entry("heads/main", RefKind::LocalBranch, A, true),
            entry("heads/feature/keep", RefKind::LocalBranch, A, false),
        ];
        let found = on_tip(&refs, &head(A));
        assert!(!found.kept);
        assert_eq!(found.branches, vec!["feature/keep".to_string()]);
    }

    #[test]
    fn a_branch_another_worktree_has_out_is_kept() {
        let refs = vec![
            entry("heads/main", RefKind::LocalBranch, A, true),
            entry("heads/out", RefKind::LocalBranch, A, false),
            entry("heads/free", RefKind::LocalBranch, A, false),
        ];
        let found = OnTip::of(&refs, &head(A), |name| name == "out");
        assert!(found.kept, "git leaves a branch another worktree has out");
        assert_eq!(found.branches, vec!["free".to_string()]);
    }

    #[test]
    fn a_ref_somewhere_else_holds_nothing() {
        let refs = vec![
            entry("heads/main", RefKind::LocalBranch, A, true),
            entry("tags/v1", RefKind::Tag, B, false),
            entry("heads/old", RefKind::LocalBranch, B, false),
        ];
        assert_eq!(on_tip(&refs, &head(A)), OnTip::default());
    }

    #[test]
    fn an_annotated_tag_counts_by_the_commit_it_peels_to() {
        let mut tag = entry("tags/v1", RefKind::Tag, B, false);
        tag.peeled = Some(Oid::from_hex_str(A).expect("test oid"));
        let refs = vec![entry("heads/main", RefKind::LocalBranch, A, true), tag];
        assert!(on_tip(&refs, &head(A)).kept);
    }

    #[test]
    fn an_unborn_branch_has_no_tip_to_hold() {
        let state = HeadState {
            branch: Some("main".to_string()),
            oid: None,
            detached: false,
        };
        assert_eq!(on_tip(&[], &state), OnTip::default());
    }

    /// Read as git reads a boolean (`--type=bool`). Git alone reads the
    /// type, so the command line is where it is pinned; what it makes of
    /// each spelling is `reachable_integration::periodic`'s.
    #[tokio::test]
    async fn the_setting_is_read_as_a_boolean() {
        let (exec, asked) = crate::refusing::git();
        let cancel = CancellationToken::new();
        rewrites_take_branches_along(&exec, &crate::refusing::nowhere(), &cancel)
            .await
            .expect_err("there is no git here to read with");
        assert_eq!(
            asked.displays(),
            ["git config --type=bool --get rebase.updateRefs"]
        );
    }

    #[test]
    fn the_exclusion_drops_the_refs_heads_prefix() {
        let cmd = command(Path::new("."), "HEAD", &["feature/login".to_string()]);
        let args = cmd.describe();
        assert!(args.contains("--exclude=feature/login"), "{args}");
        assert!(!args.contains("--exclude=refs/heads/"), "{args}");
    }

    #[test]
    fn every_moving_branch_is_excluded_ahead_of_the_branches() {
        let moving = ["main".to_string(), "keep".to_string()];
        let args = command(Path::new("."), "HEAD", &moving).describe();
        let branches = args.find("--branches").expect("the branches are walked");
        for name in ["main", "keep"] {
            let at = args
                .find(&format!("--exclude={name}"))
                .unwrap_or_else(|| panic!("{name} is excluded: {args}"));
            assert!(
                at < branches,
                "an exclusion applies to what follows it: {args}"
            );
        }
    }

    #[test]
    fn nothing_is_excluded_without_a_branch_name() {
        let args = command(Path::new("."), "HEAD", &[String::new()]).describe();
        assert!(!args.contains("--exclude"), "{args}");
        assert!(args.contains("--branches"), "{args}");
    }
}
