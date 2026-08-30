//! Where a press on a ref moves to, and what standing in its way costs:
//! the move a chip's letter and the local branch add up to, whether one
//! can run at all, and what leaving a stopped operation undoes.
//!
//! Beside the menus rather than in with them: the two share nothing but
//! the repository state they read, and one file of every rule this module
//! holds outgrew the ceiling (structure.md §上限).

use crate::integrate::InProgress;
use crate::opstate::OpState;
use crate::status::Counts;

/// The move a press on a ref adds up to, in the word
/// `RepoPage.switchToRef` branches on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchAction {
    /// Nothing moves: the branch the tree is already on, the detached
    /// marker (it names no branch), or a tag — moving onto one could only
    /// detach HEAD, so a tag's row offers a branch at its commit instead.
    None,
    /// The branch is checked out in another working copy, and **git
    /// refuses the move outright** (`fatal: '<branch>' is already used by
    /// worktree at …`, measured), locked or not. The only refusal
    /// no stash can get past and no operation put down can clear — the
    /// branch is simply somewhere else, and the way to it is that copy.
    OpenHolder,
    /// An ordinary `switch` onto an existing local branch.
    Switch,
    /// The remote branch has no local one yet: the move materialises it
    /// (`switch` creating the local branch off the remote ref).
    Materialize,
    /// The remote branch's local one already exists, so landing on the
    /// remote ref moves the existing branch onto it — the one move here
    /// that can leave commits unreachable, so what it would cost is
    /// git's to answer before it runs (`checkout_moving_branch`).
    MoveBranch,
}

impl SwitchAction {
    /// The word the dispatcher branches on; `""` for [`Self::None`].
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "",
            Self::OpenHolder => "holder",
            Self::Switch => "switch",
            Self::Materialize => "materialize",
            Self::MoveBranch => "move",
        }
    }
}

/// The action itself. `kind_letter` is the chip record's letter (`L`
/// local / `R` remote — every other letter moves nothing); `local` the
/// local branch the move lands on (for `R`, the remote ref's local
/// name); `held_by_worktree` the other working copy holding that local
/// branch, empty when none does; `local_oid` that branch's commit, empty
/// when no such branch exists.
pub fn switch_action(
    kind_letter: &str,
    local: &str,
    current_branch: &str,
    held_by_worktree: &str,
    local_oid: &str,
) -> SwitchAction {
    match kind_letter {
        "L" => {
            if local == current_branch {
                SwitchAction::None
            } else if !held_by_worktree.is_empty() {
                SwitchAction::OpenHolder
            } else {
                SwitchAction::Switch
            }
        }
        "R" => {
            if !held_by_worktree.is_empty() {
                SwitchAction::OpenHolder
            } else if local_oid.is_empty() {
                SwitchAction::Materialize
            } else {
                SwitchAction::MoveBranch
            }
        }
        _ => SwitchAction::None,
    }
}

/// Whether a move already sent has reached the screen: HEAD stands on
/// the branch it was landing on, and the ref listing has that branch.
/// `landing` is the branch that move puts HEAD on — empty is "none is on
/// its way", which has landed nothing.
///
/// **Both halves, because they arrive apart.** A write answers before it
/// publishes what it moved (`session::write`), and the status comes back
/// long before a listing of fifty thousand refs does. Read between the
/// two, the screen still says the remote branch has no local one — so a
/// second press on the same chip is decided from the very picture the
/// first one was, and sends the same `switch --create`, which git
/// refuses because the first one made the branch.
pub fn move_landed(landing: &str, current_branch: &str, landing_oid: &str) -> bool {
    !landing.is_empty() && landing == current_branch && !landing_oid.is_empty()
}

/// Whether a move has to clear the way before it can run. **git refuses
/// every `switch` while a merge / rebase / cherry-pick / revert stands**
/// — clean tree, conflicted tree and resolved-and-staged tree all get
/// the same `cannot switch branch while …` — and it refuses one over an
/// unmerged index too, which is what `--quit` leaves behind (measured, 2.55).
/// So the way out is a question raised before anything is sent, never a
/// refusal read back off the log (デザイン規約 §進行中の操作から出る). Bisect rides
/// the same gate: it is an operation standing, whatever git would say.
pub fn moves_blocked(ops: &OpState, counts: &Counts) -> bool {
    ops.any() || counts.conflicted > 0
}

/// Whether putting the standing operation down costs anything — which
/// decides the whole shape of the question a blocked move raises.
///
/// **A rebase is the one that does.** `git rebase --quit` leaves HEAD
/// detached at the half-rewritten line with every copy it already made
/// unreferenced (measured, 2.55), so its way out is `--abort` — and an abort
/// throws away work in hand. The rest take `--quit`: the commits an
/// earlier step already made stay, and the tree goes into a stash rather
/// than into the reflog, so nothing is destroyed.
pub fn leaving_undoes(op: Option<InProgress>) -> bool {
    op == Some(InProgress::Rebase)
}

/// The command the blocked-move question opens its line with (§git 用語の
/// コード表記): the exit card's own row for the same act where leaving
/// undoes, and `stash` everywhere else — that is where the reader goes to
/// find their work afterwards.
pub fn leave_code(op: Option<InProgress>) -> &'static str {
    if leaving_undoes(op) {
        "rebase --abort"
    } else {
        "stash"
    }
}

/// Whether leaving the stopped commit out costs nothing.
///
/// An interactive rebase stops on a commit that came out empty and names
/// `--skip` as the way past it (measured —
/// `an_interactive_rebase_stops_on_an_emptied_commit_and_names_skip`).
/// Nothing is conflicted or staged while it stands there, and untracked
/// files say nothing about the stopped commit and do not count.
///
/// A clean tree alone cannot answer, though: an `edit` stop is exactly as
/// clean, the commit there is applied and skipping it takes it back out.
/// `editing` is the session's word on why git stopped
/// ([`crate::integrate::RebaseStop`]), and it overrules the tree.
pub fn skip_is_free(counts: &Counts, editing: bool) -> bool {
    !editing && counts.conflicted == 0 && counts.staged == 0 && counts.unstaged == 0
}
