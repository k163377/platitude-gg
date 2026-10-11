//! What a write that did not happen has to say for itself, stated on the
//! screen as a report (デザイン規約 §答えの要らない報せ). A failure that has
//! one carries it beside git's whole message, which the command log holds.
//!
//! The screen writes both lines from the kind: what said no — a hook here,
//! a signing key, another git, a rule over there — in its own words, and
//! the words of whoever said it only under the command's row in the log.

/// Which report this is — what the screen says in its own words is chosen
/// from this alone (`Words.writeReported`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    /// The far side would not take a ref away.
    RemoteDelete(RemoteWhy),
    /// The far side would not move a ref to what was sent.
    RemoteUpdate(RemoteWhy),
    /// The `pre-push` hook here turned the send down: git ran it before
    /// talking to the remote, which heard nothing.
    PrePush,
    /// The same for a delete.
    PrePushDelete,
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
    /// A commit that was not made — always something outside saying no:
    /// what this application could answer is gated before the press. This
    /// one for what git said in words of its own that none of the three
    /// below name.
    Commit,
    /// A commit hook here (`pre-commit`, `commit-msg`,
    /// `prepare-commit-msg`) turned it down.
    CommitHook,
    /// git could not sign it: a key gone, a signing program that failed.
    CommitUnsigned,
    /// Another git process held a lock the commit needed.
    CommitLocked,

    /// A part of a file that was not taken because the file changed since
    /// the selection (fingerprint mismatch, or the hunk is gone) — refused
    /// here before git was asked, or after git's apply turned the patch
    /// down and a second read showed the file had moved
    /// (`stage::partial::apply_built`); the pane rereads the file on this
    /// answer. One kind per verb, since the heading says what
    /// did not happen.
    StaleStage,
    StaleUnstage,
    StaleDiscard,
    /// A part of a conflicted file, which has more than one old side and
    /// cannot be cut into a patch. The pane withholds the pieces; this is
    /// for a write that got here anyway.
    ConflictedPart,
    /// A rename git would not make. The name itself is answered before the
    /// press, so what reaches git is the rest — a name taken in the moment
    /// since, a branch being rebased or bisected. Nothing moved and the box
    /// stays open, so the gesture goes on
    /// (デザイン規約 §答えの要らない報せ「下端の線は必ず状態色を着る」).
    /// git's words are the log's; the screen says it in its own.
    RenameRefused,
    /// The same, because another git process held a lock it needed — at
    /// that moment, or one that crashed and left the lock file behind.
    RenameLocked,
    /// A branch rename git stopped after moving the ref: the new name is
    /// there and the old one gone, but what follows it — the branch's
    /// config section, another worktree's HEAD — did not move. `name` is the
    /// new name; the row under the old one is gone, so no box answers it.
    RenameUnfinished,
    /// A rename that stopped between its two halves: the new name was
    /// made and the old one is still there. The one report about something
    /// half done, hence its state colour (デザイン規約 §状態「進行中で対処が要る」).
    HalfRenamed,
    /// A stash git would not store under the new message, or whose entry
    /// could not be read again — nothing moved, and the box stays open as
    /// a branch's does ([`Self::RenameRefused`]). The heading names no
    /// entry: `stash@{n}` is not a name the screen shows.
    StashRenameRefused,
    /// The same, because another git process held the stash's lock.
    StashRenameLocked,
    /// The stash chosen left the list before anything was written — a
    /// drop or a pop elsewhere. Nothing to try again, so the box closes.
    StashGone,
    /// A worktree `git worktree remove` would not take away because of
    /// its uncommitted changes — nothing was deleted. git's words end in
    /// advice to force it, which the screen does not offer.
    WorktreeKept,
    /// The same, because it is locked; the report's fact is the reason it
    /// was locked with (`worktree lock --reason`), empty for none.
    WorktreeLocked,
    /// The same, for anything else git turned down (submodules in it, its
    /// `.git` not what git expects).
    WorktreeRemoveRefused,
    /// A worktree git took off its list but could not empty: git goes
    /// on to drop its record after a folder it could not delete in full
    /// (a file held open is the usual cause on Windows). Half done, as
    /// [`Self::HalfRenamed`].
    WorktreeHalfRemoved,
    /// A worktree `git worktree add` would not make — the branch out in
    /// another worktree or its name taken, between the menu opening and
    /// git: the menus answer each before the press. git takes back the
    /// worktree it began.
    WorktreeNotAdded,
    /// The same, because git still lists the place for another worktree
    /// whose folder is gone.
    WorktreePlaceTaken,
    /// The same, with the `-b` branch git made before turning it down
    /// left behind; the report's fact is that branch.
    WorktreeBranchLeft,
    /// A worktree git made whose `post-checkout` hook then failed: git
    /// keeps it (builtin/worktree.c — a hook's failure does not warrant
    /// its deletion), so it is there to use, perhaps without what the
    /// hook was to put in it (an LFS file's contents).
    WorktreeHookFailed,
    /// The folder a new worktree was to go in holds something — turned
    /// down here, before git, which would have made the branch first
    /// ([`crate::worktrees::add`]). Nothing was made, and there is nobody
    /// to quote.
    WorktreeFolderTaken,
    /// Another worktree's work not applied here because git said the apply
    /// would overwrite what is here: a file changed here that the work
    /// changes too, or an untracked one where it adds one. Nothing was
    /// written. `name` is that worktree's.
    CarriedInTheWay,
    /// The same, turned down before git: a file stands here — committed or
    /// not — where one of its untracked files would go. git's own apply,
    /// where the rest lands, would refuse that file only after it, and leave
    /// the rest applied.
    CarriedFileInTheWay,
    /// The same, because of what is there: that worktree went, has a
    /// conflict standing, or holds a file git would not take — found while
    /// it was read, before anything was applied.
    CarriedUnread,
    /// The same, turned down by git for anything else — untracked files here
    /// it would lose or remove rather than overwrite, a conflict standing
    /// here, an index another tool holds (about which the minimum's git says
    /// nothing). Its words, where it has any, are the log's.
    CarriedRefused,

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

    /// The three only git's run can tell — after it started, so its row is
    /// in the log with the words.
    ///
    /// The `pre-rebase` hook here turned the rewrite down; nothing moved.
    RewritePreRebase,
    /// A commit the replay remade could not be signed. The replay is put
    /// back where it started ([`crate::sequencer::rebase_interactive`]).
    RewriteUnsigned,
    /// A commit hook here turned down a commit the replay remade; put back
    /// the same way.
    RewriteCommitHook,
}

/// What the far side said no for, as far as its answer tells — the reason
/// git prints in brackets after `[remote rejected]`, and for two of them
/// the far side's own lines. Each is a sentence of the screen's; the far
/// side's words are the log's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteWhy {
    /// A hook over there declined it — `pre-receive`, `update`,
    /// `push-to-checkout`, a forge's protected branch: a rule it keeps.
    Rule,
    /// `receive.denyNonFastForwards`: it takes no push that drops commits
    /// it holds, forced or not (`git init --shared` turns it on).
    Rewrite,
    /// `receive.denyDeletes`: it deletes no branch.
    NoDeletes,
    /// `receive.denyCurrentBranch`: it has a working tree of its own, with
    /// the branch sent to checked out in it.
    CheckedOut,
    /// `receive.denyDeleteCurrent`: the branch its `HEAD` names.
    Current,
    /// More than it unpacks (`receive.maxInputSize`).
    TooLarge,
    /// A push from a shallow clone, which it does not take.
    Shallow,
    /// Another git process over there held the ref's lock.
    Locked,
    /// Anything else.
    Other,
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
    /// The one thing the screen's second line names beside the heading's
    /// names, for the two kinds that have one: the reason a worktree was
    /// locked with ([`ReportKind::WorktreeLocked`]) and the branch a failed
    /// add left ([`ReportKind::WorktreeBranchLeft`]). Empty for the rest.
    ///
    /// Never git's or anybody's message: the sentence is the screen's
    /// (`Words.writeReportedWhy`, rules-refs/app-ui.md「Rust に文言を置かない」),
    /// and what git said is under the command's row in the log
    /// (デザイン規約 §答えの要らない報せ).
    pub fact: String,
}

/// A report on a command git ran and turned down: the command and git's
/// words are the log's (デザイン規約 §git が言ったことを読む場所), the report
/// the screen's (§答えの要らない報せ).
#[must_use]
pub fn reported(
    report: WriteReport,
    command: String,
    out: &crate::process::GitOutput,
) -> crate::error::GitError {
    crate::error::GitError::Reported {
        command,
        code: out.code,
        stderr: out.failure_message(),
        report: Box::new(report),
    }
}

/// How git says a lock file it needed was there already (`LC_ALL=C`; the
/// same words in lockfile.c's `unable_to_lock_message` from the minimum to
/// 2.55, whatever advice follows them, and on a remote's `remote:` lines
/// too). The files backend's: a reftable repository's words fall to the
/// plain refusal.
const LOCK_TAKEN: &str = ".lock': File exists";

/// Whether git said another git held a lock it needed — at that moment,
/// or one that crashed and left the lock file behind.
#[must_use]
pub(crate) fn lock_taken(said: &str) -> bool {
    said.contains(LOCK_TAKEN)
}

/// A rename git would not make; `from` is the name the row still carries,
/// since nothing moved.
#[must_use]
pub fn rename_refused(
    from: &str,
    command: String,
    out: &crate::process::GitOutput,
) -> crate::error::GitError {
    let kind = if lock_taken(&out.stderr_utf8()) {
        ReportKind::RenameLocked
    } else {
        ReportKind::RenameRefused
    };
    reported(WriteReport::about(kind, from), command, out)
}

/// A branch rename git stopped after the ref moved
/// ([`ReportKind::RenameUnfinished`]); `to` is the name the branch has now.
#[must_use]
pub fn rename_unfinished(
    to: &str,
    command: String,
    out: &crate::process::GitOutput,
) -> crate::error::GitError {
    reported(
        WriteReport::about(ReportKind::RenameUnfinished, to),
        command,
        out,
    )
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
        report: Box::new(WriteReport::about(ReportKind::HalfRenamed, name)),
    }
}

/// A stash rename git turned down before the new entry went in — at its
/// `stash store`, or a read of the entry. `failed` is that step's error,
/// whose words the log has under its row.
#[must_use]
pub fn stash_not_renamed(failed: crate::error::GitError) -> crate::error::GitError {
    let crate::error::GitError::Failed {
        command,
        code,
        stderr,
    } = failed
    else {
        return failed;
    };
    let kind = if lock_taken(&stderr) {
        ReportKind::StashRenameLocked
    } else {
        ReportKind::StashRenameRefused
    };
    crate::error::GitError::Reported {
        command,
        code,
        stderr,
        report: Box::new(WriteReport::local(kind)),
    }
}

/// The stash chosen is no longer in the list; `oid` is only the log's
/// record. Nothing ran.
#[must_use]
pub fn stash_gone(oid: &str) -> crate::error::GitError {
    crate::error::GitError::Withheld {
        message: format!("the stash {oid} is no longer in the list"),
        report: Box::new(WriteReport::local(ReportKind::StashGone)),
    }
}

/// What git says when changes stand in the way (`LC_ALL=C`; the same
/// words since `worktree remove` came in, 2.17).
const WORKTREE_UNCLEAN: &str = "contains modified or untracked files";

/// A worktree git did not take away, from its answer to
/// `worktree remove`. git's `die()` exits 128 before anything is deleted;
/// any other failure came after the folder's deletion began, and git drops
/// its record regardless (builtin/worktree.c `remove_worktree`).
///
/// `locked` is the reason git said the worktree is locked with (empty for
/// none), where a lock is what it said no for — the first thing it checks.
#[must_use]
pub fn worktree_not_removed(
    name: &str,
    command: String,
    out: &crate::process::GitOutput,
    locked: Option<&str>,
) -> crate::error::GitError {
    let report = match (out.code, locked) {
        (128, Some(reason)) => {
            WriteReport::about(ReportKind::WorktreeLocked, name).with_fact(reason)
        }
        (128, None) if out.stderr_utf8().contains(WORKTREE_UNCLEAN) => {
            WriteReport::about(ReportKind::WorktreeKept, name)
        }
        (128, None) => WriteReport::about(ReportKind::WorktreeRemoveRefused, name),
        _ => WriteReport::about(ReportKind::WorktreeHalfRemoved, name),
    };
    reported(report, command, out)
}

/// What git says when the place is still listed for a worktree whose
/// folder is gone, locked or not (`LC_ALL=C`; builtin/worktree.c
/// `check_candidate_path`, the same words at the minimum and 2.55).
const WORKTREE_PLACE_LISTED: &str = "is a missing but ";

/// A worktree git did not make, from its answer to `worktree add`.
/// `left` is the `-b` branch git made before it turned the rest down, for
/// a branch that is there now and was not before.
#[must_use]
pub fn worktree_not_added(
    name: &str,
    command: String,
    out: &crate::process::GitOutput,
    left: Option<&str>,
) -> crate::error::GitError {
    let report = match left {
        Some(branch) => WriteReport::about(ReportKind::WorktreeBranchLeft, name).with_fact(branch),
        None if out.stderr_utf8().contains(WORKTREE_PLACE_LISTED) => {
            WriteReport::about(ReportKind::WorktreePlaceTaken, name)
        }
        None => WriteReport::about(ReportKind::WorktreeNotAdded, name),
    };
    reported(report, command, out)
}

/// A worktree git made, and then answered non-zero for: its
/// `post-checkout` hook failed ([`ReportKind::WorktreeHookFailed`]).
#[must_use]
pub fn worktree_hook_failed(
    name: &str,
    command: String,
    out: &crate::process::GitOutput,
) -> crate::error::GitError {
    reported(
        WriteReport::about(ReportKind::WorktreeHookFailed, name),
        command,
        out,
    )
}

/// A worktree turned down before git ran: its folder is not empty.
/// `path` is only the log's record.
#[must_use]
pub fn worktree_folder_taken(name: &str, path: &str) -> crate::error::GitError {
    crate::error::GitError::Withheld {
        message: format!("'{path}' is not empty"),
        report: Box::new(WriteReport::about(ReportKind::WorktreeFolderTaken, name)),
    }
}

/// What git says when an apply would write over work here, tracked or
/// untracked (`LC_ALL=C`; merge-ort's and unpack-trees' own words since
/// long before the minimum).
const CARRIED_OVERWRITES: &str = "would be overwritten";

/// Another worktree's work that `git stash apply` would not put here. The
/// screen writes both lines (what was in the way is all git's message
/// says, and it ends in advice to the terminal); the log keeps git's words.
/// Only the refusal that names work here as what it would overwrite says
/// so; any other is one git's words have to explain.
#[must_use]
pub fn carried_refused(
    name: &str,
    command: String,
    out: &crate::process::GitOutput,
) -> crate::error::GitError {
    let kind = if out.failure_message().contains(CARRIED_OVERWRITES) {
        ReportKind::CarriedInTheWay
    } else {
        ReportKind::CarriedRefused
    };
    reported(WriteReport::about(kind, name), command, out)
}

/// The same, turned down before git ran — `kind` says why (a file standing
/// here, or what was read there). `message` is only the log's record.
#[must_use]
pub fn carried_withheld(name: &str, kind: ReportKind, message: String) -> crate::error::GitError {
    crate::error::GitError::Withheld {
        message,
        report: Box::new(WriteReport::about(kind, name)),
    }
}

/// The seven rewrites this end turns down before git runs. There is no
/// command to name, and the screen writes both lines (rules-refs/app-ui.md
/// 「Rust に文言を置かない」); `message` is only the log's record.
fn withheld(kind: ReportKind, message: String) -> crate::error::GitError {
    crate::error::GitError::Withheld {
        message,
        report: Box::new(WriteReport::local(kind)),
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

/// A rewrite git started and something here turned down
/// ([`ReportKind::RewritePreRebase`], [`ReportKind::RewriteUnsigned`],
/// [`ReportKind::RewriteCommitHook`]) — from the rebase's failure, whose
/// words its row in the log keeps.
#[must_use]
pub fn rewrite_refused(
    kind: ReportKind,
    command: String,
    code: i32,
    stderr: String,
) -> crate::error::GitError {
    crate::error::GitError::Reported {
        command,
        code,
        stderr,
        report: Box::new(WriteReport::local(kind)),
    }
}

impl WriteReport {
    /// A report about a ref on a remote.
    #[must_use]
    pub fn on_remote(kind: ReportKind, remote: &str, name: &str) -> Self {
        Self {
            kind,
            remote: remote.to_string(),
            name: name.to_string(),
            fact: String::new(),
        }
    }

    /// A report about something this repository did on its own.
    #[must_use]
    pub fn local(kind: ReportKind) -> Self {
        Self {
            kind,
            remote: String::new(),
            name: String::new(),
            fact: String::new(),
        }
    }

    /// The same, about one ref of its own; the heading is written from
    /// `name`.
    #[must_use]
    pub fn about(kind: ReportKind, name: &str) -> Self {
        Self {
            kind,
            remote: String::new(),
            name: name.to_string(),
            fact: String::new(),
        }
    }

    /// The same, with the one thing its second line names ([`Self::fact`]).
    #[must_use]
    pub fn with_fact(self, fact: &str) -> Self {
        Self {
            fact: fact.to_string(),
            ..self
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
        assert!(report.fact.is_empty());
        assert!(
            err.to_string().contains("the stash list moved"),
            "the step's own words reach the log: {err}"
        );
    }

    fn out(code: i32, stderr: &str) -> crate::process::GitOutput {
        crate::process::GitOutput {
            code,
            stdout: Vec::new(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    /// Copied off a real run (git 2.55.0.windows.3, the worktree git itself
    /// was running in): the entry went and the folder stayed. Past `die()`;
    /// git names the worktree's own folder, never the file that held on
    /// (`remove_dir_recurse` puts the path back to its start before it
    /// returns), so the words are the log's like any other.
    #[test]
    fn a_removal_that_failed_past_its_first_deletion_is_half_done() {
        let said = "error: failed to delete 'C:/t/worktree-rm/worktree-cur': Permission denied\n";
        let err = worktree_not_removed(
            "worktree-cur",
            "git worktree remove".to_string(),
            &out(255, said),
            None,
        );

        let report = err.report().expect("a report");
        assert_eq!(report.kind, ReportKind::WorktreeHalfRemoved);
        assert_eq!(report.name, "worktree-cur");
        assert!(report.fact.is_empty());
        assert!(err.to_string().contains("Permission denied"), "{err}");
    }

    /// git's 128s before anything is deleted (builtin/worktree.c at the
    /// minimum and 2.55): the lock the listing shows, checked first, then
    /// changes, then the rest.
    #[test]
    fn a_removal_git_turned_down_is_told_by_the_lock_and_the_changes() {
        let removal = |said: &str, locked: Option<&str>| {
            let err = worktree_not_removed("wt", "git".to_string(), &out(128, said), locked);
            let report = err.report().expect("a report").clone();
            (report.kind, report.fact)
        };
        assert_eq!(
            removal(
                "fatal: cannot remove a locked working tree, lock reason: on a stick\n\
                 use 'remove -f -f' to override or unlock first",
                Some("on a stick")
            ),
            (ReportKind::WorktreeLocked, "on a stick".to_string())
        );
        assert_eq!(
            removal(
                "fatal: cannot remove a locked working tree;\n\
                 use 'remove -f -f' to override or unlock first",
                Some("")
            ),
            (ReportKind::WorktreeLocked, String::new())
        );
        assert_eq!(
            removal(
                "fatal: 'C:/t/wt' contains modified or untracked files, use --force to delete it",
                None
            ),
            (ReportKind::WorktreeKept, String::new())
        );
        assert_eq!(
            removal(
                "fatal: working trees containing submodules cannot be moved or removed",
                None
            ),
            (ReportKind::WorktreeRemoveRefused, String::new())
        );
    }

    /// git's words for a place still listed, locked or not, and a branch
    /// that is there afterwards.
    #[test]
    fn an_add_git_turned_down_says_what_it_left() {
        let add = |said: &str, left: Option<&str>| {
            let err = worktree_not_added("wt", "git".to_string(), &out(128, said), left);
            let report = err.report().expect("a report").clone();
            (report.kind, report.fact)
        };
        assert_eq!(
            add(
                "fatal: 'C:/t/wt' is a missing but already registered worktree;\n\
                 use 'add -f' to override, or 'prune' or 'remove' to clear",
                None
            )
            .0,
            ReportKind::WorktreePlaceTaken
        );
        assert_eq!(
            add(
                "fatal: 'C:/t/wt' is a missing but locked worktree;\n\
                 use 'add -f -f' to override, or 'unlock' and 'prune' or 'remove' to clear",
                None
            )
            .0,
            ReportKind::WorktreePlaceTaken
        );
        assert_eq!(
            add(
                "fatal: 'C:/t/wt' is a missing but already registered worktree;",
                Some("topic")
            ),
            (ReportKind::WorktreeBranchLeft, "topic".to_string()),
            "a branch left behind is what the line says"
        );
        assert_eq!(
            add(
                "fatal: 'topic' is already used by worktree at 'C:/t/other'",
                None
            )
            .0,
            ReportKind::WorktreeNotAdded
        );
    }

    /// A lock on the stash's ref or its log, against anything else git
    /// said; an error that is not git's failure goes through as it was.
    #[test]
    fn a_stash_rename_git_turned_down_is_told_by_its_lock() {
        let failed = |stderr: &str| crate::error::GitError::Failed {
            command: "git stash store".to_string(),
            code: 1,
            stderr: stderr.to_string(),
        };
        let kind = |err: crate::error::GitError| err.report().map(|report| report.kind);
        assert_eq!(
            kind(stash_not_renamed(failed(
                "error: update_ref failed for ref 'refs/stash': cannot lock ref 'refs/stash': \
                 Unable to create 'C:/t/.git/refs/stash.lock': File exists."
            ))),
            Some(ReportKind::StashRenameLocked)
        );
        assert_eq!(
            kind(stash_not_renamed(failed(
                "fatal: log for 'stash' only has 1 entries"
            ))),
            Some(ReportKind::StashRenameRefused)
        );
        let other = crate::error::GitError::Rejected {
            message: "could not read stash@{0}".to_string(),
        };
        assert_eq!(kind(stash_not_renamed(other)), None);
    }

    /// The screen writes both lines from the kind; the log gets the
    /// sentence alone.
    #[test]
    fn a_withheld_rewrite_names_no_command_and_quotes_nobody() {
        let err = rewrite_off_branch("c74dfaf");

        let report = err.report().expect("a report");
        assert_eq!(report.kind, ReportKind::RewriteOffBranch);
        assert!(report.fact.is_empty());
        assert_eq!(
            err.to_string(),
            "c74dfaf is not in the history of the current branch",
            "the log holds the sentence and nothing about a command"
        );
    }

    /// git's own refusals of a `stash apply` (unpack-trees at the minimum):
    /// the two that would overwrite work here say so; one that would lose or
    /// remove untracked files, and the minimum's silence over an index lock,
    /// are left to git's words.
    #[test]
    fn a_carried_refusal_says_overwritten_only_where_git_does() {
        let kind = |stderr: &str| {
            let out = crate::process::GitOutput {
                code: 1,
                stdout: Vec::new(),
                stderr: stderr.as_bytes().to_vec(),
            };
            let err = carried_refused("topic", "git stash apply".to_string(), &out);
            err.report().expect("a report").kind
        };
        assert_eq!(
            kind(
                "error: Your local changes to the following files would be overwritten by merge:\n\ta.txt\n"
            ),
            ReportKind::CarriedInTheWay
        );
        assert_eq!(
            kind(
                "error: The following untracked working tree files would be overwritten by merge:\n\tn.txt\n"
            ),
            ReportKind::CarriedInTheWay
        );
        assert_eq!(
            kind(
                "error: The following untracked working tree files would be removed by merge:\n\tx\n"
            ),
            ReportKind::CarriedRefused
        );
        assert_eq!(kind(""), ReportKind::CarriedRefused);
    }
}
