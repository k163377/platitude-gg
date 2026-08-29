//! The line-by-line walk over a patch, unified and combined alike.

use super::combined::read_combined_line;
use super::header::{
    parse_hunk_header, parse_side_path, split_git_header_paths, strip_prefix_a, strip_prefix_b,
};
use super::unified::read_unified_line;
use super::{DiffHunk, FilePatch};

/// What one pass over a patch is building: the files closed so far, the one
/// being read, and where each side of its open hunk has got to.
///
/// A record rather than six locals because the walk below has four phases and
/// each touches a different three or four of them — as locals, every phase
/// reads as if it could touch all six.
#[derive(Default)]
struct Reading {
    files: Vec<FilePatch>,
    current: Option<FilePatch>,
    hunk: Option<DiffHunk>,
    old_no: u32,
    new_no: u32,
    /// Where each parent of the hunk being read has got to. Empty for a
    /// unified diff, which counts its one old side in `old_no`; one entry
    /// per parent for a combined one.
    parent_no: Vec<u32>,
}

impl Reading {
    fn flush_hunk(&mut self) {
        if let (Some(file), Some(h)) = (self.current.as_mut(), self.hunk.take()) {
            file.hunks.push(h);
        }
    }

    fn flush_file(&mut self) {
        self.flush_hunk();
        if let Some(f) = self.current.take() {
            self.files.push(f);
        }
    }

    /// The three lines that start a new file, and whether this was one.
    fn opens_file(&mut self, line: &str) -> bool {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            self.flush_file();
            let mut file = FilePatch::default();
            // Fallback paths from the header; `---`/`+++` refine them.
            if let Some((a, b)) = split_git_header_paths(rest) {
                file.old_path = strip_prefix_a(&a);
                file.new_path = strip_prefix_b(&b);
            }
            self.current = Some(file);
            return true;
        }

        // A combined header names one path, not a pair: there is no single
        // old side to name. `---`/`+++` follow and carry the prefixes.
        if let Some(rest) = line
            .strip_prefix("diff --cc ")
            .or_else(|| line.strip_prefix("diff --combined "))
        {
            self.flush_file();
            let path = rest.trim().to_string();
            self.current = Some(FilePatch {
                old_path: Some(path.clone()),
                new_path: Some(path),
                is_combined: true,
                ..FilePatch::default()
            });
            return true;
        }

        if let Some(path) = line.strip_prefix("* Unmerged path ") {
            self.flush_file();
            self.files.push(FilePatch {
                old_path: None,
                new_path: Some(path.trim().to_string()),
                unmerged: true,
                ..FilePatch::default()
            });
            return true;
        }
        false
    }

    /// The pre-hunk header lines that refine the file being read, and
    /// whether this was one. Asked only while no hunk is open: past that
    /// point a `--- ` is a removed line whose content starts with dashes.
    fn reads_header(&mut self, line: &str) -> bool {
        if let Some(rest) = line.strip_prefix("--- ") {
            if let Some(f) = self.current.as_mut() {
                f.old_path = parse_side_path(rest, "a/");
            }
            return true;
        }
        if let Some(rest) = line.strip_prefix("+++ ") {
            if let Some(f) = self.current.as_mut() {
                f.new_path = parse_side_path(rest, "b/");
            }
            return true;
        }
        if line.starts_with("Binary files ") || line.starts_with("GIT binary patch") {
            if let Some(f) = self.current.as_mut() {
                f.is_binary = true;
            }
            return true;
        }
        false
    }

    /// A hunk header, which closes whatever hunk was open and starts the
    /// counting again from what it names.
    fn opens_hunk(&mut self, line: &str) {
        self.flush_hunk();
        if self.current.is_none() {
            // Patch without a `diff --git` header (plain `git diff`
            // between blobs); synthesize a file entry.
            self.current = Some(FilePatch::default());
        }
        let Some(h) = parse_hunk_header(line) else {
            tracing::trace!(line, "unparsable hunk header");
            return;
        };
        self.old_no = h.old_start;
        self.new_no = h.new_start;
        self.parent_no = if h.extra_old.is_empty() {
            Vec::new()
        } else {
            std::iter::once(h.old_start)
                .chain(h.extra_old.iter().map(|(start, _)| *start))
                .collect()
        };
        if !self.parent_no.is_empty()
            && let Some(f) = self.current.as_mut()
        {
            // A combined hunk under a synthesized file entry: the
            // shape says what the header would have.
            f.is_combined = true;
        }
        self.hunk = Some(h);
    }

    /// A content line of the open hunk. Header noise outside one — index,
    /// mode, similarity — reaches here and is dropped, which is what makes
    /// this the walk's last resort rather than a case of its own.
    fn reads_body(&mut self, line: &str) {
        let Some(h) = self.hunk.as_mut() else {
            return;
        };
        if self.parent_no.is_empty() {
            read_unified_line(h, line, &mut self.old_no, &mut self.new_no);
        } else {
            read_combined_line(h, line, &mut self.parent_no, &mut self.new_no);
        }
    }

    fn finish(mut self) -> Vec<FilePatch> {
        self.flush_file();
        self.files
    }
}

/// Parses `git diff` / `git diff-tree -p` output into per-file patches.
///
/// Unknown lines are skipped (with a trace log) rather than failing: a diff
/// that renders slightly incomplete beats an empty error pane.
pub fn parse_patch(bytes: &[u8]) -> Vec<FilePatch> {
    let text = String::from_utf8_lossy(bytes);
    let mut reading = Reading::default();
    // `str::lines` never yields a phantom empty line after the final
    // newline and strips one trailing CR (an interior one stays: CRLF file
    // content is data).
    for line in text.lines() {
        if reading.opens_file(line) {
            continue;
        }
        if reading.hunk.is_none() && reading.reads_header(line) {
            continue;
        }
        // A body line always opens with its marker columns (` `, `+`, `-`
        // or a lone `\`), so a run of `@` at the head is unambiguously a
        // hunk header — of one `@` per parent plus one.
        if line.starts_with("@@") {
            reading.opens_hunk(line);
            continue;
        }
        reading.reads_body(line);
    }
    reading.finish()
}

#[cfg(test)]
mod tests {
    use super::super::DiffLineKind;
    use super::super::testkit::PATCH;
    use super::*;
    #[test]
    fn parses_a_multi_file_patch() {
        let files = parse_patch(PATCH.as_bytes());
        assert_eq!(files.len(), 3);

        let main = &files[0];
        assert_eq!(main.path(), "src/main.rs");
        assert_eq!(main.old_path.as_deref(), Some("src/main.rs"));
        assert_eq!(main.hunks.len(), 2);
        assert!(!main.is_binary);

        let h1 = &main.hunks[0];
        assert_eq!(
            (h1.old_start, h1.old_count, h1.new_start, h1.new_count),
            (1, 4, 1, 5)
        );
        assert_eq!(h1.heading, "fn main");
        let kinds: Vec<DiffLineKind> = h1.lines.iter().map(|l| l.kind).collect();
        assert_eq!(
            kinds,
            vec![
                DiffLineKind::Context,
                DiffLineKind::Deletion,
                DiffLineKind::Addition,
                DiffLineKind::Addition,
                DiffLineKind::Context,
            ]
        );
        // Line numbering: context advances both, del only old, add only new.
        assert_eq!(h1.lines[0].old_no, Some(1));
        assert_eq!(h1.lines[0].new_no, Some(1));
        assert_eq!(h1.lines[1].old_no, Some(2));
        assert_eq!(h1.lines[1].new_no, None);
        assert_eq!(h1.lines[2].old_no, None);
        assert_eq!(h1.lines[2].new_no, Some(2));
        assert_eq!(h1.lines[4].old_no, Some(3));
        assert_eq!(h1.lines[4].new_no, Some(4));
        assert_eq!(h1.lines[1].text, "line two");

        let added = &files[1];
        assert_eq!(added.old_path, None, "added file has no old side");
        assert_eq!(added.path(), "added.txt");
        assert_eq!(added.hunks[0].lines[0].new_no, Some(1));
        assert_eq!(added.hunks[0].lines[1].kind, DiffLineKind::NoNewline);

        let binary = &files[2];
        assert!(binary.is_binary);
        assert!(binary.hunks.is_empty());
    }

    #[test]
    fn parses_deleted_file() {
        let patch = "\
diff --git a/gone.txt b/gone.txt
deleted file mode 100644
--- a/gone.txt
+++ /dev/null
@@ -1,1 +0,0 @@
-bye
";
        let files = parse_patch(patch.as_bytes());
        assert_eq!(files[0].new_path, None);
        assert_eq!(files[0].path(), "gone.txt");
        assert_eq!(files[0].hunks[0].lines[0].kind, DiffLineKind::Deletion);
    }

    #[test]
    fn tolerates_paths_with_spaces_via_side_lines() {
        let patch = "\
diff --git a/has space.txt b/has space.txt
--- a/has space.txt\t
+++ b/has space.txt\t
@@ -1 +1 @@
-x
+y
";
        let files = parse_patch(patch.as_bytes());
        assert_eq!(files[0].path(), "has space.txt");
    }

    #[test]
    fn empty_context_line_is_tolerated() {
        let patch = "\
--- a/f
+++ b/f
@@ -1,3 +1,3 @@
 a

-b
+c
";
        let files = parse_patch(patch.as_bytes());
        let lines = &files[0].hunks[0].lines;
        assert_eq!(lines[1].kind, DiffLineKind::Context);
        assert_eq!(lines[1].text, "");
        assert_eq!(lines[2].old_no, Some(3), "empty context advanced both");
    }

    #[test]
    fn crlf_patch_lines_are_normalized_but_content_kept() {
        let patch = "--- a/f\r\n+++ b/f\r\n@@ -1 +1 @@\r\n-x\r\n+y\r\n";
        let files = parse_patch(patch.as_bytes());
        assert_eq!(files[0].hunks[0].lines[0].text, "x");
    }

    #[test]
    fn empty_input_yields_no_files() {
        assert!(parse_patch(b"").is_empty());
    }
}
