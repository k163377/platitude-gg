//! How far a diff reaches sideways: the lines the pane measures to find out.

use platitude_core::highlight::DiffColors;
use platitude_core::parse::diff::{DiffLine, DiffLineKind, FilePatch};
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, Listed, Record, field};

/// One line as the pane will measure it: the markup the row draws, and
/// whether the row draws it bold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub bold: bool,
    pub line: String,
}

/// The lines worth measuring, longest first by column count.
pub type Candidates = Listed<Candidate>;

impl Record for Candidate {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("bold", &self.bold)
            .put("line", &self.line)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            bold: field(map, "bold")?,
            line: field(map, "line")?,
        })
    }
}

impl platitude_core::mem::Footprint for Candidate {
    fn heap_bytes(&self) -> usize {
        self.line.heap_bytes()
    }
}

/// Columns a tab stands for (デザイン規約 §シンタックスハイライト).
pub(super) const TAB_WIDTH: usize = 4;

/// How many lines the pane is handed; each costs a text layout when a
/// file opens.
const MOST_CANDIDATES: usize = 8;

/// The lines worth measuring before the rows that hold them are laid out,
/// one [`Candidate`] each; empty where there is nothing to send
/// (デザイン規約 §diff を横へ送る, rules-refs/app-ui.md「diff は横へ送る・溝は動かない」).
///
/// **Lines, because a column is not a width**: the fallback a Latin-only
/// mono family hands a wide glyph to is not monospaced, and a combining
/// mark is a column counted and nothing drawn.
///
/// **A head start**: the column ranking is a guess, so nothing is decided
/// by leaving a line out — the rows' laid-out widths settle the reach
/// (`DiffReach`). The text is the row's own (`markup::styled`); the
/// source line would measure `<b>` as markup and tabs at Qt's own stop.
/// Hunk headings are left out: they never travel.
pub fn widest_lines(patches: &[FilePatch], colors: &DiffColors) -> Candidates {
    let mut lines: Vec<(usize, usize, usize, usize)> = Vec::new();
    for (patch_index, patch) in patches.iter().enumerate() {
        for (hunk_index, hunk) in patch.hunks.iter().enumerate() {
            for (line_index, line) in hunk.lines.iter().enumerate() {
                // Rows only: the no-newline note rides its line as a mark
                // (`encode::rows`), so its words are never drawn.
                if line.kind == DiffLineKind::NoNewline {
                    continue;
                }
                lines.push((columns_of(&line.text), patch_index, hunk_index, line_index));
            }
        }
    }
    lines.sort_unstable_by(|a, b| b.cmp(a));
    lines.truncate(MOST_CANDIDATES);
    Candidates::new(
        lines
            .into_iter()
            .map(|(_, patch_index, hunk_index, line_index)| {
                let line = &patches[patch_index].hunks[hunk_index].lines[line_index];
                let read = colors.line(patch_index, hunk_index, line_index);
                Candidate {
                    bold: is_bold(line),
                    line: super::markup::styled(&line.text, &read.spans),
                }
            })
            .collect(),
    )
}

/// The two kinds the rows set in bold (`DiffRowDelegate`). Its exception
/// for a conflict fence is left out: a fence reaches past nothing.
fn is_bold(line: &DiffLine) -> bool {
    matches!(line.kind, DiffLineKind::Addition | DiffLineKind::Deletion)
}

/// How many columns one line stands in — for the ranking only (see
/// [`widest_lines`]).
fn columns_of(text: &str) -> usize {
    let mut cols = 0usize;
    for ch in text.chars() {
        cols += step_of(ch, cols);
    }
    cols
}

/// How far one character carries a line that has already reached `col`: a
/// tab reaches the next stop, a wide glyph takes two columns, anything
/// else one.
///
/// The one rule every walk of a line steps by ([`columns_of`],
/// `markup::push_escaped`, `markup::spelled_ranges`): a tab spelled by one
/// rule and counted by another puts every place after it out by the
/// difference.
pub(super) fn step_of(ch: char, col: usize) -> usize {
    if ch == '\t' {
        TAB_WIDTH - (col % TAB_WIDTH)
    } else {
        1 + usize::from(is_wide(ch))
    }
}

/// Whether the glyph is East Asian Wide / Fullwidth, or an emoji sharing
/// that advance (off Unicode's table). Only [`step_of`] asks: **this stays
/// a count** — where a glyph is drawn is the row's own layout's question
/// (`LineRuler`).
pub(super) fn is_wide(ch: char) -> bool {
    matches!(u32::from(ch),
        0x1100..=0x115F
        | 0x2E80..=0x303E
        | 0x3041..=0x33FF
        | 0x3400..=0x4DBF
        | 0x4E00..=0x9FFF
        | 0xA000..=0xA4CF
        | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE6F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6
        | 0x1F300..=0x1F64F
        | 0x1F900..=0x1F9FF
        | 0x20000..=0x3FFFD)
}

#[cfg(test)]
mod tests {
    use super::*;
    use platitude_core::highlight::DiffColors;
    use platitude_core::parse::diff::parse_patch;
    use qtbridge::QVariantConvertible;

    fn records(candidates: &Candidates) -> Vec<(bool, String)> {
        candidates
            .iter()
            .map(|c| (c.bold, c.line.clone()))
            .collect()
    }

    /// The candidates and the rows, read out of one patch.
    fn picked(patch: &str) -> (Vec<(bool, String)>, Vec<String>) {
        let patches = parse_patch(patch.as_bytes());
        let colors = DiffColors::default();
        let rows = super::super::rows::flatten_patches(&patches, true, &colors, None)
            .into_iter()
            .map(|r| r.text)
            .collect();
        (records(&widest_lines(&patches, &colors)), rows)
    }

    #[test]
    fn the_longest_lines_go_over_to_be_measured() {
        let patch = "\
--- a/f
+++ b/f
@@ -1,3 +1,3 @@ a heading long enough to beat every line below it
 short
-mid line
+the longest line in this patch
";
        let (picked, _) = picked(patch);
        // Longest first; the heading never travels, so it is not among them.
        assert_eq!(
            picked,
            [
                (
                    true,
                    "the&nbsp;longest&nbsp;line&nbsp;in&nbsp;this&nbsp;patch".to_string()
                ),
                (true, "mid&nbsp;line".to_string()),
                (false, "short".to_string()),
            ]
        );
    }

    #[test]
    fn a_line_is_not_dropped_for_standing_in_fewer_columns() {
        // The top line counts 6 columns and is drawn in 3 (combining
        // marks); the one below counts 4 and is drawn in 4. The ranking has
        // them the wrong way round, so it decides nothing: both go over.
        let patch = "\
--- a/f
+++ b/f
@@ -1,2 +1,2 @@
-e\u{301}e\u{301}e\u{301}
+0000
";
        let (picked, _) = picked(patch);
        assert_eq!(
            picked,
            [
                (true, "e\u{301}e\u{301}e\u{301}".to_string()),
                (true, "0000".to_string())
            ]
        );
    }

    #[test]
    fn what_goes_over_is_what_the_row_draws() {
        // The row and the ruler read one string in one format.
        let patch = "\
--- a/f
+++ b/f
@@ -1,2 +1,2 @@
-<b>x</b>
+a\tb
";
        let (picked, rows) = picked(patch);
        let said: Vec<String> = picked.into_iter().map(|(_, text)| text).collect();
        assert_eq!(said, ["&lt;b&gt;x&lt;/b&gt;", "a&nbsp;&nbsp;&nbsp;b"]);
        for line in &said {
            assert!(rows.contains(line), "{line} is not a row: {rows:?}");
        }
    }

    #[test]
    fn the_note_about_a_missing_newline_is_not_a_line_to_reach_the_end_of() {
        // Its words are drawn nowhere (`encode::rows`), and are longer than
        // anything else here, so a slip shows.
        let patch = "\
--- a/f
+++ b/f
@@ -1,1 +1,1 @@
-short
\\ No newline at end of file
+ok
";
        let (picked, _) = picked(patch);
        assert_eq!(
            picked,
            [(true, "short".to_string()), (true, "ok".to_string())]
        );
    }

    #[test]
    fn a_line_goes_over_whole_whatever_it_holds() {
        let patch = "\
--- a/f
+++ b/f
@@ -1,1 +1,1 @@
+4:1ab\u{1f600}
";
        let patches = parse_patch(patch.as_bytes());
        let candidates = widest_lines(&patches, &DiffColors::default());
        assert_eq!(records(&candidates), [(true, "4:1ab\u{1f600}".to_string())]);
        assert_eq!(
            <Candidates as QVariantConvertible>::try_from_qvariant(&candidates.to_qvariant()),
            Ok(candidates)
        );
    }

    #[test]
    fn a_tab_reaches_its_stop_and_a_wide_glyph_takes_two_columns() {
        assert_eq!(step_of('日', 0), 2);
        assert_eq!(columns_of("日本語版"), 8);
        assert_eq!(columns_of("\t"), TAB_WIDTH);
        assert_eq!(columns_of("123\t"), TAB_WIDTH);
        assert_eq!(columns_of("1234\t"), 2 * TAB_WIDTH);
        // 日本語 stands on 0..6, so two columns are all the tab has left
        // to reach the stop at 8.
        assert_eq!(columns_of("日本語\t"), 2 * TAB_WIDTH);
    }

    #[test]
    fn a_tab_is_two_columns_wide_without_being_a_wide_glyph() {
        assert_eq!(step_of('\t', 2), 2);
        assert!(!is_wide('\t'));
        assert!(is_wide('日'));
        assert!(!is_wide('š'));
    }

    #[test]
    fn a_diff_with_no_lines_has_nowhere_sideways_to_go() {
        assert!(widest_lines(&[], &DiffColors::default()).is_empty());
        assert!(
            widest_lines(
                &parse_patch(b"* Unmerged path gone.txt\n"),
                &DiffColors::default()
            )
            .is_empty()
        );
    }
}
