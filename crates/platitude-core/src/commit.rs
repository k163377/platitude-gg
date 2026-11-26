//! Creating and amending commits.
//!
//! Messages travel on stdin (`--file=-`), not in `-m`: Windows bounds
//! argument length. `--cleanup=whitespace` is pinned so a repository's
//! `commit.cleanup` cannot rewrite what the editor showed (a typed `#` line
//! stays).
//!
//! Hooks always run: `--no-verify` is never passed. They do not inherit the
//! message's stdin (git starts them with `no_stdin`), and `pre-commit` runs
//! before git reads the message (v2.43.0 commit.c / hook.c).

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};
use crate::repo::RepoInfo;

/// Knobs of one commit invocation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CommitOptions {
    /// Replace the current HEAD commit.
    pub amend: bool,
    /// Permit a commit that changes nothing (`--allow-empty`).
    pub allow_empty: bool,
    /// On amend, take over authorship (`--reset-author`).
    pub reset_author: bool,
}

/// Joins the two fields of a message editor into one commit message:
/// summary, blank line, description ([`split_message`] reads it back).
pub fn join_message(subject: &str, body: &str) -> String {
    let subject = subject.trim();
    let body = body.trim();
    if body.is_empty() {
        return subject.to_string();
    }
    if subject.is_empty() {
        return body.to_string();
    }
    format!("{subject}\n\n{body}")
}

/// A recorded message as a reader is shown it, in the two fields: the
/// first line, and the rest from past the empty lines that part it from
/// the first — each as written, a subject's indent and a body's own
/// blanks and blank lines kept (デザイン規約
/// §リポジトリの文は原文で描く). Line endings are only line breaks:
/// `\r\n` and a lone `\r` read as `\n`, all a text box keeps of them. A
/// field a caret comes into takes [`split_message`]'s cut instead, the
/// one a save writes. Not git's `%s` / `%b`: those take the whole first
/// paragraph as the subject, and the subject field holds one line.
pub fn shown_message(message: &str) -> (String, String) {
    let message = message.replace("\r\n", "\n").replace('\r', "\n");
    let (subject, rest) = message.split_once('\n').unwrap_or((&message, ""));
    (
        subject.to_string(),
        rest.trim_start_matches('\n').to_string(),
    )
}

/// Splits a commit message back into the editor's two fields: the first
/// line, and the rest without the separating blank line.
pub fn split_message(message: &str) -> (String, String) {
    let message = message.replace("\r\n", "\n");
    let (subject, rest) = match message.split_once('\n') {
        Some((s, r)) => (s, r),
        None => (message.as_str(), ""),
    };
    (
        subject.trim().to_string(),
        rest.trim_start_matches('\n').trim_end().to_string(),
    )
}

/// Commits the staged content.
///
/// An empty `message` is only valid together with [`CommitOptions::amend`],
/// where it means "keep the existing message" (`--no-edit`).
///
/// An amend of a commit that is itself empty (an `--allow-empty` marker)
/// goes with `--allow-empty`: git refuses an amend whose tree is HEAD^'s
/// otherwise (`You asked to amend … would make it empty`, the whole status
/// on stdout, exit 1), and its own `reword` passes the same — so a
/// message-only amend of an empty HEAD goes through, from the WIP box, the
/// details pane and a plan's reword alike. An amend that would empty a
/// commit with changes is the screen's to refuse before the press
/// ([`amend_would_empty`]). Held by `an_empty_commits_message_is_amended_on_its_own`,
/// `an_amend_empties_only_a_commit_whose_changes_are_staged_back` and
/// `rewording_an_empty_older_commit_replaces_its_message`.
///
/// Nothing is read back: a second command asking where HEAD landed could
/// fail on its own and report a made commit as failed. The refresh after
/// the write reads it.
pub async fn commit(
    executor: &GitExecutor,
    repo: &RepoInfo,
    message: &str,
    options: CommitOptions,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let allow_empty = options.allow_empty
        || (options.amend && head_is_empty(executor, &repo.workdir, cancel).await?);
    // Traced: a hook's no and an ssh signing program's are in git's record,
    // not its words ([`refused_by`]).
    let mut cmd = GitCommand::new().cwd(&repo.workdir).traced().arg("commit");
    if options.amend {
        cmd = cmd.arg("--amend");
    }
    if allow_empty {
        cmd = cmd.arg("--allow-empty");
    }
    if options.reset_author {
        cmd = cmd.arg("--reset-author");
    }

    if message.trim().is_empty() {
        if !options.amend {
            return Err(GitError::UnexpectedOutput {
                command: "git commit".to_string(),
                message: "refusing to commit with an empty message".to_string(),
            });
        }
        cmd = cmd.arg("--no-edit");
    } else {
        cmd = cmd
            .args(["--cleanup=whitespace", "--file=-"])
            .input(normalized(message));
    }

    let command = cmd.describe();
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code != 0 {
        return Err(refused(command, &out));
    }
    Ok(())
}

/// A commit that was not made, as something to report
/// (デザイン規約 §答えの要らない報せ): refusals this end could answer are
/// taken away before the button can be pressed, so a non-zero exit is
/// something outside saying no ([`refused_by`]).
///
/// Gated before the press: the identity, an empty message, an empty
/// selection, an amend that would leave nothing, an amend during a merge,
/// unmerged paths. The three named refusals are everyday (gitの扱い.md
/// §異常時の扱いの基準): hooks from husky, lefthook or pre-commit, a signing
/// key or agent for verified commits, an IDE's git behind the scenes holding
/// a lock. The rest (an index that cannot be written: the disk) is out of
/// scope and keeps the plain kind.
fn refused(command: String, out: &crate::process::GitOutput) -> GitError {
    crate::report::reported(
        crate::report::WriteReport::local(refused_by(out)),
        command,
        out,
    )
}

/// The hooks whose no stops a commit (builtin/commit.c; `post-commit`'s end
/// changes nothing).
pub(crate) const COMMIT_HOOKS: [&str; 3] = ["pre-commit", "prepare-commit-msg", "commit-msg"];

/// Whether git could not sign: its own words name the signing (`LC_ALL=C`,
/// gpg-interface.c — gpg or gpgsm gave no signature, no ssh key is named or
/// none can be had, the ssh-keygen there cannot sign, the key or the
/// signature could not pass through the temporary file), or — for an ssh
/// signature, whose program's failure git passes on as that program's words
/// alone (`sign_buffer_ssh`) — the program (`-Y sign`) ended non-zero in its
/// trace. git's own `failed to write commit object` is no evidence: it says
/// the same when the object cannot be written.
pub(crate) fn signing_failed(said: &str, trace: &crate::process::Trace) -> bool {
    crate::relied_messages::SIGNING_FAILED.found_in(said) || trace.ssh_signer_failed()
}

/// Which of the outside's refusals a failed `git commit` was, as far as git
/// said or recorded it. A hook first, by its end in git's trace — git adds
/// nothing of its own to a hook's no, and the hook's own output, which may
/// carry a lock's or a signer's words, lands among git's: the index's lock
/// is taken before the hooks run, the signing and the ref's lock after them
/// all. Then a signing failure in git's own words (or an ssh signer's end,
/// [`signing_failed`]) and a lock git named. The rest is git's words to
/// explain (`git turned it down`). The warning and `hint:` 2.55's
/// `core.commentChar=auto` puts ahead every time change none of it. Net:
/// the four `refused_commit` tests of `commit_integration`.
fn refused_by(out: &crate::process::GitOutput) -> crate::report::ReportKind {
    use crate::report::ReportKind;
    let said = out.stderr_utf8();
    if out.trace.hook_failed(&COMMIT_HOOKS) {
        ReportKind::CommitHook
    } else if signing_failed(&said, &out.trace) {
        ReportKind::CommitUnsigned
    } else if crate::report::lock_taken(&said) {
        ReportKind::CommitLocked
    } else {
        ReportKind::Commit
    }
}

/// Reads HEAD's message, for prefilling an amend editor. `None` on an
/// unborn branch.
pub async fn head_message(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Option<String>, GitError> {
    let message = head_log(executor, workdir, "%B", cancel).await?;
    Ok(message.map(|m| m.trim_end_matches('\n').to_string()))
}

/// HEAD's author, twice: as git recorded it, which an amend keeps and
/// `--reset-author` replaces — the spelling that replacing names — and as
/// `.mailmap` spells it (`%aN` / `%aE`), the graph's and the details pane's,
/// which faces are drawn from. `oid` is the commit it is the author of;
/// `None` in the empty answer an unborn HEAD gets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeadAuthor {
    pub oid: Option<Oid>,
    pub name: String,
    pub email: String,
    pub shown_name: String,
    pub shown_email: String,
}

/// Reads HEAD's author ([`HeadAuthor`]). `None` on an unborn branch.
pub async fn head_author(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Option<HeadAuthor>, GitError> {
    let format = "%H%x00%an%x00%ae%x00%aN%x00%aE";
    let Some(text) = head_log(executor, workdir, format, cancel).await? else {
        return Ok(None);
    };
    let unexpected = || GitError::UnexpectedOutput {
        command: "git log -1".to_string(),
        message: "the commit and its author's spellings are not NUL-separated".to_string(),
    };
    let mut fields = text.trim_end_matches('\n').splitn(5, '\0');
    let mut field = || fields.next().map(str::to_string).ok_or_else(unexpected);
    let oid = Oid::from_hex_str(&field()?).map_err(|_| unexpected())?;
    Ok(Some(HeadAuthor {
        oid: Some(oid),
        name: field()?,
        email: field()?,
        shown_name: field()?,
        shown_email: field()?,
    }))
}

/// HEAD's commit through `git log -1 --format=<format>`; `None` on an
/// unborn branch.
async fn head_log(
    executor: &GitExecutor,
    workdir: &Path,
    format: &str,
    cancel: &CancellationToken,
) -> Result<Option<String>, GitError> {
    // Asked first, so a failure of the log read below stays a failure.
    let probe = GitCommand::new().cwd(workdir).answers_by_code(1).args([
        "rev-parse",
        "--verify",
        "--quiet",
        "HEAD",
    ]);
    if executor.run_unchecked(probe, cancel).await?.code != 0 {
        return Ok(None);
    }
    let format = format!("--format={format}");
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["log", "-1", format.as_str()]);
    Ok(Some(
        executor.run(cmd, cancel).await?.stdout_utf8().into_owned(),
    ))
}

/// Whether HEAD changes nothing against its first parent (the empty tree
/// for a root commit) — a marker made with `--allow-empty`. A merge
/// answers true too (`diff-tree` shows nothing for one unasked), which
/// costs nothing: git amends a merge whatever its tree.
async fn head_is_empty(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args([
            "diff-tree",
            "--quiet",
            "--no-commit-id",
            "--root",
            "-r",
            "HEAD",
            "--",
        ])
        .answers_by_code(1);
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => Ok(true),
        1 => Ok(false),
        code => Err(GitError::Failed {
            command: "git diff-tree --quiet HEAD".to_string(),
            code,
            stderr: out.stderr_utf8().into_owned(),
        }),
    }
}

/// Whether amending HEAD now would leave it changing nothing: what is
/// staged takes back everything HEAD changed, so the index's tree is
/// HEAD's first parent's. git refuses that amend, so the screen refuses
/// it first: the app's `RepoTab.amendEmpties` lowers both the commit
/// button and the details pane's save (rewording HEAD is a `commit
/// --amend` too, and takes what is staged with it). When to ask again is
/// core's (`RepoSession::watch_amend`). A message-only amend of an empty
/// HEAD is not this one ([`commit`] lets it through). A root commit
/// answers false: emptying one means taking every file out of the index.
/// Compared as trees, modes included, so a mode is a change: an amend of
/// a HEAD that changed only a mode, or one staging only a mode, goes
/// through.
pub async fn amend_would_empty(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let parent = GitCommand::new().cwd(workdir).answers_by_code(1).args([
        "rev-parse",
        "--verify",
        "--quiet",
        "HEAD~1",
    ]);
    if executor.run_unchecked(parent, cancel).await?.code != 0 {
        return Ok(false);
    }
    if head_is_empty(executor, workdir, cancel).await? {
        return Ok(false);
    }
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["diff-index", "--cached", "--quiet", "HEAD~1", "--"])
        .answers_by_code(1);
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => Ok(true),
        1 => Ok(false),
        code => Err(GitError::Failed {
            command: "git diff-index --cached --quiet HEAD~1".to_string(),
            code,
            stderr: out.stderr_utf8().into_owned(),
        }),
    }
}

/// True when HEAD exists and is a merge commit (amending one is a
/// different proposition and the UI warns about it).
pub async fn head_is_merge(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["log", "-1", "--format=%P"]);
    let out = executor.run(cmd, cancel).await?;
    Ok(out.stdout_utf8().split_whitespace().count() > 1)
}

/// The commit HEAD points at.
pub async fn head_oid(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Oid, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["rev-parse", "--verify", "HEAD"]);
    let out = executor.run(cmd, cancel).await?;
    Oid::from_hex(out.stdout_utf8().trim().as_bytes()).map_err(|_| GitError::UnexpectedOutput {
        command: "git rev-parse --verify HEAD".to_string(),
        message: out.stdout_utf8().trim().to_string(),
    })
}

/// Whether `tip` (`HEAD`, or a branch's full ref) can reach `oid` — the
/// commit is the tip itself or something it was built on: a push of that
/// tip then fast-forwards a remote branch standing on `oid`.
pub async fn is_in_history(
    executor: &GitExecutor,
    workdir: &Path,
    oid: &Oid,
    tip: &str,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["merge-base", "--is-ancestor"])
        .arg(oid.to_hex())
        .arg(tip)
        .answers_by_code(1);
    let out = executor.run_unchecked(cmd, cancel).await?;
    match out.code {
        0 => Ok(true),
        1 => Ok(false),
        code => Err(GitError::Failed {
            command: format!("git merge-base --is-ancestor {} {tip}", oid.to_hex()),
            code,
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }),
    }
}

/// How many commits `rev` reaches that `tip` does not — what a force push
/// of `tip` would take off the far side (デザイン規約 §相手の履歴を置き換える),
/// or what a branch `rev` moved onto `tip` would lose. Only asked when
/// `rev` is here to walk. The `--` keeps git from statting the range as a
/// path (`edge_strings::a_range_of_names_on_the_wall_is_counted_not_statted`).
pub async fn count_beyond(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    tip: &str,
    cancel: &CancellationToken,
) -> Result<u32, GitError> {
    let range = format!("{tip}..{rev}");
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["rev-list", "--count"])
        .arg(&range)
        .arg("--");
    let out = executor.run(cmd, cancel).await?;
    out.stdout_utf8()
        .trim()
        .parse()
        .map_err(|_| GitError::UnexpectedOutput {
            command: format!("git rev-list --count {range}"),
            message: "a count was expected".to_string(),
        })
}

/// Normalizes editor text for git: CRLF to LF and exactly one trailing
/// newline. `--cleanup=whitespace` handles the rest.
fn normalized(message: &str) -> String {
    let mut text = message.replace("\r\n", "\n");
    while text.ends_with('\n') {
        text.pop();
    }
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refusing;

    /// The executor cannot run anything, so a commit that reached for git
    /// would fail on the spawn rather than come back as this refusal.
    #[tokio::test]
    async fn an_empty_message_is_refused_for_a_new_commit_without_asking_git() {
        let (exec, asked) = refusing::git();
        let repo = refusing::repo();

        let err = commit(
            &exec,
            &repo,
            "   \n\n",
            CommitOptions::default(),
            &CancellationToken::new(),
        )
        .await
        .expect_err("an empty message is refused");

        assert!(err.to_string().contains("empty message"), "{err}");
        assert_eq!(asked.count(), 0, "nothing was asked of git");
    }

    #[test]
    fn the_two_editor_fields_join_with_a_blank_line() {
        assert_eq!(join_message("subject", "body"), "subject\n\nbody");
        assert_eq!(join_message("  subject  ", "  "), "subject");
        assert_eq!(join_message("", "body only"), "body only");
        assert_eq!(join_message("", ""), "");
    }

    #[test]
    fn a_message_splits_back_into_the_fields_it_came_from() {
        assert_eq!(
            split_message("subject\n\nbody\nmore"),
            ("subject".to_string(), "body\nmore".to_string())
        );
        assert_eq!(
            split_message("subject only\n"),
            ("subject only".to_string(), String::new())
        );
        // git's own log output arrives with CRLF on Windows checkouts.
        assert_eq!(
            split_message("subject\r\n\r\nbody\r\n"),
            ("subject".to_string(), "body".to_string())
        );
        assert_eq!(
            split_message("subject\nrun-on"),
            ("subject".to_string(), "run-on".to_string())
        );
    }

    #[test]
    fn a_message_is_shown_as_written_but_for_its_line_endings() {
        let shown = shown_message;
        assert_eq!(
            shown("    indented subject\n\n\n  The body.   \n\nlast"),
            (
                "    indented subject".to_string(),
                "  The body.   \n\nlast".to_string()
            ),
            "the indent, the blanks and the body's own blank line stay"
        );
        assert_eq!(
            shown("subject\r\n\r\nbody\r\nmore"),
            ("subject".to_string(), "body\nmore".to_string()),
            "CRLF is a line break"
        );
        assert_eq!(
            shown("subject\r\rbody"),
            ("subject".to_string(), "body".to_string()),
            "so is a lone CR"
        );
        assert_eq!(
            shown("subject only"),
            ("subject only".to_string(), String::new())
        );
        assert_eq!(
            shown("subject\nrun-on"),
            ("subject".to_string(), "run-on".to_string())
        );
    }

    #[test]
    fn join_and_split_round_trip() {
        let (subject, body) = split_message(&join_message("s", "b1\nb2"));
        assert_eq!((subject.as_str(), body.as_str()), ("s", "b1\nb2"));
    }

    #[test]
    fn normalizes_line_endings_and_trailing_newline() {
        assert_eq!(
            normalized("subject\r\n\r\nbody\r\n\n\n"),
            "subject\n\nbody\n"
        );
        assert_eq!(normalized("subject"), "subject\n");
    }

    #[test]
    fn keeps_interior_blank_lines() {
        assert_eq!(normalized("a\n\n\nb"), "a\n\n\nb\n");
    }
}
