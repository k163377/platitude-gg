//! What a non-zero push was: outdated, turned down here by the `pre-push`
//! hook, turned down over there and for what, or a plain failure.
//!
//! Kept apart from [`super::push`] because the tag commands read it too
//! ([`super::tags`]) and none of it looks at a [`super::PushSpec`].

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};
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
    pub(super) fn branch(remote: &'a str, name: &'a str, deleting: bool) -> Self {
        Self {
            remote,
            name,
            refname: format!("refs/heads/{name}"),
            deleting,
        }
    }

    pub(super) fn tag(remote: &'a str, name: &'a str, deleting: bool) -> Self {
        Self {
            remote,
            name,
            refname: format!("refs/tags/{name}"),
            deleting,
        }
    }
}

/// Which a non-zero push of a branch is: one that would drop the remote's
/// commits, leased to a reading the remote has left, turned down by the
/// `pre-push` hook here or by the far side, or a failure.
///
/// The refusals are told apart because only some have a next move (a
/// fetch answers `fetch first`, nothing answers a protected branch), and
/// the ones a fetch answers by what they found, so each reaches the screen
/// as its own report (デザイン規約 §答えの要らない報せ).
pub(super) async fn refusal(
    executor: &GitExecutor,
    workdir: &Path,
    command: String,
    out: &crate::process::GitOutput,
    sent: &Sent<'_>,
    cancel: &CancellationToken,
) -> GitError {
    let porcelain = out.stdout_utf8();
    let kind = match sent_line(&porcelain, &sent.refname) {
        Some(line) if line.rejected_for("(stale info)") => moved(sent.deleting),
        Some(line) if line.would_drop() => ReportKind::Outdated,
        _ => return refused(executor, workdir, command, out, sent, cancel).await,
    };
    outdated(kind, command, out, sent)
}

/// The same reading for a tag: a lease the remote has left, as for a
/// branch, and a plain push onto a name the remote holds on another commit
/// (`already exists`) — both answered by reading the remote's tags again
/// (`session::push_tag`), which a fetch does not do (`--prune` keeps the
/// local tag, `--prune-tags` exits 1).
pub(super) async fn tag_refusal(
    executor: &GitExecutor,
    workdir: &Path,
    command: String,
    out: &crate::process::GitOutput,
    sent: &Sent<'_>,
    cancel: &CancellationToken,
) -> GitError {
    let porcelain = out.stdout_utf8();
    let kind = match sent_line(&porcelain, &sent.refname) {
        Some(line) if line.rejected_for("(stale info)") => moved(sent.deleting),
        Some(line) if line.rejected_for("(already exists)") => ReportKind::TagElsewhere,
        _ => return refused(executor, workdir, command, out, sent, cancel).await,
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
    sent_line(porcelain, refname).is_some_and(|line| line.rejected_for("(already exists)"))
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

/// The reading for a refusal no read here answers: the `pre-push` hook
/// here, the far side, or a plain failure, which the log answers.
async fn refused(
    executor: &GitExecutor,
    workdir: &Path,
    command: String,
    out: &crate::process::GitOutput,
    sent: &Sent<'_>,
    cancel: &CancellationToken,
) -> GitError {
    let porcelain = out.stdout_utf8();
    let stderr = out.stderr_utf8();
    let kind = match sent_line(&porcelain, &sent.refname) {
        Some(line) if line.summary.starts_with(REMOTE_REJECTED) => {
            let why = remote_why(line.summary, &stderr);
            match sent.deleting {
                true => ReportKind::RemoteDelete(why),
                false => ReportKind::RemoteUpdate(why),
            }
        }
        None if hook_turned_down(executor, workdir, out, cancel).await => match sent.deleting {
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

/// How git marks a ref the far side turned down, against `[rejected]` for
/// one this end turned down itself, each with a next move here.
const REMOTE_REJECTED: &str = "[remote rejected]";

/// What the far side said no for: git's own reason in the brackets
/// (receive-pack's, the same words at the minimum and 2.55), and for the
/// two the brackets leave open its `remote:` lines. A hook's is whatever
/// it declined as — a forge names its own (`protected branch hook
/// declined`).
fn remote_why(summary: &str, stderr: &str) -> RemoteWhy {
    let why = bracket_reason(summary);
    let far_side = remote_lines(stderr);
    match why {
        _ if why.ends_with("hook declined") => RemoteWhy::Rule,
        "non-fast-forward" => RemoteWhy::Rewrite,
        "deletion prohibited" => RemoteWhy::NoDeletes,
        "branch is currently checked out" => RemoteWhy::CheckedOut,
        "deletion of the current branch prohibited" => RemoteWhy::Current,
        "shallow update not allowed" => RemoteWhy::Shallow,
        // index-pack's own words on the far side, which runs it in the
        // locale it was given — another language there reads as the rest.
        "unpacker error" if far_side.contains("pack exceeds maximum allowed size") => {
            RemoteWhy::TooLarge
        }
        // `failed to update ref` at the minimum, `reference already exists`
        // at 2.55: only its lines say it was a lock.
        _ if crate::report::lock_taken(&far_side) => RemoteWhy::Locked,
        _ => RemoteWhy::Other,
    }
}

/// The far side's own lines on stderr, git's `remote:` taken off — what
/// it said, as against what this end's git said.
fn remote_lines(stderr: &str) -> String {
    stderr
        .lines()
        .filter_map(|line| line.strip_prefix("remote:"))
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

/// Whether git answered the refspec itself, before any hook runs
/// (remote.c `match_explicit_lhs` / `match_explicit`, `LC_ALL=C`, the same
/// at the minimum and 2.55): a `src` / `dst refspec` that matches nothing
/// or more than one ref — the ref to send went from here since the press,
/// or a tag shares the branch's name — and a ref to delete the remote no
/// longer holds. Read off git's own `error:` lines, not a hook's words.
fn refspec_refused(stderr: &str) -> bool {
    stderr.lines().any(|line| {
        line.strip_prefix("error: ").is_some_and(|said| {
            said.starts_with("src refspec ")
                || said.starts_with("dst refspec ")
                || said.ends_with("remote ref does not exist")
        })
    })
}

/// Whether the `pre-push` hook here turned the push down. git runs it
/// before it talks to the remote and stops at its non-zero exit with
/// nothing of its own but `failed to push some refs` — no `To` line, no ref
/// line (transport.c `run_pre_push_hook`, then `goto done`). So: exit 1, no
/// line for the ref, none of git's refspec answers, and a hook there to
/// have run.
async fn hook_turned_down(
    executor: &GitExecutor,
    workdir: &Path,
    out: &crate::process::GitOutput,
    cancel: &CancellationToken,
) -> bool {
    out.code == 1
        && !refspec_refused(&out.stderr_utf8())
        && has_pre_push_hook(executor, workdir, cancel).await
}

/// Whether git has a `pre-push` hook to run here: the file it would run,
/// wherever `core.hooksPath` puts it — `rev-parse --git-path` follows that
/// setting (path.c `adjust_git_path`), relative to the worktree's top.
/// A failed read is no hook: the push's own failure then goes to the log.
async fn has_pre_push_hook(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> bool {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["rev-parse", "--git-path", "hooks/pre-push"]);
    let Ok(out) = executor.run(cmd, cancel).await else {
        return false;
    };
    runnable(&workdir.join(out.stdout_utf8().trim()))
}

/// Whether git would run the file at `path` as a hook: there, and on Unix
/// executable — git passes over one that is not, with a hint.
fn runnable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.is_file() && meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        meta.is_file()
    }
}

/// One ref line of a `--porcelain` push: `<flag>\t<from>:<to>\t<summary>`,
/// where `!` is a refusal and a delete's `<from>` is empty.
struct RefLine<'a> {
    flag: &'a str,
    summary: &'a str,
}

impl RefLine<'_> {
    /// Whether this end turned the ref down with `why` in its summary —
    /// `[rejected]` only: the far side's `[remote rejected]` can carry the
    /// same words (`receive.denyNonFastForwards` answers `non-fast-forward`)
    /// and is no reading of ours a fetch would put right.
    fn rejected_for(&self, why: &str) -> bool {
        self.flag == "!" && self.summary.starts_with("[rejected]") && self.summary.contains(why)
    }

    /// Whether this end turned the ref down because the remote holds
    /// commits the send would drop: `fetch first` (not here yet),
    /// `non-fast-forward` (here, but not in what was sent) and `remote ref
    /// updated since checkout` (a lease with no oid, under
    /// `push.useForceIfIncludes`). The other refusal a fetch answers,
    /// `stale info`, is a lease's and read apart ([`refusal`]).
    fn would_drop(&self) -> bool {
        [
            "(fetch first)",
            "(non-fast-forward)",
            "(remote ref updated since checkout)",
        ]
        .iter()
        .any(|why| self.rejected_for(why))
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
        assert!(line(REFUSED_STALE_LEASE).rejected_for("(stale info)"));
        assert!(is_taken(REFUSED_TAG_TAKEN, "refs/tags/v1"));
        assert!(sent_line(REFUSED_TAG_TAKEN, MAIN).is_none());
    }

    /// The far side's `non-fast-forward` is its own rule: no fetch here puts
    /// it right.
    #[test]
    fn the_far_sides_refusal_is_not_read_as_this_ends() {
        let far = line(FAR_SIDE_NON_FAST_FORWARD);
        assert!(!far.would_drop());
        assert!(far.summary.starts_with(REMOTE_REJECTED));
    }

    #[test]
    fn pushes_that_landed_are_not_refusals() {
        for landed in [FORCED_UPDATE, UP_TO_DATE, NEW_BRANCH] {
            let landed = line(landed);
            assert!(!landed.would_drop());
            assert!(!landed.rejected_for("(stale info)"));
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
