//! What the background threads send the main thread, one enum per
//! feed.

use super::*;

/// What came back about a folder asked for, before it is a tab. Every road
/// in waits for one (`strip::landing_for` answers it).
#[derive(Debug)]
pub enum OpenMsg {
    /// The worktree the folder is in, and its repository
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
/// yet (`CloneDialog.qml`).
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
        /// Why a remote could not be reached, as the word the command
        /// log's header line is written from (`Words.unreachedWhy`); empty
        /// for every other failure, whose header line is git's first line
        /// ([`platitude_core::remote::unreached`]).
        why: String,
    },
    /// git stopped part-way through the write in flight and left the
    /// operation standing. Arrives before the `WriteState` ending that
    /// write, whose `kind` says which write it was.
    WriteStopped,
    /// A read the page's pace started has ended (`SessionEvent::PacedRead`):
    /// this tree's where `worktree` is `None`, else that worktree's, by the path git
    /// printed.
    PacedRead {
        worktree: Option<String>,
    },
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
        /// (`WorktreeModel.headSeq`).
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
    /// coming for it. Published onto the other feeds, ahead of this
    /// message; whether their drains have run by the time the tab's takes
    /// it is not this message's to say, so the tab waits them out
    /// (`RepoTab::settle_heard`) before a run photographing the page the
    /// write leaves goes on (`AutoActDriver.writeBarrier`). The id alone:
    /// two writes of a kind answer alike (`write_watch`).
    WriteSettled {
        id: u64,
    },
    /// The status read the queue made behind that write did not land
    /// (`SessionEvent::WriteSettled::failed` naming `FollowUp::Status`),
    /// pushed ahead of its `WriteSettled`. A stage press that moved every
    /// row it names stops holding the doors on it
    /// (`ops::StageAhead::read_failed`); the press itself stands until the
    /// next status that lands.
    StatusUnread {
        id: u64,
    },
    /// Pushed by the hub when what a status reader drew moved the stage
    /// presses — one let go, or the doors of one whose status read failed
    /// (`ops::StageAhead::take_notice`): the tab reads the presses again —
    /// its Commit button waits on them (`RepoTab::read_writes_out`).
    PressLetGo,
    /// A branch move would take `lost` commits off the branch and was not
    /// made.
    MoveNeedsAsk {
        local: String,
        start: String,
        lost: i32,
    },
    /// What the discard log lists changed under a write
    /// (`SessionEvent::DiscardsChanged`): `restored` is how each part a
    /// restore brought back came back, empty for a write's record.
    DiscardsChanged {
        restored: Vec<platitude_core::discards::Restored>,
    },
    /// Whether a remote already carries a branch name. The question rides
    /// along: the asking box may be on another name by now, or the
    /// question on another branch (`local`).
    RemoteBranch {
        target: platitude_core::remote::RemoteBranch,
        local: String,
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
    /// The read behind [`Self::Author`] failed (`SessionEvent::AuthorUnread`).
    AuthorUnread,
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
    /// Whether an amend pressed now would leave HEAD changing nothing.
    AmendChecked {
        empties: bool,
    },
    /// A fetch nobody asked for (the interval's, or an opening's) started
    /// or ended. Kept off the error surface — the toolbar indicator
    /// carries it, so an offline machine stays quiet. Under the write's
    /// own id and kind: the indicator turns until that write's settle,
    /// which comes as any other's (`WriteSettled`).
    AutoFetch {
        id: u64,
        kind: platitude_core::OperationKind,
        running: bool,
        error: String,
    },
}

impl TabMsg {
    /// A failure for the command log's header line, with the word the
    /// line is written from where the failure is a remote out of reach.
    pub fn op_error(message: String) -> Self {
        let why = Self::unreached_word(&message);
        TabMsg::OpError { message, why }
    }

    /// The header line's word for a remote that could not be reached
    /// ([`platitude_core::remote::unreached`]), empty for any other
    /// failure.
    pub fn unreached_word(said: &str) -> String {
        platitude_core::remote::unreached(said)
            .map_or("", |why| why.word())
            .to_string()
    }
}

/// Where HEAD stands, from the one record (`session::standing`) — handed
/// to every consumer that draws something at HEAD, so none reads a source
/// of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadMsg {
    /// The commit; none on a branch with no commits yet.
    pub oid: Option<platitude_core::Oid>,
    /// The branch, empty when detached or unborn-without-a-name.
    pub branch: String,
    pub detached: bool,
    /// While detached, the branch a rebase or bisect keeps to the worktree
    /// (`HeadState::held`); empty otherwise.
    pub held: String,
    /// The report's number (`SessionEvent::HeadObserved::seq`), rising
    /// across sessions. A report at or above the `head_seq` a write's
    /// answer named looked after the write.
    pub seq: u64,
}

impl HeadMsg {
    pub fn of(head: &platitude_core::refs::HeadState, seq: u64) -> Self {
        Self {
            oid: head.oid,
            branch: head.branch.clone().unwrap_or_default(),
            detached: head.detached,
            held: head.held.clone().unwrap_or_default(),
            seq,
        }
    }
}

/// The headline consumer's feed (`WorktreeModel`) — one queue, so the
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
/// stash is a read of its own, landing apart from the refs: a dropped row
/// measured against the refs' stamp would come back until it lands.
#[derive(Debug)]
pub struct StashList {
    pub entries: Vec<StashEntry>,
    pub looked: u64,
}

/// The worktrees, and when the read that listed them looked
/// (`SessionEvent::WorktreesLoaded`) — a removed worktree's row waits on it, as
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
    /// Where HEAD stands — finds the worktree's row and starts a range
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
    /// The graph laid out again without walking (see
    /// [`SessionEvent::LogRelaid`]): the rows it keeps come by id, from the
    /// graph `from` or from what an earlier one set aside.
    Relaid {
        generation: u64,
        from: u64,
        rows: Vec<RelaidRow>,
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
    /// The graph's walk has gone silent, or a pass has answered since
    /// (see [`SessionEvent::LogSilent`]). No generation, as `Stale`.
    Silent {
        silent: bool,
    },
    /// The walk that put the discard log's entry on the graph has landed
    /// (see [`SessionEvent::DiscardWalked`]): the entry's first tip, full
    /// hex, empty for the walk that took one off.
    DiscardWalked {
        tip: String,
    },
}

/// Command-log messages: one git invocation, start and end, and the end of
/// a write's commands.
///
/// Both halves of an invocation carry `run`, the session they came from:
/// the log outlives sessions, each numbers its invocations from one, and
/// matched by id alone a new session's first command would answer for the
/// old one's row (`CommandsModel::Invocation`).
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
        /// The write it ran under (`OperationId::as_u64`), 0 for a read
        /// outside any.
        operation: u64,
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
    /// The write `operation` has settled, and every command it ran was
    /// said ahead of this on the same feed: a write's run has returned
    /// before its `SessionEvent::WriteSettled` goes out, and each command
    /// says its start and end from inside that run
    /// (`process::GitExecutor::execute`). The log's receipt of a write's
    /// records, a write with no command included (`CommandsModel.recorded`).
    Settled { operation: u64 },
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

/// What another worktree is holding, for the read-only pane
/// (`SessionEvent::CarriedStatusLoaded`). Not a [`StatusMsg`]: the rest of
/// that is about the tree this window can write.
#[derive(Debug)]
pub struct CarriedStatusMsg {
    /// The worktree this is about, as git printed its path. Only the
    /// worktree the pane stands on is sent (`RepoSession::read_carried_status`).
    pub at: String,
    /// The worktree's own name, for the band that says whose files these are.
    pub name: String,
    pub status: WorktreeStatus,
}

/// What a reader holding `status` weighs for it in a memory report: the
/// whole of it where the reader is its one holder — a picture drawn with
/// the stage presses in it, another worktree's rows — and nothing where it
/// is shared ([`StatusMsg::status`]): the session counts the one it keeps
/// (`status-told`), and counted at every holder one allocation would read
/// as five. Once the session has gone on to the next status, an earlier one
/// still held by two readers or more is counted by nobody.
#[must_use]
pub fn own_status_bytes(status: &Arc<WorktreeStatus>) -> usize {
    use platitude_core::mem::Footprint as _;
    if Arc::strong_count(status) == 1 {
        status.heap_bytes()
    } else {
        0
    }
}

#[derive(Debug)]
pub struct StatusMsg {
    /// The session's own allocation (`SessionEvent::StatusLoaded::status`),
    /// shared by every reader it is handed to: a status is as long as the
    /// tree is dirty, and a copy per reader would be made on every read.
    pub status: Arc<WorktreeStatus>,
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
    /// What the tick read of the push of the status's branch. Rides the
    /// status because it is per-branch: read for the branch in `status`.
    pub push: platitude_core::remote::StatusPush,
    /// Pending files whose change has something to say about line endings.
    pub eol_marks: Arc<Vec<platitude_core::session::EolMark>>,
    /// Pending files that need Git LFS where git cannot run it.
    pub lfs_needed: usize,
    /// Of those, the paths a discard could not copy (the filter is
    /// required): their discard is not brought back.
    pub not_copied: Arc<Vec<String>>,
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
        /// The pane's number for the ask this answers
        /// (`SessionEvent::DiffLoaded::asked`).
        asked: u64,
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
        /// that has no patch: a repository of its own inside the working tree
        /// (`details::embedded`).
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
    /// The diff of `target` could not be read: the pane stops waiting
    /// (git's words are on the tab's error surface).
    Unread { target: DiffTarget },
}

impl DiffMsg {
    pub fn target(&self) -> &DiffTarget {
        match self {
            DiffMsg::Loaded { target, .. }
            | DiffMsg::Coloured { target, .. }
            | DiffMsg::Unread { target } => target,
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
