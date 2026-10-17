//! The pure half of a push: what one could do, decided from
//! configuration and counts already in hand.
//!
//! Nothing here runs git; [`super::push`] does, and the two must resolve
//! the same destination or the toolbar names a remote the send never goes
//! to.

/// What a push of a branch can do, read off the last status (the branch
/// the worktree is on) or refs listing (any other) and the marked remote.
/// The counts come from the last fetch, so a push they say fits may still
/// be refused (デザイン規約 §リモートへ送る).
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
    /// anywhere, so none is offered.
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

/// What a branch is measured against, as its configuration names it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Upstream<'a> {
    /// Nothing is configured.
    #[default]
    None,
    /// A remote branch, `<remote>/<branch>` as the remote branches are
    /// listed.
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
/// ([`pushes_elsewhere`]): the tracking ref of the branch over there, and
/// the branch's counts against it ([`super::push_track`] on the status
/// tick, [`super::push_tracks`] on the refs read for the other branches).
/// The upstream's counts are about the other remote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PushTrack {
    /// The tracking ref as the remote branches are listed (`fork/main`).
    /// Empty where nothing here tracks the branch over there: never
    /// fetched, or no fetch refspec of that remote takes it in.
    pub tracking: String,
    pub ahead: i32,
    pub behind: i32,
}

/// Which remote the marks send this branch to, empty where neither is
/// set. The branch's own mark beats the repository's (git's order, as in
/// [`super::plan_push`]).
fn marked_remote<'a>(push_remote: &'a str, push_default: &'a str) -> &'a str {
    if push_remote.is_empty() {
        push_default
    } else {
        push_remote
    }
}

/// Whether `upstream` is a branch on `remote` — never so for a branch
/// here, which is on no remote.
///
/// Cut against the configured `remotes` ([`crate::refs::split_remote_ref`]);
/// an upstream on a remote the list does not know falls back to the
/// mark's exact prefix.
fn upstream_is_on<'a>(
    upstream: Upstream<'a>,
    remote: &str,
    remotes: impl IntoIterator<Item = &'a str>,
) -> bool {
    let Upstream::Remote(upstream) = upstream else {
        return false;
    };
    if remote.is_empty() {
        return false;
    }
    match crate::refs::split_remote_ref(upstream, remotes) {
        Some((on, _)) => on == remote,
        None => upstream
            .strip_prefix(remote)
            .is_some_and(|rest| rest.starts_with('/')),
    }
}

/// Where a push would send this branch, spelled `<remote>/<branch>` as
/// the toolbar and the branch's row show it. Empty where there is no
/// branch to send, or nowhere at all to send it — a branch measured
/// against one here and marked to no remote, and a mark naming this
/// repository, among them.
///
/// The pure half of [`super::plan_push`]'s destination, off configuration
/// the snapshot already carries — same order, same naming rule.
pub fn push_target<'a>(
    branch: &str,
    upstream: Upstream<'a>,
    push_remote: &str,
    push_default: &str,
    fallback_remote: &str,
    remotes: impl IntoIterator<Item = &'a str>,
) -> String {
    if branch.is_empty() {
        return String::new();
    }
    let marked = marked_remote(push_remote, push_default);
    if marked == THIS_REPOSITORY {
        return String::new();
    }
    if marked.is_empty() {
        return match upstream {
            Upstream::Remote(upstream) => upstream.to_string(),
            Upstream::Here => String::new(),
            Upstream::None if fallback_remote.is_empty() => String::new(),
            Upstream::None => format!("{fallback_remote}/{branch}"),
        };
    }
    match upstream {
        Upstream::Remote(name) if upstream_is_on(upstream, marked, remotes) => name.to_string(),
        _ => format!("{marked}/{branch}"),
    }
}

/// Whether a mark sends the push to another remote than the one
/// `upstream` is on — a branch here being on none: then the upstream's
/// counts are about somewhere the push does not go, and the destination's
/// own ([`PushTrack`]) speak for it. `push_remote` is the branch's own
/// mark and `push_default` the repository's ([`marked_remote`]). A mark
/// naming this repository sends it to no remote, so to none with counts.
pub fn pushes_elsewhere<'a>(
    upstream: Upstream<'a>,
    push_remote: &str,
    push_default: &str,
    remotes: impl IntoIterator<Item = &'a str>,
) -> bool {
    let marked = marked_remote(push_remote, push_default);
    !marked.is_empty() && marked != THIS_REPOSITORY && !upstream_is_on(upstream, marked, remotes)
}

/// The standing itself: the table of counts, read against the upstream,
/// or against `push_track` where the push goes elsewhere
/// ([`pushes_elsewhere`]). `upstream_tracked` is whether its ref is there
/// to count against — false for git's `[gone]`, which asks where the
/// branch goes whichever kind the upstream was.
#[expect(clippy::too_many_arguments)]
pub fn push_standing<'a>(
    unborn: bool,
    detached: bool,
    branch: &str,
    upstream: Upstream<'a>,
    upstream_tracked: bool,
    ahead: i32,
    behind: i32,
    push_remote: &str,
    push_default: &str,
    remotes: impl IntoIterator<Item = &'a str>,
    push_track: &PushTrack,
) -> PushStanding {
    if detached || branch.is_empty() {
        return PushStanding::Closed;
    }
    // Before anything about remotes: with no ref to name, git refuses
    // every send.
    if unborn {
        return PushStanding::Unborn;
    }
    if upstream == Upstream::None || !upstream_tracked {
        return PushStanding::Publish;
    }
    // Resolved to this repository: marked to `.` itself, or to nothing with
    // the upstream here.
    let marked = marked_remote(push_remote, push_default);
    if marked == THIS_REPOSITORY || (marked.is_empty() && upstream == Upstream::Here) {
        return PushStanding::StaysHere;
    }
    if pushes_elsewhere(upstream, push_remote, push_default, remotes) {
        return marked_standing(push_track);
    }
    counted(ahead, behind)
}

/// The standing where a mark alone says where the push goes — to another
/// remote than the upstream's, or at all, the upstream being a branch
/// here. git refuses the push by the destination's ref (`[rejected]
/// (non-fast-forward)`), so only that ref's counts say how it goes, and
/// before that ref is fetched none do.
fn marked_standing(push_track: &PushTrack) -> PushStanding {
    if push_track.tracking.is_empty() {
        return PushStanding::Elsewhere;
    }
    counted(push_track.ahead, push_track.behind)
}

/// The table of counts.
fn counted(ahead: i32, behind: i32) -> PushStanding {
    if behind > 0 {
        return if ahead > 0 {
            PushStanding::Diverged
        } else {
            PushStanding::Behind
        };
    }
    if ahead > 0 {
        PushStanding::Ready
    } else {
        PushStanding::Clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REMOTES: [&str; 3] = ["origin", "my", "my/fork"];

    /// Nothing here tracks the branch where the push goes.
    const NO_TRACK: PushTrack = PushTrack {
        tracking: String::new(),
        ahead: 0,
        behind: 0,
    };

    fn standing_of(
        upstream: &str,
        tracked: bool,
        ahead: i32,
        behind: i32,
        marked: &str,
    ) -> PushStanding {
        push_standing(
            false,
            false,
            "main",
            Upstream::of(upstream, false),
            tracked,
            ahead,
            behind,
            "",
            marked,
            REMOTES,
            &NO_TRACK,
        )
    }

    /// [`standing_of`] on a tracked upstream, with the destination's own
    /// counts beside the upstream's.
    fn standing_tracking(
        upstream: &str,
        ahead: i32,
        behind: i32,
        marked: &str,
        push_track: &PushTrack,
    ) -> PushStanding {
        push_standing(
            false,
            false,
            "main",
            Upstream::Remote(upstream),
            true,
            ahead,
            behind,
            "",
            marked,
            REMOTES,
            push_track,
        )
    }

    fn track(tracking: &str, ahead: i32, behind: i32) -> PushTrack {
        PushTrack {
            tracking: tracking.to_string(),
            ahead,
            behind,
        }
    }

    #[test]
    fn a_branch_with_no_commits_has_no_refspec_to_send() {
        assert_eq!(
            push_standing(
                true,
                false,
                "main",
                Upstream::None,
                false,
                0,
                0,
                "",
                "",
                REMOTES,
                &NO_TRACK
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
            Upstream::Remote(upstream),
            true,
            2,
            0,
            push_remote,
            "",
            REMOTES,
            &NO_TRACK,
        )
    }

    fn target_of(upstream: &str, push_remote: &str, push_default: &str) -> String {
        push_target(
            "main",
            Upstream::of(upstream, false),
            push_remote,
            push_default,
            "origin",
            REMOTES,
        )
    }

    #[test]
    fn a_push_standing_is_read_without_sending_anything() {
        assert_eq!(
            push_standing(
                false,
                true,
                "",
                Upstream::None,
                false,
                0,
                0,
                "",
                "",
                [],
                &NO_TRACK
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
            standing_tracking("origin/main", 1, 0, "my", &track("my/main", 1, 1)),
            PushStanding::Diverged,
            "git refuses the plain push there, so only the overwrite reaches"
        );
        assert_eq!(
            standing_tracking("origin/main", 1, 0, "my", &track("my/main", 0, 2)),
            PushStanding::Behind
        );
        assert_eq!(
            standing_tracking("origin/main", 1, 0, "my", &track("my/main", 0, 0)),
            PushStanding::Clean,
            "the destination has it all, though the upstream does not"
        );
        assert_eq!(
            standing_tracking("origin/main", 0, 0, "my", &track("my/main", 3, 0)),
            PushStanding::Ready,
            "the upstream has it all, the destination does not"
        );
        assert_eq!(
            standing_tracking("origin/main", 1, 0, "origin", &track("my/main", 0, 2)),
            PushStanding::Ready,
            "a mark on the upstream's own remote leaves the upstream's counts speaking"
        );
        assert_eq!(
            push_standing(
                false,
                false,
                "main",
                Upstream::Remote("origin/main"),
                false,
                1,
                0,
                "",
                "my",
                REMOTES,
                &track("my/main", 1, 0)
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
                Upstream::Remote("origin/main"),
                true,
                2,
                0,
                "origin",
                "my",
                REMOTES,
                &NO_TRACK
            ),
            PushStanding::Ready,
            "the branch's mark wins over the repository's, so the counts still speak"
        );
        assert_eq!(
            push_standing(
                false,
                false,
                "main",
                Upstream::Remote("origin/main"),
                true,
                2,
                0,
                "my",
                "origin",
                REMOTES,
                &NO_TRACK
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
        let here = |push_remote: &str, push_default: &str, push_track: &PushTrack| {
            push_standing(
                false,
                false,
                "topic",
                Upstream::Here,
                true,
                2,
                0,
                push_remote,
                push_default,
                REMOTES,
                push_track,
            )
        };
        let target = |push_remote: &str, push_default: &str| {
            push_target(
                "topic",
                Upstream::of("origin/base", true),
                push_remote,
                push_default,
                "origin",
                REMOTES,
            )
        };
        assert_eq!(
            (here("", "", &NO_TRACK), target("", "")),
            (PushStanding::StaysHere, String::new()),
            "marked to no remote: nowhere to send it, not even the fallback"
        );
        assert_eq!(
            (
                here("origin", "", &track("origin/topic", 0, 0)),
                target("origin", "")
            ),
            (PushStanding::Clean, "origin/topic".to_string()),
            "level with origin/topic, however far ahead of the branch here"
        );
        assert_eq!(
            here("", "my", &track("my/topic", 0, 1)),
            PushStanding::Behind,
            "the repository's mark sends it just as well"
        );
        assert_eq!(target("", "my"), "my/topic");
        assert_eq!(
            here("origin", "", &NO_TRACK),
            PushStanding::Elsewhere,
            "nothing here tracks origin/topic yet"
        );
        assert_eq!(
            push_standing(
                false,
                false,
                "topic",
                Upstream::Here,
                false,
                0,
                0,
                "",
                "",
                REMOTES,
                &NO_TRACK
            ),
            PushStanding::Publish,
            "gone, it asks where the branch goes, as any gone upstream does"
        );
        assert!(pushes_elsewhere(Upstream::Here, "", "origin", REMOTES));
        assert!(!pushes_elsewhere(Upstream::Here, "", "", REMOTES));
    }

    /// `.` is git's name for this repository: a mark set to it sends the
    /// push to no remote, whatever the upstream — while a branch with
    /// nothing to count against still asks where it goes, the answer
    /// naming the remote itself.
    #[test]
    fn a_mark_naming_this_repository_sends_nowhere() {
        let with = |upstream: Upstream<'static>, tracked: bool, push_remote: &str, push_default| {
            push_standing(
                false,
                false,
                "topic",
                upstream,
                tracked,
                1,
                0,
                push_remote,
                push_default,
                REMOTES,
                &track("origin/topic", 1, 0),
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
                push_target(
                    "topic",
                    Upstream::Here,
                    push_remote,
                    push_default,
                    "origin",
                    REMOTES
                ),
                ""
            );
            assert!(!pushes_elsewhere(
                Upstream::Remote("origin/topic"),
                push_remote,
                push_default,
                REMOTES
            ));
        }
    }

    /// Every arm of `plan_push`'s destination.
    #[test]
    fn a_push_target_follows_gits_own_order() {
        assert_eq!(target_of("origin/main", "", ""), "origin/main");
        assert_eq!(
            target_of("", "", ""),
            "origin/main",
            "nothing tracked and nothing marked falls back to the caller's remote"
        );
        assert_eq!(
            target_of("origin/trunk", "", ""),
            "origin/trunk",
            "the upstream names the branch on its own remote"
        );
        assert_eq!(
            target_of("origin/trunk", "", "my"),
            "my/main",
            "anywhere else the branch goes under its own name"
        );
        assert_eq!(
            target_of("origin/trunk", "my", ""),
            "my/main",
            "the branch's own mark says so just as well"
        );
        assert_eq!(
            target_of("origin/trunk", "origin", "my"),
            "origin/trunk",
            "the branch's mark beats the repository's, and points back home"
        );
        assert_eq!(
            target_of("", "my", ""),
            "my/main",
            "a branch that tracks nothing goes to the mark, not the fallback"
        );
    }

    #[test]
    fn a_push_target_cuts_the_remote_by_name() {
        assert_eq!(
            push_target(
                "main",
                Upstream::Remote("my/fork/main"),
                "",
                "my/fork",
                "origin",
                REMOTES
            ),
            "my/fork/main"
        );
        assert_eq!(
            push_target(
                "main",
                Upstream::Remote("my/fork/main"),
                "",
                "my",
                "origin",
                REMOTES
            ),
            "my/main"
        );
        assert_eq!(
            push_target(
                "main",
                Upstream::Remote("gone/main"),
                "",
                "gone",
                "origin",
                REMOTES
            ),
            "gone/main",
            "an upstream on a remote the list has lost still matches its own mark"
        );
    }

    #[test]
    fn a_push_target_says_nothing_where_there_is_nothing_to_say() {
        assert_eq!(
            push_target("", Upstream::None, "", "", "origin", REMOTES),
            ""
        );
        assert_eq!(
            push_target("main", Upstream::None, "", "", "", REMOTES),
            "",
            "a repository with no remote at all has no destination to name"
        );
    }
}
