//! Syntax colours for the lines a diff shows.
//!
//! What arrives here is a parsed patch, not a file: a hunk and the few
//! lines of context around it. So the reading starts clean at every hunk
//! heading rather than at the top of a file nobody fetched, and a block
//! comment that opened fifty lines above the hunk is not known about. The
//! seam for ever fixing that is the shape of [`colors`]: what a line is
//! read against is a state value carried from the line before, so a
//! caller that does hold the whole file can walk it into the hunk instead
//! of starting over. Nothing downstream would change — the rows are
//! addressed the same way either way.
//!
//! # Conflicts
//!
//! The working tree of a conflicted file has git's markers in it, and the
//! two sides between them are **alternatives rather than a sequence**:
//! read one straight after the other, a lexer carries whatever `ours`
//! left open — a `/*`, a `"""` — into `theirs`, and everything below is
//! painted as the inside of something that is not there. So the markers
//! are read as structure. The state is saved where the region opens, put
//! back at each divide, and what the file goes on in is where **ours**
//! left off: that is the side the branch being worked on is standing on,
//! and where the two disagree nothing further down is certain anyway.
//!
//! Marker lines are matched exactly — seven characters, then the end of
//! the line or a space and a label — and **only inside a combined diff**,
//! which is the only form git prints for a path it stopped on. A row of
//! `=` under a heading in Markdown, or a page of documentation quoting a
//! marker, is text like any other.

use std::sync::OnceLock;

use syntect::highlighting::{HighlightIterator, HighlightState, Highlighter, Theme, ThemeSet};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

use crate::parse::diff::{DiffLineKind, FilePatch};

/// The theme token colours are taken from. Only foregrounds are read:
/// backgrounds, the row wash and the diff's own green and red stay the
/// app's (デザイン規約 §シンタックスハイライト).
const THEME: &str = "base16-ocean.dark";

/// A colour as the theme gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// One run of a line the theme paints in a single colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// Length of the run in **bytes** of [`crate::parse::diff::DiffLine`]'s
    /// text — the same string the row will draw, so the runs can be laid
    /// over it without measuring anything twice.
    pub len: usize,
    pub color: Rgb,
}

/// What was read off one line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineColors {
    /// The theme's runs, empty where the line has none.
    pub spans: Vec<Span>,
    /// One of git's conflict markers — the fence around a region rather
    /// than anything the file says, so the pane draws it as scaffolding.
    /// Read whether or not the language is one the set knows: a conflict
    /// in a plain text file has the same fences.
    pub fence: bool,
}

impl LineColors {
    fn nothing() -> Self {
        Self::default()
    }

    fn said_nothing(&self) -> bool {
        self.spans.is_empty() && !self.fence
    }
}

type HunkColors = Vec<LineColors>;
type PatchColors = Vec<HunkColors>;

/// Colours for one diff, addressed the way its rows already are: which
/// patch, which hunk, which line of it.
///
/// Every patch and every line gets an entry even where there is nothing
/// to say — a binary file, a language nothing was found for — so the
/// shape always matches the diff it was made from and a caller can walk
/// the two together without counting.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffColors {
    patches: Vec<PatchColors>,
}

impl DiffColors {
    /// What was read off one line — nothing, where nothing was. Borrowed
    /// rather than handed over: the caller asks once per row of a diff
    /// that can be tens of thousands of rows long.
    pub fn line(&self, patch: usize, hunk: usize, line: usize) -> &LineColors {
        static NOTHING: LineColors = LineColors {
            spans: Vec::new(),
            fence: false,
        };
        self.patches
            .get(patch)
            .and_then(|p| p.get(hunk))
            .and_then(|h| h.get(line))
            .unwrap_or(&NOTHING)
    }

    /// Whether nothing in this diff was read — the pane then draws
    /// exactly what it drew before there was any of this.
    pub fn is_empty(&self) -> bool {
        self.patches
            .iter()
            .flatten()
            .all(|hunk| hunk.iter().all(LineColors::said_nothing))
    }
}

/// Reads every text line of `patches` and answers what colour each run of
/// it is. Best effort throughout: a language that is not in the set, a
/// binary patch, a regex that will not run — each of those is a diff that
/// comes out the colour it always was, never an error and never a gap.
///
/// This is CPU work with no waiting in it. Callers on an async runtime
/// should hand it to a blocking thread rather than hold a worker.
pub fn colors(patches: &[FilePatch]) -> DiffColors {
    let assets = assets();
    DiffColors {
        patches: patches
            .iter()
            .map(|patch| patch_colors(assets, patch))
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// The set and the theme
// ---------------------------------------------------------------------------

struct Assets {
    syntaxes: SyntaxSet,
    theme: Theme,
}

/// Loaded once, on the first diff that asks — never at startup. A window
/// that is opened and closed without a file being read pays none of it,
/// and the load is inside the same background hop as the diff itself.
fn assets() -> &'static Assets {
    static ASSETS: OnceLock<Assets> = OnceLock::new();
    ASSETS.get_or_init(|| Assets {
        syntaxes: two_face::syntax::extra_newlines(),
        theme: ThemeSet::load_defaults()
            .themes
            .remove(THEME)
            .unwrap_or_default(),
    })
}

/// The language for a path, or `None` — which is the answer for every
/// file the set has never heard of, and the reason a plain-text diff
/// looks exactly as it did.
fn syntax_for<'a>(syntaxes: &'a SyntaxSet, path: &str) -> Option<&'a SyntaxReference> {
    let name = path.rsplit('/').next().unwrap_or(path);
    // Extension first, then the whole name: Sublime's definitions list
    // `Makefile` and `Dockerfile` among their extensions, and a dotfile
    // (`.gitignore`) reads as an extension already.
    let ext = name.rsplit_once('.').map_or(name, |(_, e)| e);
    syntaxes
        .find_syntax_by_extension(ext)
        .or_else(|| syntaxes.find_syntax_by_extension(name))
}

// ---------------------------------------------------------------------------
// Walking a patch
// ---------------------------------------------------------------------------

fn patch_colors(assets: &Assets, patch: &FilePatch) -> PatchColors {
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
    patch
        .hunks
        .iter()
        .map(|hunk| {
            let mut walk = Walk::new(assets, &highlighter, syntax, patch.is_combined);
            hunk.lines.iter().map(|line| walk.read(line)).collect()
        })
        .collect()
}

/// Where the reading of a hunk has got to.
struct Walk<'a> {
    assets: &'a Assets,
    highlighter: &'a Highlighter<'a>,
    /// `None` for a language the set does not know — the fences are still
    /// read, the words are simply left the colour they were.
    state: Option<LineState>,
    /// Only a combined diff — the form git prints for a path it stopped
    /// on — has its markers read as structure. See the module note.
    combined: bool,
    region: Option<Region>,
}

/// A lexer's place between two lines. Cloned to stand a conflict's sides
/// beside each other rather than after one another.
#[derive(Clone)]
struct LineState {
    parse: ParseState,
    highlight: HighlightState,
}

/// A conflict region being walked through.
struct Region {
    /// Where the lexer stood on the line above `<<<<<<<`. Both later
    /// sides start from here.
    entry: LineState,
    /// Where `ours` left off — what the file goes on in once the region
    /// closes. `None` until the first divide is reached.
    ours_end: Option<LineState>,
    /// Whether the side being read now is the first one, which is the
    /// only one whose ending is kept.
    in_ours: bool,
}

/// Which of git's conflict markers a line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Marker {
    /// `<<<<<<< ours`
    Open,
    /// `||||||| base` — diff3 and zdiff3 only.
    Base,
    /// `=======`
    Split,
    /// `>>>>>>> theirs`
    Close,
}

impl<'a> Walk<'a> {
    fn new(
        assets: &'a Assets,
        highlighter: &'a Highlighter<'a>,
        syntax: Option<&SyntaxReference>,
        combined: bool,
    ) -> Self {
        Self {
            assets,
            highlighter,
            state: syntax.map(|syntax| LineState {
                parse: ParseState::new(syntax),
                highlight: HighlightState::new(highlighter, ScopeStack::new()),
            }),
            combined,
            region: None,
        }
    }

    fn read(&mut self, line: &crate::parse::diff::DiffLine) -> LineColors {
        // `\ No newline at end of file` is git talking, not the file.
        if line.kind == DiffLineKind::NoNewline {
            return LineColors::nothing();
        }
        if self.combined
            && let Some(marker) = conflict_marker(&line.text)
        {
            self.cross(marker);
            // No runs: the fence is not the file talking, and the pane
            // draws it as the scaffolding it is.
            return LineColors {
                spans: Vec::new(),
                fence: true,
            };
        }
        LineColors {
            spans: self.paint(&line.text),
            fence: false,
        }
    }

    /// Moves the lexer over one of git's markers.
    fn cross(&mut self, marker: Marker) {
        // Nothing to put back where no lexer is running — the fences of a
        // language the set does not know are still fences.
        if self.state.is_none() {
            return;
        }
        match marker {
            Marker::Open => {
                self.region = self.state.clone().map(|entry| Region {
                    entry,
                    ours_end: None,
                    in_ours: true,
                });
            }
            Marker::Base | Marker::Split => {
                let here = self.state.clone();
                if let Some(region) = self.region.as_mut() {
                    if region.in_ours {
                        region.ours_end = here;
                        region.in_ours = false;
                    }
                    self.state = Some(region.entry.clone());
                }
            }
            Marker::Close => {
                if let Some(region) = self.region.take() {
                    self.state = Some(region.ours_end.unwrap_or(region.entry));
                }
            }
        }
    }

    fn paint(&mut self, text: &str) -> Vec<Span> {
        // Split so the lexer's own state can be borrowed apart from the
        // set and the theme it reads against.
        let Walk {
            assets,
            highlighter,
            state,
            ..
        } = self;
        let Some(state) = state.as_mut() else {
            return Vec::new();
        };
        // syntect's default set is the one built for lines that still
        // carry their terminator: several of its contexts close on `$`,
        // and a line handed over without one holds them open.
        let mut buf = String::with_capacity(text.len() + 1);
        buf.push_str(text);
        buf.push('\n');
        let Ok(ops) = state.parse.parse_line(&buf, &assets.syntaxes) else {
            // A rule that would not run leaves the state where it was.
            // One line goes out plain; the pane does not go out empty.
            return Vec::new();
        };
        let mut spans: Vec<Span> = Vec::new();
        for (style, piece) in HighlightIterator::new(&mut state.highlight, &ops, &buf, highlighter)
        {
            // The terminator was ours to add and is not part of the row.
            let len = piece.strip_suffix('\n').unwrap_or(piece).len();
            if len == 0 {
                continue;
            }
            let color = Rgb {
                r: style.foreground.r,
                g: style.foreground.g,
                b: style.foreground.b,
            };
            // Runs the theme paints alike are one run: the count is what
            // the row's markup costs, and a line of plain prose comes
            // back from the lexer in a dozen identical pieces.
            match spans.last_mut() {
                Some(last) if last.color == color => last.len += len,
                _ => spans.push(Span { len, color }),
            }
        }
        spans
    }
}

/// Whether a line is one of git's conflict markers, matched exactly:
/// seven of the character and no more, then the end of the line or a
/// space before whatever label git put there.
///
/// Only ever asked inside a combined diff (see the module note), so the
/// eight-character rule of `=` some file draws under a heading, and the
/// seven-character one Markdown draws under a title, are both text.
fn conflict_marker(line: &str) -> Option<Marker> {
    let bytes = line.as_bytes();
    let (marker, ch) = match bytes.first()? {
        b'<' => (Marker::Open, b'<'),
        b'|' => (Marker::Base, b'|'),
        b'=' => (Marker::Split, b'='),
        b'>' => (Marker::Close, b'>'),
        _ => return None,
    };
    if bytes.iter().take_while(|b| **b == ch).count() != 7 {
        return None;
    }
    match bytes.get(7) {
        None | Some(b' ') | Some(b'\t') => Some(marker),
        Some(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::diff::parse_patch;

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

    /// A conflict as git prints it for a path it stopped on: the working
    /// tree against both stages at once, two marker columns per row. The
    /// `ours` side opens a block comment and never closes it.
    const CONFLICTED: &str = "\
diff --cc src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@@ -1,3 -1,3 +1,7 @@@
  fn main() {
++<<<<<<< HEAD
++    /* ours
++=======
++    let x = 2;
++>>>>>>> other
  }
";

    fn patches(text: &str) -> Vec<FilePatch> {
        parse_patch(text.as_bytes())
    }

    #[test]
    fn marker_needs_exactly_seven() {
        assert_eq!(conflict_marker("<<<<<<< HEAD"), Some(Marker::Open));
        assert_eq!(conflict_marker("<<<<<<<"), Some(Marker::Open));
        assert_eq!(conflict_marker("======="), Some(Marker::Split));
        assert_eq!(conflict_marker("||||||| merged common"), Some(Marker::Base));
        assert_eq!(conflict_marker(">>>>>>> other/branch"), Some(Marker::Close));
        // Eight is a rule someone drew, not a marker git wrote.
        assert_eq!(conflict_marker("========"), None);
        assert_eq!(conflict_marker("<<<<<<<<"), None);
        // Six is not one either, and neither is a marker with something
        // pressed straight up against it.
        assert_eq!(conflict_marker("======"), None);
        assert_eq!(conflict_marker("=======x"), None);
        assert_eq!(conflict_marker("// ======="), None);
        assert_eq!(conflict_marker(""), None);
    }

    #[test]
    fn rust_lines_are_taken_apart() {
        let colors = colors(&patches(RUST));
        // `-    let x = 1;` is the second line of the only hunk.
        let deleted = colors.line(0, 0, 1);
        assert!(
            deleted.spans.len() > 1,
            "a line with a keyword and a number in it is not one colour: {deleted:?}"
        );
        let total: usize = deleted.spans.iter().map(|s| s.len).sum();
        assert_eq!(total, "    let x = 1;".len(), "runs cover the whole line");
    }

    #[test]
    fn kotlin_is_in_the_set() {
        let patch = "\
diff --git a/A.kt b/A.kt
--- a/A.kt
+++ b/A.kt
@@ -1,1 +1,1 @@
-fun a(): Int = 1
+fun b(): Int = 2
";
        let colors = colors(&patches(patch));
        assert!(
            colors.line(0, 0, 0).spans.len() > 1,
            "Kotlin is one of the languages two-face adds to Sublime's own set"
        );
    }

    #[test]
    fn a_language_nobody_knows_gets_no_colour() {
        let patch = "\
diff --git a/notes.qqq b/notes.qqq
--- a/notes.qqq
+++ b/notes.qqq
@@ -1,1 +1,1 @@
-one
+two
";
        assert!(colors(&patches(patch)).is_empty());
    }

    #[test]
    fn binary_patches_are_left_alone() {
        let patch = "\
diff --git a/logo.png b/logo.png
--- a/logo.png
+++ b/logo.png
Binary files a/logo.png and b/logo.png differ
";
        assert!(colors(&patches(patch)).is_empty());
    }

    #[test]
    fn a_conflicts_sides_do_not_run_into_each_other() {
        let parsed = patches(CONFLICTED);
        assert!(parsed[0].is_combined, "the fixture is a combined diff");
        let colors = colors(&parsed);
        // Rows: 0 ` fn main() {`, 1 `<<<<<<<`, 2 `/* ours`, 3 `=======`,
        // 4 `let x = 2;`, 5 `>>>>>>>`, 6 `}`.
        for row in [1, 3, 5] {
            let fence = colors.line(0, 0, row);
            assert!(fence.fence, "row {row} is one of git's fences");
            assert!(fence.spans.is_empty(), "and it is not read as code");
        }
        assert!(
            !colors.line(0, 0, 4).fence,
            "the side itself is not a fence"
        );
        let theirs = colors.line(0, 0, 4).spans.clone();
        assert!(
            theirs.len() > 1,
            "`let x = 2;` is code, not the inside of the comment `ours` opened: {theirs:?}"
        );
    }

    #[test]
    fn the_line_after_a_conflict_goes_on_in_ours() {
        // `ours` opens a comment and leaves it open, `theirs` does not.
        // What follows the region belongs to the branch being worked on,
        // so the closing brace is inside a comment here — one run, the
        // comment's colour, and the same colour the `/* ours` line got.
        let colors = colors(&patches(CONFLICTED));
        let opened = colors.line(0, 0, 2);
        let after = colors.line(0, 0, 6);
        assert_eq!(
            after.spans.len(),
            1,
            "still inside what `ours` opened: {after:?}"
        );
        assert_eq!(
            after.spans.first().map(|s| s.color),
            opened.spans.last().map(|s| s.color)
        );
    }

    #[test]
    fn a_conflict_in_a_language_nobody_knows_still_has_its_fences() {
        let patch = "\
diff --cc notes.qqq
--- a/notes.qqq
+++ b/notes.qqq
@@@ -1,1 -1,1 +1,5 @@@
++<<<<<<< HEAD
++ours
++=======
++theirs
++>>>>>>> other
";
        let colors = colors(&patches(patch));
        assert!(
            colors.line(0, 0, 0).fence,
            "the fence is git's, not the file's"
        );
        assert!(colors.line(0, 0, 2).fence);
        assert!(colors.line(0, 0, 4).fence);
        assert!(
            !colors.line(0, 0, 1).fence,
            "and what is between them is not"
        );
        assert!(
            colors.line(0, 0, 1).spans.is_empty(),
            "nothing was found to colour it with"
        );
    }

    #[test]
    fn markers_outside_a_conflict_are_ordinary_text() {
        // The same shapes in a unified diff — a file that merely writes
        // about conflicts. Nothing here is structure.
        let patch = "\
diff --git a/README.md b/README.md
--- a/README.md
+++ b/README.md
@@ -1,3 +1,3 @@
 Resolve it by hand:
-<<<<<<< HEAD
+<<<<<<< main
 =======
";
        let parsed = patches(patch);
        assert!(!parsed[0].is_combined);
        let colors = colors(&parsed);
        let line = colors.line(0, 0, 1);
        assert!(!line.fence, "nothing here is a fence");
        assert!(
            !line.spans.is_empty(),
            "read as text, so it has the colour text has"
        );
    }
}
