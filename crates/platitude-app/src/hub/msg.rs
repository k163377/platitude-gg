//! What the background threads send the main thread, one enum per
//! feed.

use super::*;

/// What came back about a folder somebody asked for, before it is a tab.
///
/// **Every road in waits for one of these** (デザイン規約 §タブの所作
/// 「判定は 1 か所に置く」): the working copy a folder opens and the
/// repository that copy hangs off are git's to say, and until they are
/// said there is no telling a second copy of what is already open from a
/// repository nobody has opened yet.
#[derive(Debug)]
pub enum OpenMsg {
    /// It opens, and here is where: the working copy the asked-for
    /// folder is in, and the repository it belongs to
    /// ([`platitude_core::repo::Place`]).
    Placed { place: platitude_core::repo::Place },
    /// It does not. `kind` is `plain` / `bare` / `other` — the same three
    /// the tab's own failure screen names, which is where every road but
    /// the picker's shows this (デザイン規約 §可否・警告の出し場所).
    Refused {
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
    /// one to open — git was told where to put it, so this is where it
    /// is.
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
    /// write it was is that answer's own `kind`.
    WriteStopped,
    /// A write command started / ended. A failed write also arrives as an
    /// `OpError`, so the existing error surface needs no special case;
    /// `error` is here as well so an editor can tell whether the write it
    /// asked for is the one that failed.
    ///
    /// **Except the failures something else reports**: where the write
    /// did not happen and something outside this application said so,
    /// `report` carries what it said and no `OpError` is sent — the page
    /// answers it with a notice of its own (デザイン規約 §答えの要らない報せ).
    WriteState {
        /// The id the queue handed back when the write was asked for
        /// (`platitude_core::OperationId`) — what a reader waiting on its
        /// own write matches, whatever answered in between.
        id: u64,
        /// Which write — the queue's own kind, carried as it is
        /// (`platitude_core::OperationKind`). What an answer means is
        /// read off it on this side of the bridge (`drain::settle_write`),
        /// and the word QML shows is made from it there and nowhere
        /// earlier.
        kind: platitude_core::OperationKind,
        running: bool,
        error: String,
        report: Option<platitude_core::WriteReport>,
        /// The smallest number the first report of HEAD after this write
        /// can carry (`SessionEvent::WriteFinished::head_seq`); 0 while
        /// the write is running. What a landing on the write's tip arms
        /// on (`WorkTreeModel.headSeq`).
        head_seq: u64,
        /// The smallest stamp a listing that looked after this write can
        /// carry (`SessionEvent::WriteFinished::reads_from`); 0 while the
        /// write is running. What rows held off the screen for the write
        /// are put down on — the listings travel feeds of their own and
        /// are applied in no fixed order against this one, so arriving is
        /// not evidence and only the stamp is (`ops::StandIn`).
        reads_from: u64,
    },
    /// Everything that write invalidated has been read again and
    /// published (`SessionEvent::WriteSettled`) — the last of its three
    /// boundaries, and the only one that speaks for the readings behind
    /// the answer.
    ///
    /// **Published.** What the window is showing is the window's own
    /// business; this says only that nothing further is coming for that
    /// write, which is what a run waiting to photograph the page a write
    /// leaves has no other way to know
    /// (`AutoActDriver.writeBarrier`).
    ///
    /// The id and nothing else: what a reader does with it is match its
    /// own, so the kind would only be a second way to ask the same
    /// question — and the wrong one, since two writes of a kind answer
    /// alike (`write_watch`).
    WriteSettled {
        id: u64,
    },
    /// A branch move would leave commits unreachable and was not made.
    MoveNeedsAsk {
        local: String,
        start: String,
    },
    /// Whether a remote already carries a branch name, as of now. The
    /// question rides along: the box that asked may be on another name
    /// by the time this arrives.
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
        /// `"yes"` / `"no"` / `"unknown"` — the same three the rows answer
        /// this with (`GraphModel::branch_delete_merged`), so a reader
        /// need not know which side spoke.
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
    /// one an opening fires. Kept off the shared error surface: the
    /// toolbar indicator is what carries this state, so a laptop that is
    /// simply offline stays quiet.
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

/// Where HEAD stands, as the one record has it (`session::standing`) —
/// the message every consumer that draws something *at* HEAD is handed,
/// so none of them reads it out of a source of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadMsg {
    /// The commit, hex; empty on a branch with no commits yet.
    pub oid_hex: String,
    /// The branch, empty when detached or unborn-without-a-name.
    pub branch: String,
    pub detached: bool,
    /// The report's number (`SessionEvent::HeadObserved::seq`): a move,
    /// or the first read after a write, in the order accepted and across
    /// sessions. A consumer armed on a write's answer holds the number
    /// that answer named, and a report at or above it looked after the
    /// write.
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

/// The headline consumer's feed (`WorkTreeModel`): where the tree stands
/// and what the last status found on it — one queue, in the order the
/// session said them, so the counts never arrive ahead of the branch
/// they are about.
#[derive(Debug)]
pub enum StateMsg {
    Head(HeadMsg),
    Status(Box<StatusMsg>),
    /// Whether a remote already has the commit HEAD is on. `oid_hex` is
    /// the commit the answer is about, empty on an unborn branch; a
    /// consumer holding a HEAD this is not yet about shows no warning.
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

/// What the stashes section is handed: the entries, and when the read
/// that listed them looked (`SessionEvent::StashesLoaded`).
///
/// The stash is read behind a flight of its own, after the graph is
/// rebuilt, so it carries a stamp of its own: a dropped row measured
/// against the refs' listing would come back for the whole of that
/// rebuild.
#[derive(Debug)]
pub struct StashList {
    pub entries: Vec<StashEntry>,
    pub looked: u64,
}

/// What a sidebar refs section is handed: the snapshot its rows are, and
/// — for the branches section — where HEAD stands, so the row it
/// highlights and the stand-in it rides are the one record's
/// (`session::standing`).
#[derive(Debug)]
pub enum RefsMsg {
    Snapshot {
        snapshot: Arc<RefsSnapshot>,
        /// When the pass that read this looked at the repository
        /// (`SessionEvent::RefsLoaded::looked`) — **the pass's own
        /// moment**, ahead of the three the three sections apply it at.
        /// A consumer holding rows off the screen for a write measures
        /// this against the write's `reads_from`.
        looked: u64,
    },
    Head(HeadMsg),
}

/// Graph-model messages (log stream lifecycle).
#[derive(Debug)]
pub enum GraphMsg {
    /// Where HEAD stands — what the row the working tree is on is found
    /// by, and what a range asked of the rows starts from. Carried on
    /// this feed: the chips arrive a pass after the rows, and a move
    /// lands before either.
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
    /// The graph standing on screen has fallen behind the repository, or
    /// has caught up with it again (see [`SessionEvent::LogStale`]). No
    /// generation: what it describes is the whole picture, whatever
    /// stream drew it.
    Stale {
        stale: bool,
    },
}

/// Command-log messages: one git invocation, start and end.
///
/// **Both halves name the session they came from** (`run`), because the
/// log outlives it: a tab standing in another working copy keeps its
/// rows and opens a session over the same feeds, and that session counts
/// its invocations from one like every other. Matched by the id alone,
/// its first command would answer for the row the last session's first
/// command left (`CommandsModel::Invocation`).
#[derive(Debug)]
pub enum CommandMsg {
    Started {
        run: u64,
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
        /// What the command waited for a slot before it was spawned —
        /// the application's time, shown apart from git's
        /// (`platitude_core::process::Slots`).
        waited_ms: i64,
        elapsed_ms: i64,
        message: String,
    },
}

/// The badge's own two halves, arriving on their own several times a
/// second while a write that replays is out (`SessionEvent::OpProgress`).
/// A slice of [`StatusMsg`] — the same consumer reads both, and this one
/// costs no git process.
#[derive(Debug)]
pub struct OpProgressMsg {
    pub op_state: OpState,
    pub progress: Option<platitude_core::conflict::Progress>,
}

/// What another working copy is holding, for the read-only pane
/// (`SessionEvent::CarriedStatusLoaded`).
///
/// **Its own message**, though the lists it feeds are the same three:
/// everything else on a [`StatusMsg`] is about the tree this window can
/// write — the HEAD its counts stand beside, what git is in the middle of
/// here, where a push would go — and a copy has none of it to give. What
/// is left is the files, and the copy they belong to.
#[derive(Debug)]
pub struct CarriedStatusMsg {
    /// The copy this is about, as git printed its path. The page holds
    /// the one it is showing and drops anything else.
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
    /// the config again for itself.
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
    /// The count a remote already has of the range, asked again after
    /// the refs moved under the open plan. Names the range, so a plan
    /// since put away drops it.
    Published { range: String, published: i32 },
}

/// What the diff feed carries. Two messages, because the colours come
/// out behind the rows they belong to — see
/// [`platitude_core::session::SessionEvent::DiffColoured`] for why.
#[derive(Debug)]
pub enum DiffMsg {
    Loaded {
        target: DiffTarget,
        patches: Arc<Vec<FilePatch>>,
        /// Boxed because it is most of this message: the sides carry a
        /// path and a size each, and a feed holding several of these
        /// would pay that for every message on it — including the
        /// colours, which have no preview at all.
        preview: Option<Box<FilePreview>>,
        /// Fingerprint of the diff's source bytes; selections carry it
        /// back so a partial write can refuse a drifted diff.
        fingerprint: u64,
        /// What the same bytes said about line endings, if anything.
        endings: Option<platitude_core::eol::Notice>,
        /// What changed inside each row (`intraline`) — rides with the
        /// rows themselves, ahead of the colours.
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

/// What the details feed carries: the answer, or word that none is
/// coming — the pane holds a spinner against the oid it asked for, and
/// only a message can take it down.
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
