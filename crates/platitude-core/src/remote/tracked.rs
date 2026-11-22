//! A remote branch as this repository lists it.

use super::RemoteBranch;

/// A remote branch as a row here shows it — `origin/feature/x`, the
/// remote-tracking ref `refs/remotes/origin/feature/x` — with the branch
/// over there it mirrors. What a press on such a row carries.
///
/// The row's name stays whole for everything about the ref here: the row's
/// key, the ref a write copies from, what leaves the graph. The far side
/// is read off it once, as this is built, for what goes to the remote —
/// nothing downstream cuts the name again, and nothing joins the halves
/// back into it ([`RemoteBranch`] is for names given apart).
///
/// The reading is the cut at the configured remote that owns the name
/// ([`crate::refs::split_remote_ref_or_first_slash`]): what the default
/// fetch refspec mirrors (`+refs/heads/*:refs/remotes/<remote>/*`). A
/// refspec that renames (`refs/heads/main:refs/remotes/origin/trunk`) is
/// not followed: the far side comes out as the row's own half (`trunk`).
/// Where the remote has no such branch the delete sent there fails — in a
/// replace, after the new name is made; where it has one, the row's
/// commit is all that leases it. Such a configuration is exceptional and
/// gets no mechanism of its own (user decision — ask before changing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackedBranch {
    short: String,
    far: RemoteBranch,
}

impl TrackedBranch {
    /// The row listed as `short`, read against the remotes this repository
    /// has. `None` for a name no slash divides: it names no remote to
    /// address.
    pub fn listed<'a>(short: &'a str, remotes: impl IntoIterator<Item = &'a str>) -> Option<Self> {
        let (remote, branch) = crate::refs::split_remote_ref_or_first_slash(short, remotes)?;
        Some(Self {
            short: short.to_string(),
            far: RemoteBranch {
                remote: remote.to_string(),
                branch: branch.to_string(),
            },
        })
    }

    /// The name the row is listed and keyed under (`origin/feature/x`).
    pub fn short(&self) -> &str {
        &self.short
    }

    /// The remote-tracking ref itself, `refs/remotes/<short>`.
    pub fn tracking_ref(&self) -> String {
        format!("refs/remotes/{}", self.short)
    }

    /// The branch over there this row mirrors.
    pub fn far(&self) -> &RemoteBranch {
        &self.far
    }

    /// A branch called `name` on the same remote: where a replace, or a
    /// rename carried over, sends the new name.
    pub fn beside(&self, name: &str) -> RemoteBranch {
        RemoteBranch {
            remote: self.far.remote.clone(),
            branch: name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listed(short: &str, remotes: &[&str]) -> Option<TrackedBranch> {
        TrackedBranch::listed(short, remotes.iter().copied())
    }

    /// The row's name comes back as it went in, whatever the far side
    /// reads as: a remote's own name may hold a slash, and the longest
    /// configured one owns the row.
    #[test]
    fn the_row_keeps_its_name_and_the_far_side_is_cut_at_the_owning_remote() {
        let row = listed("team/fork/feature/x", &["team", "team/fork"]).expect("a remote row");
        assert_eq!(row.short(), "team/fork/feature/x");
        assert_eq!(row.tracking_ref(), "refs/remotes/team/fork/feature/x");
        assert_eq!(
            row.far(),
            &RemoteBranch {
                remote: "team/fork".to_string(),
                branch: "feature/x".to_string(),
            }
        );
        assert_eq!(row.beside("renamed").to_string(), "team/fork/renamed");
    }

    /// No configured remote owns the name — the list still loading, or
    /// refs a removed remote left: the first slash answers, so the press
    /// reaches git, which refuses loudly.
    #[test]
    fn a_row_no_configured_remote_owns_is_cut_at_its_first_slash() {
        let row = listed("gone/feature/x", &["origin"]).expect("a slash to cut at");
        assert_eq!(row.far().remote, "gone");
        assert_eq!(row.far().branch, "feature/x");
        assert_eq!(row.short(), "gone/feature/x");
    }

    #[test]
    fn a_name_no_slash_divides_is_no_remote_row() {
        assert_eq!(listed("main", &["origin"]), None);
    }
}
