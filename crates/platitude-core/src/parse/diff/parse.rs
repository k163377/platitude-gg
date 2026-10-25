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
/// A record, not locals, so each phase of the walk shows which fields it
/// touches.
#[derive(Default)]
struct Reading {
    files: Vec<FilePatch>,
    current: Option<FilePatch>,
    hunk: Option<DiffHunk>,
    old_no: u32,
    new_no: u32,
    /// Where each parent of the open hunk has got to: empty for a unified
    /// diff (its one old side is `old_no`), one per parent for a combined
    /// one.
    parent_no: Vec<u32>,
    /// The commit the patches that follow are of, where the stream names
    /// one (`\u{1}` at the head of a line — `DiffTarget::Choice`). Empty
    /// for a diff of one thing.
    label: String,
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
            file.from_commit = self.label.clone();
            self.current = Some(file);
            return true;
        }

        // A combined header names one path; `---`/`+++` follow and carry
        // the prefixes.
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
                from_commit: self.label.clone(),
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
        // A mode change comes as a pair, `old mode` first.
        if let Some(f) = self.current.as_mut() {
            if let Some(old) = line.strip_prefix("old mode ") {
                f.mode_change = Some((old.trim().to_string(), String::new()));
            } else if let Some(new) = line.strip_prefix("new mode ")
                && let Some((_, to)) = f.mode_change.as_mut()
            {
                *to = new.trim().to_string();
            }
        }
        // The mode lines: a gitlink says `160000` on whichever its side has
        // (`index a..b 160000`, `new file mode 160000`, `old mode …`).
        if [
            "index ",
            "new file mode ",
            "deleted file mode ",
            "old mode ",
            "new mode ",
        ]
        .iter()
        .any(|head| line.starts_with(head))
        {
            if line.ends_with(" 160000")
                && let Some(f) = self.current.as_mut()
            {
                f.gitlink = true;
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

    /// A content line of the open hunk. The walk's last resort: header
    /// noise outside a hunk (index, mode, similarity) is dropped here.
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
/// Unknown lines are skipped (with a trace log): a diff that renders
/// slightly incomplete beats an empty error pane.
pub fn parse_patch(bytes: &[u8]) -> Vec<FilePatch> {
    let text = String::from_utf8_lossy(bytes);
    let mut reading = Reading::default();
    // `str::lines` never yields a phantom empty line after the final
    // newline and strips one trailing CR (an interior one stays: CRLF file
    // content is data).
    for line in text.lines() {
        // A stream of several commits' patches names each before its own
        // (`DiffTarget::Choice`). No diff line can begin with this byte:
        // git never writes it, and a file holding it is printed as binary.
        if let Some(rest) = line.strip_prefix('\u{1}') {
            // Close the previous file first: the blank line git writes
            // after the record would land in its last hunk as a context
            // line.
            reading.flush_file();
            reading.label = rest.to_string();
            continue;
        }
        if reading.opens_file(line) {
            continue;
        }
        if reading.hunk.is_none() && reading.reads_header(line) {
            continue;
        }
        // A body line always opens with its marker columns (` `, `+`, `-`
        // or a lone `\`), so a run of `@` at the head is a hunk header.
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
    use super::super::testkit::PATCH;
    use super::super::{DiffLineKind, DiffLineLocation};
    use super::*;

    #[test]
    fn a_location_reads_back_the_line_it_names() {
        let files = parse_patch(PATCH.as_bytes());
        let text = |patch, hunk, line| {
            DiffLineLocation { patch, hunk, line }
                .line_in(&files)
                .map(|found| found.text.as_str())
        };
        assert_eq!(text(0, 0, 1), Some("line two"));
        assert_eq!(text(0, 0, 2), Some("line two changed"), "another line");
        assert_eq!(text(0, 1, 2), Some("tail two changed"), "another hunk");
        assert_eq!(text(1, 0, 0), Some("hello"), "another patch");
        assert_eq!(text(3, 0, 0), None, "no fourth patch");
        assert_eq!(text(2, 0, 0), None, "a binary patch has no hunk");
        assert_eq!(text(0, 2, 0), None, "no third hunk");
        assert_eq!(text(0, 1, 3), None, "no fourth line");
    }

    /// A table laid out as the lines are: each entry names its own place.
    #[test]
    fn a_location_reads_back_its_entry_of_a_table() {
        let table = vec![
            vec![vec!["0.0.0", "0.0.1"], vec!["0.1.0"]],
            vec![vec!["1.0.0", "1.0.1", "1.0.2"]],
        ];
        let entry = |patch, hunk, line| {
            DiffLineLocation { patch, hunk, line }
                .entry_in(&table)
                .copied()
        };
        assert_eq!(entry(0, 0, 1), Some("0.0.1"));
        assert_eq!(entry(1, 0, 1), Some("1.0.1"), "another patch");
        assert_eq!(entry(0, 1, 0), Some("0.1.0"), "another hunk");
        assert_eq!(entry(1, 0, 2), Some("1.0.2"), "another line");
        assert_eq!(entry(2, 0, 0), None, "no third patch");
        assert_eq!(entry(1, 1, 0), None, "no second hunk");
        assert_eq!(entry(0, 1, 1), None, "no second line");
    }

    #[test]
    fn locations_run_in_the_diff_s_order() {
        let at = |patch, hunk, line| DiffLineLocation { patch, hunk, line };
        assert!(
            at(0, 9, 9) < at(1, 0, 0),
            "a patch before its hunks and lines"
        );
        assert!(at(1, 0, 9) < at(1, 1, 0), "a hunk before its lines");
        assert!(at(1, 1, 0) < at(1, 1, 1));
    }

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
        assert_eq!(added.hunks[0].lines[1].old_no, None);
        assert_eq!(added.hunks[0].lines[1].new_no, None);
        assert_eq!(added.hunks[0].lines[1].text, "\\ No newline at end of file");

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
        assert_eq!((lines[1].old_no, lines[1].new_no), (Some(2), Some(2)));
        assert_eq!(lines[2].old_no, Some(3), "empty context advanced both");
        assert_eq!(lines[3].new_no, Some(3), "empty context advanced both");
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

    /// git 2.55's patch of a moved gitlink, and of one newly staged: the
    /// mode is on the `index` line or the `new file mode` line.
    #[test]
    fn a_gitlink_says_so_in_its_mode() {
        let moved = "\
diff --git a/vendor/nest b/vendor/nest
index 8da2dfc..1d97e67 160000
--- a/vendor/nest
+++ b/vendor/nest
@@ -1 +1 @@
-Subproject commit 8da2dfce92c89b7e81b20dc0b59c3f748369c33b
+Subproject commit 1d97e6774296644ee38675b47710bf8fcef90186
";
        let added = "\
diff --git a/vendor/nest b/vendor/nest
new file mode 160000
index 0000000..1d97e67
--- /dev/null
+++ b/vendor/nest
@@ -0,0 +1 @@
+Subproject commit 1d97e6774296644ee38675b47710bf8fcef90186
";
        let file = "\
diff --git a/a.txt b/a.txt
index 1111111..2222222 100644
--- a/a.txt
+++ b/a.txt
@@ -1 +1 @@
-one
+two
";
        assert!(parse_patch(moved.as_bytes())[0].gitlink);
        assert!(parse_patch(added.as_bytes())[0].gitlink);
        assert!(!parse_patch(file.as_bytes())[0].gitlink);
        assert_eq!(
            parse_patch(moved.as_bytes())[0].hunks[0].lines.len(),
            2,
            "the mode line is header, not a row"
        );
    }

    /// git 2.55's patch of a file made executable along with a line, and of
    /// one whose mode alone changed: the pair is read, and is no row.
    #[test]
    fn a_mode_change_is_read_off_its_pair() {
        let with_lines = "\
diff --git a/tool.sh b/tool.sh
old mode 100644
new mode 100755
index 21797cf..26af3fb
--- a/tool.sh
+++ b/tool.sh
@@ -24,3 +24,3 @@ echo \"step 22 of the tool\"
 echo \"step 24 of the tool\"
-echo \"step 25 of the tool\"
+echo \"step 25 of the tool, reworded\"
";
        let alone = "\
diff --git a/tool.sh b/tool.sh
old mode 100755
new mode 100644
";
        let changed = parse_patch(with_lines.as_bytes());
        assert_eq!(
            changed[0].mode_change,
            Some(("100644".to_string(), "100755".to_string()))
        );
        assert_eq!(changed[0].hunks[0].lines.len(), 3);
        let bare = parse_patch(alone.as_bytes());
        assert_eq!(
            bare[0].mode_change,
            Some(("100755".to_string(), "100644".to_string()))
        );
        assert!(bare[0].hunks.is_empty());
        assert_eq!(bare[0].new_path.as_deref(), Some("tool.sh"));
    }
}
