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
        if head_tip.is_none() {
            return Ok(LogTotals::default());
        }

        // Stashes are part of the graph: their oids join the walk and the
        // synthetic index/untracked parents are sifted out below.
        let stash_refs: HashMap<Oid, String> = stash::load(&self.executor, workdir, cancel)
            .await
            .map(|list| list.into_iter().map(|s| (s.oid, s.name)).collect())
            .unwrap_or_default();

        // What a standing merge is bringing in: the WIP row leashes it,
        // and the walk is told to start there so the leash has a node to
        // land on even when no branch or remote points at it any more.
        let incoming = self.merge_incoming();

        let cmd = walk_command(workdir, options, &stash_refs, &incoming);

        let mut parser = LogParser::new();
        let mut pending: Vec<CommitMeta> = Vec::new();
        let mut sifter = Sifter::new(&stash_refs);
        let mut first_sent = false;
        let mut parse_error: Option<String> = None;
        let mut totals = LogTotals::default();

        // Dirty working tree: prepend the synthetic WIP row so the current
        // chain owns lane 0 from the very first paint.
        if self.wip_dirty.load(Ordering::SeqCst)
            && let Some(head_oid) = head_tip
        {
            self.emit_wip_row(generation, &head_oid, &incoming);
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
        if head_tip.is_none() {
            return Ok(0);
        }

        // A standing merge's sides join the row and the walk here too (see
        // stream_log).
        let incoming = self.merge_incoming();

        // Dirty working tree: prepend the synthetic WIP row (mirrors
        // stream_log).
        if self.wip_dirty.load(Ordering::SeqCst)
            && let Some(head_oid) = head_tip
        {
            out.push(wip_row(&head_oid, &incoming, builder));
        }

        // Stashes join the walk here too (see stream_log).
        let stash_refs: HashMap<Oid, String> = stash::load(&self.executor, workdir, cancel)
            .await
            .map(|list| list.into_iter().map(|s| (s.oid, s.name)).collect())
            .unwrap_or_default();

        let cmd = walk_command(workdir, options, &stash_refs, &incoming);

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

    /// Sends the synthetic WIP row (dirty working tree) as its own chunk.
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
