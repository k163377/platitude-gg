//! Where a press on a ref moves to, whether it can run at all, and what
//! leaving a stopped operation undoes. Apart from the menus: the two
//! share nothing but the repository state they read.

use super::{RefKind, TreeStanding};
use crate::integrate::InProgress;
use crate::opstate::OpState;
use crate::status::Counts;

/// The move a press on a ref adds up to, in the word
/// `RepoPage.switchToRef` branches on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchAction {
    /// Nothing moves: the branch the tree is already on, or a press that
    /// names no branch to land on ([`SwitchTarget::from_words`]).
    None,
    /// The branch is checked out in another worktree: git refuses the
    /// move outright (`fatal: '<branch>' is already used by worktree at
    /// …`), locked or not, and no stash or leaving an operation clears it.
    /// So the press stands the tab in that worktree without asking (user
    /// decision — ask before changing): that writes nothing and is one press
    /// back. No question bar and no `!`: that mark means "pressing asks".
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

/// The branch here a press lands on, and which row was pressed for it:
/// the branch's own, or a remote branch's — whose move lands on the local
/// branch of its name (`RepoPage.switchToRef` asks which), never on the
/// remote row's own name (`origin/feat` lands on `feat`). `local` is never
/// empty.
///
/// The fields are this module's: the one way to build it is
/// [`Self::from_words`], which is where every other press ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchTarget<'a> {
    pressed: Pressed,
    local: &'a str,
}

/// The row a [`SwitchTarget`] was pressed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pressed {
    Branch,
    Remote,
}

impl<'a> SwitchTarget<'a> {
    /// `kind` is the word the chip goes out under ([`RefKind::from_word`])
    /// and `local` the local branch the move lands on. `None` for every
    /// kind but a branch and a remote branch — a tag could only detach
    /// HEAD, so its row offers a branch at its commit instead; a stash, a
    /// worktree's chip and the HEAD marker name no branch — and for no
    /// name.
    pub fn from_words(kind: &str, local: &'a str) -> Option<Self> {
        let pressed = match RefKind::from_word(kind)? {
            RefKind::Branch => Pressed::Branch,
            RefKind::Remote => Pressed::Remote,
            RefKind::Tag | RefKind::Stash | RefKind::Worktree => return None,
        };
        (!local.is_empty()).then_some(Self { pressed, local })
    }
}

/// The action itself. `target` is where the press lands, `None` where it
/// lands nowhere; `tree` where the worktree stands; `held` whether another
/// worktree holds that local branch; `local_listed` whether the listing
/// has that branch.
///
/// A remote row moves its local branch even where the tree is on it:
/// landing the current branch on its remote ref is a move.
pub fn switch_action(
    target: Option<SwitchTarget<'_>>,
    tree: &TreeStanding,
    held: bool,
    local_listed: bool,
) -> SwitchAction {
    let Some(SwitchTarget { pressed, local }) = target else {
        return SwitchAction::None;
    };
    match pressed {
        Pressed::Branch => {
            if tree.is_current(local) {
                SwitchAction::None
            } else if held {
                SwitchAction::OpenHolder
            } else {
                SwitchAction::Switch
            }
        }
        Pressed::Remote => {
            if held {
                SwitchAction::OpenHolder
            } else if !local_listed {
                SwitchAction::Materialize
            } else {
                SwitchAction::MoveBranch
            }
        }
    }
}

/// Whether a move already sent has reached the screen: HEAD stands on
/// the branch it was landing on (`tree`), and the ref listing has that
/// branch (`landing_listed`). `landing` empty is "none is on its way",
/// which has landed nothing: no empty name is current.
///
/// **Both halves, because they arrive apart**: status comes back long
/// before a large ref listing, and a second press read in between still
/// sees no local branch and sends a second `switch --create`, which git
/// refuses.
pub fn move_landed(landing: &str, tree: &TreeStanding, landing_listed: bool) -> bool {
    tree.is_current(landing) && landing_listed
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
