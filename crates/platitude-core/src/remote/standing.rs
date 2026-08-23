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

/// The standing itself. `remotes` are the configured remote names:
/// whether the upstream tracks the marked remote is answered by the
/// longest-name cut ([`crate::refs::split_remote_ref`]), never the first
/// slash, since a remote's own name may contain `/`. An upstream on a
/// remote the list does not know (stale config) falls back to the exact
/// prefix the mark itself gives.
#[expect(clippy::too_many_arguments)]
pub fn push_standing<'a>(
    detached: bool,
    branch: &str,
    upstream: &'a str,
    upstream_tracked: bool,
    ahead: i32,
    behind: i32,
    push_default: &str,
    remotes: impl IntoIterator<Item = &'a str>,
) -> PushStanding {
    if detached || branch.is_empty() {
        return PushStanding::Closed;
    }
    if upstream.is_empty() || !upstream_tracked {
        return PushStanding::Publish;
    }
    let marks_another = !push_default.is_empty()
        && match crate::refs::split_remote_ref(upstream, remotes) {
            Some((remote, _)) => remote != push_default,
            None => !upstream
                .strip_prefix(push_default)
                .is_some_and(|rest| rest.starts_with('/')),
        };
    if marks_another {
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

    fn standing_of(
        upstream: &str,
        tracked: bool,
        ahead: i32,
        behind: i32,
        marked: &str,
    ) -> PushStanding {
        push_standing(
            false,
            "main",
            upstream,
            tracked,
            ahead,
            behind,
            marked,
            ["origin", "my", "my/fork"],
        )
    }

    #[test]
    fn a_push_standing_is_read_without_sending_anything() {
        assert_eq!(
            push_standing(true, "", "", false, 0, 0, "", []),
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
}
