//! Unified-diff (patch) parser.
//!
//! Produces line-classified data with old/new line numbers so the UI can
//! render a unified diff without interpreting anything itself. Tolerant of
//! multi-file patches; commands are expected to run with `--no-ext-diff`
//! and default `a/` `b/` prefixes (the diff runners pin these).

/// Classification of one diff content line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Addition,
    Deletion,
    /// `\ No newline at end of file`
    NoNewline,
}

/// One rendered line of a hunk (text without its marker char).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    /// Line number on the old side (context/deletion).
    pub old_no: Option<u32>,
    /// Line number on the new side (context/addition).
    pub new_no: Option<u32>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    /// Function-context heading after the closing `@@` (may be empty).
    pub heading: String,
    pub lines: Vec<DiffLine>,
}

/// One file's patch within the parsed output.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilePatch {
    /// Path on the old side (`None` for added files).
    pub old_path: Option<String>,
    /// Path on the new side (`None` for deleted files).
    pub new_path: Option<String>,
    pub is_binary: bool,
    pub hunks: Vec<DiffHunk>,
}

impl FilePatch {
    /// Display path: new side, falling back to old (deleted files).
    pub fn path(&self) -> &str {
        self.new_path
            .as_deref()
            .or(self.old_path.as_deref())
            .unwrap_or("")
    }
}

/// Parses `git diff` / `git diff-tree -p` output into per-file patches.
///
/// Unknown lines are skipped (with a trace log) rather than failing: a diff
/// that renders slightly incomplete beats an empty error pane.
pub fn parse_patch(bytes: &[u8]) -> Vec<FilePatch> {
    let text = String::from_utf8_lossy(bytes);
    let mut files: Vec<FilePatch> = Vec::new();
    let mut current: Option<FilePatch> = None;
    let mut hunk: Option<DiffHunk> = None;
    let mut old_no = 0u32;
    let mut new_no = 0u32;

    fn flush_hunk(current: &mut Option<FilePatch>, hunk: &mut Option<DiffHunk>) {
        if let (Some(file), Some(h)) = (current.as_mut(), hunk.take()) {
            file.hunks.push(h);
        }
    }
    fn flush_file(files: &mut Vec<FilePatch>, current: &mut Option<FilePatch>) {
        if let Some(f) = current.take() {
            files.push(f);
        }
    }

    // `str::lines` never yields a phantom empty line after the final
    // newline and strips one trailing `\r` (interior `\r` stays: CRLF file
    // content is data).
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            flush_hunk(&mut current, &mut hunk);
            flush_file(&mut files, &mut current);
            let mut file = FilePatch::default();
            // Fallback paths from the header; `---`/`+++` refine them.
            if let Some((a, b)) = split_git_header_paths(rest) {
                file.old_path = strip_prefix_a(&a);
                file.new_path = strip_prefix_b(&b);
            }
            current = Some(file);
            continue;
        }

        if hunk.is_none() {
            // Pre-hunk header lines of the current file.
            if let Some(rest) = line.strip_prefix("--- ") {
                if let Some(f) = current.as_mut() {
                    f.old_path = parse_side_path(rest, "a/");
                }
                continue;
            }
            if let Some(rest) = line.strip_prefix("+++ ") {
                if let Some(f) = current.as_mut() {
                    f.new_path = parse_side_path(rest, "b/");
                }
                continue;
            }
            if line.starts_with("Binary files ") || line.starts_with("GIT binary patch") {
                if let Some(f) = current.as_mut() {
                    f.is_binary = true;
                }
                continue;
            }
        }

        if let Some(rest) = line.strip_prefix("@@ ") {
            flush_hunk(&mut current, &mut hunk);
            if current.is_none() {
                // Patch without a `diff --git` header (plain `git diff`
                // between blobs); synthesize a file entry.
                current = Some(FilePatch::default());
            }
            if let Some(h) = parse_hunk_header(rest) {
                old_no = h.old_start;
                new_no = h.new_start;
                hunk = Some(h);
            } else {
                tracing::trace!(line, "unparsable hunk header");
            }
            continue;
        }

        let Some(h) = hunk.as_mut() else {
            continue; // other header noise (index, mode, similarity, ...)
        };
        let mut chars = line.chars();
        match chars.next() {
            Some(' ') => {
                h.lines.push(DiffLine {
                    kind: DiffLineKind::Context,
                    old_no: Some(old_no),
                    new_no: Some(new_no),
                    text: chars.as_str().to_string(),
                });
                old_no += 1;
                new_no += 1;
            }
            Some('+') => {
                h.lines.push(DiffLine {
                    kind: DiffLineKind::Addition,
                    old_no: None,
                    new_no: Some(new_no),
                    text: chars.as_str().to_string(),
                });
                new_no += 1;
            }
            Some('-') => {
                h.lines.push(DiffLine {
                    kind: DiffLineKind::Deletion,
                    old_no: Some(old_no),
                    new_no: None,
                    text: chars.as_str().to_string(),
                });
                old_no += 1;
            }
            Some('\\') => {
                h.lines.push(DiffLine {
                    kind: DiffLineKind::NoNewline,
                    old_no: None,
                    new_no: None,
                    text: line.to_string(),
                });
            }
            None => {
                // A completely empty line inside a hunk is a context line
                // whose content is empty (git prints a lone space, but some
                // tools strip trailing whitespace; tolerate).
                h.lines.push(DiffLine {
                    kind: DiffLineKind::Context,
                    old_no: Some(old_no),
                    new_no: Some(new_no),
                    text: String::new(),
                });
                old_no += 1;
                new_no += 1;
            }
            _ => tracing::trace!(line, "unexpected line inside hunk"),
        }
    }
    flush_hunk(&mut current, &mut hunk);
    flush_file(&mut files, &mut current);
    files
}

/// `-a,b +c,d @@ heading` (counts default to 1 when omitted).
fn parse_hunk_header(rest: &str) -> Option<DiffHunk> {
    let (ranges, heading) = match rest.split_once("@@") {
        Some((r, h)) => (r.trim(), h.trim().to_string()),
        None => (rest.trim(), String::new()),
    };
    let mut parts = ranges.split_whitespace();
    let old = parts.next()?.strip_prefix('-')?;
    let new = parts.next()?.strip_prefix('+')?;
    let parse_range = |s: &str| -> Option<(u32, u32)> {
        match s.split_once(',') {
            Some((a, b)) => Some((a.parse().ok()?, b.parse().ok()?)),
            None => Some((s.parse().ok()?, 1)),
        }
    };
    let (old_start, old_count) = parse_range(old)?;
    let (new_start, new_count) = parse_range(new)?;
    Some(DiffHunk {
        old_start,
        old_count,
        new_start,
        new_count,
        heading,
        lines: Vec::new(),
    })
}

/// Path after `---` / `+++`: `/dev/null`, or `<prefix>path` with an
/// optional trailing tab git adds for paths containing spaces.
fn parse_side_path(rest: &str, prefix: &str) -> Option<String> {
    let cleaned = rest.trim_end_matches('\t');
    if cleaned == "/dev/null" {
        return None;
    }
    let path = cleaned.strip_prefix(prefix).unwrap_or(cleaned);
    Some(path.to_string())
}

fn strip_prefix_a(p: &str) -> Option<String> {
    parse_side_path(p, "a/")
}

fn strip_prefix_b(p: &str) -> Option<String> {
    parse_side_path(p, "b/")
}

/// Best-effort split of `a/old b/new` from the `diff --git` line. Paths
/// with spaces make this ambiguous; `---`/`+++` lines are authoritative.
fn split_git_header_paths(rest: &str) -> Option<(String, String)> {
    let idx = rest.find(" b/")?;
    let a = rest[..idx].to_string();
    let b = rest[idx + 1..].to_string();
    Some((a, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATCH: &str = "\
diff --git a/src/main.rs b/src/main.rs
index 1111111..2222222 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,4 +1,5 @@ fn main
 line one
-line two
+line two changed
+line two point five
 line three
@@ -10,2 +11,2 @@
 tail one
-tail two
+tail two changed
diff --git a/added.txt b/added.txt
new file mode 100644
index 0000000..3333333
--- /dev/null
+++ b/added.txt
@@ -0,0 +1,1 @@
+hello
\\ No newline at end of file
diff --git a/logo.png b/logo.png
index 4444444..5555555 100644
Binary files a/logo.png and b/logo.png differ
";

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
