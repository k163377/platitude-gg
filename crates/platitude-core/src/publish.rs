//! Whether commits about to be rewritten are already on a remote.
//!
//! Rewriting published history is legal and sometimes right, so this only
//! answers the question — the warning and the decision belong to the UI
//! (デザイン規約「push 済みの範囲は言うだけ」).
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
/// already on screen.
///
/// `rows` is the walk's own order, **children before every parent**
/// (`--date-order`). That is what makes one pass enough: by the time a
/// row is reached, every commit that could have carried a side's colour
/// down to it has been through here, so its two answers are final and
/// the first hit can return.
///
/// **The whole range is walked.** `published` is closed under
/// ancestors, so for a range with no merge in it the answer is the
/// oldest row's mark — but a merge brings in a side whose mark says
/// nothing about the first parent's, and the range is then a set of
/// commits.
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
    // bounded by the open lanes, the way the walk's own marking is
    // (`session::published::PublishMarks`).
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

/// Which commit `branch --delete` measures a branch's tip against —
/// git's `branch_merged`: the branch's upstream where that resolves, and
/// HEAD otherwise (measured: `upstream_integration`). `None` where
/// neither is known, which leaves the question unanswered.
pub fn delete_reference(upstream: Option<Oid>, head: Option<Oid>) -> Option<Oid> {
    upstream.or(head)
}

/// Whether `from` reaches `to` — whether `to` is an ancestor of `from`,
/// or `from` itself — answered off rows already on screen, the way
/// [`range_rewrites_published`] answers its range. What `branch --delete`
/// asks of its reference point ([`delete_reference`];
/// `merge-base --is-ancestor <branch> <reference>`), read here so the
/// delete row can wear `-D` in the frame the menu opens in
/// ([`crate::branch::is_merged_into`] is the same question asked the slow
/// way).
///
/// `rows` is the walk's own order, children before every parent, so one
/// pass down from `from` is enough: by the time `to`'s row is reached,
/// every path from `from` that could have carried the colour down to it
/// has been through here.
///
/// **`None` where the window cannot say.** `to` on no row is a commit the
/// walk stopped short of, and so is a `from` on none; `Some(false)` is
/// given only where `from` is drawn and `to` was reached without the
/// colour — either `from` was emitted earlier and none of its ancestors
/// in between is `to`, or `from` is emitted later, which the order says
/// is not a descendant.
pub fn reaches<'a>(
    rows: impl IntoIterator<Item = WalkedRow<'a>>,
    from: Oid,
    to: Oid,
) -> Option<bool> {
    if from == to {
        return Some(true);
    }
    let mut pending: HashSet<Oid> = HashSet::from([from]);
    let mut from_drawn = false;
    let mut to_drawn = false;
    for row in rows {
        if row.oid == to {
            to_drawn = true;
            if pending.contains(&to) {
                return Some(true);
            }
            // `from` still ahead in the order, if it is drawn at all, is
            // not above `to`; the answer then waits only on whether it is
            // drawn.
            if from_drawn {
                return Some(false);
            }
            continue;
        }
        if row.oid == from {
            from_drawn = true;
        }
        if pending.remove(&row.oid) {
            pending.extend(row.parents.iter().copied());
        }
        if to_drawn && from_drawn {
            return Some(false);
        }
    }
    (from_drawn && to_drawn).then_some(false)
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

    /// git's `branch_merged`, as `upstream_integration` measures it: the
    /// upstream where there is one, HEAD otherwise, and no answer where
    /// neither is known.
    #[test]
    fn the_delete_measures_against_the_upstream_where_there_is_one() {
        let one = Oid::from_hex_str(&"1".repeat(40)).expect("test oid");
        let two = Oid::from_hex_str(&"2".repeat(40)).expect("test oid");
        assert_eq!(delete_reference(Some(one), Some(two)), Some(one));
        assert_eq!(delete_reference(None, Some(two)), Some(two));
        assert_eq!(delete_reference(Some(one), None), Some(one));
        assert_eq!(delete_reference(None, None), None);
    }

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

    impl Window {
        fn reaches(&self, from: u8, to: u8) -> Option<bool> {
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
            super::reaches(
                rows.iter().map(|(oid, parents, published)| WalkedRow {
                    oid: *oid,
                    parents,
                    published: *published,
                }),
                oid(from),
                oid(to),
            )
        }
    }

    /// A branch merged into its reference point is one the reference
    /// reaches — down the first parent or through a merge's other side.
    #[test]
    fn a_reference_reaches_the_commits_behind_it() {
        let stretch = a_local_stretch();
        assert_eq!(stretch.reaches(1, 3), Some(true));
        assert_eq!(stretch.reaches(1, 1), Some(true), "a commit reaches itself");
        assert_eq!(
            stretch.reaches(3, 1),
            Some(false),
            "an older commit does not reach a newer one"
        );
        let merged = a_merged_side();
        assert_eq!(
            merged.reaches(5, 7),
            Some(true),
            "through the merge's second side"
        );
        assert_eq!(
            merged.reaches(6, 7),
            Some(false),
            "a sibling is not an ancestor"
        );
    }

    /// Two branches side by side, neither reaching the other, are told
    /// apart from the window not saying — both are drawn.
    #[test]
    fn two_drawn_commits_neither_above_the_other_answer_no() {
        let window = Window(vec![
            (1, vec![3], false),
            (2, vec![3], false),
            (3, vec![], false),
        ]);
        assert_eq!(window.reaches(1, 2), Some(false));
        assert_eq!(window.reaches(2, 1), Some(false));
    }

    /// A commit the window does not draw cannot be answered for either
    /// way: the answer is git's to give.
    #[test]
    fn a_commit_off_the_window_leaves_the_question_open() {
        let stretch = a_local_stretch();
        assert_eq!(
            stretch.reaches(1, 9),
            None,
            "the branch is older than the window"
        );
        assert_eq!(stretch.reaches(9, 4), None, "the reference is not drawn");
    }
}
