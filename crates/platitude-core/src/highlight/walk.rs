//! Where the reading of one hunk has got to, and how a conflict's
//! two sides are stood beside each other rather than after one
//! another.

use syntect::highlighting::{HighlightIterator, HighlightState, Highlighter};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference};

use crate::parse::diff::DiffLineKind;

use super::theme::Assets;
use super::{LineColors, Rgb, Span};

/// Where the reading of a hunk has got to.
pub(super) struct Walk<'a> {
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
#[derive(Clone)]
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
    pub(super) fn new(
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

    /// A reading that starts where this one stands — what a hunk's rows
    /// are coloured against while the file's own walk carries on.
    pub(super) fn fork(&self) -> Walk<'a> {
        Walk {
            assets: self.assets,
            highlighter: self.highlighter,
            state: self.state.clone(),
            combined: self.combined,
            region: self.region.clone(),
        }
    }

    pub(super) fn read(&mut self, line: &crate::parse::diff::DiffLine) -> LineColors {
        // `\ No newline at end of file` is git talking, not the file.
        if line.kind == DiffLineKind::NoNewline {
            return LineColors::nothing();
        }
        if self.combined
            && let Some(marker) = conflict_marker(&line.text)
        {
            self.cross(marker);
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

    pub(super) fn paint(&mut self, text: &str) -> Vec<Span> {
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
    use super::super::colors;
    use super::super::testkit::patches;
    use super::*;
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
    fn a_conflicts_sides_do_not_run_into_each_other() {
        let parsed = patches(CONFLICTED);
        assert!(parsed[0].is_combined, "the fixture is a combined diff");
        let colors = colors(&parsed, None);
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
        let colors = colors(&patches(CONFLICTED), None);
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
        let colors = colors(&patches(patch), None);
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
        let colors = colors(&parsed, None);
        let line = colors.line(0, 0, 1);
        assert!(!line.fence, "nothing here is a fence");
        assert!(
            !line.spans.is_empty(),
            "read as text, so it has the colour text has"
        );
    }
}
