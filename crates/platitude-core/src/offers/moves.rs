//! Where a press on a ref moves to, whether it can run at all, and what
//! leaving a stopped operation undoes. Apart from the menus: the two
//! share nothing but the repository state they read.

use crate::integrate::InProgress;
use crate::opstate::OpState;
use crate::status::Counts;

/// The move a press on a ref adds up to, in the word
/// `RepoPage.switchToRef` branches on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchAction {
    /// Nothing moves: the branch the tree is already on, the detached
    /// marker, or a tag — moving onto one could only detach HEAD, so its
    /// row offers a branch at its commit instead.
    None,
    /// The branch is checked out in another working copy: git refuses the
    /// move outright (`fatal: '<branch>' is already used by worktree at
    /// …`), locked or not, and no stash or leaving an operation clears it.
    /// So the press stands the tab in that copy (デザイン規約
    /// §進行中の操作から出る) without asking: that writes nothing and is
    /// one press back.
    OpenHolder,
    /// An ordinary `switch` onto an existing local branch.
    Switch,
    /// The remote branch has no local one yet: the move materialises it
    /// (`switch` creating the local branch off the remote ref).
    Materialize,
    /// The remote branch's local one already exists, so the move puts the
    /// existing branch on the remote ref — the one move that can leave
    /// commits unreachable, so git answers what it costs first
    /// (`checkout_moving_branch`).
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

/// The action itself. `kind` is the word the chip goes out under
/// (`branch` / `remote` — every other kind moves nothing); `local` the
/// local branch the move lands on (for `remote`, the remote ref's local
/// name); `held_by_worktree` the other working copy holding that local
/// branch, empty when none does; `local_oid` that branch's commit, empty
/// when no such branch exists.
pub fn switch_action(
    kind: &str,
    local: &str,
    current_branch: &str,
    held_by_worktree: &str,
    local_oid: &str,
) -> SwitchAction {
    match kind {
        "branch" => {
            if local == current_branch {
                SwitchAction::None
            } else if !held_by_worktree.is_empty() {
                SwitchAction::OpenHolder
            } else {
                SwitchAction::Switch
            }
        }
        "remote" => {
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
/// `landing` empty is "none is on its way", which has landed nothing.
///
/// **Both halves, because they arrive apart**: status comes back long
/// before a large ref listing, and a second press read in between still
/// sees no local branch and sends a second `switch --create`, which git
/// refuses.
pub fn move_landed(landing: &str, current_branch: &str, landing_oid: &str) -> bool {
    !landing.is_empty() && landing == current_branch && !landing_oid.is_empty()
}

/// Whether a move has to clear the way before it can run. git refuses
/// every `switch` while a merge / rebase / cherry-pick / revert stands,
/// whatever the tree (`cannot switch branch while …`), and over an
/// unmerged index too — what `--quit` leaves behind. So the question is
/// raised before anything is sent (デザイン規約 §進行中の操作から出る);
/// bisect rides the same gate.
pub fn moves_blocked(ops: &OpState, counts: &Counts) -> bool {
    ops.any() || counts.conflicted > 0
}

/// Whether putting the standing operation down costs anything — which
/// shapes the question a blocked move raises.
///
/// Only a rebase: `rebase --quit` leaves HEAD detached with the copies
/// made so far unreferenced, so its way out is `--abort`, which throws
/// away work in hand. The rest take `--quit`: earlier steps' commits
/// stay, and the tree goes into a stash.
pub fn leaving_undoes(op: Option<InProgress>) -> bool {
    op == Some(InProgress::Rebase)
}

/// The command the blocked-move question opens its line with (デザイン規約
/// §git 用語のコード表記): `rebase --abort` where leaving undoes, else
/// `stash` — where the reader finds their work afterwards.
pub fn leave_code(op: Option<InProgress>) -> &'static str {
    if leaving_undoes(op) {
        "rebase --abort"
    } else {
        "stash"
    }
}

/// Whether leaving the stopped commit out costs nothing: a tree with
/// nothing conflicted, staged or unstaged (untracked files aside) is a
/// commit that came out empty, which git names `--skip` for
/// (`an_interactive_rebase_stops_on_an_emptied_commit_and_names_skip`).
///
/// An `edit` stop is just as clean, but skipping it takes an applied
/// commit out, so `editing` ([`crate::integrate::RebaseStop`]) overrules
/// the tree.
pub fn skip_is_free(counts: &Counts, editing: bool) -> bool {
    !editing && counts.conflicted == 0 && counts.staged == 0 && counts.unstaged == 0
}
