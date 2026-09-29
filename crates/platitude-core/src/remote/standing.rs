//! The pure half of a push: what one could do, decided from
//! configuration and counts already in hand.
//!
//! Nothing here runs git; [`super::push`] does, and the two must resolve
//! the same destination or the toolbar names a remote the send never goes
//! to.

/// What a push of the current branch can do, read off the last status
/// and the marked remote. The counts come from the last fetch, so a push
/// they say fits may still be refused (デザイン規約 §リモートへ送る).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushStanding {
    /// No branch here to send: detached, or nothing checked out.
    Closed,
    /// No commits yet: git refuses the send (`src refspec <branch> does
    /// not match any`).
    Unborn,
    /// Never sent, or the tracking ref is gone: where the branch goes is
    /// a question to ask.
    Publish,
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
            Self::Elsewhere => "elsewhere",
            Self::Ready => "ready",
            Self::Clean => "clean",
            Self::Behind => "behind",
            Self::Diverged => "diverged",
        }
    }
}

/// Where a mark sends the push to another remote than the upstream's
/// ([`pushes_elsewhere`]): the tracking ref of the branch over there, and
/// the branch's counts against it ([`super::push_track`], on the status
/// tick). The upstream's counts are about the other remote.
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
/// [`super::plan_current_push`]).
fn marked_remote<'a>(push_remote: &'a str, push_default: &'a str) -> &'a str {
    if push_remote.is_empty() {
        push_default
    } else {
        push_remote
    }
}

/// Whether `upstream` is a branch on `remote`.
///
/// Cut against the configured `remotes` ([`crate::refs::split_remote_ref`]);
/// an upstream on a remote the list does not know falls back to the
/// mark's exact prefix.
fn upstream_is_on<'a>(
    upstream: &'a str,
    remote: &str,
    remotes: impl IntoIterator<Item = &'a str>,
) -> bool {
    if upstream.is_empty() || remote.is_empty() {
        return false;
    }
    match crate::refs::split_remote_ref(upstream, remotes) {
        Some((on, _)) => on == remote,
        None => upstream
            .strip_prefix(remote)
            .is_some_and(|rest| rest.starts_with('/')),
    }
}

/// Where the button would send this branch, spelled `<remote>/<branch>`
/// as the toolbar shows it. Empty where there is no branch to send, or
/// nowhere at all to send it.
///
/// The pure half of [`super::plan_current_push`]'s destination, off
/// configuration the snapshot already carries — same order, same naming
/// rule.
pub fn push_target<'a>(
    branch: &str,
    upstream: &'a str,
    push_remote: &str,
    push_default: &str,
    fallback_remote: &str,
    remotes: impl IntoIterator<Item = &'a str>,
) -> String {
    if branch.is_empty() {
        return String::new();
    }
    let marked = marked_remote(push_remote, push_default);
    if marked.is_empty() {
        return match (upstream, fallback_remote) {
            ("", "") => String::new(),
            ("", fallback) => format!("{fallback}/{branch}"),
            (upstream, _) => upstream.to_string(),
        };
    }
    if upstream_is_on(upstream, marked, remotes) {
        upstream.to_string()
    } else {
        format!("{marked}/{branch}")
    }
}

/// Whether a mark sends the push to another remote than the one
/// `upstream` is on: then the upstream's counts are about a remote the
/// push does not go to, and the destination's own ([`PushTrack`]) speak
/// for it. `push_remote` is the branch's own mark and `push_default` the
/// repository's ([`marked_remote`]).
pub fn pushes_elsewhere<'a>(
    upstream: &'a str,
    push_remote: &str,
    push_default: &str,
    remotes: impl IntoIterator<Item = &'a str>,
) -> bool {
    let marked = marked_remote(push_remote, push_default);
    !marked.is_empty() && !upstream_is_on(upstream, marked, remotes)
}

/// The standing itself: the table of counts, read against the upstream,
/// or against `push_track` where the push goes elsewhere
/// ([`pushes_elsewhere`]).
#[expect(clippy::too_many_arguments)]
pub fn push_standing<'a>(
    unborn: bool,
    detached: bool,
    branch: &str,
    upstream: &'a str,
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
    if upstream.is_empty() || !upstream_tracked {
        return PushStanding::Publish;
    }
    let (ahead, behind) = if pushes_elsewhere(upstream, push_remote, push_default, remotes) {
        // git refuses the push by the destination's ref (`[rejected]
        // (non-fast-forward)`), so only that ref's counts say how it goes.
        if push_track.tracking.is_empty() {
            return PushStanding::Elsewhere;
        }
        (push_track.ahead, push_track.behind)
    } else {
        (ahead, behind)
    };
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
            false, false, "main", upstream, tracked, ahead, behind, "", marked, REMOTES, &NO_TRACK,
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
            false, false, "main", upstream, true, ahead, behind, "", marked, REMOTES, push_track,
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
                true, false, "main", "", false, 0, 0, "", "", REMOTES, &NO_TRACK
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
            upstream,
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
            upstream,
            push_remote,
            push_default,
            "origin",
            REMOTES,
        )
    }

    #[test]
    fn a_push_standing_is_read_without_sending_anything() {
        assert_eq!(
            push_standing(false, true, "", "", false, 0, 0, "", "", [], &NO_TRACK),
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
                "origin/main",
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
                "origin/main",
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
                "origin/main",
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

    /// Every arm of `plan_current_push`'s destination.
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
            push_target("main", "my/fork/main", "", "my/fork", "origin", REMOTES),
            "my/fork/main"
        );
        assert_eq!(
            push_target("main", "my/fork/main", "", "my", "origin", REMOTES),
            "my/main"
        );
        assert_eq!(
            push_target("main", "gone/main", "", "gone", "origin", REMOTES),
            "gone/main",
            "an upstream on a remote the list has lost still matches its own mark"
        );
    }

    #[test]
    fn a_push_target_says_nothing_where_there_is_nothing_to_say() {
        assert_eq!(push_target("", "", "", "", "origin", REMOTES), "");
        assert_eq!(
            push_target("main", "", "", "", "", REMOTES),
            "",
            "a repository with no remote at all has no destination to name"
        );
    }
}
