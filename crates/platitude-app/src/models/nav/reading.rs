//! What a branch is measured against, as an opened row and a chip's card
//! name it (`NavFacts.readingOf`): one record off the branch's one
//! `BranchItem`, so the name, whether git can reach it, the section it is
//! a row of and the commit it stands on cannot disagree.

use platitude_core::session::BranchItem;
use qtbridge::qtbridge_type_lib::QVariantMap;

use crate::encode::{Fields, Record, field};

/// The upstream a branch is configured with, wherever it stands — a
/// reading listed on a remote, a branch here (`remote = .`), or gone from
/// the listing. `None` of it where the branch has none
/// ([`UpstreamReading::of`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpstreamReading {
    /// By short name (`BranchItem::upstream_named`).
    pub upstream: String,
    /// Nothing in the listing answers to the name — git's `[gone]`
    /// (`BranchItem::upstream_gone`), the warning the row's badge wears.
    pub gone: bool,
    /// It is a branch here (`BranchItem::upstream_here`): the section whose
    /// mark the line wears and whose row a press goes to. A short name
    /// cannot say so — a branch here may be called `origin/main`.
    pub here: bool,
    /// Hex; the commit it stands on (`BranchItem::upstream_oid`). Empty
    /// where it is gone: a gone one stands nowhere.
    pub oid: String,
}

impl UpstreamReading {
    /// What `branch` is measured against; `None` where it is measured
    /// against nothing (a remote-tracking ref is measured against nothing).
    pub fn of(branch: &BranchItem) -> Option<Self> {
        let upstream = branch.upstream_named();
        (!upstream.is_empty()).then(|| Self {
            upstream: upstream.to_string(),
            gone: !branch.upstream_gone.is_empty(),
            here: branch.upstream_here,
            oid: crate::encode::oid_text(branch.upstream_oid),
        })
    }
}

impl Record for UpstreamReading {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("upstream", &self.upstream)
            .put("gone", &self.gone)
            .put("upstreamHere", &self.here)
            .put("upstreamOid", &self.oid)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        Some(Self {
            upstream: field(map, "upstream")?,
            gone: field(map, "gone")?,
            here: field(map, "upstreamHere")?,
            oid: field(map, "upstreamOid")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::testkit::*;
    use super::*;

    /// Each of the three places `BranchItem` names its upstream reads as
    /// one record, the section it is a row of and its commit beside it; a
    /// gone one stands on no commit, and a branch measured against nothing
    /// reads nothing.
    #[test]
    fn every_place_an_upstream_stands_reads_as_one_record() {
        let mut reads = local("reads", false);
        reads.upstream = "origin/reads".into();
        reads.upstream_oid = Some(oid("1"));
        let mut stacked = local("stacked", false);
        stacked.upstream_local = "main".into();
        stacked.upstream_here = true;
        stacked.upstream_oid = Some(oid("2"));
        let mut lost = local("lost", false);
        lost.upstream_gone = "origin/lost".into();
        let mut lost_here = local("lost-here", false);
        lost_here.upstream_gone = "base".into();
        lost_here.upstream_here = true;
        let read = |upstream: &str, gone: bool, here: bool, oid: String| UpstreamReading {
            upstream: upstream.to_string(),
            gone,
            here,
            oid,
        };
        assert_eq!(
            UpstreamReading::of(&reads),
            Some(read("origin/reads", false, false, oid("1").to_hex()))
        );
        assert_eq!(
            UpstreamReading::of(&stacked),
            Some(read("main", false, true, oid("2").to_hex())),
            "a branch here"
        );
        assert_eq!(
            UpstreamReading::of(&lost),
            Some(read("origin/lost", true, false, String::new()))
        );
        assert_eq!(
            UpstreamReading::of(&lost_here),
            Some(read("base", true, true, String::new())),
            "a branch here, gone"
        );
        assert_eq!(UpstreamReading::of(&local("plain", false)), None);
        assert_eq!(UpstreamReading::of(&remote("origin/reads")), None);
    }

    /// The record reads back as it went over (`encode::wire`).
    #[test]
    fn the_record_reads_back_as_it_went() {
        let sent = UpstreamReading {
            upstream: "main".into(),
            gone: false,
            here: true,
            oid: oid("2").to_hex(),
        };
        assert_eq!(UpstreamReading::from_map(&sent.to_map()), Some(sent));
    }
}
