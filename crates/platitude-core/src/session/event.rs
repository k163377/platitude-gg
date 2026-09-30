//! Everything the session can tell the UI, as one enum.

use super::*;

#[derive(Debug)]
pub enum SessionEvent {
    Opened {
        info: RepoInfo,
    },
    /// The repository would not open. `path` is the one asked for: only
    /// some errors carry it, and the screen names the folder for all.
    OpenFailed {
        path: PathBuf,
        error: GitError,
    },
    /// A (re)load of the graph began; the model must reset.
    LogStarted {
        generation: u64,
    },
    LogChunk {
        generation: u64,
        rows: Vec<LogRow>,
    },
    LogFinished {
        generation: u64,
        total: u32,
        elapsed_ms: u64,
        /// Commits the walk emitted — what `--max-count` caps, so the
        /// window limit whenever `truncated`. The truncation footer reads
        /// this, not `total`: the WIP row and sifted stash parents move
        /// the shown-row count off any round figure.
        walked: u32,
        /// True when the stream stopped at the configured window limit.
        truncated: bool,
    },
    LogFailed {
        generation: u64,
        error: String,
    },
    /// Whether the graph on screen is known to be out of date: whole, but
    /// an off-screen rebuild that should have replaced it did not
    /// (`RepoSession::run_swap_pass`). No error here — that went out as
    /// [`SessionEvent::OpFailed`].
    ///
    /// Sent on the turn only, hence the flag: a rebuild answering
    /// [`RefreshOutcome::Unchanged`] sends nothing (a quiet tick must not
    /// wake the consumer), so a mark that could only be put on would stand
    /// for the rest of the session.
    LogStale {
        stale: bool,
    },
    /// A background rebuild replaces the whole graph in one step. One
    /// event on purpose: split, a consumer draining between the messages
    /// paints an empty model for a frame (a white flash).
    LogReplaced {
        generation: u64,
        rows: Vec<LogRow>,
        elapsed_ms: u64,
        /// As on [`SessionEvent::LogFinished`].
        walked: u32,
        truncated: bool,
    },
    /// The graph on screen laid out again without walking — a delete that
    /// is out taking the commits only it held off the screen, or a refused
    /// one putting them back (`session::leaving`). Commit rows come by id
    /// and new lanes ([`RelaidRow::Moved`]): the consumer already has the
    /// rest of them, from `from`, the graph it is being moved off; a
    /// consumer on another graph drops the whole message. A row it took
    /// away earlier can come back the same way, so it keeps those
    /// ([`RelaidRow`]).
    ///
    /// The footer is the one a walk without those commits would report:
    /// fewer walked where the window held every commit, the same where it
    /// was full — the walk that answers for the delete fills that from
    /// below.
    LogRelaid {
        generation: u64,
        from: u64,
        rows: Vec<RelaidRow>,
        /// As on [`SessionEvent::LogFinished`].
        walked: u32,
        truncated: bool,
    },
    /// Labels of already-delivered rows changed. `generation` names the
    /// graph the row numbers index; a consumer showing another graph
    /// drops them.
    LabelsChanged {
        generation: u64,
        rows: Vec<(u32, Vec<RefLabel>)>,
    },
    /// Where HEAD stands, per the newest read that reported it
    /// (`session::standing`). Sent when it moved, and once by the first
    /// read to land after a write even if it did not — the report a
    /// consumer waiting on the write's result is owed.
    ///
    /// `seq` numbers the reports in acceptance order, across sessions: a
    /// report at or above a write's
    /// [`WriteFinished::head_seq`](SessionEvent::WriteFinished) looked
    /// after that write.
    HeadObserved {
        head: HeadState,
        seq: u64,
    },
    /// Whether a remote already has the commit HEAD is on (the `already
    /// pushed` note an amend wears), read off the walk's marks
    /// (`session::published`); sent when the answer moves. `oid` is the
    /// commit it is about (`None` when unborn), so a consumer holding
    /// another HEAD can tell.
    HeadPublished {
        oid: Option<Oid>,
        published: bool,
    },
    /// Shared: every sidebar section is handed the whole snapshot, and a
    /// copy per section would duplicate tens of thousands of strings.
    RefsLoaded {
        snapshot: Arc<RefsSnapshot>,
        /// When the sending pass looked at the repository
        /// (`Standing::stamp`, taken before git is spawned). Measured
        /// against a write's [`reads_from`](Self::WriteFinished): at or
        /// above it, this listing saw what the write left; below, it says
        /// nothing about the write.
        ///
        /// Names this pass even when `snapshot` is an earlier pass's
        /// pointer (an unmoved repository rebuilds nothing).
        looked: u64,
    },
    /// What another working copy is holding, read because somebody is
    /// looking at its row (`RepoSession::read_carried_status`). Not
    /// [`SessionEvent::StatusLoaded`], which everything about this
    /// window's own tree hangs off.
    CarriedStatusLoaded {
        /// The copy this is about; the pane drops a read that lands after
        /// it moved on to another.
        path: String,
        /// The name the header says — the copy's own.
        name: String,
        status: WorkTreeStatus,
    },
    StatusLoaded {
        status: WorkTreeStatus,
        /// The [`HeadObserved::seq`](SessionEvent::HeadObserved) this
        /// status stands beside, taken after the read's own HEAD went into
        /// the record. Tells a consumer the counts are about the HEAD it
        /// holds; a consumer waiting on a write's tree waits for it.
        head_seq: u64,
        /// When the repository was looked at for this read, stamped before
        /// `git status` is spawned (`Standing::stamp`). The count in
        /// `progress` is read after the whole status, so an
        /// [`SessionEvent::OpProgress`] that looked later can be newer
        /// while it arrives first: the badge's count is whichever looked
        /// last.
        looked: u64,
        op_state: OpState,
        /// "commit N of M" while a rebase is stepping through commits.
        progress: Option<conflict::Progress>,
        /// What to call the two sides of a conflict, for whatever is
        /// stopped. Default while nothing is.
        sides: conflict::Sides,
        /// The message a stopped merge is about to record, comment lines
        /// stripped (`integrate::stopped_message`). Empty unless a merge
        /// is standing — the only operation finished from the commit box.
        op_message: String,
        /// The merge tool git would launch, empty when none is configured
        /// or when nothing is conflicted. Display only: the launch reads
        /// the config again.
        merge_tool: String,
        /// `branch.<branch>.pushRemote` for the status's branch, empty
        /// where unset. Rides the status so it is never read against
        /// another branch (the repository-wide mark rides the refs
        /// snapshot, `RepoSession::note_push_default`). Display and
        /// standing only — [`crate::remote::plan_current_push`] re-reads
        /// the keys to send.
        push_remote: String,
        /// The branch's counts against the tracking ref where a mark sends
        /// its push, when that is not the upstream's remote
        /// ([`crate::remote::pushes_elsewhere`]); empty everywhere else.
        /// Rides the status for the reason `push_remote` does.
        push_track: crate::remote::PushTrack,
        /// Pending paths whose change has something to say about line
        /// endings. Shared: most status reads repeat the previous answer.
        eol_marks: Arc<Vec<EolMark>>,
        /// Pending files that need Git LFS where git cannot run it; 0
        /// wherever it can, or where nothing pending needs it
        /// ([`crate::lfs`]).
        lfs_needed: usize,
        /// Why a standing rebase stopped: the tree alone cannot tell an
        /// `edit` stop from an empty one, and the exit card's words and
        /// `--skip`'s cost turn on which (デザイン規約 §進行中の操作から出る).
        /// Default while no rebase is.
        stop: integrate::RebaseStop,
    },
    /// What operation is standing and how far it has got — the badge's
    /// two halves and nothing else ([`RepoSession::refresh_op_progress`]),
    /// the live operation decided as [`integrate::InProgress::from_state`]
    /// does.
    ///
    /// Its own event because it is asked several times a second and costs
    /// no process (files under the git directory), where the status
    /// snapshot costs a whole-tree `git status`
    /// (`ci/baseline/poll-cost-windows-x64.md`). The state rides with the
    /// count: a short rebase is over before the poll tick that would have
    /// raised the badge.
    OpProgress {
        op_state: OpState,
        progress: Option<conflict::Progress>,
        /// When the git directory was read, on the same stamps as
        /// `StatusLoaded::looked`.
        looked: u64,
    },
    /// Answer to [`RepoSession::ask_merge_tools`]: names the settings field
    /// can offer, deliberate ones first. Empty is a valid answer.
    ///
    /// Sent twice where config named anything: those first, so the slow
    /// full read does not hold back a fast answer, then everything.
    /// `settled` marks the last.
    MergeToolsLoaded {
        names: Vec<String>,
        settled: bool,
    },
    /// Answer to [`RepoSession::check_remote_branch`]: whether the remote
    /// already carries that exact name. The question is echoed back: the
    /// asking box may have moved on to another name.
    RemoteBranchChecked {
        remote: String,
        branch: String,
        state: remote::RemoteBranchState,
        /// The commit the remote advertised (hex; empty where none) —
        /// what an overwrite leases against.
        tip: String,
        /// Commits that tip reaches and this history does not — what an
        /// overwrite would drop. Zero unless `Refused`, the only state
        /// that has both the commit and the walk.
        theirs: u32,
    },
    /// Answer to [`RepoSession::check_branch_delete`]: whether the branch
    /// is merged into the reference point `branch --delete` measures
    /// against (its upstream, or HEAD without one). The branch is echoed
    /// back: the asking menu may be over another row by then.
    BranchDeleteChecked {
        branch: String,
        /// `None` where the reads could not say. Drawn like merged (no
        /// `-D`: a delete not shown to be refused is offered plain, and git
        /// answers the press), but kept apart for reading a run afterwards
        /// — every ask is answered, so a bare delete row is one whose
        /// answer said so.
        merged: Option<bool>,
    },
    /// Answer to [`RepoSession::check_plan_published`]: how many commits
    /// of a plan's range a remote already has. The range is echoed back:
    /// the asking plan may have been replaced by then.
    PlanPublished {
        range: String,
        published: u32,
    },
    /// Answer to [`RepoSession::ask_rebase_plan`]: the rows the
    /// interactive-rebase screen opens over. `from` is echoed back (inside
    /// the preview) and `generation` numbers the ask: two clicks a moment
    /// apart need not finish in order.
    RebasePlanLoaded {
        generation: u64,
        preview: rebase_plan::PlanPreview,
    },
    /// The same question refused by this end: the range cannot be
    /// replayed, and the screen never opens. The screen writes the
    /// wording, so the kind travels.
    RebasePlanRefused {
        generation: u64,
        from: String,
        refusal: rebase_plan::PlanRefusal,
    },
    /// The same question, and the read failed. The error goes out as its
    /// own [`SessionEvent::OpFailed`]; this lets the asking model leave
    /// its waiting state.
    RebasePlanFailed {
        generation: u64,
        from: String,
    },
    /// Whether anything besides HEAD's branch still reaches its tip — so
    /// the rewrite rows can tell whether the old commits stay drawn or go
    /// to the reflog ([`crate::reachable`]). Sent when the answer moves.
    HeadReachChecked {
        reached_elsewhere: bool,
    },
    /// Answer to [`RepoSession::check_signature`].
    SignatureChecked {
        oid: String,
        signature: identity::Signature,
    },
    /// Answer to [`RepoSession::load_head_commit`] — what an amend starts
    /// from: HEAD's message, and whose commit it is about to replace. All
    /// empty on an unborn branch.
    HeadCommitLoaded {
        head: commit::HeadCommit,
    },
    /// Author identity and signing configuration. Emitted on open so the
    /// UI can ask for an identity before the first commit fails.
    AuthorLoaded {
        config: identity::AuthorConfig,
    },
    StashesLoaded {
        stashes: Vec<StashEntry>,
        /// When this listing looked, read like
        /// [`RefsLoaded::looked`](Self::RefsLoaded). Its own stamp because
        /// the stash is its own read, after the graph rebuild: measured
        /// against the refs' stamp, a dropped entry would reappear for the
        /// whole rebuild.
        looked: u64,
    },
    WorktreesLoaded {
        worktrees: Vec<crate::worktrees::WorktreeEntry>,
        /// When this listing looked, as [`Self::StashesLoaded`]: a removed
        /// copy's row waits on a listing that saw the removal.
        looked: u64,
    },
    DetailsLoaded {
        generation: u64,
        details: CommitDetails,
    },
    /// What a choice of several commits changed: the file list alone, the
    /// commits' rows being on screen already (デザイン規約 §複数のコミットを選ぶ).
    /// Shares the details numbering — both answer the same pane.
    SelectionLoaded {
        generation: u64,
        files: Vec<crate::parse::name_status::FileChange>,
    },
    /// A details failure is addressed to its request, including its error.
    /// The consumer surfaces it only if that request is still current.
    DetailsFailed {
        generation: u64,
        oid: Oid,
        error: GitError,
    },
    DiffLoaded {
        target: DiffTarget,
        /// Shared with the colouring that follows, which reads the same
        /// lines.
        patches: Arc<Vec<FilePatch>>,
        /// Image files / binary sizes when the text diff is not the whole
        /// story (`None` for ordinary text files).
        preview: Option<FilePreview>,
        /// Fingerprint of the bytes `patches` was parsed from. Hunk/line
        /// selections carry it back, so a partial write can refuse a diff
        /// that drifted under the selection (`stage::apply_partial`).
        fingerprint: u64,
        /// What the same bytes said about line endings, if anything. Rides
        /// with the diff: a notice that appears after the reader has
        /// started is worse than none.
        endings: Option<crate::eol::Notice>,
        /// What changed inside each row (`intraline`). With the rows, not
        /// behind them like colours: it is cheap, and where a change is is
        /// the first thing a reader looks for.
        marks: Arc<crate::intraline::IntraMarks>,
        /// For the one row that has no patch at all — a repository of its
        /// own inside the working copy — the commit a stage of it would
        /// point at (`details::embedded`). `None` for every other target.
        embedded: Option<crate::details::Embedded>,
    },
    /// Syntax colours for the lines of a diff already sent. Behind the
    /// rows: colouring a few thousand lines lands outside the 100ms an
    /// interaction is allowed (ci/baseline/code-costs-windows-x64.md §着色),
    /// so the rows go out as soon as git answers and the colours follow.
    ///
    /// Never sent for a diff the reader has left: colouring it would be
    /// paid out of the file they are on now (`RepoSession::diff_epoch`).
    DiffColoured {
        target: DiffTarget,
        colors: crate::highlight::DiffColors,
        /// Whether these are the colours the diff ends on. False on the
        /// quick first answer a deep fallback-lexer diff sends ahead of
        /// its full read — anything waiting for "the colours" waits for
        /// true, or it latches the interim (app-ui.md §UI 自動化).
        settled: bool,
    },
    /// A background refresh/query failed (op is a stable identifier).
    OpFailed {
        op: &'static str,
        error: GitError,
    },
    /// A branch move would leave commits unreachable, so it was not made.
    /// Nothing changed; the UI asks before running it for real
    /// ([`RepoSession::checkout`] with [`CheckoutTarget::ForceCreate`]).
    MoveNeedsAsk {
        local: String,
        start: String,
    },
    /// The queue took a write up: git is about to run it. `id` is the
    /// acceptance's ([`RepoSession::write`]); every later event about
    /// this write carries it.
    WriteStarted {
        id: OperationId,
        kind: OperationKind,
    },
    /// git stopped part-way through a write, leaving the operation
    /// standing to be finished. The
    /// [`WriteFinished`](SessionEvent::WriteFinished) that follows carries
    /// no error; this is what sends the screen to the conflicts.
    WriteStopped {
        id: OperationId,
        kind: OperationKind,
    },
    /// A write's git command ended — the second of its three boundaries,
    /// between acceptance and [`WriteSettled`](SessionEvent::WriteSettled);
    /// the reads it invalidated have not been made yet. `error` is git's
    /// own message.
    ///
    /// `report` is set where something outside this application refused
    /// the write and what it said is a report
    /// ([`crate::report::WriteReport`]); `error` still carries git's whole
    /// message for the log.
    ///
    /// `head_seq` is the smallest `seq` the first HEAD report after this
    /// write can carry (`Standing::fence`): the report at or above it
    /// answers a consumer waiting on the write, whichever arrives first.
    /// Unasked reads move it too.
    ///
    /// `reads_from` is the same fence for **listings**: the smallest stamp
    /// a read that looked after this write can carry, compared with the
    /// `looked` of [`RefsLoaded`](Self::RefsLoaded) and
    /// [`StashesLoaded`](Self::StashesLoaded). Counting arrivals cannot
    /// replace it: a listing in flight when the write ended says nothing
    /// about the write.
    WriteFinished {
        id: OperationId,
        kind: OperationKind,
        error: Option<String>,
        report: Option<crate::report::WriteReport>,
        head_seq: u64,
        reads_from: u64,
    },
    /// The reads the write invalidated have been made — the last of its
    /// three boundaries: nothing more is said under this id, and the
    /// queue takes the next request. Read by now: the tree and refs as the
    /// write's [`AfterWrite`] asks, the graph where either moved, the
    /// stash and worktree listings and the reads they asked for, and the
    /// author configuration after an identity write. Not the reachability
    /// walk (`settle_head_reach`), which answers when it can.
    ///
    /// `failed` names the reads that did not put the write's result on
    /// screen. Each also went out as [`OpFailed`](SessionEvent::OpFailed),
    /// which carries no id; this ties it to the write. A graph rebuild a
    /// newer request took over is answered by that request's pass.
    ///
    /// Sent after a cancelled write too, with nothing read or failed, so
    /// the boundaries balance while the session closes.
    WriteSettled {
        id: OperationId,
        kind: OperationKind,
        failed: Vec<FollowUp>,
    },
    /// A git subprocess was spawned (command log). Only what the user
    /// asked for, unless background reads were switched on — plus the
    /// fetches nobody asked for that git said no to, which arrive when
    /// they end (`process::Kept`).
    CommandStarted {
        id: u64,
        /// The command as a log line shows it.
        display: String,
        /// The same command with the always-applied configuration and
        /// environment spelled out, for copying.
        full: String,
        /// Wall clock at the spawn, milliseconds since the epoch.
        at_ms: i64,
        /// Whether the reader asked for it. A row nobody asked for raises
        /// no panel and reddens no mark
        /// (デザイン規約 §git が言ったことを読む場所).
        asked: bool,
        /// The write this command ran under (its acceptance id), so a
        /// compound write reads as one operation. `None` for a read; a
        /// write's unasked fetches carry it too.
        operation: Option<OperationId>,
    },
    /// The command with this id ended.
    CommandFinished {
        id: u64,
        end: CommandEnd,
        /// Time spent waiting for a slot before the spawn
        /// (`process::Slots`) — the application's time, told apart from
        /// git's.
        waited_ms: u64,
        /// What the process took, from the spawn to the reap.
        elapsed_ms: u64,
        /// git's own output on the way out (stderr), or the reason it
        /// never ran. Empty when it said nothing.
        message: String,
    },
}
