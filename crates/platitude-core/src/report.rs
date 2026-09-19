//! What a write that did not happen has to say for itself.
//!
//! **A report is somebody else's no.** Something outside it said no under
//! a rule of its own — a protected branch, a repository rule, a
//! `pre-receive` hook over there, a `pre-commit` hook here, a signing key
//! that would not sign — or git worked out from what this end holds that
//! the write could not stand. Nothing was half done and there is nothing
//! here to put right, so the screen states it as a report
//! (デザイン規約 §答えの要らない報せ).
//!
//! Every failure that has one of these carries it beside git's whole
//! message, which goes on being what the command log holds.

/// Which report this is — what the screen says in its own words is chosen
/// from this alone (`Words.writeReported`).
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
    /// reader is still in the middle of the gesture
    /// (デザイン規約 §答えの要らない報せ の色の軸).
    RenameRefused,
    /// A rename that stopped between its two halves: the new name was
    /// made and the old one is still there.
    ///
    /// **The reports about something half done**, which is why they wear
    /// a state colour (デザイン規約 §状態 — 進行中で対処が要る).
    HalfRenamed,

    /// The five shapes a history cannot be rewritten in, worked out from
    /// the commits themselves before a rebase is ever spawned
    /// ([`crate::sequencer::plan_edit`]). **git is never asked**, so
    /// unlike every kind above there is no row in the log to read: what
    /// the reader is told is the whole of what happened.
    ///
    /// One kind each because the heading they share says only that
    /// nothing was rewritten, and the reason is the part worth knowing —
    /// five different reasons, five sentences the screen picks from
    /// (`Words.writeReportedWhy`).
    ///
    /// A merge inside the range a rewrite would replay. A plain
    /// interactive rebase drops merges, so it would come back flattened.
    RewriteAcrossMerge,
    /// A commit the current branch cannot see. Its row is on screen — the
    /// graph draws every branch — but a rebase only ever rewrites the one
    /// the tree is standing on.
    RewriteOffBranch,
    /// A fold with nothing to fold into: the commit is the first one.
    FoldFirstCommit,
    /// The commit under the range is not in this clone — a shallow one,
    /// where replaying from there would cut the branch off from the
    /// history it was made on.
    RewriteUnfetchedBase,
    /// Every commit in the branch dropped at once. git replays what is
    /// left onto a made-up empty commit, leaving the branch pointing at
    /// an empty tree with no message (実測), so this end stops first.
    DropAllCommits,

    /// **The two the outside world makes, after the plan was already
    /// worked out.** Unlike the five above — which are about the commits
    /// as they were read and so cannot come true between the press and
    /// the spawn — these are the premise going while the write is on its
    /// way, and a reader who did nothing wrong is the one who meets them
    /// (P3-確認事項 §要判断 `GitError::Rejected`).
    ///
    /// The branch moved after the plan was composed: the todo is a fixed
    /// list of ids and a rebase drops what it leaves out without a word,
    /// so the replay is refused with nothing touched.
    RewriteTipMoved,
    /// A merge, cherry-pick, revert, rebase or bisect was standing when
    /// the rewrite reached git — started from a terminal, most often,
    /// since the screen holds its own doors shut while one stands. The
    /// operation is named on the band.
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
    /// The ref the write was about — a branch or a tag, spelled the way
    /// the user's screen spells it. Empty where the write was about no
    /// ref at all.
    pub name: String,
    /// **Whoever said no, in their own words**, with the framing git puts
    /// in front of them taken off (`remote:`, `hint:`). Written by
    /// somebody else — a forge, a hook, git itself — so it is carried
    /// across word for word: the screen quotes it under a sentence of
    /// its own
    /// (デザイン規約 §長さ「詳しい事情は git の出力(コマンドログ)が言う」).
    ///
    /// Empty where nothing was said, which is a report of one line —
    /// **and that is how the screen tells the two apart**: this end's own
    /// refusals quote nobody, so the sentence under the heading is the
    /// UI's to write (`Words.writeReportedWhy`, app-ui.md「Rust に文言を
    /// 置かない」).
    pub reason: String,
}

/// A rename git would not make, said in git's own words.
///
/// Written in one place because both callers — a branch and a tag —
/// read exactly the same way: the name is the one the row still
/// carries, since nothing moved.
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
/// message carried for the log names the step ([`GitError::Reported`]'s
/// display).
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

/// The five rewrites this end turns down for itself, in the words the log
/// keeps.
///
/// **Nothing ran.** A plan is built out of what `rev-list` answered, and
/// these are the shapes it cannot make a todo list out of, so there is no
/// command to name and nobody else's words to quote: the report carries no
/// reason and the screen writes both of its lines
/// (`Words.writeReported` / `writeReportedWhy`, app-ui.md「Rust に文言を
/// 置かない」). The sentence here is the record the log holds, and the one
/// place these five are worded in this crate.
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
///
/// **A report, because a reader can be standing right in front of it**:
/// nothing here was pressed wrongly, and nothing ran that the command log
/// could show a row for — the bar is the only surface that can say what
/// happened (デザイン規約 §答えの要らない報せ).
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

    /// A rewrite turned down before git was asked quotes nobody and names
    /// no command: the log gets the sentence on its own, and the screen
    /// writes both of its lines from the kind.
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
