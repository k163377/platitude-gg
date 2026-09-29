//! What a write that did not happen has to say for itself, stated on the
//! screen as a report (デザイン規約 §答えの要らない報せ). A failure that has
//! one carries it beside git's whole message, which the command log holds.

/// Which report this is — what the screen says in its own words is chosen
/// from this alone (`Words.writeReported`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    /// The far side would not take a ref away.
    RemoteDelete,
    /// The far side would not move a ref to what was sent.
    RemoteUpdate,
    /// git would not send at all: the remote holds commits the send would
    /// drop (`fetch first`, `non-fast-forward`, `remote ref updated since
    /// checkout`). The session has already queued the fetch that shows
    /// them ([`crate::session::RepoSession::push`]).
    Outdated,
    /// A lease the remote has left (`stale info`): the ref — a branch or a
    /// tag — moved or went over there since the screen read it, so nothing
    /// pinned to that reading was sent. Caught up with as
    /// [`Self::Outdated`] is: a fetch for a branch, that remote's tags read
    /// again for a tag (`session::push_tag`).
    Moved,
    /// The same refusal for a delete.
    MovedDelete,
    /// A plain tag push onto a name the remote holds on another commit
    /// (`already exists` — on the same commit it is answered as sent,
    /// [`crate::remote::push_tag`]). That remote's tags are read again.
    TagElsewhere,
    /// A commit that was not made — always something outside saying no (a
    /// hook, a signing key, another git holding the index): what this
    /// application could answer is gated before the press.
    Commit,

    /// A part of a file that was not taken because the file changed since
    /// the selection (fingerprint mismatch, or the hunk is gone) — refused
    /// here before git was asked; the pane rereads the file on this answer.
    /// One kind per verb, since the heading says what did not happen.
    StaleStage,
    StaleUnstage,
    StaleDiscard,
    /// A part of a conflicted file, which has more than one old side and
    /// cannot be cut into a patch. The pane withholds the pieces; this is
    /// for a write that got here anyway.
    ConflictedPart,
    /// A rename git would not make (most often: the name is taken). Nothing
    /// moved and the box stays open, so the gesture goes on
    /// (デザイン規約 §答えの要らない報せ「下端の線は必ず状態色を着る」).
    RenameRefused,
    /// A rename that stopped between its two halves: the new name was
    /// made and the old one is still there. The one report about something
    /// half done, hence its state colour (デザイン規約 §状態「進行中で対処が要る」).
    HalfRenamed,
    /// A working copy `git worktree remove` would not take away — nothing
    /// was deleted. The reason is empty where it was uncommitted changes:
    /// git's words end in advice to force it, which the screen does not
    /// offer, so it writes its own (as [`Self::Outdated`]).
    WorktreeKept,
    /// A working copy git took off its list but could not empty: git goes
    /// on to drop its record after a folder it could not delete in full
    /// (a file held open is the usual cause on Windows). Half done, as
    /// [`Self::HalfRenamed`].
    WorktreeHalfRemoved,

    /// The five shapes a history cannot be rewritten in, found before a
    /// rebase is spawned ([`crate::sequencer::plan_edit`]). git is never
    /// asked, so there is no log row; one kind each because the reason is
    /// the sentence the screen picks (`Words.writeReportedWhy`).
    ///
    /// A merge inside the range: a plain interactive rebase would flatten it.
    RewriteAcrossMerge,
    /// A commit the current branch cannot see — the graph draws every
    /// branch, but a rebase rewrites only the current one.
    RewriteOffBranch,
    /// A fold with nothing to fold into: the commit is the first one.
    FoldFirstCommit,
    /// The commit under the range is not in this (shallow) clone; replaying
    /// from there would cut the branch off its history.
    RewriteUnfetchedBase,
    /// Every commit in the branch dropped at once: git would leave the
    /// branch on a made-up empty commit with no message, so this end stops
    /// first.
    DropAllCommits,

    /// The two the outside world makes after the plan was worked out — the
    /// premise going while the write is on its way
    /// (rules-refs/core.md「`Withheld` へ寄せたのは画面から届く 3 つだけ」).
    ///
    /// The branch moved after the plan was composed: a rebase silently
    /// drops what the fixed todo leaves out, so the replay is refused with
    /// nothing touched.
    RewriteTipMoved,
    /// A merge, cherry-pick, revert, rebase or bisect was standing when
    /// the rewrite reached git (typically started from a terminal). The
    /// band names the operation.
    RewriteWhileStanding,
}

impl ReportKind {
    /// Whether this end read the remote before it moved — the refusals a
    /// read of the remote answers, which the session makes right away
    /// (a fetch for a branch, the remote's tags for a tag).
    #[must_use]
    pub fn is_outdated(self) -> bool {
        matches!(
            self,
            Self::Outdated | Self::Moved | Self::MovedDelete | Self::TagElsewhere
        )
    }
}

/// One report: the two halves the screen is made of, and the names the
/// first half is written from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteReport {
    pub kind: ReportKind,
    /// The remote as the user named it (`origin`), where one was
    /// involved. Empty otherwise.
    pub remote: String,
    /// The ref the write was about, spelled as the screen spells it; empty
    /// for none.
    pub name: String,
    /// Whoever said no, in their own words with git's framing (`remote:`,
    /// `hint:`) taken off — carried word for word for the screen to quote.
    ///
    /// Empty when this end refused on its own: that is how the screen knows
    /// the sentence is its own to write (`Words.writeReportedWhy`,
    /// rules-refs/app-ui.md「Rust に文言を置かない」).
    pub reason: String,
}

/// A rename git would not make, in git's own words; `from` is the name the
/// row still carries, since nothing moved. git ran, so the command is named
/// and the log keeps its failed row (デザイン規約 §git が言ったことを読む場所).
#[must_use]
pub fn rename_refused(
    from: &str,
    command: String,
    out: &crate::process::GitOutput,
) -> crate::error::GitError {
    let said = out.failure_message();
    crate::error::GitError::Reported {
        command,
        code: out.code,
        stderr: said.clone(),
        report: Box::new(WriteReport::about(ReportKind::RenameRefused, from, said)),
    }
}

/// A rename that got as far as the new name and no further. No command of
/// its own ran, so the message carried for the log is the failed step's
/// error ([`GitError::Reported`]'s display).
#[must_use]
pub fn half_renamed(name: &str, from: crate::error::GitError) -> crate::error::GitError {
    crate::error::GitError::Reported {
        command: "rename".to_string(),
        code: 0,
        stderr: from.to_string(),
        report: Box::new(WriteReport::about(
            ReportKind::HalfRenamed,
            name,
            String::new(),
        )),
    }
}

/// What git says when changes stand in the way (`LC_ALL=C`; the same
/// words since `worktree remove` came in, 2.17).
const WORKTREE_UNCLEAN: &str = "contains modified or untracked files";

/// A working copy git did not take away, from its answer to
/// `worktree remove`. git's `die()` exits 128 before anything is deleted;
/// any other failure came after the folder's deletion began, and git drops
/// its record regardless (builtin/worktree.c `remove_worktree`).
#[must_use]
pub fn worktree_not_removed(
    name: &str,
    command: String,
    out: &crate::process::GitOutput,
) -> crate::error::GitError {
    let said = out.failure_message();
    let (kind, reason) = match out.code {
        128 if said.contains(WORKTREE_UNCLEAN) => (ReportKind::WorktreeKept, String::new()),
        128 => (ReportKind::WorktreeKept, said.clone()),
        _ => (ReportKind::WorktreeHalfRemoved, said.clone()),
    };
    crate::error::GitError::Reported {
        command,
        code: out.code,
        stderr: said,
        report: Box::new(WriteReport::about(kind, name, reason)),
    }
}

/// The seven rewrites this end turns down before git runs. There is no
/// command to name and no words to quote, so the report carries no reason
/// and the screen writes both lines (rules-refs/app-ui.md「Rust に文言を
/// 置かない」); `message` is only the log's record.
fn withheld(kind: ReportKind, message: String) -> crate::error::GitError {
    crate::error::GitError::Withheld {
        message,
        report: Box::new(WriteReport::local(kind, String::new())),
    }
}

/// A rewrite whose range holds a merge commit.
#[must_use]
pub fn rewrite_across_merge() -> crate::error::GitError {
    withheld(
        ReportKind::RewriteAcrossMerge,
        "this range contains a merge commit, which a rebase would drop".to_string(),
    )
}

/// A rewrite of a commit the current branch cannot see.
#[must_use]
pub fn rewrite_off_branch(oid: &str) -> crate::error::GitError {
    withheld(
        ReportKind::RewriteOffBranch,
        format!("{oid} is not in the history of the current branch"),
    )
}

/// A fold of the first commit, which has nothing before it.
#[must_use]
pub fn fold_first_commit(oid: &str) -> crate::error::GitError {
    withheld(
        ReportKind::FoldFirstCommit,
        format!("{oid} is the first commit, so it has nothing to fold into"),
    )
}

/// A rewrite whose range stands on a commit this clone does not have.
#[must_use]
pub fn rewrite_unfetched_base() -> crate::error::GitError {
    withheld(
        ReportKind::RewriteUnfetchedBase,
        "this clone does not have the commit below the range, so a rebase there \
         would cut the branch off from the rest of its history"
            .to_string(),
    )
}

/// A drop that would take the last commit with it.
#[must_use]
pub fn drop_all_commits() -> crate::error::GitError {
    withheld(
        ReportKind::DropAllCommits,
        "dropping every commit would leave the branch with no history".to_string(),
    )
}

/// A replay whose branch moved after the plan was worked out.
#[must_use]
pub fn rewrite_tip_moved() -> crate::error::GitError {
    withheld(
        ReportKind::RewriteTipMoved,
        "the branch tip moved after the plan was composed; nothing was rewritten".to_string(),
    )
}

/// A rewrite asked for while an operation was standing. `standing` is
/// git's own verb for it, which the log keeps; the screen names it from
/// the band instead ([`ReportKind::RewriteWhileStanding`]).
#[must_use]
pub fn rewrite_while_standing(standing: &str) -> crate::error::GitError {
    withheld(
        ReportKind::RewriteWhileStanding,
        format!("a {standing} is in progress here; nothing was rewritten"),
    )
}

impl WriteReport {
    /// A report about a ref on a remote.
    #[must_use]
    pub fn on_remote(kind: ReportKind, remote: &str, name: &str, reason: String) -> Self {
        Self {
            kind,
            remote: remote.to_string(),
            name: name.to_string(),
            reason,
        }
    }

    /// A report about something this repository did on its own.
    #[must_use]
    pub fn local(kind: ReportKind, reason: String) -> Self {
        Self {
            kind,
            remote: String::new(),
            name: String::new(),
            reason,
        }
    }

    /// The same, about one ref of its own; the heading is written from
    /// `name`.
    #[must_use]
    pub fn about(kind: ReportKind, name: &str, reason: String) -> Self {
        Self {
            kind,
            remote: String::new(),
            name: name.to_string(),
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The failed step is this application's own, so the screen writes the
    /// sentence (rules-refs/app-ui.md「Rust に文言を置かない」) and the step's
    /// words go only to the log.
    #[test]
    fn a_half_done_rename_names_the_ref_and_quotes_nobody() {
        let inner = crate::error::GitError::UnexpectedOutput {
            command: "git rev-parse --verify".to_string(),
            message: "the stash list moved while renaming".to_string(),
        };
        let err = half_renamed("stash@{0}", inner);

        let report = err.report().expect("a report");
        assert_eq!(report.kind, ReportKind::HalfRenamed);
        assert_eq!(report.name, "stash@{0}");
        assert!(report.reason.is_empty());
        assert!(
            err.to_string().contains("the stash list moved"),
            "the step's own words reach the log: {err}"
        );
    }

    /// Copied off a real run (git 2.55.0.windows.3, the copy git itself
    /// was running in): the entry went and the folder stayed. Past `die()`,
    /// so git's words are all there is to say which file held on.
    #[test]
    fn a_removal_that_failed_past_its_first_deletion_is_half_done() {
        let out = crate::process::GitOutput {
            code: 255,
            stdout: Vec::new(),
            stderr: b"error: failed to delete 'C:/t/wtrm/wt-cur': Permission denied\n".to_vec(),
        };
        let err = worktree_not_removed("wt-cur", "git worktree remove".to_string(), &out);

        let report = err.report().expect("a report");
        assert_eq!(report.kind, ReportKind::WorktreeHalfRemoved);
        assert_eq!(report.name, "wt-cur");
        assert_eq!(
            report.reason,
            "error: failed to delete 'C:/t/wtrm/wt-cur': Permission denied"
        );
    }

    /// The screen writes both lines from the kind; the log gets the
    /// sentence alone.
    #[test]
    fn a_withheld_rewrite_names_no_command_and_quotes_nobody() {
        let err = rewrite_off_branch("c74dfaf");

        let report = err.report().expect("a report");
        assert_eq!(report.kind, ReportKind::RewriteOffBranch);
        assert!(report.reason.is_empty());
        assert_eq!(
            err.to_string(),
            "c74dfaf is not in the history of the current branch",
            "the log holds the sentence and nothing about a command"
        );
    }
}
