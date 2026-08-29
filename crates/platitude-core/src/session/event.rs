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
    /// Shared rather than owned: every sidebar section is handed the whole
    /// snapshot and reads its own part of it, and a deep copy per section
    /// is tens of thousands of strings duplicated for nobody
    /// (`JetBrains/kotlin`: 45,782 tags).
    RefsLoaded {
        snapshot: Arc<RefsSnapshot>,
    },
    StatusLoaded {
        status: WorkTreeStatus,
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
    /// the time this lands. No event is sent where the reads fail —
    /// silence leaves the delete row on its plain form, and a refusal,
    /// if any, lands the way it always has.
    BranchDeleteChecked {
        branch: String,
        merged: bool,
    },
    /// Answer to [`RepoSession::check_publish`].
    PublishChecked {
        range: String,
        state: publish::PublishState,
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
    },
    WorktreesLoaded {
        worktrees: Vec<crate::worktrees::WorktreeEntry>,
    },
    DetailsLoaded {
        details: CommitDetails,
    },
    /// The details read for `oid` failed. The failure itself goes out as
    /// [`SessionEvent::OpFailed`] like every read's; this one is for the
    /// pane showing a spinner against that oid, which nothing else would
    /// take down.
    DetailsFailed {
        oid: Oid,
    },
    DiffLoaded {
        target: DiffTarget,
        /// Shared with the colouring that follows, which reads the same
        /// lines to say what colour each run of them is.
        patches: Arc<Vec<FilePatch>>,
        /// Image bytes / binary sizes when the text diff is not the whole
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
    },
    /// Syntax colours for the lines of a diff that has already been sent.
    ///
    /// Behind the rows rather than with them. Colouring is the one part of
    /// reading a diff that computes rather than waits, and it is not
    /// small: 6,000 lines of Rust measured 951ms against 3ms for the same
    /// text under a name nothing can be said about (measured),
    /// which is most of a second on the wrong side of the 100ms an
    /// interaction is allowed. So the rows go out the moment git answers
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
    /// A write operation started; the UI can show it as in flight.
    WriteStarted {
        op: &'static str,
    },
    /// A write did what it was asked and git stopped part-way through it,
    /// leaving the operation standing for someone to finish. Not a
    /// failure: the [`WriteFinished`](SessionEvent::WriteFinished) that
    /// follows carries no error, and this says the one thing that answer
    /// cannot — that the screen should go to the conflicts rather than to
    /// a commit that was never made.
    WriteStopped {
        op: &'static str,
    },
    /// A write operation's Git command ended. `error` carries Git's own
    /// message. The queue can still be settling its follow-up snapshots and
    /// graph, so this is deliberately not a queue-idle boundary.
    ///
    /// `report` is the failure with something else to be made of it: the
    /// write did not happen, something outside this application said so,
    /// and what it said is a report rather than an error
    /// ([`crate::report::WriteReport`]). It rides beside `error`, which
    /// goes on carrying git's whole message for the log.
    WriteFinished {
        op: &'static str,
        error: Option<String>,
        report: Option<crate::report::WriteReport>,
    },
    /// A git subprocess was spawned (command log). Only what the user
    /// asked for, unless background reads were switched on.
    CommandStarted {
        id: u64,
        /// The command as a log line shows it.
        display: String,
        /// The same command with the always-applied configuration and
        /// environment spelled out, for copying.
        full: String,
        /// Wall clock at the spawn, milliseconds since the epoch.
        at_ms: i64,
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
