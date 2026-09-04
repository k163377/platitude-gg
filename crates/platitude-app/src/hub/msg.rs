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

/// What became of a clone, which has no tab to report into: the dialog
/// that asked for it is still standing, and it is the whole of what there
/// is on screen about it (デザイン規約 §リポジトリを取り寄せる).
#[derive(Debug)]
pub enum CloneMsg {
    /// It came down. The path is the folder that was named, which is the
    /// one to open — git was told where to put it rather than left to
    /// print where it did.
    Done { path: PathBuf },
    /// It did not, and `message` is git's own answer. Nothing here reads
    /// it: a destination that is taken, a URL nothing answers and a
    /// refused credential all come back the same way, and the dialog
    /// quotes what git said.
    Failed { message: String },
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
    /// git stopped part-way through the write in flight and left the
    /// operation standing. Arrives before the `WriteState` that ends that
    /// write, so the answer the page reads already knows it — and which
    /// write it was is that answer's own `op`, not repeated here.
    WriteStopped,
    /// A write command started / ended. A failed write also arrives as an
    /// `OpError`, so the existing error surface needs no special case;
    /// `error` is here as well so an editor can tell whether the write it
    /// asked for is the one that failed.
    ///
    /// **Except the failures that are not this window's to report**:
    /// where the write did not happen and something outside this
    /// application said so, `report` carries what it said and no
    /// `OpError` is sent, because the page answers it with a notice
    /// instead (デザイン規約 §答えの要らない報せ).
    WriteState {
        op: String,
        running: bool,
        error: String,
        report: Option<platitude_core::WriteReport>,
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
    /// Configured remote names — where a branch with no upstream can go —
    /// and which of them this repository sends pushes to. `push_default` is
    /// empty where none is marked; `push_default_local` says whether this
    /// repository's own config is what marked it, since a mark from
    /// anywhere else cannot be cleared here.
    Remotes {
        names: Vec<String>,
        /// Their fetch URLs, in the same order — what the form that
        /// corrects one opens with already filled in.
        urls: Vec<String>,
        push_default: String,
        push_default_local: bool,
    },
    /// HEAD's message and author, for prefilling an amend.
    HeadCommit {
        message: String,
        author_name: String,
        author_email: String,
    },
    /// A fetch nobody asked for started or ended — the interval's, or the
    /// one an opening fires. Kept off the shared error surface: a laptop
    /// that is simply offline must not raise a fresh banner every
    /// interval, so the toolbar indicator carries this state instead.
    AutoFetch {
        running: bool,
        error: String,
        /// Whether a failure here may raise the command log. The fetch an
        /// opening fires says no: opening a tab is not a request to reach
        /// the network, and a machine that is offline would have the
        /// panel thrown up at it every time it opened one.
        announce: bool,
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
    /// The graph standing on screen has fallen behind the repository, or
    /// has caught up with it again (see [`SessionEvent::LogStale`]). No
    /// generation: what it describes is the picture as a whole, not one
    /// stream's rows.
    Stale {
        stale: bool,
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
        /// Whether the reader asked for it. A fetch nobody asked for
        /// arrives here only when git said no, and its row is the record
        /// of that and nothing else: it neither reddens the mark nor
        /// raises the panel (デザイン規約 §git が言ったことを読む場所).
        asked: bool,
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

/// The badge's own two halves, arriving on their own several times a
/// second while a write that replays is out (`SessionEvent::OpProgress`).
/// A slice of [`StatusMsg`] rather than a message of its own kind — the
/// same consumer reads both, and this one costs no git process.
#[derive(Debug)]
pub struct OpProgressMsg {
    pub op_state: OpState,
    pub progress: Option<platitude_core::conflict::Progress>,
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
    /// The message a stopped merge is about to record. Empty unless one
    /// is standing.
    pub op_message: String,
    /// The merge tool git would launch. Empty with none configured, or
    /// while nothing is conflicted. Names the menu row; the launch reads
    /// the config again rather than trusting this.
    pub merge_tool: String,
    /// Where this branch's own mark sends a push
    /// (`branch.<branch>.pushRemote`), empty where it marks none. Rides
    /// the status because it is per-branch: the branch it was read for is
    /// the one in `status`.
    pub push_remote: String,
    /// Pending files whose change has something to say about line endings.
    pub eol_marks: Arc<Vec<platitude_core::session::EolMark>>,
    /// Why a standing rebase is standing — the `edit` stop that a clean
    /// tree cannot be told from an empty stop by. Default while none is.
    pub stop: platitude_core::integrate::RebaseStop,
}

/// What the rebase-plan feed carries: the rows the interactive-rebase
/// screen opens over, or the refusal of this end's own that keeps it shut.
/// Either way `from` says which click it answers — the click that asked
/// may be behind another by the time this lands.
#[derive(Debug)]
pub enum PlanMsg {
    Loaded {
        preview: platitude_core::rebase_plan::PlanPreview,
    },
    Refused {
        from: String,
        refusal: platitude_core::rebase_plan::PlanRefusal,
    },
    /// The read failed (the failure itself reaches the error surface);
    /// this puts the model's waiting state down.
    Failed { from: String },
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
        /// What changed inside each row (`intraline`) — rides with the
        /// rows, not behind them like the colours.
        marks: Arc<platitude_core::intraline::IntraMarks>,
    },
    /// Colours for the lines of the diff named by `target`, addressed the
    /// way the rows are. Never arrives for a diff nobody is on.
    Coloured {
        target: DiffTarget,
        colors: platitude_core::highlight::DiffColors,
        /// Whether these are the colours the diff ends on (see
        /// `SessionEvent::DiffColoured`).
        settled: bool,
    },
}

impl DiffMsg {
    pub fn target(&self) -> &DiffTarget {
        match self {
            DiffMsg::Loaded { target, .. } | DiffMsg::Coloured { target, .. } => target,
        }
    }
}

/// What the details feed carries: the answer, or word that none is
/// coming — the pane holds a spinner against the oid it asked for, and
/// only a message can take it down.
#[derive(Debug)]
pub enum DetailsMsg {
    Loaded {
        generation: u64,
        details: Box<platitude_core::details::CommitDetails>,
    },
    /// Only the consumer of the matching request may surface this error.
    Failed {
        generation: u64,
        oid_hex: String,
        message: String,
    },
}

impl DetailsMsg {
    pub fn generation(&self) -> u64 {
        match self {
            Self::Loaded { generation, .. } | Self::Failed { generation, .. } => *generation,
        }
    }
}
