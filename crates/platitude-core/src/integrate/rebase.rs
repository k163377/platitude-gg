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
    /// (git 2.38+, inside the minimum version).
    pub update_refs: bool,
    /// `--root`: replay from the first commit; `upstream` is left off, as
    /// the first commit has no parent to name.
    pub root: bool,
}

/// What a rebase did; the plain one and the driven one in
/// [`crate::sequencer`] answer alike (git refuses both over a dirty tree in
/// the same words).
#[derive(Debug)]
pub enum RebaseOutcome {
    /// git took the rebase through to the end.
    Done,
    /// git refused before touching anything because uncommitted work is in
    /// the way. Carries the refusal: the caller retries through a stash
    /// (`RepoSession`'s carry), and a second refusal has to say why in
    /// git's own words.
    Blocked(GitError),
    /// git stopped part-way and left the rebase standing — not a failure;
    /// the carried work waits in the stash until the operation is over
    /// (デザイン規約 §未コミット変更がある状態で履歴を書き換える).
    /// Becomes [`super::Landing`] after the carry
    /// (`session::build::rewrite_carrying`).
    Stopped,
}

/// `git rebase <upstream>`.
///
/// A dirty tree is answered and the caller carries the work across itself:
/// `--autostash` restores staged work as unstaged
/// (デザイン規約 §未コミット変更がある状態で履歴を書き換える).
pub async fn rebase(
    executor: &GitExecutor,
    workdir: &Path,
    upstream: &str,
    options: &RebaseOptions,
    cancel: &CancellationToken,
) -> Result<RebaseOutcome, GitError> {
    // Exit 1 is an answer; 128 stays a failure
    // (rules/core.md「終了コードで答える問い合わせは」).
    let cmd = rebase_command(workdir, upstream, options, None).answers_by_code(1);
    let result = executor.run(cmd, cancel).await.map(drop);
    landed(executor, workdir, result, cancel).await
}

/// Sorts a rebase's result into [`RebaseOutcome`], shared by the plain
/// rebase above and the driven one in [`crate::sequencer`].
///
/// Exit 1 is both the refusal a stash gets past (told by its wording) and
/// the stop (told by `rebase-merge` left standing). The code is read as
/// well as the marker: a rebase asked for while another is in progress
/// exits 128 with `rebase-merge` there, and the marker alone would hide
/// git's reason. A read that fails answers "not a stop", so git's own
/// words reach the screen.
///
/// Exit 0 is not always Done: an `edit` stop exits 0 with the marker
/// standing (`an_edit_stop_says_so_and_names_the_commit_it_sits_on`).
pub(crate) async fn landed(
    executor: &GitExecutor,
    workdir: &Path,
    result: Result<(), GitError>,
    cancel: &CancellationToken,
) -> Result<RebaseOutcome, GitError> {
    let Err(error) = result else {
        // Unlike the exit-1 branch, a failed probe travels: "Done" makes
        // the caller sweep the reword message files a standing rebase's
        // todo still reads.
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
/// Reads human-facing output under the same exception [`crate::branch`]
/// takes for `switch`: git has no machine-readable "why will you not
/// rebase", and the answer decides whether a retry is worth it (`LC_ALL=C`
/// pins the wording). The two wordings are the halves of git's clean-tree
/// check, identical for plain and interactive rebases. Untracked files are
/// not in the way, so nothing is stashed for them.
///
/// Anything unrecognised is `false` and travels on as an ordinary error:
/// a reworded message costs the retry only.
fn work_is_in_the_way(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    text.contains("cannot rebase:")
        && (text.contains("unstaged changes") || text.contains("uncommitted changes"))
}

/// Why a standing rebase is standing: an `edit` stop leaves the tree as
/// clean as a stop over an emptied commit, and only this read tells them
/// apart (デザイン規約 §フル interactive rebase). Default where nothing is
/// standing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RebaseStop {
    /// Stopped on purpose at an `edit` step (HEAD on the applied commit, to
    /// be amended); git marks it by leaving `rebase-merge/amend`.
    pub editing: bool,
    /// The commit the stop left HEAD on, full hex — the amend marker's
    /// contents (git's `intend_to_amend`). Empty where the file could not
    /// be read.
    ///
    /// Not `rebase-merge/stopped-sha`: that names the todo's commit, which
    /// a reword, squash or reorder ahead of the `edit` step leaves out of
    /// the history
    /// (`an_edit_stop_after_a_reword_names_the_replayed_commit_not_the_todos`).
    pub oid: String,
}

/// Reads why the standing rebase stopped and how far it got, in one
/// process — it runs once per status tick for the life of a stop. Only
/// worth asking while [`opstate::detect`] says one is standing; with none,
/// everything comes back default.
///
/// Progress reads both backends; the stop's reason reads only the merge
/// side — the apply backend has no `edit` to stop at.
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
    ] {
        cmd = cmd.args(["--git-path", rel]);
    }
    let out = executor.run(cmd, cancel).await?;
    let text = out.stdout_utf8();
    // `--git-path` prints paths relative to the workdir or absolute;
    // joining handles both.
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
    // A marker there but unreadable still says `editing` — it errs toward
    // holding `--skip` back (`skip_is_free`).
    let amend = paths.get(4);
    let editing = amend.is_some_and(|p| p.exists());
    let oid = amend
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    Ok((progress, RebaseStop { editing, oid }))
}

/// The same count as [`rebase_standing`], read without a process — for a
/// replay still running, whose number the screen counts out several times
/// a second. `git_dir` is the work tree's own git directory as `repo::open`
/// resolved it (linked worktrees included), which is where its rebase
/// state lives.
///
/// `None` both for no standing rebase and for unreadable files: either way
/// there is nothing to count yet.
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
        // `--no-rebase-merges` pins off `rebase.rebaseMerges`: the editor
        // keeps the lines git put in (`sequencer::merge_todo`), so its
        // `label onto` / `reset onto` would travel into the plan and replay
        // a shape nobody composed. Driven form only — a plain rebase leaves
        // the config the person's own
        // (rules-refs/core.md「駆動 rebase だけ `--no-rebase-merges` で釘付け」).
        cmd = cmd
            .args(["--interactive", "--no-rebase-merges"])
            .env("GIT_SEQUENCE_EDITOR", editor);
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

    /// Word for word as git writes them; `integrate_integration` runs both
    /// for real, plain and interactive.
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

    #[test]
    fn only_the_driven_rebase_pins_rebase_merges_off() {
        let options = RebaseOptions::default();
        let driven =
            rebase_command(Path::new("repo"), "upstream", &options, Some("editor")).describe();
        assert!(driven.contains("--interactive"), "{driven}");
        assert!(driven.contains("--no-rebase-merges"), "{driven}");
        let plain = rebase_command(Path::new("repo"), "upstream", &options, None).describe();
        assert!(!plain.contains("rebase-merges"), "{plain}");
    }
}
