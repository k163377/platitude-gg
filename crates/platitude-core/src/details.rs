//! Commit details (metadata + changed files) and on-demand file diffs.
//!
//! Merge commits are diffed against their **first parent** (the common GUI
//! convention). Patch-producing diff runs pin `--no-ext-diff` and the
//! standard `a/ b/` prefixes so the patch parser sees a stable shape
//! regardless of user config (the `--name-status` file list emits no
//! patch and needs neither).

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::parse::diff::{FilePatch, parse_patch};
use crate::parse::name_status::{FileChange, parse_name_status};
use crate::process::{GitCommand, GitExecutor, literal_pathspec};

/// Fields: id, parents, author name/email/time, committer name/email/time,
/// co-author trailers, full message body. NUL-separated, record
/// NUL-terminated via `-z`.
///
/// Author and committer are the mailmap spellings, for the same reason the
/// graph log asks for them (`parse::log`) — and because these two have to
/// agree: a person whose picture is drawn on their row in the graph would
/// otherwise lose it the moment the row was clicked. **The trailers are
/// not**: git applies no mailmap to them, and they are message text rather
/// than a person field, so a co-author who also authors commits can appear
/// under two spellings.
///
/// The trailer field is git's own answer, not a scan of the message: the
/// rules for what counts as a trailer (last paragraph, key: value shape,
/// folded continuations) belong to git and are not worth reimplementing.
/// `key=` matches case-insensitively, so the `Co-Authored-By` the tooling
/// writes and the `Co-authored-by` the convention documents both land
/// here. Values are joined with U+001F, which no address or name can
/// contain, so the NUL field split survives (git 2.55 measured; the
/// options are all 2.23 or older).
const DETAILS_FORMAT_ARG: &str = "--format=%H%x00%P%x00%aN%x00%aE%x00%at%x00%cN%x00%cE%x00%ct%x00\
     %(trailers:key=Co-authored-by,valueonly,unfold,separator=%x1F)%x00%B";
const DETAILS_FIELDS: usize = 10;
/// Separates one trailer value from the next inside that field.
const TRAILER_SEP: char = '\u{1f}';

/// Arguments pinning diff output shape against user configuration.
const DIFF_SHAPE_ARGS: [&str; 6] = [
    "-c",
    "diff.noprefix=false",
    "-c",
    "diff.mnemonicPrefix=false",
    "-c",
    "diff.external=",
];

/// Someone the message credits alongside the author.
///
/// A commit object holds one author; everyone else is a `Co-authored-by`
/// trailer, which is a line in the message and nothing more. The name is
/// what git returns; the address is optional, because nothing stops a
/// trailer from carrying a bare name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoAuthor {
    pub name: String,
    /// Without the angle brackets; empty when the trailer had none.
    pub email: String,
}

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
    /// `Co-authored-by` trailers, in the order the message lists them.
    pub co_authors: Vec<CoAuthor>,
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
    let mut message = text(9);
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
        co_authors: parse_co_authors(&text(8)),
        message,
        files: Vec::new(),
    })
}

/// Splits the trailer field into one entry per credited person.
///
/// The shape git hands over is whatever the message wrote after the
/// colon. `Name <address>` is the convention every tool that reads these
/// follows, so the address is taken from the last angle-bracketed run;
/// anything else stays a name in full rather than being guessed at.
fn parse_co_authors(field: &str) -> Vec<CoAuthor> {
    field
        .split(TRAILER_SEP)
        .filter_map(|entry| {
            let entry = entry.trim();
            if entry.is_empty() {
                return None;
            }
            let Some(open) = entry.rfind('<') else {
                return Some(CoAuthor {
                    name: entry.to_string(),
                    email: String::new(),
                });
            };
            let rest = &entry[open + 1..];
            let Some(close) = rest.rfind('>') else {
                return Some(CoAuthor {
                    name: entry.to_string(),
                    email: String::new(),
                });
            };
            Some(CoAuthor {
                name: entry[..open].trim_end().to_string(),
                email: rest[..close].to_string(),
            })
        })
        .collect()
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
    Ok(
        file_diff_with_fingerprint(executor, workdir, target, cancel)
            .await?
            .0,
    )
}

/// [`file_diff`] plus the fingerprint of the bytes it was parsed from.
///
/// The fingerprint travels with the parsed diff to the UI and comes back
/// attached to hunk/line selections, so a partial write can tell "the
/// diff the selection was made on" from "the diff the write re-ran"
/// (`stage::apply_partial`). Positional selections are only meaningful
/// against the exact bytes they indexed.
pub async fn file_diff_with_fingerprint(
    executor: &GitExecutor,
    workdir: &Path,
    target: &DiffTarget,
    cancel: &CancellationToken,
) -> Result<(Vec<FilePatch>, u64), GitError> {
    let raw = file_diff_raw(executor, workdir, target, cancel).await?;
    Ok((parse_patch(&raw), fingerprint(&raw)))
}

/// Stable fingerprint of a raw diff. Drift detection, not cryptography:
/// two runs of the same command over an unchanged file produce the same
/// bytes, and any edit in between changes them.
pub fn fingerprint(raw: &[u8]) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    raw.hash(&mut hasher);
    hasher.finish()
}

/// Same diff as [`file_diff`], returned unparsed.
///
/// Partial staging rebuilds patches from these bytes (see [`crate::patch`]),
/// so both paths must run the exact same command: hunk and line indices are
/// only meaningful against the output they were derived from.
pub async fn file_diff_raw(
    executor: &GitExecutor,
    workdir: &Path,
    target: &DiffTarget,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, GitError> {
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
            c = c.arg("--").arg(literal_pathspec(path));
            if let Some(orig) = orig_path {
                c = c.arg(literal_pathspec(orig));
            }
            c
        }
        DiffTarget::Staged { path, orig_path } => {
            let mut c = base.args(["diff", "--cached", "--no-ext-diff", "--find-renames", "--"]);
            c = c.arg(literal_pathspec(path));
            if let Some(orig) = orig_path {
                c = c.arg(literal_pathspec(orig));
            }
            c
        }
        DiffTarget::Unstaged { path } => base
            .args(["diff", "--no-ext-diff", "--"])
            .arg(literal_pathspec(path)),
        DiffTarget::Untracked { path } => {
            // `--no-index` renders file content as an all-additions patch;
            // it exits 1 when the sides differ, which is the normal case.
            // Its arguments are filenames, not pathspecs — no magic prefix.
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
            return Ok(out.stdout);
        }
    };
    let out = executor.run(cmd, cancel).await?;
    Ok(out.stdout)
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
            "Carol <carol@example.com>",
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
        assert_eq!(d.co_authors.len(), 1);
        assert_eq!(d.co_authors[0].name, "Carol");
        assert_eq!(d.co_authors[0].email, "carol@example.com");
        assert_eq!(d.message, "subject line\n\nbody 日本語\nsecond body line");
    }

    #[test]
    fn co_author_field_splits_on_the_unit_separator() {
        let got = parse_co_authors("Claude Opus 5 <noreply@anthropic.com>\u{1f}Bob <b@e.com>");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "Claude Opus 5");
        assert_eq!(got[0].email, "noreply@anthropic.com");
        assert_eq!(got[1].name, "Bob");
        assert_eq!(got[1].email, "b@e.com");
    }

    #[test]
    fn co_author_without_an_address_keeps_its_whole_name() {
        // Nothing in git requires the angle-bracket form, so a trailer
        // that has none is a name, not a name with a broken address.
        let got = parse_co_authors("Nameless");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "Nameless");
        assert_eq!(got[0].email, "");

        let unclosed = parse_co_authors("Half <open");
        assert_eq!(unclosed[0].name, "Half <open");
        assert_eq!(unclosed[0].email, "");
    }

    #[test]
    fn co_author_name_may_hold_angle_brackets_of_its_own() {
        // The address is the last bracketed run, so a name that contains
        // brackets keeps them.
        let got = parse_co_authors("A <B> C <c@e.com>");
        assert_eq!(got[0].name, "A <B> C");
        assert_eq!(got[0].email, "c@e.com");
    }

    #[test]
    fn no_trailer_yields_no_co_authors() {
        assert!(parse_co_authors("").is_empty());
        assert!(parse_co_authors("\u{1f}").is_empty());
    }

    #[test]
    fn parse_details_rejects_short_input() {
        assert!(parse_details(b"garbage").is_none());
    }
}
