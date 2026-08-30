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
/// dirty working tree in the very same words (measured).
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
    /// 書き換える, by design).
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
/// (measured, 2.55).
///
/// **The code has to be read as well as the marker.** The one failure
/// that leaves `rebase-merge` standing is a rebase asked for while
/// another is already in progress, and git spends 128 on it (measured, 2.55)
/// — asking the repository alone would report that as a stop and hide
/// the sentence telling the person what is actually there. Every other
/// failure exits 128 with nothing standing.
///
/// A read that fails answers "not a stop", so git's own words are what
/// reaches the screen: this is a question *about* that failure, and
/// letting it replace the answer would report a `rev-parse` where git
/// said why it would not rebase.
///
/// **Exit 0 is not the whole of Done either.** The `edit` stop is the one
/// stop git exits 0 on — the pause was asked for, so git does not count
/// it against the command (measured, 2.55 —
/// `an_edit_stop_says_so_and_names_the_commit_it_sits_on`). The marker
/// left standing is what tells it from a rebase that ran out the end.
pub(crate) async fn landed(
    executor: &GitExecutor,
    workdir: &Path,
    result: Result<(), GitError>,
    cancel: &CancellationToken,
) -> Result<RebaseOutcome, GitError> {
    let Err(error) = result else {
        // The probe's own failure travels, asymmetrically from the exit-1
        // branch below: there git's words are the answer and a failed read
        // must not replace them, while here "Done" has consequences of its
        // own — the caller sweeps the reword message files a standing
        // rebase's todo still reads — so a stop must never be missed
        // quietly. A loud error costs a red line; the next status poll
        // still finds the standing rebase and raises the exit card.
        return match opstate::detect(executor, workdir, cancel).await {
            Ok(state) if state.rebasing => Ok(RebaseOutcome::Stopped),
            Ok(_) => Ok(RebaseOutcome::Done),
            Err(probe) => Err(probe),
        };
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
/// interactive one word them identically (measured 2.55, both in
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

/// Why a standing rebase is standing, where git wrote it down.
///
/// A stop at an `edit` step leaves the tree as clean as a stop over an
/// emptied commit, so the tree cannot tell the two apart — this read is
/// what can (P3-確認事項 §A). Default everywhere nothing is standing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RebaseStop {
    /// The rebase stopped on purpose at an `edit` step: the commit is
    /// applied, HEAD sits on it, and amending it is what the stop is for.
    /// git says so by leaving `rebase-merge/amend` behind — the file its
    /// own `--continue` reads to know the commit may have been amended.
    pub editing: bool,
    /// The commit the stop is about (`rebase-merge/stopped-sha`),
    /// abbreviated as git wrote it; empty where it wrote none.
    pub oid: String,
}

/// Reads why the standing rebase stopped, and how far it got, in one
/// process. Only worth asking while [`opstate::detect`] says one is
/// standing; with none, everything here comes back default — and the
/// caller pays the spawn once per status tick for the life of a stop,
/// which is why the two questions share it.
///
/// Progress reads both backends (the merge backend counts in
/// `rebase-merge/msgnum`, the apply backend in `rebase-apply/next`); the
/// stop's reason reads only the merge side — the apply backend has no
/// `edit` to stop at, and a conflicted stop is already told by the tree.
pub async fn rebase_standing(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<(Option<crate::conflict::Progress>, RebaseStop), GitError> {
    let mut cmd = GitCommand::new().cwd(workdir).arg("rev-parse");
    for rel in [
        "rebase-merge/msgnum",
        "rebase-merge/end",
        "rebase-apply/next",
        "rebase-apply/last",
        "rebase-merge/amend",
        "rebase-merge/stopped-sha",
    ] {
        cmd = cmd.args(["--git-path", rel]);
    }
    let out = executor.run(cmd, cancel).await?;
    let text = out.stdout_utf8();
    // `--git-path` prints paths relative to the cwd (the workdir) or
    // absolute ones; joining handles both (the shape `opstate::detect` uses).
    let paths: Vec<std::path::PathBuf> = text
        .lines()
        .map(|rel| workdir.join(rel.trim_end()))
        .collect();
    let count = |at: usize| -> Option<u32> {
        std::fs::read_to_string(paths.get(at)?)
            .ok()?
            .trim()
            .parse()
            .ok()
    };
    let mut progress = None;
    for pair in [(0, 1), (2, 3)] {
        if let (Some(current), Some(total)) = (count(pair.0), count(pair.1))
            && total > 0
        {
            progress = Some(crate::conflict::Progress { current, total });
            break;
        }
    }
    let editing = paths.get(4).is_some_and(|p| p.exists());
    let oid = paths
        .get(5)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    Ok((progress, RebaseStop { editing, oid }))
}

/// The same count, read without a process.
///
/// [`rebase_standing`] spends a `rev-parse` to learn where the four files
/// are, because it is also asking two questions that only make sense
/// together and it runs once per status tick. **A replay that is still
/// running is the other case**: the number moves every few milliseconds
/// and the screen is meant to count it out, so the read has to be cheap
/// enough to repeat several times a second — and it is, because the one
/// thing `rev-parse` was answering is already known. `repo::open` resolves
/// the git directory once, linked worktrees included, and the rebase state
/// of a work tree lives under that work tree's own git directory.
///
/// Both backends again, and `None` for "no rebase is standing" as well as
/// for one whose files cannot be read: the caller is asking about a write
/// it started, and either answer means there is nothing to count yet.
#[must_use]
pub fn rebase_progress(git_dir: &Path) -> Option<crate::conflict::Progress> {
    let count = |rel: &str| -> Option<u32> {
        std::fs::read_to_string(git_dir.join(rel))
            .ok()?
            .trim()
            .parse()
            .ok()
    };
    for (current, total) in [
        ("rebase-merge/msgnum", "rebase-merge/end"),
        ("rebase-apply/next", "rebase-apply/last"),
    ] {
        if let (Some(current), Some(total)) = (count(current), count(total))
            && total > 0
        {
            return Some(crate::conflict::Progress { current, total });
        }
    }
    None
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
