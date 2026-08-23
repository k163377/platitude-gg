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

/// How many columns one line takes: a tab reaches the next stop, a glyph
/// the East Asian blocks draw full width takes two, everything else takes
/// one. An approximation on purpose — it decides how far the reader may
/// send the text, and being a column out at the end of the longest line in
/// a file costs nothing that eliding it cost.
fn columns_of(text: &str) -> usize {
    let mut cols = 0usize;
    for ch in text.chars() {
        cols = if ch == '\t' {
            (cols / TAB_COLUMNS + 1) * TAB_COLUMNS
        } else {
            cols + usize::from(is_wide(ch)) + 1
        };
    }
    cols
}

/// Whether the glyph is one a mono font draws two columns wide. The ranges
/// are the East Asian Wide and Fullwidth blocks plus the emoji that share
/// their advance — read off Unicode's own table rather than derived, so
/// the list is what it is. Shared with `markup::display_ranges`, which
/// places the emphasis wash under the same glyphs.
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
    fn a_diff_with_no_lines_has_nowhere_sideways_to_go() {
        assert_eq!(widest_columns(&[]), 0);
        assert_eq!(
            widest_columns(&parse_patch(b"* Unmerged path gone.txt\n")),
            0
        );
    }
}
