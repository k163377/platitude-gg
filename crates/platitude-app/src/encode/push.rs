//! The push of the branch HEAD is on, as one value QML hands on and never
//! builds (`WorktreeModel.headPush`), and the toolbar's answer to it
//! (`GitFacts.toolbarPush`).

use platitude_core::AheadBehind;
use platitude_core::remote::{HeadPush, HeadPushFacts, PushTrack, StatusPush};
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, One, Record, field};

/// What the toolbar's push slot takes (`GitFacts.toolbarPush`).
pub type Push = One<HeadPush>;

/// What the toolbar's push stands on, read whole (`HeadPushFacts`): the
/// fields a menu's `push` row reads of its branch (`nav::BranchPush`), the
/// remote branch an overwrite leases to by name (`PublishFlow.leaseOid`
/// reads its commit off the remotes as the press goes out), and the counts
/// the tip says.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolbarPush {
    /// `PushStanding`'s word (`publish` / `ready` / `stays-here` …), for
    /// the words the button says.
    pub state: String,
    /// A plain press sends (`PushStanding::sends_plain`).
    pub sends_plain: bool,
    /// Only the held overwrite reaches (`PushStanding::overwrites`).
    pub overwrites: bool,
    pub target: String,
    pub measured: String,
    pub ahead: i32,
    pub behind: i32,
    pub upstream: String,
}

impl From<HeadPushFacts> for ToolbarPush {
    fn from(facts: HeadPushFacts) -> Self {
        Self {
            state: facts.standing.as_str().to_string(),
            sends_plain: facts.standing.sends_plain(),
            overwrites: facts.standing.overwrites(),
            target: facts.target,
            measured: facts.measured,
            ahead: shown(facts.counts.ahead),
            behind: shown(facts.counts.behind),
            upstream: facts.upstream,
        }
    }
}

impl Record for ToolbarPush {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("state", &self.state)
            .put("sendsPlain", &self.sends_plain)
            .put("overwrites", &self.overwrites)
            .put("target", &self.target)
            .put("measured", &self.measured)
            .put("ahead", &self.ahead)
            .put("behind", &self.behind)
            .put("upstream", &self.upstream)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        Some(Self {
            state: field(map, "state")?,
            sends_plain: field(map, "sendsPlain")?,
            overwrites: field(map, "overwrites")?,
            target: field(map, "target")?,
            measured: field(map, "measured")?,
            ahead: field(map, "ahead")?,
            behind: field(map, "behind")?,
            upstream: field(map, "upstream")?,
        })
    }
}

/// Flat, under `WorktreeModel`'s property names (`pushRemote`,
/// `pushDefault`, `tracked` — what the harness reads off it), in Qt's
/// spellings of "nothing": a flag down beside the upstream's counts, an
/// empty name for each mark and the destination's tracking ref
/// ([`named`]) — its remote riding beside it. The counts are Qt's `int`s,
/// a count past one drawn as its largest.
impl Record for HeadPush {
    fn to_map(&self) -> QVariantMap {
        let counts = self.upstream_counts.unwrap_or_default();
        let track = self.status.track.as_ref();
        let pushed = track.map(|track| track.counts).unwrap_or_default();
        Fields::new()
            .put("branch", &self.branch)
            .put("detached", &self.detached)
            .put("unborn", &self.unborn)
            .put("settled", &self.settled)
            .put("upstream", &self.upstream)
            .put("upstreamHere", &self.status.upstream_here)
            .put("tracked", &self.upstream_counts.is_some())
            .put("ahead", &shown(counts.ahead))
            .put("behind", &shown(counts.behind))
            .put(
                "pushRemote",
                self.status.push_remote.as_deref().unwrap_or_default(),
            )
            .put(
                "pushDefault",
                self.status.push_default.as_deref().unwrap_or_default(),
            )
            .put(
                "pushTrackRemote",
                track.map_or("", |track| track.remote.as_str()),
            )
            .put(
                "pushTracking",
                track.map_or("", |track| track.tracking.as_str()),
            )
            .put("pushAhead", &shown(pushed.ahead))
            .put("pushBehind", &shown(pushed.behind))
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        let tracked: bool = field(map, "tracked")?;
        let counts = counts_of(field(map, "ahead")?, field(map, "behind")?);
        let pushed = counts_of(field(map, "pushAhead")?, field(map, "pushBehind")?);
        let remote: String = field(map, "pushTrackRemote")?;
        Some(Self {
            branch: field(map, "branch")?,
            detached: field(map, "detached")?,
            unborn: field(map, "unborn")?,
            settled: field(map, "settled")?,
            upstream: field(map, "upstream")?,
            upstream_counts: tracked.then_some(counts),
            status: StatusPush {
                push_remote: named(field(map, "pushRemote")?),
                push_default: named(field(map, "pushDefault")?),
                upstream_here: field(map, "upstreamHere")?,
                track: named(field(map, "pushTracking")?).map(|tracking| PushTrack {
                    remote,
                    tracking,
                    counts: pushed,
                }),
            },
        })
    }
}

/// Qt's word for no name — an empty string — read as none: the one place
/// the toolbar's push reads it so, for the record's marks and tracking ref
/// (`GitFacts.toolbarPush`).
fn named<S: AsRef<str>>(name: S) -> Option<S> {
    Some(name).filter(|name| !name.as_ref().is_empty())
}

fn shown(count: u32) -> i32 {
    i32::try_from(count).unwrap_or(i32::MAX)
}

/// A pair of counts as QML hands it back: none of them is under zero; one
/// that were counts for nothing.
fn counts_of(ahead: i32, behind: i32) -> AheadBehind {
    AheadBehind {
        ahead: u32::try_from(ahead).unwrap_or(0),
        behind: u32::try_from(behind).unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qtbridge::QVariantConvertible;

    fn marked_elsewhere() -> HeadPush {
        HeadPush {
            branch: "feature/日本語".into(),
            detached: false,
            unborn: false,
            settled: true,
            upstream: "origin/feature/日本語".into(),
            upstream_counts: Some(AheadBehind {
                ahead: 2,
                behind: 1,
            }),
            status: StatusPush {
                push_remote: Some("fork".into()),
                push_default: Some("origin".into()),
                upstream_here: false,
                track: Some(PushTrack {
                    remote: "fork".into(),
                    tracking: "fork/feature/日本語".into(),
                    counts: AheadBehind {
                        ahead: 1,
                        behind: 3,
                    },
                }),
            },
        }
    }

    fn round_trip(push: HeadPush) -> Option<HeadPush> {
        let held = Push::new(push);
        <Push as QVariantConvertible>::try_from_qvariant(&held.to_qvariant())
            .map(|back| (*back).clone())
    }

    #[test]
    fn a_push_round_trips() {
        assert_eq!(round_trip(marked_elsewhere()), Some(marked_elsewhere()));
        assert_eq!(
            round_trip(HeadPush::default()),
            Some(HeadPush::default()),
            "nothing counted, nothing marked, nothing tracked over there"
        );
    }

    /// The bridge's spellings of "nothing" reach the rules as none,
    /// whatever numbers ride with them.
    #[test]
    fn a_flag_down_and_an_empty_name_are_nothing() {
        use qtbridge::qtbridge_type_lib::QString;
        let mut map = marked_elsewhere().to_map();
        map.insert(QString::from("tracked"), false.to_qvariant());
        map.insert(QString::from("pushRemote"), String::new().to_qvariant());
        map.insert(QString::from("pushDefault"), String::new().to_qvariant());
        map.insert(QString::from("pushTracking"), String::new().to_qvariant());
        let read = HeadPush::from_map(&map).expect("a record");
        assert_eq!(read.upstream_counts, None);
        assert_eq!(read.status.push_remote, None);
        assert_eq!(read.status.push_default, None);
        assert_eq!(read.status.track, None);
    }

    #[test]
    fn a_pair_of_counts_comes_back_as_it_went_out() {
        let pair = |ahead, behind| AheadBehind { ahead, behind };
        assert_eq!(counts_of(2, 1), pair(2, 1));
        assert_eq!(counts_of(0, 0), pair(0, 0));
        assert_eq!(
            counts_of(shown(u32::MAX), 0),
            pair(i32::MAX.unsigned_abs(), 0),
            "a count drawn at its largest stays a count"
        );
        assert_eq!(counts_of(-1, -7), pair(0, 0), "under zero counts for none");
    }
}
