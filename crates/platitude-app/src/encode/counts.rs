//! How far a branch stands from what it is measured against, as QML reads
//! the pair: one record, never two slots asked of one name.

use platitude_core::AheadBehind;
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, Record, field};

/// A count of commits in the `int` QML draws it as: one past the largest
/// is drawn as the largest.
pub fn shown_count(commits: u32) -> i32 {
    i32::try_from(commits).unwrap_or(i32::MAX)
}

/// `ahead` / `behind`, as [`shown_count`] draws each, and `level` where
/// both are 0 (`AheadBehind::is_level`) — what decides whether a pair is
/// drawn at all (`HeadTrack`), so no reader weighs the two numbers itself.
/// Read back from the counts alone.
impl Record for AheadBehind {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("ahead", &shown_count(self.ahead))
            .put("behind", &shown_count(self.behind))
            .put("level", &self.is_level())
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        Some(Self {
            ahead: u32::try_from(field::<i32>(map, "ahead")?).ok()?,
            behind: u32::try_from(field::<i32>(map, "behind")?).ok()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::One;
    use qtbridge::QVariantConvertible;

    #[test]
    fn a_pair_of_counts_round_trips() {
        let counts = One::new(AheadBehind {
            ahead: 3,
            behind: 12,
        });
        let back =
            <One<AheadBehind> as QVariantConvertible>::try_from_qvariant(&counts.to_qvariant());
        assert_eq!(back.map(|b| *b), Some(*counts));
    }

    #[test]
    fn a_pair_says_level_only_where_both_counts_are_zero() {
        let level_of =
            |ahead, behind| field::<bool>(&AheadBehind { ahead, behind }.to_map(), "level");
        assert_eq!(level_of(0, 0), Some(true));
        assert_eq!(level_of(1, 0), Some(false));
        assert_eq!(level_of(0, 1), Some(false));
        assert_eq!(level_of(2, 3), Some(false));
    }
}
