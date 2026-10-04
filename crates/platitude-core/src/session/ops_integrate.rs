//! History integration: merge, rebase (plain and interactive), sequencer
//! edits, cherry-pick / revert, and conflict / merge-tool handling.

use super::build::{Replay, Rewrite, rewrite_carrying, run_plan, standing_name};
use super::discard_record::{rebase_began_on, stopped_rebase_tips};
use super::*;

/// Numbers every plan ask this process makes, so an answer can say which
/// click it belongs to.
///
/// Process-global: the feed carrying these answers outlives the session
/// that filled it, and a counter restarting at 1 would put a closed
/// session's answer above the new one's.
static NEXT_PLAN_ASK: AtomicU64 = AtomicU64::new(1);

/// Numbers every merge-tool ask, for the log only: a screen stuck on
/// `loading=true choices=0` may be an ask never accepted, an opening never
/// finished or a git read never back, and the stages logged in
/// [`RepoSession::ask_merge_tools`] say which, tied to their ask by this
/// number (two tabs are two sessions; a screen can be opened twice).
static MERGE_TOOL_ASKS: AtomicU64 = AtomicU64::new(1);

impl RepoSession {
    /// Says a write came to rest on a stop ([`SessionEvent::WriteStopped`])
    /// — the one thing the write's own answer cannot say: that the press
    /// is answered by the working tree (デザイン規約 §進行中の操作から出る).
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

    /// `git rebase <upstream>`, carrying uncommitted work across like every
    /// other rewrite here (デザイン規約 §未コミット変更がある状態で履歴を書き換える).
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
                let before = session.tips_before(&exec, &repo, &cancel).await?;
                let head = session.moving_off(&exec, &repo, &head(), &cancel).await?;
                let rewrite = Rewrite::Onto {
                    upstream: &upstream,
                    options: &options,
                };
                let landing = rewrite_carrying(&exec, &repo, &rewrite, &cancel).await?;
                session.note_landing(landing);
                session
                    .record_rebase_group(&exec, &repo, before, &cancel)
                    .await;
                session.tell_if_left(&exec, &repo, &head, &cancel).await;
                Ok(())
            },
        )
    }

    /// `git rebase --interactive` with a plan assembled in the UI. The todo
    /// editor is the helper binary shipped beside the application.
    ///
    /// `expect_head` (full hex) is the tip the plan was composed against
    /// (pinned when the plan opens). A tip moved since — a terminal,
    /// another session — is refused with nothing touched, or the todo
    /// would silently drop whatever landed. It travels with the plan
    /// because the carry spawns the replay twice and both check it
    /// ([`Replay::tip_still_stands`]).
    ///
    /// **A standing operation is refused too**, before anything is spawned.
    /// A merge / cherry-pick / revert stopped from a terminal leaves HEAD
    /// where it was, so the tip check passes, and the replay would destroy
    /// it: git refuses the rebase in the words the carry reads as a dirty
    /// tree, and the stash that follows takes `MERGE_HEAD` with it
    /// (`session_integration::standing_op`). The set is the badge's
    /// (`opText != ""`), bisect included — wider than the carry's own guard.
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
                let before = session.tips_before(&exec, &repo, &cancel).await?;
                let head = session.moving_off(&exec, &repo, &head(), &cancel).await?;
                let replay = Replay::of(&upstream, &steps, options, &expect_head)?;
                let landing =
                    rewrite_carrying(&exec, &repo, &Rewrite::Replay(&replay), &cancel).await?;
                session.note_landing(landing);
                session
                    .record_rebase_group(&exec, &repo, before, &cancel)
                    .await;
                session.tell_if_left(&exec, &repo, &head, &cancel).await;
                Ok(())
            },
        )
    }

    /// Asks what an interactive rebase from `from` (a full commit id)
    /// would be made of; the answer arrives as
    /// [`SessionEvent::RebasePlanLoaded`], or as
    /// [`SessionEvent::RebasePlanRefused`] where the range cannot be
    /// replayed. A read, so it stays off the write queue.
    ///
    /// **One ask at a time** ([`Latest::begin_numbered`]): a second
    /// right-click cancels the earlier read (`from^..HEAD` off a deep
    /// commit walks the whole branch), and the numbering keeps an answer
    /// already past cancelling from overtaking the one the screen awaits.
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
            // A superseded read says nothing: its failure is this end's own
            // cancellation.
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
                    // Both: the failure to the error surface, and word to
                    // the asker so its waiting state comes down.
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
                session.run_plan_told(&exec, &repo, &plan, &cancel).await
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
                session.run_plan_told(&exec, &repo, &plan, &cancel).await
            },
        )
    }

    /// Replaces one commit's message; the newest is amended, not replayed.
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
                    commit::commit(&exec, &repo, &message, options, &cancel).await?;
                    session.tell_if_left(&exec, &repo, &[head], &cancel).await;
                    return Ok(());
                }
                let plan = sequencer::plan_edit(
                    &exec,
                    &repo.workdir,
                    &oid,
                    sequencer::Edit::Reword(message),
                    &cancel,
                )
                .await?;
                session.run_plan_told(&exec, &repo, &plan, &cancel).await
            },
        )
    }

    /// Runs a squash / drop / reword plan, the branches it moved together
    /// and what it moved HEAD off told to the discard log as a rebase's are.
    async fn run_plan_told(
        &self,
        exec: &GitExecutor,
        repo: &RepoInfo,
        plan: &sequencer::EditPlan,
        cancel: &CancellationToken,
    ) -> Result<(), GitError> {
        let before = self.tips_before(exec, repo, cancel).await?;
        let head = self.moving_off(exec, repo, &head(), cancel).await?;
        let landing = run_plan(exec, repo, plan, cancel).await?;
        self.note_landing(landing);
        self.record_rebase_group(exec, repo, before, cancel).await;
        self.tell_if_left(exec, repo, &head, cancel).await;
        Ok(())
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

    /// Continues / aborts / skips whatever operation is in progress. A
    /// rebase moving branches together moves them as it finishes, so the
    /// continue or skip that finishes it puts them on the discard record as
    /// one (破棄記録仕様.md §2); an abort or a quit moves none.
    pub fn resolve_current(
        self: &Arc<Self>,
        continuation: integrate::Continuation,
    ) -> Option<OperationId> {
        let session = Arc::clone(self);
        self.write(
            OperationKind::Resolve,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let (before, began) = match continuation {
                    integrate::Continuation::Continue | integrate::Continuation::Skip => (
                        stopped_rebase_tips(&repo.git_dir),
                        rebase_began_on(&repo.git_dir),
                    ),
                    integrate::Continuation::Abort | integrate::Continuation::Quit => (None, None),
                };
                integrate::resolve_current(&exec, &repo.workdir, continuation, &cancel).await?;
                session
                    .record_rebase_group(&exec, &repo, before, &cancel)
                    .await;
                let began: Vec<Oid> = began.into_iter().collect();
                session.tell_if_left(&exec, &repo, &began, &cancel).await;
                Ok(())
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
    /// Holds the write lock for as long as the user keeps the tool open.
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
    /// (cheap) and tools git found installed (seconds on Windows, kept for
    /// the life of the process).
    ///
    /// Off the write queue: seconds there would hold the poll out and put
    /// every later write behind a dialog nobody is waiting on.
    ///
    /// Answers in up to two waves — config names, then the installed
    /// sweep — with `settled` marking the last, so the fast half never
    /// waits on the slow one. A failed read contributes nothing: the field
    /// is free text, and offering nothing is a working state.
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
            // **Every road out of here ends in the event**: the screen
            // marks its combo loading on accept (`repo_tab::list_merge_tools`)
            // and only this event puts it down. The settings screen can be
            // opened inside the opening (`workdir_when_open` waits that
            // out); an opening that settled without a tree offers nothing.
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
    pub fn set_merge_tool(self: &Arc<Self>, tool: String) -> Option<OperationId> {
        // The refresh behind this write has to re-read the name, or the
        // menu row would repeat the one from before the write.
        self.merge_tool_wanted
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.write(
            // Not `Mergetool`: the pane reads that as a tool being open.
            OperationKind::Config,
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                conflict::set_merge_tool(&exec, &repo.workdir, &tool, &cancel).await
            },
        )
    }
}

/// The rev [`RepoSession::moving_off`] reads for a write that moves HEAD's
/// branch.
fn head() -> Vec<String> {
    vec!["HEAD".to_string()]
}
