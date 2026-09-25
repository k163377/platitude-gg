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
    /// git would not send at all: what this end holds about the remote is
    /// older than the remote (`fetch first`, `stale info`,
    /// `non-fast-forward`). The session has already queued the fetch that
    /// corrects it ([`crate::session::RepoSession::push`]).
    Outdated,
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
