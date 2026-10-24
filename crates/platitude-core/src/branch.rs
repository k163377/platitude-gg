//! Checkout and local branch management.
//!
//! Uses `switch`, never `checkout`: `checkout` also restores files, so a
//! branch named like a path is ambiguous to it.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// What a checkout should land on — always a branch; nothing here detaches
/// HEAD (デザイン規約 §ブランチ・コミットへの移動). A HEAD detached
/// elsewhere is read and worked from as normal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckoutTarget {
    /// An existing local branch.
    Branch { name: String },
    /// A remote-tracking branch: creates `local` tracking it and switches.
    Track { remote_ref: String, local: String },
    /// An existing local branch, moved to `start` before landing on it.
    /// Commits only that branch had are left unreferenced (the UI asks
    /// first). One command, so git does both or neither.
    ForceCreate { local: String, start: String },
}

/// What a move did.
#[derive(Debug)]
pub enum CheckoutOutcome {
    /// HEAD moved, carrying whatever uncommitted work did not stand in
    /// the way.
    Moved,
    /// git refused because uncommitted work stands in the way, touching
    /// nothing. Carries the refusal: the caller retries through a stash
    /// (`RepoSession::checkout`), and a second refusal is reported in git's
    /// own words.
    Blocked(GitError),
}

/// Whether a name opens with `-`: `git branch` reads one as an option
/// wherever nothing ends them first — as `git worktree add -b` hands its
/// name on.
#[must_use]
pub fn reads_as_option(name: &str) -> bool {
    name.starts_with('-')
}

/// Whether git will take a name for a branch: `check-ref-format`'s rules
/// ([`crate::tag::ref_format_ok`]), less a name that reads as an option or
/// is `HEAD` (`git check-ref-format --branch`). A pure function, since a
/// name box asks on every keystroke; `tests/it/worktree_add.rs` holds it
/// to real git.
#[must_use]
pub fn is_valid_name(name: &str) -> bool {
    !reads_as_option(name) && name != "HEAD" && crate::tag::ref_format_ok(name)
}

/// Whether git's refusal is the "your work is in the way" one, which a
/// stash gets past.
///
/// Reads human-facing output (under `LC_ALL=C`) because git has no
/// machine-readable answer to "why can I not move". Anything unrecognised
/// is `false`: a reworded message costs the retry only.
fn work_is_in_the_way(said: &str) -> bool {
    crate::relied_messages::SWITCH_IN_THE_WAY.found_in(said)
}

/// Moves HEAD to `target`, taking uncommitted work along where git will
/// have it.
///
/// A collision is carried by stashing across the move
/// (`RepoSession::checkout`), not `--merge`: that reports a conflicted
/// result as success with nothing to abort, and refuses while anything is
/// staged.
pub async fn checkout(
    executor: &GitExecutor,
    workdir: &Path,
    target: &CheckoutTarget,
    cancel: &CancellationToken,
) -> Result<CheckoutOutcome, GitError> {
    // Exit 1 is "not while that work is there" (the caller retries through
    // a stash); 128 stays a failure.
    let cmd = GitCommand::new()
        .cwd(workdir)
        .answers_by_code(1)
        .arg("switch");
    let cmd = match target {
        CheckoutTarget::Branch { name } => cmd.args(["--", name.as_str()]),
        CheckoutTarget::Track { remote_ref, local } => {
            cmd.args(["--create", local, "--track", remote_ref])
        }
        // `--no-track` is load-bearing: without it a remote-tracking start
        // silently overwrites the existing branch's upstream
        // (rules-refs/core.md, the `switch --force-create` line).
        CheckoutTarget::ForceCreate { local, start } => {
            cmd.args(["--no-track", "--force-create", local, start])
        }
    };
    match executor.run(cmd, cancel).await {
        Ok(_) => Ok(CheckoutOutcome::Moved),
        Err(refusal) => match &refusal {
            GitError::Failed { stderr, .. } if work_is_in_the_way(stderr) => {
                Ok(CheckoutOutcome::Blocked(refusal))
            }
            _ => Err(refusal),
        },
    }
}

/// What a reset does to the index and the working tree once the branch
/// itself has moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetMode {
    /// Branch only. Whatever the commits left behind changed stays in the
    /// index, on top of anything staged already.
    Soft,
    /// Branch and index. Every file keeps the content it has on disk;
    /// none of it is staged any more.
    Mixed,
    /// Branch, index and working tree. Uncommitted work is destroyed —
    /// git kept no copy of it to restore.
    Hard,
}

impl ResetMode {
    fn flag(self) -> &'static str {
        match self {
            ResetMode::Soft => "--soft",
            ResetMode::Mixed => "--mixed",
            ResetMode::Hard => "--hard",
        }
    }
}

/// Moves the ref HEAD is on (the current branch) to `rev`, an object id.
///
/// No `--` (to `reset` it opens the pathspec form, which takes no mode
/// flag) and no `--end-of-options` (the minimum git refuses it in `reset`
/// — see [`crate::version::MINIMUM_GIT`]). A caller with a *name* resolves it first
/// with `rev-parse --end-of-options`.
///
/// Not a way out of an operation in progress (the UI does not offer it):
/// mid-merge, `Soft` refuses and the other two silently drop `MERGE_HEAD`.
pub async fn reset(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    mode: ResetMode,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["reset", "--quiet", mode.flag(), rev]);
    executor.run(cmd, cancel).await.map(drop)
}

/// Creates a branch at `start_point` (HEAD when `None`), optionally
/// switching to it.
pub async fn create(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    start_point: Option<&str>,
    switch_to: bool,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new().cwd(workdir);
    let cmd = if switch_to {
        cmd.args(["switch", "--create", name])
    } else {
        cmd.args(["branch", "--", name])
    };
    let cmd = match start_point {
        Some(start) => cmd.arg(start),
        None => cmd,
    };
    executor.run(cmd, cancel).await.map(drop)
}

/// Deletes a local branch. `force` maps to `-D` (drops unmerged work);
/// without it git refuses to delete an unmerged branch itself.
///
/// Both spellings are the delete row's chip, so the menu and the command
/// log read the same (`-D` is what git's own refusal hint suggests).
pub async fn delete(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    force: bool,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let flag = if force { "-D" } else { "--delete" };
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["branch", flag, "--", name]);
    executor.run(cmd, cancel).await.map(drop)
}

/// Renames a local branch. `force` allows overwriting an existing name.
///
/// A refusal is a report, under a name box that stays open
/// ([`crate::report::rename_refused`]; デザイン規約 §答えの要らない報せ).
/// git moves the ref first and the branch's config section and the
/// worktrees' HEADs after it, dying on a failure there with the ref already
/// moved — so a failure is checked against the refs, whatever the message
/// says ([`crate::report::rename_unfinished`]).
pub async fn rename(
    executor: &GitExecutor,
    workdir: &Path,
    from: &str,
    to: &str,
    force: bool,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let flag = if force { "-M" } else { "-m" };
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["branch", flag, "--", from, to]);
    let command = cmd.describe();
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code == 0 {
        return Ok(());
    }
    let moved = !has_branch(executor, workdir, from, cancel).await?
        && has_branch(executor, workdir, to, cancel).await?;
    if moved {
        return Err(crate::report::rename_unfinished(to, command, &out));
    }
    Err(crate::report::rename_refused(from, command, &out))
}

/// Whether `refs/heads/<name>` is there (`rev-parse --verify` exit 1 = not).
pub(crate) async fn has_branch(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["rev-parse", "--verify", "--quiet", "--end-of-options"])
        .arg(format!("refs/heads/{name}"))
        .answers_by_code(1);
    Ok(executor.run_unchecked(cmd, cancel).await?.code == 0)
}

/// Records which remote branch a local one is measured against
/// (`branch.<name>.remote` / `.merge`).
///
/// `branch --set-upstream-to` when the remote-tracking ref is here; git
/// refuses a name it has no ref for, so otherwise the pair is written with
/// `config`, `.remote` first (rules-refs/core.md「upstream の書き込みは 2 通りで」;
/// デザイン規約 §ブランチが測られる相手を決める). A branch here as the
/// upstream (remote `.`) has no remote-tracking ref and goes the second way.
///
/// The flag takes the full refname: the `origin/main` shorthand is
/// ambiguous, and refused, when a local branch has that name.
///
/// A branch checked out in another worktree takes it (unlike delete).
pub async fn set_upstream(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    target: &crate::remote::RemoteBranch,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let upstream = format!("refs/remotes/{}/{}", target.remote, target.branch);
    if ref_is_here(executor, workdir, &upstream, cancel).await? {
        let cmd = GitCommand::new().cwd(workdir).args([
            "branch",
            &format!("--set-upstream-to={upstream}"),
            "--end-of-options",
            name,
        ]);
        return executor.run(cmd, cancel).await.map(drop);
    }
    for (key, value) in [
        (format!("branch.{name}.remote"), target.remote.clone()),
        (format!("branch.{name}.merge"), target.head_ref()),
    ] {
        let cmd = GitCommand::new()
            .cwd(workdir)
            .args(["config", "--local", &key, &value]);
        executor.run(cmd, cancel).await?;
    }
    Ok(())
}

/// Takes `branch.<name>.merge` and `.remote` out of this repository's own
/// config file — what a branch was measured against is gone. The caller
/// has read both keys there.
///
/// The keys by name, not `branch --unset-upstream <name>`: that resolves
/// `HEAD` to the branch checked out, so a `[branch "HEAD"]` section would
/// cost the current branch its upstream. git counts an upstream only with
/// both keys (`remote.c` `set_merge` drops a merge without a remote), so a
/// stop between the two leaves none either way. A key not in this file is
/// git's to refuse (`--unset-all` exits 5), and the write fails as it said.
/// `branch.<name>.pushRemote` stays: it names a remote, not a branch over
/// there.
pub async fn unset_upstream(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    for field in ["merge", "remote"] {
        let key = format!("branch.{name}.{field}");
        let cmd = GitCommand::new()
            .cwd(workdir)
            .args(["config", "--local", "--unset-all", &key]);
        executor.run(cmd, cancel).await?;
    }
    Ok(())
}

/// Whether this repository holds `full`, which decides the two paths of
/// [`set_upstream`].
async fn ref_is_here(
    executor: &GitExecutor,
    workdir: &Path,
    full: &str,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new().cwd(workdir).answers_by_code(1).args([
        "rev-parse",
        "--verify",
        "--quiet",
        "--end-of-options",
        full,
    ]);
    Ok(executor.run_unchecked(cmd, cancel).await?.code == 0)
}

/// True when every commit of `rev` is already reachable from `into`.
///
/// This is what makes deleting a branch safe; the UI asks before offering
/// the forced delete.
pub async fn is_merged_into(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    into: &str,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["merge-base", "--is-ancestor", rev, into])
        .answers_by_code(1);
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => Ok(true),
        1 => Ok(false),
        code => Err(GitError::Failed {
            command: format!("git merge-base --is-ancestor {rev} {into}"),
            code,
            stderr: out.stderr_utf8().trim().to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Messages git prints under LC_ALL=C (the integration tests prove the
    // installed git still says them).

    #[test]
    fn tracked_collisions_are_worth_another_go() {
        let stderr = "error: Your local changes to the following files would \
                      be overwritten by checkout:\n\ta.txt\nPlease commit your \
                      changes or stash them before you switch branches.\nAborting";
        assert!(work_is_in_the_way(stderr));
    }

    #[test]
    fn untracked_collisions_are_too() {
        let plain = "error: The following untracked working tree files would \
                     be overwritten by checkout:\n\tc.txt\nPlease move or remove \
                     them before you switch branches.\nAborting";
        let merging = "error: Untracked working tree file 'c.txt' would be \
                       overwritten by merge.";
        assert!(work_is_in_the_way(plain));
        assert!(
            work_is_in_the_way(merging),
            "the merging path names one file at a time"
        );
    }

    #[test]
    fn every_other_failure_stays_an_error() {
        for stderr in [
            "fatal: invalid reference: nope",
            "error: you need to resolve your current index first\na.txt: needs merge",
            "fatal: cannot continue with staged changes in the following files:\na.txt",
            "fatal: cannot switch branch while cherry-picking\nConsider \
             \"git cherry-pick --quit\" or \"git worktree add\".",
        ] {
            assert!(!work_is_in_the_way(stderr), "{stderr}");
        }
    }

    /// No `--` and no `--end-of-options` ([`reset`]).
    #[tokio::test]
    async fn a_reset_names_its_mode_and_the_revision_and_nothing_more() {
        let (exec, asked) = crate::refusing::git();
        let cancel = CancellationToken::new();
        for mode in [ResetMode::Soft, ResetMode::Mixed, ResetMode::Hard] {
            reset(
                &exec,
                &crate::refusing::nowhere(),
                "0123abcd",
                mode,
                &cancel,
            )
            .await
            .expect_err("there is no git here to reset with");
        }
        assert_eq!(
            asked.displays(),
            [
                "git reset --quiet --soft 0123abcd",
                "git reset --quiet --mixed 0123abcd",
                "git reset --quiet --hard 0123abcd",
            ]
        );
    }

    /// The start point, when there is one, comes after the name.
    #[tokio::test]
    async fn a_new_branch_is_made_with_branch_or_moved_onto_with_switch_create() {
        let (exec, asked) = crate::refusing::git();
        let cancel = CancellationToken::new();
        let workdir = crate::refusing::nowhere();
        for (start, switch_to) in [(None, false), (Some("0123abcd"), true)] {
            create(&exec, &workdir, "topic", start, switch_to, &cancel)
                .await
                .expect_err("there is no git here to branch with");
        }
        assert_eq!(
            asked.displays(),
            ["git branch -- topic", "git switch --create topic 0123abcd"]
        );
    }
}
