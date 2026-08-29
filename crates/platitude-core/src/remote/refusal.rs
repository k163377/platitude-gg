//! What a non-zero push was: outdated, turned down over there, or a
//! plain failure — and the words whoever said no wrote for themselves.
//!
//! Kept apart from [`super::push`], which is about where a push goes and
//! how hard it may overwrite: this half is read by the tag commands as
//! well ([`super::tags`]), and none of it looks at a [`PushSpec`].

use crate::error::GitError;
use crate::report::{ReportKind, WriteReport};

/// Which of the three a non-zero push of a branch is: outdated, turned
/// down over there, or a plain failure.
///
/// The two refusals are told apart because **only one of them has a next
/// move** — fetching is what answers `fetch first`, and nothing answers a
/// protected branch — so they reach the screen as two reports with
/// different words and only one of them queues anything
/// (デザイン規約 §答えの要らない報せ).
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
                // git's own advice, which is the whole of what is known
                // here: the far side said nothing, this end worked the
                // refusal out from what it holds. Where the advice is
                // switched off there is still the parenthetical git
                // writes on the ref line itself.
                hint_words(&out.stderr_utf8(), &out.stdout_utf8()),
            )),
        };
    }
    refused(command, out, remote, branch, deleting)
}

/// The same reading for a push that **no fetch can answer** — a tag,
/// whose local name a fetch leaves exactly where it is (`remote::tags`).
///
/// Either the far side turned it down under a rule of its own, or it is
/// a plain failure; there is no third reading to offer.
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
/// **`[remote rejected]` is git's own separation** and the whole of what
/// this reads: `[rejected]` is a refusal this end worked out from what it
/// holds, and every one of those has a next move here. A host that could
/// not be reached prints no ref lines at all, so silence answers `None`.
///
/// The explanation is the far side's, never ours: its `remote:` lines
/// where it wrote any, and git's own parenthetical where it wrote none
/// (a bare `receive-pack` refusing a deletion says nothing else).
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

/// What the far side said for itself, out of the lines git copies to
/// stderr under `remote:`.
///
/// The `error:` some servers put in front of every line goes with the
/// framing: what is left is read under a sentence that has already said
/// what did not happen, and a second word for "this went wrong" there
/// only makes a report look like a fault of the application's.
///
/// **The lines are joined into one.** What reads them is a report with a
/// heading of its own, and a report is a heading and one line under it
/// (デザイン規約 §長さ) — so they run on as the sentences they are, and
/// what does not fit is read in the log with the command it came from.
/// Which of them carries the rule is not something this end can know: a
/// forge writes the summary first and the rule it broke after it, a hook
/// writes whatever its author wrote.
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

/// **git's own words** for a refusal it made itself, out of the `hint:`
/// lines it writes under one.
///
/// The same shape as [`remote_words`] and for the same reason: nobody
/// over there said anything, so what goes under the heading is the
/// explanation git wrote, quoted rather than rewritten.
///
/// **Advice can be switched off** (`advice.pushNonFastForward=false`, and
/// a git that words its hints differently from one release to the next),
/// so the fallback is the parenthetical on the ref line — `fetch first`,
/// `stale info` — which is git's machine-readable half and always there.
fn hint_words(stderr: &str, porcelain: &str) -> String {
    let mut said: Vec<&str> = Vec::new();
    for line in stderr.lines() {
        let Some(rest) = line.trim_end().strip_prefix(crate::report::HINT_PREFIX) else {
            continue;
        };
        let rest = rest.trim();
        if !rest.is_empty() {
            said.push(rest);
        }
    }
    if !said.is_empty() {
        return said.join(" ");
    }
    porcelain
        .lines()
        .find_map(|line| {
            let mut fields = line.split('\t');
            (fields.next() == Some("!")).then(|| fields.nth(1))?
        })
        .map(bracket_reason)
        .unwrap_or_default()
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
/// Three summaries say the same thing: a plain push found commits it would
/// drop (`fetch first`, or `non-fast-forward` for a ref that is not the
/// current branch's upstream), or a lease was pinned to a commit the remote
/// has since left (`stale info`). All three are answered by fetching.
///
/// Anything else — a hook, a protected branch, an unreachable host — is
/// not something a fetch helps with; whether the far side decided it is
/// [`refused_reason`]'s question. A host that could not be reached at all
/// prints no ref lines, so it cannot be mistaken for one of these.
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

    /// Recorded from git 2.51 pushing to a local bare repository (the
    /// refusals from a second clone pushing first, from a lease pinned to
    /// what it had left, and from a `pre-receive` hook exiting non-zero).
    /// Every line here is `--porcelain` output as git wrote it.
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

    /// A refusal the remote decided on its own terms. Fetching tells us
    /// nothing about it, so it must not be dressed up as something to
    /// retry — and neither must a host that never answered, which prints
    /// no ref lines at all.
    #[test]
    fn refusals_a_fetch_cannot_help_with_are_left_alone() {
        assert!(!is_outdated(REFUSED_BY_THE_FAR_SIDE));
        assert!(!is_outdated(""));
    }

    /// The same run of git 2.51 against a bare repository whose
    /// `pre-receive` hook exits non-zero, and the words GitHub writes
    /// through it (measured — the two `remote: error:` lines are exactly what
    /// a protected branch answers a deletion with).
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

    /// A server that says nothing for itself still has to leave the
    /// screen something to read, and git's own parenthetical is it.
    #[test]
    fn a_silent_far_side_leaves_gits_parenthetical() {
        assert_eq!(
            refused_reason(REFUSED_BY_THE_FAR_SIDE, "To C:/tmp/remote.git\n").as_deref(),
            Some("pre-receive hook declined")
        );
    }

    /// The refusals this end worked out for itself are not the far side
    /// speaking, whatever else is on stderr — nor is a host that never
    /// answered at all.
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
