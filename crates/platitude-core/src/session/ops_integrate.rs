//! History integration: merge, rebase (plain and interactive), sequencer
//! edits, cherry-pick / revert, and conflict / merge-tool handling.

use super::build::{Replay, Rewrite, rewrite_carrying, run_plan};
use super::*;

impl RepoSession {
    /// Says a write came to rest on a stop rather than on a commit, when
    /// that is what git did ([`SessionEvent::WriteStopped`]).
    ///
    /// A stop is not a failed write: git left the operation standing and
    /// everything it did is on screen — the badge, the exit card, the
    /// conflicted rows (デザイン規約 §進行中の操作から出る). The event
    /// says the one thing the write's own answer cannot, that the press
    /// is answered by the working tree and not by a commit at the tip.
    fn note_landing(&self, op: &'static str, landing: integrate::Landing) {
        if landing == integrate::Landing::Stopped {
            self.sink.event(SessionEvent::WriteStopped { op });
        }
    }

    /// `git merge <rev>`.
    pub fn merge(self: &Arc<Self>, rev: String, options: integrate::MergeOptions) {
        let session = Arc::clone(self);
        self.write(
            "merge",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let landing =
                    integrate::merge(&exec, &repo.workdir, &rev, &options, &cancel).await?;
                session.note_landing("merge", landing);
                Ok(())
            },
        );
    }

    /// `git rebase <upstream>`, carrying uncommitted work across the way
    /// every other rewrite here does — nothing is asked, and the
    /// staged/unstaged split survives
    /// (デザイン規約 §未コミット変更がある状態で履歴を書き換える).
    pub fn rebase(self: &Arc<Self>, upstream: String, options: integrate::RebaseOptions) {
        self.write(
            "rebase",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let rewrite = Rewrite::Onto {
                    upstream: &upstream,
                    options: &options,
                };
                rewrite_carrying(&exec, &repo, &rewrite, &cancel).await
            },
        );
    }

    /// `git rebase --interactive` with a plan assembled in the UI. The todo
    /// editor is the helper binary shipped beside the application.
    pub fn rebase_interactive(
        self: &Arc<Self>,
        upstream: String,
        steps: Vec<sequencer::RebaseStep>,
        options: integrate::RebaseOptions,
    ) {
        self.write(
            "rebase",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let replay = Replay::of(&upstream, &steps, options)?;
                rewrite_carrying(&exec, &repo, &Rewrite::Replay(&replay), &cancel).await
            },
        );
    }

    /// Folds one commit into its parent.
    pub fn squash_into_parent(self: &Arc<Self>, oid: String) {
        self.write(
            "squash",
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
                run_plan(&exec, &repo, &plan, &cancel).await
            },
        );
    }

    /// Leaves one commit out of the history.
    pub fn drop_commit(self: &Arc<Self>, oid: String) {
        self.write(
            "drop",
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
                run_plan(&exec, &repo, &plan, &cancel).await
            },
        );
    }

    /// Replaces one commit's message.
    ///
    /// The newest commit is amended instead of replayed: an amend touches
    /// nothing else, while a rebase would rewrite every commit after it.
    pub fn reword(self: &Arc<Self>, oid: String, message: String) {
        self.write(
            "reword",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let head = commit::head_oid(&exec, &repo.workdir, &cancel).await?;
                if head.to_hex() == oid {
                    let options = CommitOptions {
                        amend: true,
                        ..Default::default()
                    };
                    return commit::commit(&exec, &repo, &message, options, &cancel)
                        .await
                        .map(drop);
                }
                let plan = sequencer::plan_edit(
                    &exec,
                    &repo.workdir,
                    &oid,
                    sequencer::Edit::Reword(message),
                    &cancel,
                )
                .await?;
                run_plan(&exec, &repo, &plan, &cancel).await
            },
        );
    }

    /// `git cherry-pick <revs>`.
    pub fn cherry_pick(self: &Arc<Self>, revs: Vec<String>) {
        let session = Arc::clone(self);
        self.write(
            "cherry-pick",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let landing = integrate::cherry_pick(&exec, &repo.workdir, &revs, &cancel).await?;
                session.note_landing("cherry-pick", landing);
                Ok(())
            },
        );
    }

    /// `git revert <revs>`.
    pub fn revert(self: &Arc<Self>, revs: Vec<String>) {
        let session = Arc::clone(self);
        self.write(
            "revert",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let landing = integrate::revert(&exec, &repo.workdir, &revs, &cancel).await?;
                session.note_landing("revert", landing);
                Ok(())
            },
        );
    }

    /// Continues / aborts / skips whatever operation is in progress.
    pub fn resolve_current(self: &Arc<Self>, continuation: integrate::Continuation) {
        self.write(
            "resolve",
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                integrate::resolve_current(&exec, &repo.workdir, continuation, &cancel)
                    .await
                    .map(drop)
            },
        );
    }

    /// Resolves conflicted paths by taking one side wholesale.
    pub fn take_side(self: &Arc<Self>, paths: Vec<String>, side: conflict::Side) {
        self.write(
            "resolve",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                conflict::take_side(&exec, &repo.workdir, &paths, side, &cancel).await
            },
        );
    }

    /// Hands conflicted paths to `git mergetool` (empty = all of them).
    ///
    /// Runs under the write lock like any other write, which means it holds
    /// the lock for as long as the user keeps the tool open.
    pub fn mergetool(self: &Arc<Self>, paths: Vec<String>) {
        self.write(
            "mergetool",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                conflict::mergetool(&exec, &repo.workdir, &paths, &cancel).await
            },
        );
    }

    /// Reads what the settings field can offer: tools named in config
    /// (cheap) and tools git found installed (about eight seconds on
    /// Windows, kept for the life of the process).
    ///
    /// Deliberately **not** on the write queue, for the reason the remote
    /// tag read is not: eight seconds there would raise `write_busy`, hold
    /// the poll out, and put every later write behind a dialog nobody is
    /// waiting on.
    ///
    /// The answers arrive in up to two waves — config names in
    /// milliseconds, the installed sweep when it lands — with `settled`
    /// marking the last, so the fast half never waits on the slow one.
    /// Either read failing contributes nothing rather than failing the
    /// pair — what it feeds is a free text field, and offering nothing
    /// is a working state.
    pub fn ask_merge_tools(self: &Arc<Self>) {
        let Ok(permit) = Arc::clone(&self.merge_tools_slot).try_acquire_owned() else {
            tracing::debug!("merge tools: the previous read has not finished");
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let _permit = permit;
            let Some(workdir) = s.workdir() else {
                return;
            };
            let cancel = s.root_cancel.clone();
            // Config first: these are deliberate choices, and they arrive
            // in milliseconds where the other takes seconds. Publish them
            // on their own rather than making them wait for it.
            let mut names = conflict::user_defined_tools(&s.executor, &workdir, &cancel)
                .await
                .unwrap_or_default();
            if !names.is_empty() {
                s.sink.event(SessionEvent::MergeToolsLoaded {
                    names: names.clone(),
                    settled: false,
                });
            }
            let installed = conflict::available_tools(&s.executor, &workdir, &cancel)
                .await
                .unwrap_or_default();
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

    pub(super) fn merge_tool_seen(&self) -> String {
        match self.merge_tool_seen.lock() {
            Ok(g) => g.clone(),
            Err(e) => e.into_inner().clone(),
        }
    }

    pub(super) fn set_merge_tool_seen(&self, tool: String) {
        match self.merge_tool_seen.lock() {
            Ok(mut g) => *g = tool,
            Err(e) => *e.into_inner() = tool,
        }
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
    pub fn set_merge_tool(self: &Arc<Self>, tool: String) {
        // The refresh this write triggers has to re-read, or it would
        // repeat the name from before the write.
        self.merge_tool_wanted
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.write(
            // Not "mergetool": that label is what the pane watches to know
            // a tool is open, and writing the setting is not opening one.
            "config",
            AfterWrite::Snapshots,
            move |exec, repo, cancel| async move {
                conflict::set_merge_tool(&exec, &repo.workdir, &tool, &cancel).await
            },
        );
    }
}
