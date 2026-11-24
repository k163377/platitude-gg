//! How far a branch stands from what it is measured against, as QML reads
//! the pair: one record, never two slots asked of one name.

use platitude_core::AheadBehind;
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, Record, field};

/// `ahead` / `behind`, in the `int` QML draws them as — saturating, as the
/// rows' roles carry them.
impl Record for AheadBehind {
    fn to_map(&self) -> QVariantMap {
        let int = |commits: u32| i32::try_from(commits).unwrap_or(i32::MAX);
        Fields::new()
            .put("ahead", &int(self.ahead))
            .put("behind", &int(self.behind))
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
}
