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
    /// Empty where nothing was said, which is a report of one line.
    pub reason: String,
}

/// The prefix git puts in front of its own advice, which the words under
/// a report are taken out from behind
/// ([`crate::remote::push`] reads it; the far side's is `remote:`).
pub const HINT_PREFIX: &str = "hint:";

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
}
