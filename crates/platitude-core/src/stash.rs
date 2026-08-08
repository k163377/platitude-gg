//! Stash listing and stash operations.
//!
//! Selectors (`stash@{0}`) come from a listing and are passed straight
//! back; nothing here builds one from an index, because the numbering
//! shifts under every push and drop.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};

/// `--format=` for `stash list`; fields: reflog selector, commit id,
/// committer time, reflog subject. Records are NUL-terminated via `-z`
/// (stash list forwards options to `git log`).
pub const STASH_FORMAT_ARG: &str = "--format=%gd%x00%H%x00%ct%x00%gs";

const STASH_FIELDS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashEntry {
    /// Reflog selector (`stash@{0}`), the argument for later stash ops.
    pub name: String,
    pub oid: Oid,
    /// Committer time (unix seconds).
    pub time: i64,
    /// Reflog subject (`WIP on main: ...` or the custom message).
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("malformed stash list output")]
pub struct StashParseError;

/// Parses `stash list` output produced with [`STASH_FORMAT_ARG`] + `-z`.
pub fn parse_stashes(bytes: &[u8]) -> Result<Vec<StashEntry>, StashParseError> {
    let tokens: Vec<&[u8]> = bytes.split(|b| *b == 0).collect();
    // Trailing empty token after the final NUL (or a single empty token
    // for empty output).
    let tokens = match tokens.split_last() {
        Some((last, rest)) if last.iter().all(|b| b.is_ascii_whitespace()) => rest,
        _ => &tokens[..],
    };
    if tokens.is_empty() {
        return Ok(Vec::new());
    }
    if tokens.len() % STASH_FIELDS != 0 {
        return Err(StashParseError);
    }
    let mut out = Vec::with_capacity(tokens.len() / STASH_FIELDS);
    for record in tokens.chunks_exact(STASH_FIELDS) {
        let name = String::from_utf8_lossy(record[0]).into_owned();
        let oid = Oid::from_hex(record[1]).map_err(|_| StashParseError)?;
        let time = std::str::from_utf8(record[2])
            .map_err(|_| StashParseError)?
            .trim()
            .parse()
            .map_err(|_| StashParseError)?;
        let message = String::from_utf8_lossy(record[3]).into_owned();
        out.push(StashEntry {
            name,
            oid,
            time,
            message,
        });
    }
    Ok(out)
}

/// Loads the stash list.
pub async fn load(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Vec<StashEntry>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["stash", "list", "-z", STASH_FORMAT_ARG]);
    let out = executor.run(cmd, cancel).await?;
    parse_stashes(&out.stdout).map_err(|e| GitError::UnexpectedOutput {
        command: "git stash list".to_string(),
        message: e.to_string(),
    })
}

/// Knobs of `git stash push`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PushOptions {
    /// Stash untracked files too (`--include-untracked`).
    pub include_untracked: bool,
    /// Leave the index as it is (`--keep-index`).
    pub keep_index: bool,
    /// Stash only what is staged (`--staged`).
    pub staged_only: bool,
}

/// Commit `refs/stash` points at, or `None` with no stash at all.
///
/// Taken before and after a [`push`] to tell whether an entry was really
/// made: on a clean tree `git stash push` exits 0 having created nothing,
/// and `stash@{0}` then names whatever entry was already there.
pub async fn tip(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Option<String>, GitError> {
    // Exit 1 is the answer "there is no stash here", which every
    // repository that has never had one gives. Left unmarked it reads as
    // a failed command and raises the log over a question nobody asked.
    let cmd = GitCommand::new().cwd(workdir).answers_by_code().args([
        "rev-parse",
        "--verify",
        "--quiet",
        "refs/stash",
    ]);
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code != 0 {
        return Ok(None);
    }
    let text = out.stdout_utf8().trim().to_string();
    Ok((!text.is_empty()).then_some(text))
}

/// `git stash push`: saves the working tree, optionally limited to `paths`.
pub async fn push(
    executor: &GitExecutor,
    workdir: &Path,
    message: &str,
    options: PushOptions,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let mut cmd = GitCommand::new().cwd(workdir).args(["stash", "push"]);
    if options.include_untracked {
        cmd = cmd.arg("--include-untracked");
    }
    if options.keep_index {
        cmd = cmd.arg("--keep-index");
    }
    if options.staged_only {
        cmd = cmd.arg("--staged");
    }
    if !message.trim().is_empty() {
        cmd = cmd.args(["--message", message]);
    }
    if !paths.is_empty() {
        cmd = cmd
            .arg("--")
            .args(paths.iter().map(|p| crate::process::literal_pathspec(p)));
    }
    executor.run(cmd, cancel).await.map(|_| ())
}

/// `git stash pop <selector>`: restores and removes the entry.
///
/// Answers by code for the same reason [`pop_with_index`] does: exit 1
/// covers both a restore that landed conflicted and one that did nothing,
/// so it is data for the caller to weigh against the working tree, not a
/// failure to report on sight. Whether it turns out to be one is said by
/// the write it belongs to.
pub async fn pop(
    executor: &GitExecutor,
    workdir: &Path,
    selector: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .answers_by_code()
        .args(["stash", "pop", selector]);
    executor.run(cmd, cancel).await.map(|_| ())
}

/// `git stash pop --index`: restores the entry *and* the split between
/// what was staged in it and what was not.
///
/// Non-zero exit does not mean nothing happened. When the restore
/// conflicts git gives up on the index part ("Index was not unstashed"),
/// leaves the markers in the files and **keeps the entry** — which is what
/// leaves a way back. Callers decide what that is worth by looking at the
/// working tree afterwards, not at the exit code.
pub async fn pop_with_index(
    executor: &GitExecutor,
    workdir: &Path,
    selector: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .answers_by_code()
        .args(["stash", "pop", "--index", selector]);
    executor.run(cmd, cancel).await.map(|_| ())
}

/// `git stash apply <selector>`: restores and keeps the entry.
pub async fn apply(
    executor: &GitExecutor,
    workdir: &Path,
    selector: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    run_selector(executor, workdir, "apply", selector, cancel).await
}

/// Whether a string can be a stash's label.
///
/// Not a ref name — a stash is named by its reflog subject, which is free
/// text — so the only rules are the reflog's own: one line, and something
/// on it.
pub fn is_valid_message(message: &str) -> bool {
    !message.trim().is_empty() && !message.chars().any(|c| c.is_control())
}

/// Renames a stash entry: the label the list shows.
///
/// git has no rename for one, so this is built from what it does have.
/// The entry's commit is written again with the new message and nothing
/// else changed (same tree, same parents, same identities and dates —
/// `commit-tree` only replaces the text), `stash store` puts that on the
/// reflog, and the old entry is dropped. Rewriting the commit rather than
/// storing the original under a new reflog message is what keeps the two
/// places a stash's message is read — the list and the commit itself —
/// saying the same thing.
///
/// Two consequences to know about:
/// - **the entry moves to the top of the list.** The list is a reflog, and
///   a reflog only grows at the front; there is no writing into the middle
///   of one.
/// - **its commit id changes.** Nothing but the reflog points at a stash,
///   so nothing is left dangling.
///
/// The old entry is dropped last and only after its identity is confirmed
/// at the position the store pushed it to: a rename that ends up keeping
/// both entries is a mess, but losing the work is worse.
pub async fn rename(
    executor: &GitExecutor,
    workdir: &Path,
    selector: &str,
    message: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let entry = read_entry(executor, workdir, selector, cancel).await?;
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .args(["commit-tree", &entry.tree]);
    for parent in &entry.parents {
        cmd = cmd.args(["-p", parent]);
    }
    let out = executor
        .run(
            cmd.args(["-m", message])
                .env("GIT_AUTHOR_NAME", &entry.author_name)
                .env("GIT_AUTHOR_EMAIL", &entry.author_email)
                .env("GIT_AUTHOR_DATE", &entry.author_date)
                .env("GIT_COMMITTER_NAME", &entry.committer_name)
                .env("GIT_COMMITTER_EMAIL", &entry.committer_email)
                .env("GIT_COMMITTER_DATE", &entry.committer_date),
            cancel,
        )
        .await?;
    let stored = out.stdout_utf8().trim().to_string();
    if stored.is_empty() {
        return Err(GitError::Rejected {
            message: "git wrote no commit for the renamed stash".to_string(),
        });
    }
    // No `--` here: `stash store` takes one commit and no pathspec, and
    // the separator is not in its usage.
    let store = GitCommand::new()
        .cwd(workdir)
        .args(["stash", "store", "-m", message, &stored]);
    executor.run(store, cancel).await?;

    // `store` prepends, so the old entry is one further down than it was.
    let Some(shifted) = shift_selector(selector) else {
        return Err(GitError::Rejected {
            message: format!("{selector} is not a stash selector this window can move"),
        });
    };
    let at = GitCommand::new()
        .cwd(workdir)
        .args(["rev-parse", "--verify", "--quiet", &shifted]);
    let found = executor.run_unchecked(at, cancel).await?;
    if found.stdout_utf8().trim() != entry.oid {
        return Err(GitError::Rejected {
            message: "the stash list moved while renaming: the new entry was \
                      stored, and the one it replaces is still there"
                .to_string(),
        });
    }
    run_selector(executor, workdir, "drop", &shifted, cancel).await
}

/// `stash@{n}` → `stash@{n+1}`, which is where an entry stands once a new
/// one has been pushed in front of it.
fn shift_selector(selector: &str) -> Option<String> {
    let (head, index) = selector.rsplit_once("@{")?;
    let index = index.strip_suffix('}')?.parse::<u32>().ok()?;
    Some(format!("{head}@{{{}}}", index + 1))
}

/// One stash entry's commit, in the pieces a rewrite needs.
struct StashCommit {
    oid: String,
    tree: String,
    parents: Vec<String>,
    author_name: String,
    author_email: String,
    author_date: String,
    committer_name: String,
    committer_email: String,
    committer_date: String,
}

async fn read_entry(
    executor: &GitExecutor,
    workdir: &Path,
    selector: &str,
    cancel: &CancellationToken,
) -> Result<StashCommit, GitError> {
    const FORMAT: &str = "--format=%H%x00%T%x00%P%x00%an%x00%ae%x00%aI%x00%cn%x00%ce%x00%cI";
    let cmd =
        GitCommand::new()
            .cwd(workdir)
            .args(["log", "-1", FORMAT, "--end-of-options", selector]);
    let out = executor.run(cmd, cancel).await?;
    let text = out.stdout_utf8();
    let fields: Vec<&str> = text.trim_end_matches('\n').split('\0').collect();
    let [oid, tree, parents, an, ae, ad, cn, ce, cd] = fields.as_slice() else {
        return Err(GitError::Rejected {
            message: format!("could not read {selector}"),
        });
    };
    Ok(StashCommit {
        oid: (*oid).to_string(),
        tree: (*tree).to_string(),
        parents: parents.split_whitespace().map(str::to_string).collect(),
        author_name: (*an).to_string(),
        author_email: (*ae).to_string(),
        author_date: (*ad).to_string(),
        committer_name: (*cn).to_string(),
        committer_email: (*ce).to_string(),
        committer_date: (*cd).to_string(),
    })
}

/// `git stash drop <selector>`: discards the entry. Destructive — the
/// caller confirms first.
pub async fn drop(
    executor: &GitExecutor,
    workdir: &Path,
    selector: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    run_selector(executor, workdir, "drop", selector, cancel).await
}

async fn run_selector(
    executor: &GitExecutor,
    workdir: &Path,
    op: &str,
    selector: &str,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let cmd = GitCommand::new().cwd(workdir).args(["stash", op, selector]);
    executor.run(cmd, cancel).await.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn z(tokens: &[&str]) -> Vec<u8> {
        let mut v = Vec::new();
        for t in tokens {
            v.extend_from_slice(t.as_bytes());
            v.push(0);
        }
        v
    }

    #[test]
    fn parses_entries() {
        let bytes = z(&[
            "stash@{0}",
            SHA,
            "1700000000",
            "WIP on main: 1234567 subject",
            "stash@{1}",
            SHA,
            "1699999999",
            "On feature: custom message",
        ]);
        let stashes = parse_stashes(&bytes).unwrap();
        assert_eq!(stashes.len(), 2);
        assert_eq!(stashes[0].name, "stash@{0}");
        assert_eq!(stashes[0].time, 1_700_000_000);
        assert_eq!(stashes[1].message, "On feature: custom message");
    }

    #[test]
    fn empty_output_means_no_stashes() {
        assert!(parse_stashes(b"").unwrap().is_empty());
        assert!(parse_stashes(b"\n").unwrap().is_empty());
    }

    #[test]
    fn wrong_arity_is_an_error() {
        let bytes = z(&["stash@{0}", SHA]);
        assert!(parse_stashes(&bytes).is_err());
    }

    #[test]
    fn a_stored_entry_pushes_the_others_one_down() {
        assert_eq!(shift_selector("stash@{0}").as_deref(), Some("stash@{1}"));
        assert_eq!(shift_selector("stash@{9}").as_deref(), Some("stash@{10}"));
        assert_eq!(shift_selector("stash"), None);
        assert_eq!(shift_selector("stash@{tip}"), None);
    }

    #[test]
    fn a_label_is_one_line_with_something_on_it() {
        assert!(is_valid_message("half of the refactor"));
        assert!(is_valid_message("日本語のラベル"));
        assert!(!is_valid_message(""));
        assert!(!is_valid_message("   "));
        assert!(!is_valid_message("two\nlines"));
        assert!(!is_valid_message("tab\there"));
    }
}
