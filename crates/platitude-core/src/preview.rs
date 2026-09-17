//! Best-effort content previews for the diff pane.
//!
//! A text diff says nothing useful about a binary file, and an image is
//! better shown than described. This module finds the actual old/new
//! content of a diff target so the UI can render images and report
//! binary sizes — as **files**: the working tree's side is the file
//! already there, and a blob's side is written out of `git cat-file`
//! into a file of this run's own ([`PreviewFiles`]) that a `file:` URL
//! can name. Everything is best-effort: a side that
//! cannot be read is simply absent, which is also what "added" and
//! "deleted" look like.

use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio_util::sync::CancellationToken;

use crate::details::DiffTarget;
use crate::process::{GitCommand, GitExecutor};

/// Extensions the UI renders with a QML `Image`, with the MIME type the
/// preview reports for them. Extension-based on purpose: content sniffing
/// would need both sides fetched before deciding whether to fetch them.
const IMAGE_TYPES: [(&str, &str); 11] = [
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("bmp", "image/bmp"),
    ("webp", "image/webp"),
    ("svg", "image/svg+xml"),
    ("ico", "image/x-icon"),
    ("tif", "image/tiff"),
    ("tiff", "image/tiff"),
    ("avif", "image/avif"),
];

/// MIME type for paths the preview treats as images. Formats the runtime
/// lacks a decoder for degrade in the UI (`Image.status === Error`), so
/// listing a type here is safe.
pub fn image_mime(path: &str) -> Option<&'static str> {
    let ext = Path::new(path).extension()?.to_str()?;
    IMAGE_TYPES
        .iter()
        .find(|(e, _)| ext.eq_ignore_ascii_case(e))
        .map(|(_, mime)| *mime)
}

/// One side of a preview (old = before the change, new = after).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewSide {
    /// Content size in bytes.
    pub size: u64,
    /// Where the content can be read from a file — the working-tree file
    /// itself, or the file this run wrote the blob to. Only for images,
    /// and only while the read that made it stands: the next read of the
    /// same pane takes it away again ([`PreviewFiles::sweep_before`]).
    pub file: Option<PathBuf>,
    /// Why a wanted `file` is not there. A picture whose blob could not
    /// be written out still reports its size, and this is the only place
    /// what stopped it survives — nothing downstream can tell that side
    /// from one no file was ever asked for. `None` for every side that
    /// has its file, and for every side that was never to have one.
    pub unwritten: Option<String>,
}

/// Old/new content of one diff target, loaded when the text diff is not
/// the whole story (binary files, images).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePreview {
    /// Set when the path names an image the UI should render.
    pub image_mime: Option<&'static str>,
    /// `None` = the file does not exist on that side (added/deleted) or
    /// could not be read.
    pub old: Option<PreviewSide>,
    pub new: Option<PreviewSide>,
}

/// Where one side's content lives.
enum SideSource {
    /// `<rev>:<path>` — readable through `git cat-file`. `path` is the
    /// repository path the blob is filed under, whose extension names the
    /// file it is written to.
    Blob { spec: String, path: String },
    /// A file in the working tree.
    WorkTree(PathBuf),
    /// The side does not exist (e.g. the old side of an untracked file).
    Absent,
}

// ---------------------------------------------------------------------------
// The files a blob side is written to
// ---------------------------------------------------------------------------

/// Where this run keeps every preview file: one directory of its own under
/// the system temp, named after the process, and one directory per session
/// inside it.
pub fn run_dir() -> PathBuf {
    std::env::temp_dir().join(format!("platitude-gg-{}", std::process::id()))
}

/// Removes everything the run wrote, directory included — for the moment
/// after the last session is gone. Best-effort: what a reader still holds
/// open stays until it lets go, and a directory that was never made is
/// nothing to remove.
pub fn remove_run_dir() {
    let dir = run_dir();
    if let Err(error) = std::fs::remove_dir_all(&dir)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::debug!(path = %dir.display(), %error, "the run's preview files were not all removed");
    }
}

/// The preview files one session writes, and the sweeps that take them
/// away again.
///
/// A blob's bytes have no path a `file:` URL could name, so they are
/// written to one: `<epoch>-<side>.<ext>` under this session's directory,
/// the epoch being the diff read's own ([`crate::session::RepoSession`]
/// numbers them), so a file re-read is a new file and the URL that names
/// it is a new URL — an `Image` reloads on a source that changed and on
/// nothing else. What takes them away is the pane moving on: the read
/// handed to the pane sweeps the reads before it
/// ([`Self::sweep_before`]), the pane closing sweeps them all
/// ([`Self::release`]), and the session closing removes the directory
/// ([`Self::remove_all`]). Only a read that has finished writing is ever
/// swept — one still being written is left for the next sweep, whatever
/// its number, so a slow read that lands after a fast one can still hand
/// the pane files that are there.
pub struct PreviewFiles {
    dir: PathBuf,
    /// The reads whose files are on disk and no longer being written.
    settled: Mutex<BTreeSet<u64>>,
    /// Whether `dir` sits in the run's own directory, which the last of
    /// these out removes — a directory a caller chose has a parent that
    /// is nobody's to remove.
    in_run_dir: bool,
}

/// How many sessions the run's directory belongs to.
///
/// **An empty directory is not an abandoned one.** A session that has
/// not previewed a picture yet has left nothing in it, so emptiness says
/// nothing about who still needs it — and taking it then costs the next
/// write its file: `create_dir_all` makes the parent and the child in
/// two steps, and a removal landing between them fails the write with
/// `NotFound`, which lands the side by size alone. Counting is what says
/// the directory is still somebody's; the lock is what keeps a session
/// being made from racing the exit that read the count as zero.
static SESSIONS: Mutex<usize> = Mutex::new(0);

impl PreviewFiles {
    /// A directory of this run's own for one session. Made on first
    /// write, so a session that never previews a picture never touches
    /// the disk.
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        *crate::session::relock(&SESSIONS) += 1;
        Self {
            dir: run_dir().join(format!("s{serial}")),
            settled: Mutex::new(BTreeSet::new()),
            in_run_dir: true,
        }
    }

    /// The same, at a directory the caller chose.
    pub fn at(dir: PathBuf) -> Self {
        Self {
            dir,
            settled: Mutex::new(BTreeSet::new()),
            in_run_dir: false,
        }
    }

    /// Where the files of the read numbered `epoch` go.
    pub fn read(&self, epoch: u64) -> PreviewRead<'_> {
        PreviewRead { files: self, epoch }
    }

    /// Removes the files of every settled read numbered below `epoch` —
    /// what the read that has just been handed to the pane makes of the
    /// ones before it. Anything the pane still shows of those it has
    /// already decoded.
    pub fn sweep_before(&self, epoch: u64) {
        self.sweep(|read| read < epoch);
    }

    /// Removes the files of every settled read — the pane closed.
    pub fn release(&self) {
        self.sweep(|_| true);
    }

    /// Removes everything, directory included — the session closing. A
    /// read still writing recreates nothing: its file stays until the
    /// session is dropped, which removes the directory once more.
    pub fn remove_all(&self) {
        crate::session::relock(&self.settled).clear();
        if let Err(error) = std::fs::remove_dir_all(&self.dir)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::debug!(path = %self.dir.display(), %error, "preview files not all removed");
        }
    }

    /// A file that would not go — on Windows, one the pane's decoder still
    /// has open — keeps its read settled, so the next sweep asks again.
    fn sweep(&self, wanted: impl Fn(u64) -> bool) {
        let mut settled = crate::session::relock(&self.settled);
        let gone: Vec<u64> = settled.iter().copied().filter(|e| wanted(*e)).collect();
        if gone.is_empty() {
            return;
        }
        let mut held = BTreeSet::new();
        if let Ok(entries) = std::fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let Some(read) = epoch_of(&name.to_string_lossy()) else {
                    continue;
                };
                if !gone.contains(&read) {
                    continue;
                }
                if let Err(error) = std::fs::remove_file(entry.path()) {
                    tracing::debug!(path = %entry.path().display(), %error, "preview file not removed");
                    held.insert(read);
                }
            }
        }
        settled.retain(|e| !gone.contains(e) || held.contains(e));
    }
}

impl Default for PreviewFiles {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PreviewFiles {
    fn drop(&mut self) {
        self.remove_all();
        if !self.in_run_dir {
            return;
        }
        // The last session out turns the light off, and only that one:
        // while the count is held down here, no session can be made that
        // would want the directory back (see [`SESSIONS`]). What a read
        // cut short left in it keeps it, the way it always did.
        let mut sessions = crate::session::relock(&SESSIONS);
        *sessions = sessions.saturating_sub(1);
        if *sessions > 0 {
            return;
        }
        if let Err(error) = std::fs::remove_dir(run_dir())
            && !matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
            )
        {
            tracing::debug!(%error, "the run's preview directory was not removed");
        }
    }
}

/// The number in front of `<epoch>-<side>.<ext>`.
fn epoch_of(name: &str) -> Option<u64> {
    name.split('-').next()?.parse().ok()
}

/// One diff read's claim on the files: where its sides go, and the word
/// that it has finished writing them.
pub struct PreviewRead<'a> {
    files: &'a PreviewFiles,
    epoch: u64,
}

impl PreviewRead<'_> {
    /// The file a side of this read is written to: named by the read and
    /// the side, with the extension of the path the blob is filed under
    /// (`old.jpg` → `<epoch>-old.jpg`), which is what a decoder is picked
    /// by when the bytes do not say.
    fn path_for(&self, side: &str, repo_path: &str) -> PathBuf {
        let name = match Path::new(repo_path).extension().and_then(|e| e.to_str()) {
            Some(ext) => format!("{}-{side}.{ext}", self.epoch),
            None => format!("{}-{side}", self.epoch),
        };
        self.files.dir.join(name)
    }

    /// Both sides are written, or never will be: from here on a sweep may
    /// take this read's files.
    fn settle(&self) {
        crate::session::relock(&self.files.settled).insert(self.epoch);
    }
}

// ---------------------------------------------------------------------------
// The preview itself
// ---------------------------------------------------------------------------

/// Loads the preview for `target`; `None` when a text diff already tells
/// the whole story (non-image, non-binary files). An image's blob sides
/// are written under `read`, which is settled on the way out whatever
/// was found.
pub async fn file_preview(
    executor: &GitExecutor,
    workdir: &Path,
    target: &DiffTarget,
    is_binary: bool,
    read: PreviewRead<'_>,
    cancel: &CancellationToken,
) -> Option<FilePreview> {
    let mime = image_mime(target_path(target));
    if mime.is_none() && !is_binary {
        read.settle();
        return None;
    }
    let want_file = mime.is_some();
    let (old_src, new_src) = side_sources(workdir, target);
    let old = load_side(executor, workdir, &old_src, want_file, &read, "old", cancel).await;
    let new = load_side(executor, workdir, &new_src, want_file, &read, "new", cancel).await;
    read.settle();
    Some(FilePreview {
        image_mime: mime,
        old,
        new,
    })
}

/// A file bigger than this is not walked for syntax context: the reading
/// is linear in what comes before the hunk, and past this the wait costs
/// more than a first line in the wrong colour.
pub const SOURCE_BYTE_CAP: u64 = 4 * 1024 * 1024;

/// The whole of the side a diff's colours are read against — the new one
/// where the target has it, the old one for a file that was deleted.
///
/// What it is for is context. A hunk starts wherever it starts, and a
/// lexer only knows what a line means if it walked the file to get there
/// ([`crate::highlight`]). `None` where the side cannot be read, is not
/// UTF-8, or is over [`SOURCE_BYTE_CAP`] — the colours then start each
/// hunk clean.
///
/// The blob side is one read: a `cat-file -s` probe would spawn a
/// whole process to save the rare oversized read. The working-tree
/// side asks the metadata first — that one is free.
pub async fn source_text(
    executor: &GitExecutor,
    workdir: &Path,
    target: &DiffTarget,
    cancel: &CancellationToken,
) -> Option<String> {
    let (old, new) = side_sources(workdir, target);
    let bytes = match read_source(executor, workdir, &new, cancel).await {
        Some(bytes) => bytes,
        // No new side: the file was deleted, and every row of its diff
        // comes from the old one.
        None => read_source(executor, workdir, &old, cancel).await?,
    };
    if bytes.len() as u64 > SOURCE_BYTE_CAP {
        tracing::debug!(bytes = bytes.len(), "file too big to read colours against");
        return None;
    }
    String::from_utf8(bytes).ok()
}

async fn read_source(
    executor: &GitExecutor,
    workdir: &Path,
    source: &SideSource,
    cancel: &CancellationToken,
) -> Option<Vec<u8>> {
    match source {
        SideSource::Blob { spec, .. } => {
            if !blob_is_there(executor, workdir, spec, cancel).await {
                return None;
            }
            let cmd = GitCommand::new()
                .cwd(workdir)
                .args(["cat-file", "blob"])
                .arg(spec);
            let out = executor.run_unchecked(cmd, cancel).await.ok()?;
            (out.code == 0).then_some(out.stdout)
        }
        SideSource::WorkTree(path) => {
            if tokio::fs::metadata(path).await.ok()?.len() > SOURCE_BYTE_CAP {
                return None;
            }
            tokio::fs::read(path).await.ok()
        }
        SideSource::Absent => None,
    }
}

/// The path whose extension decides whether this target is an image, and
/// whether anything can be said about its colours.
pub(crate) fn target_path(target: &DiffTarget) -> &str {
    match target {
        DiffTarget::Commit { path, .. }
        | DiffTarget::Range { path, .. }
        | DiffTarget::Choice { path, .. }
        | DiffTarget::Staged { path, .. }
        | DiffTarget::Unstaged { path }
        | DiffTarget::Untracked { path } => path,
    }
}

/// What each side of `target` diffs, mirroring the commands
/// [`crate::details::file_diff_raw`] runs for it.
fn side_sources(workdir: &Path, target: &DiffTarget) -> (SideSource, SideSource) {
    let blob = |rev: &str, path: &str| SideSource::Blob {
        spec: format!("{rev}:{path}"),
        path: path.to_string(),
    };
    match target {
        DiffTarget::Commit {
            oid,
            parent,
            path,
            orig_path,
        } => {
            let old_path = orig_path.as_deref().unwrap_or(path);
            let old = match parent {
                Some(p) => blob(&p.to_hex(), old_path),
                None => SideSource::Absent,
            };
            (old, blob(&oid.to_hex(), path))
        }
        // Both sides are commits, so both are blobs — the older one under
        // whatever name the rename detection gave it there.
        DiffTarget::Range {
            from,
            to,
            path,
            orig_path,
        } => {
            let old_path = orig_path.as_deref().unwrap_or(path);
            (blob(&from.to_hex(), old_path), blob(&to.to_hex(), path))
        }
        // **A stack of patches has no pair of sides.** The pane shows what
        // each chosen commit did to the file in turn, so there is no one
        // "before" and no one "after" to put a picture of side by side.
        DiffTarget::Choice { .. } => (SideSource::Absent, SideSource::Absent),
        DiffTarget::Staged { path, orig_path } => {
            let old_path = orig_path.as_deref().unwrap_or(path);
            (blob("HEAD", old_path), blob(":0", path))
        }
        DiffTarget::Unstaged { path } => {
            (blob(":0", path), SideSource::WorkTree(workdir.join(path)))
        }
        DiffTarget::Untracked { path } => {
            (SideSource::Absent, SideSource::WorkTree(workdir.join(path)))
        }
    }
}

/// Whether the object database holds this side at all.
///
/// Asking is not optional. The specs [`side_sources`] builds name a side
/// that is routinely not there — the parent side of a file the commit
/// added, `HEAD:` before there is a HEAD, `:0:` for a staged deletion —
/// and `cat-file` answers by failing (`fatal: Not a valid object name`,
/// exit 128). 128 is outside the 0/1 a [`GitCommand::answers_by_code`]
/// command may answer with, and stays there: a `cat-file` exiting 128
/// because the repository is gone has failed. `rev-parse --verify -q`
/// asks the same question and says no with exit 1, which the command
/// log keeps as an answer
/// (規約 core.md §終了コードで答える問い合わせはコマンドログでも答え).
///
/// A read that never ran and a spec that resolved to nothing are the same
/// answer here: both mean this side has no content to show.
async fn blob_is_there(
    executor: &GitExecutor,
    workdir: &Path,
    spec: &str,
    cancel: &CancellationToken,
) -> bool {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .answers_by_code(1)
        .args(["rev-parse", "--verify", "-q"])
        .arg(spec);
    matches!(executor.run_unchecked(cmd, cancel).await, Ok(out) if out.code == 0)
}

async fn load_side(
    executor: &GitExecutor,
    workdir: &Path,
    source: &SideSource,
    want_file: bool,
    read: &PreviewRead<'_>,
    side: &str,
    cancel: &CancellationToken,
) -> Option<PreviewSide> {
    match source {
        SideSource::Blob { spec, path } => {
            let into = want_file.then(|| read.path_for(side, path));
            blob_side(executor, workdir, spec, into, cancel).await
        }
        SideSource::WorkTree(path) => worktree_side(path, want_file).await,
        SideSource::Absent => None,
    }
}

/// Reads one side out of the object database, once [`blob_is_there`] has
/// said there is one to read. Having no side is data — it is what
/// added, deleted and unborn HEAD all look like from here.
///
/// An image side is written to `into` as it streams out of `cat-file`,
/// and its size is what arrived; the bytes stream through. A side
/// that could not be written is reported by size, the way a non-image
/// binary is, and carries what stopped it
/// ([`PreviewSide::unwritten`]) — the size on its own reads exactly like
/// a side no picture was ever wanted from, and a reader looking at the
/// run afterwards would have nothing else to go on.
async fn blob_side(
    executor: &GitExecutor,
    workdir: &Path,
    spec: &str,
    into: Option<PathBuf>,
    cancel: &CancellationToken,
) -> Option<PreviewSide> {
    if !blob_is_there(executor, workdir, spec, cancel).await {
        return None;
    }
    let mut unwritten = None;
    if let Some(path) = into {
        match write_blob(executor, workdir, spec, &path, cancel).await {
            Ok(size) => {
                return Some(PreviewSide {
                    size,
                    file: Some(path),
                    unwritten: None,
                });
            }
            Err(error) => {
                tracing::debug!(spec, %error, "preview blob not written; size only");
                unwritten = Some(error.to_string());
            }
        }
    }
    let size_cmd = GitCommand::new()
        .cwd(workdir)
        .args(["cat-file", "-s"])
        .arg(spec);
    let out = executor.run_unchecked(size_cmd, cancel).await.ok()?;
    if out.code != 0 {
        return None;
    }
    let size: u64 = out.stdout_utf8().trim().parse().ok()?;
    Some(PreviewSide {
        size,
        file: None,
        unwritten,
    })
}

/// Streams `git cat-file blob <spec>` into `path`, answering how many
/// bytes arrived. A file left half-written by a failure is removed.
async fn write_blob(
    executor: &GitExecutor,
    workdir: &Path,
    spec: &str,
    path: &Path,
    cancel: &CancellationToken,
) -> Result<u64, WriteBlobError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|error| WriteBlobError::at(dir, "could not be made", error))?;
    }
    let mut file = std::fs::File::create(path)
        .map_err(|error| WriteBlobError::at(path, "could not be opened", error))?;
    let mut written = 0u64;
    let mut failed: Option<std::io::Error> = None;
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["cat-file", "blob"])
        .arg(spec);
    let ran = executor
        .run_streaming(cmd, cancel, &mut |chunk| {
            if failed.is_some() {
                return;
            }
            match file.write_all(chunk) {
                Ok(()) => written += chunk.len() as u64,
                Err(error) => failed = Some(error),
            }
        })
        .await;
    drop(file);
    let outcome = match (ran, failed) {
        (Err(error), _) => Err(WriteBlobError::Git(error)),
        (Ok(_), Some(error)) => Err(WriteBlobError::at(path, "could not be written to", error)),
        (Ok(_), None) => Ok(written),
    };
    if outcome.is_err()
        && let Err(error) = std::fs::remove_file(path)
    {
        tracing::debug!(path = %path.display(), %error, "half-written preview file not removed");
    }
    outcome
}

#[derive(Debug, thiserror::Error)]
enum WriteBlobError {
    /// `cat-file` itself fell over.
    #[error("{0}")]
    Git(#[from] crate::error::GitError),
    /// A step of the write did. **Which path and which step is the whole
    /// diagnosis**: a bare `NotFound` says nothing about whether the
    /// session's directory, the file in it or the stream into it is what
    /// went, and the side that comes back carries only this sentence.
    #[error("{} {step}: {error}", path.display())]
    Io {
        path: PathBuf,
        step: &'static str,
        error: std::io::Error,
    },
}

impl WriteBlobError {
    fn at(path: &Path, step: &'static str, error: std::io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            step,
            error,
        }
    }
}

/// Reads one side straight from the working tree: the file itself is the
/// preview, so nothing is copied.
async fn worktree_side(path: &Path, want_file: bool) -> Option<PreviewSide> {
    let meta = tokio::fs::metadata(path).await.ok()?;
    if !meta.is_file() {
        return None;
    }
    Some(PreviewSide {
        size: meta.len(),
        file: want_file.then(|| path.to_path_buf()),
        unwritten: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_mime_matches_extensions_case_insensitively() {
        assert_eq!(image_mime("art/logo.PNG"), Some("image/png"));
        assert_eq!(image_mime("photo.jpeg"), Some("image/jpeg"));
        assert_eq!(image_mime("icon.svg"), Some("image/svg+xml"));
        assert_eq!(image_mime("readme.md"), None);
        assert_eq!(image_mime("no-extension"), None);
        assert_eq!(image_mime("tricky.png.txt"), None);
    }

    /// A directory of this test's own, so what it sweeps is only what it
    /// wrote. The temp dir outlives the files: dropping them removes
    /// their own directory and nothing above it.
    fn files() -> (tempfile::TempDir, PreviewFiles) {
        let dir = tempfile::tempdir().expect("a temp dir");
        let files = PreviewFiles::at(dir.path().join("s"));
        (dir, files)
    }

    fn write(files: &PreviewFiles, epoch: u64, side: &str) -> PathBuf {
        let read = files.read(epoch);
        let path = read.path_for(side, "art/logo.png");
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("dir");
        std::fs::write(&path, b"png").expect("write");
        path
    }

    #[test]
    fn a_side_is_named_by_its_read_and_keeps_the_extension() {
        let (_dir, files) = files();
        let read = files.read(7);
        assert!(read.path_for("old", "a/b.JPG").ends_with("7-old.JPG"));
        assert!(read.path_for("new", "bare").ends_with("7-new"));
    }

    #[test]
    fn a_sweep_takes_the_settled_reads_before_the_epoch_and_leaves_the_rest() {
        let (_dir, files) = files();
        let first = write(&files, 1, "old");
        let second = write(&files, 2, "new");
        let unsettled = write(&files, 3, "new");
        files.read(1).settle();
        files.read(2).settle();
        files.sweep_before(3);
        assert!(!first.exists(), "a settled read before the epoch goes");
        assert!(!second.exists());
        assert!(unsettled.exists(), "a read still writing is left alone");
        // The unsettled one lands and is swept by the next read after it.
        files.read(3).settle();
        files.sweep_before(4);
        assert!(!unsettled.exists());
    }

    #[test]
    fn a_release_takes_every_settled_read_whatever_its_number() {
        let (_dir, files) = files();
        let newest = write(&files, 9, "new");
        let writing = write(&files, 10, "new");
        files.read(9).settle();
        files.release();
        assert!(!newest.exists());
        assert!(writing.exists(), "not settled, so not this sweep's to take");
    }

    #[test]
    fn dropping_the_files_removes_their_directory() {
        let (temp, files) = files();
        let path = write(&files, 1, "new");
        let dir = files.dir.clone();
        drop(files);
        assert!(!path.exists());
        assert!(!dir.exists());
        assert!(
            temp.path().exists(),
            "what is above the files is left alone"
        );
    }

    /// The run's directory is shared, so a session on its way out leaves
    /// it to the sessions still holding it — including the ones that have
    /// written nothing yet and so left it looking abandoned. Taking it
    /// from them costs their next write its file ([`SESSIONS`]).
    ///
    /// Only this half is assertable from a test binary running its tests
    /// side by side: whether the *last* session out removes the directory
    /// depends on the sessions the other tests are holding.
    #[test]
    fn the_run_directory_stays_while_another_session_holds_it() {
        let live = PreviewFiles::new();
        std::fs::create_dir_all(run_dir()).expect("the run's directory");
        drop(PreviewFiles::new());
        assert!(
            run_dir().exists(),
            "a session's exit does not take the directory another one is about to write to"
        );
        drop(live);
    }

    #[test]
    fn a_read_of_the_files_of_nothing_sweeps_nothing_and_fails_nothing() {
        let (_dir, files) = files();
        files.read(1).settle();
        files.sweep_before(2);
        files.release();
        files.remove_all();
    }
}
