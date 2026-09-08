//! How far a diff reaches sideways: the lines the pane measures to find out.

use platitude_core::highlight::DiffColors;
use platitude_core::parse::diff::{DiffLine, DiffLineKind, FilePatch};

/// Columns a tab stands for (デザイン規約 §シンタックスハイライト).
pub(super) const TAB_WIDTH: usize = 4;

/// How many lines the pane is handed. Every one of them costs a text
/// layout when a file is opened, so this is how far ahead of the rows the
/// pane is willing to read.
const MOST_CANDIDATES: usize = 8;

/// The lines worth measuring before the rows that hold them are laid out,
/// each as `<length>:<bold><markup>` with the length in UTF-16 units, one
/// record after another. Empty where there is nothing to send.
///
/// What it is for: the pane draws the code without eliding it and lets the
/// reader send it sideways, so it wants to know how far sideways there is
/// to go before the reader has scrolled through the file
/// (デザイン規約 §diff を横へ送る). **Lines and not a count of columns,
/// because a column is not a width and no arithmetic here can make it
/// one**: a wide glyph is counted as two columns, and the fallback a
/// Latin-only mono family hands it to is not monospaced at all — measured
/// on Windows at `fontCode`, against 8px for the font's own columns:
/// `日` 13, `の` 11, `。` 9, `「` 7, an emoji 18. And it cuts the other
/// way as often: a combining mark is a character this walk counts and a
/// glyph the font draws nothing extra for, so `e`+U+0301 four hundred
/// times counts 800 columns and is drawn in 400. What leaves here is the
/// text to measure.
///
/// **A head start, not the answer.** Which line is drawn furthest cannot
/// be known here, so this hands over the longest few by column count and
/// nothing is decided by leaving a line out: what settles the reach is the
/// width of the rows as they are laid out (`DiffReach`), and a line no
/// record named is measured when its row reaches the screen. Ranking by
/// columns is a guess; dropping a line on one was a bug — a line beaten
/// on both counts a walk of columns knows about (`0` seven hundred times
/// against those four hundred combining pairs) is drawn 2,400px further.
///
/// The text is the row's own (`markup::styled`): escaped, its tabs spelled
/// as the spaces the row draws them as, and read in one format. Handing
/// over the source line instead measured a row's `<b>` as markup and its
/// tabs at Qt's own stop.
///
/// Hunk headings are not among them: they stand at the viewport's own left
/// edge and never travel, so a long `@@` line is not something to scroll
/// to the end of.
pub fn widest_lines(patches: &[FilePatch], colors: &DiffColors) -> String {
    let mut lines: Vec<(usize, usize, usize, usize)> = Vec::new();
    for (patch_index, patch) in patches.iter().enumerate() {
        for (hunk_index, hunk) in patch.hunks.iter().enumerate() {
            for (line_index, line) in hunk.lines.iter().enumerate() {
                // Every line here is a row on screen, and only those:
                // git's no-newline note is not one — the rows carry it as
                // a mark at the end of the line it is about
                // (`encode::rows`), so its own words are never drawn and
                // are not something to reach the end of.
                if line.kind == DiffLineKind::NoNewline {
                    continue;
                }
                lines.push((columns_of(&line.text), patch_index, hunk_index, line_index));
            }
        }
    }
    lines.sort_unstable_by(|a, b| b.cmp(a));
    lines.truncate(MOST_CANDIDATES);
    let mut out = String::new();
    for (_, patch_index, hunk_index, line_index) in lines {
        let line = &patches[patch_index].hunks[hunk_index].lines[line_index];
        let read = colors.line(patch_index, hunk_index, line_index);
        push_candidate(
            &mut out,
            line,
            &super::markup::styled(&line.text, &read.spans),
        );
    }
    out
}

/// One line as the pane will measure it: how long the markup is, whether
/// it is drawn bold, and the markup itself.
fn push_candidate(out: &mut String, line: &DiffLine, markup: &str) {
    // Counted the way the pane counts, so a line carrying anything the
    // records are parted by — or an astral glyph, which QML holds as two
    // — is still cut out whole (`DiffTextMetrics.reachRecords`).
    out.push_str(&markup.encode_utf16().count().to_string());
    out.push(':');
    out.push(match line.kind {
        // The two the rows set in bold (`DiffRowDelegate`). A conflict
        // fence is the exception there and not here: git's `<<<<<<<` is
        // seven characters and reaches past nothing.
        DiffLineKind::Addition | DiffLineKind::Deletion => '1',
        _ => '0',
    });
    out.push_str(markup);
}

/// How many columns one line stands in — the ranking above, and nothing
/// else: it is a count of what a walk of the text can see, which is not
/// what the font draws (see [`widest_lines`]).
fn columns_of(text: &str) -> usize {
    let mut cols = 0usize;
    for ch in text.chars() {
        cols += step_of(ch, cols);
    }
    cols
}

/// How far one character carries a line that has already reached `col`: a
/// tab reaches the next stop, a glyph the East Asian blocks draw full
/// width takes two columns, everything else takes one.
///
/// The single rule for it, and it takes no stop of its own, because every
/// walk of a line has to arrive at the same columns: [`columns_of`] ranks
/// the lines worth measuring, `markup::push_escaped` spells a tab as that
/// many `&nbsp;`, and `markup::spelled_ranges` counts the places those
/// `&nbsp;` stand in. A tab spelled by one rule and counted by another puts
/// every place after it out by the difference.
pub(super) fn step_of(ch: char, col: usize) -> usize {
    if ch == '\t' {
        TAB_WIDTH - (col % TAB_WIDTH)
    } else {
        1 + usize::from(is_wide(ch))
    }
}

/// Whether the glyph is one a mono font draws two columns wide. The ranges
/// are the East Asian Wide and Fullwidth blocks plus the emoji that share
/// their advance — read off Unicode's own table rather than derived, so
/// the list is what it is.
///
/// Only [`step_of`] asks, and only so that a line reaches the same tab
/// stops everywhere it is walked. **Nothing turns this into pixels any
/// more**: where a glyph is drawn is a question for the row's own layout
/// (`LineRuler`), which needs no count of anything.
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

    /// The records read back the way `DiffTextMetrics.reachRecords` reads
    /// them: each one's length in UTF-16 units cuts out its own text, so a
    /// line carrying a digit, a colon or anything else the packing spells
    /// with is still handed over whole.
    fn records(packed: &str) -> Vec<(bool, String)> {
        let units: Vec<u16> = packed.encode_utf16().collect();
        let mut out = Vec::new();
        let mut at = 0;
        while at < units.len() {
            let cut = at
                + units[at..]
                    .iter()
                    .position(|&u| u == u16::from(b':'))
                    .expect("every record says how long it is");
            let len: usize = String::from_utf16_lossy(&units[at..cut])
                .parse()
                .expect("that length is a number");
            let bold = units[cut + 1] == u16::from(b'1');
            let from = cut + 2;
            out.push((bold, String::from_utf16_lossy(&units[from..from + len])));
            at = from + len;
        }
        out
    }

    /// The candidates, and the rows they are supposed to be — read out of
    /// one patch so a difference between them fails rather than passing
    /// unnoticed.
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
        // Longest first, and the heading is not among them: it stands at
        // the pane's own left edge and never travels.
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
        // The one on top counts 800 columns and is drawn in 400 — a
        // combining mark is a character this walk counts and a glyph the
        // font draws nothing extra for. The one below counts 700 and is
        // drawn in 700. Ranking by columns puts them the wrong way round,
        // which is why nothing is decided by the ranking: **both go
        // over**, and what settles the reach is the rows (`DiffReach`).
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
        // The row and the ruler read one string in one format. Handing
        // over the source line instead had a `Label` reading `<b>` as
        // markup and a tab at Qt's own stop — two ways to measure a line
        // this pane never draws.
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
        // git says it in a line of its own and the rows carry it as a
        // mark on the line it is about (`encode::rows`), so its words are
        // drawn nowhere — and they are longer than anything else here,
        // which is what makes the difference visible.
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
    fn a_line_says_how_long_it_is_in_the_units_qml_counts_in() {
        // An astral glyph is one character here and two units there, and
        // the length is what cuts the record out — a count of characters
        // would take two units too few and hand the pane a line with its
        // tail cut off.
        let patch = "\
--- a/f
+++ b/f
@@ -1,1 +1,1 @@
+ab\u{1f600}
";
        let patches = parse_patch(patch.as_bytes());
        let packed = widest_lines(&patches, &DiffColors::default());
        assert_eq!(packed, "4:1ab\u{1f600}");
        assert_eq!(records(&packed), [(true, "ab\u{1f600}".to_string())]);
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
        // `step_of(..) == 2` is a different question from `is_wide`: a tab
        // reaches its stop in two columns as well.
        assert_eq!(step_of('\t', 2), 2);
        assert!(!is_wide('\t'));
        assert!(is_wide('日'));
        assert!(!is_wide('š'));
    }

    #[test]
    fn a_diff_with_no_lines_has_nowhere_sideways_to_go() {
        assert_eq!(widest_lines(&[], &DiffColors::default()), "");
        assert_eq!(
            widest_lines(
                &parse_patch(b"* Unmerged path gone.txt\n"),
                &DiffColors::default()
            ),
            ""
        );
    }
}
