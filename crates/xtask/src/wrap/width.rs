//! How wide a character stands, and how a line end between two drawn
//! characters is drawn.
//!
//! Width is the column count of Unicode 16.0's East Asian Width: F and W
//! take two columns, A (`→`, `—`, `§`, `①`) one, as a terminal or an
//! editor in a Western locale draws it.
//!
//! A line end is read by the segment break rules the CSS Text drafts
//! wrote out (Level 4 leaves them to the browser; its source keeps them),
//! in their two plain clauses: removed next to U+200B, removed between
//! two F/W/H characters neither of which is Hangul. Every other line end
//! is taken to draw as a space — the draft's further clauses (an A
//! punctuation mark beside F/W in Chinese or Japanese text, emoji read as
//! A) are not followed, so such a line end is joined as a space and a
//! space there may end a line; both draw a space wherever the plain
//! clauses are the browser's.

/// Ranges of F and W (Unicode 16.0).
const WIDE: &[(u32, u32)] = &[
    (0x1100, 0x115F),
    (0x231A, 0x231B),
    (0x2329, 0x232A),
    (0x23E9, 0x23EC),
    (0x23F0, 0x23F0),
    (0x23F3, 0x23F3),
    (0x25FD, 0x25FE),
    (0x2614, 0x2615),
    (0x2630, 0x2637),
    (0x2648, 0x2653),
    (0x267F, 0x267F),
    (0x268A, 0x268F),
    (0x2693, 0x2693),
    (0x26A1, 0x26A1),
    (0x26AA, 0x26AB),
    (0x26BD, 0x26BE),
    (0x26C4, 0x26C5),
    (0x26CE, 0x26CE),
    (0x26D4, 0x26D4),
    (0x26EA, 0x26EA),
    (0x26F2, 0x26F3),
    (0x26F5, 0x26F5),
    (0x26FA, 0x26FA),
    (0x26FD, 0x26FD),
    (0x2705, 0x2705),
    (0x270A, 0x270B),
    (0x2728, 0x2728),
    (0x274C, 0x274C),
    (0x274E, 0x274E),
    (0x2753, 0x2755),
    (0x2757, 0x2757),
    (0x2795, 0x2797),
    (0x27B0, 0x27B0),
    (0x27BF, 0x27BF),
    (0x2B1B, 0x2B1C),
    (0x2B50, 0x2B50),
    (0x2B55, 0x2B55),
    (0x2E80, 0x303E),
    (0x3041, 0x3247),
    (0x3250, 0x33FF),
    (0x3400, 0x9FFF),
    (0xA000, 0xA4CF),
    (0xA960, 0xA97F),
    (0xAC00, 0xD7A3),
    (0xF900, 0xFAFF),
    (0xFE10, 0xFE19),
    (0xFE30, 0xFE6F),
    (0xFF00, 0xFF60),
    (0xFFE0, 0xFFE6),
    (0x16FE0, 0x16FE4),
    (0x16FF0, 0x16FF1),
    (0x17000, 0x187F7),
    (0x18800, 0x18CD5),
    (0x18CFF, 0x18D08),
    (0x1AFF0, 0x1AFFE),
    (0x1B000, 0x1B122),
    (0x1B132, 0x1B132),
    (0x1B150, 0x1B152),
    (0x1B155, 0x1B155),
    (0x1B164, 0x1B167),
    (0x1B170, 0x1B2FB),
    (0x1D300, 0x1D376),
    (0x1F004, 0x1F004),
    (0x1F0CF, 0x1F0CF),
    (0x1F18E, 0x1F18E),
    (0x1F191, 0x1F19A),
    (0x1F200, 0x1F251),
    (0x1F260, 0x1F265),
    (0x1F300, 0x1F320),
    (0x1F32D, 0x1F335),
    (0x1F337, 0x1F37C),
    (0x1F37E, 0x1F393),
    (0x1F3A0, 0x1F3CA),
    (0x1F3CF, 0x1F3D3),
    (0x1F3E0, 0x1F3F0),
    (0x1F3F4, 0x1F3F4),
    (0x1F3F8, 0x1F43E),
    (0x1F440, 0x1F440),
    (0x1F442, 0x1F4FC),
    (0x1F4FF, 0x1F53D),
    (0x1F54B, 0x1F54E),
    (0x1F550, 0x1F567),
    (0x1F57A, 0x1F57A),
    (0x1F595, 0x1F596),
    (0x1F5A4, 0x1F5A4),
    (0x1F5FB, 0x1F64F),
    (0x1F680, 0x1F6C5),
    (0x1F6CC, 0x1F6CC),
    (0x1F6D0, 0x1F6D2),
    (0x1F6D5, 0x1F6DF),
    (0x1F6EB, 0x1F6EF),
    (0x1F6F4, 0x1F6FF),
    (0x1F7E0, 0x1F7EB),
    (0x1F7F0, 0x1F7F0),
    (0x1F90C, 0x1F93A),
    (0x1F93C, 0x1F945),
    (0x1F947, 0x1F9FF),
    (0x1FA70, 0x1FAFF),
    (0x20000, 0x3FFFD),
];

/// Ranges of H, which CSS joins like F and W but which stand one column.
const HALF: &[(u32, u32)] = &[(0x20A9, 0x20A9), (0xFF61, 0xFFDC), (0xFFE8, 0xFFEE)];

/// Hangul, which CSS keeps the space for: its words are spaced.
const HANGUL: &[(u32, u32)] = &[
    (0x1100, 0x11FF),
    (0x3130, 0x318F),
    (0xA960, 0xA97F),
    (0xAC00, 0xD7FF),
    (0xFFA0, 0xFFDC),
];

/// Combining marks and the invisible joiners, which take no column.
const ZERO: &[(u32, u32)] = &[
    (0x0300, 0x036F),
    (0x200B, 0x200F),
    (0x2060, 0x2064),
    (0x20D0, 0x20FF),
    (0x3099, 0x309A),
    (0xFE00, 0xFE0F),
];

/// A line never starts on these (JIS X 4051's closing brackets, stops,
/// middle dots, small kana, the prolonged sound mark).
const NO_BREAK_BEFORE: &str = "、。，．・：；？！ー）」』】〕〉》］｝〙〗〟々〻ゝゞヽヾ\
                               ぁぃぅぇぉっゃゅょゎゕゖァィゥェォッャュョヮヵヶ〜～゛゜";

/// A line never ends on these (the opening brackets, and the middle dot
/// that binds both of its neighbours).
const NO_BREAK_AFTER: &str = "（「『【〔〈《［｛〘〖〝・";

fn within(ranges: &[(u32, u32)], c: char) -> bool {
    let code = u32::from(c);
    ranges
        .binary_search_by(|&(low, high)| {
            if high < code {
                std::cmp::Ordering::Less
            } else if low > code {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

pub(super) fn columns(c: char) -> usize {
    if within(ZERO, c) {
        0
    } else if within(WIDE, c) {
        2
    } else {
        1
    }
}

pub(super) fn text_columns(text: &str) -> usize {
    text.chars().map(columns).sum()
}

const ZERO_WIDTH_SPACE: char = '\u{200B}';

/// F, W or H, and not Hangul: a line end between two of these is removed.
/// The combining voiced sound marks (U+3099, U+309A) are W, so a kana
/// written with one is as tight as the composed one.
fn tight(c: char) -> bool {
    (within(WIDE, c) || within(HALF, c)) && !within(HANGUL, c)
}

/// Whether a line end between these two drawn characters is removed
/// rather than drawn as a space.
pub(super) fn joins_tight(before: char, after: char) -> bool {
    before == ZERO_WIDTH_SPACE || after == ZERO_WIDTH_SPACE || (tight(before) && tight(after))
}

/// Whether a line may end between `before` and `after`, two characters
/// with nothing between them in the source. Never before a combining
/// mark, which belongs to the character before it.
pub(super) fn breaks_between(before: char, after: char) -> bool {
    tight(before)
        && tight(after)
        && !within(ZERO, after)
        && !NO_BREAK_AFTER.contains(before)
        && !NO_BREAK_BEFORE.contains(after)
}

#[cfg(test)]
mod tests {
    use super::{
        HALF, HANGUL, WIDE, ZERO, breaks_between, columns, joins_tight, text_columns, tight,
    };

    /// `binary_search_by` reads the tables as sorted and disjoint.
    #[test]
    fn every_table_is_sorted_and_disjoint() {
        for table in [WIDE, HALF, HANGUL, ZERO] {
            for pair in table.windows(2) {
                assert!(pair[0].1 < pair[1].0, "{pair:?}");
            }
            for &(low, high) in table {
                assert!(low <= high, "{low:#x}..{high:#x}");
            }
        }
    }

    #[test]
    fn japanese_stands_two_columns_and_ambiguous_signs_one() {
        assert_eq!(text_columns("日本語、。「」（）"), 18);
        assert_eq!(text_columns("ひらがなカタカナ"), 16);
        assert_eq!(text_columns("→—§①×…"), 6);
        assert_eq!(text_columns("✅❌"), 4);
        assert_eq!(columns('a'), 1);
        assert_eq!(columns('\u{FE0F}'), 0);
    }

    #[test]
    fn a_line_may_end_between_two_japanese_characters() {
        assert!(breaks_between('日', '本'));
        assert!(breaks_between('る', 'カ'));
        assert!(breaks_between('。', '次'));
        assert!(!breaks_between('a', '日'));
        assert!(!breaks_between('日', 'a'));
        assert!(!breaks_between('한', '국'));
    }

    #[test]
    fn a_line_neither_opens_on_a_stop_nor_closes_on_a_bracket() {
        assert!(!breaks_between('本', '、'));
        assert!(!breaks_between('本', '。'));
        assert!(!breaks_between('本', '」'));
        assert!(!breaks_between('本', 'ー'));
        assert!(!breaks_between('本', 'ょ'));
        assert!(!breaks_between('「', '本'));
        assert!(!breaks_between('（', '本'));
        assert!(!breaks_between('本', '・'));
        assert!(!breaks_between('・', '本'));
    }

    #[test]
    fn halfwidth_kana_joins_tight_at_one_column() {
        assert!(tight('ｱ'));
        assert_eq!(columns('ｱ'), 1);
        assert!(!tight('→'));
        assert!(!tight('ﾡ'), "halfwidth Hangul keeps its space");
    }

    /// Unicode 16.0's East Asian Width, at the edges the table is cut at.
    #[test]
    fn the_table_follows_unicode_16() {
        assert_eq!(columns('☰'), 2);
        assert_eq!(columns('䷀'), 2);
        assert_eq!(columns('\u{3248}'), 1, "circled ten on black square is A");
        assert_eq!(columns('🛠'), 1);
        assert_eq!(columns('🖥'), 1);
        assert_eq!(columns('🔧'), 2);
        assert_eq!(columns('🚀'), 2);
        assert_eq!(columns('\u{20A9}'), 1);
        assert!(tight('\u{20A9}'));
    }

    /// `が` written as `か` and U+3099: as tight as the composed one, and
    /// never split from its mark.
    #[test]
    fn a_combining_voiced_mark_joins_tight_and_stays_with_its_kana() {
        assert!(joins_tight('\u{3099}', 'で'));
        assert!(!breaks_between('か', '\u{3099}'));
        assert!(breaks_between('\u{3099}', 'で'));
    }

    #[test]
    fn a_line_end_by_a_zero_width_space_is_removed() {
        assert!(joins_tight('a', '\u{200B}'));
        assert!(joins_tight('\u{200B}', 'b'));
        assert!(joins_tight('日', '本'));
        assert!(!joins_tight('日', 'a'));
    }
}
