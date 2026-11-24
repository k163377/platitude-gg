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
    /// worktree being left are dropped, but the command log still hears how a
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
                let (kind, message) = match &error {
                    platitude_core::GitError::NotARepository { bare: true, .. } => {
                        ("bare", error.to_string())
                    }
                    platitude_core::GitError::NotARepository { .. } => ("plain", error.to_string()),
                    // git's words alone: they end in the line that allows
                    // the folder.
                    platitude_core::GitError::UntrustedOwner { stderr, .. } => {
                        ("untrusted", stderr.clone())
                    }
                    // git's words name the file to put right.
                    platitude_core::GitError::ConfigUnreadable { stderr, .. } => {
                        ("config", stderr.clone())
                    }
                    _ => ("other", error.to_string()),
                };
                self.feeds.tab.push(TabMsg::OpenFailed {
                    kind,
                    path: path.to_string_lossy().into_owned(),
                    message,
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
            SessionEvent::LogSilent { silent } => {
                self.feeds.graph.push(GraphMsg::Silent { silent })
            }
            SessionEvent::DiscardWalked { tip } => self.feeds.graph.push(GraphMsg::DiscardWalked {
                tip: crate::encode::oid_text(tip),
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
                    oid_hex: crate::encode::oid_text(oid),
                    published,
                });
            }
            SessionEvent::RefsLoaded { snapshot, looked } => {
                if !self.remotes_already_told(&snapshot) {
                    let origin = &snapshot.marks.origin;
                    self.feeds.tab.push(TabMsg::Remotes {
                        names: snapshot.remote_names.clone(),
                        urls: snapshot.remote_urls.clone(),
                        push_default: origin
                            .push_default
                            .as_ref()
                            .map(|marked| marked.remote.clone())
                            .unwrap_or_default(),
                        push_default_local: origin
                            .push_default
                            .as_ref()
                            .is_some_and(|marked| marked.local),
                        checkout_default: origin.checkout_default.clone().unwrap_or_default(),
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
            // One consumer, unlike the three below: another worktree's changes
            // are one run of paths (`models::nav::Bucket::Whole`).
            SessionEvent::CarriedStatusLoaded { path, name, status } => {
                self.feeds.carried_nav.push_replace(CarriedStatusMsg {
                    at: path,
                    name,
                    status,
                });
            }
            SessionEvent::PacedRead { worktree } => {
                self.feeds.tab.push(TabMsg::PacedRead { worktree })
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
                push,
                eol_marks,
                lfs_needed,
                not_copied,
                stop,
            } => {
                // One message per WIP list: each answers about the whole tree
                // (`NavSectionModel::told`). The rows are the session's one
                // allocation, handed to every reader (`StatusMsg::status`).
                for run in [
                    &self.feeds.status_nav_conflicts,
                    &self.feeds.status_nav_unstaged,
                    &self.feeds.status_nav_staged,
                ] {
                    run.push_replace(StatusMsg {
                        status: Arc::clone(&status),
                        head_seq,
                        looked,
                        op_state,
                        progress,
                        sides: sides.clone(),
                        op_message: op_message.clone(),
                        merge_tool: merge_tool.clone(),
                        push: push.clone(),
                        eol_marks: Arc::clone(&eol_marks),
                        lfs_needed,
                        not_copied: Arc::clone(&not_copied),
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
                        push,
                        eol_marks,
                        lfs_needed,
                        not_copied,
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
                asked,
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
                    asked,
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
            SessionEvent::DiffUnread { target } => {
                self.feeds.diff.push(DiffMsg::Unread { target });
            }
            SessionEvent::CommandStarted {
                id,
                display,
                full,
                at_ms,
                asked,
                operation,
            } => self.feeds.commands.push(CommandMsg::Started {
                run: self.run,
                id,
                display,
                full,
                at_ms,
                asked,
                operation: operation.map_or(0, OperationId::as_u64),
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
            SessionEvent::OpFailed { op, error } => self
                .feeds
                .tab
                .push(TabMsg::op_error(format!("{op}: {error}"))),
            SessionEvent::MoveNeedsAsk { local, start, lost } => {
                self.feeds.tab.push(TabMsg::MoveNeedsAsk {
                    local,
                    start,
                    lost: i32::try_from(lost).unwrap_or(i32::MAX),
                })
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
            SessionEvent::AuthorUnread => self.feeds.tab.push(TabMsg::AuthorUnread),
            SessionEvent::RemoteBranchChecked {
                target,
                local,
                state,
                tip,
                theirs,
            } => {
                self.feeds.tab.push(TabMsg::RemoteBranch {
                    target,
                    local,
                    state: state.as_str().to_string(),
                    tip: crate::encode::oid_text(tip),
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
            // (`hub/tests/plan.rs`).
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
            SessionEvent::AmendChecked { empties } => {
                self.feeds.tab.push(TabMsg::AmendChecked { empties });
            }
            // Unasked fetches go to the toolbar indicator only: no press
            // waits on their id, and their failure raises no panel (デザイン規約
            // §リモートから取り込む). The id rides along for the indicator,
            // which turns until that write's settle (`RepoTab::unshown`).
            SessionEvent::WriteStarted {
                id,
                kind: kind @ (OperationKind::AutoFetch | OperationKind::OpenFetch),
            } => {
                self.feeds.tab.push(TabMsg::AutoFetch {
                    id: id.as_u64(),
                    kind,
                    running: true,
                    error: String::new(),
                });
            }
            SessionEvent::WriteFinished {
                id,
                kind: kind @ (OperationKind::AutoFetch | OperationKind::OpenFetch),
                error,
                ..
            } => {
                self.feeds.tab.push(TabMsg::AutoFetch {
                    id: id.as_u64(),
                    kind,
                    running: false,
                    error: error.unwrap_or_default(),
                });
            }
            // The log's receipt as well as the tab's: the write's commands
            // went out on the log's feed, which drains apart from the tab's.
            SessionEvent::WriteSettled { id, failed, .. } => {
                self.feeds.commands.push(CommandMsg::Settled {
                    operation: id.as_u64(),
                });
                if failed.contains(&platitude_core::session::FollowUp::Status) {
                    self.feeds
                        .tab
                        .push(TabMsg::StatusUnread { id: id.as_u64() });
                }
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
                    self.feeds
                        .tab
                        .push(TabMsg::op_error(format!("{}: {message}", kind.label())));
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

#[cfg(test)]
mod tests {
    use super::*;
    use platitude_core::session::FollowUp;

    /// The two messages a settle puts on the tab's feed, by write.
    fn told(feeds: &Feeds) -> Vec<(&'static str, u64)> {
        feeds
            .tab
            .drain()
            .iter()
            .filter_map(|message| match message {
                TabMsg::StatusUnread { id } => Some(("unread", *id)),
                TabMsg::WriteSettled { id } => Some(("settled", *id)),
                _ => None,
            })
            .collect()
    }

    /// A write that settled behind a status read that failed tells the tab
    /// so under the write's id, ahead of the settle: a reader that takes the
    /// settle as the write's last word has the failure by then. Another
    /// read failing is the settle alone.
    #[test]
    fn a_settle_behind_a_failed_status_read_says_so_ahead_of_the_settle() {
        // An id is the queue's to give: a write through the door of a
        // session nobody drives is accepted and never starts.
        let tab = testkit::installed_tab();
        testkit::open_installed_tab(tab);
        let id = from_session(tab, |s| s.stage_paths(vec!["a.txt".to_string()]))
            .flatten()
            .expect("accepted");
        let feeds = Arc::new(Feeds::default());
        let sink = BridgeSink::new(Arc::clone(&feeds), 1);
        let settled = |failed| SessionEvent::WriteSettled {
            id,
            kind: OperationKind::Stage,
            failed,
        };

        sink.event(settled(vec![FollowUp::Graph]));
        assert_eq!(told(&feeds), [("settled", id.as_u64())]);
        sink.event(settled(vec![FollowUp::Graph, FollowUp::Status]));
        assert_eq!(
            told(&feeds),
            [("unread", id.as_u64()), ("settled", id.as_u64())]
        );
    }

    /// A fetch nobody asked for reaches the tab off the write state's
    /// surface, its start and its answer under the id its settle comes
    /// under: the toolbar's ring turns between the first and the last
    /// (`RepoTab::unshown`).
    #[test]
    fn an_unasked_fetch_is_told_under_the_id_its_settle_comes_under() {
        let tab = testkit::installed_tab();
        testkit::open_installed_tab(tab);
        let id = from_session(tab, |s| s.stage_paths(vec!["a.txt".to_string()]))
            .flatten()
            .expect("accepted");
        let feeds = Arc::new(Feeds::default());
        let sink = BridgeSink::new(Arc::clone(&feeds), 1);
        for kind in [OperationKind::AutoFetch, OperationKind::OpenFetch] {
            sink.event(SessionEvent::WriteStarted { id, kind });
            sink.event(SessionEvent::WriteFinished {
                id,
                kind,
                error: None,
                report: None,
                head_seq: 0,
                reads_from: 0,
            });
            sink.event(SessionEvent::WriteSettled {
                id,
                kind,
                failed: Vec::new(),
            });
            let told: Vec<_> = feeds
                .tab
                .drain()
                .iter()
                .map(|message| match message {
                    TabMsg::AutoFetch {
                        id,
                        kind,
                        running: true,
                        ..
                    } => ("started", *id, Some(*kind)),
                    TabMsg::AutoFetch {
                        id,
                        kind,
                        running: false,
                        ..
                    } => ("answered", *id, Some(*kind)),
                    TabMsg::WriteSettled { id } => ("settled", *id, None),
                    other => panic!("an unasked fetch told as {other:?}"),
                })
                .collect();
            assert_eq!(
                told,
                [
                    ("started", id.as_u64(), Some(kind)),
                    ("answered", id.as_u64(), Some(kind)),
                    ("settled", id.as_u64(), None),
                ]
            );
        }
    }

    /// A status reaches its four readers — the three bucket lists and the
    /// worktree's counts — as the allocation the session sent: copied for
    /// each, every read would copy a list as long as the tree is dirty four
    /// times over.
    #[test]
    fn a_status_reaches_every_reader_as_the_allocation_the_session_sent() {
        let feeds = Arc::new(Feeds::default());
        let sink = BridgeSink::new(Arc::clone(&feeds), 1);
        let status = Arc::new(WorktreeStatus {
            items: vec![platitude_core::status::StatusItem::Untracked {
                path: "new.txt".to_string(),
            }],
            ..WorktreeStatus::default()
        });
        sink.event(SessionEvent::StatusLoaded {
            status: Arc::clone(&status),
            head_seq: 1,
            looked: 1,
            op_state: OpState::default(),
            progress: None,
            sides: platitude_core::conflict::Sides::default(),
            op_message: String::new(),
            merge_tool: String::new(),
            push: platitude_core::remote::StatusPush::default(),
            eol_marks: Arc::default(),
            lfs_needed: 0,
            not_copied: Arc::default(),
            stop: platitude_core::integrate::RebaseStop::default(),
        });

        for list in [
            &feeds.status_nav_conflicts,
            &feeds.status_nav_unstaged,
            &feeds.status_nav_staged,
        ] {
            let told = list.drain().pop().expect("the list was told the status");
            assert!(Arc::ptr_eq(&told.status, &status));
        }
        let counts = feeds.status.drain().pop();
        let Some(StateMsg::Status(told)) = counts else {
            panic!("the worktree's counts were told the status");
        };
        assert!(Arc::ptr_eq(&told.status, &status));
        assert_eq!(
            own_status_bytes(&status),
            0,
            "and no reader of a shared one weighs it as its own"
        );
    }
}
