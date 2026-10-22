//! What the remotes already have: whether commits about to be rewritten
//! are on one, whether one commit reaches another, and which remotes hold
//! a commit.
//!
//! This only answers; the warning and the decision are the UI's
//! (デザイン規約「ローカルの履歴書き換えはクリック」).
//!
//! "Published" means reachable from some remote-tracking ref — only as
//! fresh as the last fetch.
//!
//! [`state_of`] asks git (a process away); [`range_rewrites_published`],
//! [`reaches`] and [`holders`] read the rows the walk already drew
//! (`session::published`), in the frame a menu opens in
//! (デザイン規約 §行が読む答えはどこから来るか).

use std::collections::{HashMap, HashSet};
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

/// One walked row (`session::LogRow`): parents as the walk sifted them,
/// and the walk's published mark.
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
/// `rows` must be in walk order, **children before every parent**
/// (`--date-order`), so one pass settles each row and the first hit can
/// return. The whole range is walked: a merge brings in a side whose mark
/// says nothing about the first parent's.
///
/// **The window is the answer** (as for `session::published`): a `head` on
/// no row puts nothing in the range (`false`); an `onto` on no row leaves
/// all of `head`'s reach in it — the right answer for a base older than
/// the window.
pub fn range_rewrites_published<'a>(
    rows: impl IntoIterator<Item = WalkedRow<'a>>,
    head: Oid,
    onto: Oid,
) -> bool {
    // Ids waiting to be reached, per side — bounded by the open lanes.
    let mut from_head: HashSet<Oid> = HashSet::from([head]);
    let mut from_onto: HashSet<Oid> = HashSet::from([onto]);
    for row in rows {
        let in_head = from_head.remove(&row.oid);
        // Both sides carry on down: what `onto` reaches is left alone
        // however `head` also reaches it.
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
/// git's `branch_merged`: the upstream where it resolves, else HEAD
/// (pinned by `upstream_integration`).
pub fn delete_reference(upstream: Option<Oid>, head: Option<Oid>) -> Option<Oid> {
    upstream.or(head)
}

/// Whether `from` reaches `to` (`to` is `from` or its ancestor), off rows
/// already on screen so the delete row can wear `-D` in the frame the menu
/// opens in: `branch --delete` asks it of [`delete_reference`], and
/// [`crate::branch::is_merged_into`] asks it the slow way.
///
/// `rows` must be in walk order, children before every parent, so one
/// pass down from `from` is enough.
///
/// **`None` where the window cannot say** — `from` or `to` on no row.
/// `Some(false)` only when both are drawn and `to` was reached without
/// the colour (a `from` drawn after `to` cannot be above it).
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
            // A `from` later in the order is not above `to`: what is left
            // is whether it is drawn.
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

/// Which remotes already have `target`, off rows already on screen, so a
/// menu offering a link on a remote can leave out the remotes without the
/// commit as it opens (デザイン規約 §行が読む答えはどこから来るか). `tips`
/// pairs each remote-tracking branch's commit with its remote — an index
/// into the caller's list; bit `i` of the answer is remote `i` (one past
/// the 64th counts as none).
///
/// `rows` in walk order, children before every parent, so one pass down
/// to `target` settles it — every tip above it has been through (the same
/// prefix property `session::published` rests on). Zero where `target` is
/// on no row.
pub fn holders<'a>(
    rows: impl IntoIterator<Item = WalkedRow<'a>>,
    target: Oid,
    tips: &[(Oid, u32)],
) -> u64 {
    let mut pending: HashMap<Oid, u64> = HashMap::new();
    for (oid, remote) in tips {
        *pending.entry(*oid).or_default() |= 1u64.checked_shl(*remote).unwrap_or(0);
    }
    for row in rows {
        let Some(mask) = pending.remove(&row.oid) else {
            if row.oid == target {
                return 0;
            }
            continue;
        };
        if row.oid == target {
            return mask;
        }
        for parent in row.parents {
            *pending.entry(*parent).or_default() |= mask;
        }
    }
    0
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
        Oid::from_hex_str(&hex).unwrap()
    }

    /// `(id, parents, published)` rows in walk order, newest first.
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

    /// Local 1 and 2 on top of pushed 3 and 4.
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
        // Onto HEAD itself, and onto a descendant: nothing to replay.
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

    /// The oldest first-parent row of the range (6) is local, yet the
    /// range holds pushed 7: no single row's mark answers this.
    #[test]
    fn a_merged_in_side_counts_even_where_the_first_parent_is_local() {
        assert!(a_merged_side().asked(5, 8));
    }

    #[test]
    fn a_side_the_new_base_already_reaches_is_left_alone() {
        // Rebasing onto 7 replays 5 and 6 only, and neither is pushed.
        assert!(!a_merged_side().asked(5, 7));
    }

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

    /// Both drawn, so `Some(false)` rather than the window's `None`.
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

    impl Window {
        fn holders(&self, target: u8, tips: &[(u8, u32)]) -> u64 {
            let rows: Vec<(Oid, Vec<Oid>)> = self
                .0
                .iter()
                .map(|(id, parents, _)| (oid(*id), parents.iter().map(|p| oid(*p)).collect()))
                .collect();
            let tips: Vec<(Oid, u32)> = tips.iter().map(|(id, at)| (oid(*id), *at)).collect();
            super::holders(
                rows.iter().map(|(oid, parents)| WalkedRow {
                    oid: *oid,
                    parents,
                    published: false,
                }),
                oid(target),
                &tips,
            )
        }
    }

    #[test]
    fn each_remote_holds_what_its_branches_reach() {
        // Remote 0 stands on 6, remote 1 on the merge 5 — whose sides are
        // 6 and 7 — and remote 2 on 7.
        let merged = a_merged_side();
        let tips = [(6, 0), (5, 1), (7, 2)];
        assert_eq!(merged.holders(5, &tips), 0b010, "the merge itself");
        assert_eq!(merged.holders(6, &tips), 0b011, "its first side");
        assert_eq!(merged.holders(7, &tips), 0b110, "its second side");
        assert_eq!(merged.holders(8, &tips), 0b111, "the base under both");
    }

    #[test]
    fn a_commit_above_every_tip_or_off_the_window_is_held_by_none() {
        let stretch = a_local_stretch();
        assert_eq!(stretch.holders(1, &[(3, 0)]), 0, "above the tip");
        assert_eq!(stretch.holders(9, &[(3, 0)]), 0, "on no row");
        assert_eq!(stretch.holders(4, &[]), 0, "no remote at all");
        assert_eq!(
            stretch.holders(4, &[(3, 0), (3, 0)]),
            1,
            "two branches on one commit"
        );
        assert_eq!(
            stretch.holders(4, &[(3, 64)]),
            0,
            "past the 64th remote counts as none"
        );
    }
}
