//! The escapes a hook reads from the command it is asked about: one
//! environment flag per approval the user gave in so many words. Named
//! by the commands that take them and by the docs, so it reads nothing
//! of the hook (反映前テストの機械化.md §依存木).

/// The escape for a commit in the primary checkout the user asked for in
/// so many words (`commit`). A landing does not read it — the permit does.
pub(crate) const MAIN_APPROVAL_FLAG: &str = "PGG_ALLOW_MAIN";

/// The same, for an instruction that asked for a rebase.
pub(crate) const REBASE_APPROVAL_FLAG: &str = "PGG_ALLOW_REBASE";

/// The same, for an instruction that asked for a real window.
pub(crate) const GUI_APPROVAL_FLAG: &str = "PGG_ALLOW_GUI";

/// The same, for an instruction that approved stopping processes outside this worktree.
pub(crate) const PROCESS_STOP_APPROVAL_FLAG: &str = "PGG_ALLOW_KILL";

/// The same, for an instruction that asked for a seat to be taken over
/// from whoever holds it (`seats::takeover`).
pub(crate) const TAKEOVER_APPROVAL_FLAG: &str = "PGG_ALLOW_TAKEOVER";

/// Every escape this hook reads — what a flag spelled elsewhere is held to.
pub(crate) const APPROVAL_FLAGS: [&str; 5] = [
    MAIN_APPROVAL_FLAG,
    REBASE_APPROVAL_FLAG,
    GUI_APPROVAL_FLAG,
    PROCESS_STOP_APPROVAL_FLAG,
    TAKEOVER_APPROVAL_FLAG,
];
