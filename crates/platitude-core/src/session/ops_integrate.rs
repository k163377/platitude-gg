//! History integration: merge, rebase (plain and interactive), sequencer
//! edits, cherry-pick / revert, and conflict / merge-tool handling.

use super::build::{Replay, Rewrite, rewrite_carrying, run_plan, standing_name};
use super::*;

/// Numbers every plan ask this process makes, so an answer can say which
/// click it belongs to.
///
/// Process-global, for the reason the details read's is
/// ([`super::details_read`]): the feed carrying these answers outlives
/// the session that filled it, and a counter starting again at 1 would
/// leave a closed session's answer sitting above everything the new one
/// asks.
static NEXT_PLAN_ASK: AtomicU64 = AtomicU64::new(1);

/// Numbers every merge-tool ask this process makes.
///
/// **This one exists for the log, not for the code.** The read that
/// answers a settings screen has stopped four times in a gate with the
/// indicator turning and nothing after it (`P3-確認事項.md`), and what
/// the logs could not say was how far it had got: the screen reports
/// `loading=true choices=0` either way, whether the ask was never
/// accepted, or the opening never finished, or a git read never came
/// back. The stages in [`RepoSession::ask_merge_tools`] say which, and
/// the number ties them to the ask they belong to — the slot makes the
/// read single-flight, but two tabs are two sessions and one screen can
/// be opened twice.
static MERGE_TOOL_ASKS: AtomicU64 = AtomicU64::new(1);

impl RepoSession {
    /// Says a write came to rest on a stop, when that is what git did
    /// ([`SessionEvent::WriteStopped`]).
    ///
    /// A stop leaves the operation standing, with everything it did on
    /// screen — the badge, the exit card, the conflicted rows
    /// (デザイン規約 §進行中の操作から出る). The event says the one thing
    /// the write's own answer cannot, that the press is answered by the
    /// working tree.
    ///
    /// Called from inside the write's own task, so the write it speaks
    /// for is the one the queue is serving ([`Self::running_write`]).
    pub(super) fn note_landing(&self, landing: integrate::Landing) {
        if landing != integrate::Landing::Stopped {
            return;
        }
        match self.running_write() {
            Some(write) => self.sink.event(SessionEvent::WriteStopped {
                id: write.id,
                kind: write.kind,
            }),
            None => tracing::warn!("a stop was noted with no write being served"),
        }
    }

    /// `git merge <rev>`.
    pub fn merge(
        self: &Arc<Self>,
        rev: String,
        options: integrate::MergeOptions,
    ) -> Option<OperationId> {
        let session = Arc::clone(self);
        self.write(
            OperationKind::Merge,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let landing =
                    integrate::merge(&exec, &repo.workdir, &rev, &options, &cancel).await?;
                session.note_landing(landing);
                Ok(())
            },
        )
    }

    /// `git rebase <upstream>`, carrying uncommitted work across the way
    /// every other rewrite here does — nothing is asked, and the
    /// staged/unstaged split survives
    /// (デザイン規約 §未コミット変更がある状態で履歴を書き換える).
    pub fn rebase(
        self: &Arc<Self>,
        upstream: String,
        options: integrate::RebaseOptions,
    ) -> Option<OperationId> {
        let session = Arc::clone(self);
        self.write(
            OperationKind::Rebase,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let rewrite = Rewrite::Onto {
                    upstream: &upstream,
                    options: &options,
                };
                let landing = rewrite_carrying(&exec, &repo, &rewrite, &cancel).await?;
                session.note_landing(landing);
                Ok(())
            },
        )
    }

    /// `git rebase --interactive` with a plan assembled in the UI. The todo
    /// editor is the helper binary shipped beside the application.
    ///
    /// `expect_head` (full hex) is the tip the plan was composed against —
    /// anything else refuses, there being no unpinned replay. The screen
    /// pins it when the plan opens; a terminal
    /// or another session moving the branch in between would leave the
    /// plan's todo silently dropping whatever landed, so a tip that moved
    /// is refused and nothing is touched. It travels with the plan,
    /// because the carry spawns the replay twice and both spawns need it
    /// in front of them
    /// ([`Replay::tip_still_stands`]).
    ///
    /// **A standing operation takes the plan's premise too.**
    /// An operation started from a terminal — `git merge topic` that stops
    /// on a conflict, a cherry-pick, a revert — leaves HEAD exactly where
    /// it was, so the tip check sees nothing wrong, and firing the replay
    /// into it destroys the standing operation outright: git refuses the
    /// rebase in the very words the carry reads as a dirty tree, and the
    /// stash that follows takes `MERGE_HEAD` down with it (measured, 2.55 —
    /// `session_integration::standing_op`). The carry has its own guard
    /// now; this one is the plan's, and it says so before anything is
    /// spawned. The set is `opText != ""` on the screen — the same
    /// operations the badge names, bisect included.
    pub fn rebase_interactive(
        self: &Arc<Self>,
        upstream: String,
        steps: Vec<sequencer::RebaseStep>,
        options: integrate::RebaseOptions,
        expect_head: String,
    ) -> Option<OperationId> {
        let session = Arc::clone(self);
        self.write(
            OperationKind::Rebase,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let state = opstate::detect(&exec, &repo.workdir, &cancel).await?;
                if state.any() {
                    return Err(report::rewrite_while_standing(standing_name(&state)));
                }
                let replay = Replay::of(&upstream, &steps, options, &expect_head)?;
                let landing =
                    rewrite_carrying(&exec, &repo, &Rewrite::Replay(&replay), &cancel).await?;
                session.note_landing(landing);
                Ok(())
            },
        )
    }

    /// Asks what an interactive rebase from `from` (a full commit id)
    /// would be made of; the answer arrives as
    /// [`SessionEvent::RebasePlanLoaded`], or as
    /// [`SessionEvent::RebasePlanRefused`] where the range cannot be
    /// replayed. A read like [`RepoSession::check_publish`] — nothing is
    /// touched, so it stays off the write queue.
    ///
    /// **One ask at a time** ([`Latest::begin_numbered`]). The screen has one plan, so
    /// a second right-click replaces the first: the earlier read is
    /// cancelled where it stands — `from^..HEAD` off a deep commit is a
    /// walk of the whole branch, and leaving it running is a walk nobody
    /// will read — and its answer, if it was already past the point of
    /// stopping, is one the numbering keeps from overtaking the answer
    /// the screen is waiting for.
    pub fn ask_rebase_plan(self: &Arc<Self>, from: String) {
        let (generation, cancel) = self
            .plan_read
            .begin_numbered(&self.root_cancel, &NEXT_PLAN_ASK);
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let Some(workdir) = s.workdir() else {
                return;
            };
            if cancel.is_cancelled() {
                return;
            }
            let answer = rebase_plan::preview(&s.executor, &workdir, &from, &cancel).await;
            // A superseded read says nothing at all: the click it answers
            // is one the screen has already left behind, and its failure
            // is this end's own cancellation, which the error surface
            // leaves alone.
            if cancel.is_cancelled() {
                return;
            }
            match answer {
                Ok(rebase_plan::PlanAnswer::Plan(preview)) => {
                    s.sink.event(SessionEvent::RebasePlanLoaded {
                        generation,
                        preview: *preview,
                    });
                }
                Ok(rebase_plan::PlanAnswer::Refused(refusal)) => {
                    s.sink.event(SessionEvent::RebasePlanRefused {
                        generation,
                        from,
                        refusal,
                    });
                }
                Err(error) => {
                    // Both halves: the failure itself to the shared error
                    // surface, and word to the asker so its waiting state
                    // comes down.
                    s.sink
                        .event(SessionEvent::RebasePlanFailed { generation, from });
                    s.fail("rebase-plan", error);
                }
            }
        });
    }

    /// Folds one commit into its parent.
    pub fn squash_into_parent(self: &Arc<Self>, oid: String) -> Option<OperationId> {
        let session = Arc::clone(self);
        self.write(
            OperationKind::Squash,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let plan = sequencer::plan_edit(
                    &exec,
                    &repo.workdir,
                    &oid,
                    sequencer::Edit::SquashIntoParent,
                    &cancel,
                )
                .await?;
                let landing = run_plan(&exec, &repo, &plan, &cancel).await?;
                session.note_landing(landing);
                Ok(())
            },
        )
    }

    /// Leaves one commit out of the history.
    pub fn drop_commit(self: &Arc<Self>, oid: String) -> Option<OperationId> {
        let session = Arc::clone(self);
        self.write(
            OperationKind::Drop,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let plan = sequencer::plan_edit(
                    &exec,
                    &repo.workdir,
                    &oid,
                    sequencer::Edit::Drop,
                    &cancel,
                )
                .await?;
                let landing = run_plan(&exec, &repo, &plan, &cancel).await?;
                session.note_landing(landing);
                Ok(())
            },
        )
    }

    /// Replaces one commit's message.
    ///
    /// The newest commit is amended: an amend touches nothing else,
    /// while a replay would rewrite every commit after it.
    pub fn reword(self: &Arc<Self>, oid: String, message: String) -> Option<OperationId> {
        let session = Arc::clone(self);
        self.write(
            OperationKind::Reword,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let head = commit::head_oid(&exec, &repo.workdir, &cancel).await?;
                if head.to_hex() == oid {
                    let options = CommitOptions {
                        amend: true,
                        ..Default::default()
                    };
                    return commit::commit(&exec, &repo, &message, options, &cancel).await;
                }
                let plan = sequencer::plan_edit(
                    &exec,
                    &repo.workdir,
                    &oid,
                    sequencer::Edit::Reword(message),
                    &cancel,
                )
                .await?;
                let landing = run_plan(&exec, &repo, &plan, &cancel).await?;
                session.note_landing(landing);
                Ok(())
            },
        )
    }

    /// `git cherry-pick <revs>`.
    pub fn cherry_pick(self: &Arc<Self>, revs: Vec<String>) -> Option<OperationId> {
        let session = Arc::clone(self);
        self.write(
            OperationKind::CherryPick,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let landing = integrate::cherry_pick(&exec, &repo.workdir, &revs, &cancel).await?;
                session.note_landing(landing);
                Ok(())
            },
        )
    }

    /// `git revert <revs>`.
    pub fn revert(self: &Arc<Self>, revs: Vec<String>) -> Option<OperationId> {
        let session = Arc::clone(self);
        self.write(
            OperationKind::Revert,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let landing = integrate::revert(&exec, &repo.workdir, &revs, &cancel).await?;
                session.note_landing(landing);
                Ok(())
            },
        )
    }

    /// Continues / aborts / skips whatever operation is in progress.
    pub fn resolve_current(
        self: &Arc<Self>,
        continuation: integrate::Continuation,
    ) -> Option<OperationId> {
        self.write(
            OperationKind::Resolve,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                integrate::resolve_current(&exec, &repo.workdir, continuation, &cancel)
                    .await
                    .map(drop)
            },
        )
    }

    /// Resolves conflicted paths by taking one side wholesale.
    pub fn take_side(
        self: &Arc<Self>,
        paths: Vec<String>,
        side: conflict::Side,
    ) -> Option<OperationId> {
        self.write(
            OperationKind::Resolve,
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                conflict::take_side(&exec, &repo.workdir, &paths, side, &cancel).await
            },
        )
    }

    /// Hands conflicted paths to `git mergetool` (empty = all of them).
    ///
    /// Runs under the write lock like any other write, which means it holds
    /// the lock for as long as the user keeps the tool open.
    pub fn mergetool(self: &Arc<Self>, paths: Vec<String>) -> Option<OperationId> {
        self.write(
            OperationKind::Mergetool,
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                conflict::mergetool(&exec, &repo.workdir, &paths, &cancel).await
            },
        )
    }

    /// Reads what the settings field can offer: tools named in config
    /// (cheap) and tools git found installed (about eight seconds on
    /// Windows, kept for the life of the process).
    ///
    /// A read of its own, for the reason the remote tag read is: eight
    /// seconds on the write queue would hold the poll out and put every
    /// later write behind a dialog nobody is waiting on.
    ///
    /// The answers arrive in up to two waves — config names in
    /// milliseconds, the installed sweep when it lands — with `settled`
    /// marking the last, so the fast half never waits on the slow one.
    /// Either read failing contributes nothing — what it feeds is a
    /// free text field, and offering nothing is a working
    /// state.
    pub fn ask_merge_tools(self: &Arc<Self>) {
        let ask = MERGE_TOOL_ASKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let Ok(permit) = Arc::clone(&self.merge_tools_slot).try_acquire_owned() else {
            tracing::info!(ask, "merge tools: the previous read has not finished");
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let _permit = permit;
            tracing::info!(ask, "merge tools: asked");
            // **Every road out of here ends in the event.** The screen
            // marks its combo loading the instant this is accepted
            // (`repo_tab::list_merge_tools`) and only this event puts
            // it down, so a road that says nothing is an indicator that
            // turns for as long as the window is open. The ask arrives
            // from a settings screen that can be opened inside the
            // opening, where there is no working tree *yet*
            // ([`RepoSession::workdir_when_open`] waits that out); once
            // the opening has settled without one there is nothing to
            // offer, and offering nothing is a working state.
            let Some(workdir) = s.workdir_when_open().await else {
                tracing::info!(ask, "merge tools: the opening settled with no working tree");
                s.sink.event(SessionEvent::MergeToolsLoaded {
                    names: Vec::new(),
                    settled: true,
                });
                return;
            };
            tracing::info!(ask, "merge tools: the working tree is known");
            let cancel = s.root_cancel.clone();
            // Config first: these are deliberate choices, and they arrive
            // in milliseconds where the other takes seconds. Publish them
            // on their own.
            let mut names = conflict::user_defined_tools(&s.executor, &workdir, &cancel)
                .await
                .unwrap_or_default();
            tracing::info!(ask, named = names.len(), "merge tools: what config names");
            if !names.is_empty() {
                s.sink.event(SessionEvent::MergeToolsLoaded {
                    names: names.clone(),
                    settled: false,
                });
            }
            let installed = conflict::available_tools(&s.executor, &workdir, &cancel)
                .await
                .unwrap_or_default();
            tracing::info!(
                ask,
                found = installed.len(),
                "merge tools: what the machine has"
            );
            for name in installed {
                if !names.contains(&name) {
                    names.push(name);
                }
            }
            s.sink.event(SessionEvent::MergeToolsLoaded {
                names,
                settled: true,
            });
        });
    }

    /// Has the next status read name the merge tool even with nothing
    /// conflicted, so a settings field can show what is configured now.
    pub fn ask_merge_tool(self: &Arc<Self>) {
        self.merge_tool_wanted
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.refresh_status();
    }

    /// Records which merge tool to launch; empty clears the choice.
    ///
    /// Refreshing afterwards is what re-reads the name for the menu row,
    /// so the row and the config never disagree for longer than a write.
    pub fn set_merge_tool(self: &Arc<Self>, tool: String) -> Option<OperationId> {
        // The refresh this write triggers has to re-read, or it would
        // repeat the name from before the write.
        self.merge_tool_wanted
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.write(
            // `Config`, because `Mergetool` is what the pane watches to
            // know a tool is open, and writing the setting opens none.
            OperationKind::Config,
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                conflict::set_merge_tool(&exec, &repo.workdir, &tool, &cancel).await
            },
        )
    }
}
