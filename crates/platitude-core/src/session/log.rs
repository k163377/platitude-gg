//! The log -> graph pipeline: options, the direct and swap passes, and
//! the background refreshes. What one pass reads is [`super::walk`].

use super::graph_refresh::{PassAnswer, PassStart};
use super::rows::LogTotals;
use super::*;

/// The reads a pass takes before the walk: the remote-tracking tips the
/// rows are marked against, and the walk's own inputs ([`WalkInputs`]).
/// One set serves both passes of one ask (`RepoSession::restart_log`):
/// the rebuilding pass starts the moment the painting one lands, so asking
/// again would launch processes for an answer one walk older. A stash
/// pushed outside meanwhile is drawn by the next tick
/// (`session_integration::rebuild::the_two_passes_of_one_ask_read_the_stashes_once`).
#[derive(Clone)]
pub(super) struct PassReads {
    tips: RemoteTips,
    inputs: WalkInputs,
}

impl RepoSession {
    pub fn log_options(&self) -> LogOptions {
        *self.lock_log_options()
    }

    /// Whether the graph is drawing tags — what every row's chips are cut
    /// against (`LabelIndex::labels_of`). Read from the session, not the
    /// pass's `LogOptions`: a restart's fast pass walks without tags, and
    /// cutting to it would blink every tag off for that pass.
    pub(super) fn tags_shown(&self) -> bool {
        self.lock_log_options().include_tags
    }

    /// Toggles tags in the graph walk and restarts the stream.
    pub fn set_include_tags(self: &Arc<Self>, include_tags: bool) {
        {
            let mut options = self.lock_log_options();
            if options.include_tags == include_tags {
                return;
            }
            options.include_tags = include_tags;
        }
        self.restart_log();
    }

    /// Changes the graph window size (`None` = full history) and restarts.
    /// Also resets the tail's step (`log_window_step`), which growing the
    /// window leaves alone — the one place the step is decided. A direct
    /// pass: the graph empties and the view goes back to the top, which is
    /// why the tail's press takes `grow_log_window` instead. Called while
    /// the tab is still opening it runs no pass (no workdir yet, so
    /// `restart_log` returns early) and the open does not walk twice.
    pub fn set_log_limit(self: &Arc<Self>, limit: Option<u32>) {
        {
            let mut options = self.lock_log_options();
            if options.limit == limit {
                return;
            }
            options.limit = limit;
            if let Some(initial) = limit {
                options.step = log_window_step(initial);
            }
        }
        self.restart_log();
    }

    /// What one press of the graph's tail would add here.
    pub fn log_window_step(&self) -> u32 {
        self.lock_log_options().step
    }

    /// Widens the graph window by one step and rebuilds it in place with a
    /// swap pass: a direct pass would blank the rows and send the reader at
    /// the bottom back to the top. A no-op on a window that is already the
    /// whole history.
    pub fn grow_log_window(self: &Arc<Self>) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let (options, was) = {
            let mut options = self.lock_log_options();
            let Some(limit) = options.limit else {
                return;
            };
            options.limit = Some(limit.saturating_add(options.step));
            (*options, limit)
        };
        let grown = options.limit;

        let (pass, _) = self.take_pass(true);

        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let PassStart { token, lap, answer } = pass;
            let Some(_lap) = lap.start(|| answer.answer(RefreshOutcome::Silent)).await else {
                answer.answer(RefreshOutcome::Cancelled);
                return;
            };
            // Only a failure leaves the press unanswered: a swap pass
            // reports a failed walk to the tab (`run_swap_pass` -> `fail`),
            // not the graph. Put the window back and take the ordinary
            // route, which does answer the graph.
            let outcome = s
                .run_swap_pass(&workdir, options, &token, None, &answer)
                .await;
            answer.answer(outcome);
            if outcome == RefreshOutcome::Failed {
                // Only if the window is still this press's: the failure can
                // arrive after somebody else set a window of their own.
                let mut options = s.lock_log_options();
                if options.limit != grown {
                    return;
                }
                options.limit = Some(was);
                drop(options);
                s.restart_log();
            }
        });
    }

    fn lock_log_options(&self) -> std::sync::MutexGuard<'_, LogOptions> {
        relock(&self.log_options)
    }

    /// The graph on screen has fallen behind the repository, or caught up
    /// again; said on the turn only ([`SessionEvent::LogStale`]). Every
    /// pass that is not cancelled ends in one of the two — only a rebuild
    /// that could not be walked says `true`.
    pub(super) fn tell_graph_stale(&self, stale: bool) {
        if self.graph_stale.swap(stale, Ordering::SeqCst) != stale {
            self.sink.event(SessionEvent::LogStale { stale });
        }
    }

    /// The graph's walk has gone silent, or a pass has answered since
    /// ([`SessionEvent::LogSilent`]); said on the turn only. Put down only
    /// where a pass answers for the graph — landed, or failed and told as
    /// such — never where one starts: a Reload over a silent walk keeps the
    /// mark up until its own walk answers, instead of taking it down and
    /// putting it back a budget later.
    pub(super) fn tell_graph_silent(&self, silent: bool) {
        if self.graph_silent.swap(silent, Ordering::SeqCst) != silent {
            self.sink.event(SessionEvent::LogSilent { silent });
        }
    }

    /// A walk of `answer`'s pass has landed: the graph on screen is the
    /// repository's, so neither mark stands — an earlier failure's, nor a
    /// silent walk's.
    pub(super) fn tell_graph_landed(&self, answer: &PassAnswer) {
        self.tell_graph_stale(false);
        self.walk_answered(answer);
    }

    /// A walk of `answer`'s pass has answered, landed or failed: git is not
    /// silent there, for the band or for the asks behind it
    /// (`asks::Takeover::spoke`) — a second walk under the same pass is
    /// watched afresh.
    pub(super) fn walk_answered(&self, answer: &PassAnswer) {
        self.log_run.spoke(answer.number);
        self.tell_graph_silent(false);
    }

    /// The walk behind `answer` has printed nothing for its silence budget
    /// (`walk::walk_silence`). Whoever waits on the pass is answered now, as
    /// not landed ([`RefreshOutcome::Silent`]) — a write's settling, a
    /// poll's single flight, the passes a fetch waits out, and the asks
    /// from behind the screen waiting behind it; an ask from the screen
    /// takes the pass over whatever the streak (`asks::Takeover`); and the
    /// band says so. The walk runs on: if git answers it lands as any pass
    /// does, and that clears the band.
    ///
    /// Told on the walk's own task, from the executor's watch, once a spawn.
    pub(super) fn walk_went_silent(&self, answer: &PassAnswer) {
        tracing::warn!("the graph walk has printed nothing for its silence budget; left running");
        self.log_run.went_silent(answer.number);
        self.tell_graph_silent(true);
        answer.answer(RefreshOutcome::Silent);
    }

    /// What a pass reads before it can walk, taken concurrently: no answer
    /// feeds another, and in series the process launches dominate the time
    /// to the first row (ci/baseline/code-costs-windows-x64.md §git のプロセス代).
    pub(super) async fn pass_reads(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        cancel: &CancellationToken,
    ) -> Result<PassReads, GitError> {
        let (tips, inputs) = self.reads_for(None, workdir, cancel).await;
        Ok(PassReads {
            tips,
            inputs: inputs?,
        })
    }

    /// [`Self::pass_reads`] with the stashes a listing behind the same
    /// reason already read — for a caller that read them before asking for
    /// the walk (`write::settle_after`, `refresh_quick`, `poll_reads`).
    /// `None` where they no longer stand ([`Self::stashes_standing`],
    /// checked here, just before the walk is asked for, which is what can
    /// be overtaken): what overtook the list moved the rest a pass reads
    /// ahead as well (HEAD, the remote tips), so the pass is left to read
    /// all of it at its turn.
    pub(super) async fn pass_reads_listed(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        cancel: &CancellationToken,
        stashes: StashRead,
    ) -> Option<PassReads> {
        let listed = self.stashes_standing(stashes)?;
        let (tips, inputs) = tokio::join!(
            self.remote_tips(workdir, cancel),
            self.walk_inputs(workdir, cancel, Some(listed))
        );
        Some(PassReads {
            tips,
            inputs: inputs.ok()?,
        })
    }

    /// What this pass walks with: the reads the ask above it took, or its
    /// own. The halves come back apart: a failed walk input is the walk's
    /// to report, and the tips still seed the pass's marks.
    async fn reads_for(
        self: &Arc<Self>,
        taken: Option<PassReads>,
        workdir: &std::path::Path,
        cancel: &CancellationToken,
    ) -> (RemoteTips, Result<WalkInputs, GitError>) {
        match taken {
            Some(PassReads { tips, inputs }) => (tips, Ok(inputs)),
            None => tokio::join!(
                self.remote_tips(workdir, cancel),
                self.walk_inputs(workdir, cancel, None)
            ),
        }
    }

    /// Restarts the log → graph stream. With tags enabled this runs two
    /// passes: a tag-less one that paints at once, then a tag-inclusive
    /// rebuild swapped in without a flicker — `--tags` makes git set up a
    /// frontier over every tag before the first row, which the first pass
    /// does not wait for.
    ///
    /// Reset-and-stream is for open, Reload, the tags toggle and a window
    /// size change only; background triggers go through `refresh_log`,
    /// which stays silent when the picture is the same.
    ///
    /// A walk this cancels sends neither `LogFinished` nor `LogFailed`: a
    /// later `refresh_log` replaces it with `LogReplaced` (rightly), so a
    /// waiter takes either shape (`support::session::pass_of`).
    ///
    /// While a tab opens, the fast pass may never land: a rebuild that
    /// overtakes it swallows it without a word. A test asserts only its
    /// order there (before the swap, if it lands) and pins its presence and
    /// row count on a toggle after a settled baseline, where nothing can
    /// take the log token from the two passes
    /// (`session_integration::window::tag_only_commits_follow_the_include_tags_option`).
    pub fn restart_log(self: &Arc<Self>) {
        let Some(workdir) = self.workdir() else {
            return;
        };

        let (pass, _) = self.take_pass(true);

        let s = Arc::clone(self);
        let options = self.log_options();
        self.runtime.spawn(async move {
            let PassStart { token, lap, answer } = pass;
            let Some(_lap) = lap.start(|| answer.answer(RefreshOutcome::Silent)).await else {
                answer.answer(RefreshOutcome::Cancelled);
                return;
            };
            // The last pass answers for the ask — or the first walk to go
            // silent (`PassAnswer`).
            let outcome = if options.include_tags {
                let fast = LogOptions {
                    include_tags: false,
                    ..options
                };
                let reads = s.pass_reads(&workdir, &token).await.ok();
                match s
                    .run_direct_pass(&workdir, fast, &token, reads.clone(), &answer)
                    .await
                {
                    RefreshOutcome::Changed => {
                        s.run_swap_pass(&workdir, options, &token, reads, &answer)
                            .await
                    }
                    stopped => stopped,
                }
            } else {
                s.run_direct_pass(&workdir, options, &token, None, &answer)
                    .await
            };
            answer.answer(outcome);
        });
    }

    /// Takes the log stream over: a fresh token for this pass, the one it
    /// displaces stopped or waited out ([`Lap::start`]), the numbered ask it
    /// answers as (`graph_refresh::GraphRun`) — a displaced pass's caller waits for the
    /// later number ([`RepoSession::graph_answer`]) — and its place among
    /// the passes still walking, all bound into the one answer the pass owes
    /// ([`PassAnswer`]); and the boundary a caller waits on for that answer.
    ///
    /// Call on the caller's thread before spawning the pass: call order
    /// decides which pass owns the graph, and spawn order does not follow
    /// it. This token alone decides ownership: `log_gen` is taken in spawn
    /// order, and judging by both can make each pass stand down, leaving
    /// Reload with no answer. Its place is taken here too: registered from
    /// inside the task, the boundary could read "idle" for a pass not
    /// started yet.
    ///
    /// The number and the handover are taken under one lock: otherwise two
    /// racing asks could number in one order and take the stream in the
    /// other, leaving the displaced pass waiting on a number nothing
    /// answers.
    ///
    /// A follow ask taken over (`session::asks`): the pass starts with
    /// [`Lap::start`], which waits for the pass under way to land where
    /// stopping it would make too many stopped in a row — a stream of asks
    /// on a slow machine still lands a picture. `asked` where the ask is
    /// the screen's own: it stops a pass gone silent whatever the streak,
    /// and an ask from behind the screen left behind one is answered
    /// `Silent` where it waits (`asks::Takeover::take`).
    pub(super) fn take_pass(&self, asked: bool) -> (PassStart, RefreshTask) {
        let (lap, run) = self
            .log_run
            .take(&self.root_cancel, asked, || self.graph_passes.ask());
        let held = self.graph_passes.enter();
        let (answer, task) = PassAnswer::owed(run, lap.number(), held);
        let token = lap.token();
        (PassStart { token, lap, answer }, task)
    }

    /// Streams one pass straight to the UI (chunked, resets the graph).
    /// `Changed` where the stream reached the consumer; otherwise
    /// `Cancelled` (taken over) or `Failed`.
    async fn run_direct_pass(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        options: LogOptions,
        cancel: &CancellationToken,
        reads: Option<PassReads>,
        answer: &PassAnswer,
    ) -> RefreshOutcome {
        let mut watch = PassWatch::operation(self);
        // No older than what the stream lays its worktrees' rows from.
        let readings = self.carried();
        // Before the lock, because reading them can go to git.
        let (tips, inputs) = self.reads_for(reads, workdir, cancel).await;
        let generation = {
            // Reset and announce under one lock: every reader of row
            // numbers in `shared` sends under the same lock, so no message
            // describes a graph the consumer is not on yet (see
            // `apply_refs`). `None` from a session that has let go of what
            // it drew (`RepoSession::keeps_what_it_reads`).
            let Some(mut shared) = self.store_shared() else {
                return RefreshOutcome::Cancelled;
            };
            // Superseded: resetting now would blank the record of what is
            // on screen for a stream nobody wants, and the next rebuild,
            // finding no record, would paste the same picture again (an idle
            // flicker). A pass stood down leaves the state as it is.
            if cancel.is_cancelled() {
                watch.answered();
                return RefreshOutcome::Cancelled;
            }
            // Numbered under the lock it installs under: a graph laid out
            // again meanwhile (`session::leaving`) took a number too, and a
            // stream numbered below the graph on screen is one the
            // consumer refuses.
            let generation = self.log_gen.fetch_add(1, Ordering::SeqCst) + 1;
            shared.builder = GraphBuilder::new();
            // Seeded first, so every emitted row already carries the
            // answer the menus read off it.
            shared.publish_marks = PublishMarks::new(tips);
            shared.generation = generation;
            shared.applied.clear();
            shared.sent_rows.clear();
            shared.walked.clear();
            // No footer until the walk ends, so a rebuild landing in
            // between finds none to compare against.
            shared.sent_footer = None;
            self.sink.event(SessionEvent::LogStarted { generation });
            // The stale graph has just been cleared.
            self.tell_graph_stale(false);
            watch.announced(generation);
            generation
        };
        // Outside the lock: a fault raised here unwinds, and `watch` is
        // what reports it.
        self.run_pass_step(PassStep::Streaming);
        let started = Instant::now();
        // A raised fault or a failed walk input stands in for the walk and
        // takes the reporting arm a failed git would (`PassHooks::fault`).
        let walk = match (self.pass_fault(PassStep::Streaming), inputs) {
            (Some(error), _) => Err(error),
            (None, Err(error)) => Err(error),
            (None, Ok(inputs)) => {
                self.stream_log(workdir, generation, options, cancel, inputs, answer)
                    .await
            }
        };
        match walk {
            Ok(totals) => {
                // HEAD's published mark goes to the record before the
                // consumer hears the pass is over — only from the stream
                // the consumer is on; a superseded one's rows were refused
                // (`emit_rows`).
                if self.lock_shared().generation == generation {
                    self.settle_head_published(totals.head, totals.head_published);
                }
                let footer = Footer {
                    walked: totals.walked,
                    // Judged on the walked count: the shown count drifts
                    // both ways from it (the WIP row, sifted stash parents).
                    truncated: options.limit.is_some_and(|n| totals.walked >= n),
                };
                {
                    // Recorded and sent under one lock, so a rebuild
                    // comparing against this footer finds it only after
                    // the consumer has been told.
                    let Some(mut shared) = self.store_shared() else {
                        return RefreshOutcome::Cancelled;
                    };
                    // Only the stream the consumer is on records its
                    // footer (see `emit_rows`).
                    if shared.generation == generation {
                        shared.sent_footer = Some(footer);
                        shared.walked.footer = Some(footer);
                        self.leaving_walked(answer.ask);
                    }
                    self.sink.event(SessionEvent::LogFinished {
                        generation,
                        total: totals.shown,
                        elapsed_ms: started.elapsed().as_millis() as u64,
                        walked: footer.walked,
                        truncated: footer.truncated,
                    });
                }
                self.tell_graph_landed(answer);
                // A stream can neither leave rows out nor lay a row again as
                // it goes, and a laying asked for meanwhile has no finished
                // graph to lay: a delete that is out takes its commits off it,
                // and another worktree's reading that moved under it is laid
                // on it (a swap pass does both before it publishes). This
                // window's own row is drawn by the walk its status asks for.
                if self.deletes_out() || !Arc::ptr_eq(&readings, &self.carried()) {
                    self.relay_screen();
                }
                watch.answered();
                RefreshOutcome::Changed
            }
            Err(error) => {
                let cancelled = matches!(error, GitError::Cancelled { .. });
                if !cancelled {
                    self.sink.event(SessionEvent::LogFailed {
                        generation,
                        error: error.to_string(),
                    });
                    self.walk_answered(answer);
                }
                // A cancelled pass says nothing on purpose: whoever
                // cancelled it is the one drawing now.
                watch.answered();
                if cancelled {
                    RefreshOutcome::Cancelled
                } else {
                    RefreshOutcome::Failed
                }
            }
        }
    }

    /// The swap pass's walk, or what stands in for it: a raised fault or a
    /// failed walk input (as in `run_direct_pass`).
    async fn walk_off_screen(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        options: LogOptions,
        cancel: &CancellationToken,
        into: Building<'_>,
        inputs: Result<WalkInputs, GitError>,
        answer: &PassAnswer,
    ) -> Result<LogTotals, GitError> {
        match (self.pass_fault(PassStep::Swapping), inputs) {
            (Some(error), _) => Err(error),
            (None, Err(error)) => Err(error),
            (None, Ok(inputs)) => {
                self.collect_log(workdir, options, cancel, into, inputs, answer)
                    .await
            }
        }
    }

    /// Builds a full pass off-screen, then swaps it in whole with one
    /// `LogReplaced`, so the replacement is flicker-free. `reads` is the
    /// ask's shared set, or `None` to take its own ([`PassReads`]).
    pub(super) async fn run_swap_pass(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        options: LogOptions,
        cancel: &CancellationToken,
        reads: Option<PassReads>,
        answer: &PassAnswer,
    ) -> RefreshOutcome {
        // Superseded before it began: skip the walk, the most expensive
        // read in the app. One half of closing a baseline; the other is
        // `wait_for_graph_passes` (`support::session::CaptureSink::opened_graph`).
        // The check under the lock below stops a finished pass from
        // installing.
        if cancel.is_cancelled() {
            return RefreshOutcome::Cancelled;
        }
        // Off-screen: a pass that never finishes leaves a real picture
        // standing (`PassWatch`).
        let mut watch = PassWatch::operation(self);
        self.run_pass_step(PassStep::Swapping);
        let started = Instant::now();
        let mut builder = GraphBuilder::new();
        // Taken before the walk, which runs across awaits and cannot hold
        // this lock (`published::RemoteTips`).
        let (tips, inputs) = self.reads_for(reads, workdir, cancel).await;
        let mut marks = PublishMarks::new(tips);
        let mut rows: Vec<LogRow> = Vec::new();
        // What the synthetic rows are about to be laid from, so the
        // publish below can tell whether it moved while the walk ran
        // (`session::relay`).
        let laid_from = self.standing_rows();

        let result = self
            .walk_off_screen(
                workdir,
                options,
                cancel,
                Building {
                    builder: &mut builder,
                    marks: &mut marks,
                    out: &mut rows,
                },
                inputs,
                answer,
            )
            .await;
        let walked = match result {
            Ok(totals) => {
                // HEAD's mark goes to the record even if the picture turns
                // out unchanged.
                self.settle_head_published(totals.head, totals.head_published);
                totals.walked
            }
            Err(error) => {
                watch.answered();
                // A cancelled pass says nothing (as in `run_direct_pass`).
                if matches!(error, GitError::Cancelled { .. }) {
                    return RefreshOutcome::Cancelled;
                }
                // The graph on screen is whole but no longer this
                // repository's: the band says so (`STALE GRAPH`), and git's
                // words go where every read's do.
                self.tell_graph_stale(true);
                self.walk_answered(answer);
                self.fail("log", error);
                return RefreshOutcome::Failed;
            }
        };

        // As walked, before anything is laid out of it (`Shared::walked`).
        let mut record = Walked::of(&rows);
        let standing = self.lay_again_if_moved(&mut rows, &mut builder, &laid_from);
        let holders = self.holders();
        let tags = self.tags_shown();
        {
            let Some(mut shared) = self.store_shared() else {
                return RefreshOutcome::Cancelled;
            };
            // Superseded — read off the token, not the generation, which
            // follows spawn order.
            if cancel.is_cancelled() {
                watch.answered();
                return RefreshOutcome::Cancelled;
            }
            // A delete that is out keeps its commits off this graph too.
            let gone = self.lay_leaving(
                &mut rows,
                &mut builder,
                &record,
                &standing,
                &holders,
                &shared.label_map,
            );
            let total = rows.len() as u32;
            let elapsed_ms = started.elapsed().as_millis() as u64;
            let applied = wear_chips(&mut rows, &shared.label_map, tags);
            let walk_footer = Footer {
                walked,
                // See run_direct_pass: the walk is what decides
                // truncation.
                truncated: options.limit.is_some_and(|n| walked >= n),
            };
            record.footer = Some(walk_footer);
            // The walk's, less what a delete out now took off it.
            let footer = leaving::footer_without(walk_footer, &gone);
            // Every pass speaks while the worktree row is held back
            // ([`PassHooks::holds_back_the_worktree_row`]): after a
            // stopped replay that row is the only difference, so the pass
            // would be dropped as the same picture. Only a harness raises it.
            let unchanged = !self.holds_back_the_worktree_row() && shared.shows(&rows, footer);
            shared.builder = builder;
            shared.publish_marks = marks;
            shared.applied = applied;
            shared.walked = record;
            self.leaving_walked(answer.ask);
            if unchanged {
                // The UI already shows exactly this; swapping would only
                // reset the view. The generation stays: nothing was sent,
                // and this walk numbered its rows the same way or it would
                // not compare equal.
                tracing::debug!(total, "graph rebuild unchanged; swap skipped");
                // A walk that found the screen current clears an earlier
                // failure's mark, and a silent walk's.
                self.tell_graph_landed(answer);
                watch.answered();
                return RefreshOutcome::Unchanged;
            }
            shared.sent_rows = rows.iter().map(RowPrint::of).collect();
            shared.sent_footer = Some(footer);
            // Numbered as it installs, as a stream is (`run_direct_pass`).
            let generation = self.log_gen.fetch_add(1, Ordering::SeqCst) + 1;
            shared.generation = generation;
            // Under the lock, as in run_direct_pass.
            self.sink.event(SessionEvent::LogReplaced {
                generation,
                rows,
                elapsed_ms,
                walked: footer.walked,
                truncated: footer.truncated,
            });
            self.tell_graph_landed(answer);
        }
        watch.answered();
        RefreshOutcome::Changed
    }

    /// Refreshes refs, status(+op state), stashes and worktrees
    /// concurrently, then asks for the graph once if any of them moved —
    /// handing the walk the stash list the listing read — the manual
    /// refresh. A page on screen is read at its pace instead
    /// (`session::pacer`).
    ///
    /// One rebuild for all of them: asked apiece, a commit made outside
    /// (a ref moved, the tree came clean) asked twice, the second taking
    /// the first's walk over part-way. The other worktrees' pass stays
    /// apart: a status per worktree, too slow to wait on (its rows ask for
    /// their own walk as they land).
    pub fn refresh_quick(self: &Arc<Self>) {
        self.settle_snapshots(true);
        // A refresh asked by hand reads every worktree as well.
        self.refresh_carried();
    }

    /// The opening's [`Self::refresh_quick`], the stashes left to the
    /// opening's first pass, which reads and publishes them
    /// ([`RepoSession::walk_inputs`]).
    ///
    /// Do not put the other worktrees' pass off to cheapen the open (user
    /// decision — ask before changing; the tab-switch costs are listed in
    /// the app's `RepoPageStack.qml`): it starts here, with the rest.
    pub(super) fn refresh_opening(self: &Arc<Self>) {
        self.settle_snapshots(false);
        self.refresh_carried();
    }

    fn settle_snapshots(self: &Arc<Self>, list_stashes: bool) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        // The places in the flights are taken here, and the status flight
        // held on until the rebuild is asked: a caller closing the reads
        // (`wait_for_snapshot_reads`) has closed that ask too, as when each
        // read asked from inside its own pass — else a baseline's
        // `refresh_log_tracked` would be taken over by this later ask. The
        // pass is counted from here, so `wait_for_graph_passes` covers it.
        let refs = self.refs_read.stamp();
        let tree = self.status_read.stamp();
        let held = self.graph_passes.enter();
        self.status_read.enter();
        let asking = Asking(Arc::clone(self));
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let _held = held;
            let _asking = asking;
            // Before either read, as the poll does (`start_refresh_poll`).
            s.forget_what_the_config_decides();
            let listings = async {
                if list_stashes {
                    s.read_listings().await
                } else {
                    (StashRead::default(), s.read_worktrees().await)
                }
            };
            let (refs, tree, (stashes, listed)) =
                tokio::join!(s.read_refs_at(refs), s.read_status_at(tree), listings);
            // A stash taken or dropped outside moves no ref the refs read
            // lists: the listing is its only word.
            let moved = refs == Reread::Moved || tree == Reread::Moved || stashes.moved;
            if moved || listed.walk {
                let reads = s.pass_reads_listed(&workdir, &s.root_cancel, stashes).await;
                s.refresh_log_with(reads);
            }
        });
    }
}

/// The status flight held by a [`RepoSession::refresh_quick`] until it has
/// asked for the rebuild its reads imply — let go on drop, so one that
/// unwound or went down with the runtime still lets the boundary close.
struct Asking(Arc<RepoSession>);

impl Drop for Asking {
    fn drop(&mut self) {
        self.0.status_read.leave();
    }
}

/// Puts each row's chips on it, answering them by row — what the next
/// refs read diffs against (`apply_refs`).
fn wear_chips(rows: &mut [LogRow], labels: &LabelIndex, tags: bool) -> HashMap<u32, Vec<RefLabel>> {
    let mut applied = HashMap::new();
    for row in rows {
        let at = row.row;
        let Some(commit) = row.commit_mut() else {
            continue;
        };
        let chips = labels.labels_of(&commit.oid, tags);
        if !chips.is_empty() {
            commit.labels = chips.to_vec();
            applied.insert(at, chips.to_vec());
        }
    }
    applied
}
