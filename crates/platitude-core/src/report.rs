//! What a write that did not happen has to say for itself.
//!
//! **A report is not an error of this application's.** Something outside
//! it said no under a rule of its own — a protected branch, a repository
//! rule, a `pre-receive` hook over there, a `pre-commit` hook here, a
//! signing key that would not sign — or git worked out from what this end
//! holds that the write could not stand. Nothing was half done and there
//! is nothing here to put right, so the screen states it rather than
//! raising git's words as a failure (デザイン規約 §答えの要らない報せ).
//!
//! Every failure that has one of these carries it beside git's whole
//! message, which goes on being what the command log holds.

/// Which report this is — what the screen says in its own words is chosen
/// from this, and never from git's wording (`Words.writeReported`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    /// The far side would not take a ref away.
    RemoteDelete,
    /// The far side would not move a ref to what was sent.
    RemoteUpdate,
    /// git would not send at all: what this end holds about the remote is
    /// older than the remote itself (`fetch first`, `stale info`,
    /// `non-fast-forward`).
    ///
    /// **The one kind with a move behind it** — the fetch that shows what
    /// the remote really holds is queued by the session
    /// ([`crate::session::RepoSession::push`]), so by the time this is
    /// read the answer is already on its way.
    Outdated,
    /// A commit that was not made. Everything a commit can be refused for
    /// that this application could answer is taken away before the button
    /// is pressed (an identity, a message, something staged), so what is
    /// left is always something outside it saying no: a hook, a signing
    /// key, another git holding the index.
    Commit,

    /// A part of a file that was not taken, because the file is not the
    /// one the selection was made on any more (デザイン規約 §答えの要らない報せ).
    ///
    /// **This end is the one that said no**, before git was asked: the
    /// bytes carry a fingerprint and it did not match, or the hunk the
    /// selection indexes is no longer in the diff. Nothing was written
    /// either way, and the next move is already made — the pane reads the
    /// file again on this answer — so it is a report like the rest.
    ///
    /// Three of them because the heading says what did not happen and
    /// these are three different things not happening.
    StaleStage,
    StaleUnstage,
    StaleDiscard,
    /// A part of a conflicted file, which has more than one old side and
    /// so cannot be cut into a patch that applies. The pane withholds the
    /// pieces, so nothing should ask — this is what a write that got here
    /// anyway says for itself.
    ConflictedPart,
    /// A rename git would not make — most often because the new name is
    /// already taken. **Nothing moved and the box is still open**, so the
    /// reader is in the middle of the gesture rather than past it
    /// (デザイン規約 §答えの要らない報せ の色の軸).
    RenameRefused,
    /// A rename that stopped between its two halves: the new name was
    /// made and the old one is still there.
    ///
    /// **The reports about something half done**, which is why they wear
    /// a state colour (デザイン規約 §状態 — 進行中で対処が要る).
    HalfRenamed,
}

/// One report: the two halves the screen is made of, and the names the
/// first half is written from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteReport {
    pub kind: ReportKind,
    /// The remote as the user named it (`origin`), where one was
    /// involved. Empty otherwise.
    pub remote: String,
    /// The ref the write was about — a branch or a tag, spelled the way
    /// the user's screen spells it. Empty where the write was about no
    /// ref at all.
    pub name: String,
    /// **Whoever said no, in their own words**, with the framing git puts
    /// in front of them taken off (`remote:`, `hint:`). Written by
    /// somebody else — a forge, a hook, git itself — so it is carried
    /// across rather than interpreted: the screen quotes it under a
    /// sentence of its own, and never rewrites it
    /// (デザイン規約 §長さ「詳しい事情は git の出力に出ているので UI 文言で代弁しない」).
    ///
    /// Empty where nothing was said, which is a report of one line —
    /// **and that is how the screen tells the two apart**: this end's own
    /// refusals quote nobody, so the sentence under the heading is the
    /// UI's to write (`Words.writeReportedWhy`, app-ui.md「Rust に文言を
    /// 置かない」).
    pub reason: String,
}

/// The prefix git puts in front of its own advice, which the words under
/// a report are taken out from behind
/// ([`crate::remote::push`] reads it; the far side's is `remote:`).
pub const HINT_PREFIX: &str = "hint:";

/// A rename git would not make, said in git's own words.
///
/// Written here rather than in each caller because both of them — a
/// branch and a tag — read exactly the same way: the name is the one the
/// row still carries, since nothing moved.
///
/// **git ran**, so the command is named and the log keeps its row under a
/// red edge the way it does for any other refusal — what changes is only
/// where the reader is told (デザイン規約 §git が言ったことを読む場所).
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

/// A rename that got as far as the new name and no further.
///
/// **Nothing here ran a command of its own** — what failed is one step of
/// several, and its own error is what the log already holds — so the
/// message carried for the log names the step rather than pretending to a
/// command line ([`GitError::Reported`]'s display).
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

    /// The same, about one ref of its own — a rename knows the name it
    /// was about, and the heading is written from it.
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

    /// The half-done rename says what it is about and quotes nobody: what
    /// failed is a step of this application's own making, and the sentence
    /// under the heading belongs in the UI's language
    /// (app-ui.md「Rust に文言を置かない」). What the step said goes to the log
    /// instead, which is the one place it is any use.
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
}
