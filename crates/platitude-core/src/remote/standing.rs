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

use super::route::PushRoute;

/// What a push of a branch can do, read off the last status (the branch
/// the worktree is on) or refs listing (any other) and the marked remote.
/// The counts come from the last fetch, so a push they say fits may still
/// be refused (デザイン規約 §リモートへ送る): they are trusted only to say
/// a push cannot land. With no remote at all a branch is `Publish` — the
/// question can make one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PushStanding {
    /// No branch here to send: detached, or nothing checked out.
    #[default]
    Closed,
    /// No commits yet: git refuses the send (`src refspec <branch> does
    /// not match any`).
    Unborn,
    /// Never sent, or the tracking ref is gone: where the branch goes is
    /// a question to ask.
    Publish,
    /// The push resolves to this repository (`.`, [`THIS_REPOSITORY`]):
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

/// git's name for this repository where a remote's goes — in
/// `branch.<n>.remote` (the upstream is a branch here) and in a mark alike.
/// A push there moves a branch here; it sends nothing anywhere.
pub(super) const THIS_REPOSITORY: &str = ".";

/// What a branch is measured against, as its configuration names it —
/// the toolbar's from the status tick (`PushMarks::upstream_is_here`), a
/// row's from the listing (`RefEntry::upstream_is_here`). Net:
/// `upstream_here_integration`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Upstream<'a> {
    /// Nothing is configured.
    #[default]
    None,
    /// A remote branch, `<remote>/<branch>` as the remote branches are
    /// listed — or git's own short name for an upstream fetched outside
    /// them ([`super::PushMarks::upstream_name`]).
    Remote(&'a str),
    /// A branch here (`branch.<n>.remote = .`). Told by configuration,
    /// never off a name: a branch here may be called `origin/main`.
    Here,
}

impl<'a> Upstream<'a> {
    /// From a name for it (empty where none is configured) and whether
    /// configuration puts it here.
    pub fn of(name: &'a str, here: bool) -> Self {
        match (name.is_empty(), here) {
            (true, _) => Self::None,
            (false, true) => Self::Here,
            (false, false) => Self::Remote(name),
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
    /// The tracking ref as the remote branches are listed (`fork/main`).
    pub tracking: String,
    pub counts: crate::AheadBehind,
}

/// The standing itself: the table of counts, read against the upstream
/// (`upstream_counts`), or against `push_track` where the push goes
/// elsewhere ([`PushRoute::goes_elsewhere`]). `upstream_counts` is none
/// where the upstream's ref is not there to count against — git's
/// `[gone]`, which asks where the branch goes whichever kind the upstream
/// was.
pub fn push_standing(
    unborn: bool,
    detached: bool,
    branch: &str,
    route: PushRoute<'_>,
    upstream_counts: Option<crate::AheadBehind>,
    push_track: Option<&PushTrack>,
) -> PushStanding {
    if detached || branch.is_empty() {
        return PushStanding::Closed;
    }
    // Before anything about remotes: with no ref to name, git refuses
    // every send.
    if unborn {
        return PushStanding::Unborn;
    }
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
            false,
            false,
            "main",
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
            false,
            false,
            "main",
            route(Upstream::Remote(upstream), "", marked),
            Some(crate::AheadBehind { ahead, behind }),
            push_track,
        )
    }

    fn track(tracking: &str, ahead: u32, behind: u32) -> PushTrack {
        PushTrack {
            tracking: tracking.to_string(),
            counts: crate::AheadBehind { ahead, behind },
        }
    }

    #[test]
    fn a_branch_with_no_commits_has_no_refspec_to_send() {
        assert_eq!(
            push_standing(
                true,
                false,
                "main",
                route(Upstream::None, "", ""),
                None,
                None
            ),
            PushStanding::Unborn
        );
    }

    /// [`standing_of`], with the branch's own mark set and the
    /// repository's empty.
    fn standing_marked_on_branch(upstream: &str, push_remote: &str) -> PushStanding {
        push_standing(
            false,
            false,
            "main",
            route(Upstream::Remote(upstream), push_remote, ""),
            Some(crate::AheadBehind {
                ahead: 2,
                behind: 0,
            }),
            None,
        )
    }

    #[test]
    fn a_push_standing_is_read_without_sending_anything() {
        assert_eq!(
            push_standing(
                false,
                true,
                "",
                PushRoute::new(Upstream::None, Marked::Unmarked, []),
                None,
                None
            ),
            PushStanding::Closed
        );
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
            standing_tracking("origin/main", 1, 0, "my", Some(&track("my/main", 1, 1))),
            PushStanding::Diverged,
            "git refuses the plain push there, so only the overwrite reaches"
        );
        assert_eq!(
            standing_tracking("origin/main", 1, 0, "my", Some(&track("my/main", 0, 2))),
            PushStanding::Behind
        );
        assert_eq!(
            standing_tracking("origin/main", 1, 0, "my", Some(&track("my/main", 0, 0))),
            PushStanding::Clean,
            "the destination has it all, though the upstream does not"
        );
        assert_eq!(
            standing_tracking("origin/main", 0, 0, "my", Some(&track("my/main", 3, 0))),
            PushStanding::Ready,
            "the upstream has it all, the destination does not"
        );
        assert_eq!(
            standing_tracking("origin/main", 1, 0, "origin", Some(&track("my/main", 0, 2))),
            PushStanding::Ready,
            "a mark on the upstream's own remote leaves the upstream's counts speaking"
        );
        assert_eq!(
            push_standing(
                false,
                false,
                "main",
                route(Upstream::Remote("origin/main"), "", "my"),
                None,
                Some(&track("my/main", 1, 0))
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
                false,
                false,
                "main",
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
                false,
                false,
                "main",
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
                false,
                false,
                "topic",
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
                here("origin", "", Some(&track("origin/topic", 0, 0))),
                target("origin", "")
            ),
            (PushStanding::Clean, Some("origin/topic".to_string())),
            "level with origin/topic, however far ahead of the branch here"
        );
        assert_eq!(
            here("", "my", Some(&track("my/topic", 0, 1))),
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
            push_standing(
                false,
                false,
                "topic",
                route(Upstream::Here, "", ""),
                None,
                None
            ),
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
                false,
                false,
                "topic",
                route(upstream, push_remote, push_default),
                tracked.then_some(crate::AheadBehind {
                    ahead: 1,
                    behind: 0,
                }),
                Some(&track("origin/topic", 1, 0)),
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
}
