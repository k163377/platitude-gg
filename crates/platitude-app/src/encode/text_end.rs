//! Where the reader's selection of a diff's text ends: the end the hand
//! carried, which stands where a text box's caret would.

use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, Optional, Record, field};

/// The moving end of a selection: its column (`0` the rows' own lines,
/// `1` the new side of a split row), its row, and the place along that
/// row's line as the row spells it (`spelled_place`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEnd {
    pub side: i32,
    pub row: i32,
    pub place: i32,
}

/// That end, or nothing where no selection stands.
pub type Ended = Optional<TextEnd>;

impl Record for TextEnd {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("side", &self.side)
            .put("row", &self.row)
            .put("place", &self.place)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            side: field(map, "side")?,
            row: field(map, "row")?,
            place: field(map, "place")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qtbridge::QVariantConvertible;

    #[test]
    fn an_end_round_trips_and_none_is_nothing() {
        let ended = Ended::some(TextEnd {
            side: 1,
            row: 7,
            place: 12,
        });
        let back = <Ended as QVariantConvertible>::try_from_qvariant(&ended.to_qvariant());
        assert_eq!(back.map(|b| (*b).clone()), Ok((*ended).clone()));
        assert!(!Ended::none().to_qvariant().is_valid());
    }
}
