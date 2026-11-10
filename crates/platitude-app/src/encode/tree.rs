//! Where the worktree stands, as one value QML hands on and never builds
//! (`WorktreeModel.reading`).

use platitude_core::offers::TreeStanding;
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, One, Record, field};

/// What the menus and the message boxes read of the worktree: what core's
/// rules stand on, and what the rows draw beside them. One record, so a
/// menu raised under a command keeps the whole of it as it stood
/// (`RepoPage.treeAtAsk`).
///
/// Its fields are flat, under `WorktreeModel`'s own property names: QML
/// reads `tree.branch` off a kept one as off the model, and a slot taking
/// [`TreeStanding`] reads its part of the same map, leaving the rest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TreeReading {
    pub standing: TreeStanding,
    pub unborn: bool,
    pub hard_reset_takes: i32,
    pub hard_reset_not_copied: bool,
    pub head_reached_elsewhere: bool,
    pub stash_standing: String,
    pub not_copied: Vec<String>,
}

pub type Reading = One<TreeReading>;

/// The standing alone (`WorktreeModel.standing`).
pub type Standing = One<TreeStanding>;

/// [`TreeStanding`]'s fields — written once, for both records.
fn put_standing(fields: Fields, standing: &TreeStanding) -> Fields {
    fields
        .put("branch", &standing.branch)
        .put("detached", &standing.detached)
        .put("headOid", &standing.head_oid)
        .put("opText", &standing.op_text)
        .put("opEditing", &standing.op_editing)
        .put("conflictCount", &standing.conflict_count)
        .put("upstream", &standing.upstream)
        .put("pullBlocked", &standing.diverged)
}

fn read_standing(map: &QVariantMap) -> Result<TreeStanding, ()> {
    Ok(TreeStanding {
        branch: field(map, "branch")?,
        detached: field(map, "detached")?,
        head_oid: field(map, "headOid")?,
        op_text: field(map, "opText")?,
        op_editing: field(map, "opEditing")?,
        conflict_count: field(map, "conflictCount")?,
        upstream: field(map, "upstream")?,
        diverged: field(map, "pullBlocked")?,
    })
}

/// What a `GitFacts` slot takes: the part of a [`TreeReading`] the rules
/// read.
impl Record for TreeStanding {
    fn to_map(&self) -> QVariantMap {
        put_standing(Fields::new(), self).done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        read_standing(map)
    }
}

impl Record for TreeReading {
    fn to_map(&self) -> QVariantMap {
        put_standing(Fields::new(), &self.standing)
            .put("unborn", &self.unborn)
            .put("hardResetTakes", &self.hard_reset_takes)
            .put("hardResetNotCopied", &self.hard_reset_not_copied)
            .put("headReachedElsewhere", &self.head_reached_elsewhere)
            .put("stashStanding", &self.stash_standing)
            .put("notCopied", &self.not_copied)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            standing: read_standing(map)?,
            unborn: field(map, "unborn")?,
            hard_reset_takes: field(map, "hardResetTakes")?,
            hard_reset_not_copied: field(map, "hardResetNotCopied")?,
            head_reached_elsewhere: field(map, "headReachedElsewhere")?,
            stash_standing: field(map, "stashStanding")?,
            not_copied: field(map, "notCopied")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qtbridge::QVariantConvertible;

    fn reading() -> TreeReading {
        TreeReading {
            standing: TreeStanding {
                branch: "feature/日本語".into(),
                detached: false,
                head_oid: "abc123".into(),
                op_text: "MERGING".into(),
                op_editing: true,
                conflict_count: 2,
                upstream: "origin/feature/日本語".into(),
                diverged: true,
            },
            unborn: true,
            hard_reset_takes: 3,
            hard_reset_not_copied: true,
            head_reached_elsewhere: true,
            stash_standing: "conflicts".into(),
            not_copied: vec!["big.bin".into()],
        }
    }

    #[test]
    fn a_reading_round_trips() {
        let held = Reading::new(reading());
        let back = <Reading as QVariantConvertible>::try_from_qvariant(&held.to_qvariant());
        assert_eq!(back.map(|b| (*b).clone()), Ok(reading()));
    }

    /// What a slot taking the standing alone receives of a whole reading.
    #[test]
    fn the_standing_reads_out_of_a_whole_reading() {
        let held = Reading::new(reading());
        let standing =
            <One<TreeStanding> as QVariantConvertible>::try_from_qvariant(&held.to_qvariant());
        assert_eq!(standing.map(|s| (*s).clone()), Ok(reading().standing));
    }
}
