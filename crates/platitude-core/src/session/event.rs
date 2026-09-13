//! Everything the session can tell the UI, as one enum.

use super::*;

/// Everything the session can tell the UI.
#[derive(Debug)]
pub enum SessionEvent {
    Opened {
        info: RepoInfo,
    },
    /// The repository would not open. The path is the one the session was
    /// asked for: only some errors carry it themselves, and the screen
    /// names the folder whichever error it was.
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
        /// Commits the walk emitted — what `--max-count` caps, so this
        /// equals the window limit whenever `truncated`. The truncation
        /// footer's number: `total` counts shown rows, which the WIP row
        /// and sifted stash parents move off any round figure.
        walked: u32,
        /// True when the stream stopped at the configured window limit
        /// (older history exists but is not shown).
        truncated: bool,
    },
    LogFailed {
        generation: u64,
        error: String,
    },
    /// Whether the graph on screen is known to have fallen behind the
    /// repository — a whole picture that an off-screen rebuild would have
    /// replaced and did not, so no row is missing and every one of them
    /// is out of date (`RepoSession::run_swap_pass`).
    ///
    /// **Sent on the turn only**, which is why it carries a flag rather
    /// than being two events or none: the common answer a rebuild gives
    /// is [`RefreshOutcome::Unchanged`], which sends nothing at all so
    /// that a quiet auto-fetch tick does not wake the consumer. A mark
    /// that could only be *put on* would then stand for the rest of the
    /// session, and one sent on every pass would cost exactly the wakeup
    /// the silence is there to save.
    ///
    /// **No words.** Whatever git said about the rebuild went out as
    /// [`SessionEvent::OpFailed`] like every other read's, so it is
    /// already in the error surface and the command log; the graph is
    /// told the state and nothing else. A pass that fell over silently
    /// has none to give anyway (`session::pass_watch`).
    LogStale {
        stale: bool,
    },
    /// A background rebuild finished and replaces the whole graph in one
    /// step. Deliberately one event rather than Started/Chunk/Finished:
    /// those travel as separate queued messages, and a consumer that
    /// drains between them paints an empty model for a frame — visible
    /// as a white flash whenever a background refresh finds changes.
    LogReplaced {
        generation: u64,
        rows: Vec<LogRow>,
        elapsed_ms: u64,
        /// As on [`SessionEvent::LogFinished`].
        walked: u32,
        truncated: bool,
    },
    /// Labels of already-delivered rows changed (refs arrived/refreshed).
    /// `generation` names the graph the row numbers were read from, so a
    /// consumer showing another one drops them instead of putting chips
    /// on whatever commit now sits at those numbers.
    LabelsChanged {
        generation: u64,
        rows: Vec<(u32, Vec<RefLabel>)>,
    },
    /// Where HEAD stands, as the newest read that reported it left it —
    /// the one answer every consumer reads it from
    /// (`session::standing`). Sent when it moved, and once more by the
    /// first read to land after a write whether or not it moved: that is
    /// the report a consumer waiting on "the repository as the write left
    /// it" is owed. A quiet tick sends nothing.
    ///
    /// `seq` numbers the report, in the order the reports were accepted
    /// and across sessions (`session::standing`): a consumer waiting on a
    /// write holds the number that write's answer named
    /// ([`WriteFinished::head_seq`](SessionEvent::WriteFinished)), and a
    /// report numbered at or above it is one that looked after the write.
    HeadObserved {
        head: HeadState,
        seq: u64,
    },
    /// Whether a remote already has the commit HEAD is on — the `already
    /// pushed` note an amend wears — read off the walk's own marks
    /// (`session::published`) and sent when the answer moved. `oid` is
    /// the commit the answer is about, `None` on a branch with no commits
    /// yet, so a consumer holding a HEAD this is not yet about can tell.
    HeadPublished {
        oid: Option<Oid>,
        published: bool,
    },
    /// Shared rather than owned: every sidebar section is handed the whole
    /// snapshot and reads its own part of it, and a deep copy per section
    /// is tens of thousands of strings duplicated for nobody
    /// (`JetBrains/kotlin`: 45,782 tags).
    RefsLoaded {
        snapshot: Arc<RefsSnapshot>,
        /// When the pass that sent this looked at the repository
        /// (`Standing::stamp`, taken before git is spawned) — **not when
        /// it arrived**. What a consumer holding rows off the screen for
        /// a write measures against that write's
        /// [`reads_from`](Self::WriteFinished): at or above it, this
        /// listing saw what the write left; below it, the listing was
        /// already in flight and says nothing about the write.
        ///
        /// The snapshot beside it may be the pointer an earlier pass
        /// published (an unmoved repository rebuilds nothing), and this
        /// still names this pass — the question it answers is when the
        /// repository was looked at, not when these rows were built.
        looked: u64,
    },
    /// What another working copy is holding, read because somebody is
    /// looking at its row (`RepoSession::read_carried_status`).
    ///
    /// **Its own event and not [`SessionEvent::StatusLoaded`].** That one
    /// is this window's tree — the commit box, the band, the graph's own
    /// row and the counts a write is settled against all hang off it, and
    /// a copy's status arriving there would move every one of them.
    CarriedStatusLoaded {
        /// The copy this is about; the pane reads it back to know whose
        /// changes it is showing, and a read that lands after the reader
        /// moved on is dropped by it.
        path: String,
        /// The name the header says — the copy's own, not this tree's.
        name: String,
        status: WorkTreeStatus,
    },
    StatusLoaded {
        status: WorkTreeStatus,
        /// The number of the report of HEAD this status stands beside —
        /// the record's, after what the read saw of HEAD went into it
        /// ([`HeadObserved::seq`](SessionEvent::HeadObserved)). What says
        /// the counts are about the HEAD the consumer holds, and what a
        /// consumer waiting on a write's tree waits for.
        head_seq: u64,
        op_state: OpState,
        /// "commit N of M" while a rebase is stepping through commits.
        progress: Option<conflict::Progress>,
        /// What to call the two sides of a conflict, for whatever is
        /// stopped. Default while nothing is.
        sides: conflict::Sides,
        /// The message a stopped merge is about to record, comment lines
        /// already gone (`integrate::stopped_message`). Empty unless a
        /// merge is standing: it is the one operation this application
        /// finishes from the commit box, so it is the one that has a
        /// message to put there.
        op_message: String,
        /// The merge tool git would launch, empty when none is configured
        /// or when nothing is conflicted. Display only — the launch reads
        /// the config again, so a stale name here cannot start anything.
        merge_tool: String,
        /// `branch.<branch>.pushRemote` for the branch named above, empty
        /// where the branch does not mark one (and where there is no
        /// branch at all).
        ///
        /// Rides the status rather than the refs so it cannot be read
        /// against a different branch than the one it was asked for: it
        /// is per-branch configuration, and the branch, its upstream and
        /// its counts arrive here together. The same read carries the
        /// repository's own mark, which flows through the refs snapshot
        /// instead (`RepoSession::note_push_default`). Display and
        /// standing only — [`crate::remote::plan_current_push`] reads
        /// the keys again before a send, so a stale name here cannot
        /// misdirect one.
        push_remote: String,
        /// Pending paths whose change has something to say about line
        /// endings. Shared rather than copied: most status reads repeat the
        /// previous answer unchanged, and every open tab does it.
        eol_marks: Arc<Vec<EolMark>>,
        /// Why a standing rebase is standing — the `edit` stop looks like
        /// the empty stop from the tree alone, and the exit card's words
        /// and its `--skip`'s cost both turn on which it is
        /// (デザイン規約 §進行中の操作から出る). Default while no rebase is.
        stop: integrate::RebaseStop,
    },
    /// What operation is standing and how far it has got — the badge's
    /// two halves and nothing else ([`RepoSession::refresh_op_progress`]).
    ///
    /// Its own event rather than a status snapshot because it is asked
    /// several times a second: the count moves about every eleven
    /// milliseconds and the screen is there to count it out, while the
    /// snapshot around it costs a `git status` of the whole work tree
    /// (`ci/baseline/poll-cost-windows-x64.md`). Both halves come from
    /// files under the git directory, so the whole event costs no process
    /// at all.
    ///
    /// **The state rides with the count** because the badge needs both:
    /// the count alone would arrive at a badge that is not on screen, a
    /// rebase of a few hundred commits being over long before the ten-
    /// second tick that would have raised it. Which operation is the live
    /// one is still core's answer and still the same one
    /// ([`integrate::InProgress::from_state`]) — this is that answer read
    /// from the cheaper side.
    OpProgress {
        op_state: OpState,
        progress: Option<conflict::Progress>,
    },
    /// Answer to [`RepoSession::ask_merge_tools`]: names the settings field
    /// can offer, deliberate ones first. Empty is a valid answer.
    ///
    /// Sent twice where config named anything: once with those, so the
    /// eight-second read does not hold back an answer that took
    /// milliseconds, and once with everything. `settled` marks the last.
    MergeToolsLoaded {
        names: Vec<String>,
        settled: bool,
    },
    /// Answer to [`RepoSession::check_remote_branch`]: whether the remote
    /// already carries that exact name, as of this moment rather than as of
    /// the last fetch. The question is echoed back because the box that
    /// asked it may have moved on to another name by the time this lands.
    RemoteBranchChecked {
        remote: String,
        branch: String,
        /// What a push under that name would meet: taken or not, and where
        /// taken, whether git would carry it or turn it down. Silence from
        /// the remote is one of the answers rather than a missing one —
        /// reading it as "nothing is there" would send on the assumption
        /// that git refuses what it does not.
        state: remote::RemoteBranchState,
        /// The commit the remote advertised, hex, empty where it named
        /// none. What an overwrite would have to lease against, so the
        /// question can offer one pinned to what it showed.
        tip: String,
        /// How many commits that tip reaches that this history does not —
        /// what an overwrite would take off that branch. Zero unless the
        /// answer was `Refused`, which is the only state where both the
        /// commit and the walk are here.
        theirs: u32,
    },
    /// Answer to [`RepoSession::check_branch_delete`]: whether the branch
    /// is merged into the reference point `branch --delete` measures
    /// against (its upstream, or HEAD without one). The branch is echoed
    /// back because the menu that asked may be open over another row by
    /// the time this lands.
    BranchDeleteChecked {
        branch: String,
        /// `None` where the reads could not say. **Not the same as
        /// merged**, though the row draws them alike: neither wears `-D`,
        /// because a delete that has not been shown to be refused is
        /// offered in its plain form and git gives the real answer to
        /// the press. What the third state is for is the reader looking
        /// at a run afterwards — every ask is answered, so a delete row
        /// still bare is one whose answer said so.
        merged: Option<bool>,
    },
    /// Answer to [`RepoSession::check_plan_published`]: how many commits
    /// of a plan's range a remote already has, as of now. The range is
    /// echoed back because the plan that asked may have been put away and
    /// another opened by the time this lands.
    PlanPublished {
        range: String,
        published: u32,
    },
    /// Answer to [`RepoSession::ask_rebase_plan`]: the rows the
    /// interactive-rebase screen opens over. `from` is echoed back (inside
    /// the preview) because the click that asked may be behind another by
    /// the time this lands, and `generation` says which ask it answers —
    /// the asks are numbered, and two clicks a moment apart need not
    /// finish in the order they were made.
    RebasePlanLoaded {
        generation: u64,
        preview: rebase_plan::PlanPreview,
    },
    /// The same question answered with a refusal of this end's own: the
    /// range cannot be replayed, and the screen never opens. The wording
    /// is the screen's to write, so what travels is the kind.
    RebasePlanRefused {
        generation: u64,
        from: String,
        refusal: rebase_plan::PlanRefusal,
    },
    /// The same question and the read itself failed. The failure reaches
    /// the shared error surface as its own [`SessionEvent::OpFailed`];
    /// this one exists so the model asking can put its waiting state
    /// down — silence would leave it loading forever.
    RebasePlanFailed {
        generation: u64,
        from: String,
    },
    /// Whether anything besides the branch HEAD is on still reaches its
    /// tip, so the rows that rewrite history can tell a rewrite that
    /// leaves the old commits drawn from one that leaves them to the
    /// reflog (see [`crate::reachable`]). Sent when the answer moves.
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
        /// When this listing looked, read the way
        /// [`RefsLoaded::looked`](Self::RefsLoaded) is. The stash has a
        /// stamp of its own because it has a read of its own, behind its
        /// own flight and after the graph is rebuilt: a consumer that
        /// measured a dropped entry against the refs' stamp would put the
        /// row back for the whole of that rebuild.
        looked: u64,
    },
    WorktreesLoaded {
        worktrees: Vec<crate::worktrees::WorktreeEntry>,
    },
    DetailsLoaded {
        generation: u64,
        details: CommitDetails,
    },
    /// What a choice of several commits changed — the file list alone,
    /// the rows naming the commits being on screen already
    /// (デザイン規約 §複数のコミットを選ぶ). Shares the details numbering,
    /// the two being answers to the same pane.
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
        /// lines to say what colour each run of them is.
        patches: Arc<Vec<FilePatch>>,
        /// Image files / binary sizes when the text diff is not the whole
        /// story (`None` for ordinary text files).
        preview: Option<FilePreview>,
        /// Fingerprint of the bytes `patches` was parsed from. Hunk/line
        /// selections carry it back, so a partial write can refuse a diff
        /// that drifted under the selection (`stage::apply_partial`).
        fingerprint: u64,
        /// What the same bytes said about line endings, if anything. Rides
        /// with the diff rather than following it: a notice that appears
        /// after the reader has started is worse than none.
        endings: Option<crate::eol::Notice>,
        /// What changed inside each row (`intraline`). With the rows, not
        /// behind them like the colours: it is read off the rows alone,
        /// costs milliseconds, and where a change is is the first thing a
        /// reader looks for.
        marks: Arc<crate::intraline::IntraMarks>,
        /// For the one row that has no patch at all — a repository of its
        /// own inside the working copy — the commit a stage of it would
        /// point at (`details::embedded`). `None` for every other target.
        embedded: Option<crate::details::Embedded>,
    },
    /// Syntax colours for the lines of a diff that has already been sent.
    ///
    /// Behind the rows rather than with them. Colouring is the one part of
    /// reading a diff that computes rather than waits, and it is not
    /// small: a few thousand lines of a language the set knows costs two
    /// orders of magnitude more than the same text under a name nothing
    /// can be said about, and lands on the wrong side of the 100ms an
    /// interaction is allowed (ci/baseline/code-costs-windows-x64.md
    /// §着色). So the rows go out the moment git answers
    /// and the colours follow — a large file reads black and white for a
    /// beat and then takes its colour, rather than showing nothing at all
    /// for as long as the colouring takes.
    ///
    /// A diff nobody is on any more never raises this: the reader has
    /// moved, and the cost of colouring what they left would be paid out
    /// of the file they are on now (`RepoSession::diff_epoch`).
    DiffColoured {
        target: DiffTarget,
        colors: crate::highlight::DiffColors,
        /// Whether these are the colours the diff ends on. False on the
        /// quick first answer a deep fallback-lexer diff sends ahead of
        /// its full read — anything waiting for "the colours" waits for
        /// true, or it latches a shot of the interim
        /// (app-ui.md §UI 自動化の因果性).
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
    /// one the acceptance handed back ([`RepoSession::write`]), and every
    /// event about this write from here on carries it
    /// (`crate::operation`).
    WriteStarted {
        id: OperationId,
        kind: OperationKind,
    },
    /// A write did what it was asked and git stopped part-way through it,
    /// leaving the operation standing for someone to finish. Not a
    /// failure: the [`WriteFinished`](SessionEvent::WriteFinished) that
    /// follows carries no error, and this says the one thing that answer
    /// cannot — that the screen should go to the conflicts rather than to
    /// a commit that was never made.
    WriteStopped {
        id: OperationId,
        kind: OperationKind,
    },
    /// A write's git command ended — the second of the write's three
    /// boundaries, between its acceptance and
    /// [`WriteSettled`](SessionEvent::WriteSettled). `error` carries
    /// git's own message. The reads the write invalidated have not been
    /// made yet, so this is deliberately not a boundary for them.
    ///
    /// `report` is the failure with something else to be made of it: the
    /// write did not happen, something outside this application said so,
    /// and what it said is a report rather than an error
    /// ([`crate::report::WriteReport`]). It rides beside `error`, which
    /// goes on carrying git's whole message for the log.
    ///
    /// `head_seq` is the smallest number the first report of HEAD after
    /// this write can carry (`Standing::fence`): a consumer landing on
    /// what the write left arms on it, and is answered by the report
    /// numbered at or above it — whichever of the two reaches it first.
    /// Its own number, not the write's id: reads nobody asked for move it
    /// too.
    ///
    /// `reads_from` is the same fence for the consumers that wait on a
    /// **listing** instead: the smallest stamp a read that looked after
    /// this write can carry, against which [`RefsLoaded`](Self::RefsLoaded)
    /// and [`StashesLoaded`](Self::StashesLoaded) name when they looked.
    /// A consumer showing rows as already gone measures by it — the
    /// listing already in flight when the write ended says nothing about
    /// the write, and counting arrivals cannot tell the two apart once
    /// they are travelling separate feeds.
    WriteFinished {
        id: OperationId,
        kind: OperationKind,
        error: Option<String>,
        report: Option<crate::report::WriteReport>,
        head_seq: u64,
        reads_from: u64,
    },
    /// The reads the write invalidated have been made — the last of its
    /// three boundaries, after which nothing more is said under this id
    /// and the queue takes the next request. What has been read by now:
    /// the working tree and the refs where the write's [`AfterWrite`]
    /// reads them, the graph where either moved, the stash and worktree
    /// listings and the reads those asked for (a working copy taken or
    /// given back re-joins the refs and, standing on no branch, walks the
    /// graph again), and the author configuration after an identity
    /// write. Not the reachability walk behind the rewrite rows
    /// (`settle_head_reach`), which answers when it can and is skipped
    /// while the previous one runs.
    ///
    /// `failed` names the reads among those that did not put the write's
    /// result on screen — empty where every one of them landed. Each
    /// failure also went out as [`OpFailed`](SessionEvent::OpFailed)
    /// under the same label, but that event carries no id; this is what
    /// ties it to the write. A graph rebuild a newer request took over
    /// is not a failure: that request's pass answers for the repository.
    ///
    /// Sent after a cancelled write too, with nothing read and nothing
    /// failed: the session is closing, and the boundaries still balance.
    WriteSettled {
        id: OperationId,
        kind: OperationKind,
        failed: Vec<FollowUp>,
    },
    /// A git subprocess was spawned (command log). Only what the user
    /// asked for, unless background reads were switched on — plus the
    /// fetches nobody asked for that git said no to, which arrive when
    /// they end rather than when they start (`process::Kept`).
    CommandStarted {
        id: u64,
        /// The command as a log line shows it.
        display: String,
        /// The same command with the always-applied configuration and
        /// environment spelled out, for copying.
        full: String,
        /// Wall clock at the spawn, milliseconds since the epoch.
        at_ms: i64,
        /// Whether the reader is the one who asked for it. A row that
        /// nobody asked for is the record that git said no and nothing
        /// more: it raises no panel and reddens no mark
        /// (デザイン規約 §git が言ったことを読む場所).
        asked: bool,
        /// The write this command ran under, for a compound write to be
        /// read as one operation of several commands — the id its
        /// acceptance handed back. `None` for a read; every write's
        /// commands carry its id, the fetches nobody asked for included,
        /// whose row appears only where git said no.
        operation: Option<OperationId>,
    },
    /// The command with this id ended.
    CommandFinished {
        id: u64,
        end: CommandEnd,
        elapsed_ms: u64,
        /// git's own output on the way out (stderr), or the reason it
        /// never ran. Empty when it said nothing.
        message: String,
    },
}
