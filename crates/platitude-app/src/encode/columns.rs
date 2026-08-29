//! How wide a diff is, in columns of the mono font it is set in.

use platitude_core::parse::diff::FilePatch;

/// Where the tab stops of a line of code sit, in columns.
const TAB_COLUMNS: usize = 8;

/// How many columns of the mono font the longest line of the diff needs.
///
/// What it is for: the pane draws the code without eliding it and lets the
/// reader send it sideways, so it has to know how far sideways there is to
/// go before a single one of those rows exists (デザイン規約 §diff を横へ
/// 送る). Columns rather than pixels — the pane owns the font, and one
/// measured character is all it takes to turn these into a width.
///
/// Hunk headings are not counted: they stand at the viewport's own left
/// edge and never travel, so a long `@@` line is not something to scroll
/// to the end of.
pub fn widest_columns(patches: &[FilePatch]) -> i32 {
    let widest = patches
        .iter()
        .flat_map(|p| p.hunks.iter())
        .flat_map(|h| h.lines.iter())
        .map(|l| columns_of(&l.text))
        .max()
        .unwrap_or(0);
    i32::try_from(widest).unwrap_or(i32::MAX)
}

/// Whether any line of the diff carries a glyph the mono font draws two
/// columns wide.
///
/// What it is for: the pane needs what such a glyph advances *past* the
/// two columns [`step_of`] counts it as (`DiffTextMetrics.wideDelta`),
/// and the only way to have that number is to set one and measure it —
/// which, on a Latin-only mono family, hands the glyph to whatever
/// fallback the system has and **loads that font: tens of megabytes of
/// working set, in every window, repository open or not** (measured).
/// The number it buys is multiplied by a count of wide glyphs at every
/// place it is used — the wash's x and width in `DiffRowDelegate`, the
/// stand in [`super::hit_byte`] — so where a diff has none of them it is
/// multiplied by zero. This is what the pane asks before it builds the
/// ruler.
///
/// A walk of its own rather than a second answer out of
/// [`widest_columns`]: both are once per file opened, over text the
/// encoder has already walked, and the two questions read better apart
/// than as a pair nobody unpacks.
pub fn has_wide(patches: &[FilePatch]) -> bool {
    patches
        .iter()
        .flat_map(|p| p.hunks.iter())
        .flat_map(|h| h.lines.iter())
        .any(|l| any_wide(&l.text))
}

/// Whether one piece of text carries such a glyph — [`has_wide`] for a
/// line that arrives on its own, which is how the command log's rows
/// come (`models::commands`).
pub fn any_wide(text: &str) -> bool {
    text.chars().any(is_wide)
}

/// How many columns one line takes, walked a character at a time by
/// [`step_of`]. An approximation on purpose — it decides how far the
/// reader may send the text, and being a column out at the end of the
/// longest line in a file costs nothing that eliding it cost.
fn columns_of(text: &str) -> usize {
    let mut cols = 0usize;
    for ch in text.chars() {
        cols += step_of(ch, cols, TAB_COLUMNS);
    }
    cols
}

/// How far one character carries a line that has already reached `col`: a
/// tab reaches the next stop, a glyph the East Asian blocks draw full
/// width takes two columns, everything else takes one.
///
/// The single rule for it, because three walks of a line have to arrive
/// at the same columns: [`columns_of`] measures how far sideways the pane
/// may send the text, `markup::push_escaped` spells a tab as that many
/// `&nbsp;`, and `markup::display_ranges` re-walks the row to lay the
/// emphasis wash on the columns those `&nbsp;` and glyphs land on. A wash
/// walked by any other rule would sit beside the characters it names
/// rather than on them.
///
/// Only the stop is the caller's to choose — the pane draws at
/// `markup::TAB_WIDTH` and measures at [`TAB_COLUMNS`] — so `tab_width`
/// comes in as an argument, and nothing else here does.
pub(super) fn step_of(ch: char, col: usize, tab_width: usize) -> usize {
    if ch == '\t' {
        tab_width - (col % tab_width)
    } else {
        1 + usize::from(is_wide(ch))
    }
}

/// Whether the glyph is one a mono font draws two columns wide. The ranges
/// are the East Asian Wide and Fullwidth blocks plus the emoji that share
/// their advance — read off Unicode's own table rather than derived, so
/// the list is what it is.
///
/// `markup::display_ranges` asks this as well as [`step_of`], because two
/// columns is not two of the mono font's advances: a Latin-only mono
/// family hands these glyphs to a fallback that advances one em, so the
/// pane has to know how many of them a run stands on before it can put the
/// wash in pixels. `step_of(..) == 2` is a different question — a tab
/// reaches its stop in two columns as well.
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
    use platitude_core::parse::diff::parse_patch;

    #[test]
    fn the_widest_line_is_the_one_the_reader_can_send_furthest() {
        let patch = "\
--- a/f
+++ b/f
@@ -1,3 +1,3 @@ a heading long enough to beat every line below it
 short
-mid line
+the longest line in this patch
";
        let widest = widest_columns(&parse_patch(patch.as_bytes()));
        assert_eq!(widest, "the longest line in this patch".len() as i32);
    }

    #[test]
    fn a_tab_reaches_its_stop_and_a_wide_glyph_takes_two_columns() {
        // One tab from column 0 lands on 8, so the line is 8 + 2 = 10;
        // the Japanese one is 4 glyphs at two columns each.
        let patch = "\
--- a/f
+++ b/f
@@ -1,2 +1,2 @@
-\tab
+日本語版
";
        assert_eq!(widest_columns(&parse_patch(patch.as_bytes())), 10);
        assert_eq!(columns_of("日本語版"), 8);
        assert_eq!(columns_of("\t"), TAB_COLUMNS);
        assert_eq!(columns_of("1234567\t"), TAB_COLUMNS);
        assert_eq!(columns_of("12345678\t"), 2 * TAB_COLUMNS);
    }

    #[test]
    fn the_columns_a_wide_glyph_took_are_columns_the_tab_need_not_walk() {
        // 日本語 stands on 0..6, so one tab is enough to reach the stop
        // at 8 — the same three glyphs counted as one each would send it
        // to 8 with five columns to spare.
        assert_eq!(step_of('日', 0, TAB_COLUMNS), 2);
        assert_eq!(columns_of("日本語\t"), TAB_COLUMNS);
        assert_eq!(columns_of("日本語版\t"), 2 * TAB_COLUMNS);
    }

    #[test]
    fn only_a_diff_with_a_wide_glyph_in_it_asks_for_the_ruler() {
        let wide = "\
--- a/f
+++ b/f
@@ -1,2 +1,2 @@
-ascii only
+日本語版
";
        // The heading is not a line of code, and neither is the file
        // header: a wide glyph up there stands at the viewport's own edge
        // and no wash is ever laid on it.
        let heading_only = "\
--- a/f
+++ b/f
@@ -1,1 +1,1 @@ 日本語版
 ascii only
";
        assert!(has_wide(&parse_patch(wide.as_bytes())));
        assert!(!has_wide(&parse_patch(heading_only.as_bytes())));
        assert!(!has_wide(&[]));
    }

    #[test]
    fn a_tab_is_two_columns_wide_without_being_a_wide_glyph() {
        // `step_of(..) == 2` is a different question: what the ruler
        // measures is the fallback's advance, and a tab has none.
        assert_eq!(step_of('\t', 6, TAB_COLUMNS), 2);
        assert!(!any_wide("\tab"));
        assert!(any_wide("日"));
        assert!(!any_wide("Tomášek"));
        assert!(!any_wide(""));
    }

    #[test]
    fn a_diff_with_no_lines_has_nowhere_sideways_to_go() {
        assert_eq!(widest_columns(&[]), 0);
        assert_eq!(
            widest_columns(&parse_patch(b"* Unmerged path gone.txt\n")),
            0
        );
    }
}
