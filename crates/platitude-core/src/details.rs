//! Commit details (metadata + changed files) and on-demand file diffs.
//!
//! Merge commits are diffed against their **first parent** (the common GUI
//! convention). All diff runs pin `--no-ext-diff` and the standard `a/ b/`
//! prefixes so the patch parser sees a stable shape regardless of user
//! config.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::parse::diff::{FilePatch, parse_patch};
use crate::parse::name_status::{FileChange, parse_name_status};
use crate::process::{GitCommand, GitExecutor};

/// Fields: id, parents, author name/email/time, committer name/email/time,
/// full message body. NUL-separated, record NUL-terminated via `-z`.
const DETAILS_FORMAT_ARG: &str =
    "--format=%H%x00%P%x00%an%x00%ae%x00%at%x00%cn%x00%ce%x00%ct%x00%B";
const DETAILS_FIELDS: usize = 9;

/// Arguments pinning diff output shape against user configuration.
const DIFF_SHAPE_ARGS: [&str; 6] = [
    "-c",
    "diff.noprefix=false",
    "-c",
    "diff.mnemonicPrefix=false",
    "-c",
    "diff.external=",
];

/// Full metadata of one commit plus its changed files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitDetails {
    pub oid: Oid,
    pub parents: Vec<Oid>,
    pub author_name: String,
    pub author_email: String,
    pub author_time: i64,
    pub committer_name: String,
    pub committer_email: String,
    pub committer_time: i64,
    /// Full message (subject + body).
    pub message: String,
    /// Changed files vs the first parent (creation diff for root commits).
    pub files: Vec<FileChange>,
}

/// Loads commit metadata and its changed-file list (two plumbing calls).
pub async fn commit_details(
    executor: &GitExecutor,
    workdir: &Path,
    oid: &Oid,
    cancel: &CancellationToken,
) -> Result<CommitDetails, GitError> {
    let hex = oid.to_hex();
    let cmd =
        GitCommand::new()
            .cwd(workdir)
            .args(["show", "--no-patch", "-z", DETAILS_FORMAT_ARG, &hex]);
    let out = executor.run(cmd, cancel).await?;
    let mut details = parse_details(&out.stdout).ok_or_else(|| GitError::UnexpectedOutput {
        command: format!("git show --no-patch {hex}"),
        message: "unexpected field layout".to_string(),
    })?;

    let mut cmd = GitCommand::new().cwd(workdir).args(DIFF_SHAPE_ARGS).args([
        "diff-tree",
        "-r",
        "--no-commit-id",
        "-z",
        "--name-status",
        "--find-renames",
    ]);
    cmd = match details.parents.first() {
        Some(p1) => cmd.args([p1.to_hex(), hex.clone()]),
        None => cmd.args(["--root".to_string(), hex.clone()]),
    };
    let files_out = executor.run(cmd, cancel).await?;
    details.files =
        parse_name_status(&files_out.stdout).map_err(|e| GitError::UnexpectedOutput {
            command: format!("git diff-tree --name-status {hex}"),
            message: e.to_string(),
        })?;
    Ok(details)
}

fn parse_details(bytes: &[u8]) -> Option<CommitDetails> {
    let fields: Vec<&[u8]> = bytes.splitn(DETAILS_FIELDS, |b| *b == 0).collect();
    if fields.len() != DETAILS_FIELDS {
        return None;
    }
    let text = |i: usize| String::from_utf8_lossy(fields[i]).into_owned();
    let time = |i: usize| {
        std::str::from_utf8(fields[i])
            .ok()
            .and_then(|s| s.trim().parse::<i64>().ok())
    };

    let oid = Oid::from_hex(fields[0]).ok()?;
    let mut parents = Vec::new();
    for hex in fields[1].split(|b| *b == b' ').filter(|s| !s.is_empty()) {
        parents.push(Oid::from_hex(hex).ok()?);
    }
    // %B is the last field; `-z` terminates the record with NUL, and show
    // appends a newline after the format expansion. Trim both.
    let mut message = text(8);
    while message.ends_with(['\0', '\n']) {
        message.pop();
    }

    Some(CommitDetails {
        oid,
        parents,
        author_name: text(2),
        author_email: text(3),
        author_time: time(4)?,
        committer_name: text(5),
        committer_email: text(6),
        committer_time: time(7)?,
        message,
        files: Vec::new(),
    })
}

/// Which diff a pane is asking for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffTarget {
    /// One file of a commit, diffed against the first parent.
    Commit {
        oid: Oid,
        /// First parent (`None` for root commits → creation diff).
        parent: Option<Oid>,
        path: String,
        /// Source path when the file list reported a rename/copy.
        orig_path: Option<String>,
    },
    /// Index vs HEAD for one file.
    Staged {
        path: String,
        orig_path: Option<String>,
    },
    /// Working tree vs index for one file.
    Unstaged { path: String },
    /// Content of an untracked file rendered as an all-additions diff.
    Untracked { path: String },
}

/// Loads a unified diff for one file.
pub async fn file_diff(
    executor: &GitExecutor,
    workdir: &Path,
    target: &DiffTarget,
    cancel: &CancellationToken,
) -> Result<Vec<FilePatch>, GitError> {
    let base = GitCommand::new().cwd(workdir).args(DIFF_SHAPE_ARGS);
    let cmd = match target {
        DiffTarget::Commit {
            oid,
            parent,
            path,
            orig_path,
        } => {
            let mut c = base.args([
                "diff-tree",
                "-r",
                "--no-commit-id",
                "-p",
                "--no-ext-diff",
                "--find-renames",
            ]);
            c = match parent {
                Some(p1) => c.args([p1.to_hex(), oid.to_hex()]),
                None => c.args(["--root".to_string(), oid.to_hex()]),
            };
            c = c.arg("--").arg(path);
            if let Some(orig) = orig_path {
                c = c.arg(orig);
            }
            c
        }
        DiffTarget::Staged { path, orig_path } => {
            let mut c = base.args(["diff", "--cached", "--no-ext-diff", "--find-renames", "--"]);
            c = c.arg(path);
            if let Some(orig) = orig_path {
                c = c.arg(orig);
            }
            c
        }
        DiffTarget::Unstaged { path } => base.args(["diff", "--no-ext-diff", "--"]).arg(path),
        DiffTarget::Untracked { path } => {
            // `--no-index` renders file content as an all-additions patch;
            // it exits 1 when the sides differ, which is the normal case.
            let out = executor
                .run_unchecked(
                    base.args(["diff", "--no-ext-diff", "--no-index", "--", "/dev/null"])
                        .arg(path),
                    cancel,
                )
                .await?;
            if out.code != 0 && out.code != 1 {
                return Err(GitError::Failed {
                    command: format!("git diff --no-index /dev/null {path}"),
                    code: out.code,
                    stderr: out.stderr_utf8().trim().to_string(),
                });
            }
            return Ok(parse_patch(&out.stdout));
        }
    };
    let out = executor.run(cmd, cancel).await?;
    Ok(parse_patch(&out.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_details_extracts_all_fields() {
        let sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let p1 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let p2 = "cccccccccccccccccccccccccccccccccccccccc";
        let mut bytes = Vec::new();
        for field in [
            sha,
            &format!("{p1} {p2}"),
            "Alice",
            "alice@example.com",
            "1700000000",
            "Bob",
            "bob@example.com",
            "1700000060",
            "subject line\n\nbody 日本語\nsecond body line\n",
        ] {
            bytes.extend_from_slice(field.as_bytes());
            bytes.push(0);
        }
        let d = parse_details(&bytes).unwrap();
        assert_eq!(d.oid.to_hex(), sha);
        assert_eq!(d.parents.len(), 2);
        assert_eq!(d.author_name, "Alice");
        assert_eq!(d.committer_email, "bob@example.com");
        assert_eq!(d.author_time, 1_700_000_000);
        assert_eq!(d.message, "subject line\n\nbody 日本語\nsecond body line");
    }

    #[test]
    fn parse_details_rejects_short_input() {
        assert!(parse_details(b"garbage").is_none());
    }
}
