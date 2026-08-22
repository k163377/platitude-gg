//! `git rebase`, and the refusal a stash gets past.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use super::opstate;
use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RebaseOptions {
    /// `--onto <newbase>`: transplant onto something other than `upstream`.
    pub onto: Option<String>,
    /// Which branch to rebase; HEAD when `None`.
    pub branch: Option<String>,
    /// Move refs that pointed into the rewritten range along with it
    /// (git 2.38+; the minimum supported version is well past that).
    pub update_refs: bool,
    /// `--root`: replay from the first commit, which has no parent to name
    /// as upstream. The `upstream` argument is left off entirely — editing
    /// the very first commit is impossible otherwise.
    pub root: bool,
}

/// What a rebase did — the plain one here and the driven one in
/// [`crate::sequencer`] answer alike, because git refuses both over a
/// dirty working tree in the very same words (実測).
#[derive(Debug)]
pub enum RebaseOutcome {
    /// git took the rebase through to the end.
    Done,
    /// git refused before touching anything, because uncommitted work is
    /// in the way, so the repository is exactly as it was. Carries the
    /// refusal itself: the caller goes round again through a stash
    /// (`RepoSession`'s carry), and a second refusal — the tree is empty
    /// by then, so something git cannot see past is holding it — has to
    /// say why it gave up, in git's own words.
    Blocked(GitError),
    /// git stopped part-way and left the rebase standing. Not a failure:
    /// the badge, the exit card and the conflicted rows are the whole of
    /// what happened, and the carried work waits in the stash until the
    /// operation is over (デザイン規約 §未コミット変更がある状態で履歴を
    /// 書き換える, 2026-08-22 ユーザー判断).
    ///
    /// [`super::Landing`] is what this becomes once the carry is behind
    /// it (`session::build::rewrite_carrying`) — a refusal cannot reach
    /// the screen, because going round through a stash is the answer to
    /// one.
    Stopped,
}

/// `git rebase <upstream>`.
///
/// `--autostash` is not passed, here or anywhere: it restores with a
/// plain `stash apply`, so everything that was staged comes back
/// unstaged, and no flag turns that off. A dirty tree is answered rather
/// than failed, and the caller carries the work across itself
/// (デザイン規約 §未コミット変更がある状態で履歴を書き換える).
pub async fn rebase(
    executor: &GitExecutor,
    workdir: &Path,
    upstream: &str,
    options: &RebaseOptions,
    cancel: &CancellationToken,
) -> Result<RebaseOutcome, GitError> {
    // Exit 1 is this command answering rather than failing. Only 0 and 1
    // are answers, so the 128 a name git does not know exits with still
    // reads as the failure it is (規約 §終了コードで答える問い合わせ).
    let cmd = rebase_command(workdir, upstream, options, None).answers_by_code(1);
    let result = executor.run(cmd, cancel).await.map(drop);
    landed(executor, workdir, result, cancel).await
}

/// Sorts a rebase's result into [`RebaseOutcome`], shared by the plain
/// rebase above and the driven one in [`crate::sequencer`].
///
/// **Exit 1 is the whole of what this command answers with**, and both
/// answers wear it: the refusal a stash gets past, and the stop that
/// leaves the rebase standing. The two are told apart by what is on
/// disk — a refusal touched nothing, a stop left `rebase-merge` behind
/// (実測 2.55).
///
/// **The code has to be read as well as the marker.** The one failure
/// that leaves `rebase-merge` standing is a rebase asked for while
/// another is already in progress, and git spends 128 on it (実測 2.55)
/// — asking the repository alone would report that as a stop and hide
/// the sentence telling the person what is actually there. Every other
/// failure exits 128 with nothing standing.
///
/// A read that fails answers "not a stop", so git's own words are what
/// reaches the screen: this is a question *about* that failure, and
/// letting it replace the answer would report a `rev-parse` where git
/// said why it would not rebase.
pub(crate) async fn landed(
    executor: &GitExecutor,
    workdir: &Path,
    result: Result<(), GitError>,
    cancel: &CancellationToken,
) -> Result<RebaseOutcome, GitError> {
    let Err(error) = result else {
        return Ok(RebaseOutcome::Done);
    };
    let GitError::Failed {
        command,
        code,
        stderr,
    } = error
    else {
        return Err(error);
    };
    if work_is_in_the_way(&stderr) {
        return Ok(RebaseOutcome::Blocked(GitError::Failed {
            command,
            code,
            stderr,
        }));
    }
    if code == 1
        && opstate::detect(executor, workdir, cancel)
            .await
            .is_ok_and(|state| state.rebasing)
    {
        return Ok(RebaseOutcome::Stopped);
    }
    Err(GitError::Failed {
        command,
        code,
        stderr,
    })
}

/// Whether git's refusal is the "commit or stash them" one it gives
/// before touching anything, which a stash gets past.
///
/// Classifying human-facing output is otherwise off limits here, and this
/// earns the same exception [`crate::branch`] takes for `switch`: git
/// offers no machine-readable answer to "why will you not rebase", and
/// the answer decides whether the rebase is worth a second attempt. Every
/// invocation runs under `LC_ALL=C`, so the C-locale wording arrives.
///
/// The two wordings are the halves of git's own clean-tree check —
/// "cannot rebase: You have unstaged changes." and "cannot rebase: Your
/// index contains uncommitted changes." — and a plain rebase and an
/// interactive one word them identically (実測 2.55, both in
/// `integrate_integration`). Untracked files are not in the way at all: a
/// rebase over a tree holding only those goes straight through, so
/// nothing is stashed for them.
///
/// Anything unrecognised is `false` and travels on as an ordinary error:
/// a reworded message costs the retry, never correctness.
fn work_is_in_the_way(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    text.contains("cannot rebase:")
        && (text.contains("unstaged changes") || text.contains("uncommitted changes"))
}

/// Builds a rebase command; `todo_editor` turns it into an interactive one
/// driven by that `GIT_SEQUENCE_EDITOR` (see [`crate::sequencer`]).
pub(crate) fn rebase_command(
    workdir: &Path,
    upstream: &str,
    options: &RebaseOptions,
    todo_editor: Option<&str>,
) -> GitCommand {
    let mut cmd = GitCommand::new().cwd(workdir).arg("rebase");
    if options.update_refs {
        cmd = cmd.arg("--update-refs");
    }
    if let Some(editor) = todo_editor {
        cmd = cmd.arg("--interactive").env("GIT_SEQUENCE_EDITOR", editor);
    }
    if let Some(onto) = &options.onto {
        cmd = cmd.args(["--onto", onto]);
    }
    if options.root {
        cmd = cmd.arg("--root");
    } else {
        cmd = cmd.args(["--", upstream]);
    }
    if let Some(branch) = &options.branch {
        cmd = cmd.arg(branch);
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both halves of git's clean-tree check, word for word as 2.55 wrote
    /// them; the integration tests run the real thing, against a plain
    /// rebase and an interactive one alike.
    #[test]
    fn the_two_refusals_a_stash_gets_past() {
        assert!(work_is_in_the_way(
            "error: cannot rebase: You have unstaged changes.\n\
             error: Please commit or stash them."
        ));
        assert!(work_is_in_the_way(
            "error: cannot rebase: Your index contains uncommitted changes.\n\
             error: Please commit or stash them."
        ));
    }

    /// Anything else is an ordinary failure: a stash would not help, and
    /// retrying would rebase over a repository git has already touched.
    #[test]
    fn a_stopped_rebase_is_not_a_refusal() {
        assert!(!work_is_in_the_way(
            "error: could not apply 36726c1... c3\n\
             hint: Resolve all conflicts manually"
        ));
        assert!(!work_is_in_the_way(
            "fatal: It seems that there is already a rebase-merge directory"
        ));
        assert!(!work_is_in_the_way(
            "error: cannot rebase onto multiple branches"
        ));
        assert!(!work_is_in_the_way(""));
    }
}
