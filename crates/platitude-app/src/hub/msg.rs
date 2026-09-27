//! What the background threads send the main thread, one enum per
//! feed.

use super::*;

/// What came back about a folder asked for, before it is a tab. Every road
/// in waits for one (デザイン規約 §タブの所作「判定は 1 か所に置く」).
#[derive(Debug)]
pub enum OpenMsg {
    /// The working copy the folder is in, and its repository
    /// ([`platitude_core::repo::Place`]).
    Placed { place: platitude_core::repo::Place },
    /// `kind` is `plain` / `bare` / `other`, the three the tab's failure
    /// screen names — where every road but the picker's shows this
    /// (デザイン規約 §可否・警告の出し場所).
    Refused {
        path: PathBuf,
        /// The folder to reopen the picker at — the one this sits in.
        near: String,
        kind: &'static str,
        /// git's own words, for `other` alone.
        message: String,
    },
}

/// What became of a clone, for the dialog that asked — there is no tab
/// yet (デザイン規約 §リポジトリを取り寄せる).
#[derive(Debug)]
pub enum CloneMsg {
    /// The folder that was named — git was told to put it there.
    Done { path: PathBuf },
    /// `message` is git's own answer, quoted by the dialog and not
    /// classified here.
    Failed { message: String },
}

/// Tab-level messages (open lifecycle + background errors + write state).
#[derive(Debug)]
pub enum TabMsg {
    Opened {
        title: String,
        path: String,
    },
    /// The repository behind this tab would not open. `kind` as in
    /// `OpenMsg::Refused`; `message` is git's own, shown only for `other`.
    OpenFailed {
        kind: &'static str,
        path: String,
        message: String,
    },
    OpError {
        message: String,
    },
    /// git stopped part-way through the write in flight and left the
    /// operation standing. Arrives before the `WriteState` ending that
    /// write, whose `kind` says which write it was.
    WriteStopped,
    /// A write started / ended. A failed write also arrives as an
    /// `OpError`; `error` is here too so an editor can tell whether its
    /// own write failed — except where `report` is set: then no `OpError`
    /// is sent and the page shows a notice (デザイン規約 §答えの要らない報せ).
    WriteState {
        /// The id handed back when the write was asked for
        /// (`platitude_core::OperationId`) — what a reader waiting on its
        /// own write matches.
        id: u64,
        /// The queue's kind as it is; what an answer means, and the word
        /// QML shows, are made from it in `drain::settle_write`.
        kind: platitude_core::OperationKind,
        running: bool,
        error: String,
        report: Option<platitude_core::WriteReport>,
        /// The smallest number the first HEAD report after this write can
        /// carry (`SessionEvent::WriteFinished::head_seq`); 0 while
        /// running. What a landing on the write's tip arms on
        /// (`WorkTreeModel.headSeq`).
        head_seq: u64,
        /// The smallest stamp a listing that looked after this write can
        /// carry (`SessionEvent::WriteFinished::reads_from`); 0 while
        /// running. Rows held off screen for the write are put back on it:
        /// listings arrive on other feeds in no fixed order, so only the
        /// stamp is evidence (`ops::StandIn`).
        reads_from: u64,
    },
    /// Everything that write invalidated has been read again and
    /// published (`SessionEvent::WriteSettled`) — nothing further is
    /// coming for it, which says nothing about what the window has drawn.
    /// What a run photographing the page a write leaves waits on
    /// (`AutoActDriver.writeBarrier`). The id alone: two writes of a kind
    /// answer alike (`write_watch`).
    WriteSettled {
        id: u64,
    },
    /// A branch move would leave commits unreachable and was not made.
    MoveNeedsAsk {
        local: String,
        start: String,
    },
    /// Whether a remote already carries a branch name. The question rides
    /// along: the asking box may be on another name by now.
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
    /// Whether `branch --delete` would go through. The branch rides along:
    /// the menu may be over another row by now.
    BranchDelete {
        branch: String,
        /// `"yes"` / `"no"` / `"unknown"`, as the rows answer it
        /// (`GraphModel::branch_delete_merged`).
        merged: &'static str,
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
        /// `commit.gpgsign` alone: the commit editor is what says so,
        /// and `tag.gpgsign` would make it say it falsely.
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
    /// Configured remote names and which one pushes go to. `push_default`
    /// is empty where none is marked; `push_default_local` says whether
    /// this repository's own config marked it — a mark from elsewhere
    /// cannot be cleared here.
    Remotes {
        names: Vec<String>,
        /// Their fetch URLs, in the same order.
        urls: Vec<String>,
        push_default: String,
        push_default_local: bool,
        /// `checkout.defaultRemote`, the other key marking a remote as
        /// origin; empty where unset.
        checkout_default: String,
    },
    /// HEAD's message and author, for prefilling an amend.
    HeadCommit {
        message: String,
        author_name: String,
        author_email: String,
    },
    /// A fetch nobody asked for (the interval's, or an opening's) started
    /// or ended. Kept off the error surface — the toolbar indicator
    /// carries it, so an offline machine stays quiet.
    AutoFetch {
        running: bool,
        error: String,
        /// Whether a failure may raise the command log. An opening's fetch
        /// says no: opening a tab is not a request to reach the network.
        announce: bool,
    },
}

/// Where HEAD stands, from the one record (`session::standing`) — handed
/// to every consumer that draws something at HEAD, so none reads a source
/// of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadMsg {
    /// The commit, hex; empty on a branch with no commits yet.
    pub oid_hex: String,
    /// The branch, empty when detached or unborn-without-a-name.
    pub branch: String,
    pub detached: bool,
    /// The report's number (`SessionEvent::HeadObserved::seq`), rising
    /// across sessions. A report at or above the `head_seq` a write's
    /// answer named looked after the write.
    pub seq: u64,
}

impl HeadMsg {
    pub fn of(head: &platitude_core::refs::HeadState, seq: u64) -> Self {
        Self {
            oid_hex: head.oid.map(|o| o.to_hex()).unwrap_or_default(),
            branch: head.branch.clone().unwrap_or_default(),
            detached: head.detached,
            seq,
        }
    }
}

/// The headline consumer's feed (`WorkTreeModel`) — one queue, so the
/// counts never arrive ahead of the branch they are about.
#[derive(Debug)]
pub enum StateMsg {
    Head(HeadMsg),
    Status(Box<StatusMsg>),
    /// Whether a remote already has the commit HEAD is on. `oid_hex` is
    /// that commit, empty on an unborn branch; a consumer holding another
    /// HEAD shows no warning.
    HeadPublished {
        oid_hex: String,
        published: bool,
    },
    /// Whether anything besides the current branch still reaches its
    /// tip, which is what a rewrite here would cost.
    HeadReach {
        reached_elsewhere: bool,
    },
}

/// The stashes section's entries, and when the read that listed them
/// looked (`SessionEvent::StashesLoaded`). A stamp of its own because the
/// stash is read after the graph rebuild: a dropped row measured against
/// the refs' stamp would come back for the whole rebuild.
#[derive(Debug)]
pub struct StashList {
    pub entries: Vec<StashEntry>,
    pub looked: u64,
}

/// The working copies, and when the read that listed them looked
/// (`SessionEvent::WorktreesLoaded`) — a removed copy's row waits on it, as
/// a dropped stash waits on [`StashList`].
#[derive(Debug)]
pub struct WorktreeList {
    pub entries: Vec<platitude_core::worktrees::WorktreeEntry>,
    pub looked: u64,
}

/// A sidebar refs section's snapshot, and — for the branches section —
/// where HEAD stands (`session::standing`).
#[derive(Debug)]
pub enum RefsMsg {
    Snapshot {
        snapshot: Arc<RefsSnapshot>,
        /// When the pass that read this looked
        /// (`SessionEvent::RefsLoaded::looked`) — the pass's moment, not
        /// when a section applies it. Measured against a write's
        /// `reads_from`.
        looked: u64,
    },
    Head(HeadMsg),
}

/// Graph-model messages (log stream lifecycle).
#[derive(Debug)]
pub enum GraphMsg {
    /// Where HEAD stands — finds the working tree's row and starts a range
    /// asked of the rows. Its own message: the chips arrive a pass after
    /// the rows, and a move lands before either.
    Head(HeadMsg),
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
    /// The graph on screen has fallen behind the repository, or caught up
    /// (see [`SessionEvent::LogStale`]). No generation: it is about the
    /// whole picture, whatever stream drew it.
    Stale {
        stale: bool,
    },
}

/// Command-log messages: one git invocation, start and end.
///
/// Both halves carry `run`, the session they came from: the log outlives
/// sessions, each numbers its invocations from one, and matched by id
/// alone a new session's first command would answer for the old one's
/// row (`CommandsModel::Invocation`).
#[derive(Debug)]
pub enum CommandMsg {
    Started {
        run: u64,
        id: u64,
        display: String,
        full: String,
        at_ms: i64,
        /// Whether the reader asked for it. An unasked fetch arrives only
        /// when git said no, and its row neither reddens the mark nor
        /// raises the panel (デザイン規約 §git が言ったことを読む場所).
        asked: bool,
    },
    Finished {
        run: u64,
        id: u64,
        /// Exit code, or `None` when git never ran to a code of its own
        /// (killed by the timeout, cancelled, failed to start).
        code: Option<i32>,
        /// Why there is no code, for the row to say instead.
        note: String,
        /// The code is an answer this command was asked for, so the row
        /// is not a failure however it exited.
        answered: bool,
        /// Time spent waiting for a slot before the spawn — the
        /// application's, shown apart from git's
        /// (`platitude_core::process::Slots`).
        waited_ms: i64,
        elapsed_ms: i64,
        message: String,
    },
}

/// The badge's two halves, several times a second while a replaying write
/// is out (`SessionEvent::OpProgress`). A slice of [`StatusMsg`] that
/// costs no git process.
#[derive(Debug)]
pub struct OpProgressMsg {
    pub op_state: OpState,
    pub progress: Option<platitude_core::conflict::Progress>,
    /// When it was read (`SessionEvent::OpProgress::looked`).
    pub looked: u64,
}

/// What another working copy is holding, for the read-only pane
/// (`SessionEvent::CarriedStatusLoaded`). Not a [`StatusMsg`]: the rest of
/// that is about the tree this window can write.
#[derive(Debug)]
pub struct CarriedStatusMsg {
    /// The copy this is about, as git printed its path. The page drops
    /// any copy but the one it is showing.
    pub at: String,
    /// The copy's own name, for the band that says whose files these are.
    pub name: String,
    pub status: WorkTreeStatus,
}

#[derive(Debug)]
pub struct StatusMsg {
    pub status: WorkTreeStatus,
    /// The number of the report of HEAD this status stands beside
    /// (`SessionEvent::StatusLoaded::head_seq`): the counts are about
    /// the HEAD the consumer holds only while the two numbers agree.
    pub head_seq: u64,
    /// When the tree was looked at (`SessionEvent::StatusLoaded::looked`):
    /// orders `op_state` and `progress` against [`OpProgressMsg`].
    pub looked: u64,
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
    /// the config again.
    pub merge_tool: String,
    /// Where this branch's own mark sends a push
    /// (`branch.<branch>.pushRemote`), empty where it marks none. Rides the
    /// status because it is per-branch: read for the branch in `status`.
    pub push_remote: String,
    /// Pending files whose change has something to say about line endings.
    pub eol_marks: Arc<Vec<platitude_core::session::EolMark>>,
    /// Why a standing rebase is standing — the `edit` stop that a clean
    /// tree cannot be told from an empty stop by. Default while none is.
    pub stop: platitude_core::integrate::RebaseStop,
}

/// The rebase-plan feed: the rows the interactive-rebase screen opens
/// over, or the refusal that keeps it shut. `from` says which click it
/// answers — that click may be behind another by now.
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
    /// The count of the range a remote already has, asked again after the
    /// refs moved under the open plan. Names the range, so a plan since put
    /// away drops it.
    Published { range: String, published: i32 },
}

/// The diff feed: the rows, and their colours behind them (see
/// [`platitude_core::session::SessionEvent::DiffColoured`]).
#[derive(Debug)]
pub enum DiffMsg {
    Loaded {
        target: DiffTarget,
        patches: Arc<Vec<FilePatch>>,
        /// Boxed: it is most of this message, and unboxed every message on
        /// the feed — colours included — would pay its size.
        preview: Option<Box<FilePreview>>,
        /// Fingerprint of the diff's source bytes; selections carry it
        /// back so a partial write can refuse a drifted diff.
        fingerprint: u64,
        /// What the same bytes said about line endings, if anything.
        endings: Option<platitude_core::eol::Notice>,
        /// What changed inside each row (`intraline`), ahead of the colours.
        marks: Arc<platitude_core::intraline::IntraMarks>,
        /// The commit a stage of this row would point at, for the one row
        /// that has no patch: a repository of its own inside the working
        /// copy (`details::embedded`).
        embedded: Option<platitude_core::details::Embedded>,
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

/// The details feed: the answer, or word that none is coming — only a
/// message can take down the pane's spinner.
#[derive(Debug)]
pub enum DetailsMsg {
    Loaded {
        generation: u64,
        details: Box<platitude_core::details::CommitDetails>,
    },
    /// What a choice of several commits changed — the file list alone
    /// (デザイン規約 §複数のコミットを選ぶ).
    Selection {
        generation: u64,
        files: Vec<platitude_core::parse::name_status::FileChange>,
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
            Self::Loaded { generation, .. }
            | Self::Selection { generation, .. }
            | Self::Failed { generation, .. } => *generation,
        }
    }
}
