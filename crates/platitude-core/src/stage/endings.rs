//! How the lines of a partly discarded file end.
//!
//! Where git converts the file, the diff is in index space, and `apply`
//! reads the file back through the line-ending conversion and writes every
//! line out again, so the settings decide how each one ends — lines the
//! discard is not about included (`core.autocrlf=true` turns an LF file
//! CRLF; `input` turns a CRLF file LF). A file that ends its lines one way
//! is written back that way wherever a setting can say so.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::eol::{self, Eol, Shape};
use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor, literal_pathspec};

/// The setting `apply` runs `patch` (a discard of `path`) under so the file
/// keeps its line endings, if one does. `core.autocrlf` is the one that says
/// how a line ends on the way out (`input` LF, `true` CRLF).
///
/// - An LF file reads the same under either (nothing to convert), and
///   `input` writes it as it was, converted by git or not.
/// - A CRLF file is pinned only where git compared it in index space — the
///   lines the patch keeps whole carry no CR. Where they carry it, git
///   compared the bytes as they are and `apply` reads and writes them so;
///   `true` there would write a result with no CR left in it (a hunk brought
///   back whole) CRLF throughout. A patch that keeps no line with an ending
///   says neither, and is left to git.
///
/// A file with both endings has no setting that keeps it where git converts
/// it, and a `.gitattributes` `eol` has the last word over any.
pub(super) async fn kept(
    executor: &GitExecutor,
    workdir: &Path,
    path: &str,
    patch: &[u8],
    cancel: &CancellationToken,
) -> Result<Option<&'static str>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["ls-files", "--eol", "-z", "--"])
        .arg(literal_pathspec(path));
    let out = executor.run(cmd, cancel).await?;
    let shape = eol::working_tree_shapes(&out.stdout)
        .into_iter()
        .next()
        .map_or(Shape::Nothing, |(_, shape)| shape);
    Ok(match shape {
        Shape::Uniform(Eol::Lf) => Some("core.autocrlf=input"),
        Shape::Uniform(Eol::Crlf) if in_index_space(patch) => Some("core.autocrlf=true"),
        Shape::Uniform(Eol::Crlf) | Shape::Mixed | Shape::Nothing => None,
    })
}

/// Whether the working-tree lines a reverse patch keeps (context and
/// additions) are there, and none of them ends in CR — what git prints for
/// a CRLF file it converted before comparing. A line followed by `\ No
/// newline at end of file` has no ending to tell by and is not counted.
fn in_index_space(patch: &[u8]) -> bool {
    let mut in_hunk = false;
    let mut kept = 0usize;
    let mut last_kept = false;
    for line in patch.split(|b| *b == b'\n') {
        if line.starts_with(b"@@ ") {
            in_hunk = true;
            last_kept = false;
            continue;
        }
        if in_hunk && line.starts_with(b"\\") && last_kept {
            kept -= 1;
        }
        last_kept = in_hunk && matches!(line.first(), Some(b' ' | b'+'));
        if !last_kept {
            continue;
        }
        if line.ends_with(b"\r") {
            return false;
        }
        kept += 1;
    }
    kept > 0
}

#[cfg(test)]
mod tests {
    use super::in_index_space;

    #[test]
    fn a_kept_line_with_its_cr_is_a_diff_of_bytes_as_they_are() {
        let raw = b"--- a/f\n+++ b/f\n@@ -1,2 +1,2 @@\n-a\n+a\r\n b\r\n";
        assert!(!in_index_space(raw));
    }

    #[test]
    fn kept_lines_without_cr_are_in_index_space() {
        let raw = b"--- a/f\n+++ b/f\n@@ -1,2 +1,2 @@\n-a\n+A\n b\n";
        assert!(in_index_space(raw));
    }

    #[test]
    fn an_unterminated_line_says_neither() {
        // `diff.context=0` at the end of a CRLF file git compared as bytes:
        // the one kept line ends the file and has no CR to show.
        let raw = b"--- a/f\n+++ b/f\n@@ -3 +3 @@\n-c\r\n+Z\n\\ No newline at end of file\n";
        assert!(!in_index_space(raw));
    }

    #[test]
    fn a_patch_that_keeps_no_line_says_neither() {
        // `diff.context=0`, a deletion at the top: the `+++` header line is
        // not a kept line.
        let raw = b"--- a/f\n+++ b/f\n@@ -1 +0,0 @@\n-a\n";
        assert!(!in_index_space(raw));
    }
}
