//! Where git's order sends a push: the branch's own mark, then the
//! repository's, then what the branch tracks (git-config(5);
//! [`super::plan_push`]). The marks are read in that order once, into
//! [`Marked`], which the send matches on as well; the route weighs them
//! against the upstream as the toolbar and a branch's row name it, for the
//! standing and the label. The send works its destination out of the keys
//! themselves (`branch.<n>.remote` / `.merge`), so the label and the send
//! still spell the upstream's half twice — held together only by
//! `push::tests::the_label_names_the_remote_the_send_uses`, which walks
//! every arrangement of the keys through both.

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

    /// The remote branch it is, `None` for a branch here — which names no
    /// remote whatever its name looks like — and for none.
    pub fn remote_branch(self) -> Option<&'a str> {
        match self {
            Self::Remote(name) => Some(name),
            Self::Here | Self::None => None,
        }
    }
}

/// Which way the marks point a push, before the upstream is weighed: the
/// branch's own `branch.<n>.pushRemote` beats the repository's
/// `remote.pushDefault`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Marked<'a> {
    /// Neither is set: the upstream decides, else the caller's fallback.
    #[default]
    Unmarked,
    /// The mark names this repository (`.`, [`THIS_REPOSITORY`]): the push
    /// moves a branch here and sends nothing anywhere.
    Here,
    /// The mark names a remote. git does not check that one exists: an
    /// unknown name is taken for a URL.
    Remote(&'a str),
}

impl<'a> Marked<'a> {
    /// From the branch's own mark and the repository's, `None` where unset.
    pub fn of(push_remote: Option<&'a str>, push_default: Option<&'a str>) -> Self {
        match push_remote.or(push_default) {
            None => Self::Unmarked,
            Some(THIS_REPOSITORY) => Self::Here,
            Some(remote) => Self::Remote(remote),
        }
    }
}

/// One branch's push resolved: what it is measured against, which way the
/// marks point, and whether the two are on the same remote.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PushRoute<'a> {
    upstream: Upstream<'a>,
    marked: Marked<'a>,
    /// Whether the upstream is a branch on the marked remote — never so
    /// for a branch here, which is on no remote, nor without a remote mark.
    upstream_on_marked: bool,
}

impl<'a> PushRoute<'a> {
    /// The upstream is cut against the configured `remotes`
    /// ([`crate::refs::split_remote_ref`]); one on a remote the list does
    /// not know falls back to the mark's exact prefix.
    pub fn new(
        upstream: Upstream<'a>,
        marked: Marked<'a>,
        remotes: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        let upstream_on_marked = match (upstream, marked) {
            (Upstream::Remote(upstream), Marked::Remote(remote)) => {
                match crate::refs::split_remote_ref(upstream, remotes) {
                    Some((on, _)) => on == remote,
                    None => upstream
                        .strip_prefix(remote)
                        .is_some_and(|rest| rest.starts_with('/')),
                }
            }
            _ => false,
        };
        Self {
            upstream,
            marked,
            upstream_on_marked,
        }
    }

    pub fn upstream(self) -> Upstream<'a> {
        self.upstream
    }

    pub fn marked(self) -> Marked<'a> {
        self.marked
    }

    /// The push resolves to this repository (`PushStanding::StaysHere`):
    /// marked to `.` itself, or marked to nothing with the upstream here.
    pub fn stays_here(self) -> bool {
        match self.marked {
            Marked::Here => true,
            Marked::Unmarked => self.upstream == Upstream::Here,
            Marked::Remote(_) => false,
        }
    }

    /// A mark sends the push to another remote than the one the upstream
    /// is on — a branch here being on none: then the upstream's counts are
    /// about somewhere the push does not go, and the destination's own
    /// ([`super::PushTrack`]) speak for it. A mark naming this repository
    /// sends it to no remote, so to none with counts.
    pub fn goes_elsewhere(self) -> bool {
        matches!(self.marked, Marked::Remote(_)) && !self.upstream_on_marked
    }

    /// Whether the destination's own counts are the ones that speak for the
    /// push, so a read has to ask for them ([`super::push_track`] on the
    /// status tick, [`super::push_tracks`] on the refs read): the upstream
    /// is there to count against (`upstream_counted`) — with none, the push
    /// asks where it goes and nothing is counted — and the push goes
    /// elsewhere ([`Self::goes_elsewhere`]).
    pub fn counts_elsewhere(self, upstream_counted: bool) -> bool {
        upstream_counted && self.upstream != Upstream::None && self.goes_elsewhere()
    }

    /// Where the push sends `branch`, spelled `<remote>/<branch>` as the
    /// toolbar and the branch's row show it. `None` where there is no
    /// branch to send, or nowhere at all to send it — a branch measured
    /// against one here and marked to no remote, and a mark naming this
    /// repository, among them. `fallback_remote` is where a branch that
    /// tracks nothing, marked nowhere, goes.
    ///
    /// The branch goes under its own name wherever it is not going to the
    /// remote it tracks, as git's triangular-push refspec does.
    pub fn target(self, branch: &str, fallback_remote: &str) -> Option<String> {
        if branch.is_empty() {
            return None;
        }
        match (self.marked, self.upstream) {
            (Marked::Here, _) | (Marked::Unmarked, Upstream::Here) => None,
            (Marked::Unmarked, Upstream::Remote(upstream)) => Some(upstream.to_string()),
            (Marked::Unmarked, Upstream::None) if fallback_remote.is_empty() => None,
            (Marked::Unmarked, Upstream::None) => Some(format!("{fallback_remote}/{branch}")),
            (Marked::Remote(_), Upstream::Remote(upstream)) if self.upstream_on_marked => {
                Some(upstream.to_string())
            }
            (Marked::Remote(remote), _) => Some(format!("{remote}/{branch}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REMOTES: [&str; 3] = ["origin", "my", "my/fork"];

    fn route(upstream: Upstream<'static>, marked: Marked<'static>) -> PushRoute<'static> {
        PushRoute::new(upstream, marked, REMOTES)
    }

    #[test]
    fn a_branchs_own_mark_beats_the_repositorys() {
        assert_eq!(Marked::of(None, None), Marked::Unmarked);
        assert_eq!(Marked::of(Some("my"), None), Marked::Remote("my"));
        assert_eq!(Marked::of(None, Some("my")), Marked::Remote("my"));
        assert_eq!(
            Marked::of(Some("origin"), Some("my")),
            Marked::Remote("origin")
        );
        assert_eq!(Marked::of(Some("."), Some("my")), Marked::Here);
        assert_eq!(
            Marked::of(Some("my"), Some(".")),
            Marked::Remote("my"),
            "a `.` the branch's own mark overrides points nowhere"
        );
        assert_eq!(Marked::of(None, Some(".")), Marked::Here);
    }

    /// `my/fork/main` lives on `my/fork`: a mark on `my` is another remote
    /// even though the name is a prefix of the upstream.
    #[test]
    fn the_upstream_is_cut_against_the_configured_remotes() {
        let on = |upstream, remote| {
            !route(Upstream::Remote(upstream), Marked::Remote(remote)).goes_elsewhere()
        };
        assert!(on("origin/main", "origin"));
        assert!(!on("origin/main", "my"));
        assert!(!on("my/fork/main", "my"));
        assert!(on("my/fork/main", "my/fork"));
        assert!(on("gone/main", "gone"), "unknown to the list: exact prefix");
        assert!(!on("gone/main", "origin"));
        assert!(!on("gonemain", "gone"), "a prefix must end at a slash");
    }

    #[test]
    fn a_push_stays_here_by_a_dot_or_by_an_upstream_here_left_unmarked() {
        assert!(route(Upstream::Here, Marked::Unmarked).stays_here());
        assert!(route(Upstream::Here, Marked::Here).stays_here());
        assert!(route(Upstream::Remote("origin/main"), Marked::Here).stays_here());
        assert!(route(Upstream::None, Marked::Here).stays_here());
        assert!(!route(Upstream::Here, Marked::Remote("origin")).stays_here());
        assert!(!route(Upstream::Remote("origin/main"), Marked::Unmarked).stays_here());
        assert!(!route(Upstream::None, Marked::Unmarked).stays_here());
    }

    /// Only a remote mark sends the push somewhere the upstream is not: a
    /// branch here is on no remote, and `.` is no remote at all.
    #[test]
    fn only_a_remote_mark_sends_the_push_elsewhere() {
        assert!(route(Upstream::Here, Marked::Remote("origin")).goes_elsewhere());
        assert!(route(Upstream::None, Marked::Remote("origin")).goes_elsewhere());
        assert!(!route(Upstream::Here, Marked::Unmarked).goes_elsewhere());
        assert!(!route(Upstream::Remote("origin/main"), Marked::Here).goes_elsewhere());
        assert!(!route(Upstream::Remote("origin/main"), Marked::Unmarked).goes_elsewhere());
    }

    /// The destination's own counts are asked for only where they speak:
    /// an upstream there to count against, and the push going elsewhere.
    #[test]
    fn only_a_counted_upstream_with_the_push_elsewhere_reads_the_destination() {
        let elsewhere = route(Upstream::Remote("origin/main"), Marked::Remote("my"));
        assert!(elsewhere.counts_elsewhere(true));
        assert!(
            !elsewhere.counts_elsewhere(false),
            "no count against the upstream: the push asks where it goes"
        );
        assert!(
            !route(Upstream::Remote("origin/main"), Marked::Remote("origin"))
                .counts_elsewhere(true)
        );
        assert!(route(Upstream::Here, Marked::Remote("origin")).counts_elsewhere(true));
        assert!(
            !route(Upstream::None, Marked::Remote("origin")).counts_elsewhere(true),
            "nothing configured: a first push, nothing counted"
        );
    }

    /// Every arm of `plan_push`'s destination.
    #[test]
    fn a_target_follows_gits_own_order() {
        let target = |upstream, marked| route(upstream, marked).target("main", "origin");
        let named = |name: &str| Some(name.to_string());
        assert_eq!(
            target(Upstream::Remote("origin/trunk"), Marked::Unmarked),
            named("origin/trunk"),
            "the upstream names the branch on its own remote"
        );
        assert_eq!(
            target(Upstream::None, Marked::Unmarked),
            named("origin/main"),
            "nothing tracked and nothing marked falls back to the caller's remote"
        );
        assert_eq!(
            target(Upstream::Remote("origin/trunk"), Marked::Remote("my")),
            named("my/main"),
            "anywhere else the branch goes under its own name"
        );
        assert_eq!(
            target(Upstream::Remote("origin/trunk"), Marked::Remote("origin")),
            named("origin/trunk"),
            "a mark pointing back home keeps the upstream's name"
        );
        assert_eq!(
            target(Upstream::None, Marked::Remote("my")),
            named("my/main"),
            "a branch that tracks nothing goes to the mark, not the fallback"
        );
        assert_eq!(
            target(Upstream::Remote("my/fork/main"), Marked::Remote("my")),
            named("my/main")
        );
        assert_eq!(
            target(Upstream::Remote("my/fork/main"), Marked::Remote("my/fork")),
            named("my/fork/main")
        );
        assert_eq!(
            target(Upstream::Remote("gone/main"), Marked::Remote("gone")),
            named("gone/main"),
            "an upstream on a remote the list has lost still matches its own mark"
        );
        assert_eq!(
            target(Upstream::Here, Marked::Remote("my")),
            named("my/main"),
            "a branch here goes to the mark under its own name"
        );
    }

    #[test]
    fn a_target_says_nothing_where_there_is_nothing_to_say() {
        assert_eq!(
            route(Upstream::None, Marked::Unmarked).target("", "origin"),
            None
        );
        assert_eq!(
            route(Upstream::None, Marked::Unmarked).target("main", ""),
            None,
            "a repository with no remote at all has no destination to name"
        );
        assert_eq!(
            route(Upstream::Here, Marked::Unmarked).target("main", "origin"),
            None,
            "marked to no remote: nowhere to send it, not even the fallback"
        );
        for upstream in [
            Upstream::Here,
            Upstream::Remote("origin/main"),
            Upstream::None,
        ] {
            assert_eq!(route(upstream, Marked::Here).target("main", "origin"), None);
        }
    }
}
