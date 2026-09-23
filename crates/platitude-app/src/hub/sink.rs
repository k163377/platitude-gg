//! The one place a core session event turns into a feed message.

use super::*;

/// Routes core session events into the per-tab feeds. Runs on background
/// tokio threads, blocking only for the short feed locks.
pub(super) struct BridgeSink {
    pub(super) feeds: Arc<Feeds>,
    /// Which session of its tab this one speaks for, counting from one.
    /// Stamped on the command log's messages, which are the only ones
    /// that outlive the session that made them (`CommandMsg`).
    run: u64,
    /// Set when the tab this sink fed was released or closed
    /// (`Hub::release_tab` / `Hub::close_tab`). A write the close let run
    /// on (`RepoSession::close`) answers minutes later — into a page that
    /// no longer exists, or worse, into the fresh session a reselected
    /// tab has opened over the same `Feeds`. Retired, the late answers
    /// go nowhere at all.
    pub(super) retired: std::sync::atomic::AtomicBool,
    /// The same, for a page that is *staying*: the tab has been stood in
    /// another working copy (`Hub::restand_tab`), so everything this
    /// session has left to say about a repository would land on a page
    /// reading another copy of it — but the command log is the record of
    /// what this window ran, and the write the close let run on is still
    /// running. Its row is on screen saying so, and this is what lets it
    /// say how it ended.
    retired_for_reads: std::sync::atomic::AtomicBool,
    /// The refs snapshot the tab was last told its remotes out of. A quiet
    /// tick republishes the very same one (`session::chips`), and the tab
    /// — every binding on it — is woken only for a snapshot it has
    /// yet to hear. Weak, so the sink keeps nothing alive.
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

    /// Whether `event` is one this sink still carries. Everything is,
    /// until the session is let go of; after that it is nothing, or —
    /// where the page stayed and only the copy under it changed — the
    /// command log alone (see the two members).
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

    /// No more of this session's answers reach the feeds — the page is
    /// gone, and the feeds may already be speaking for its successor.
    pub(super) fn retire(&self) {
        self.retired
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Nothing this session has left to say about a repository reaches
    /// the feeds, and what it has left to say about its own commands
    /// still does (see `retired_for_reads`).
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
            SessionEvent::LabelsChanged { generation, rows } => {
                self.feeds.graph.push(GraphMsg::Labels { generation, rows });
            }
            SessionEvent::HeadObserved { head, seq } => {
                let head = HeadMsg::of(&head, seq);
                // Every consumer that draws something at HEAD, and only
                // those: the headline, the branches section (its
                // highlighted row and the stand-in that rides above it)
                // and the graph (the row the pin leads to). Each holds
                // the newest report and nothing older.
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
                // The tab hears about its remotes only when the snapshot
                // is a new one: a quiet tick republishes the same
                // pointer, and the bindings on the tab are left where
                // they are.
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
            // One list and one consumer, where the window's own status is
            // copied to three below: another copy's changes are shown as
            // one run of paths (`models::nav::Bucket::Whole`).
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
                op_state,
                progress,
                sides,
                op_message,
                merge_tool,
                push_remote,
                eol_marks,
                stop,
            } => {
                // One copy per bucket run: each of the WIP pane's lists
                // shows a run of its own and answers about the whole tree
                // (`NavSectionModel::told`), so each holds the status.
                for run in [
                    &self.feeds.status_nav_conflicts,
                    &self.feeds.status_nav_unstaged,
                    &self.feeds.status_nav_staged,
                ] {
                    run.push_replace(StatusMsg {
                        status: status.clone(),
                        head_seq,
                        op_state,
                        progress,
                        sides: sides.clone(),
                        op_message: op_message.clone(),
                        merge_tool: merge_tool.clone(),
                        push_remote: push_remote.clone(),
                        eol_marks: Arc::clone(&eol_marks),
                        stop: stop.clone(),
                    });
                }
                self.feeds
                    .status
                    .push_coalescing(StateMsg::Status(Box::new(StatusMsg {
                        status,
                        head_seq,
                        op_state,
                        progress,
                        sides,
                        op_message,
                        merge_tool,
                        push_remote,
                        eol_marks,
                        stop,
                    })));
            }
            // `push_replace`, like the snapshot it is a slice of: what the
            // badge shows is where the replay is *now*, and a tick the GUI
            // thread was too busy to drain is a number nobody wants back.
            SessionEvent::OpProgress { op_state, progress } => self
                .feeds
                .op_progress
                .push_replace(OpProgressMsg { op_state, progress }),
            SessionEvent::StashesLoaded { stashes, looked } => {
                self.feeds.stash.push_replace(crate::hub::StashList {
                    entries: stashes,
                    looked,
                })
            }
            SessionEvent::WorktreesLoaded { worktrees } => {
                self.feeds.worktrees.push_replace(worktrees)
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
                // Every message kept: rows and colours are two messages
                // of one diff, and two diffs asked for a moment apart
                // need not finish in that order — the newest arrival is
                // not always the wanted one. The consumer's `drain`
                // picks by key and drops the rest, so nothing
                // accumulates.
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
            // The write a command ran under is not drawn yet: the log
            // reads its rows one command at a time.
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
                // Eight verdicts, three outcomes: only `G` may read as
                // verified (`SignatureStatus::is_trusted`), `B` is the one
                // that says the content moved, and everything else in
                // between is a signature nobody here can judge. The letter
                // rides along so the tooltip can say which one it was.
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
            // Ordered by the ask, the way the details feed is and for
            // the same reason: two right-clicks a moment apart need not
            // finish in that order, and a plain replace lets the *older*
            // answer be the one waiting when the consumer drains — which
            // the model then drops as stale (`asked_from`), leaving the
            // newer click unanswered and the screen waiting on a plan
            // that can no longer arrive.
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
            // The fetches nobody asked for are the toolbar indicator's:
            // nobody holds an id for them, and the indicator is all an
            // offline machine shows for them.
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
            // The page waits on the answer (`TabMsg::WriteState`). The
            // run that photographs the page a write leaves waits on this
            // one: it is what says the last of those reads has been
            // published.
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
                // A write that did not happen and has something to say
                // for itself is the page's to report: it says so in
                // words of its own, and the red line that would say
                // "something went wrong here" is left for the failures
                // nothing else answers
                // (デザイン規約 §答えの要らない報せ).
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
