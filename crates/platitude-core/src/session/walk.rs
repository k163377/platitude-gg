//! One pass over `git log`: the command both readers run, the streaming
//! and buffered readers themselves, what they send as they go, and what a
//! walk that stopped means.

use super::rows::{LogTotals, Sifter, StreamItem, wip_row};
use super::*;

impl RepoSession {
    pub(super) async fn stream_log(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        generation: u64,
        options: LogOptions,
        cancel: &CancellationToken,
    ) -> Result<LogTotals, GitError> {
        // An unborn HEAD has nothing to log. The refs read already
        // answered this, and asking git again is two processes in front
        // of the first chunk on every rebuild (`known_head_tip`).
        let head_tip = match self.known_head_tip() {
            Some(tip) => tip,
            None => refs::head_state(&self.executor, workdir, cancel).await?.oid,
        };
        let Some(head_tip) = head_tip else {
            // No commits yet, but there can still be something to commit:
            // the working-tree row does not hang off HEAD, so it stands
            // here on its own where the first commit will.
            let mut totals = LogTotals::default();
            if self.pending_commit().is_some() {
                self.emit_wip_root_row(generation);
                totals.shown += 1;
            }
            return Ok(totals);
        };

        // Stashes are part of the graph: their oids join the walk and the
        // synthetic index/untracked parents are sifted out below.
        let stash_refs: HashMap<Oid, String> = stash::load(&self.executor, workdir, cancel)
            .await
            .map(|list| list.into_iter().map(|s| (s.oid, s.name)).collect())
            .unwrap_or_default();

        // The row at the top and the walk under it, decided once (see
        // `pending_commit`): the sides a standing merge brings in are
        // leashed by that row, and the walk is told to start there so
        // each leash has a node to land on even when no branch or remote
        // points at it any more.
        let pending_row = self.pending_commit();
        let incoming = pending_row.as_deref().unwrap_or_default();

        let cmd = walk_command(workdir, options, &stash_refs, incoming);

        let mut parser = LogParser::new();
        let mut pending: Vec<CommitMeta> = Vec::new();
        let mut sifter = Sifter::new(&stash_refs);
        let mut first_sent = false;
        let mut parse_error: Option<String> = None;
        let mut totals = LogTotals::default();

        // Something to commit: prepend the synthetic WIP row so the
        // current chain owns lane 0 from the very first paint.
        if pending_row.is_some() {
            self.emit_wip_row(generation, &head_tip, incoming);
            totals.shown += 1;
        }

        let result = self
            .executor
            .run_streaming(cmd, cancel, &mut |bytes| {
                if parse_error.is_some() {
                    return;
                }
                if let Err(e) = parser.feed(bytes, &mut pending) {
                    parse_error = Some(e.to_string());
                    cancel.cancel();
                    return;
                }
                let threshold = if first_sent {
                    CHUNK_ROWS
                } else {
                    FIRST_CHUNK_ROWS
                };
                if pending.len() >= threshold {
                    let items = sifter.take(&mut pending);
                    totals.shown += items.len() as u32;
                    first_sent = true;
                    self.emit_rows(generation, &items, parser.pool());
                }
            })
            .await;

        if let Err(e) = result {
            return Err(map_walk_error(e, parse_error));
        }
        parser
            .finish()
            .map_err(|e| unreadable_walk(e.to_string()))?;
        if !pending.is_empty() {
            let items = sifter.take(&mut pending);
            totals.shown += items.len() as u32;
            self.emit_rows(generation, &items, parser.pool());
        }
        totals.walked = sifter.walked;
        Ok(totals)
    }

    /// Buffered variant of [`RepoSession::stream_log`]: rows accumulate
    /// into the caller's builder/vec without touching shared state or the
    /// sink (used by every offscreen rebuild — the tag-inclusive swap
    /// pass and `refresh_log`'s background refreshes). Returns the number
    /// of commits the walk emitted (what `--max-count` limits — the shown
    /// row count is `out.len()`).
    pub(super) async fn collect_log(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        options: LogOptions,
        cancel: &CancellationToken,
        builder: &mut GraphBuilder,
        out: &mut Vec<LogRow>,
    ) -> Result<u32, GitError> {
        // An unborn HEAD has nothing to log (see stream_log).
        let head_tip = match self.known_head_tip() {
            Some(tip) => tip,
            None => refs::head_state(&self.executor, workdir, cancel).await?.oid,
        };
        let Some(head_tip) = head_tip else {
            // Still something to commit (see stream_log).
            if self.pending_commit().is_some() {
                out.push(super::rows::wip_root_row(builder));
            }
            return Ok(0);
        };

        // The row and the sides it leashes, decided once (see stream_log).
        let pending_row = self.pending_commit();
        let incoming = pending_row.as_deref().unwrap_or_default();

        // Something to commit: prepend the synthetic WIP row (mirrors
        // stream_log).
        if pending_row.is_some() {
            out.push(wip_row(&head_tip, incoming, builder));
        }

        // Stashes join the walk here too (see stream_log).
        let stash_refs: HashMap<Oid, String> = stash::load(&self.executor, workdir, cancel)
            .await
            .map(|list| list.into_iter().map(|s| (s.oid, s.name)).collect())
            .unwrap_or_default();

        let cmd = walk_command(workdir, options, &stash_refs, incoming);

        let mut parser = LogParser::new();
        let mut pending: Vec<CommitMeta> = Vec::new();
        let mut sifter = Sifter::new(&stash_refs);
        let mut parse_error: Option<String> = None;

        let result = self
            .executor
            .run_streaming(cmd, cancel, &mut |bytes| {
                if parse_error.is_some() {
                    return;
                }
                if let Err(e) = parser.feed(bytes, &mut pending) {
                    parse_error = Some(e.to_string());
                    cancel.cancel();
                    return;
                }
                for item in &sifter.take(&mut pending) {
                    out.push(item.row(parser.pool(), builder));
                }
            })
            .await;
        if let Err(e) = result {
            return Err(map_walk_error(e, parse_error));
        }
        parser
            .finish()
            .map_err(|e| unreadable_walk(e.to_string()))?;
        for item in &sifter.take(&mut pending) {
            out.push(item.row(parser.pool(), builder));
        }
        Ok(sifter.walked)
    }

    /// The sides the uncommitted row would leash, or `None` when there is
    /// no such row because nothing is stacked on HEAD.
    ///
    /// A different question from "is the tree dirty". Resolving every
    /// conflict of a merge as ours and staging it leaves `git status`
    /// empty with `MERGE_HEAD` still standing, and the commit written
    /// there is still a merge carrying both parents (measured 2.55 — a
    /// cherry-pick in the same state refuses the commit instead, which is
    /// why a stopped sequence is not this row). The row draws the commit
    /// that is about to be written, so it is there whenever there is one.
    ///
    /// One answer for the row and for the walk, because they are one
    /// decision: the sides join the walk only to give the row's dotted
    /// edges something to land on, so a walk that reached `MERGE_HEAD`
    /// with no row above it would leave a commit on screen that no tip
    /// names and no edge reaches — appearing and vanishing with a merge
    /// the graph never mentions.
    fn pending_commit(&self) -> Option<Vec<Oid>> {
        let incoming = self.merge_incoming();
        let stacked = self.wip_dirty.load(Ordering::SeqCst) || !incoming.is_empty();
        stacked.then_some(incoming)
    }

    /// Sends the same row for a branch with no commits yet.
    fn emit_wip_root_row(&self, generation: u64) {
        let mut guard = self.lock_shared();
        if guard.generation != generation {
            return; // this stream is not the graph on screen (see emit_rows)
        }
        let row = super::rows::wip_root_row(&mut guard.builder);
        guard.sent_rows.push(RowPrint::of(&row));
        self.sink.event(SessionEvent::LogChunk {
            generation,
            rows: vec![row],
        });
    }

    /// Sends the synthetic WIP row as its own chunk.
    fn emit_wip_row(&self, generation: u64, head: &Oid, incoming: &[Oid]) {
        let mut guard = self.lock_shared();
        if guard.generation != generation {
            return; // this stream is not the graph on screen (see emit_rows)
        }
        let row = wip_row(head, incoming, &mut guard.builder);
        guard.sent_rows.push(RowPrint::of(&row));
        self.sink.event(SessionEvent::LogChunk {
            generation,
            rows: vec![row],
        });
    }

    /// Builds graph rows for a batch and sends them (holding the shared
    /// lock so generations cannot interleave).
    ///
    /// A stream may only add to the graph it reset: `generation` matching
    /// the installed one is what says these rows belong to what the
    /// consumer shows. The counter would answer a different question —
    /// it moves for passes that never reach anybody, and a stream still
    /// on screen would stop delivering halfway through.
    fn emit_rows(&self, generation: u64, batch: &[StreamItem], pool: &crate::model::StrPool) {
        let tags = self.tags_shown();
        let mut guard = self.lock_shared();
        if guard.generation != generation {
            return;
        }
        let shared = &mut *guard;
        let mut rows = Vec::with_capacity(batch.len());
        for item in batch {
            let mut row = item.row(pool, &mut shared.builder);
            let labels = shared.label_map.labels_of(&item.meta.oid, tags).to_vec();
            if !labels.is_empty() {
                row.labels = labels.clone();
                shared.applied.insert(row.row, labels);
            }
            rows.push(row);
        }
        shared.sent_rows.extend(rows.iter().map(RowPrint::of));
        self.sink.event(SessionEvent::LogChunk { generation, rows });
    }
}

/// The walk both passes run: the same commits in the same order through
/// the same window.
///
/// A stash may vanish between the listing and the walk, so the oids that
/// join it are asked for with `--ignore-missing`.
fn walk_command(
    workdir: &std::path::Path,
    options: LogOptions,
    stash_refs: &HashMap<Oid, String>,
    incoming: &[Oid],
) -> GitCommand {
    let mut cmd = GitCommand::new()
        .cwd(workdir)
        .args(["log", "-z", "--date-order", LOG_FORMAT_ARG])
        .args(["HEAD", "--branches", "--remotes"])
        .no_timeout();
    if options.include_tags {
        cmd = cmd.arg("--tags");
    }
    if let Some(limit) = options.limit {
        cmd = cmd.arg(format!("--max-count={limit}"));
    }
    // Stashes and the sides of a standing merge are named by id: neither
    // is under `--branches` or `--remotes` (a merge of a tag, of
    // `FETCH_HEAD`, or of a branch deleted since it stopped is reachable
    // by nothing else), and the row that leashes them needs the node its
    // dotted edge lands on.
    let mut extra = stash_refs.keys().chain(incoming).peekable();
    if extra.peek().is_some() {
        cmd = cmd.arg("--ignore-missing");
        for oid in extra {
            cmd = cmd.arg(oid.to_hex());
        }
    }
    cmd
}

/// What a walk that stopped means.
///
/// A self-inflicted cancel means the parser hit a fatal error: it cancels
/// the run itself, so the cancellation is how that arrives here, and what
/// could not be read is the answer worth reporting.
fn map_walk_error(error: GitError, parse_error: Option<String>) -> GitError {
    match parse_error {
        Some(message) => unreadable_walk(message),
        None => error,
    }
}

/// git log said something the parser could not read.
fn unreadable_walk(message: String) -> GitError {
    GitError::UnexpectedOutput {
        command: "git log".to_string(),
        message,
    }
}
