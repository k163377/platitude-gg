//! The one place a core session event turns into a feed message.

use super::*;

/// Routes core session events into the per-tab feeds. Runs on background
/// tokio threads, blocking only for the short feed locks.
pub(super) struct BridgeSink {
    pub(super) feeds: Arc<Feeds>,
    /// Which session of its tab this is, stamped on the command log's
    /// messages (`CommandMsg`).
    run: u64,
    /// Set when the tab was released or closed. A write the close let run
    /// on (`RepoSession::close`) answers late, possibly into the fresh
    /// session a reselected tab opened over the same `Feeds`; retired, it
    /// goes nowhere.
    pub(super) retired: std::sync::atomic::AtomicBool,
    /// The same for a page that stays (`Hub::restand_tab`): reads about the
    /// copy being left are dropped, but the command log still hears how a
    /// write that ran on ended — its row is on screen.
    retired_for_reads: std::sync::atomic::AtomicBool,
    /// The refs snapshot the tab last had its remotes from. A quiet tick
    /// republishes the same one (`session::chips`), and the tab is woken
    /// only for a new one. Weak, so the sink keeps nothing alive.
    remotes_told: Mutex<std::sync::Weak<RefsSnapshot>>,
}

impl BridgeSink {
    pub(super) fn new(feeds: Arc<Feeds>, run: u64) -> Self {
        Self {
            feeds,
            run,
            retired: std::sync::atomic::AtomicBool::new(false),
            retired_for_reads: std::sync::atomic::AtomicBool::new(false),
            remotes_told: Mutex::new(std::sync::Weak::new()),
        }
    }

    /// Whether `event` still goes through: everything until retired; after
    /// `retire_reads`, the command log alone.
    fn carries(&self, event: &SessionEvent) -> bool {
        use std::sync::atomic::Ordering::SeqCst;
        if self.retired.load(SeqCst) {
            return false;
        }
        !self.retired_for_reads.load(SeqCst)
            || matches!(
                event,
                SessionEvent::CommandStarted { .. } | SessionEvent::CommandFinished { .. }
            )
    }

    /// Whether `snapshot` is the one the tab already has its remotes
    /// from, marking it as told either way.
    fn remotes_already_told(&self, snapshot: &Arc<RefsSnapshot>) -> bool {
        let mut told = match self.remotes_told.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if std::sync::Weak::ptr_eq(&told, &Arc::downgrade(snapshot)) {
            return true;
        }
        *told = Arc::downgrade(snapshot);
        false
    }

    /// No more of this session's answers reach the feeds.
    pub(super) fn retire(&self) {
        self.retired
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Only the command log's messages reach the feeds from now on (see
    /// `retired_for_reads`).
    pub(super) fn retire_reads(&self) {
        self.retired_for_reads
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

impl SessionSink for BridgeSink {
    #[expect(clippy::too_many_lines)]
    fn event(&self, event: SessionEvent) {
        if !self.carries(&event) {
            return;
        }
        match event {
            SessionEvent::Opened { info } => {
                let title = info
                    .workdir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| info.workdir.to_string_lossy().into_owned());
                self.feeds.tab.push(TabMsg::Opened {
                    title,
                    path: info.workdir.to_string_lossy().into_owned(),
                });
            }
            SessionEvent::OpenFailed { path, error } => {
                let kind = match &error {
                    platitude_core::GitError::NotARepository { bare: true, .. } => "bare",
                    platitude_core::GitError::NotARepository { .. } => "plain",
                    _ => "other",
                };
                self.feeds.tab.push(TabMsg::OpenFailed {
                    kind,
                    path: path.to_string_lossy().into_owned(),
                    message: error.to_string(),
                });
            }
            SessionEvent::LogStarted { generation } => {
                self.feeds.graph.push(GraphMsg::Started { generation });
            }
            SessionEvent::LogChunk { generation, rows } => {
                self.feeds.graph.push(GraphMsg::Chunk { generation, rows });
            }
            SessionEvent::LogFinished {
                generation,
                total,
                elapsed_ms,
                walked,
                truncated,
            } => self.feeds.graph.push(GraphMsg::Finished {
                generation,
                total,
                elapsed_ms,
                walked,
                truncated,
            }),
            SessionEvent::LogFailed { generation, error } => {
                self.feeds.graph.push(GraphMsg::Failed {
                    generation,
                    message: error,
                })
            }
            SessionEvent::LogStale { stale } => self.feeds.graph.push(GraphMsg::Stale { stale }),
            SessionEvent::DiscardWalked { tip } => self.feeds.graph.push(GraphMsg::DiscardWalked {
                tip: tip.map(|tip| tip.to_hex()).unwrap_or_default(),
            }),
            SessionEvent::LogReplaced {
                generation,
                rows,
                elapsed_ms,
                walked,
                truncated,
            } => self.feeds.graph.push(GraphMsg::Replaced {
                generation,
                rows,
                elapsed_ms,
                walked,
                truncated,
            }),
            SessionEvent::LogRelaid {
                generation,
                from,
                rows,
                walked,
                truncated,
            } => self.feeds.graph.push(GraphMsg::Relaid {
                generation,
                from,
                rows,
                walked,
                truncated,
            }),
            SessionEvent::LabelsChanged { generation, rows } => {
                self.feeds.graph.push(GraphMsg::Labels { generation, rows });
            }
            SessionEvent::HeadObserved { head, seq } => {
                let head = HeadMsg::of(&head, seq);
                // Every consumer that draws something at HEAD, and only
                // those; each keeps the newest report.
                self.feeds
                    .status
                    .push_coalescing(StateMsg::Head(head.clone()));
                self.feeds
                    .refs_branches
                    .push_coalescing(RefsMsg::Head(head.clone()));
                self.feeds.graph.push_coalescing(GraphMsg::Head(head));
            }
            SessionEvent::HeadPublished { oid, published } => {
                self.feeds.status.push_coalescing(StateMsg::HeadPublished {
                    oid_hex: oid.map(|o| o.to_hex()).unwrap_or_default(),
                    published,
                });
            }
            SessionEvent::RefsLoaded { snapshot, looked } => {
                if !self.remotes_already_told(&snapshot) {
                    self.feeds.tab.push(TabMsg::Remotes {
                        names: snapshot.remote_names.clone(),
                        urls: snapshot.remote_urls.clone(),
                        push_default: snapshot
                            .push_default
                            .as_ref()
                            .map(|marked| marked.remote.clone())
                            .unwrap_or_default(),
                        push_default_local: snapshot
                            .push_default
                            .as_ref()
                            .is_some_and(|marked| marked.local),
                        checkout_default: snapshot.checkout_default.clone().unwrap_or_default(),
                    });
                }
                self.feeds.refs_branches.push_coalescing(RefsMsg::Snapshot {
                    snapshot: Arc::clone(&snapshot),
                    looked,
                });
                self.feeds.refs_remotes.push_coalescing(RefsMsg::Snapshot {
                    snapshot: Arc::clone(&snapshot),
                    looked,
                });
                self.feeds
                    .refs_tags
                    .push_coalescing(RefsMsg::Snapshot { snapshot, looked });
            }
            // One consumer, unlike the three below: another copy's changes
            // are one run of paths (`models::nav::Bucket::Whole`).
            SessionEvent::CarriedStatusLoaded { path, name, status } => {
                self.feeds.carried_nav.push_replace(CarriedStatusMsg {
                    at: path,
                    name,
                    status,
                });
            }
            SessionEvent::StatusLoaded {
                status,
                head_seq,
                looked,
                op_state,
                progress,
                sides,
                op_message,
                merge_tool,
                push_remote,
                push_track,
                eol_marks,
                lfs_needed,
                stop,
            } => {
                // One copy per WIP list: each answers about the whole tree
                // (`NavSectionModel::told`).
                for run in [
                    &self.feeds.status_nav_conflicts,
                    &self.feeds.status_nav_unstaged,
                    &self.feeds.status_nav_staged,
                ] {
                    run.push_replace(StatusMsg {
                        status: status.clone(),
                        head_seq,
                        looked,
                        op_state,
                        progress,
                        sides: sides.clone(),
                        op_message: op_message.clone(),
                        merge_tool: merge_tool.clone(),
                        push_remote: push_remote.clone(),
                        push_track: push_track.clone(),
                        eol_marks: Arc::clone(&eol_marks),
                        lfs_needed,
                        stop: stop.clone(),
                    });
                }
                self.feeds
                    .status
                    .push_coalescing(StateMsg::Status(Box::new(StatusMsg {
                        status,
                        head_seq,
                        looked,
                        op_state,
                        progress,
                        sides,
                        op_message,
                        merge_tool,
                        push_remote,
                        push_track,
                        eol_marks,
                        lfs_needed,
                        stop,
                    })));
            }
            // `push_replace`: the badge shows where the replay is now, and
            // an undrained tick is stale.
            SessionEvent::OpProgress {
                op_state,
                progress,
                looked,
            } => self.feeds.op_progress.push_replace(OpProgressMsg {
                op_state,
                progress,
                looked,
            }),
            SessionEvent::StashesLoaded { stashes, looked } => {
                self.feeds.stash.push_replace(crate::hub::StashList {
                    entries: stashes,
                    looked,
                })
            }
            SessionEvent::WorktreesLoaded { worktrees, looked } => {
                self.feeds.worktrees.push_replace(crate::hub::WorktreeList {
                    entries: worktrees,
                    looked,
                })
            }
            SessionEvent::DetailsLoaded {
                generation,
                details,
            } => self.feeds.details.push_latest(
                generation,
                DetailsMsg::Loaded {
                    generation,
                    details: Box::new(details),
                },
            ),
            SessionEvent::SelectionLoaded { generation, files } => self
                .feeds
                .details
                .push_latest(generation, DetailsMsg::Selection { generation, files }),
            SessionEvent::DetailsFailed {
                generation,
                oid,
                error,
            } => self.feeds.details.push_latest(
                generation,
                DetailsMsg::Failed {
                    generation,
                    oid_hex: oid.to_hex(),
                    message: format!("details: {error}"),
                },
            ),
            SessionEvent::DiffLoaded {
                target,
                patches,
                preview,
                fingerprint,
                endings,
                marks,
                embedded,
            } => {
                // Every message kept: rows and colours are two messages of
                // one diff, and diffs need not finish in the order asked.
                // `drain` picks by key and drops the rest.
                self.feeds.diff.push(DiffMsg::Loaded {
                    target,
                    patches,
                    preview: preview.map(Box::new),
                    fingerprint,
                    endings,
                    marks,
                    embedded,
                });
            }
            SessionEvent::DiffColoured {
                target,
                colors,
                settled,
            } => {
                self.feeds.diff.push(DiffMsg::Coloured {
                    target,
                    colors,
                    settled,
                });
            }
            // `operation` is not drawn yet: the log reads one command per
            // row.
            SessionEvent::CommandStarted {
                id,
                display,
                full,
                at_ms,
                asked,
                operation: _,
            } => self.feeds.commands.push(CommandMsg::Started {
                run: self.run,
                id,
                display,
                full,
                at_ms,
                asked,
            }),
            SessionEvent::CommandFinished {
                id,
                end,
                waited_ms,
                elapsed_ms,
                message,
            } => {
                let (code, note) = match end {
                    CommandEnd::Exited(code) | CommandEnd::Answered(code) => {
                        (Some(code), String::new())
                    }
                    CommandEnd::TimedOut => (None, "timed out".to_string()),
                    CommandEnd::Cancelled => (None, "cancelled".to_string()),
                    CommandEnd::Failed => (None, "did not run".to_string()),
                };
                self.feeds.commands.push(CommandMsg::Finished {
                    run: self.run,
                    id,
                    code,
                    note,
                    answered: matches!(end, CommandEnd::Answered(_)),
                    waited_ms: waited_ms as i64,
                    elapsed_ms: elapsed_ms as i64,
                    message,
                });
            }
            SessionEvent::OpFailed { op, error } => self.feeds.tab.push(TabMsg::OpError {
                message: format!("{op}: {error}"),
            }),
            SessionEvent::MoveNeedsAsk { local, start } => {
                self.feeds.tab.push(TabMsg::MoveNeedsAsk { local, start })
            }
            SessionEvent::DiscardsChanged { restored } => {
                self.feeds.tab.push(TabMsg::DiscardsChanged { restored })
            }
            SessionEvent::AuthorLoaded { config } => {
                self.feeds.tab.push(TabMsg::Author {
                    complete: config.identity.is_complete(),
                    name: config.identity.name.unwrap_or_default(),
                    email: config.identity.email.unwrap_or_default(),
                    sign_commits: config.signing.sign_commits,
                    signing_format: config.signing.format.as_str().to_string(),
                });
            }
            SessionEvent::RemoteBranchChecked {
                remote,
                branch,
                state,
                tip,
                theirs,
            } => {
                self.feeds.tab.push(TabMsg::RemoteBranch {
                    remote,
                    branch,
                    state: state.as_str().to_string(),
                    tip,
                    theirs: i32::try_from(theirs).unwrap_or(i32::MAX),
                });
            }
            SessionEvent::BranchDeleteChecked { branch, merged } => {
                self.feeds.tab.push(TabMsg::BranchDelete {
                    branch,
                    merged: match merged {
                        Some(true) => "yes",
                        Some(false) => "no",
                        None => "unknown",
                    },
                });
            }
            SessionEvent::SignatureChecked { oid, signature } => {
                use platitude_core::identity::SignatureStatus;
                // Only `G` reads as verified (`SignatureStatus::is_trusted`)
                // and `B` says the content moved; the rest are signatures
                // nobody here can judge. The letter rides along for the
                // tooltip.
                self.feeds.tab.push(TabMsg::Signature {
                    oid,
                    kind: match signature.status {
                        SignatureStatus::Absent => "",
                        SignatureStatus::Good => "verified",
                        SignatureStatus::Bad => "bad",
                        _ => "signed",
                    }
                    .to_string(),
                    code: signature.status.code().to_string(),
                    signer: signature.signer,
                });
            }
            // Ordered by the ask, like details: with a plain replace the
            // older answer can be the one left to drain, which the model
            // drops as stale (`asked_from`) and the screen waits forever
            // (`hub/plan_tests.rs`).
            SessionEvent::RebasePlanLoaded {
                generation,
                preview,
            } => {
                self.feeds
                    .plan
                    .push_latest(generation, PlanMsg::Loaded { preview });
            }
            SessionEvent::RebasePlanRefused {
                generation,
                from,
                refusal,
            } => {
                self.feeds
                    .plan
                    .push_latest(generation, PlanMsg::Refused { from, refusal });
            }
            SessionEvent::RebasePlanFailed { generation, from } => {
                self.feeds
                    .plan
                    .push_latest(generation, PlanMsg::Failed { from });
            }
            SessionEvent::PlanPublished { range, published } => {
                self.feeds.plan.push(PlanMsg::Published {
                    range,
                    published: i32::try_from(published).unwrap_or(i32::MAX),
                });
            }
            SessionEvent::HeadReachChecked { reached_elsewhere } => {
                self.feeds
                    .status
                    .push_coalescing(StateMsg::HeadReach { reached_elsewhere });
            }
            SessionEvent::MergeToolsLoaded { names, settled } => {
                self.feeds.tab.push(TabMsg::MergeTools { names, settled });
            }
            SessionEvent::HeadCommitLoaded { head } => {
                self.feeds.tab.push(TabMsg::HeadCommit {
                    message: head.message,
                    author_name: head.author_name,
                    author_email: head.author_email,
                });
            }
            // Unasked fetches go to the toolbar indicator only: nobody holds
            // an id for them.
            SessionEvent::WriteStarted {
                kind: kind @ (OperationKind::AutoFetch | OperationKind::OpenFetch),
                ..
            } => {
                self.feeds.tab.push(TabMsg::AutoFetch {
                    running: true,
                    error: String::new(),
                    announce: kind == OperationKind::AutoFetch,
                });
            }
            SessionEvent::WriteFinished {
                kind: kind @ (OperationKind::AutoFetch | OperationKind::OpenFetch),
                error,
                ..
            } => {
                self.feeds.tab.push(TabMsg::AutoFetch {
                    running: false,
                    error: error.unwrap_or_default(),
                    announce: kind == OperationKind::AutoFetch,
                });
            }
            SessionEvent::WriteSettled { id, .. } => {
                self.feeds
                    .tab
                    .push(TabMsg::WriteSettled { id: id.as_u64() });
            }
            SessionEvent::WriteStopped { .. } => self.feeds.tab.push(TabMsg::WriteStopped),
            SessionEvent::WriteStarted { id, kind } => {
                self.feeds.tab.push(TabMsg::WriteState {
                    id: id.as_u64(),
                    kind,
                    running: true,
                    error: String::new(),
                    report: None,
                    head_seq: 0,
                    reads_from: 0,
                });
            }
            SessionEvent::WriteFinished {
                id,
                kind,
                error,
                report,
                head_seq,
                reads_from,
            } => {
                // A reported refusal is the page's notice, not an
                // `OpError` (デザイン規約 §答えの要らない報せ).
                if let Some(message) = &error
                    && report.is_none()
                {
                    self.feeds.tab.push(TabMsg::OpError {
                        message: format!("{}: {message}", kind.label()),
                    });
                }
                self.feeds.tab.push(TabMsg::WriteState {
                    id: id.as_u64(),
                    kind,
                    running: false,
                    error: error.unwrap_or_default(),
                    report,
                    head_seq,
                    reads_from,
                });
            }
        }
    }
}
