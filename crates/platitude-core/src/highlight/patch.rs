//! One diff at a time: which lines of the file to walk before a hunk
//! begins, and whether the file still says what the hunk says.

use syntect::highlighting::Highlighter;

use crate::parse::diff::{DiffLineKind, FilePatch};

use super::theme::{Assets, assets, syntax_for};
use super::walk::Walk;
use super::{DiffColors, PatchColors};

/// How far into a file the reading will walk to reach a hunk. The walk is
/// linear and was measured at ~28,000 lines a second (2026-08-13,
/// release), so this is about 180ms — as long as opening a diff can spend
/// on colour. A hunk past it reads cold, which is what every file the set
/// does not know reads like anyway.
const CONTEXT_LINE_CAP: usize = 5_000;

/// Reads every text line of `patches` and answers what colour each run of
/// it is. Best effort throughout: a language that is not in the set, a
/// binary patch, a regex that will not run — each of those is a diff that
/// comes out the colour it always was, never an error and never a gap.
///
/// This is CPU work with no waiting in it. Callers on an async runtime
/// should hand it to a blocking thread rather than hold a worker.
/// `source` is the whole of the side the diff's line numbers count in
/// ([`crate::preview::source_text`]); without it every hunk starts cold
/// (see the module note).
pub fn colors(patches: &[FilePatch], source: Option<&str>) -> DiffColors {
    let assets = assets();
    DiffColors {
        patches: patches
            .iter()
            .map(|patch| patch_colors(assets, patch, source))
            .collect(),
    }
}

fn patch_colors(assets: &Assets, patch: &FilePatch, source: Option<&str>) -> PatchColors {
    if patch.is_binary || patch.unmerged {
        return Vec::new();
    }
    let syntax = syntax_for(&assets.syntaxes, patch.path());
    // A language nothing was found for still has its fences read: the
    // markers are git's, not the file's, and a conflict in a plain text
    // file has exactly the same ones.
    if syntax.is_none() && !patch.is_combined {
        return Vec::new();
    }
    let highlighter = Highlighter::new(&assets.theme);
    // Which side the file we were handed is, and therefore which of the
    // hunk's two line numbers counts in it. A deleted file has no new
    // side and every row of its diff comes from the old one.
    let new_side = patch.new_path.is_some();
    let lines: Vec<&str> = source.map(|s| s.lines().collect()).unwrap_or_default();
    // The file itself, walked to wherever the next hunk starts. Kept
    // apart from the walk that colours the rows: a hunk's rows are the
    // two sides interleaved, and feeding those back would leave the
    // reading somewhere the file never goes.
    let mut file = Walk::new(assets, &highlighter, syntax, patch.is_combined);
    let mut at = 0usize;
    let mut out = Vec::with_capacity(patch.hunks.len());
    for hunk in &patch.hunks {
        let (start, count) = if new_side {
            (hunk.new_start, hunk.new_count)
        } else {
            (hunk.old_start, hunk.old_count)
        };
        let start = start.saturating_sub(1) as usize;
        let mut rows = None;
        if !lines.is_empty() && start <= lines.len() && start <= CONTEXT_LINE_CAP {
            while at < start {
                file.paint(lines[at]);
                at += 1;
            }
            if hunk_agrees(hunk, &lines, start, new_side) {
                rows = Some(file.fork());
            }
        }
        // Nothing to start from — no file, or one that does not say what
        // this hunk says it does.
        let mut rows =
            rows.unwrap_or_else(|| Walk::new(assets, &highlighter, syntax, patch.is_combined));
        out.push(hunk.lines.iter().map(|line| rows.read(line)).collect());
        // Step the file over the lines this hunk covers, so the next one
        // starts from where the file really is.
        for _ in 0..count {
            let Some(line) = lines.get(at) else {
                break;
            };
            file.paint(line);
            at += 1;
        }
    }
    out
}

/// Whether the file we were handed says at `start` what the hunk says is
/// there. A diff and a file read a moment apart can disagree — someone
/// saved between them — and a reading walked through the wrong text is
/// worse than one that admits it does not know.
fn hunk_agrees(
    hunk: &crate::parse::diff::DiffHunk,
    lines: &[&str],
    start: usize,
    new_side: bool,
) -> bool {
    let first = hunk.lines.iter().find(|line| match line.kind {
        DiffLineKind::Context => true,
        DiffLineKind::Addition => new_side,
        DiffLineKind::Deletion => !new_side,
        DiffLineKind::NoNewline => false,
    });
    first.is_none_or(|line| lines.get(start) == Some(&line.text.as_str()))
}

#[cfg(test)]
mod tests {
    use super::super::testkit::patches;
    use super::*;
    /// One changed line of Rust, in the ordinary unified form.
    const RUST: &str = "\
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,3 @@
 fn main() {
-    let x = 1;
+    let x = 2;
 }
";

    #[test]
    fn rust_lines_are_taken_apart() {
        let colors = colors(&patches(RUST), None);
        // `-    let x = 1;` is the second line of the only hunk.
        let deleted = colors.line(0, 0, 1);
        assert!(
            deleted.spans.len() > 1,
            "a line with a keyword and a number in it is not one colour: {deleted:?}"
        );
        let total: usize = deleted.spans.iter().map(|s| s.len).sum();
        assert_eq!(total, "    let x = 1;".len(), "runs cover the whole line");
    }

    /// A hunk in the middle of a block comment: read from the top of the
    /// file it is a comment, read cold it is code. The one line the diff
    /// shows is the same line either way — only the walk that reached it
    /// differs.
    const INSIDE_A_COMMENT: &str = "\
diff --git a/src/a.rs b/src/a.rs
--- a/src/a.rs
+++ b/src/a.rs
@@ -2,1 +2,1 @@
-let x = 1;
+let x = 2;
";
    const COMMENTED_OUT: &str = "/* an old idea:\nlet x = 2;\n*/\nfn main() {}\n";

    #[test]
    fn the_file_is_walked_into_the_hunk() {
        let colors = colors(&patches(INSIDE_A_COMMENT), Some(COMMENTED_OUT));
        let line = colors.line(0, 0, 1);
        assert_eq!(
            line.spans.len(),
            1,
            "line 2 of that file is inside a comment, whatever it says: {line:?}"
        );
    }

    #[test]
    fn without_the_file_a_hunk_starts_cold() {
        // The same patch, and the difference is the whole point of
        // fetching the file: cold, the lexer has no idea it is inside
        // anything and reads a statement.
        let colors = colors(&patches(INSIDE_A_COMMENT), None);
        assert!(colors.line(0, 0, 1).spans.len() > 1);
    }

    #[test]
    fn a_file_that_disagrees_with_the_hunk_is_not_used() {
        // Saved between the diff and the read: line 2 is not what the
        // hunk says is there, so walking it would place the reading
        // somewhere the diff never was.
        let elsewhere = "/* an old idea:\nsomething else entirely\n*/\nfn main() {}\n";
        let colors = colors(&patches(INSIDE_A_COMMENT), Some(elsewhere));
        assert!(
            colors.line(0, 0, 1).spans.len() > 1,
            "fell back to the cold read rather than trusting the wrong file"
        );
    }

    #[test]
    fn binary_patches_are_left_alone() {
        let patch = "\
diff --git a/logo.png b/logo.png
--- a/logo.png
+++ b/logo.png
Binary files a/logo.png and b/logo.png differ
";
        assert!(colors(&patches(patch), None).is_empty());
    }
}
