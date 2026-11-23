//! The `pull` row of a ref's menu: whether it is there, what it runs, and
//! why it is greyed (デザイン規約 §取り込んで合流させる).
//!
//! A row on a branch's menus and never a toolbar button (user decision —
//! ask before changing): the toolbar's fixed targets speak for the whole
//! repository, while a pull moves one branch, whose row is the target.
//! The row stands only where it can name the one local branch that moves
//! (user decision — ask before changing): the tree's branch on either end
//! of the comparison the screen already shows as ↑↓, another local branch
//! on its own row. Not on another remote branch's row (user decision —
//! ask before changing): `git pull` moves the branch it runs on, so there
//! it would move one the row does not name — bringing that remote branch
//! into the current one is merge / rebase. Not on the upstream row of
//! another local branch either, for the same reason, nor on tags and
//! stashes.
//!
//! Greyed rather than gone wherever git would turn the press down: a row
//! goes only where its target does not exist (デザイン規約 §メニュー). How a
//! diverged tree's two lines are joined is a choice, and this product does
//! not make it by default for the reader — the row is greyed and its tip
//! says where to choose (`Words.pullDiverged`: the remote branch row's
//! `rebase <current> onto it`, or `merge`); the same reason pins the pull
//! to `--ff-only` (`remote::pull`). Every judgement here is by counts
//! alone, as of the last fetch: a far side that moved since leaves the row
//! live, git refuses the press, and the log comes up.
//!
//! Apart from [`super::ref_menu`] because it reads what no other row
//! reads: the tree's own comparison with its upstream, and the row's own
//! branch's.

use super::{FarSide, Leads, Reading, RefKind, RefTarget, RowStanding};

/// What the `pull` row runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRuns {
    /// `git pull` in this worktree (`remote::pull`): the branch it is on,
    /// brought up to its upstream by a fast-forward (`--ff-only --no-rebase`).
    Here,
    /// The row's own branch, one this worktree is not on, fast-forwarded to
    /// its upstream (`remote::fast_forward`).
    Aside,
}

/// Why the row is there but greyed: the press would be turned down, and
/// the reason is known before it (デザイン規約 §メニュー「今できない」).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullOut {
    /// An operation stands in this worktree: nothing is pulled until it is
    /// finished or put down (user decision — ask before changing). Every
    /// row the tree offers greys, saying `Finish the stopped operation
    /// first` as the merge / rebase rows do. Over a diverged tree the
    /// operation is what is said.
    Standing,
    /// The tree's branch and its upstream have both moved: merging and
    /// rebasing are the reader's to choose between, and git's settings do
    /// not get to choose for them (user decision — ask before changing).
    /// Told by `WorktreeModel.pullBlocked`: ahead and behind both non-zero.
    Diverged,
    /// The row's branch has commits its upstream lacks, so no fast-forward
    /// reaches it — git would answer `[rejected] (non-fast-forward)` — and
    /// a press must not move a branch somewhere unexpected (user decision —
    /// ask before changing). Only a pull from the branch itself can take
    /// them in: switch to it and use its row (a diverged one there says
    /// where next). Its tip says what cannot happen, then why, naming the
    /// upstream (user decision — ask before changing; `Words.pullOwnCommits`).
    /// Told by `NavSectionModel.aheadOf` being non-zero.
    OwnCommits,
    /// Another worktree has the row's branch out, and git will not fetch
    /// into a branch checked out anywhere. Where to go is the `Open` row
    /// above it, naming that worktree: the pull is done in its tab. Said
    /// before the other two — nothing done here lifts it.
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
    /// `here` / `aside`, then `out-standing` / `out-diverged` / `out-own` /
    /// `out-held`.
    pub fn words(&self) -> Vec<&'static str> {
        let mut words = vec![match self.runs {
            PullRuns::Here => "here",
            PullRuns::Aside => "aside",
        }];
        match self.out {
            Some(PullOut::Standing) => words.push("out-standing"),
            Some(PullOut::Diverged) => words.push("out-diverged"),
            Some(PullOut::OwnCommits) => words.push("out-own"),
            Some(PullOut::Held) => words.push("out-held"),
            None => {}
        }
        words
    }
}

/// The rule. The tree's own comparison is
/// [`super::TreeStanding::upstream`] and [`super::TreeStanding::diverged`];
/// `far` is the row's own branch's ([`FarSide::Branch`]): whether a remote
/// reading is listed for it, and how many commits it has that the reading
/// lacks, as of the last fetch.
///
/// - **`git pull` on the two ends of the tree's own comparison**: the
///   branch it is on and the remote-tracking ref that branch is measured
///   against. No other remote row: a pull there would move a branch the
///   row does not name.
/// - **The fast-forward on every other local branch with an upstream.**
///   Detached is no bar: nothing here is touched.
/// - **Greyed on every one of those rows while an operation stands** —
///   a stopped rebase or bisect leaves the tree detached, so its branches
///   all stand aside then. Nowhere without an upstream — git has no far
///   side to go to.
///
/// Without an upstream git turns the plain pull down (`no tracking
/// information`). The fast-forward aside needs an upstream on a remote:
/// the reading is a remote one only ([`Reading`]), so a branch tracking
/// one here (`branch.<n>.remote = .`) has no row.
pub fn pull_row(row: &RowStanding<'_>, far: FarSide) -> Option<PullOffer> {
    let RefTarget { kind, name } = row.target?;
    let tree = row.tree;
    if !row.open {
        return None;
    }
    let standing = tree.op_standing();
    let tree_end = tree.on_branch()
        && !tree.upstream.is_empty()
        && match kind {
            RefKind::Branch => tree.is_current(name),
            RefKind::Remote => name == tree.upstream,
            RefKind::Tag | RefKind::Stash | RefKind::Worktree => false,
        };
    if tree_end {
        let out = if standing {
            Some(PullOut::Standing)
        } else {
            tree.diverged.then_some(PullOut::Diverged)
        };
        return Some(PullOffer {
            runs: PullRuns::Here,
            out,
        });
    }
    let FarSide::Branch(own) = far.on(kind) else {
        return None;
    };
    if tree.is_current(name) || own.reading == Reading::None {
        return None;
    }
    // The holder first: nothing done here lifts it. The operation is
    // finished here, and own commits are taken in from the branch itself,
    // which the reader cannot switch to from here.
    let out = if row.leads == Leads::Holder {
        Some(PullOut::Held)
    } else if standing {
        Some(PullOut::Standing)
    } else if own.ahead > 0 {
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
    use super::super::testkit::{standing, target};
    use super::super::{BranchFar, TreeStanding};
    use super::*;

    /// An idle tree on `main`, tracking `origin/main` and level with it.
    fn tree() -> TreeStanding {
        TreeStanding {
            upstream: "origin/main".into(),
            ..TreeStanding::on("main")
        }
    }

    /// A local branch's far side: a remote reading listed with `ahead`
    /// commits of its own on top, or none.
    fn tracking(ahead: Option<u32>) -> FarSide {
        FarSide::Branch(BranchFar {
            reading: if ahead.is_some() {
                Reading::Together
            } else {
                Reading::None
            },
            tracked: ahead.is_some(),
            has_remote: true,
            ahead: ahead.unwrap_or(0),
        })
    }

    /// The row on `tree` in an open tab, leading nowhere else; a branch row
    /// tracks a reading level with it, any other reads nothing over there.
    fn pull_on(kind: RefKind, full: &str, tree: &TreeStanding) -> Option<PullOffer> {
        let far = if kind == RefKind::Branch {
            tracking(Some(0))
        } else {
            FarSide::Unread
        };
        pull_row(&standing(Some(target(kind, full)), tree), far)
    }

    /// The row on the idle tree; the cases bend one input at a time.
    fn on(kind: RefKind, full: &str) -> Option<PullOffer> {
        pull_on(kind, full, &tree())
    }

    /// Branch `full` aside on `tree`: `leads` where another worktree holds
    /// it, tracking a reading with `ahead` commits of its own, or none.
    fn aside(
        full: &str,
        tree: &TreeStanding,
        leads: Leads,
        ahead: Option<u32>,
    ) -> Option<PullOffer> {
        let row = RowStanding {
            leads,
            ..standing(Some(target(RefKind::Branch, full)), tree)
        };
        pull_row(&row, tracking(ahead))
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
        assert_eq!(
            pull_row(&standing(None, &tree()), tracking(Some(0))),
            None,
            "a row that names no ref"
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
        assert_eq!(
            aside("scratch", &tree(), Leads::Nowhere, None),
            None,
            "no upstream is nowhere to pull from"
        );
        assert_eq!(
            pull_row(
                &standing(Some(target(RefKind::Branch, "feat")), &tree()),
                FarSide::Unread
            ),
            None,
            "nothing read over there is no upstream to pull from"
        );
    }

    #[test]
    fn a_tree_with_no_upstream_still_pulls_its_other_branches() {
        let main = TreeStanding::on("main");
        assert_eq!(
            runs(aside("main", &main, Leads::Nowhere, None)),
            None,
            "git turns a plain pull down with no tracking information"
        );
        assert_eq!(
            runs(aside("main", &main, Leads::Nowhere, Some(0))),
            None,
            "the branch the tree is on is never fast-forwarded, even while the tree's own reading \
             of its upstream has not answered: git refuses to fetch into a checked-out branch"
        );
        assert_eq!(
            runs(aside("feat", &main, Leads::Nowhere, Some(0))),
            Some(PullRuns::Aside)
        );
    }

    #[test]
    fn detached_there_is_no_tree_end_but_every_branch_is_aside() {
        let detached = TreeStanding::detached();
        assert_eq!(
            runs(pull_on(RefKind::Remote, "origin/main", &detached)),
            None
        );
        assert_eq!(
            runs(pull_on(RefKind::Branch, "main", &detached)),
            Some(PullRuns::Aside),
            "nothing is checked out to pull into, and nothing here is touched"
        );
    }

    #[test]
    fn a_tab_not_open_offers_no_pull() {
        for (kind, full) in [
            (RefKind::Branch, "main"),
            (RefKind::Remote, "origin/main"),
            (RefKind::Branch, "feat"),
        ] {
            let tree = tree();
            let row = RowStanding {
                open: false,
                ..standing(Some(target(kind, full)), &tree)
            };
            assert_eq!(pull_row(&row, tracking(Some(0))), None, "{full}");
        }
    }

    #[test]
    fn a_standing_operation_greys_every_row_the_pull_stands_on() {
        let merging = |diverged: bool| TreeStanding {
            diverged,
            ..tree().standing("MERGING")
        };
        let rebasing = TreeStanding::detached().standing("REBASE 1/3");
        for (kind, full) in [(RefKind::Branch, "main"), (RefKind::Remote, "origin/main")] {
            assert_eq!(
                pull_on(kind, full, &merging(true)),
                Some(PullOffer {
                    runs: PullRuns::Here,
                    out: Some(PullOut::Standing)
                }),
                "{full}: a merge keeps the tree on its branch, and the operation is named over the divergence"
            );
        }
        assert_eq!(
            aside("feat", &merging(false), Leads::Nowhere, Some(2)),
            Some(PullOffer {
                runs: PullRuns::Aside,
                out: Some(PullOut::Standing)
            }),
            "named over the branch's own commits"
        );
        assert_eq!(
            pull_on(RefKind::Branch, "main", &rebasing),
            Some(PullOffer {
                runs: PullRuns::Aside,
                out: Some(PullOut::Standing)
            }),
            "a stopped rebase leaves the tree detached, so the branch it replays stands aside"
        );
        assert_eq!(
            pull_on(RefKind::Remote, "origin/main", &rebasing),
            None,
            "detached, no remote row is an end of the tree's comparison"
        );
        assert_eq!(
            pull_on(RefKind::Branch, "main", &merging(false)).map(|offer| offer.words()),
            Some(vec!["here", "out-standing"])
        );
    }

    #[test]
    fn the_tree_diverged_greys_git_pull_on_both_ends() {
        let diverged = TreeStanding {
            diverged: true,
            ..tree()
        };
        for (kind, full) in [(RefKind::Branch, "main"), (RefKind::Remote, "origin/main")] {
            assert_eq!(
                pull_on(kind, full, &diverged),
                Some(PullOffer {
                    runs: PullRuns::Here,
                    out: Some(PullOut::Diverged)
                }),
                "{full}"
            );
        }
        assert_eq!(
            aside("feat", &diverged, Leads::Nowhere, Some(0)).and_then(|offer| offer.out),
            None,
            "the tree's divergence is not another branch's"
        );
    }

    #[test]
    fn a_branch_aside_greys_on_commits_of_its_own_and_on_another_worktree() {
        let out = |leads: Leads, ahead: u32| {
            aside("feat", &tree(), leads, Some(ahead)).and_then(|offer| offer.out)
        };
        assert_eq!(
            out(Leads::Nowhere, 0),
            None,
            "behind or level: a fast-forward reaches it"
        );
        assert_eq!(
            out(Leads::Nowhere, 2),
            Some(PullOut::OwnCommits),
            "git turns the fetch down as a non-fast-forward (measured)"
        );
        assert_eq!(
            out(Leads::Holder, 0),
            Some(PullOut::Held),
            "git will not fetch into a branch checked out elsewhere"
        );
        assert_eq!(
            out(Leads::Holder, 2),
            Some(PullOut::Held),
            "the holder is named first: switching to it is not on offer from here"
        );
        assert_eq!(
            aside("feat", &tree().standing("MERGING"), Leads::Holder, Some(0))
                .and_then(|offer| offer.out),
            Some(PullOut::Held),
            "and over a standing operation: finishing it here frees nothing there"
        );
        assert_eq!(
            aside("feat", &tree(), Leads::Holder, Some(0)).map(|offer| offer.words()),
            Some(vec!["aside", "out-held"])
        );
    }
}
