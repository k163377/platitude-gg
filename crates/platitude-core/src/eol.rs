//! Line endings: what git already did, made visible.
//!
//! Nothing in this module converts anything, and nothing writes git config.
//! What lands in the index is git's decision (`core.autocrlf`, `core.eol`,
//! `.gitattributes`); all that happens here is reading the bytes git printed
//! and naming what is in them. Every line-ending accident the other GUIs are
//! known for comes from a "fix it for me" button that rewrites a global
//! setting, so there is no such button and no such code path.
//!
//! Four cases are worth saying out loud (internal-docs/P3-確認事項.md):
//!
//! | | case | decided by |
//! |---|---|---|
//! | (a) | an existing file's endings flip | the patch bytes — exact |
//! | (b) | the change makes a file mixed, or more mixed | the patch bytes — exact |
//! | (c) | a new file does not match its neighbours | a sample — an estimate |
//! | (d) | a file that had no ending gains its first | a sample — an estimate |
//!
//! (a) and (b) are settled here by [`read`]. (c) and (d) need to know what
//! the files around this one look like, which the patch cannot say; [`read`]
//! reports the shape it found and the caller pairs it with a baseline.
//!
//! # Why the patch bytes and not `ls-files --eol`
//!
//! `ls-files --eol` answers "what does this file look like now", which is
//! not the question. A file that was already mixed before anyone touched it
//! is not this change's fault and gets no notice; the same file gets one the
//! moment the change adds a line that disagrees with it. Only the diff knows
//! which lines are new.
//!
//! # What git hands over (measured, git 2.55)
//!
//! - The terminator is part of the content line: a CRLF line prints as
//!   `+one\r\n`, so the `\r` sits immediately before the `\n`.
//! - A whole-file flip prints every line as `-` then `+` with no context —
//!   which is what makes "the endings changed" distinguishable from "some
//!   lines changed".
//! - `\ No newline at end of file` follows the line it is about and belongs
//!   to whichever side that line was on. It appears for the old side, the
//!   new side, or both.
//! - Under `core.autocrlf=true` a worktree turned to CRLF produces an
//!   **empty** diff (git compares in index space) while status still calls
//!   the file modified, and an untracked CRLF file renders through
//!   `--no-index` with its CRs already normalised away. So on that setting
//!   these cases mostly cannot arise — correctly, because git is converting
//!   and there is nothing to warn about. The exception is an index blob that
//!   already holds CRs, where git converts nothing and the CRs are real data.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor, literal_pathspec};

/// A line terminator this module can name.
///
/// Old-Mac CR-only files are not a case: a file with no `\n` in it reads as
/// one line with no terminator, which is also how git counts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eol {
    Lf,
    Crlf,
}

impl Eol {
    /// The spelling a person reads. **Not a code chip** — the chip shape is
    /// lowercase monospace and an all-caps abbreviation does not sit in it
    /// (デザイン規約 §git 用語のコード表記).
    pub fn as_str(self) -> &'static str {
        match self {
            Eol::Lf => "LF",
            Eol::Crlf => "CRLF",
        }
    }
}

/// What one file's patch says. (c) and (d) still need a baseline before
/// anything is shown; the rest are final.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// Nothing to say about this file.
    Quiet,
    /// (a) every line of an existing file changed its ending.
    Flipped { from: Eol, to: Eol },
    /// (b) the change adds lines that disagree with the file they land in.
    Mixed { lines: u32, added: Eol, file: Eol },
    /// (c) a file that did not exist before, whose endings are uniform.
    NewFile { eol: Eol },
    /// (d) a file that held no line ending at all now has one.
    FirstEnding { eol: Eol },
}

impl Reading {
    /// Whether this reading was decided by the patch bytes alone.
    ///
    /// History diffs show only these: (c) and (d) would need the files
    /// **around** the changed one as they stood at that commit, which is a
    /// different tree from the one on disk.
    pub fn is_exact(self) -> bool {
        matches!(self, Reading::Flipped { .. } | Reading::Mixed { .. })
    }
}

/// Where the files a baseline was drawn from sat, so the notice can name
/// the range it is actually speaking for instead of implying a wider one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// Every sample shares this extension and sits in the same directory.
    Here(String),
    /// Every sample shares this extension.
    Ext(String),
    /// The samples have nothing in common but the repository.
    Repo,
}

/// What the files around a path look like. An estimate, and only ever
/// consulted for the two cases that have nothing exact to go on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Baseline {
    pub eol: Eol,
    pub scope: Scope,
}

/// A settled statement about one file, ready to be worded.
///
/// The two estimates carry the baseline they were measured against, because
/// the sentence has to name the range the sample actually covered rather
/// than imply the whole repository agrees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// (a) `Line endings change · CRLF → LF`
    Flipped { from: Eol, to: Eol },
    /// (b) `Mixed line endings · 3 added lines use CRLF, this file uses LF`
    Mixed { lines: u32, added: Eol, file: Eol },
    /// (c) `New file uses CRLF · other .kt files here look like LF`
    NewFile { eol: Eol, baseline: Baseline },
    /// (d) `First line ending in this file · CRLF · …`
    FirstEnding { eol: Eol, baseline: Baseline },
}

/// Pairs a reading with a baseline. `None` is silence, and every way of not
/// knowing ends up here: git ruling the path out, no baseline to compare
/// against, or a file that agrees with its neighbours after all.
///
/// A history diff simply arrives with no baseline, which is what keeps the
/// two estimates out of it: the files around a changed one, as they stood at
/// that commit, are not the files on disk.
pub fn settle(reading: Reading, baseline: Option<&Baseline>) -> Option<Notice> {
    match reading {
        Reading::Quiet => None,
        Reading::Flipped { from, to } => Some(Notice::Flipped { from, to }),
        Reading::Mixed { lines, added, file } => Some(Notice::Mixed { lines, added, file }),
        Reading::NewFile { eol } => match baseline {
            Some(b) if b.eol != eol => Some(Notice::NewFile {
                eol,
                baseline: b.clone(),
            }),
            _ => None,
        },
        Reading::FirstEnding { eol } => match baseline {
            Some(b) if b.eol != eol => Some(Notice::FirstEnding {
                eol,
                baseline: b.clone(),
            }),
            _ => None,
        },
    }
}

/// What git's own settings decide about a path before anything is sampled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ruling {
    /// `.gitattributes` says the path is not text. The only exclusion there
    /// is: build output, test data and the rest are `.gitattributes`'
    /// business, not a list this app keeps.
    NotText,
    /// git decides the stored endings itself, so a new file cannot disagree
    /// with its neighbours and there is nothing to compare.
    Normalised,
    /// Nothing decides it. The files around this one are the only answer.
    Open,
}

/// One file of a patch and what its lines said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sighting {
    /// The path as the patch header spelled it, prefix stripped.
    ///
    /// Not unquoted: git C-quotes headers for paths holding quotes, control
    /// characters or backslashes, and the patch parser next door leaves
    /// those alone too. A caller matching these against a known set of paths
    /// simply misses such a file, which costs a mark rather than putting one
    /// on the wrong row.
    pub path: String,
    pub reading: Reading,
}

/// Reads every file in a patch. Empty when there is nothing to say.
///
/// Accepts the multi-file output of a whole-tree `git diff` as readily as
/// one file's, so the pane that opened a single diff and the pass that marks
/// every pending file run the same code over the same shape of bytes.
pub fn read(raw: &[u8]) -> Vec<Sighting> {
    let mut out = Vec::new();
    let mut scan: Option<Scan> = None;

    let mut rest = raw;
    while !rest.is_empty() {
        let (line, tail, terminated) = match rest.iter().position(|b| *b == b'\n') {
            Some(i) => (&rest[..i], &rest[i + 1..], true),
            None => (rest, &rest[rest.len()..], false),
        };
        rest = tail;

        // Inside a hunk every line is content until the counts run out, so
        // the header shapes below cannot be confused with a context line
        // that happens to start with `diff --git`.
        if let Some(s) = scan.as_mut()
            && s.in_hunk()
        {
            s.content(line, terminated);
            continue;
        }

        if line.starts_with(b"diff --git ") {
            flush(&mut out, scan.take());
            scan = Some(Scan::default());
            continue;
        }
        if line.starts_with(b"diff --cc ") || line.starts_with(b"diff --combined ") {
            flush(&mut out, scan.take());
            let mut s = Scan {
                path: text_after(line, b"diff --cc ")
                    .or_else(|| text_after(line, b"diff --combined ")),
                ..Scan::default()
            };
            // More than one old side: no single "before" to compare against,
            // and the file is mid-conflict anyway.
            s.quiet = true;
            scan = Some(s);
            continue;
        }
        if let Some(path) = text_after(line, b"* Unmerged path ") {
            flush(&mut out, scan.take());
            scan = Some(Scan {
                path: Some(path),
                quiet: true,
                ..Scan::default()
            });
            continue;
        }

        let Some(s) = scan.as_mut() else {
            continue;
        };
        s.header(line);
    }
    flush(&mut out, scan);
    out
}

/// Reads a patch that covers one file. [`Reading::Quiet`] when it covers
/// none or more than one, since neither answers "what about this file".
pub fn read_one(raw: &[u8]) -> Reading {
    match read(raw).as_slice() {
        [only] => only.reading,
        _ => Reading::Quiet,
    }
}

/// How many usable samples a baseline needs.
const SAMPLES: usize = 3;
/// How many files may be read looking for them. Neighbours that turn out to
/// be unusable are replaced, but not forever.
const READS: usize = 9;
/// A file this size is not opened for a vote. `ls-files --eol` reads the
/// whole worktree file to fill its `w/` column — 213ms for one 120MB file,
/// measured — and one neighbour's opinion is not worth that.
const SAMPLE_MAX_BYTES: u64 = 1 << 20;

/// What git's settings say about one path.
///
/// Two spawns, both cheap, and the answer is what lets the sampling below
/// be skipped entirely: with `core.autocrlf` converting or `text` set in
/// `.gitattributes`, git normalises what it stores, so a new file's endings
/// cannot disagree with anything and no neighbour needs reading.
pub async fn ruling(
    executor: &GitExecutor,
    workdir: &Path,
    path: &str,
    cancel: &CancellationToken,
) -> Result<Ruling, GitError> {
    let attrs = attributes(executor, workdir, path, cancel).await?;
    if attrs.not_text {
        return Ok(Ruling::NotText);
    }
    if attrs.decided || normalises(executor, workdir, cancel).await? {
        return Ok(Ruling::Normalised);
    }
    Ok(Ruling::Open)
}

/// What the files around `path` look like, or `None` when the answer is
/// "unknown" and the right thing to do is say nothing.
///
/// Unknown covers more ground than it sounds like: git already deciding the
/// endings, too few neighbours worth reading, and a sample that does not
/// agree with itself all come back the same way, because they all mean the
/// app has no business naming a house style.
pub async fn baseline(
    executor: &GitExecutor,
    workdir: &Path,
    path: &str,
    cancel: &CancellationToken,
) -> Result<Option<Baseline>, GitError> {
    match ruling(executor, workdir, path, cancel).await? {
        Ruling::NotText | Ruling::Normalised => Ok(None),
        Ruling::Open => sample(executor, workdir, path, cancel).await,
    }
}

struct Attributes {
    /// `-text`: git is told this path is not text.
    not_text: bool,
    /// `text` or `eol` is spelled out, so what gets stored is settled.
    decided: bool,
}

async fn attributes(
    executor: &GitExecutor,
    workdir: &Path,
    path: &str,
    cancel: &CancellationToken,
) -> Result<Attributes, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["check-attr", "-z", "text", "eol", "--"])
        .arg(path);
    let out = executor.run(cmd, cancel).await?;
    // `-z` prints one `path\0attr\0value\0` triple per attribute asked for.
    let fields: Vec<&[u8]> = out.stdout.split(|b| *b == 0).collect();
    let mut attrs = Attributes {
        not_text: false,
        decided: false,
    };
    for triple in fields.chunks(3) {
        let [_, attr, value] = triple else { continue };
        match (
            String::from_utf8_lossy(attr).as_ref(),
            String::from_utf8_lossy(value).as_ref(),
        ) {
            ("text", "unset") => attrs.not_text = true,
            ("text", "set") => attrs.decided = true,
            ("eol", "lf" | "crlf") => attrs.decided = true,
            _ => {}
        }
    }
    Ok(attrs)
}

/// Whether `core.autocrlf` converts on the way into the index.
///
/// **`--get-regexp` answers from every config level at once, lowest first,
/// and the last one is the effective value.** Reading the first match makes
/// every repository on Windows look like it normalises: the Git for Windows
/// installer writes `core.autocrlf=true` into the system config, and a
/// repository that sets `false` for itself shows up as the second record
/// (measured on this machine — system `true`, repo `false`, in that order).
///
/// `core.eol` is read in the same breath and deliberately not consulted:
/// it only takes effect where a path is already text by attribute or by
/// `autocrlf`, both of which have answered by then, so on its own it never
/// decides anything.
async fn normalises(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        // Neither key being set answers with code 1, which is an answer.
        .answers_by_code()
        .args(["config", "-z", "--get-regexp", r"^core\.(autocrlf|eol)$"]);
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code == 1 {
        return Ok(false);
    }
    if out.code != 0 {
        return Err(GitError::Failed {
            command: "git config --get-regexp core.autocrlf".to_string(),
            code: out.code,
            stderr: out.failure_message(),
        });
    }
    let mut effective = false;
    for record in out.stdout.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let text = String::from_utf8_lossy(record);
        let Some((key, value)) = text.split_once('\n') else {
            continue;
        };
        if key.trim() == "core.autocrlf" {
            // `input` converts on the way in and not on the way out, which
            // is still git deciding what gets stored.
            effective = matches!(value.trim(), "true" | "input");
        }
    }
    Ok(effective)
}

/// Three neighbours' worth of opinion, or `None`.
async fn sample(
    executor: &GitExecutor,
    workdir: &Path,
    path: &str,
    cancel: &CancellationToken,
) -> Result<Option<Baseline>, GitError> {
    let (dir, ext) = split_dir_ext(path);
    let mut picked: Vec<(String, Group)> = Vec::new();
    let held = |picked: &[(String, Group)], p: &String| picked.iter().any(|(q, _)| q == p);

    // Same extension in the same directory first: a repository with a house
    // style usually has it per directory, and this listing is the cheapest.
    if !ext.is_empty() {
        let here = list(executor, workdir, Some(dir), cancel).await?;
        let neighbours: Vec<String> = here
            .into_iter()
            .filter(|p| p != path && parent_of(p) == dir && extension_of(p) == ext)
            .collect();
        take_spread(&neighbours, READS, Group::Here, &mut picked);
    }

    // Then the same extension anywhere, then anything at all. A file with no
    // extension has no first two groups and goes straight to the last.
    if picked.len() < READS {
        let all = list(executor, workdir, None, cancel).await?;
        if !ext.is_empty() {
            let by_ext: Vec<String> = all
                .iter()
                .filter(|p| *p != path && !held(&picked, p) && extension_of(p) == ext)
                .cloned()
                .collect();
            take_spread(&by_ext, READS - picked.len(), Group::Ext, &mut picked);
        }
        if picked.len() < READS {
            let rest: Vec<String> = all
                .into_iter()
                .filter(|p| p != path && !held(&picked, p))
                .collect();
            take_spread(&rest, READS - picked.len(), Group::Any, &mut picked);
        }
    }

    // Reading a file is the expensive part, so the ones that cannot be read
    // usefully are dropped before git is asked, not after. The group travels
    // with the path so dropping one cannot shift what the rest claim.
    picked.retain(|(p, _)| readable(workdir, p));
    if picked.len() < SAMPLES {
        return Ok(None);
    }

    let paths: Vec<String> = picked.iter().map(|(p, _)| p.clone()).collect();
    let mut votes = Tally::default();
    let mut counted = 0usize;
    // The notice may only claim the range every voter actually came from.
    let mut widest = Group::Here;
    for (index, eol) in worktree_endings(executor, workdir, &paths, cancel).await? {
        if counted == SAMPLES {
            break;
        }
        votes.add(eol);
        counted += 1;
        if let Some((_, group)) = picked.get(index) {
            widest = widest.max(*group);
        }
    }
    if counted < SAMPLES {
        return Ok(None);
    }
    let Some(eol) = votes.majority() else {
        return Ok(None);
    };
    let scope = match widest {
        Group::Here => Scope::Here(ext.to_string()),
        Group::Ext => Scope::Ext(ext.to_string()),
        Group::Any => Scope::Repo,
    };
    Ok(Some(Baseline { eol, scope }))
}

/// How far from the file a sample had to be drawn. Ordered widest-last: the
/// scope a notice may claim is the widest any of its voters came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Group {
    Here,
    Ext,
    Any,
}

/// Index paths, optionally under one directory. The index only — asking for
/// endings here would read every worktree file in the repository (24.7s on
/// the 106k-file reference repository, measured, against 42ms for a handful
/// of settled paths).
async fn list(
    executor: &GitExecutor,
    workdir: &Path,
    dir: Option<&str>,
    cancel: &CancellationToken,
) -> Result<Vec<String>, GitError> {
    let mut cmd = GitCommand::new().cwd(workdir).args(["ls-files", "-z"]);
    // A literal directory pathspec matches its whole subtree, which is
    // narrower than the index and needs no glob escaping.
    if let Some(dir) = dir.filter(|d| !d.is_empty()) {
        cmd = cmd.arg("--").arg(literal_pathspec(dir));
    }
    let out = executor.run(cmd, cancel).await?;
    Ok(out
        .stdout
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf8_lossy(p).into_owned())
        .collect())
}

/// The worktree ending of each path git can name one for, with the index it
/// came in at. Unusable samples are simply absent.
async fn worktree_endings(
    executor: &GitExecutor,
    workdir: &Path,
    paths: &[String],
    cancel: &CancellationToken,
) -> Result<Vec<(usize, Eol)>, GitError> {
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .args(["ls-files", "--eol", "-z", "--"]);
    for p in paths {
        cmd = cmd.arg(literal_pathspec(p));
    }
    let out = executor.run(cmd, cancel).await?;

    let mut found: Vec<(usize, Eol)> = Vec::new();
    for record in out.stdout.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let text = String::from_utf8_lossy(record);
        // `i/lf    w/crlf  attr/                 \t<path>`
        let Some((columns, path)) = text.split_once('\t') else {
            continue;
        };
        let worktree = columns
            .split_whitespace()
            .find_map(|c| c.strip_prefix("w/"))
            .unwrap_or_default();
        // Empty means the path is not checked out, `mixed` has no single
        // answer to give, and `-text` and `none` have nothing to say.
        let eol = match worktree {
            "lf" => Eol::Lf,
            "crlf" => Eol::Crlf,
            _ => continue,
        };
        if let Some(index) = paths.iter().position(|p| p == path) {
            found.push((index, eol));
        }
    }
    // git answers in its own order; the caller's ranking is the one that
    // decides which three get to vote.
    found.sort_by_key(|(index, _)| *index);
    Ok(found)
}

/// Whether a candidate is worth opening: present, a file, and small enough
/// that reading it is not the most expensive thing the app does today.
fn readable(workdir: &Path, path: &str) -> bool {
    let Ok(meta) = std::fs::metadata(workdir.join(path)) else {
        return false;
    };
    meta.is_file() && meta.len() <= SAMPLE_MAX_BYTES
}

/// Takes up to `want` entries spread across the list rather than the first
/// `want`. Index order is alphabetical, so the head of a repository is all
/// one corner of it; a spread is still deterministic but is a sample of the
/// repository instead of a sample of its first directory.
fn take_spread(from: &[String], want: usize, group: Group, into: &mut Vec<(String, Group)>) {
    if want == 0 || from.is_empty() {
        return;
    }
    if from.len() <= want {
        into.extend(from.iter().map(|p| (p.clone(), group)));
        return;
    }
    let stride = from.len() / want;
    for step in 0..want {
        if let Some(p) = from.get(step * stride) {
            into.push((p.clone(), group));
        }
    }
}

/// What a baseline may be reused for: everything of the same extension in
/// the same directory has the same neighbours and the same answer.
pub fn cache_key(path: &str) -> (String, String) {
    let (dir, ext) = split_dir_ext(path);
    (dir.to_string(), ext.to_string())
}

/// The directory a path sits in and the extension of its file name, both
/// empty when it has none.
fn split_dir_ext(path: &str) -> (&str, &str) {
    (parent_of(path), extension_of(path))
}

fn parent_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
}

/// The extension of the file name, without the dot. A leading dot is a
/// name, not an extension: `.gitignore` has none.
fn extension_of(path: &str) -> &str {
    let name = match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    };
    match name.rfind('.') {
        Some(i) if i > 0 => &name[i + 1..],
        _ => "",
    }
}

fn flush(out: &mut Vec<Sighting>, scan: Option<Scan>) {
    let Some(s) = scan else { return };
    let reading = s.settle();
    if reading == Reading::Quiet {
        return;
    }
    out.push(Sighting {
        path: s.path.unwrap_or_default(),
        reading,
    });
}

/// Which column of the patch a line came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Minus,
    Plus,
    Context,
}

/// How many lines of each ending one side holds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Tally {
    lf: u32,
    crlf: u32,
}

impl Tally {
    fn add(&mut self, eol: Eol) {
        match eol {
            Eol::Lf => self.lf += 1,
            Eol::Crlf => self.crlf += 1,
        }
    }

    fn take(&mut self, eol: Eol) {
        match eol {
            Eol::Lf => self.lf = self.lf.saturating_sub(1),
            Eol::Crlf => self.crlf = self.crlf.saturating_sub(1),
        }
    }

    fn count(self, eol: Eol) -> u32 {
        match eol {
            Eol::Lf => self.lf,
            Eol::Crlf => self.crlf,
        }
    }

    fn is_empty(self) -> bool {
        self.lf == 0 && self.crlf == 0
    }

    /// The one ending present, when the other is absent.
    fn sole(self) -> Option<Eol> {
        match (self.lf, self.crlf) {
            (0, 0) => None,
            (_, 0) => Some(Eol::Lf),
            (0, _) => Some(Eol::Crlf),
            _ => None,
        }
    }

    /// The ending that outnumbers the other. A tie has no answer.
    fn majority(self) -> Option<Eol> {
        match self.lf.cmp(&self.crlf) {
            std::cmp::Ordering::Greater => Some(Eol::Lf),
            std::cmp::Ordering::Less => Some(Eol::Crlf),
            std::cmp::Ordering::Equal => None,
        }
    }
}

#[derive(Debug, Default)]
struct Scan {
    path: Option<String>,
    minus: Tally,
    plus: Tally,
    context: Tally,
    /// The old side's last line carried no terminator at all.
    minus_bare: bool,
    plus_bare: bool,
    /// Binary, combined or unmerged: no line endings to talk about.
    quiet: bool,
    /// The old side is `/dev/null`.
    added: bool,
    /// The new side is `/dev/null`.
    removed: bool,
    old_left: u32,
    new_left: u32,
    last: Option<(Side, Eol)>,
}

impl Scan {
    fn in_hunk(&self) -> bool {
        self.old_left > 0 || self.new_left > 0
    }

    fn header(&mut self, line: &[u8]) {
        if line.starts_with(b"Binary files ") || line.starts_with(b"GIT binary patch") {
            self.quiet = true;
            return;
        }
        if line.starts_with(b"new file mode ") {
            self.added = true;
            return;
        }
        if line.starts_with(b"deleted file mode ") {
            self.removed = true;
            return;
        }
        if let Some(rest) = text_after(line, b"--- ") {
            if rest == "/dev/null" {
                self.added = true;
            }
            return;
        }
        if let Some(rest) = text_after(line, b"+++ ") {
            if rest == "/dev/null" {
                self.removed = true;
            } else {
                self.path = Some(strip_side_prefix(&rest, "b/"));
            }
            return;
        }
        if line.starts_with(b"@@@") {
            // A combined hunk, reached without its `diff --cc` header.
            self.quiet = true;
            return;
        }
        if let Some((old, new)) = hunk_counts(line) {
            self.old_left = old;
            self.new_left = new;
            self.last = None;
        }
    }

    fn content(&mut self, line: &[u8], terminated: bool) {
        // The marker belongs to the line above it and is outside both
        // counts, so it is handled before anything is charged to a side.
        if line.starts_with(b"\\") {
            if let Some((side, eol)) = self.last.take() {
                match side {
                    Side::Minus => {
                        self.minus.take(eol);
                        self.minus_bare = true;
                    }
                    Side::Plus => {
                        self.plus.take(eol);
                        self.plus_bare = true;
                    }
                    Side::Context => {
                        self.context.take(eol);
                        self.minus_bare = true;
                        self.plus_bare = true;
                    }
                }
            }
            return;
        }

        let (side, charge_old, charge_new) = match line.first() {
            Some(b'-') => (Side::Minus, true, false),
            Some(b'+') => (Side::Plus, false, true),
            Some(b' ') => (Side::Context, true, true),
            // An empty line inside a hunk is git's context line for an empty
            // source line with the marker column eaten by a tool in the
            // middle; count it as context so the budgets still land.
            None => (Side::Context, true, true),
            // Anything else means the counts were wrong; leave the hunk and
            // let the header path have the line.
            _ => {
                self.old_left = 0;
                self.new_left = 0;
                self.header(line);
                return;
            }
        };

        if charge_old {
            self.old_left = self.old_left.saturating_sub(1);
        }
        if charge_new {
            self.new_left = self.new_left.saturating_sub(1);
        }

        // Only a terminated line has an ending to classify; an unterminated
        // final line is about to be corrected by `\ No newline` anyway.
        if !terminated {
            self.last = None;
            return;
        }
        let eol = if line.last() == Some(&b'\r') {
            Eol::Crlf
        } else {
            Eol::Lf
        };
        match side {
            Side::Minus => self.minus.add(eol),
            Side::Plus => self.plus.add(eol),
            Side::Context => self.context.add(eol),
        }
        self.last = Some((side, eol));
    }

    fn settle(&self) -> Reading {
        if self.quiet || self.removed {
            return Reading::Quiet;
        }
        // Nothing was added, so nothing this change did can have made the
        // endings worse. Deleting the odd lines out of a mixed file is a
        // repair, not a warning.
        if self.plus.is_empty() {
            return Reading::Quiet;
        }

        if self.added {
            return match (self.plus.sole(), self.plus.majority()) {
                (Some(eol), _) => Reading::NewFile { eol },
                // A new file that arrives already mixed is case (b): the
                // lines that disagree with the rest are all its own.
                (None, Some(file)) => mixed(self.plus, file),
                (None, None) => Reading::Quiet,
            };
        }

        // (d) — the old side had no terminator anywhere. Its only line was
        // the bare one, so `minus_bare` with nothing tallied says it all.
        if self.minus_bare && self.minus.is_empty() && self.context.is_empty() {
            return match self.plus.majority().or_else(|| self.plus.sole()) {
                Some(eol) => Reading::FirstEnding { eol },
                None => Reading::Quiet,
            };
        }

        // (a) — no line survived untouched and each side speaks with one
        // voice. A one-line file whose only line changed its ending lands
        // here too, which is the same statement about a smaller file.
        if self.context.is_empty()
            && let (Some(from), Some(to)) = (self.minus.sole(), self.plus.sole())
            && from != to
        {
            return Reading::Flipped { from, to };
        }

        // (b) — what the file uses is what the untouched lines use. Falling
        // back to the old side and then to the new keeps a fully rewritten
        // file answerable; a tie means the file is already so mixed that
        // there is no "this file uses" to name, and nothing is said.
        let Some(file) = self
            .context
            .majority()
            .or_else(|| self.minus.majority())
            .or_else(|| self.plus.majority())
        else {
            return Reading::Quiet;
        };
        mixed(self.plus, file)
    }
}

/// (b) from a side's tally: the added lines that disagree with the file.
fn mixed(plus: Tally, file: Eol) -> Reading {
    let added = match file {
        Eol::Lf => Eol::Crlf,
        Eol::Crlf => Eol::Lf,
    };
    let lines = plus.count(added);
    if lines == 0 {
        return Reading::Quiet;
    }
    Reading::Mixed { lines, added, file }
}

/// `@@ -a,b +c,d @@` → the old and new line counts. A count left out is 1.
fn hunk_counts(line: &[u8]) -> Option<(u32, u32)> {
    let rest = text_after(line, b"@@ ")?;
    let (ranges, _) = rest.split_once(" @@")?;
    let mut old = None;
    let mut new = None;
    for part in ranges.split(' ') {
        let (slot, digits) = match part.as_bytes().first() {
            Some(b'-') => (&mut old, &part[1..]),
            Some(b'+') => (&mut new, &part[1..]),
            _ => continue,
        };
        let count = match digits.split_once(',') {
            Some((_, n)) => n.parse::<u32>().ok()?,
            None => 1,
        };
        *slot = Some(count);
    }
    Some((old?, new?))
}

/// The text of a line after a fixed prefix, when it starts with one.
fn text_after(line: &[u8], prefix: &[u8]) -> Option<String> {
    let rest = line.strip_prefix(prefix)?;
    Some(String::from_utf8_lossy(rest).trim_end().to_string())
}

fn strip_side_prefix(path: &str, prefix: &str) -> String {
    let cleaned = path.trim_end_matches('\t');
    cleaned.strip_prefix(prefix).unwrap_or(cleaned).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds patch bytes from lines, so a `\r` in a test is visible where
    /// it matters instead of hidden in a raw string literal.
    fn patch(lines: &[&str]) -> Vec<u8> {
        let mut out = Vec::new();
        for l in lines {
            out.extend_from_slice(l.as_bytes());
            out.push(b'\n');
        }
        out
    }

    const HEAD: [&str; 4] = [
        "diff --git a/f.txt b/f.txt",
        "index 4cb29ea..e1587ff 100644",
        "--- a/f.txt",
        "+++ b/f.txt",
    ];

    fn with_head(body: &[&str]) -> Vec<u8> {
        let mut lines = HEAD.to_vec();
        lines.extend_from_slice(body);
        patch(&lines)
    }

    #[test]
    fn a_whole_file_flip_is_read_as_a_flip() {
        // The exact shape git printed for a three-line LF file rewritten
        // with CRLF (measured).
        let raw = with_head(&[
            "@@ -1,3 +1,3 @@",
            "-one",
            "-two",
            "-three",
            "+one\r",
            "+two\r",
            "+three\r",
        ]);
        assert_eq!(
            read_one(&raw),
            Reading::Flipped {
                from: Eol::Lf,
                to: Eol::Crlf
            }
        );
    }

    #[test]
    fn one_crlf_line_in_an_lf_file_is_read_as_mixed() {
        let raw = with_head(&[
            "@@ -1,7 +1,7 @@",
            " a",
            " b",
            " c",
            "-d",
            "+d\r",
            " e",
            " f",
            " g",
        ]);
        assert_eq!(
            read_one(&raw),
            Reading::Mixed {
                lines: 1,
                added: Eol::Crlf,
                file: Eol::Lf
            }
        );
    }

    #[test]
    fn the_file_a_notice_names_is_the_one_the_untouched_lines_use() {
        // Most of the file is rewritten with CRLF but some lines are left
        // alone: what the file "uses" has to come from those, or a change
        // big enough to outvote the file would report itself as normal.
        let mut body = vec!["@@ -1,6 +1,6 @@", " keep", " keep"];
        body.extend(std::iter::repeat_n("-line", 4));
        body.extend(std::iter::repeat_n("+line\r", 4));
        assert_eq!(
            read_one(&with_head(&body)),
            Reading::Mixed {
                lines: 4,
                added: Eol::Crlf,
                file: Eol::Lf
            }
        );
    }

    #[test]
    fn a_file_that_was_already_mixed_says_nothing_until_the_change_adds_to_it() {
        let settled = with_head(&["@@ -1,4 +1,5 @@", " a", " b", " c\r", "-d", "+d", "+e"]);
        assert_eq!(read_one(&settled), Reading::Quiet);

        let worsened = with_head(&["@@ -1,4 +1,5 @@", " a", " b", " c\r", "-d", "+d", "+e\r"]);
        assert_eq!(
            read_one(&worsened),
            Reading::Mixed {
                lines: 1,
                added: Eol::Crlf,
                file: Eol::Lf
            }
        );
    }

    #[test]
    fn a_new_file_reports_the_ending_it_arrived_with() {
        // `--no-index` against /dev/null is how an untracked file is
        // rendered; `new file mode` is what a staged add prints.
        let raw = patch(&[
            "diff --git a/fresh.txt b/fresh.txt",
            "new file mode 100644",
            "index 0000000..e3c39d1",
            "--- /dev/null",
            "+++ b/fresh.txt",
            "@@ -0,0 +1,2 @@",
            "+new\r",
            "+file\r",
        ]);
        assert_eq!(read_one(&raw), Reading::NewFile { eol: Eol::Crlf });
    }

    #[test]
    fn a_new_file_that_arrives_mixed_is_reported_as_mixed() {
        let raw = patch(&[
            "diff --git a/fresh.txt b/fresh.txt",
            "new file mode 100644",
            "index 0000000..e3c39d1",
            "--- /dev/null",
            "+++ b/fresh.txt",
            "@@ -0,0 +1,3 @@",
            "+new",
            "+file\r",
            "+here",
        ]);
        assert_eq!(
            read_one(&raw),
            Reading::Mixed {
                lines: 1,
                added: Eol::Crlf,
                file: Eol::Lf
            }
        );
    }

    #[test]
    fn a_file_with_no_ending_at_all_reports_its_first() {
        // Measured shape: the marker follows the old side's only line.
        let raw = with_head(&[
            "@@ -1 +1 @@",
            "-no-newline-here",
            "\\ No newline at end of file",
            "+no-newline-here",
        ]);
        assert_eq!(read_one(&raw), Reading::FirstEnding { eol: Eol::Lf });

        let crlf = with_head(&[
            "@@ -1 +2 @@",
            "-no-newline-here",
            "\\ No newline at end of file",
            "+no-newline-here\r",
        ]);
        assert_eq!(read_one(&crlf), Reading::FirstEnding { eol: Eol::Crlf });
    }

    #[test]
    fn losing_the_final_newline_is_not_an_ending_change() {
        // The new side's last line has no terminator, so it must not be
        // tallied as LF and turn into a flip or a mixture.
        let raw = with_head(&[
            "@@ -1,2 +1,2 @@",
            " one",
            "-two",
            "+two",
            "\\ No newline at end of file",
        ]);
        assert_eq!(read_one(&raw), Reading::Quiet);
    }

    #[test]
    fn a_deletion_only_change_says_nothing() {
        let raw = with_head(&["@@ -1,3 +1,2 @@", " a", "-b\r", " c"]);
        assert_eq!(read_one(&raw), Reading::Quiet);
    }

    #[test]
    fn a_removed_file_says_nothing() {
        let raw = patch(&[
            "diff --git a/gone.txt b/gone.txt",
            "deleted file mode 100644",
            "index e3c39d1..0000000",
            "--- a/gone.txt",
            "+++ /dev/null",
            "@@ -1,2 +0,0 @@",
            "-new\r",
            "-file\r",
        ]);
        assert_eq!(read_one(&raw), Reading::Quiet);
    }

    #[test]
    fn binary_and_combined_patches_say_nothing() {
        let binary = patch(&[
            "diff --git a/x.png b/x.png",
            "index 1111111..2222222 100644",
            "Binary files a/x.png and b/x.png differ",
        ]);
        assert_eq!(read_one(&binary), Reading::Quiet);

        let combined = patch(&[
            "diff --cc c.txt",
            "index 1111111,2222222..3333333",
            "--- a/c.txt",
            "+++ b/c.txt",
            "@@@ -1,2 -1,2 +1,3 @@@",
            "++mine\r",
            " +theirs",
        ]);
        assert_eq!(read_one(&combined), Reading::Quiet);

        let unmerged = patch(&["* Unmerged path c.txt"]);
        assert_eq!(read_one(&unmerged), Reading::Quiet);
    }

    #[test]
    fn a_content_line_that_looks_like_a_header_stays_content() {
        // The hunk counts say how many lines belong to it, so a context
        // line reading `diff --git …` cannot start a second file.
        let raw = with_head(&[
            "@@ -1,3 +1,3 @@",
            " diff --git a/x b/x",
            "-plain",
            "+plain\r",
            " tail",
        ]);
        let got = read(&raw);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].path, "f.txt");
        assert_eq!(
            got[0].reading,
            Reading::Mixed {
                lines: 1,
                added: Eol::Crlf,
                file: Eol::Lf
            }
        );
    }

    #[test]
    fn a_multi_file_patch_reports_only_the_files_with_something_to_say() {
        let mut raw = with_head(&["@@ -1,2 +1,2 @@", " a", "-b", "+b\r"]);
        raw.extend_from_slice(&patch(&[
            "diff --git a/quiet.txt b/quiet.txt",
            "index 1111111..2222222 100644",
            "--- a/quiet.txt",
            "+++ b/quiet.txt",
            "@@ -1,2 +1,2 @@",
            " a",
            "-b",
            "+c",
        ]));
        raw.extend_from_slice(&patch(&[
            "diff --git a/deep/dir/second.txt b/deep/dir/second.txt",
            "index 3333333..4444444 100644",
            "--- a/deep/dir/second.txt",
            "+++ b/deep/dir/second.txt",
            "@@ -1,3 +1,3 @@",
            "-x",
            "-y",
            "-z",
            "+x\r",
            "+y\r",
            "+z\r",
        ]));
        let got = read(&raw);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].path, "f.txt");
        assert_eq!(got[1].path, "deep/dir/second.txt");
        assert_eq!(
            got[1].reading,
            Reading::Flipped {
                from: Eol::Lf,
                to: Eol::Crlf
            }
        );
    }

    #[test]
    fn only_the_two_exact_cases_are_offered_to_history() {
        assert!(
            Reading::Flipped {
                from: Eol::Lf,
                to: Eol::Crlf
            }
            .is_exact()
        );
        assert!(
            Reading::Mixed {
                lines: 1,
                added: Eol::Crlf,
                file: Eol::Lf
            }
            .is_exact()
        );
        assert!(!Reading::NewFile { eol: Eol::Crlf }.is_exact());
        assert!(!Reading::FirstEnding { eol: Eol::Crlf }.is_exact());
        assert!(!Reading::Quiet.is_exact());
    }

    #[test]
    fn empty_and_broken_input_reads_as_nothing() {
        assert!(read(b"").is_empty());
        assert!(read(b"not a patch at all\n").is_empty());
        assert_eq!(read_one(b""), Reading::Quiet);
    }
}
