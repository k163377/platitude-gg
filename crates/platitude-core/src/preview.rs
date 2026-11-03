//! Best-effort content previews for the diff pane: the old/new content of
//! a diff target, so the UI can render images and report binary sizes.
//! A picture side is shown from the working tree's own file, or from a
//! blob named by its object ([`Picture`]): nothing is written for it. A
//! side that cannot be read is absent, which is also what "added" and
//! "deleted" look like.
//!
//! A blob's picture reaches the UI by its object, whose bytes the
//! application's image provider reads through the session when an `Image`
//! asks for them ([`PictureReader`]), never as base64 (user decision — ask
//! before changing): as a string, one 16MB image alone weighs about 43MB and a
//! picture past the limit cannot show at all; a blob read when it is drawn
//! has no limit. An opened picture costs its decoded pixels, and the pane
//! moving on or closing gives all of it back.
//!
//! Drawing a side holds of its compressed bytes only those on their way to
//! the decoder where Qt reads its format in order (PNG, JPEG, GIF, SVG), and
//! all of them, from the read to the end of its decode, for any other
//! (ci/baseline/preview-pictures-windows-x64.md).

use std::path::{Path, PathBuf};

use tokio_util::sync::CancellationToken;

use crate::Oid;
use crate::details::DiffTarget;
use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// A picture format the diff pane renders, as a path's extension names it
/// (in any case). By extension on purpose: sniffing would fetch both sides
/// before deciding whether to fetch them. A format the runtime cannot
/// decode degrades in the UI (`Image.status === Error`), so listing one is
/// safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    Bmp,
    Webp,
    Svg,
    Ico,
    Tiff,
    Avif,
}

impl ImageFormat {
    /// The format `path`'s extension names; `None` for every other path,
    /// which the pane shows no picture for.
    pub fn of_path(path: &str) -> Option<Self> {
        let ext = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
        Some(match ext.as_str() {
            "png" => Self::Png,
            "jpg" | "jpeg" => Self::Jpeg,
            "gif" => Self::Gif,
            "bmp" => Self::Bmp,
            "webp" => Self::Webp,
            "svg" => Self::Svg,
            "ico" => Self::Ico,
            "tif" | "tiff" => Self::Tiff,
            "avif" => Self::Avif,
            _ => return None,
        })
    }

    /// Whether the format is drawn from shapes, which scale smoothly,
    /// rather than from pixels.
    pub fn is_vector(self) -> bool {
        matches!(self, Self::Svg)
    }
}

/// A blob's picture: its object, its size, and the format the path it was
/// at names — what picks a decoder where the bytes do not say, as a file's
/// name would. No format where that path names none (a side renamed from
/// one that is no picture's): the bytes alone then say.
///
/// Its bytes are read when it is drawn ([`PictureReader`]) and not before,
/// nor kept after. By its object, not by `<rev>:<path>`: the index or a ref
/// may move before the picture is drawn, and the object is the content the
/// diff was read against — the same in every worktree of the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureBlob {
    oid: Oid,
    size: u64,
    format: Option<ImageFormat>,
}

impl PictureBlob {
    /// The object `oid`, `size` bytes, as it stood at `path`.
    pub fn new(oid: Oid, size: u64, path: &str) -> Self {
        Self {
            oid,
            size,
            format: ImageFormat::of_path(path),
        }
    }

    pub fn format(&self) -> Option<ImageFormat> {
        self.format
    }
}

/// What an `Image` shows one picture side from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Picture {
    /// The working tree's own file, shown where it stands.
    File(PathBuf),
    /// A blob, read when it is drawn.
    Blob(PictureBlob),
}

/// One side of a preview (old = before the change, new = after).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewSide {
    /// Content size in bytes.
    pub size: u64,
    /// What an image's side is shown from; `None` for a binary that is no
    /// image.
    pub picture: Option<Picture>,
}

/// Old/new content of one diff target, loaded when the text diff is not
/// the whole story (binary files, images).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePreview {
    /// The picture format the path names, where the UI renders one.
    pub format: Option<ImageFormat>,
    /// `None` = the file does not exist on that side (added/deleted) or
    /// could not be read.
    pub old: Option<PreviewSide>,
    pub new: Option<PreviewSide>,
}

/// Where one side's content lives.
enum SideSource {
    /// `spec` is `<rev>:<path>` for `git rev-parse`; `path` names a
    /// picture's format ([`PictureBlob`]).
    Blob {
        spec: String,
        path: String,
    },
    WorkingTree(PathBuf),
    /// The side does not exist (e.g. the old side of an untracked file).
    Absent,
}

/// Loads the preview for `target`; `None` when a text diff already tells
/// the whole story (non-image, non-binary files). An image's blob sides
/// are named by their objects, not read.
pub async fn file_preview(
    executor: &GitExecutor,
    workdir: &Path,
    target: &DiffTarget,
    is_binary: bool,
    cancel: &CancellationToken,
) -> Option<FilePreview> {
    let format = ImageFormat::of_path(target_path(target));
    if format.is_none() && !is_binary {
        return None;
    }
    let want_picture = format.is_some();
    let (old_src, new_src) = side_sources(workdir, target);
    let old = load_side(executor, workdir, &old_src, want_picture, cancel).await;
    let new = load_side(executor, workdir, &new_src, want_picture, cancel).await;
    Some(FilePreview { format, old, new })
}

/// A file bigger than this is not walked for syntax context: the reading
/// is linear in what comes before the hunk, and past this the wait costs
/// more than a first line in the wrong colour.
pub const SOURCE_BYTE_CAP: u64 = 4 * 1024 * 1024;

/// The whole of the side a diff's colours are read against (the new one,
/// or the old one for a deleted file): a lexer only knows what a hunk's
/// first line means if it walked the file to get there
/// ([`crate::highlight`]). `None` where the side cannot be read, is not
/// UTF-8, or is over [`SOURCE_BYTE_CAP`] — each hunk's colours then start
/// clean.
///
/// Only a side that is not there sends the read to the other one: a side
/// over the cap is the one the rows show, and the other side's text would
/// colour them against a file they are not. Pinned by
/// `preview_integration::a_blob_over_the_cap_is_not_read_to_its_end` and
/// `a_working_file_over_the_cap_reads_no_other_side`.
pub async fn source_text(
    executor: &GitExecutor,
    workdir: &Path,
    target: &DiffTarget,
    cancel: &CancellationToken,
) -> Option<String> {
    let (old, new) = side_sources(workdir, target);
    let read = match read_source(executor, workdir, &new, cancel).await {
        Source::Absent => read_source(executor, workdir, &old, cancel).await,
        read => read,
    };
    match read {
        Source::Text(bytes) => String::from_utf8(bytes).ok(),
        Source::TooBig => {
            tracing::debug!("file too big to read colours against");
            None
        }
        Source::Absent => None,
    }
}

/// What one side gave [`source_text`].
enum Source {
    /// Not there, or not readable: the other side may stand in.
    Absent,
    /// Over [`SOURCE_BYTE_CAP`], and not read to its end.
    TooBig,
    Text(Vec<u8>),
}

async fn read_source(
    executor: &GitExecutor,
    workdir: &Path,
    source: &SideSource,
    cancel: &CancellationToken,
) -> Source {
    match source {
        SideSource::Blob { spec, .. } => {
            if blob_oid(executor, workdir, spec, cancel).await.is_none() {
                return Source::Absent;
            }
            read_capped_blob(executor, workdir, spec, cancel).await
        }
        SideSource::WorkingTree(path) => match tokio::fs::metadata(path).await {
            Ok(meta) if meta.len() > SOURCE_BYTE_CAP => Source::TooBig,
            Ok(_) => match tokio::fs::read(path).await {
                Ok(bytes) => Source::Text(bytes),
                Err(_) => Source::Absent,
            },
            Err(_) => Source::Absent,
        },
        SideSource::Absent => Source::Absent,
    }
}

/// `cat-file blob`, stopped once it has passed [`SOURCE_BYTE_CAP`]: no
/// size is asked first (a `cat-file -s` is a process for every side, to
/// save the rare oversized one), and what is over the cap is never held.
/// The stop is this read's own token, a child of the caller's, so the
/// row ends `Cancelled` and the caller's token is left as it was.
async fn read_capped_blob(
    executor: &GitExecutor,
    workdir: &Path,
    spec: &str,
    cancel: &CancellationToken,
) -> Source {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["cat-file", "blob"])
        .arg(spec);
    let stop = cancel.child_token();
    let cap = usize::try_from(SOURCE_BYTE_CAP).unwrap_or(usize::MAX);
    let mut bytes = Vec::new();
    let mut over = false;
    let read = executor
        .run_streaming(cmd, &stop, &mut |chunk: &[u8]| {
            if over {
                return;
            }
            if bytes.len() + chunk.len() > cap {
                over = true;
                bytes = Vec::new();
                stop.cancel();
                return;
            }
            bytes.extend_from_slice(chunk);
        })
        .await;
    match read {
        _ if over => Source::TooBig,
        Ok(_) => Source::Text(bytes),
        Err(_) => Source::Absent,
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
        DiffTarget::Range {
            from,
            to,
            path,
            orig_path,
        } => {
            let old_path = orig_path.as_deref().unwrap_or(path);
            (blob(&from.to_hex(), old_path), blob(&to.to_hex(), path))
        }
        // A stack of patches has no one "before" and "after" to show side
        // by side.
        DiffTarget::Choice { .. } => (SideSource::Absent, SideSource::Absent),
        DiffTarget::Staged { path, orig_path } => {
            let old_path = orig_path.as_deref().unwrap_or(path);
            (blob("HEAD", old_path), blob(":0", path))
        }
        DiffTarget::Unstaged { path } => (
            blob(":0", path),
            SideSource::WorkingTree(workdir.join(path)),
        ),
        DiffTarget::Untracked { path } => (
            SideSource::Absent,
            SideSource::WorkingTree(workdir.join(path)),
        ),
    }
}

/// The object this side is, where the object database holds it at all.
///
/// The specs [`side_sources`] builds routinely name a side that is not
/// there (the parent side of an added file, `HEAD:` before there is a
/// HEAD, `:0:` for a staged deletion), and `cat-file` says so with exit
/// 128 — a failure, since a vanished repository exits 128 too.
/// `rev-parse --verify -q` says no with exit 1, an answer
/// (rules/core.md「終了コードで答える問い合わせは」; measured on 2.55 for
/// all three). Not `cat-file --batch-check`, which says "missing" with exit
/// 0 but reads its specs from stdin: the process layer hands input only to
/// writes, and closes a read's stdin at once (`process::child`).
///
/// A read that never ran and a spec that resolved to nothing are the same
/// answer here: no content to show.
async fn blob_oid(
    executor: &GitExecutor,
    workdir: &Path,
    spec: &str,
    cancel: &CancellationToken,
) -> Option<Oid> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .answers_by_code(1)
        .args(["rev-parse", "--verify", "-q"])
        .arg(spec);
    match executor.run_unchecked(cmd, cancel).await {
        Ok(out) if out.code == 0 => Oid::from_hex_str(out.stdout_utf8().trim()).ok(),
        _ => None,
    }
}

async fn load_side(
    executor: &GitExecutor,
    workdir: &Path,
    source: &SideSource,
    want_picture: bool,
    cancel: &CancellationToken,
) -> Option<PreviewSide> {
    match source {
        SideSource::Blob { spec, path } => {
            let picture_at = want_picture.then_some(path.as_str());
            blob_side(executor, workdir, spec, picture_at, cancel).await
        }
        SideSource::WorkingTree(path) => working_tree_side(path, want_picture).await,
        SideSource::Absent => None,
    }
}

/// Sizes one side in the object database and, for a picture side
/// (`picture_at`, the path it was at, given), names its object. No side is
/// data: added, deleted and unborn HEAD all look like that from here.
async fn blob_side(
    executor: &GitExecutor,
    workdir: &Path,
    spec: &str,
    picture_at: Option<&str>,
    cancel: &CancellationToken,
) -> Option<PreviewSide> {
    let oid = blob_oid(executor, workdir, spec, cancel).await?;
    let size_cmd = GitCommand::new()
        .cwd(workdir)
        .args(["cat-file", "-s"])
        .arg(oid.to_hex());
    let out = executor.run_unchecked(size_cmd, cancel).await.ok()?;
    if out.code != 0 {
        return None;
    }
    let size: u64 = out.stdout_utf8().trim().parse().ok()?;
    Some(PreviewSide {
        size,
        picture: picture_at.map(|path| Picture::Blob(PictureBlob::new(oid, size, path))),
    })
}

/// The bytes of `picture`, whole: `git cat-file blob` of its object in the
/// repository at `workdir`, into a buffer of its size.
pub async fn read_picture(
    executor: &GitExecutor,
    workdir: &Path,
    picture: &PictureBlob,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["cat-file", "blob"])
        .arg(picture.oid.to_hex());
    let mut bytes = Vec::with_capacity(usize::try_from(picture.size).unwrap_or(0));
    executor
        .run_streaming(cmd, cancel, &mut |chunk| bytes.extend_from_slice(chunk))
        .await?;
    Ok(bytes)
}

/// [`read_picture`], chunk by chunk to `to` as git gives them, the read
/// waiting while `to`'s receiver is behind (`GitExecutor::run_piped`): no
/// more of the picture stands in memory than the channel holds. So git
/// lives as long as the receiver takes — holding its process slot, and
/// within its time limit, past which the read ends failed.
pub async fn stream_picture(
    executor: &GitExecutor,
    workdir: &Path,
    picture: &PictureBlob,
    cancel: &CancellationToken,
    to: &tokio::sync::mpsc::Sender<Vec<u8>>,
) -> Result<(), GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["cat-file", "blob"])
        .arg(picture.oid.to_hex());
    executor.run_piped(cmd, cancel, to).await.map(drop)
}

/// What reads a pane's pictures when they are drawn: a session's git, its
/// worktree, the runtime it runs on and the token of the diff read the
/// pane shows, which the pane's next read and the session's close end
/// (`RepoSession::picture_reader`). The session's git, so a read under way
/// is one of the session's runs — counted where a stand-down waits for
/// them, and kept in the command log as its reads are.
#[derive(Clone)]
pub struct PictureReader {
    executor: GitExecutor,
    workdir: PathBuf,
    runtime: tokio::runtime::Handle,
    cancel: CancellationToken,
}

impl PictureReader {
    pub fn new(
        executor: GitExecutor,
        workdir: PathBuf,
        runtime: tokio::runtime::Handle,
        cancel: CancellationToken,
    ) -> Self {
        Self {
            executor,
            workdir,
            runtime,
            cancel,
        }
    }

    /// This reader, its reads stopped as well by the token answered with
    /// it: what one drawing reads with, so that each of its reads stops
    /// with the drawing.
    pub fn scoped(&self) -> (Self, CancellationToken) {
        let stop = self.cancel.child_token();
        let scoped = Self {
            cancel: stop.clone(),
            ..self.clone()
        };
        (scoped, stop)
    }

    /// Reads `picture`'s bytes on the runtime and hands them to `done`
    /// there. A read the runtime drops unrun — it has stopped — drops
    /// `done` unanswered.
    pub fn read(
        &self,
        picture: PictureBlob,
        done: impl FnOnce(Result<Vec<u8>, GitError>) + Send + 'static,
    ) {
        let reader = self.clone();
        self.runtime.spawn(async move {
            done(read_picture(&reader.executor, &reader.workdir, &picture, &reader.cancel).await);
        });
    }

    /// Streams `picture`'s bytes to `to` on the runtime ([`stream_picture`])
    /// and tells `done` how the read ended before `to` is let go: a receiver
    /// that sees the channel close already knows whether it saw the whole
    /// picture. A read the runtime drops unrun drops `done` unanswered, and
    /// `to` with it.
    pub fn stream(
        &self,
        picture: PictureBlob,
        to: tokio::sync::mpsc::Sender<Vec<u8>>,
        done: impl FnOnce(Result<(), GitError>) + Send + 'static,
    ) {
        let reader = self.clone();
        self.runtime.spawn(async move {
            let streamed = stream_picture(
                &reader.executor,
                &reader.workdir,
                &picture,
                &reader.cancel,
                &to,
            )
            .await;
            done(streamed);
            drop(to);
        });
    }
}

/// Reads one side straight from the working tree: the file itself is the
/// preview, so nothing is copied.
async fn working_tree_side(path: &Path, want_picture: bool) -> Option<PreviewSide> {
    let meta = tokio::fs::metadata(path).await.ok()?;
    if !meta.is_file() {
        return None;
    }
    Some(PreviewSide {
        size: meta.len(),
        picture: want_picture.then(|| Picture::File(path.to_path_buf())),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refusing;

    #[test]
    fn a_paths_extension_names_its_format_in_any_case() {
        assert_eq!(ImageFormat::of_path("art/logo.PNG"), Some(ImageFormat::Png));
        assert_eq!(ImageFormat::of_path("photo.jpeg"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::of_path("photo.jpg"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::of_path("scan.TIF"), Some(ImageFormat::Tiff));
        assert_eq!(ImageFormat::of_path("icon.svg"), Some(ImageFormat::Svg));
        assert_eq!(ImageFormat::of_path("readme.md"), None);
        assert_eq!(ImageFormat::of_path("no-extension"), None);
        assert_eq!(ImageFormat::of_path("tricky.png.txt"), None);
        assert_eq!(ImageFormat::of_path("art/.png"), None);
    }

    /// The executor cannot run anything and the repository is not there,
    /// so a read that slipped past is counted and finds nothing real.
    #[tokio::test]
    async fn plain_text_has_no_preview_and_asks_git_nothing() {
        let (exec, asked) = refusing::git();

        let preview = file_preview(
            &exec,
            &refusing::nowhere(),
            &DiffTarget::Unstaged {
                path: "notes.txt".to_string(),
            },
            false,
            &CancellationToken::new(),
        )
        .await;

        assert!(preview.is_none());
        assert_eq!(asked.count(), 0, "nothing was asked of git");
    }
}
