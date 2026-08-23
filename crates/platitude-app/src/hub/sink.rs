//! The one place a core session event turns into a feed message.

use super::*;

/// Routes core session events into the per-tab feeds. Runs on background
/// tokio threads; must never block beyond the short feed locks.
pub(super) struct BridgeSink {
    pub(super) feeds: Arc<Feeds>,
}

impl SessionSink for BridgeSink {
    #[expect(clippy::too_many_lines)]
    fn event(&self, event: SessionEvent) {
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
            SessionEvent::RefsLoaded { snapshot } => {
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
                });
                self.feeds.refs_branches.push_replace(Arc::clone(&snapshot));
                self.feeds.refs_remotes.push_replace(Arc::clone(&snapshot));
                self.feeds.refs_tags.push_replace(snapshot);
            }
            SessionEvent::StatusLoaded {
                status,
                op_state,
                progress,
                sides,
                op_message,
                merge_tool,
                push_remote,
                eol_marks,
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
                        op_state,
                        progress,
                        sides: sides.clone(),
                        op_message: op_message.clone(),
                        merge_tool: merge_tool.clone(),
                        push_remote: push_remote.clone(),
                        eol_marks: Arc::clone(&eol_marks),
                    });
                }
                self.feeds.status.push_replace(StatusMsg {
                    status,
                    op_state,
                    progress,
                    sides,
                    op_message,
                    merge_tool,
                    push_remote,
                    eol_marks,
                });
            }
            SessionEvent::StashesLoaded { stashes } => self.feeds.stash.push_replace(stashes),
            SessionEvent::WorktreesLoaded { worktrees } => {
                self.feeds.worktrees.push_replace(worktrees)
            }
            SessionEvent::DetailsLoaded { details } => self.feeds.details.push_replace(details),
            SessionEvent::DiffLoaded {
                target,
                patches,
                preview,
                fingerprint,
                endings,
            } => {
                // Kept rather than replaced, unlike every other feed here:
                // rows and colours are two messages of one diff, and two
                // diffs asked for a moment apart need not finish in that
                // order — the newest arrival is not always the wanted one.
                // The consumer's `drain` picks by key and drops the rest,
                // so nothing accumulates.
                self.feeds.diff.push(DiffMsg::Loaded {
                    target,
                    patches,
                    preview,
                    fingerprint,
                    endings,
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
            SessionEvent::CommandStarted {
                id,
                display,
                full,
                at_ms,
            } => self.feeds.commands.push(CommandMsg::Started {
                id,
                display,
                full,
                at_ms,
            }),
            SessionEvent::CommandFinished {
                id,
                end,
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
                    id,
                    code,
                    note,
                    answered: matches!(end, CommandEnd::Answered(_)),
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
                self.feeds.tab.push(TabMsg::BranchDelete { branch, merged });
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
            SessionEvent::PublishChecked { range, state } => {
                self.feeds.tab.push(TabMsg::Publish {
                    range,
                    total: state.total as i32,
                    published: state.published() as i32,
                });
            }
            SessionEvent::InHistoryChecked { oid, in_history } => {
                self.feeds.tab.push(TabMsg::InHistory { oid, in_history });
            }
            SessionEvent::HeadReachChecked { reached_elsewhere } => {
                self.feeds.tab.push(TabMsg::HeadReach { reached_elsewhere });
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
            SessionEvent::WriteStarted { op } if op == AUTO_FETCH_OP || op == OPEN_FETCH_OP => {
                self.feeds.tab.push(TabMsg::AutoFetch {
                    running: true,
                    error: String::new(),
                    announce: op == AUTO_FETCH_OP,
                });
            }
            SessionEvent::WriteFinished { op, error }
                if op == AUTO_FETCH_OP || op == OPEN_FETCH_OP =>
            {
                self.feeds.tab.push(TabMsg::AutoFetch {
                    running: false,
                    error: error.unwrap_or_default(),
                    announce: op == AUTO_FETCH_OP,
                });
            }
            SessionEvent::WriteStopped { .. } => self.feeds.tab.push(TabMsg::WriteStopped),
            SessionEvent::WriteStarted { op } => self.feeds.tab.push(TabMsg::WriteState {
                op: op.to_string(),
                running: true,
                error: String::new(),
            }),
            SessionEvent::WriteFinished { op, error } => {
                if let Some(message) = &error {
                    self.feeds.tab.push(TabMsg::OpError {
                        message: format!("{op}: {message}"),
                    });
                }
                self.feeds.tab.push(TabMsg::WriteState {
                    op: op.to_string(),
                    running: false,
                    error: error.unwrap_or_default(),
                });
            }
        }
    }
}
