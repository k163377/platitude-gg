//! What the background threads send the main thread, one enum per
//! feed.

use super::*;

/// What came back about a folder somebody picked, before it is a tab.
#[derive(Debug)]
pub enum PickMsg {
    /// It opens. The path is the one that was picked, not the root git
    /// resolved it to.
    Accepted { path: PathBuf },
    /// It does not, and never becomes a tab. `kind` is `plain` / `bare` /
    /// `other` — the same three the tab's own failure screen names.
    Rejected {
        path: PathBuf,
        /// The folder to reopen the picker at — the one this sits in.
        near: String,
        kind: &'static str,
        /// git's own words, for `other` alone.
        message: String,
    },
}

/// Tab-level messages (open lifecycle + background errors + write state).
#[derive(Debug)]
pub enum TabMsg {
    Opened {
        title: String,
        path: String,
    },
    /// The repository behind this tab would not open. `kind` is the same
    /// three the picker's dialog knows (`plain` / `bare` / `other`);
    /// `message` is git's own, shown only for `other`.
    OpenFailed {
        kind: &'static str,
        path: String,
        message: String,
    },
    OpError {
        message: String,
    },
    /// A write command started / ended. A failed write also arrives as an
    /// `OpError`, so the existing error surface needs no special case;
    /// `error` is here as well so an editor can tell whether the write it
    /// asked for is the one that failed.
    WriteState {
        op: String,
        running: bool,
        error: String,
    },
    /// A branch move would leave commits unreachable and was not made.
    MoveNeedsAsk {
        local: String,
        start: String,
    },
    /// How much of a range a remote already has (rewrite warning).
    Publish {
        range: String,
        total: i32,
        published: i32,
    },
    /// Whether HEAD can reach a commit — whether a rewrite may start there.
    InHistory {
        oid: String,
        in_history: bool,
    },
    /// Whether anything besides the current branch still reaches its tip,
    /// which is what a rewrite here would cost.
    HeadReach {
        reached_elsewhere: bool,
    },
    /// Whether a remote already carries a branch name, as of now rather
    /// than as of the last fetch. The question rides along: the box that
    /// asked may be on another name by the time this arrives.
    RemoteBranch {
        remote: String,
        branch: String,
        /// `platitude_core::remote::RemoteBranchState` by its wire name.
        state: String,
        /// The commit the remote advertised, hex; empty where it named none.
        tip: String,
        /// Commits that tip has and this history does not.
        theirs: i32,
    },
    /// Whether `branch --delete` would go through for the branch a menu
    /// just opened over. The branch rides along: the menu may be open
    /// over another row by the time this lands.
    BranchDelete {
        branch: String,
        merged: bool,
    },
    /// Merge tool names the settings field can offer. Empty is an answer.
    /// `settled` false is the fast half, with the slow read still out.
    MergeTools {
        names: Vec<String>,
        settled: bool,
    },
    /// Author identity and signing state.
    Author {
        name: String,
        email: String,
        complete: bool,
        /// `commit.gpgsign`, not `is_active()`: the commit editor is what
        /// says so, and `tag.gpgsign` would make it say it falsely.
        sign_commits: bool,
        signing_format: String,
    },
    /// One commit's signature: the outcome the UI shows (empty for an
    /// unsigned commit), git's own `%G?` letter behind it, and who signed.
    Signature {
        oid: String,
        kind: String,
        code: String,
        signer: String,
    },
    /// Configured remote names — where a branch with no upstream can go.
    Remotes {
        names: Vec<String>,
    },
    /// HEAD's message and author, for prefilling an amend.
    HeadCommit {
        message: String,
        author_name: String,
        author_email: String,
    },
    /// Auto fetch started or ended. Kept off the shared error surface: a
    /// laptop that is simply offline must not raise a fresh banner every
    /// interval, so the toolbar indicator carries this state instead.
    AutoFetch {
        running: bool,
        error: String,
    },
}

/// Graph-model messages (log stream lifecycle).
#[derive(Debug)]
pub enum GraphMsg {
    Started {
        generation: u64,
    },
    Chunk {
        generation: u64,
        rows: Vec<LogRow>,
    },
    Labels {
        generation: u64,
        rows: Vec<(u32, Vec<RefLabel>)>,
    },
    Finished {
        generation: u64,
        total: u32,
        elapsed_ms: u64,
        /// Commits the walk emitted (the truncation footer's number —
        /// see [`SessionEvent::LogFinished`]).
        walked: u32,
        truncated: bool,
    },
    /// Whole-graph replacement in one message (see
    /// [`SessionEvent::LogReplaced`]): applied in place so no empty
    /// model is ever visible.
    Replaced {
        generation: u64,
        rows: Vec<LogRow>,
        elapsed_ms: u64,
        walked: u32,
        truncated: bool,
    },
    Failed {
        generation: u64,
        message: String,
    },
}

/// Command-log messages: one git invocation, start and end.
#[derive(Debug)]
pub enum CommandMsg {
    Started {
        id: u64,
        display: String,
        full: String,
        at_ms: i64,
    },
    Finished {
        id: u64,
        /// Exit code, or `None` when git never ran to a code of its own
        /// (killed by the timeout, cancelled, failed to start).
        code: Option<i32>,
        /// Why there is no code, for the row to say instead.
        note: String,
        /// The code is an answer this command was asked for, so the row
        /// is not a failure however it exited.
        answered: bool,
        elapsed_ms: i64,
        message: String,
    },
}

#[derive(Debug)]
pub struct StatusMsg {
    pub status: WorkTreeStatus,
    pub op_state: OpState,
    /// "commit N of M" while a rebase is stepping.
    pub progress: Option<platitude_core::conflict::Progress>,
    /// What the two sides of a conflict are called. Empty names while
    /// nothing is stopped, or where git left nothing to name one by.
    pub sides: platitude_core::conflict::Sides,
    /// The merge tool git would launch. Empty with none configured, or
    /// while nothing is conflicted. Names the menu row; the launch reads
    /// the config again rather than trusting this.
    pub merge_tool: String,
    /// Pending files whose change has something to say about line endings.
    pub eol_marks: Arc<Vec<platitude_core::session::EolMark>>,
}

/// What the diff feed carries. Two messages rather than one, because the
/// colours come out behind the rows they belong to — see
/// [`platitude_core::session::SessionEvent::DiffColoured`] for why.
#[derive(Debug)]
pub enum DiffMsg {
    Loaded {
        target: DiffTarget,
        patches: Arc<Vec<FilePatch>>,
        preview: Option<FilePreview>,
        /// Fingerprint of the diff's source bytes; selections carry it
        /// back so a partial write can refuse a drifted diff.
        fingerprint: u64,
        /// What the same bytes said about line endings, if anything.
        endings: Option<platitude_core::eol::Notice>,
    },
    /// Colours for the lines of the diff named by `target`, addressed the
    /// way the rows are. Never arrives for a diff nobody is on.
    Coloured {
        target: DiffTarget,
        colors: platitude_core::highlight::DiffColors,
    },
}

impl DiffMsg {
    pub fn target(&self) -> &DiffTarget {
        match self {
            DiffMsg::Loaded { target, .. } | DiffMsg::Coloured { target, .. } => target,
        }
    }
}
