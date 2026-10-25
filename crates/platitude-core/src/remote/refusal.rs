//! What a non-zero push was: outdated, turned down here by the `pre-push`
//! hook, turned down over there and for what, or a plain failure.
//!
//! Kept apart from [`super::push`] because the tag commands read it too
//! ([`super::tags`]) and none of it looks at a [`super::PushSpec`].

use crate::error::GitError;
use crate::relied_messages::{self, Words};
use crate::report::{RemoteWhy, ReportKind, WriteReport};

/// The one ref a push sent, whose line in git's answer is the one read:
/// every push here sends one (`--no-follow-tags`, an explicit refspec), so
/// a line for another ref is not this push's to answer for.
pub(super) struct Sent<'a> {
    /// The remote as the user named it.
    pub remote: &'a str,
    /// The branch or tag as the screen names it.
    pub name: &'a str,
    /// Its full name over there (`refs/heads/main`, `refs/tags/v1`).
    pub refname: String,
    pub deleting: bool,
}

impl<'a> Sent<'a> {
    pub(super) fn branch(target: &'a super::RemoteBranch, deleting: bool) -> Self {
        Self {
            remote: &target.remote,
            name: &target.branch,
            refname: target.head_ref(),
            deleting,
        }
    }

    pub(super) fn tag(target: &'a super::RemoteTag, deleting: bool) -> Self {
        Self {
            remote: &target.remote,
            name: &target.tag,
            refname: target.tag_ref(),
            deleting,
        }
    }
}

/// Which a non-zero push of a branch is: one that would drop the remote's
/// commits, leased to a reading the remote has left, or one no read here
/// answers ([`refused`]).
///
/// The refusals are told apart because only some have a next move (a
/// fetch answers `fetch first`, nothing answers a protected branch), and
/// the ones a fetch answers by what they found, so each reaches the screen
/// as its own report (デザイン規約 §答えの要らない報せ).
pub(super) fn refusal(
    command: String,
    out: &crate::process::GitOutput,
    sent: &Sent<'_>,
) -> GitError {
    let porcelain = out.stdout_utf8();
    let kind = match sent_line(&porcelain, &sent.refname) {
        Some(line) if line.rejected_for(relied_messages::STALE_INFO) => moved(sent.deleting),
        Some(line) if line.would_drop() => ReportKind::Outdated,
        _ => return refused(command, out, sent),
    };
    outdated(kind, command, out, sent)
}

/// The same reading for a tag: a lease the remote has left, as for a
/// branch, and a plain push onto a name the remote holds on another commit
/// (`already exists`) — both answered by reading the remote's tags again
/// (`session::push_tag`), which a fetch does not do (`--prune` keeps the
/// local tag, `--prune-tags` exits 1).
pub(super) fn tag_refusal(
    command: String,
    out: &crate::process::GitOutput,
    sent: &Sent<'_>,
) -> GitError {
    let porcelain = out.stdout_utf8();
    let kind = match sent_line(&porcelain, &sent.refname) {
        Some(line) if line.rejected_for(relied_messages::STALE_INFO) => moved(sent.deleting),
        Some(line) if line.rejected_for(relied_messages::ALREADY_EXISTS) => {
            ReportKind::TagElsewhere
        }
        _ => return refused(command, out, sent),
    };
    outdated(kind, command, out, sent)
}

/// A lease the remote has left, sending or deleting.
fn moved(deleting: bool) -> ReportKind {
    match deleting {
        true => ReportKind::MovedDelete,
        false => ReportKind::Moved,
    }
}

/// Whether a plain tag push was refused for a name the remote already
/// holds on something else (`already exists`).
pub(super) fn is_taken(porcelain: &str, refname: &str) -> bool {
    sent_line(porcelain, refname)
        .is_some_and(|line| line.rejected_for(relied_messages::ALREADY_EXISTS))
}

/// Nothing is quoted: the far side never saw this push, and git's terminal
/// advice adds only the cause, which the screen writes itself
/// (`Words.writeReportedWhy`). The full text is in the log.
fn outdated(
    kind: ReportKind,
    command: String,
    out: &crate::process::GitOutput,
    sent: &Sent<'_>,
) -> GitError {
    crate::report::reported(
        WriteReport::on_remote(kind, sent.remote, sent.name),
        command,
        out,
    )
}

/// The reading for a refusal no read here answers: the far side by its ref
/// line; before any, the `pre-push` hook here; or a plain failure, which
/// the log answers. A signed push (`push.gpgSign`) git could not sign, or
/// that the far side takes none of, is the plain failure: the setting is
/// rare and the forges take no signed push, so nothing names it
/// (gitの扱い.md §類型).
fn refused(command: String, out: &crate::process::GitOutput, sent: &Sent<'_>) -> GitError {
    let porcelain = out.stdout_utf8();
    let stderr = out.stderr_utf8();
    let kind = match sent_line(&porcelain, &sent.refname) {
        Some(line)
            if line
                .summary
                .starts_with(relied_messages::REMOTE_REJECTED.text()) =>
        {
            let why = remote_why(line.summary, &stderr);
            match sent.deleting {
                true => ReportKind::RemoteDelete(why),
                false => ReportKind::RemoteUpdate(why),
            }
        }
        // git runs the hook before it signs or talks to the remote, and
        // stops at its non-zero end with nothing of its own but `failed to
        // push some refs` (transport.c): its trace has the end. Whatever
        // the hook printed is the hook's.
        None if out.trace.hook_failed(&["pre-push"]) => match sent.deleting {
            true => ReportKind::PrePushDelete,
            false => ReportKind::PrePush,
        },
        _ => {
            return GitError::Failed {
                command,
                code: out.code,
                stderr: out.failure_message(),
            };
        }
    };
    crate::report::reported(
        WriteReport::on_remote(kind, sent.remote, sent.name),
        command,
        out,
    )
}

/// What the far side said no for: git's own reason in the brackets
/// (receive-pack's), and for the two the brackets leave open its `remote:`
/// lines. A hook's is whatever it declined as — a forge names its own
/// (`protected branch hook declined`).
fn remote_why(summary: &str, stderr: &str) -> RemoteWhy {
    let why = bracket_reason(summary);
    let far_side = remote_lines(stderr);
    if why.ends_with(relied_messages::HOOK_DECLINED.text()) {
        RemoteWhy::Rule
    } else if relied_messages::FAR_NOT_A_FAST_FORWARD.is(why) {
        RemoteWhy::Rewrite
    } else if relied_messages::DELETION_PROHIBITED.is(why) {
        RemoteWhy::NoDeletes
    } else if relied_messages::CHECKED_OUT_THERE.is(why) {
        RemoteWhy::CheckedOut
    } else if relied_messages::CURRENT_DELETION_PROHIBITED.is(why) {
        RemoteWhy::Current
    } else if relied_messages::SHALLOW_REFUSED.is(why) {
        RemoteWhy::Shallow
    } else if relied_messages::UNPACKER_ERROR.is(why)
        && relied_messages::PACK_TOO_LARGE.found_in(&far_side)
    {
        // index-pack's own words on the far side, which runs it in the
        // locale it was given — another language there reads as the rest.
        RemoteWhy::TooLarge
    } else if crate::report::lock_taken(&far_side) {
        // `failed to update ref` at the minimum, `reference already exists`
        // at 2.55: only its lines say it was a lock.
        RemoteWhy::Locked
    } else {
        RemoteWhy::Other
    }
}

/// The far side's own lines on stderr, git's `remote:` taken off — what
/// it said, as against what this end's git said.
fn remote_lines(stderr: &str) -> String {
    stderr
        .lines()
        .filter_map(|line| line.strip_prefix(relied_messages::REMOTE_LEAD.text()))
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
}

/// git's reason out of `[remote rejected] (deletion prohibited)` — what is
/// inside the brackets, empty where there are none.
fn bracket_reason(summary: &str) -> &str {
    summary
        .split_once('(')
        .and_then(|(_, rest)| rest.rsplit_once(')'))
        .map_or("", |(inner, _)| inner.trim())
}

/// One ref line of a `--porcelain` push: `<flag>\t<from>:<to>\t<summary>`,
/// where `!` is a refusal and a delete's `<from>` is empty.
struct RefLine<'a> {
    flag: &'a str,
    summary: &'a str,
}

impl RefLine<'_> {
    /// Whether this end turned the ref down — `[rejected]` only: the far
    /// side's `[remote rejected]` can carry the same words
    /// (`receive.denyNonFastForwards` answers `non-fast-forward`) and is no
    /// reading of ours a fetch would put right.
    fn rejected(&self) -> bool {
        self.flag == "!" && self.summary.starts_with(relied_messages::REJECTED.text())
    }

    /// [`Self::rejected`], with `why` in its summary.
    fn rejected_for(&self, why: Words) -> bool {
        self.rejected() && why.found_in(self.summary)
    }

    /// Whether this end turned the ref down because the remote holds
    /// commits the send would drop: `fetch first` (not here yet),
    /// `non-fast-forward` (here, but not in what was sent) and `remote ref
    /// updated since checkout` (a lease with no oid, under
    /// `push.useForceIfIncludes`). The other refusal a fetch answers,
    /// `stale info`, is a lease's and read apart ([`refusal`]).
    fn would_drop(&self) -> bool {
        self.rejected() && relied_messages::WOULD_DROP.found_in(self.summary)
    }
}

/// The line for `refname` in a `--porcelain` push's answer, if git wrote
/// one.
fn sent_line<'a>(porcelain: &'a str, refname: &str) -> Option<RefLine<'a>> {
    porcelain.lines().find_map(|line| {
        let mut fields = line.split('\t');
        let flag = fields.next()?;
        let (_, to) = fields.next()?.rsplit_once(':')?;
        let summary = fields.next()?;
        (to == refname).then_some(RefLine { flag, summary })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: &str = "refs/heads/main";

    fn line<'a>(porcelain: &'a str) -> RefLine<'a> {
        sent_line(porcelain, MAIN).expect("a line for main")
    }

    /// `--porcelain` output recorded from git 2.51 pushing to a local bare
    /// repository.
    const REFUSED_FETCH_FIRST: &str = "To C:/tmp/remote.git\n\
         !\trefs/heads/main:refs/heads/main\t[rejected] (fetch first)\nDone\n";
    const REFUSED_STALE_LEASE: &str = "To C:/tmp/remote.git\n\
         !\trefs/heads/main:refs/heads/main\t[rejected] (stale info)\nDone\n";
    const FORCED_UPDATE: &str = "To C:/tmp/remote.git\n\
         +\trefs/heads/main:refs/heads/main\t34158f1...6e03ce6 (forced update)\nDone\n";
    const UP_TO_DATE: &str = "To C:/tmp/remote.git\n\
         =\trefs/heads/main:refs/heads/main\t[up to date]\nDone\n";
    const NEW_BRANCH: &str = "To C:/tmp/remote.git\n\
         *\trefs/heads/main:refs/heads/main\t[new branch]\nDone\n";

    /// git 2.55: the remote's tip is here but not in what was sent, and a
    /// plain tag push onto a name the remote holds on something else.
    const REFUSED_NON_FAST_FORWARD: &str = "To C:/tmp/remote.git\n\
         !\trefs/heads/main:refs/heads/main\t[rejected] (non-fast-forward)\nDone\n";
    const REFUSED_TAG_TAKEN: &str = "To C:/tmp/remote.git\n\
         !\trefs/tags/v1:refs/tags/v1\t[rejected] (already exists)\nDone\n";

    /// git 2.55 against a bare repository with `receive.denyNonFastForwards`:
    /// the far side's refusal in the same words as this end's.
    const FAR_SIDE_NON_FAST_FORWARD: &str = "To ../remote.git\n\
         !\trefs/heads/main:refs/heads/main\t[remote rejected] (non-fast-forward)\nDone\n";

    #[test]
    fn a_refusal_a_fetch_would_answer_is_recognised_by_what_it_found() {
        assert!(line(REFUSED_FETCH_FIRST).would_drop());
        assert!(line(REFUSED_NON_FAST_FORWARD).would_drop());
        assert!(
            !line(REFUSED_STALE_LEASE).would_drop(),
            "a lease the remote left says the ref moved, not that commits would go"
        );
        assert!(line(REFUSED_STALE_LEASE).rejected_for(relied_messages::STALE_INFO));
        assert!(is_taken(REFUSED_TAG_TAKEN, "refs/tags/v1"));
        assert!(sent_line(REFUSED_TAG_TAKEN, MAIN).is_none());
    }

    /// The far side's `non-fast-forward` is its own rule: no fetch here puts
    /// it right.
    #[test]
    fn the_far_sides_refusal_is_not_read_as_this_ends() {
        let far = line(FAR_SIDE_NON_FAST_FORWARD);
        assert!(!far.would_drop());
        assert!(
            far.summary
                .starts_with(relied_messages::REMOTE_REJECTED.text())
        );
    }

    #[test]
    fn pushes_that_landed_are_not_refusals() {
        for landed in [FORCED_UPDATE, UP_TO_DATE, NEW_BRANCH] {
            let landed = line(landed);
            assert!(!landed.would_drop());
            assert!(!landed.rejected_for(relied_messages::STALE_INFO));
        }
    }

    /// Only the line for the ref sent is read: one for another ref — a tag
    /// a `remote.<name>.push` or `push.followTags` added — answers for
    /// nothing here.
    #[test]
    fn a_line_for_another_ref_is_not_this_pushs() {
        // A fast-forward's flag is a space, which the line break would eat.
        let answer = "To C:/tmp/remote.git\n\
             \x20\trefs/heads/main:refs/heads/main\t1111111..2222222\n\
             !\trefs/tags/v1:refs/tags/v1\t[rejected] (already exists)\nDone\n";
        assert_eq!(line(answer).flag, " ");
        assert!(!is_taken(answer, "refs/tags/v2"));
        let deleted = "To C:/tmp/remote.git\n\
             !\t:refs/heads/side\t[remote rejected] (deletion prohibited)\nDone\n";
        assert_eq!(
            sent_line(deleted, "refs/heads/side").map(|line| line.summary),
            Some("[remote rejected] (deletion prohibited)")
        );
    }

    /// git 2.55's answers against bare and non-bare repositories (the
    /// brackets and `remote:` lines as recorded), and GitHub's for a
    /// protected branch.
    #[test]
    fn the_far_sides_reasons_are_told_apart() {
        let why = |summary: &str, stderr: &str| remote_why(summary, stderr);
        assert_eq!(
            why(
                "[remote rejected] (pre-receive hook declined)",
                "remote: rule: no pushes today"
            ),
            RemoteWhy::Rule
        );
        assert_eq!(
            why(
                "[remote rejected] (hook declined)",
                "remote: error: hook declined to update"
            ),
            RemoteWhy::Rule
        );
        assert_eq!(
            why(
                "[remote rejected] (protected branch hook declined)",
                "remote: error: GH006: Protected branch update failed for refs/heads/main."
            ),
            RemoteWhy::Rule
        );
        assert_eq!(
            why(
                "[remote rejected] (non-fast-forward)",
                "remote: error: denying non-fast-forward refs/heads/main (you should pull first)"
            ),
            RemoteWhy::Rewrite
        );
        assert_eq!(
            why("[remote rejected] (deletion prohibited)", ""),
            RemoteWhy::NoDeletes
        );
        assert_eq!(
            why("[remote rejected] (branch is currently checked out)", ""),
            RemoteWhy::CheckedOut
        );
        assert_eq!(
            why(
                "[remote rejected] (deletion of the current branch prohibited)",
                ""
            ),
            RemoteWhy::Current
        );
        assert_eq!(
            why("[remote rejected] (shallow update not allowed)", ""),
            RemoteWhy::Shallow
        );
        assert_eq!(
            why(
                "[remote rejected] (unpacker error)",
                "remote: fatal: pack exceeds maximum allowed size        \n\
                 error: remote unpack failed: unpack-objects abnormal exit"
            ),
            RemoteWhy::TooLarge
        );
        assert_eq!(
            why(
                "[remote rejected] (unpacker error)",
                "error: remote unpack failed: index-pack abnormal exit"
            ),
            RemoteWhy::Other,
            "an unpack that failed for anything else"
        );
        let locked = "remote: error: cannot lock ref 'refs/heads/main': Unable to create \
             'C:/t/remote.git/./refs/heads/main.lock': File exists.        \n\
             remote: \nremote: Another git process seems to be running in this repository";
        assert_eq!(
            why("[remote rejected] (reference already exists)", locked),
            RemoteWhy::Locked
        );
        assert_eq!(
            why("[remote rejected] (failed to update ref)", locked),
            RemoteWhy::Locked,
            "the minimum's words for the same"
        );
        assert_eq!(
            why("[remote rejected] (deny updating a hidden ref)", ""),
            RemoteWhy::Other
        );
    }
}
