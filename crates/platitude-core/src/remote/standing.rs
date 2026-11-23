//! The pure half of a push: what one could do, decided from
//! configuration and counts already in hand.
//!
//! Nothing here runs git; [`super::push`] does, and the two must resolve
//! the same destination or the toolbar names a remote the send never goes
//! to. The name shown, the counts, whether it can be pressed and the lease
//! all come from that one destination, on the toolbar and on a branch's
//! row alike. Read off names instead, a branch `topic` measured against a
//! branch here called `origin/base` and marked `pushRemote = origin`
//! would be shown going to `origin/base`, counted against it and leased
//! to origin's `base`, while the send goes to `origin/topic`.

use super::route::{Marked, PushRoute, Upstream};

/// What a push of a branch can do, read off the last status (the branch
/// the worktree is on) or refs listing (any other) and the marked remote.
/// The counts come from the last fetch, so a push they say fits may still
/// be refused (デザイン規約 §リモートへ送る): they are trusted only to say
/// a push cannot land. With no remote at all a branch is `Publish` — the
/// question can make one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PushStanding {
    /// No branch here to send: detached, or nothing checked out — and,
    /// for the branch HEAD is on, nothing known of it yet: the last status
    /// is still another branch's ([`HeadPush::settled`]).
    #[default]
    Closed,
    /// No commits yet: git refuses the send (`src refspec <branch> does
    /// not match any`).
    Unborn,
    /// Never sent, or the tracking ref is gone: where the branch goes is
    /// a question to ask.
    Publish,
    /// The push resolves to this repository (`.`,
    /// [`THIS_REPOSITORY`](super::route::THIS_REPOSITORY)):
    /// measured against a branch here ([`Upstream::Here`]) and marked to no
    /// remote, or marked to `.` itself by either key. A send there moves a
    /// branch here — the upstream where that is a branch here (`git push .
    /// <branch>:<it>`), else the branch onto itself — which is no send to
    /// anywhere, so none is offered: the toolbar and a branch's row stand
    /// unpressable saying why, and the send refuses too. Not asked as a
    /// first push either: the answer would rewrite the upstream as a
    /// remote's, and under a `.` mark the push would still stay here. The
    /// two ways are one standing, and the UI gives both the same reason
    /// (user decision — ask before changing). A branch whose upstream here
    /// is gone (`[gone]`) is `Publish` instead.
    StaysHere,
    /// The push goes to a marked remote other than the tracked one, and
    /// nothing here tracks the branch over there ([`PushTrack`]): no
    /// counts speak for it.
    Elsewhere,
    /// Commits of ours to add, and nothing in the way.
    Ready,
    /// The remote already has them all.
    Clean,
    /// The remote moved on; we have nothing to add.
    Behind,
    /// Both moved; only an overwrite can land.
    Diverged,
}

impl PushStanding {
    /// A plain press sends the branch, on the toolbar and on a branch's
    /// row alike — for `Publish`, after asking where it goes. `Elsewhere`
    /// too: nothing here counts against the far side, so the press lets
    /// git answer.
    pub fn sends_plain(self) -> bool {
        matches!(self, Self::Publish | Self::Elsewhere | Self::Ready)
    }

    /// Only an overwrite reaches: the far side has commits this branch
    /// lacks, and a tracking ref here to lease against. The press takes
    /// the held overwrite's shape on the toolbar and on a branch's row
    /// (デザイン規約 §リモートへ送る, §回数押し), so one press never carries two
    /// meanings. `Behind` too: the only send that reaches from there
    /// rewinds the far side, a plain push being answered "nothing to send".
    /// Never `Elsewhere`: nothing here tracks the branch over there, so no
    /// commit is on screen for the lease to pin to.
    pub fn overwrites(self) -> bool {
        matches!(self, Self::Behind | Self::Diverged)
    }

    /// The word the UI branches on (`PublishFlow.pushState`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Unborn => "unborn",
            Self::Publish => "publish",
            Self::StaysHere => "stays-here",
            Self::Elsewhere => "elsewhere",
            Self::Ready => "ready",
            Self::Clean => "clean",
            Self::Behind => "behind",
            Self::Diverged => "diverged",
        }
    }
}

/// Where a mark sends the push to another remote than the upstream's
/// ([`PushRoute::goes_elsewhere`]): the tracking ref of the branch over there, and
/// the branch's counts against it ([`super::push_track`] on the status
/// tick, [`super::push_tracks`] on the refs read for the other branches).
/// The upstream's counts are about the other remote.
///
/// There is one only where something here tracks the branch over there.
/// None where it was never fetched, where no fetch refspec of that remote
/// takes it in (or takes it outside the remote branches), and — with that
/// ref here — where `remote.<name>.push` sends the branch under another
/// name or not at all: such a refspec decides `%(push)` whatever
/// `push.default` says (`track_of`). A fetch reads every remote, so there
/// usually is one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushTrack {
    /// The remote the push was read going to, as git names it
    /// (`%(push:remotename)`). Not cut off `tracking`: that is wherever
    /// the remote's fetch refspec puts the branch, under any name
    /// (`refs/remotes/copy/main` for the fork).
    pub remote: String,
    /// The tracking ref as the remote branches are listed (`fork/main`).
    pub tracking: String,
    pub counts: crate::AheadBehind,
}

/// What a status tick reads of the push of its branch, beside the status
/// itself (`RepoSession::read_status_push`): read for the branch the status
/// names, so it never pairs with another, and the counts read under the
/// marks beside them. Display and standing only — a send reads the keys
/// again ([`super::plan_current_push`]). Default for a detached HEAD; where
/// the keys could not be read, the branch's last reading whole.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusPush {
    /// The branch's own mark, `branch.<branch>.pushRemote`; `None` where
    /// unset.
    pub push_remote: Option<String>,
    /// The repository's mark, `remote.pushDefault`, as this read found it;
    /// `None` where unset. What the toolbar weighs the push under
    /// ([`Self::route`]), not the refs snapshot's copy
    /// (`session::RefsSnapshot::marks`): a moved mark sends the refs out
    /// again (`RepoSession::note_push_default`), and they land before or
    /// after this tick — beside the counts it read, either order would
    /// pair one moment's mark with another's counts (pinned by
    /// `push_default_integration::a_mark_moved_outside_reaches_the_toolbar_whole`).
    pub push_default: Option<String>,
    /// The upstream is a branch here (`branch.<branch>.remote = .` —
    /// [`super::PushMarks::upstream_is_here`]): the short name git gives
    /// it can read like a remote branch's.
    pub upstream_here: bool,
    /// The counts against where a mark sends the push, when that is not
    /// the upstream's remote ([`PushRoute::goes_elsewhere`]); none
    /// everywhere else, and where nothing here tracks the branch over
    /// there.
    pub track: Option<PushTrack>,
}

impl StatusPush {
    /// The push's route under the two marks this read found, measured
    /// against `upstream` — as the status names it, empty where none; a
    /// branch here where configuration says so ([`Self::upstream_here`]) —
    /// and cut against the configured `remotes`. The one route of the
    /// branch HEAD is on: the tick weighs by it whether to read the
    /// destination's counts (`RepoSession::read_status_push`), and the
    /// toolbar's answer is read along it ([`HeadPush::route`]).
    pub fn route<'a>(
        &'a self,
        upstream: &'a str,
        remotes: impl IntoIterator<Item = &'a str>,
    ) -> PushRoute<'a> {
        PushRoute::new(
            Upstream::of(upstream, self.upstream_here),
            Marked::of(self.push_remote.as_deref(), self.push_default.as_deref()),
            remotes,
        )
    }
}

/// The push of the branch HEAD is on, as the worktree holds it: HEAD's
/// record beside what the last status read of the branch — what the
/// toolbar asks about (`WorktreeModel.headPush`). A branch's row is
/// answered off the refs instead (`session::RefsSnapshot::branch_push`).
///
/// HEAD alone can be detached or on a branch with no commits, so those
/// two are weighed here, ahead of any route ([`Self::facts`]); a listed
/// branch always has both a name and commits.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeadPush {
    /// The branch HEAD is on; empty detached, and before HEAD is read.
    pub branch: String,
    pub detached: bool,
    /// HEAD names a branch with no commits yet.
    pub unborn: bool,
    /// Whether the last status is about `branch`. Between a move of HEAD
    /// and the status behind it, the upstream, its counts and all of
    /// `status` are blank — they would be the last branch's, its own mark
    /// among them —, the push stands closed and names no destination.
    pub settled: bool,
    /// The upstream as the status names it — as the send names it
    /// ([`super::PushMarks::upstream_name`]) —, empty where none.
    pub upstream: String,
    /// The branch's counts against it; none where its ref is not there to
    /// count against.
    pub upstream_counts: Option<crate::AheadBehind>,
    pub status: StatusPush,
}

impl HeadPush {
    /// Its route under the two marks the status read, the upstream cut
    /// against the configured `remotes`.
    pub fn route<'a>(&'a self, remotes: impl IntoIterator<Item = &'a str>) -> PushRoute<'a> {
        self.status.route(&self.upstream, remotes)
    }

    /// The toolbar's whole answer, along [`Self::route`]; `fallback_remote`
    /// is where a branch that tracks nothing, marked nowhere, goes.
    /// Unsettled, it is closed and says nothing more: the route would be
    /// the last branch's keys read against this branch's name, or a guess
    /// at the fallback.
    pub fn facts<'a>(
        &'a self,
        remotes: impl IntoIterator<Item = &'a str>,
        fallback_remote: &str,
    ) -> HeadPushFacts {
        if !self.settled {
            return HeadPushFacts::default();
        }
        let route = self.route(remotes);
        let read = push_reading(route, self.upstream_counts, self.status.track.as_ref());
        // Before anything about remotes: with no ref to name, git refuses
        // every send.
        let standing = if self.detached || self.branch.is_empty() {
            PushStanding::Closed
        } else if self.unborn {
            PushStanding::Unborn
        } else {
            read.standing
        };
        HeadPushFacts {
            standing,
            target: route
                .target(&self.branch, fallback_remote)
                .unwrap_or_default(),
            measured: read.measured.unwrap_or_default().to_string(),
            counts: read.counts,
            upstream: route
                .upstream()
                .remote_branch()
                .unwrap_or_default()
                .to_string(),
        }
    }
}

/// What the toolbar's push stands on ([`HeadPush::facts`]): what a menu's
/// `push` row on another branch stands on (`session::BranchPushFacts`),
/// with the remote branch an overwrite leases to by name — the commit is
/// read off the remotes listing as the press goes out
/// (`PublishFlow.leaseOid`) — and the counts the tip says.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeadPushFacts {
    pub standing: PushStanding,
    /// Where the push goes, `<remote>/<branch>`; empty where nowhere.
    pub target: String,
    /// [`PushReading::measured`], empty where none.
    pub measured: String,
    /// [`PushReading::counts`].
    pub counts: crate::AheadBehind,
    /// The remote branch the branch is configured for, fetched or not —
    /// where a first push's question opens. Empty for a branch here, which
    /// names no remote whatever its name looks like.
    pub upstream: String,
}

/// One branch's push read along its route: the standing, and what it is
/// read against. The one reading the toolbar ([`HeadPush::facts`]) and a
/// branch's row (`session::RefsSnapshot::branch_push`) both answer from,
/// so the counts a tip says and the commit an overwrite leases to come off
/// the destination the standing weighed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PushReading<'a> {
    pub standing: PushStanding,
    /// The remote branch the counts are against: none where the push stays
    /// here; where a mark sends it elsewhere, its tracking ref there; else
    /// the upstream, when it is a remote's and fetched. What an overwrite
    /// leases to. Read by the upstream where the push goes elsewhere, the
    /// lease would pin origin's tip on a push to the fork, refused as
    /// `stale info`.
    pub measured: Option<&'a str>,
    /// The counts the standing reads: the destination's own where a mark
    /// sends the push elsewhere, else the upstream's — zero where none
    /// speak.
    pub counts: crate::AheadBehind,
}

/// [`push_standing`], with what it was read against. `push_track` speaks
/// only where it was read for the remote the marks name now
/// ([`PushTrack::remote`]): a read that could not tell repeats the counts
/// of where an earlier mark sent the push — on the status tick
/// (`session::Standing::push_of`) as on the refs read — and those leased
/// to would be refused over there as `stale info`.
pub fn push_reading<'a>(
    route: PushRoute<'a>,
    upstream_counts: Option<crate::AheadBehind>,
    push_track: Option<&'a PushTrack>,
) -> PushReading<'a> {
    let push_track = push_track
        .filter(|track| matches!(route.marked(), Marked::Remote(remote) if remote == track.remote));
    let standing = push_standing(route, upstream_counts, push_track);
    let elsewhere = route.goes_elsewhere();
    let measured = match route.upstream() {
        _ if standing == PushStanding::StaysHere => None,
        _ if elsewhere => push_track.map(|track| track.tracking.as_str()),
        Upstream::Remote(name) if upstream_counts.is_some() => Some(name),
        Upstream::Remote(_) | Upstream::Here | Upstream::None => None,
    };
    let counts = if elsewhere {
        push_track.map(|track| track.counts)
    } else {
        upstream_counts
    };
    PushReading {
        standing,
        measured,
        counts: counts.unwrap_or_default(),
    }
}

/// The standing itself, for a branch with commits under its name: the
/// table of counts, read against the upstream (`upstream_counts`), or
/// against `push_track` where the push goes elsewhere
/// ([`PushRoute::goes_elsewhere`]). `upstream_counts` is none where the
/// upstream's ref is not there to count against — git's `[gone]`, which
/// asks where the branch goes whichever kind the upstream was.
pub fn push_standing(
    route: PushRoute<'_>,
    upstream_counts: Option<crate::AheadBehind>,
    push_track: Option<&PushTrack>,
) -> PushStanding {
    // Before the `.` mark: the question's answer names the destination.
    let Some(counts) = upstream_counts.filter(|_| route.upstream() != Upstream::None) else {
        return PushStanding::Publish;
    };
    if route.stays_here() {
        return PushStanding::StaysHere;
    }
    if route.goes_elsewhere() {
        return marked_standing(push_track);
    }
    counted(counts)
}

/// The standing where a mark alone says where the push goes — to another
/// remote than the upstream's, or at all, the upstream being a branch
/// here. git refuses the push by the destination's ref (`[rejected]
/// (non-fast-forward)`), so only that ref's counts say how it goes, and
/// before that ref is fetched none do (user decision — ask before
/// changing). Read by the upstream's counts instead, a diverged
/// destination would stand a plain `push` with no `push -f` after the
/// refusal, and `ahead = 0` read as clean would make a branch unpressable
/// the moment it is marked. With a ref, the count rows of the table apply
/// unchanged, and the overwrite leases to that ref's tip as shown.
fn marked_standing(push_track: Option<&PushTrack>) -> PushStanding {
    push_track.map_or(PushStanding::Elsewhere, |track| counted(track.counts))
}

/// The table of counts.
fn counted(counts: crate::AheadBehind) -> PushStanding {
    if counts.is_diverged() {
        PushStanding::Diverged
    } else if counts.behind > 0 {
        PushStanding::Behind
    } else if counts.ahead > 0 {
        PushStanding::Ready
    } else {
        PushStanding::Clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::remote::Marked;

    const REMOTES: [&str; 3] = ["origin", "my", "my/fork"];

    /// The route the branch's own mark and the repository's give, empty
    /// standing for a key that is not set.
    fn route<'a>(
        upstream: Upstream<'a>,
        push_remote: &'a str,
        push_default: &'a str,
    ) -> PushRoute<'a> {
        let set = |mark: &'a str| Some(mark).filter(|mark| !mark.is_empty());
        PushRoute::new(
            upstream,
            Marked::of(set(push_remote), set(push_default)),
            REMOTES,
        )
    }

    fn standing_of(
        upstream: &str,
        tracked: bool,
        ahead: u32,
        behind: u32,
        marked: &str,
    ) -> PushStanding {
        push_standing(
            route(Upstream::of(upstream, false), "", marked),
            tracked.then_some(crate::AheadBehind { ahead, behind }),
            None,
        )
    }

    /// [`standing_of`] on a tracked upstream, with the destination's own
    /// counts beside the upstream's.
    fn standing_tracking(
        upstream: &str,
        ahead: u32,
        behind: u32,
        marked: &str,
        push_track: Option<&PushTrack>,
    ) -> PushStanding {
        push_standing(
            route(Upstream::Remote(upstream), "", marked),
            Some(crate::AheadBehind { ahead, behind }),
            push_track,
        )
    }

    /// A [`PushTrack`] read going to `remote`.
    fn track(remote: &str, tracking: &str, ahead: u32, behind: u32) -> PushTrack {
        PushTrack {
            remote: remote.to_string(),
            tracking: tracking.to_string(),
            counts: crate::AheadBehind { ahead, behind },
        }
    }

    /// [`standing_of`], with the branch's own mark set and the
    /// repository's empty.
    fn standing_marked_on_branch(upstream: &str, push_remote: &str) -> PushStanding {
        push_standing(
            route(Upstream::Remote(upstream), push_remote, ""),
            Some(crate::AheadBehind {
                ahead: 2,
                behind: 0,
            }),
            None,
        )
    }

    /// What a press does in each standing: a plain send, the held
    /// overwrite, or nothing at all.
    #[test]
    fn each_standing_takes_a_plain_press_the_overwrite_or_none() {
        use PushStanding::*;
        let press = |standing: PushStanding| (standing.sends_plain(), standing.overwrites());
        let plain = (true, false);
        let overwrite = (false, true);
        let none = (false, false);
        for (standing, takes) in [
            (Closed, none),
            (Unborn, none),
            (Publish, plain),
            (StaysHere, none),
            (Elsewhere, plain),
            (Ready, plain),
            (Clean, none),
            (Behind, overwrite),
            (Diverged, overwrite),
        ] {
            assert_eq!(press(standing), takes, "{standing:?}");
        }
    }

    #[test]
    fn a_push_standing_is_read_without_sending_anything() {
        assert_eq!(standing_of("", false, 0, 0, ""), PushStanding::Publish);
        assert_eq!(
            standing_of("origin/main", false, 0, 0, ""),
            PushStanding::Publish,
            "an upstream git cannot compare against still has to be published"
        );
        assert_eq!(
            standing_of("origin/main", true, 2, 0, ""),
            PushStanding::Ready
        );
        assert_eq!(
            standing_of("origin/main", true, 0, 0, ""),
            PushStanding::Clean
        );
        assert_eq!(
            standing_of("origin/main", true, 0, 3, ""),
            PushStanding::Behind
        );
        assert_eq!(
            standing_of("origin/main", true, 2, 3, ""),
            PushStanding::Diverged
        );
    }

    /// With nothing here tracking the branch over there, no counts speak
    /// for the push.
    #[test]
    fn a_marked_remote_takes_the_push_away_from_the_upstreams_counts() {
        assert_eq!(
            standing_of("origin/main", true, 2, 0, "my"),
            PushStanding::Elsewhere
        );
        assert_eq!(
            standing_of("origin/main", true, 2, 0, "origin"),
            PushStanding::Ready,
            "the mark and the upstream agree, so the counts still speak"
        );
        // `my/fork/main` lives on `my/fork`: a mark on `my` is another
        // remote even though the name is a prefix of the upstream.
        assert_eq!(
            standing_of("my/fork/main", true, 1, 0, "my"),
            PushStanding::Elsewhere
        );
        assert_eq!(
            standing_of("my/fork/main", true, 1, 0, "my/fork"),
            PushStanding::Ready
        );
        // An upstream on a remote the list does not know: exact prefix.
        assert_eq!(
            standing_of("gone/main", true, 1, 0, "gone"),
            PushStanding::Ready
        );
        assert_eq!(
            standing_of("gone/main", true, 1, 0, "origin"),
            PushStanding::Elsewhere
        );
    }

    /// The upstream is one commit behind us (`ready` against it); what the
    /// destination's own tracking ref says decides.
    #[test]
    fn the_destinations_own_counts_decide_where_the_push_goes_elsewhere() {
        assert_eq!(
            standing_tracking(
                "origin/main",
                1,
                0,
                "my",
                Some(&track("my", "my/main", 1, 1))
            ),
            PushStanding::Diverged,
            "git refuses the plain push there, so only the overwrite reaches"
        );
        assert_eq!(
            standing_tracking(
                "origin/main",
                1,
                0,
                "my",
                Some(&track("my", "my/main", 0, 2))
            ),
            PushStanding::Behind
        );
        assert_eq!(
            standing_tracking(
                "origin/main",
                1,
                0,
                "my",
                Some(&track("my", "my/main", 0, 0))
            ),
            PushStanding::Clean,
            "the destination has it all, though the upstream does not"
        );
        assert_eq!(
            standing_tracking(
                "origin/main",
                0,
                0,
                "my",
                Some(&track("my", "my/main", 3, 0))
            ),
            PushStanding::Ready,
            "the upstream has it all, the destination does not"
        );
        assert_eq!(
            standing_tracking(
                "origin/main",
                1,
                0,
                "origin",
                Some(&track("my", "my/main", 0, 2))
            ),
            PushStanding::Ready,
            "a mark on the upstream's own remote leaves the upstream's counts speaking"
        );
        assert_eq!(
            push_standing(
                route(Upstream::Remote("origin/main"), "", "my"),
                None,
                Some(&track("my", "my/main", 1, 0))
            ),
            PushStanding::Publish,
            "an upstream with no tracking ref still asks where the branch goes"
        );
    }

    #[test]
    fn a_branchs_own_mark_is_weighed_before_the_repositorys() {
        assert_eq!(
            standing_marked_on_branch("origin/main", "my"),
            PushStanding::Elsewhere,
            "the branch is marked at a remote it does not track"
        );
        assert_eq!(
            standing_marked_on_branch("origin/main", "origin"),
            PushStanding::Ready,
            "the branch's mark and its upstream agree"
        );
        assert_eq!(
            push_standing(
                route(Upstream::Remote("origin/main"), "origin", "my"),
                Some(crate::AheadBehind {
                    ahead: 2,
                    behind: 0
                }),
                None
            ),
            PushStanding::Ready,
            "the branch's mark wins over the repository's, so the counts still speak"
        );
        assert_eq!(
            push_standing(
                route(Upstream::Remote("origin/main"), "my", "origin"),
                Some(crate::AheadBehind {
                    ahead: 2,
                    behind: 0
                }),
                None
            ),
            PushStanding::Elsewhere,
            "and it wins the other way round too"
        );
    }

    /// `topic` measured against the branch here called `origin/base`, two
    /// commits ahead of it. The name says nothing about where it is: only
    /// a mark sends the branch anywhere, under its own name, and then the
    /// counts are the destination's.
    #[test]
    fn a_branch_here_is_on_no_remote_whatever_its_name_says() {
        let here = |push_remote: &'static str,
                    push_default: &'static str,
                    push_track: Option<&PushTrack>| {
            push_standing(
                route(Upstream::Here, push_remote, push_default),
                Some(crate::AheadBehind {
                    ahead: 2,
                    behind: 0,
                }),
                push_track,
            )
        };
        let target = |push_remote: &'static str, push_default: &'static str| {
            route(Upstream::of("origin/base", true), push_remote, push_default)
                .target("topic", "origin")
        };
        assert_eq!(
            (here("", "", None), target("", "")),
            (PushStanding::StaysHere, None),
            "marked to no remote: nowhere to send it, not even the fallback"
        );
        assert_eq!(
            (
                here("origin", "", Some(&track("origin", "origin/topic", 0, 0))),
                target("origin", "")
            ),
            (PushStanding::Clean, Some("origin/topic".to_string())),
            "level with origin/topic, however far ahead of the branch here"
        );
        assert_eq!(
            here("", "my", Some(&track("my", "my/topic", 0, 1))),
            PushStanding::Behind,
            "the repository's mark sends it just as well"
        );
        assert_eq!(target("", "my").as_deref(), Some("my/topic"));
        assert_eq!(
            here("origin", "", None),
            PushStanding::Elsewhere,
            "nothing here tracks origin/topic yet"
        );
        assert_eq!(
            push_standing(route(Upstream::Here, "", ""), None, None),
            PushStanding::Publish,
            "gone, it asks where the branch goes, as any gone upstream does"
        );
        assert!(route(Upstream::Here, "", "origin").goes_elsewhere());
        assert!(!route(Upstream::Here, "", "").goes_elsewhere());
    }

    /// `.` is git's name for this repository: a mark set to it sends the
    /// push to no remote, whatever the upstream — while a branch with
    /// nothing to count against still asks where it goes, the answer
    /// naming the remote itself.
    #[test]
    fn a_mark_naming_this_repository_sends_nowhere() {
        let with = |upstream: Upstream<'static>,
                    tracked: bool,
                    push_remote: &'static str,
                    push_default: &'static str| {
            push_standing(
                route(upstream, push_remote, push_default),
                tracked.then_some(crate::AheadBehind {
                    ahead: 1,
                    behind: 0,
                }),
                Some(&track("origin", "origin/topic", 1, 0)),
            )
        };
        assert_eq!(with(Upstream::Here, true, ".", ""), PushStanding::StaysHere);
        assert_eq!(with(Upstream::Here, true, "", "."), PushStanding::StaysHere);
        assert_eq!(
            with(Upstream::Remote("origin/topic"), true, ".", ""),
            PushStanding::StaysHere,
            "a remote upstream: a send there pushes the branch onto itself"
        );
        assert_eq!(
            with(Upstream::Here, true, ".", "origin"),
            PushStanding::StaysHere,
            "the branch's own mark beats the repository's"
        );
        assert_eq!(
            with(Upstream::Here, true, "origin", "."),
            PushStanding::Ready,
            "and the other way round"
        );
        assert_eq!(with(Upstream::None, false, ".", ""), PushStanding::Publish);
        assert_eq!(
            with(Upstream::Here, false, ".", ""),
            PushStanding::Publish,
            "a gone upstream too"
        );
        for (push_remote, push_default) in [(".", ""), ("", ".")] {
            assert_eq!(
                route(Upstream::Here, push_remote, push_default).target("topic", "origin"),
                None
            );
            assert!(
                !route(Upstream::Remote("origin/topic"), push_remote, push_default)
                    .goes_elsewhere()
            );
        }
    }

    /// The branch's own mark against the repository's, through the target
    /// as through the standing (the arms themselves: `route`'s tests).
    #[test]
    fn a_push_target_weighs_the_branchs_own_mark_first() {
        let target = |push_remote, push_default| {
            route(Upstream::Remote("origin/trunk"), push_remote, push_default)
                .target("main", "origin")
        };
        assert_eq!(target("my", "").as_deref(), Some("my/main"));
        assert_eq!(
            target("origin", "my").as_deref(),
            Some("origin/trunk"),
            "the branch's mark beats the repository's, and points back home"
        );
        assert_eq!(target("", "my").as_deref(), Some("my/main"));
    }

    /// What the counts are against is where the standing read them: the
    /// upstream, the destination's own ref, or nothing.
    #[test]
    fn a_reading_names_the_branch_its_counts_are_against() {
        fn read<'a>(
            route: PushRoute<'a>,
            upstream_counts: Option<crate::AheadBehind>,
            push_track: Option<&'a PushTrack>,
        ) -> (PushStanding, Option<&'a str>, crate::AheadBehind) {
            let read = push_reading(route, upstream_counts, push_track);
            (read.standing, read.measured, read.counts)
        }
        let counts = |ahead, behind| crate::AheadBehind { ahead, behind };
        let upstream = Upstream::Remote("origin/main");
        assert_eq!(
            read(route(upstream, "", ""), Some(counts(1, 0)), None),
            (PushStanding::Ready, Some("origin/main"), counts(1, 0))
        );
        let there = track("my", "my/main", 0, 2);
        assert_eq!(
            read(route(upstream, "", "my"), Some(counts(1, 0)), Some(&there)),
            (PushStanding::Behind, Some("my/main"), counts(0, 2)),
            "elsewhere, the destination's own ref and counts"
        );
        assert_eq!(
            read(route(upstream, "", "my"), Some(counts(1, 0)), None),
            (PushStanding::Elsewhere, None, counts(0, 0)),
            "nothing here tracks it over there: nothing counted, nothing to lease to"
        );
        assert_eq!(
            read(route(upstream, "", ""), None, None),
            (PushStanding::Publish, None, counts(0, 0)),
            "an upstream with no ref to count against"
        );
        assert_eq!(
            read(route(upstream, ".", ""), Some(counts(1, 0)), None),
            (PushStanding::StaysHere, None, counts(1, 0)),
            "marked to this repository: no remote branch to lease to"
        );
        assert_eq!(
            read(route(Upstream::Here, "", ""), Some(counts(2, 0)), None),
            (PushStanding::StaysHere, None, counts(2, 0)),
            "a branch here is no remote branch, whatever is counted against it"
        );
    }

    fn head(branch: &str, settled: bool, push_remote: Option<&str>) -> HeadPush {
        HeadPush {
            branch: branch.to_string(),
            settled,
            upstream: "origin/main".to_string(),
            upstream_counts: Some(crate::AheadBehind {
                ahead: 1,
                behind: 0,
            }),
            status: StatusPush {
                push_remote: push_remote.map(str::to_string),
                ..StatusPush::default()
            },
            ..HeadPush::default()
        }
    }

    /// HEAD alone can be on no branch or on one with no commits, and its
    /// counts can be another branch's for a moment: each stands closed or
    /// unborn ahead of the route — the moment naming no destination, the
    /// mark it holds being the last branch's.
    #[test]
    fn the_head_stands_closed_or_unborn_ahead_of_any_route() {
        let facts = |head: &HeadPush| head.facts(REMOTES, "origin");
        let ready = facts(&head("main", true, None));
        assert_eq!(
            ready,
            HeadPushFacts {
                standing: PushStanding::Ready,
                target: "origin/main".to_string(),
                measured: "origin/main".to_string(),
                counts: crate::AheadBehind {
                    ahead: 1,
                    behind: 0
                },
                upstream: "origin/main".to_string(),
            }
        );
        assert_eq!(
            facts(&head("main", false, Some("my"))),
            HeadPushFacts::default(),
            "nothing of main is known yet: `my` and the counts are the last branch's"
        );
        let detached = HeadPush {
            detached: true,
            ..head("", true, None)
        };
        assert_eq!(facts(&detached).standing, PushStanding::Closed);
        assert_eq!(facts(&detached).target, "");
        let unborn = HeadPush {
            unborn: true,
            upstream: String::new(),
            upstream_counts: None,
            ..head("main", true, None)
        };
        assert_eq!(
            (facts(&unborn).standing, facts(&unborn).target),
            (PushStanding::Unborn, "origin/main".to_string())
        );
        assert_eq!(facts(&HeadPush::default()).standing, PushStanding::Closed);
    }

    /// A read that could not tell repeats the branch's last counts
    /// (`session::Standing::push_of`), which are about where the marks sent
    /// the push then: moved since, they say nothing of the new destination,
    /// the toolbar's as a row's. Which remote they were read for is git's
    /// word, not the tracking ref's prefix.
    #[test]
    fn only_counts_on_the_marked_remote_speak_for_the_push() {
        let read = |remote: &str, tracking: &str| {
            let head = HeadPush {
                status: StatusPush {
                    push_remote: Some("my".to_string()),
                    track: Some(track(remote, tracking, 1, 1)),
                    ..StatusPush::default()
                },
                ..head("main", true, None)
            };
            let facts = head.facts(REMOTES, "origin");
            (facts.standing, facts.measured)
        };
        assert_eq!(
            read("origin", "origin/main"),
            (PushStanding::Elsewhere, String::new()),
            "the counts are another remote's"
        );
        assert_eq!(
            read("my/fork", "my/fork/main"),
            (PushStanding::Elsewhere, String::new()),
            "`my/fork` is another remote than `my`, whatever the prefix"
        );
        assert_eq!(
            read("my", "my/main"),
            (PushStanding::Diverged, "my/main".to_string()),
            "the marked remote's own counts speak"
        );
        assert_eq!(
            read("my", "copy/main"),
            (PushStanding::Diverged, "copy/main".to_string()),
            "wherever its fetch refspec puts the branch"
        );
    }

    #[test]
    fn the_head_names_its_upstream_only_where_it_is_a_remotes() {
        let here = HeadPush {
            status: StatusPush {
                upstream_here: true,
                push_default: Some("my".to_string()),
                ..StatusPush::default()
            },
            ..head("main", true, None)
        };
        let facts = here.facts(REMOTES, "origin");
        assert_eq!(facts.upstream, "", "a branch here named like origin's");
        assert_eq!(facts.target, "my/main");
        assert_eq!(
            head("main", true, Some(".")).facts(REMOTES, "origin"),
            HeadPushFacts {
                standing: PushStanding::StaysHere,
                target: String::new(),
                measured: String::new(),
                counts: crate::AheadBehind {
                    ahead: 1,
                    behind: 0
                },
                upstream: "origin/main".to_string(),
            }
        );
    }
}
