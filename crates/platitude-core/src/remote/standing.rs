//! The pure half of a push: what one could do, decided from
//! configuration and counts already in hand.
//!
//! Nothing here runs git. The half that does is [`super::push`], and the
//! two resolve the same destination — one worked out here and one worked
//! out there have to agree, or the toolbar names a remote the send never
//! goes to.

/// What a push of the current branch can do, read off the last status
/// and the marked remote — nothing is sent to find out. The counts come
/// from the last fetch, so they prove the negative only: a push may
/// still be refused when they say it fits (デザイン規約 §リモートへ送る).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushStanding {
    /// No branch here to send: detached, or nothing checked out.
    Closed,
    /// Never sent, or the tracking ref is gone: where the branch goes is
    /// a question rather than something to look up.
    Publish,
    /// The push goes to the marked remote, which is not the one this
    /// branch tracks — the counts are about somewhere else and say
    /// nothing at all.
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
            Self::Publish => "publish",
            Self::Elsewhere => "elsewhere",
            Self::Ready => "ready",
            Self::Clean => "clean",
            Self::Behind => "behind",
            Self::Diverged => "diverged",
        }
    }
}

/// Which remote the marks send this branch to, empty where neither is
/// set and what the branch tracks decides instead.
///
/// **The branch's own mark beats the repository's** — git's order, and
/// the first two steps of [`super::plan_current_push`]'s
/// (git-config(5); 実測 2.55). Reading only the repository's is how a
/// fork workflow, which marks the branch, gets told about the repository
/// it forked from.
fn marked_remote<'a>(push_remote: &'a str, push_default: &'a str) -> &'a str {
    if push_remote.is_empty() {
        push_default
    } else {
        push_remote
    }
}

/// Whether `upstream` is a branch on `remote`.
///
/// `remotes` are the configured remote names: the cut is the longest-name
/// one ([`crate::refs::split_remote_ref`]), never the first slash, since a
/// remote's own name may contain `/`. An upstream on a remote the list
/// does not know (stale config) falls back to the exact prefix the mark
/// itself gives.
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
/// configuration the snapshot already carries — **same order, same
/// reasons**: the branch's own `pushRemote`, then `remote.pushDefault`,
/// then what the branch tracks, and only then `fallback_remote`.
///
/// The branch keeps the name its upstream gives it only where it is going
/// to the remote it tracks. Anywhere else it goes under its own name, as
/// the refspec git builds for a triangular push does (実測 — the same
/// rule [`super::plan_current_push`] applies to `branch.<name>.merge`).
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

/// The standing itself. `push_remote` is the branch's own mark and
/// `push_default` the repository's; they are weighed in that order
/// ([`marked_remote`]), and whether the winner is the remote the branch
/// tracks is [`upstream_is_on`]'s answer.
#[expect(clippy::too_many_arguments)]
pub fn push_standing<'a>(
    detached: bool,
    branch: &str,
    upstream: &'a str,
    upstream_tracked: bool,
    ahead: i32,
    behind: i32,
    push_remote: &str,
    push_default: &str,
    remotes: impl IntoIterator<Item = &'a str>,
) -> PushStanding {
    if detached || branch.is_empty() {
        return PushStanding::Closed;
    }
    if upstream.is_empty() || !upstream_tracked {
        return PushStanding::Publish;
    }
    let marked = marked_remote(push_remote, push_default);
    if !marked.is_empty() && !upstream_is_on(upstream, marked, remotes) {
        return PushStanding::Elsewhere;
    }
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

    fn standing_of(
        upstream: &str,
        tracked: bool,
        ahead: i32,
        behind: i32,
        marked: &str,
    ) -> PushStanding {
        push_standing(
            false, "main", upstream, tracked, ahead, behind, "", marked, REMOTES,
        )
    }

    /// The same, with the branch's own mark set instead of the
    /// repository's.
    fn standing_marked_on_branch(upstream: &str, push_remote: &str) -> PushStanding {
        push_standing(
            false,
            "main",
            upstream,
            true,
            2,
            0,
            push_remote,
            "",
            REMOTES,
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
            push_standing(true, "", "", false, 0, 0, "", "", []),
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

    #[test]
    fn a_marked_remote_takes_the_push_away_from_the_counts() {
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
        // An upstream on a remote the list does not know: the exact
        // prefix of the mark's own name is all there is to go by.
        assert_eq!(
            standing_of("gone/main", true, 1, 0, "gone"),
            PushStanding::Ready
        );
        assert_eq!(
            standing_of("gone/main", true, 1, 0, "origin"),
            PushStanding::Elsewhere
        );
    }

    /// The mark a fork workflow actually sets. It is read the same way
    /// the repository's is — and it is read *first*, so a branch marked
    /// back at the remote it tracks stays where the counts speak even
    /// while the repository points somewhere else.
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
                "main",
                "origin/main",
                true,
                2,
                0,
                "origin",
                "my",
                REMOTES
            ),
            PushStanding::Ready,
            "the branch's mark wins over the repository's, so the counts still speak"
        );
        assert_eq!(
            push_standing(
                false,
                "main",
                "origin/main",
                true,
                2,
                0,
                "my",
                "origin",
                REMOTES
            ),
            PushStanding::Elsewhere,
            "and it wins the other way round too"
        );
    }

    /// Where the toolbar says the push is going. Every arm of
    /// `plan_current_push`'s destination, with nothing sent.
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

    /// The name cut, on the destination as much as on the standing: a
    /// remote called `my/fork` owns `my/fork/main`, and one called `my`
    /// does not — the same reading `push_standing` makes.
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

    /// Nothing to send, and nowhere to send it: both are said with an
    /// empty string rather than a guess.
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
