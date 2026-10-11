//! The `pull` row of a ref's menu: whether it is there, what it runs, and
//! why it is greyed (デザイン規約 §取り込んで合流させる).
//!
//! Apart from [`super::ref_menu`] because it reads what no other row
//! reads: the tree's own comparison with its upstream, and the row's own
//! branch's.

use super::RefKind;

/// What the `pull` row runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRuns {
    /// `git pull` in this worktree (`remote::pull`): the branch it is on,
    /// brought in line with its upstream the way git's settings say.
    Here,
    /// The row's own branch, one this worktree is not on, fast-forwarded to
    /// its upstream (`remote::fast_forward`).
    Aside,
}

/// Why the row is there but greyed: the press would be turned down, and
/// the reason is known before it (デザイン規約 §メニュー「今できない」).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullOut {
    /// The tree's branch and its upstream have both moved: merging and
    /// rebasing are the reader's to choose between, and git's settings do
    /// not get to choose for them.
    Diverged,
    /// The row's branch has commits its upstream lacks, so no fast-forward
    /// reaches it; only a pull from the branch itself can take them in.
    OwnCommits,
    /// Another worktree has the row's branch out, and git will not fetch
    /// into a branch checked out anywhere.
    Held,
}

/// The row, where there is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PullOffer {
    pub runs: PullRuns,
    pub out: Option<PullOut>,
}

impl PullOffer {
    /// The offer as words, the shape `GitFacts.pullRow` answers with:
    /// `here` / `aside`, then `out-diverged` / `out-own` / `out-held`.
    pub fn words(&self) -> Vec<&'static str> {
        let mut words = vec![match self.runs {
            PullRuns::Here => "here",
            PullRuns::Aside => "aside",
        }];
        match self.out {
            Some(PullOut::Diverged) => words.push("out-diverged"),
            Some(PullOut::OwnCommits) => words.push("out-own"),
            Some(PullOut::Held) => words.push("out-held"),
            None => {}
        }
        words
    }
}

/// The rule. `kind`, `full`, `open`, `current_branch`, `detached`,
/// `op_text` and `held_by_worktree` read as in [`super::ref_menu`];
/// `current_upstream` is what the tree's branch tracks
/// (`WorktreeModel.upstream`) and `current_diverged` whether the two have
/// both moved (`WorktreeModel.pullBlocked`); `row_upstream` and `row_ahead`
/// are the row's own branch's upstream and how many commits it has that
/// the upstream lacks, as of the last fetch.
///
/// - **`git pull` on the two ends of the tree's own comparison**: the
///   branch it is on and the remote-tracking ref that branch is measured
///   against. No other remote row: a pull there would move a branch the
///   row does not name.
/// - **The fast-forward on every other local branch with an upstream.**
///   Detached is no bar: nothing here is touched.
/// - **Nowhere while an operation stands**, as the rows that integrate,
///   and nowhere without an upstream — git has no far side to go to.
#[expect(clippy::too_many_arguments)]
pub fn pull_row(
    kind: RefKind,
    full: &str,
    open: bool,
    current_branch: &str,
    detached: bool,
    op_text: &str,
    current_upstream: &str,
    current_diverged: bool,
    held_by_worktree: &str,
    row_upstream: &str,
    row_ahead: i32,
) -> Option<PullOffer> {
    if !open || !op_text.is_empty() || full.is_empty() {
        return None;
    }
    let tree_end = !detached
        && !current_branch.is_empty()
        && !current_upstream.is_empty()
        && match kind {
            RefKind::Branch => full == current_branch,
            RefKind::Remote => full == current_upstream,
            RefKind::Tag | RefKind::Stash | RefKind::Worktree => false,
        };
    if tree_end {
        return Some(PullOffer {
            runs: PullRuns::Here,
            out: current_diverged.then_some(PullOut::Diverged),
        });
    }
    if kind != RefKind::Branch || full == current_branch || row_upstream.is_empty() {
        return None;
    }
    // The holder first: the reader cannot switch to the branch from here,
    // which is what the other reason asks of them.
    let out = if !held_by_worktree.is_empty() {
        Some(PullOut::Held)
    } else if row_ahead > 0 {
        Some(PullOut::OwnCommits)
    } else {
        None
    };
    Some(PullOffer {
        runs: PullRuns::Aside,
        out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An idle, open repository on `main`, tracking `origin/main` and level
    /// with it; `feat` tracks `origin/feat`. The cases bend one input at a
    /// time.
    fn on(kind: RefKind, full: &str) -> Option<PullOffer> {
        pull_row(
            kind,
            full,
            true,
            "main",
            false,
            "",
            "origin/main",
            false,
            "",
            if kind == RefKind::Branch {
                "origin/feat"
            } else {
                ""
            },
            0,
        )
    }

    fn runs(offer: Option<PullOffer>) -> Option<PullRuns> {
        offer.map(|offer| offer.runs)
    }

    #[test]
    fn git_pull_is_offered_on_the_two_ends_of_the_trees_own_comparison() {
        assert_eq!(runs(on(RefKind::Branch, "main")), Some(PullRuns::Here));
        assert_eq!(
            runs(on(RefKind::Remote, "origin/main")),
            Some(PullRuns::Here),
            "and the upstream it is measured against"
        );
        assert_eq!(
            runs(on(RefKind::Remote, "origin/feat")),
            None,
            "a pull there would move a branch the row does not name"
        );
        for kind in [RefKind::Tag, RefKind::Stash, RefKind::Worktree] {
            assert_eq!(runs(on(kind, "v1.0")), None, "{kind:?} is no branch");
        }
        assert_eq!(
            on(RefKind::Branch, "main").map(|offer| offer.words()),
            Some(vec!["here"])
        );
    }

    #[test]
    fn every_other_local_branch_with_an_upstream_is_fast_forwarded() {
        assert_eq!(
            on(RefKind::Branch, "feat"),
            Some(PullOffer {
                runs: PullRuns::Aside,
                out: None
            })
        );
        let untracked = pull_row(
            RefKind::Branch,
            "scratch",
            true,
            "main",
            false,
            "",
            "origin/main",
            false,
            "",
            "",
            0,
        );
        assert_eq!(untracked, None, "no upstream is nowhere to pull from");
    }

    #[test]
    fn a_tree_with_no_upstream_still_pulls_its_other_branches() {
        let offer = |full: &str, row_upstream: &str| {
            runs(pull_row(
                RefKind::Branch,
                full,
                true,
                "main",
                false,
                "",
                "",
                false,
                "",
                row_upstream,
                0,
            ))
        };
        assert_eq!(
            offer("main", ""),
            None,
            "git turns a plain pull down with no tracking information"
        );
        assert_eq!(
            offer("main", "origin/main"),
            None,
            "the branch the tree is on is never fast-forwarded, even while the tree's own reading \
             of its upstream has not answered: git refuses to fetch into a checked-out branch"
        );
        assert_eq!(offer("feat", "origin/feat"), Some(PullRuns::Aside));
    }

    #[test]
    fn detached_there_is_no_tree_end_but_every_branch_is_aside() {
        let offer = |kind: RefKind, full: &str, row_upstream: &str| {
            runs(pull_row(
                kind,
                full,
                true,
                "",
                true,
                "",
                "",
                false,
                "",
                row_upstream,
                0,
            ))
        };
        assert_eq!(offer(RefKind::Remote, "origin/main", ""), None);
        assert_eq!(
            offer(RefKind::Branch, "main", "origin/main"),
            Some(PullRuns::Aside),
            "nothing is checked out to pull into, and nothing here is touched"
        );
    }

    #[test]
    fn a_standing_operation_or_a_tab_not_open_offers_no_pull() {
        for (open, op_text) in [(true, "REBASE 1/3"), (false, "")] {
            for (kind, full) in [
                (RefKind::Branch, "main"),
                (RefKind::Remote, "origin/main"),
                (RefKind::Branch, "feat"),
            ] {
                let offer = pull_row(
                    kind,
                    full,
                    open,
                    "main",
                    false,
                    op_text,
                    "origin/main",
                    false,
                    "",
                    "origin/feat",
                    0,
                );
                assert_eq!(offer, None, "{full} with open={open} op={op_text:?}");
            }
        }
    }

    #[test]
    fn the_tree_diverged_greys_git_pull_on_both_ends() {
        for (kind, full) in [(RefKind::Branch, "main"), (RefKind::Remote, "origin/main")] {
            let offer = pull_row(
                kind,
                full,
                true,
                "main",
                false,
                "",
                "origin/main",
                true,
                "",
                "",
                0,
            );
            assert_eq!(
                offer,
                Some(PullOffer {
                    runs: PullRuns::Here,
                    out: Some(PullOut::Diverged)
                }),
                "{full}"
            );
        }
        let aside = pull_row(
            RefKind::Branch,
            "feat",
            true,
            "main",
            false,
            "",
            "origin/main",
            true,
            "",
            "origin/feat",
            0,
        );
        assert_eq!(
            aside.and_then(|offer| offer.out),
            None,
            "the tree's divergence is not another branch's"
        );
    }

    #[test]
    fn a_branch_aside_greys_on_commits_of_its_own_and_on_another_worktree() {
        let aside = |held: &str, ahead: i32| {
            pull_row(
                RefKind::Branch,
                "feat",
                true,
                "main",
                false,
                "",
                "origin/main",
                false,
                held,
                "origin/feat",
                ahead,
            )
            .and_then(|offer| offer.out)
        };
        assert_eq!(
            aside("", 0),
            None,
            "behind or level: a fast-forward reaches it"
        );
        assert_eq!(
            aside("", 2),
            Some(PullOut::OwnCommits),
            "git turns the fetch down as a non-fast-forward (measured)"
        );
        assert_eq!(
            aside("C:/work/other", 0),
            Some(PullOut::Held),
            "git will not fetch into a branch checked out elsewhere"
        );
        assert_eq!(
            aside("C:/work/other", 2),
            Some(PullOut::Held),
            "the holder is named first: switching to it is not on offer from here"
        );
        assert_eq!(
            pull_row(
                RefKind::Branch,
                "feat",
                true,
                "main",
                false,
                "",
                "origin/main",
                false,
                "C:/work/other",
                "origin/feat",
                0,
            )
            .map(|offer| offer.words()),
            Some(vec!["aside", "out-held"])
        );
    }
}
