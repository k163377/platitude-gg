//! What a non-zero push was: outdated, turned down over there, or a
//! plain failure — and the far side's own words.
//!
//! Kept apart from [`super::push`] because the tag commands read it too
//! ([`super::tags`]) and none of it looks at a [`super::PushSpec`].

use crate::error::GitError;
use crate::report::{ReportKind, WriteReport};

/// Which of the three a non-zero push of a branch is.
///
/// The two refusals are told apart because only one has a next move (a
/// fetch answers `fetch first`, nothing answers a protected branch), so
/// they reach the screen as different reports (デザイン規約 §答えの要らない報せ).
pub(super) fn refusal(
    command: String,
    out: &crate::process::GitOutput,
    remote: &str,
    branch: &str,
    deleting: bool,
) -> GitError {
    if is_outdated(&out.stdout_utf8()) {
        return GitError::Reported {
            command,
            code: out.code,
            stderr: out.failure_message(),
            report: Box::new(WriteReport::on_remote(
                ReportKind::Outdated,
                remote,
                branch,
                // Nothing is quoted: the far side never saw this push, and
                // git's terminal advice adds only the cause, which the
                // screen writes itself (`Words.writeReportedWhy`). The full
                // text is in the log.
                String::new(),
            )),
        };
    }
    refused(command, out, remote, branch, deleting)
}

/// The same reading for a push no fetch can answer — a tag, whose local
/// name a fetch leaves where it is (`remote::tags`): a far-side refusal
/// or a plain failure.
pub(super) fn refused(
    command: String,
    out: &crate::process::GitOutput,
    remote: &str,
    name: &str,
    deleting: bool,
) -> GitError {
    let stderr = out.failure_message();
    let Some(reason) = refused_reason(&out.stdout_utf8(), &out.stderr_utf8()) else {
        return GitError::Failed {
            command,
            code: out.code,
            stderr,
        };
    };
    let kind = match deleting {
        true => ReportKind::RemoteDelete,
        false => ReportKind::RemoteUpdate,
    };
    GitError::Reported {
        command,
        code: out.code,
        stderr,
        report: Box::new(WriteReport::on_remote(kind, remote, name, reason)),
    }
}

/// Whether the far side is the one that said no, and how it explained
/// itself.
///
/// Only `[remote rejected]` counts: `[rejected]` is a refusal this end
/// worked out, each with a next move here. An unreachable host prints no
/// ref lines, so `None`.
///
/// The explanation is the far side's `remote:` lines, or git's own
/// parenthetical where it wrote none (a bare `receive-pack` refusing a
/// deletion says nothing else).
fn refused_reason(porcelain: &str, stderr: &str) -> Option<String> {
    let mut summary = None;
    for line in porcelain.lines() {
        let mut fields = line.split('\t');
        if fields.next() != Some("!") {
            continue;
        }
        summary = fields.nth(1);
        if summary.is_some_and(|said| said.contains("[remote rejected]")) {
            break;
        }
        summary = None;
    }
    let summary = summary?;
    let words = remote_words(stderr);
    Some(if words.is_empty() {
        bracket_reason(summary)
    } else {
        words
    })
}

/// What the far side said for itself, out of git's `remote:` lines on
/// stderr.
///
/// A leading `error:` is dropped: the report's heading already says what
/// did not happen, and a second failure word makes it look like the
/// application's fault.
///
/// The lines are joined into one, since a report is a heading and one
/// line (デザイン規約 §長さ); which line carries the rule this end cannot
/// know.
fn remote_words(stderr: &str) -> String {
    let mut said: Vec<&str> = Vec::new();
    for line in stderr.lines() {
        let Some(rest) = line.trim_end().strip_prefix("remote:") else {
            continue;
        };
        let rest = rest.trim();
        let rest = rest
            .strip_prefix("error:")
            .or_else(|| rest.strip_prefix("ERROR:"))
            .unwrap_or(rest)
            .trim();
        if !rest.is_empty() {
            said.push(rest);
        }
    }
    said.join(" ")
}

/// git's own reason out of `[remote rejected] (deletion prohibited)` —
/// what is inside the brackets, or the whole summary where there are
/// none to read.
fn bracket_reason(summary: &str) -> String {
    let inner = summary
        .split_once('(')
        .and_then(|(_, rest)| rest.rsplit_once(')'))
        .map(|(inner, _)| inner.trim());
    match inner {
        Some(inner) if !inner.is_empty() => inner.to_string(),
        _ => summary.trim().to_string(),
    }
}

/// Whether a `--porcelain` push result refused a ref for knowing the remote
/// only as it used to be.
///
/// The lines are `<flag>\t<from>:<to>\t<summary>`, where `!` is a refusal.
/// Three summaries are answered by fetching: `fetch first`,
/// `non-fast-forward` (a ref that is not the current branch's upstream),
/// and `stale info` (a lease pinned to a commit the remote has left).
fn is_outdated(porcelain: &str) -> bool {
    porcelain.lines().any(|line| {
        let mut fields = line.split('\t');
        fields.next() == Some("!")
            && fields.nth(1).is_some_and(|summary| {
                summary.contains("(fetch first)")
                    || summary.contains("(stale info)")
                    || summary.contains("(non-fast-forward)")
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
         *\trefs/heads/side:refs/heads/side\t[new branch]\nDone\n";

    #[test]
    fn a_refusal_a_fetch_would_answer_is_recognised() {
        assert!(is_outdated(REFUSED_FETCH_FIRST));
        assert!(is_outdated(REFUSED_STALE_LEASE));
    }

    #[test]
    fn pushes_that_landed_are_not_refusals() {
        assert!(!is_outdated(FORCED_UPDATE));
        assert!(!is_outdated(UP_TO_DATE));
        assert!(!is_outdated(NEW_BRANCH));
    }

    #[test]
    fn refusals_a_fetch_cannot_help_with_are_left_alone() {
        assert!(!is_outdated(REFUSED_BY_THE_FAR_SIDE));
        assert!(!is_outdated(""));
    }

    /// The same git 2.51 run with a `pre-receive` hook exiting non-zero,
    /// and the `remote: error:` lines GitHub answers a protected-branch
    /// deletion with.
    const REFUSED_BY_THE_FAR_SIDE: &str = "To C:/tmp/remote.git\n\
         !\trefs/heads/main:refs/heads/main\t[remote rejected] (pre-receive hook declined)\nDone\n";
    const FAR_SIDE_WORDS: &str = "remote: error: GH006: Protected branch update failed for \
         refs/heads/main.        \nremote: error: Cannot delete a protected branch        \n\
         remote: \nTo https://github.com/owner/repo.git\n \
         ! [remote rejected] main (protected branch hook declined)\n\
         error: failed to push some refs to 'https://github.com/owner/repo.git'\n";

    #[test]
    fn the_far_sides_own_words_are_what_a_refusal_carries() {
        assert_eq!(
            refused_reason(REFUSED_BY_THE_FAR_SIDE, FAR_SIDE_WORDS).as_deref(),
            Some(
                "GH006: Protected branch update failed for refs/heads/main. \
                 Cannot delete a protected branch"
            )
        );
    }

    #[test]
    fn a_silent_far_side_leaves_gits_parenthetical() {
        assert_eq!(
            refused_reason(REFUSED_BY_THE_FAR_SIDE, "To C:/tmp/remote.git\n").as_deref(),
            Some("pre-receive hook declined")
        );
    }

    #[test]
    fn a_refusal_from_this_end_is_not_the_far_side_speaking() {
        assert_eq!(refused_reason(REFUSED_FETCH_FIRST, FAR_SIDE_WORDS), None);
        assert_eq!(refused_reason(REFUSED_STALE_LEASE, ""), None);
        assert_eq!(
            refused_reason("", "fatal: could not read from remote"),
            None
        );
        assert_eq!(refused_reason(NEW_BRANCH, ""), None);
    }
}
