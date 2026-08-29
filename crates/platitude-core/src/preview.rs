//! Best-effort content previews for the diff pane.
//!
//! A text diff says nothing useful about a binary file, and an image is
//! better shown than described. This module fetches the actual old/new
//! content of a diff target — blobs through `git cat-file`, the working
//! tree through the filesystem — so the UI can render images and report
//! binary sizes. Everything is best-effort: a side that cannot be read is
//! simply absent, which is also what "added" and "deleted" look like.

use std::path::{Path, PathBuf};

use tokio_util::sync::CancellationToken;

use crate::details::DiffTarget;
use crate::process::{GitCommand, GitExecutor};

/// Images larger than this are reported by size only; the bytes never
/// leave git. Bounds what one preview can pin in UI memory.
pub const IMAGE_BYTE_CAP: u64 = 16 * 1024 * 1024;

/// Extensions the UI renders with a QML `Image`, with the MIME type its
/// data URL carries. Extension-based on purpose: content sniffing would
/// need both sides fetched before deciding whether to fetch them.
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
/// listing a type here never breaks anything.
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
    /// The content itself — only for images within [`IMAGE_BYTE_CAP`].
    pub bytes: Option<Vec<u8>>,
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
    /// `<rev>:<path>` — readable through `git cat-file`.
    Blob(String),
    /// A file in the working tree.
    WorkTree(PathBuf),
    /// The side does not exist (e.g. the old side of an untracked file).
    Absent,
}

/// Loads the preview for `target`; `None` when a text diff already tells
/// the whole story (non-image, non-binary files).
pub async fn file_preview(
    executor: &GitExecutor,
    workdir: &Path,
    target: &DiffTarget,
    is_binary: bool,
    cancel: &CancellationToken,
) -> Option<FilePreview> {
    let mime = image_mime(target_path(target));
    if mime.is_none() && !is_binary {
        return None;
    }
    let want_bytes = mime.is_some();
    let (old_src, new_src) = side_sources(workdir, target);
    Some(FilePreview {
        image_mime: mime,
        old: load_side(executor, workdir, &old_src, want_bytes, cancel).await,
        new: load_side(executor, workdir, &new_src, want_bytes, cancel).await,
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
/// The blob side is one read, not two: a `cat-file -s` probe would spawn
/// a whole process to save the rare oversized read. The working-tree side
/// asks the metadata first — that one is free.
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
        SideSource::Blob(spec) => {
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
        | DiffTarget::Staged { path, .. }
        | DiffTarget::Unstaged { path }
        | DiffTarget::Untracked { path } => path,
    }
}

/// What each side of `target` diffs, mirroring the commands
/// [`crate::details::file_diff_raw`] runs for it.
fn side_sources(workdir: &Path, target: &DiffTarget) -> (SideSource, SideSource) {
    match target {
        DiffTarget::Commit {
            oid,
            parent,
            path,
            orig_path,
        } => {
            let old_path = orig_path.as_deref().unwrap_or(path);
            let old = match parent {
                Some(p) => SideSource::Blob(format!("{}:{old_path}", p.to_hex())),
                None => SideSource::Absent,
            };
            (old, SideSource::Blob(format!("{}:{path}", oid.to_hex())))
        }
        DiffTarget::Staged { path, orig_path } => {
            let old_path = orig_path.as_deref().unwrap_or(path);
            (
                SideSource::Blob(format!("HEAD:{old_path}")),
                SideSource::Blob(format!(":0:{path}")),
            )
        }
        DiffTarget::Unstaged { path } => (
            SideSource::Blob(format!(":0:{path}")),
            SideSource::WorkTree(workdir.join(path)),
        ),
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
/// command may answer with, and must stay so: a `cat-file` exiting 128
/// because the repository is gone has failed. `rev-parse --verify -q`
/// asks the same question and says no with exit 1, which the command log
/// keeps as an answer instead of raising itself over
/// (規約 core.md §終了コードで答える問い合わせはコマンドログの失敗にしない).
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
    want_bytes: bool,
    cancel: &CancellationToken,
) -> Option<PreviewSide> {
    match source {
        SideSource::Blob(spec) => blob_side(executor, workdir, spec, want_bytes, cancel).await,
        SideSource::WorkTree(path) => worktree_side(path, want_bytes).await,
        SideSource::Absent => None,
    }
}

/// Reads one side out of the object database, once [`blob_is_there`] has
/// said there is one to read. Having no side is data, not an error — it
/// is what added, deleted and unborn HEAD all look like from here.
async fn blob_side(
    executor: &GitExecutor,
    workdir: &Path,
    spec: &str,
    want_bytes: bool,
    cancel: &CancellationToken,
) -> Option<PreviewSide> {
    if !blob_is_there(executor, workdir, spec, cancel).await {
        return None;
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
    let mut bytes = None;
    if want_bytes && size <= IMAGE_BYTE_CAP {
        let blob_cmd = GitCommand::new()
            .cwd(workdir)
            .args(["cat-file", "blob"])
            .arg(spec);
        match executor.run_unchecked(blob_cmd, cancel).await {
            Ok(o) if o.code == 0 => bytes = Some(o.stdout),
            _ => tracing::debug!(spec, "preview blob read failed; size only"),
        }
    }
    Some(PreviewSide { size, bytes })
}

/// Reads one side straight from the working tree.
async fn worktree_side(path: &Path, want_bytes: bool) -> Option<PreviewSide> {
    let meta = tokio::fs::metadata(path).await.ok()?;
    if !meta.is_file() {
        return None;
    }
    let size = meta.len();
    let mut bytes = None;
    if want_bytes && size <= IMAGE_BYTE_CAP {
        match tokio::fs::read(path).await {
            Ok(b) => bytes = Some(b),
            Err(e) => tracing::debug!(path = %path.display(), error = %e,
                "preview file read failed; size only"),
        }
    }
    Some(PreviewSide { size, bytes })
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
}
