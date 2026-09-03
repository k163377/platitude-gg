//! Whether commits about to be rewritten are already on a remote.
//!
//! Rewriting published history is legal and sometimes right, so this only
//! answers the question — the warning and the decision belong to the UI
//! (デザイン規約「push 済みの範囲は尋ねずに言う」).
//!
//! "Published" means reachable from some remote-tracking ref, which is only
//! as fresh as the last fetch. A repository with no remotes has nothing
//! published, so nothing to warn about.
//!
//! **Two ways to the same answer, and which one a caller takes is about
//! *when* it needs it.** [`state_of`] asks git and counts, which is a
//! process away; [`range_rewrites_published`] reads it off the rows the
//! walk already marked (`session::published`), which a menu opening on a
//! row has in the same frame (デザイン規約 §行が読む答えはどこから来るか).

use std::collections::HashSet;
use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};

/// How much of a range is already on a remote.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PublishState {
    /// Commits in the range.
    pub total: u32,
    /// Of those, the ones no remote-tracking ref reaches.
    pub unpublished: u32,
}

impl PublishState {
    /// Commits in the range that a remote already has.
    pub fn published(&self) -> u32 {
        self.total.saturating_sub(self.unpublished)
    }

    /// True when rewriting the range would rewrite published history.
    pub fn rewrites_published(&self) -> bool {
        self.published() > 0
    }
}

/// The range naming exactly one commit (`<rev>^!`), for an amend warning.
pub fn only(rev: &str) -> String {
    format!("{rev}^!")
}

/// One walked row, as the range question reads it: its id, the parents
/// the walk sifted, and the mark that walk left (`session::LogRow`).
#[derive(Debug, Clone, Copy)]
pub struct WalkedRow<'a> {
    pub oid: Oid,
    pub parents: &'a [Oid],
    pub published: bool,
}

/// Whether `onto..head` holds a commit some remote already has — the
/// `rewrites pushed commits` note on a `rebase` row, answered off rows
/// already on screen instead of two `git rev-list` runs.
///
/// `rows` is the walk's own order, **children before every parent**
/// (`--date-order`). That is what makes one pass enough: by the time a
/// row is reached, every commit that could have carried a side's colour
/// down to it has been through here, so its two answers are final and
/// the first hit can return.
///
/// **A row's own mark cannot answer this.** `published` is closed under
/// ancestors, so for a range with no merge in it the answer is the
/// oldest row's mark — but a merge brings in a side whose mark says
/// nothing about the first parent's, and the range is then a set rather
/// than a stretch.
///
/// **The window is the answer**, the same stance the row marks take
/// (`session::published`) — and the two ends fall out of it differently.
/// A `head` no row carries colours nothing, so nothing is in the range:
/// `false`. An `onto` no row carries colours nothing either, which
/// leaves the whole of `head`'s reach in the range — and that is the
/// true answer for the shape it happens in, a name older than the window
/// (`--date-order` emits no parent before its children, so a commit the
/// walk stopped short of has none of its ancestors drawn either).
pub fn range_rewrites_published<'a>(
    rows: impl IntoIterator<Item = WalkedRow<'a>>,
    head: Oid,
    onto: Oid,
) -> bool {
    // Ids waiting for the walk to reach them, one set per side —
    // bounded by the open lanes rather than by the window, the way the
    // walk's own marking is (`session::published::PublishMarks`).
    let mut from_head: HashSet<Oid> = HashSet::from([head]);
    let mut from_onto: HashSet<Oid> = HashSet::from([onto]);
    for row in rows {
        let in_head = from_head.remove(&row.oid);
        // Both sides are asked, and both carry on down: a commit the
        // rebase would leave alone is one `onto` reaches, however many
        // ways `head` also reaches it.
        let in_onto = from_onto.remove(&row.oid);
        if in_head {
            from_head.extend(row.parents.iter().copied());
        }
        if in_onto {
            from_onto.extend(row.parents.iter().copied());
        }
        if in_head && !in_onto && row.published {
            return true;
        }
    }
    false
}

/// Counts a revision range and the part of it no remote has.
///
/// `range` is anything `git rev-list` accepts — `origin/main..HEAD` for a
/// rebase, [`only`] for an amend.
pub async fn state_of(
    executor: &GitExecutor,
    workdir: &Path,
    range: &str,
    cancel: &CancellationToken,
) -> Result<PublishState, GitError> {
    let total = count(executor, workdir, range, false, cancel).await?;
    if total == 0 {
        return Ok(PublishState::default());
    }
    let unpublished = count(executor, workdir, range, true, cancel).await?;
    Ok(PublishState { total, unpublished })
}

async fn count(
    executor: &GitExecutor,
    workdir: &Path,
    range: &str,
    exclude_remotes: bool,
    cancel: &CancellationToken,
) -> Result<u32, GitError> {
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .args(["rev-list", "--count", range]);
    if exclude_remotes {
        cmd = cmd.args(["--not", "--remotes"]);
    }
    let out = executor.run(cmd, cancel).await?;
    let text = out.stdout_utf8();
    text.trim().parse().map_err(|_| GitError::UnexpectedOutput {
        command: format!("git rev-list --count {range}"),
        message: text.trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_is_the_remainder() {
        let state = PublishState {
            total: 5,
            unpublished: 2,
        };
        assert_eq!(state.published(), 3);
        assert!(state.rewrites_published());
    }

    #[test]
    fn a_wholly_local_range_needs_no_warning() {
        let state = PublishState {
            total: 3,
            unpublished: 3,
        };
        assert_eq!(state.published(), 0);
        assert!(!state.rewrites_published());
        assert!(!PublishState::default().rewrites_published());
    }

    #[test]
    fn single_commit_range_uses_the_commit_only_notation() {
        assert_eq!(only("HEAD"), "HEAD^!");
    }

    fn oid(n: u8) -> Oid {
        let hex = format!("{n:02x}").repeat(20);
        // Test-only helper; the input is always valid hex.
        Oid::from_hex_str(&hex).unwrap()
    }

    /// A window of rows in walk order — `(id, parents, published)`, newest
    /// first, the shape the graph hands over.
    struct Window(Vec<(u8, Vec<u8>, bool)>);

    impl Window {
        fn asked(&self, head: u8, onto: u8) -> bool {
            let rows: Vec<(Oid, Vec<Oid>, bool)> = self
                .0
                .iter()
                .map(|(id, parents, published)| {
                    (
                        oid(*id),
                        parents.iter().map(|p| oid(*p)).collect(),
                        *published,
                    )
                })
                .collect();
            range_rewrites_published(
                rows.iter().map(|(oid, parents, published)| WalkedRow {
                    oid: *oid,
                    parents,
                    published: *published,
                }),
                oid(head),
                oid(onto),
            )
        }
    }

    /// Local commits on top of what a remote has: rebasing onto the
    /// commit they sit on rewrites none of them, rebasing onto anything
    /// below it rewrites the pushed ones.
    fn a_local_stretch() -> Window {
        Window(vec![
            (1, vec![2], false),
            (2, vec![3], false),
            (3, vec![4], true),
            (4, vec![], true),
        ])
    }

    #[test]
    fn a_range_of_local_commits_rewrites_nothing_pushed() {
        assert!(!a_local_stretch().asked(1, 3));
    }

    #[test]
    fn a_range_reaching_past_the_remote_tip_rewrites_pushed_commits() {
        assert!(a_local_stretch().asked(1, 4));
    }

    #[test]
    fn a_range_that_is_empty_rewrites_nothing() {
        // Onto the very commit HEAD is on, and onto one that already has
        // it: both leave nothing to replay.
        assert!(!a_local_stretch().asked(1, 1));
        assert!(!a_local_stretch().asked(3, 1));
    }

    /// A local merge whose second side was pushed: 5 and 6 are this
    /// clone's, 7 came off a remote, and all three sit on 8.
    fn a_merged_side() -> Window {
        Window(vec![
            (5, vec![6, 7], false),
            (6, vec![8], false),
            (7, vec![8], true),
            (8, vec![], true),
        ])
    }

    /// The case a single row's mark cannot answer: the oldest row of the
    /// range down the first parent (6) is local, and the range still
    /// holds a commit the remote has (7).
    #[test]
    fn a_merged_in_side_counts_even_where_the_first_parent_is_local() {
        assert!(a_merged_side().asked(5, 8));
    }

    #[test]
    fn a_side_the_new_base_already_reaches_is_left_alone() {
        // Rebasing onto 7 replays 5 and 6 only, and neither is pushed.
        assert!(!a_merged_side().asked(5, 7));
    }

    /// The two ends of a name the window does not draw. A HEAD off the
    /// window puts nothing in the range; a base off it leaves everything
    /// above in — which is what rebasing onto a name older than the
    /// window does.
    #[test]
    fn a_name_no_row_carries_falls_out_of_the_window() {
        assert!(!a_local_stretch().asked(9, 4));
        assert!(a_local_stretch().asked(1, 9));
    }

    #[test]
    fn without_a_remote_no_range_rewrites_anything() {
        let window = Window(vec![
            (1, vec![2], false),
            (2, vec![3], false),
            (3, vec![], false),
        ]);
        assert!(!window.asked(1, 3));
    }
}
