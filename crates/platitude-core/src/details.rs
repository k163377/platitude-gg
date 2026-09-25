//! Commit details (metadata + changed files) and on-demand file diffs.
//!
//! Merges are diffed against their first parent. Every diff run pins the
//! output shape (`DIFF_SHAPE_ARGS`, `--no-ext-diff`) against user config.

use std::collections::HashSet;
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
/// Author and committer are the mailmap spellings, matching the graph log
/// (`parse::log`) so a row's avatar survives the click. The trailers are
/// raw (git applies no mailmap to them). `key=` matches case-insensitively;
/// values are joined with U+001F (rules-refs/app-ui.md「co-author は詳細ペインの日付行」).
const DETAILS_FORMAT_ARG: &str = "--format=%H%x00%P%x00%aN%x00%aE%x00%at%x00%cN%x00%cE%x00%ct%x00\
     %(trailers:key=Co-authored-by,valueonly,unfold,separator=%x1F)%x00%B";
const DETAILS_FIELDS: usize = 10;

/// Arguments pinning diff output shape against user configuration.
const DIFF_SHAPE_ARGS: [&str; 6] = [
    "-c",
    "diff.noprefix=false",
    "-c",
    "diff.mnemonicPrefix=false",
    "-c",
    "diff.external=",
];

/// Parses git's packed trailer field; uncommitted text is
/// [`crate::trailers::co_authors_in`]'s.
pub use crate::trailers::{CoAuthor, parse_co_authors};

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

/// Loads commit metadata and its changed-file list in one `show` — on
/// Windows the process start dominates this call
/// (ci/baseline/code-costs-windows-x64.md).
pub async fn commit_details(
    executor: &GitExecutor,
    workdir: &Path,
    oid: &Oid,
    cancel: &CancellationToken,
) -> Result<CommitDetails, GitError> {
    let hex = oid.to_hex();
    let cmd = GitCommand::new().cwd(workdir).args(DIFF_SHAPE_ARGS).args([
        "show",
        "-z",
        "-r",
        "--name-status",
        "--find-renames",
        // Left alone, `show` prints no file list for a merge.
        "--diff-merges=first-parent",
        DETAILS_FORMAT_ARG,
        &hex,
    ]);
    let out = executor.run(cmd, cancel).await?;
    let unexpected = |message: String| GitError::UnexpectedOutput {
        command: format!("git show --name-status {hex}"),
        message,
    };
    let (record, files) = split_record(&out.stdout)
        .ok_or_else(|| unexpected("unexpected field layout".to_string()))?;
    let mut details =
        parse_details(record).ok_or_else(|| unexpected("unexpected field layout".to_string()))?;
    details.files = parse_name_status(files).map_err(|e| unexpected(e.to_string()))?;
    Ok(details)
}

/// Splits the combined output into the `--format` record and the
/// `--name-status` bytes behind it, counting [`DETAILS_FIELDS`] NULs — a
/// message may hold a line like `M\tsrc/main.rs`, which a scan would take
/// for a file. One newline separates the two.
fn split_record(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let mut end = 0;
    for _ in 0..DETAILS_FIELDS {
        end += bytes[end..].iter().position(|b| *b == 0)? + 1;
    }
    let (record, rest) = bytes.split_at(end);
    Some((record, rest.strip_prefix(b"\n").unwrap_or(rest)))
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
    // %B is last: trim the `-z` NUL and the newline `show` appends.
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

/// Record marker for a multi-commit log: between records are only
/// `<status>\0<path>\0` pairs, and a status is an ASCII capital.
const UNION_MARK: u8 = 0x01;

/// Commit ids per command line: Windows caps a command line at 32k, and
/// one Shift click can choose more rows than fit.
const UNION_CHUNK: usize = 200;

/// The files a set of commits changed, each against its own first
/// parent, merged into one list — per commit, since adjacent graph rows
/// need not form a git range (デザイン規約 §複数のコミットを選ぶ). A path
/// several touched wears the status of the first commit asked for
/// (callers pass newest first).
pub async fn union_files(
    executor: &GitExecutor,
    workdir: &Path,
    oids: &[Oid],
    cancel: &CancellationToken,
) -> Result<Vec<FileChange>, GitError> {
    let mut out: Vec<FileChange> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for chunk in oids.chunks(UNION_CHUNK) {
        for change in union_chunk(executor, workdir, chunk, cancel).await? {
            if seen.insert(change.path.clone()) {
                out.push(change);
            }
        }
    }
    Ok(out)
}

async fn union_chunk(
    executor: &GitExecutor,
    workdir: &Path,
    oids: &[Oid],
    cancel: &CancellationToken,
) -> Result<Vec<FileChange>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(DIFF_SHAPE_ARGS)
        .args([
            "log",
            // Plain `--no-walk` re-sorts by date, away from the graph's
            // order.
            "--no-walk=unsorted",
            "-z",
            "-r",
            "--name-status",
            "--find-renames",
            "--diff-merges=first-parent",
            "--format=%x01",
        ])
        .args(oids.iter().map(Oid::to_hex));
    let out = executor.run(cmd, cancel).await?;
    let mut changes = Vec::new();
    // The first split is what stands before the first record: nothing.
    for record in out.stdout.split(|b| *b == UNION_MARK).skip(1) {
        // `log` puts `\0\n` between the format expansion and the file
        // list; the newline would otherwise glue onto the first status.
        let files = record.strip_prefix(b"\0").unwrap_or(record);
        let files = files.strip_prefix(b"\n").unwrap_or(files);
        changes.extend(
            parse_name_status(files).map_err(|e| GitError::UnexpectedOutput {
                command: "git log --no-walk --name-status".to_string(),
                message: e.to_string(),
            })?,
        );
    }
    Ok(changes)
}

/// The files that differ between two commits' trees — a choice of exactly
/// two (デザイン規約 §複数のコミットを選ぶ). Not [`union_files`] of the two:
/// this carries what unselected commits did in between and drops what the
/// older one did itself.
pub async fn compare_files(
    executor: &GitExecutor,
    workdir: &Path,
    from: &Oid,
    to: &Oid,
    cancel: &CancellationToken,
) -> Result<Vec<FileChange>, GitError> {
    let from_hex = from.to_hex();
    let to_hex = to.to_hex();
    // The same `diff-tree --find-renames` as the patch behind each row
    // (`DiffTarget::Range`), so the two agree about renames.
    let cmd = GitCommand::new().cwd(workdir).args(DIFF_SHAPE_ARGS).args([
        "diff-tree",
        "-z",
        "-r",
        "--no-commit-id",
        "--name-status",
        "--find-renames",
        from_hex.as_str(),
        to_hex.as_str(),
    ]);
    let out = executor.run(cmd, cancel).await?;
    parse_name_status(&out.stdout).map_err(|e| GitError::UnexpectedOutput {
        command: format!("git diff-tree --name-status {from_hex} {to_hex}"),
        message: e.to_string(),
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
    /// One file between two commits' trees (a choice of exactly two).
    Range {
        from: Oid,
        to: Oid,
        path: String,
        /// Source path where the file list reported a rename/copy.
        orig_path: Option<String>,
    },
    /// One file as each of several chosen commits changed it, each
    /// commit's own patch — a choice of three or more
    /// (デザイン規約 §複数のコミットを選ぶ). A span comparison would carry
    /// unchosen commits in between.
    Choice {
        /// Newest first, the order the graph stands in.
        oids: Vec<Oid>,
        path: String,
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

/// [`file_diff`] plus the fingerprint of the bytes it was parsed from. It
/// comes back with hunk/line selections so a partial write can refuse a
/// diff that moved since (`stage::apply_partial`).
pub async fn file_diff_with_fingerprint(
    executor: &GitExecutor,
    workdir: &Path,
    target: &DiffTarget,
    cancel: &CancellationToken,
) -> Result<(Vec<FilePatch>, u64), GitError> {
    let raw = file_diff_raw(executor, workdir, target, cancel).await?;
    Ok((parse_patch(&raw), fingerprint(&raw)))
}

/// Fingerprint of a raw diff, for drift detection. Stable only within one
/// run (`DefaultHasher` promises nothing across std releases): persisting
/// it or comparing across processes needs a stronger digest. Not shared
/// with the crate's other staleness hashes
/// (rules-refs/core.md「staleness のハッシュは問いごとに持つ」).
pub fn fingerprint(raw: &[u8]) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    raw.hash(&mut hasher);
    hasher.finish()
}

/// Appends `-- <path>`, plus the rename source where there is one —
/// without both, git has no pair to match.
fn for_paths(cmd: GitCommand, path: &str, orig_path: Option<&str>) -> GitCommand {
    let cmd = cmd.arg("--").arg(literal_pathspec(path));
    match orig_path {
        Some(orig) => cmd.arg(literal_pathspec(orig)),
        None => cmd,
    }
}

/// Same diff as [`file_diff`], returned unparsed.
///
/// Partial staging rebuilds patches from these bytes ([`crate::patch`]), so
/// both must run the exact same command: hunk and line indices only mean
/// something against the bytes they came from.
pub async fn file_diff_raw(
    executor: &GitExecutor,
    workdir: &Path,
    target: &DiffTarget,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, GitError> {
    // A trailing `/` is a nested repository git would not open;
    // `--no-index` on it prints nothing. The row reads [`embedded`] instead.
    if let DiffTarget::Untracked { path } = target
        && path.ends_with('/')
    {
        return Ok(Vec::new());
    }
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
            for_paths(c, path, orig_path.as_deref())
        }
        DiffTarget::Range {
            from,
            to,
            path,
            orig_path,
        } => {
            let c = base
                .args([
                    "diff-tree",
                    "-r",
                    "--no-commit-id",
                    "-p",
                    "--no-ext-diff",
                    "--find-renames",
                ])
                .args([from.to_hex(), to.to_hex()]);
            for_paths(c, path, orig_path.as_deref())
        }
        DiffTarget::Choice {
            oids,
            path,
            orig_path,
        } => {
            // Each patch is headed by its commit (`FilePatch::from_commit`).
            // Oldest first, the way a cherry-pick would apply them.
            let c = base
                .args([
                    "log",
                    "--no-walk=unsorted",
                    "-p",
                    "--no-ext-diff",
                    "--find-renames",
                    "--diff-merges=first-parent",
                    "--format=%x01%h %s",
                ])
                .args(oids.iter().rev().map(Oid::to_hex));
            for_paths(c, path, orig_path.as_deref())
        }
        DiffTarget::Staged { path, orig_path } => for_paths(
            base.args(["diff", "--cached", "--no-ext-diff", "--find-renames"]),
            path,
            orig_path.as_deref(),
        ),
        DiffTarget::Unstaged { path } => base
            .args(["diff", "--no-ext-diff", "--"])
            .arg(literal_pathspec(path)),
        DiffTarget::Untracked { path } => {
            // Exit 1 (the sides differ) is the normal case. The arguments
            // are file names, not pathspecs — no magic prefix.
            let out = executor
                .run_unchecked(
                    base.answers_by_code(1)
                        .args(["diff", "--no-ext-diff", "--no-index", "--", "/dev/null"])
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

/// What a stage of a directory git would not open would record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Embedded {
    /// The commit the index entry would point at — the HEAD of the
    /// repository sitting there.
    On(Oid),
    /// A repository with no commit yet; `git add` refuses such a path.
    Unborn,
}

/// What a repository inside the working copy (`vendor/nest/`) is standing
/// on — a `git add` of it writes a gitlink naming that commit.
///
/// `rev-parse` walks up, so a directory that is not a repository of its own
/// answers with the outer one's HEAD; `--show-prefix` must come back empty
/// for the answer to count. `None` for every path this cannot be said about.
///
/// Exit 128 is an unborn HEAD, an answer; both lines print either way.
pub async fn embedded(
    executor: &GitExecutor,
    workdir: &Path,
    path: &str,
    cancel: &CancellationToken,
) -> Option<Embedded> {
    let cmd = GitCommand::new()
        .cwd(workdir.join(path))
        .answers_by_code(128)
        .args(["rev-parse", "--show-prefix", "HEAD"]);
    let out = executor.run_unchecked(cmd, cancel).await.ok()?;
    let text = out.stdout_utf8();
    let mut lines = text.lines();
    if !lines.next()?.trim().is_empty() {
        return None;
    }
    // The second line is the commit where there is one, and the literal
    // `HEAD` back again where there is not.
    Some(
        match lines
            .next()
            .and_then(|l| Oid::from_hex(l.trim().as_bytes()).ok())
        {
            Some(oid) => Embedded::On(oid),
            None => Embedded::Unborn,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refusing;

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
    fn parse_details_rejects_short_input() {
        assert!(parse_details(b"garbage").is_none());
    }

    /// Ten NULs and a newline, then whatever the file list is.
    fn combined(message: &str, files: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for field in [
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "Alice",
            "alice@example.com",
            "1700000000",
            "Alice",
            "alice@example.com",
            "1700000000",
            "",
            message,
        ] {
            bytes.extend_from_slice(field.as_bytes());
            bytes.push(0);
        }
        if !files.is_empty() {
            bytes.push(b'\n');
            bytes.extend_from_slice(files);
        }
        bytes
    }

    #[test]
    fn the_record_and_the_file_list_are_cut_apart_at_the_tenth_nul() {
        let bytes = combined("subject\n", b"M\0src/main.rs\0");
        let (record, files) = split_record(&bytes).unwrap();
        assert_eq!(parse_details(record).unwrap().message, "subject");
        let changes = parse_name_status(files).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "src/main.rs");
    }

    #[test]
    fn a_message_that_reads_like_a_status_line_stays_in_the_message() {
        let bytes = combined("subject\n\nM\tsrc/main.rs\n", b"M\0src/main.rs\0");
        let (record, files) = split_record(&bytes).unwrap();
        let d = parse_details(record).unwrap();
        assert_eq!(d.message, "subject\n\nM\tsrc/main.rs");
        assert_eq!(parse_name_status(files).unwrap().len(), 1);
    }

    #[test]
    fn a_commit_that_changed_nothing_ends_at_the_record() {
        let bytes = combined("empty on purpose\n", b"");
        let (record, files) = split_record(&bytes).unwrap();
        assert_eq!(parse_details(record).unwrap().message, "empty on purpose");
        assert!(files.is_empty());
        assert!(parse_name_status(files).unwrap().is_empty());
    }

    #[test]
    fn split_record_rejects_output_with_too_few_fields() {
        assert!(split_record(b"one\0two\0").is_none());
    }

    /// `log --no-walk` given no ids reads HEAD, so an empty choice must not
    /// reach git; an ask that slipped through would fail on the spawn.
    #[tokio::test]
    async fn a_choice_of_nothing_reads_nothing_and_asks_git_nothing() {
        let (exec, asked) = refusing::git();

        let files = union_files(&exec, &refusing::nowhere(), &[], &CancellationToken::new())
            .await
            .unwrap();

        assert!(files.is_empty());
        assert_eq!(asked.count(), 0, "nothing was asked of git");
    }
}
