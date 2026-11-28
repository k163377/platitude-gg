//! What the `pull` row of a ref's menu runs ([`crate::menu::pull_runs`]).
//! Which rows have it and why it greys are the menu type's
//! ([`crate::menu::Part::Pulled`], [`crate::menu::Row::Pull`]).

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

impl PullRuns {
    /// The word the menu's record carries it in (`encode::MenuAnswer`), which
    /// the row's sentence and its press read.
    pub fn word(self) -> &'static str {
        match self {
            Self::Here => "here",
            Self::Aside => "aside",
        }
    }
}
