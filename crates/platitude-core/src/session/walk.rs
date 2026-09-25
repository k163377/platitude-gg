//! One pass over `git log`: the command both readers run, the streaming
//! and buffered readers themselves, what they send as they go, and what a
//! walk that stopped means.

use super::rows::{LogTotals, Sifter, StreamItem, wip_row};
use super::*;

/// Where a buffered pass puts what it walks: lanes, publish marks and
/// rows — one off-screen picture, so they travel together.
pub(super) struct Building<'a> {
    pub(super) builder: &'a mut GraphBuilder,
    pub(super) marks: &'a mut PublishMarks,
    pub(super) out: &'a mut Vec<LogRow>,
}

/// What a walk's command cannot be built without: HEAD's commit and the
/// stashes' commits.
///
/// Read concurrently ([`RepoSession::walk_inputs`]): each is a process,
/// and in series they stand in front of the first chunk of every opening
/// (ci/baseline/code-costs-windows-x64.md §git のプロセス代). So the
/// stashes are read even for an unborn HEAD — asking HEAD first would
/// bring the series back.
#[derive(Clone)]
pub(super) struct WalkInputs {
    /// `None` is an unborn HEAD: nothing to walk.
    head_tip: Option<Oid>,
    /// Each stash's own commit, by the name it is drawn under.
    stash_refs: HashMap<Oid, String>,
}

impl RepoSession {
    /// The two reads a walk waits on, taken together.
    ///
    /// HEAD is asked of git only where no refs read has answered it
    /// (`known_head_tip`); a failed stash listing leaves the walk without
    /// stash rows.
    pub(super) async fn walk_inputs(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        cancel: &CancellationToken,
    ) -> Result<WalkInputs, GitError> {
        let head = async {
            match self.known_head_tip() {
                Some(tip) => Ok(tip),
                None => refs::head_tip(&self.executor, workdir, cancel).await,
            }
        };
        // Stash oids join the walk; their synthetic parents are sifted out
        // (`Sifter`).
        let stashes = async {
            stash::load(&self.executor, workdir, cancel)
                .await
                .map(|list| {
                    list.into_iter()
                        .map(|s| (s.oid, s.name))
                        .collect::<HashMap<Oid, String>>()
                })
                .unwrap_or_default()
        };
        let (head_tip, stash_refs) = tokio::join!(head, stashes);
        Ok(WalkInputs {
            head_tip: head_tip?,
            stash_refs,
        })
    }

    pub(super) async fn stream_log(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        generation: u64,
        options: LogOptions,
        cancel: &CancellationToken,
        inputs: WalkInputs,
    ) -> Result<LogTotals, GitError> {
        let WalkInputs {
            head_tip,
            stash_refs,
        } = inputs;
        let Some(head_tip) = head_tip else {
            // Unborn HEAD: nothing to log, but the working-tree row can
            // still stand on its own where the first commit will.
            let mut totals = LogTotals::default();
            if self.pending_commit().is_some() {
                self.emit_wip_root_row(generation);
                totals.shown += 1;
            }
            return Ok(totals);
        };

        // The row at the top and the walk under it, decided once
        // (`pending_commit`).
        let pending_row = self.pending_commit();
        let incoming = pending_row.as_deref().unwrap_or_default();

        // Detached working copies, off the worktree read — the only
        // listing that names them (`note_worktree_holders`).
        let standing = self.worktree_holders();
        let detached = detached_oids(&standing);

        let cmd = walk_command(workdir, options, &stash_refs, incoming, &detached);

        let mut parser = LogParser::new();
        let mut pending: Vec<CommitMeta> = Vec::new();
        let mut sifter = Sifter::new(&stash_refs);
        let mut carried = super::rows::CarriedRows::new(&self.carried_current());
        let mut first_sent = false;
        let mut parse_error: Option<String> = None;
        let mut totals = LogTotals {
            head: Some(head_tip),
            ..LogTotals::default()
        };

        // The WIP row goes first so the current chain owns lane 0 from
        // the first paint.
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
                    first_sent = true;
                    self.emit_rows(generation, &items, parser.pool(), &mut totals, &mut carried);
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
            self.emit_rows(generation, &items, parser.pool(), &mut totals, &mut carried);
        }
        totals.walked = sifter.walked;
        Ok(totals)
    }

    /// Buffered variant of [`RepoSession::stream_log`] for offscreen
    /// rebuilds: rows accumulate into `into` without touching shared state
    /// or the sink.
    pub(super) async fn collect_log(
        self: &Arc<Self>,
        workdir: &std::path::Path,
        options: LogOptions,
        cancel: &CancellationToken,
        into: Building<'_>,
        inputs: WalkInputs,
    ) -> Result<LogTotals, GitError> {
        let Building {
            builder,
            marks,
            out,
        } = into;
        let WalkInputs {
            head_tip,
            stash_refs,
        } = inputs;
        let Some(head_tip) = head_tip else {
            // Unborn HEAD (see stream_log).
            if self.pending_commit().is_some() {
                out.push(super::rows::wip_root_row(builder));
            }
            return Ok(LogTotals::default());
        };
        let mut totals = LogTotals {
            head: Some(head_tip),
            ..LogTotals::default()
        };

        // The row and the sides it leashes, decided once (see stream_log).
        let pending_row = self.pending_commit();
        let incoming = pending_row.as_deref().unwrap_or_default();

        if pending_row.is_some() {
            out.push(wip_row(&head_tip, incoming, builder));
        }

        let standing = self.worktree_holders();
        let detached = detached_oids(&standing);

        let cmd = walk_command(workdir, options, &stash_refs, incoming, &detached);

        let mut parser = LogParser::new();
        let mut pending: Vec<CommitMeta> = Vec::new();
        let mut sifter = Sifter::new(&stash_refs);
        let mut carried = super::rows::CarriedRows::new(&self.carried_current());
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
                    for row in carried.take_for(item, builder) {
                        out.push(row);
                    }
                    let row = item.row(parser.pool(), builder, marks);
                    totals.note_row(&item.meta.oid, row.published);
                    out.push(row);
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
            for row in carried.take_for(item, builder) {
                out.push(row);
            }
            let row = item.row(parser.pool(), builder, marks);
            totals.note_row(&item.meta.oid, row.published);
            out.push(row);
        }
        totals.shown = out.len() as u32;
        totals.walked = sifter.walked;
        Ok(totals)
    }

    /// The sides the uncommitted row would leash, or `None` when there is
    /// no such row because nothing is stacked on HEAD.
    ///
    /// Not "is the tree dirty": a merge resolved all-ours and staged
    /// leaves `git status` empty with `MERGE_HEAD` standing, and still
    /// writes a merge. The row draws the commit about to be written, and a
    /// standing operation alone is enough for one (`graph::wip_row_stands`).
    ///
    /// Sides are the merge's alone: a stopped cherry-pick raises the row
    /// the same way but writes no second parent.
    ///
    /// One answer for the row and the walk: the sides join the walk only
    /// to give the row's dotted edges something to land on, so a walk
    /// reaching `MERGE_HEAD` with no row above it would show a commit no
    /// tip names and no edge reaches.
    pub(super) fn pending_commit(&self) -> Option<Vec<Oid>> {
        // Held back where a harness asked
        // ([`PassHooks::holds_back_the_working_tree_row`]).
        if self.holds_back_the_working_tree_row() {
            return None;
        }
        let incoming = self.standing.merge_incoming();
        let stacked = self.standing.wip_dirty() || !incoming.is_empty();
        stacked.then_some(incoming)
    }

    /// Sends the working-tree row for a branch with no commits yet.
    fn emit_wip_root_row(&self, generation: u64) {
        let Some(mut guard) = self.store_shared() else {
            return;
        };
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
        let Some(mut guard) = self.store_shared() else {
            return;
        };
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

    /// Builds graph rows for a batch and sends them under the shared lock
    /// (generations cannot interleave), counting into `totals`.
    ///
    /// A stream may only add to the graph it reset: `generation` must
    /// match the installed one — not `log_gen`, which moves for passes
    /// that reach nobody and would cut a stream still on screen. A refused
    /// batch counts nothing.
    fn emit_rows(
        &self,
        generation: u64,
        batch: &[StreamItem],
        pool: &crate::model::StrPool,
        totals: &mut LogTotals,
        carried: &mut super::rows::CarriedRows,
    ) {
        let tags = self.tags_shown();
        let Some(mut guard) = self.store_shared() else {
            return;
        };
        if guard.generation != generation {
            return;
        }
        let shared = &mut *guard;
        let mut rows = Vec::with_capacity(batch.len());
        for item in batch {
            let shared = &mut *shared;
            for row in carried.take_for(item, &mut shared.builder) {
                rows.push(row);
            }
            let mut row = item.row(pool, &mut shared.builder, &mut shared.publish_marks);
            totals.note_row(&item.meta.oid, row.published);
            let labels = shared.label_map.labels_of(&item.meta.oid, tags).to_vec();
            if !labels.is_empty() {
                row.labels = labels.clone();
                shared.applied.insert(row.row, labels);
            }
            rows.push(row);
        }
        totals.shown += rows.len() as u32;
        shared.sent_rows.extend(rows.iter().map(RowPrint::of));
        self.sink.event(SessionEvent::LogChunk { generation, rows });
    }
}

/// Where the other working copies stand when they stand on no branch, as
/// the walk wants them.
fn detached_oids(standing: &super::joins::WorktreeHolders) -> Vec<Oid> {
    standing.detached.iter().map(|copy| copy.oid).collect()
}

/// The walk both passes run: the same commits in the same order through
/// the same window.
///
/// A stash may vanish between the listing and the walk, so the oids that
/// join it are asked for with `--ignore-missing` — and so may a working
/// copy, which is listed by a read of its own.
fn walk_command(
    workdir: &std::path::Path,
    options: LogOptions,
    stash_refs: &HashMap<Oid, String>,
    incoming: &[Oid],
    detached: &[Oid],
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
    // Stashes, merge sides and detached copies are named by id: none is
    // under `--branches` / `--remotes` (a merge of a tag, `FETCH_HEAD` or a
    // since-deleted branch is reachable by nothing else), and each is a row
    // somebody can be sent to (デザイン規約 §左メニューの所作).
    //
    // `HEAD` is this window's own: another copy's is in the walk only if
    // named here.
    let mut extra = stash_refs
        .keys()
        .chain(incoming)
        .chain(detached.iter())
        .peekable();
    if extra.peek().is_some() {
        cmd = cmd.arg("--ignore-missing");
        for oid in extra {
            cmd = cmd.arg(oid.to_hex());
        }
    }
    cmd
}

/// What a walk that stopped means: with a parse error, the cancel was the
/// parser's own, and the unreadable output is what to report.
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
