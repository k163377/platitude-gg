//! The push of the branch HEAD is on, as one value QML hands on and never
//! builds (`WorktreeModel.headPush`), and what a branch's push stands on —
//! the toolbar's and a branch's `push` row's alike — as one record
//! (`NavSectionModel.headPush` / `branchPush`).

use platitude_core::AheadBehind;
use platitude_core::remote::{HeadBranch, HeadPush, PushFacts, PushTrack, StatusPush};
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::counts::shown_count;
use super::wire::{Fields, One, Record, field, optional_text};

/// What the toolbar's push slot takes (`NavSectionModel.headPush`).
pub type Push = One<HeadPush>;

/// What a branch's push stands on ([`PushFacts`]) as QML reads it, both
/// doors' — the one place either answer is put in Qt's spellings of
/// "nothing": an empty name and an empty commit, counts drawn as Qt's
/// `int`s. With what a press does there, core's rules, so QML weighs only
/// whether the doors are open.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PushRecord {
    /// `PushStanding`'s word (`publish` / `ready` / `stays-here` …), for the
    /// words the button and the row say.
    pub state: String,
    /// A plain press sends (`PushStanding::sends_plain`).
    pub sends_plain: bool,
    /// …after asking where the branch goes (`PushStanding::asks`).
    pub asks: bool,
    /// Only the held overwrite reaches (`PushStanding::overwrites`).
    pub overwrites: bool,
    /// A branch's menu keeps its `push` row (`PushStanding::offered`).
    pub offered: bool,
    pub target: String,
    /// Hex; empty where the overwrite leases to git's own tracking ref.
    pub lease: String,
    pub ahead: i32,
    pub behind: i32,
    pub upstream: String,
}

impl From<PushFacts> for PushRecord {
    fn from(facts: PushFacts) -> Self {
        let standing = facts.standing;
        Self {
            state: standing.as_str().to_string(),
            sends_plain: standing.sends_plain(),
            asks: standing.asks(),
            overwrites: standing.overwrites(),
            offered: standing.offered(),
            target: facts.target.unwrap_or_default(),
            lease: facts.lease.map(|oid| oid.to_hex()).unwrap_or_default(),
            ahead: shown_count(facts.counts.ahead),
            behind: shown_count(facts.counts.behind),
            upstream: facts.upstream.unwrap_or_default(),
        }
    }
}

impl Record for PushRecord {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("state", &self.state)
            .put("sendsPlain", &self.sends_plain)
            .put("asks", &self.asks)
            .put("overwrites", &self.overwrites)
            .put("offered", &self.offered)
            .put("target", &self.target)
            .put("lease", &self.lease)
            .put("ahead", &self.ahead)
            .put("behind", &self.behind)
            .put("upstream", &self.upstream)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        Some(Self {
            state: field(map, "state")?,
            sends_plain: field(map, "sendsPlain")?,
            asks: field(map, "asks")?,
            overwrites: field(map, "overwrites")?,
            offered: field(map, "offered")?,
            target: field(map, "target")?,
            lease: field(map, "lease")?,
            ahead: field(map, "ahead")?,
            behind: field(map, "behind")?,
            upstream: field(map, "upstream")?,
        })
    }
}

/// Flat, in Qt's spellings of "nothing": an empty `branch` for HEAD on no
/// branch, `settled` down while the last status is another branch's,
/// `tracked` down beside the upstream's counts where there is nothing to
/// count against, an empty name for the upstream, each mark and the
/// destination's tracking ref ([`named`]) — its remote riding beside it.
/// The counts are Qt's `int`s, a count past one drawn as its largest. The
/// harness reads `settled`, `tracked` and the two marks (`pushRemote`,
/// `pushDefault`) off it.
impl Record for HeadPush {
    fn to_map(&self) -> QVariantMap {
        let blank = HeadBranch::default();
        let (name, read) = match self {
            Self::Detached => ("", &blank),
            Self::Moving(name) => (name.as_str(), &blank),
            Self::Read(branch) => (branch.name.as_str(), branch),
        };
        let counts = read.upstream_counts.unwrap_or_default();
        let track = read.status.track.as_ref();
        let pushed = track.map(|track| track.counts).unwrap_or_default();
        Fields::new()
            .put("branch", &name.to_string())
            .put("settled", &self.settled())
            .put("unborn", &read.unborn)
            .put("upstream", read.upstream.as_deref().unwrap_or_default())
            .put("upstreamHere", &read.status.upstream_here)
            .put("tracked", &read.upstream_counts.is_some())
            .put("ahead", &shown_count(counts.ahead))
            .put("behind", &shown_count(counts.behind))
            .put(
                "pushRemote",
                read.status.push_remote.as_deref().unwrap_or_default(),
            )
            .put(
                "pushDefault",
                read.status.push_default.as_deref().unwrap_or_default(),
            )
            .put(
                "pushTrackRemote",
                track.map_or("", |track| track.remote.as_str()),
            )
            .put(
                "pushTracking",
                track.map_or("", |track| track.tracking.as_str()),
            )
            .put("pushAhead", &shown_count(pushed.ahead))
            .put("pushBehind", &shown_count(pushed.behind))
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        let name: String = field(map, "branch")?;
        let settled: bool = field(map, "settled")?;
        if name.is_empty() {
            return Some(Self::Detached);
        }
        if !settled {
            return Some(Self::Moving(name));
        }
        let tracked: bool = field(map, "tracked")?;
        let counts = counts_of(field(map, "ahead")?, field(map, "behind")?);
        let pushed = counts_of(field(map, "pushAhead")?, field(map, "pushBehind")?);
        let remote: String = field(map, "pushTrackRemote")?;
        Some(Self::Read(HeadBranch {
            name,
            unborn: field(map, "unborn")?,
            upstream: optional_text(field::<String>(map, "upstream")?),
            upstream_counts: tracked.then_some(counts),
            status: StatusPush {
                push_remote: optional_text(field(map, "pushRemote")?),
                push_default: optional_text(field(map, "pushDefault")?),
                upstream_here: field(map, "upstreamHere")?,
                track: optional_text(field(map, "pushTracking")?).map(|tracking| PushTrack {
                    remote,
                    tracking,
                    counts: pushed,
                }),
            },
        }))
    }
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
    use platitude_core::remote::PushStanding;
    use qtbridge::QVariantConvertible;

    fn marked_elsewhere() -> HeadPush {
        HeadPush::Read(HeadBranch {
            name: "feature/日本語".into(),
            unborn: false,
            upstream: Some("origin/feature/日本語".into()),
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
        })
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
            round_trip(HeadPush::Read(HeadBranch {
                name: "main".into(),
                ..HeadBranch::default()
            })),
            Some(HeadPush::Read(HeadBranch {
                name: "main".into(),
                ..HeadBranch::default()
            })),
            "nothing counted, nothing marked, nothing tracked over there"
        );
        assert_eq!(
            round_trip(HeadPush::Moving("main".into())),
            Some(HeadPush::Moving("main".into()))
        );
        assert_eq!(round_trip(HeadPush::Detached), Some(HeadPush::Detached));
    }

    /// The bridge's spellings of "nothing" reach the rules as none,
    /// whatever numbers ride with them.
    #[test]
    fn a_flag_down_and_an_empty_name_are_nothing() {
        use qtbridge::qtbridge_type_lib::QString;
        let mut map = marked_elsewhere().to_map();
        map.insert(QString::from("tracked"), false.to_qvariant());
        map.insert(QString::from("upstream"), String::new().to_qvariant());
        map.insert(QString::from("pushRemote"), String::new().to_qvariant());
        map.insert(QString::from("pushDefault"), String::new().to_qvariant());
        map.insert(QString::from("pushTracking"), String::new().to_qvariant());
        let Some(HeadPush::Read(read)) = HeadPush::from_map(&map) else {
            panic!("a branch read");
        };
        assert_eq!(read.upstream, None);
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
            counts_of(shown_count(u32::MAX), 0),
            pair(i32::MAX.unsigned_abs(), 0),
            "a count drawn at its largest stays a count"
        );
        assert_eq!(counts_of(-1, -7), pair(0, 0), "under zero counts for none");
    }

    /// Either door's answer carries core's press rules, the Qt spellings of
    /// "nothing", and reads back whole.
    #[test]
    fn a_push_answer_carries_cores_rules_and_reads_back_whole() {
        for standing in [
            PushStanding::Closed,
            PushStanding::Publish,
            PushStanding::StaysHere,
            PushStanding::Clean,
            PushStanding::Diverged,
        ] {
            let record = PushRecord::from(PushFacts {
                standing,
                ..PushFacts::default()
            });
            assert_eq!(
                (
                    record.sends_plain,
                    record.asks,
                    record.overwrites,
                    record.offered
                ),
                (
                    standing.sends_plain(),
                    standing.asks(),
                    standing.overwrites(),
                    standing.offered()
                ),
                "{standing:?}"
            );
            assert_eq!(
                (
                    record.target.as_str(),
                    record.lease.as_str(),
                    record.upstream.as_str()
                ),
                ("", "", ""),
                "none of them is spelled empty"
            );
            assert_eq!(PushRecord::from_map(&record.to_map()), Some(record));
        }
    }
}
