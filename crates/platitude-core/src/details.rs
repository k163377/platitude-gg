//! Commit details (metadata + changed files) and on-demand file diffs.
//!
//! Merge commits are diffed against their **first parent** (the common GUI
//! convention). Every diff run — the patch producers and the
//! `--name-status` file list alike — pins `--no-ext-diff` and the
//! standard `a/ b/` prefixes, so the parsers see a stable shape
//! regardless of user config.

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

/// Arguments pinning diff output shape against user configuration.
const DIFF_SHAPE_ARGS: [&str; 6] = [
    "-c",
    "diff.noprefix=false",
    "-c",
    "diff.mnemonicPrefix=false",
    "-c",
    "diff.external=",
];

/// Whom a message credits: this parser hands git's packed trailer field
/// straight to `parse_co_authors`. Reading the same lines out of text
/// nobody has committed yet is [`crate::trailers::co_authors_in`].
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

/// Loads commit metadata and its changed-file list.
///
/// **One invocation, not two.** `show` prints the file list after the
/// format expansion, so asking for both together spares the details pane
/// a second process — and on Windows the process is the expensive part
/// of a 100ms interaction budget: git's own work in this call is a
/// rounding error beside starting it (`process::program`,
/// ci/baseline/code-costs-windows-x64.md).
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
        // Merges are read against their first parent, as everywhere else
        // here. It has to be said: left alone, `show` prints no file list
        // for a merge at all.
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
/// `--name-status` bytes behind it.
///
/// The cut is counted in NULs — the record is exactly [`DETAILS_FIELDS`] of
/// them — rather than found by looking for something that reads like a
/// status line. A commit message is free to contain a line spelled
/// `M\tsrc/main.rs`, and a scan would file it under changed files.
/// `show` writes one newline between the record and the list.
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

/// A record marker no status token can begin with, so a log covering
/// several commits can be cut back into them. What lies between two
/// records is `<status>\0<path>\0` pairs, and a status is an ASCII
/// capital (measured, git 2.55).
const UNION_MARK: u8 = 0x01;

/// How many commit ids go on one command line. The ids **are** the
/// arguments and Windows caps a command line at 32k, while one Shift
/// click can sweep a choice over more rows than that
/// (`GraphModel::oids_between`) — so the reading is chunked rather than
/// capped, and every choice a person would actually make is still one
/// invocation.
const UNION_CHUNK: usize = 200;

/// The files a set of commits changed, each against its own first
/// parent, merged into one list.
///
/// **Not a range.** A graph's rows are a walk over every branch, so two
/// rows next to each other need not be parent and child and "the commits
/// between" is no git range at all; what is well defined is what each of
/// these commits did (デザイン規約 §複数のコミットを選ぶ). A path several of
/// them touched is listed once, wearing the status of the first commit
/// asked for — callers pass them newest first, so that is the most
/// recent thing to have happened to it.
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
            // The order asked for is the order the graph stands in.
            // Plain `--no-walk` re-sorts by date, which would put the
            // status of a path onto whichever commit git thinks is
            // newest rather than whichever the reader is looking at.
            "--no-walk=unsorted",
            "-z",
            "-r",
            "--name-status",
            "--find-renames",
            "--diff-merges=first-parent",
            // Nothing but the marker: this reading wants the files, and
            // the rows naming the commits are already on screen.
            "--format=%x01",
        ])
        .args(oids.iter().map(Oid::to_hex));
    let out = executor.run(cmd, cancel).await?;
    let mut changes = Vec::new();
    // The first split is what stands before the first record: nothing.
    for record in out.stdout.split(|b| *b == UNION_MARK).skip(1) {
        // What `log` puts between the format expansion and the file list
        // is `\0\n` — the NUL `-z` terminates the record with, then the
        // newline it writes after any format (measured, git 2.55). The
        // NUL alone would come out as an empty token, which the parser
        // drops; the newline would arrive glued to the first status.
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

/// The files that differ between two commits — what "these two" means
/// where a choice holds exactly two (デザイン規約 §複数のコミットを選ぶ).
///
/// A tree against a tree, so it is answerable for any pair whether or
/// not one is an ancestor of the other. **It is not [`union_files`] of
/// the same two**: this one carries whatever unselected commits did
/// between them and drops what the older of the two did itself, that
/// being the side it is measured from.
pub async fn compare_files(
    executor: &GitExecutor,
    workdir: &Path,
    from: &Oid,
    to: &Oid,
    cancel: &CancellationToken,
) -> Result<Vec<FileChange>, GitError> {
    let from_hex = from.to_hex();
    let to_hex = to.to_hex();
    // The same plumbing the one-commit list runs through, so the two
    // agree about renames: what this list calls a rename is what the
    // patch behind the row will be asked for (`DiffTarget::Range`).
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
    /// One file between two commits — what a choice of exactly two
    /// commits reads (デザイン規約 §複数のコミットを選ぶ). Two trees, so
    /// neither has to be an ancestor of the other.
    Range {
        from: Oid,
        to: Oid,
        path: String,
        /// Source path where the file list reported a rename/copy.
        orig_path: Option<String>,
    },
    /// One file as each of several chosen commits changed it, one patch
    /// after another in walk order — what a choice of three or more reads
    /// (デザイン規約 §複数のコミットを選ぶ).
    ///
    /// **Not one diff of two states.** The file list above it is what
    /// these commits did, so the patches behind a row of it are theirs
    /// too: a comparison across the span would carry whatever unchosen
    /// commits stand in between, and this is the reading a cherry-pick of
    /// the same choice would actually apply.
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
///
/// Stable *within one run*: `DefaultHasher`'s algorithm is not promised
/// across std releases, which is enough here because every value is
/// compared against one the same binary made. Persisting a fingerprint, or
/// comparing across processes, needs a digest that promises more — and
/// that change stops at this function. The crate's other staleness hashes
/// stay their own on purpose, the nearest being
/// [`crate::highlight::LexCache`]'s source hash, which can never be asked
/// to outlive the run it was made in (rules-refs/core.md).
pub fn fingerprint(raw: &[u8]) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    raw.hash(&mut hasher);
    hasher.finish()
}

/// Closes a diff command with the file it is of: the pathspec end every
/// one of them takes, and the source name as well where the file list
/// reported a rename — git is told both, or it has no pair to match.
fn for_paths(cmd: GitCommand, path: &str, orig_path: Option<&str>) -> GitCommand {
    let cmd = cmd.arg("--").arg(literal_pathspec(path));
    match orig_path {
        Some(orig) => cmd.arg(literal_pathspec(orig)),
        None => cmd,
    }
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
    // A path git spells with a trailing `/` is a directory it would not
    // open — a repository of its own inside the working copy — and there
    // is no patch of it to ask for: `--no-index` against a directory
    // answers `Could not access` and prints nothing (measured). What the
    // row has to say instead is [`embedded`].
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
            // Each patch is named before it, so the pane can say which
            // commit it is of (`FilePatch::from_commit`). Oldest first —
            // stacked patches read the way the history ran, and the way a
            // cherry-pick would apply them.
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
            // `--no-index` renders file content as an all-additions patch;
            // it exits 1 when the sides differ, which is the normal case.
            // Its arguments are filenames, not pathspecs — no magic prefix.
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
    /// A repository with no commit yet. `git add` refuses such a path
    /// (`does not have a commit checked out`), so there is nothing this
    /// repository could record for it (measured).
    Unborn,
}

/// What the one entry git answers with for a repository inside the
/// working copy (`vendor/nest/`) is standing on.
///
/// A `git add` of that path writes a **gitlink**: one index entry of mode
/// 160000 naming the commit that repository's HEAD is on, and never the
/// files under it, which belong to that repository (measured). So the
/// commit is the whole of what this repository would keep of it.
///
/// **The answer counts only when it came from that directory.**
/// `rev-parse` walks up, so asked in a directory that is *not* a
/// repository of its own it answers with the repository above — whose
/// HEAD has nothing to do with the row (measured: a plain directory
/// answered with the outer repository's HEAD). `--show-prefix` rides
/// along and says which happened, in git's own terms rather than by
/// comparing two spellings of a path: empty is the root of the work tree
/// the answer came from, and anything else is the way down to the
/// directory asked about from a repository further up. `None` is what a
/// caller gets for every path this cannot be said about.
///
/// The exit code is the answer, not a failure: 128 is what an unborn
/// HEAD comes back as, and it is nothing for the command log to raise
/// itself over — this is only ever asked about a path git itself
/// declined to open. Both lines are printed either way (measured).
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
    // `HEAD` back again where there is not (measured).
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
        // Nothing stops a commit from describing its own diff. Counting
        // NULs is what keeps this out of the file table; scanning for a
        // status letter would put `src/main.rs` there twice.
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
}
