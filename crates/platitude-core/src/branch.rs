//! Checkout and local branch management.
//!
//! Uses `switch` throughout. `checkout` doubles as a file-restoring
//! command, so a branch whose name collides with a path is ambiguous;
//! `switch` only ever moves HEAD and says so in its errors. The
//! same split is why unstaging uses `restore` (see [`crate::stage`]).

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// What a checkout should land on.
///
/// Every variant lands on a branch. Nothing here detaches HEAD: a branch
/// is what the next commit needs somewhere to go, and the UI offers to
/// make one wherever a bare commit is what was pointed at (デザイン規約
/// §ブランチ・コミットへの移動). A HEAD already detached — left by git
/// itself, or by the command line — is read and worked from as normal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckoutTarget {
    /// An existing local branch.
    Branch { name: String },
    /// A remote-tracking branch: creates `local` tracking it and switches.
    Track { remote_ref: String, local: String },
    /// An existing local branch, moved to `start` before landing on it.
    ///
    /// Commits only that branch had are left unreferenced, so the UI asks
    /// before running this one. The move and the landing are one command
    /// (`switch --force-create`): git either does both or neither, and a
    /// working tree in the way still refuses the whole thing.
    ForceCreate { local: String, start: String },
}

/// What a move did.
#[derive(Debug)]
pub enum CheckoutOutcome {
    /// HEAD moved, carrying whatever uncommitted work did not stand in
    /// the way.
    Moved,
    /// git refused because uncommitted work stands in the way, and
    /// aborted before touching anything, so the repository is exactly as
    /// it was. Carries the refusal itself: the caller goes round again
    /// through a stash (`RepoSession::checkout`), and a second refusal —
    /// the tree is empty by then, so something git cannot see past is
    /// holding it — has to say why it gave up, in git's own words.
    Blocked(GitError),
}

/// Whether git's refusal is the everyday "your work is in the way" one,
/// which a stash gets past.
///
/// Classifying human-facing output is otherwise off limits here, and this
/// is the one place that earns the exception: git offers no
/// machine-readable answer to "why can I not move", and the answer decides
/// whether the move is worth a second attempt. Every invocation runs under
/// `LC_ALL=C`, so the C-locale wording is what arrives.
///
/// Anything unrecognised is `false` and travels on as an ordinary error: a
/// reworded message costs the retry only.
fn work_is_in_the_way(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    // "The following untracked working tree files would be overwritten by
    // checkout:", the singular "Untracked working tree file 'x' would be
    // overwritten by merge." a restore runs into, and "Your local changes
    // to the following files would be overwritten by checkout:".
    text.contains("untracked working tree file")
        || text.contains("would lose untracked files")
        || text.contains("would be overwritten by checkout")
}

/// Moves HEAD to `target`, taking uncommitted work along where git will
/// have it.
///
/// Carrying changes over a collision is done by stashing across the move
/// (`RepoSession::checkout`), which keeps both the staged/unstaged split
/// and a way back. `--merge` would three-way merge the changes in, but
/// it reports a conflicted result as a *success* with no merge left to
/// abort, and refuses to run at all while anything is
/// staged.
pub async fn checkout(
    executor: &GitExecutor,
    workdir: &Path,
    target: &CheckoutTarget,
    cancel: &CancellationToken,
) -> Result<CheckoutOutcome, GitError> {
    // Exit 1 is this command answering "not while that work is there",
    // which the caller acts on (it goes round through a stash). Only 0
    // and 1 count as answers, so the 128 a name git does not know exits
    // with still reads as the failure it is.
    let cmd = GitCommand::new()
        .cwd(workdir)
        .answers_by_code(1)
        .arg("switch");
    let cmd = match target {
        CheckoutTarget::Branch { name } => cmd.args(["--", name.as_str()]),
        CheckoutTarget::Track { remote_ref, local } => {
            cmd.args(["--create", local, "--track", remote_ref])
        }
        // **`--no-track`, and it is load-bearing.** Moving a branch says
        // nothing about what it reads, but `branch.autoSetupMerge` is on
        // by default and `--force-create` honours it for a
        // remote-tracking start point **even where the branch already
        // exists** — it overwrites `branch.<local>.remote` / `.merge`
        // with the start point (measured 2.55; `reset --hard` and
        // `branch -f` onto the same ref leave them alone). All the reflog
        // says is `branch: Reset to <start>`, so an upstream lost this
        // way is named nowhere — and that line is the only thing that
        // tells the move apart from a plain reset, which writes
        // `reset: moving to <start>` and keeps the upstream.
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

/// Moves the ref HEAD is on (the current branch) to `rev`.
///
/// `rev` is a commit, so it goes *before* any `--`: to `git reset` a
/// `--` opens the pathspec form, which takes no mode flag at all. It
/// carries no `--end-of-options` either, and that one is not
/// an oversight: the minimum git refuses the option here in *every*
/// position ("must come before non-option arguments", exit 128), alone
/// among the verbs this crate issues — branch, switch, tag, remote,
/// rev-parse, log and stash all take it (measured on 2.43, which is the
/// floor git最低バージョン整合.md sets). Nothing is lost, because what reaches this is an
/// object id off a graph row and an object id cannot read as an option.
/// A caller that ever wants to pass a *name* has to resolve it first
/// with `rev-parse --end-of-options`, since reset itself cannot say it.
///
/// Not a way out of an operation in progress, and the UI does not offer
/// it as one: mid-merge, `Soft` refuses outright ("Cannot do a soft reset
/// in the middle of a merge") while the other two drop `MERGE_HEAD`
/// without a word, abandoning the merge as a side effect (measured).
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
/// Both spellings are the ones the delete row wears as its chip, so the
/// menu, this call, and the command log read as the same words: the
/// long form for the everyday delete, and for the forced one the exact
/// spelling git's own refusal hint suggests.
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
/// **A refusal here is a report** (デザイン規約 §答えの要らない報せ):
/// nothing moved, git said why, and the box the name was typed into is
/// still open — so the answer belongs in it
/// ([`crate::report::ReportKind::RenameRefused`]).
/// The common one is a name that is already taken.
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
    match out.code {
        0 => Ok(()),
        _ => Err(crate::report::rename_refused(from, command, &out)),
    }
}

/// Records which remote branch a local one is measured against
/// (`branch.<name>.remote` / `.merge`).
///
/// **Two ways to write one pair of keys, and which one runs is whether
/// the ref is here.** `branch --set-upstream-to` is git's own, and it
/// refuses a name this repository holds no remote-tracking ref for
/// (`fatal: the requested upstream branch … does not exist`, measured —
/// git's own hint there points at `push -u`). A name not here is an
/// answer all the same: the branch is then measured against a remote
/// branch the next push makes, and nothing else can say so, so the pair
/// is written straight (デザイン規約 §ブランチが測られる相手を決める).
///
/// **The flag takes the full refname** (`refs/remotes/origin/main`). The
/// shorthand git prints and takes elsewhere is a rev-parse spelling, and
/// a local branch literally named `origin/main` makes it *ambiguous* —
/// git refuses the whole command (measured). The full form names one ref
/// and cannot be read two ways; what lands in the config is identical
/// either way.
///
/// **`.remote` goes down first.** A `.merge` standing on its own is read
/// against whatever remote git falls back to, where a `.remote` on its
/// own leaves the branch measured against nothing at all — so the half
/// a failed second write leaves behind is the harmless one.
///
/// Nothing about the working tree stands in its way: this is
/// configuration about a branch, so **a branch another working copy has
/// checked out takes it** (measured) — unlike the delete, which git
/// refuses there.
pub async fn set_upstream(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    remote: &str,
    remote_branch: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let upstream = format!("refs/remotes/{remote}/{remote_branch}");
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
        (format!("branch.{name}.remote"), remote.to_string()),
        (
            format!("branch.{name}.merge"),
            format!("refs/heads/{remote_branch}"),
        ),
    ] {
        let cmd = GitCommand::new()
            .cwd(workdir)
            .args(["config", "--local", &key, &value]);
        executor.run(cmd, cancel).await?;
    }
    Ok(())
}

/// Whether this repository holds `full`, which is what decides the two
/// halves of [`set_upstream`].
///
/// Exit 1 is the answer "no such ref", not a failure — left unmarked the
/// command log would raise itself over it (規約 §git が言ったことを読む場所).
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
        // Exit 1 here means "no", which is half of what this asks. Left
        // unmarked, the command log would read it as a failure and raise
        // itself over an answer.
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

    // The messages git actually prints under LC_ALL=C. The integration
    // tests prove the installed git still says them; these pin down which
    // ones are worth a second attempt through a stash.

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
        ] {
            assert!(!work_is_in_the_way(stderr), "{stderr}");
        }
    }
}
