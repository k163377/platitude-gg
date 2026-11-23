//! What a ref's card reads over there (`offers::FarSide`), as the fields of
//! the record the model hands QML — a branch's `NavSectionModel.branchUpstream`,
//! a tag's `tagMenu` — which QML hands on to `GitFacts.refMenuOffers` as it
//! is and never builds.

use platitude_core::offers::{BranchFar, FarSide, Reading, TagDeleteHeld, TagFar, TagSides};
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, Record, field};

/// [`FarSide`]'s fields, written into the record that carries it — flat,
/// beside the record's own, as [`super::TreeReading`] carries the
/// standing: `far` says which (`branch` / `tag`, empty for none), and the
/// rest are its parts. The TAG card reads two of them too: `heldBack`
/// (`drift` / `unnamed`) for the words its greyed rows say, and `sides` for
/// whether the copy here exists.
pub(crate) fn put_far(fields: Fields, far: &FarSide) -> Fields {
    match far {
        FarSide::Unread => fields.put("far", ""),
        FarSide::Branch(upstream) => fields
            .put("far", "branch")
            .put("reading", upstream.reading.word())
            .put("tracked", &upstream.tracked)
            .put("hasRemote", &upstream.has_remote)
            .put("ahead", &upstream.ahead),
        FarSide::Tag(tag) => fields
            .put("far", "tag")
            .put("sides", tag.sides.map_or("", TagSides::word))
            .put("hasRemote", &tag.has_remote)
            .put("heldBack", tag.held_back.word()),
    }
}

/// The inverse: `None` for a record that carries no far side, or one whose
/// parts do not read.
pub(crate) fn read_far(map: &QVariantMap) -> Option<FarSide> {
    let side: String = field(map, "far")?;
    match side.as_str() {
        "" => Some(FarSide::Unread),
        "branch" => Some(FarSide::Branch(BranchFar {
            reading: Reading::from_word(&field::<String>(map, "reading")?)?,
            tracked: field(map, "tracked")?,
            has_remote: field(map, "hasRemote")?,
            ahead: field(map, "ahead")?,
        })),
        "tag" => Some(FarSide::Tag(TagFar {
            sides: sides_of(&field::<String>(map, "sides")?)?,
            has_remote: field(map, "hasRemote")?,
            held_back: TagDeleteHeld::from_word(&field::<String>(map, "heldBack")?)?,
        })),
        _ => None,
    }
}

/// What `GitFacts.refMenuOffers` takes: the far side of whichever record
/// QML hands it, the rest of the record left.
impl Record for FarSide {
    fn to_map(&self) -> QVariantMap {
        put_far(Fields::new(), self).done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        read_far(map)
    }
}

/// A tag's sides as the record carries them: empty where the section has
/// not answered for the name (`TagFar::sides`), else the side's word —
/// `None` for one that is neither.
fn sides_of(word: &str) -> Option<Option<TagSides>> {
    if word.is_empty() {
        Some(None)
    } else {
        TagSides::from_word(word).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::{One, Optional};
    use qtbridge::QVariantConvertible;

    /// Every word each part of a far side goes out in: the three readings,
    /// and the sides — unread too — under each of the three holds.
    fn sides() -> Vec<FarSide> {
        let branches = [Reading::None, Reading::Together, Reading::Drifted]
            .into_iter()
            .map(|reading| {
                FarSide::Branch(BranchFar {
                    reading,
                    tracked: reading != Reading::None,
                    has_remote: true,
                    ahead: 2,
                })
            });
        let tags = [
            None,
            Some(TagSides::Here),
            Some(TagSides::Remote),
            Some(TagSides::Both),
        ]
        .into_iter()
        .flat_map(|sides| {
            [
                TagDeleteHeld::No,
                TagDeleteHeld::Drifted,
                TagDeleteHeld::Unnamed,
            ]
            .map(|held_back| {
                FarSide::Tag(TagFar {
                    sides,
                    has_remote: held_back != TagDeleteHeld::No,
                    held_back,
                })
            })
        });
        std::iter::once(FarSide::Unread)
            .chain(branches)
            .chain(tags)
            .collect()
    }

    #[test]
    fn every_far_side_round_trips() {
        for far in sides() {
            let held = One::new(far);
            let back =
                <One<FarSide> as QVariantConvertible>::try_from_qvariant(&held.to_qvariant());
            assert_eq!(back.map(|back| *back), Some(far));
        }
    }

    /// What the slot receives of a model's record: its far side, the record's
    /// own fields beside it left.
    #[test]
    fn the_far_side_reads_out_of_a_record_carrying_it() {
        let far = FarSide::Tag(TagFar {
            sides: Some(TagSides::Both),
            has_remote: true,
            held_back: TagDeleteHeld::Unnamed,
        });
        let record = put_far(Fields::new(), &far)
            .put("pushRemote", "origin")
            .put("carriers", "fork, mirror")
            .done();
        assert_eq!(FarSide::from_map(&record), Some(far));
    }

    /// `undefined` is the row's own menu, which reads nothing over there.
    #[test]
    fn nothing_handed_is_no_far_side() {
        let none = Optional::<FarSide>::none();
        let back =
            <Optional<FarSide> as QVariantConvertible>::try_from_qvariant(&none.to_qvariant());
        assert!(back.is_some_and(|back| back.is_none()));
    }

    #[test]
    fn a_word_nothing_answers_to_does_not_read() {
        let record = Fields::new()
            .put("far", "branch")
            .put("reading", "elsewhere")
            .put("tracked", &true)
            .put("hasRemote", &true)
            .put("ahead", &0u32)
            .done();
        assert_eq!(FarSide::from_map(&record), None);
        let record = Fields::new().put("far", "stash").done();
        assert_eq!(FarSide::from_map(&record), None);
        let record = Fields::new()
            .put("far", "tag")
            .put("sides", "neither")
            .put("hasRemote", &true)
            .put("heldBack", "")
            .done();
        assert_eq!(
            FarSide::from_map(&record),
            None,
            "a side the record cannot name is not read as unread"
        );
    }
}
