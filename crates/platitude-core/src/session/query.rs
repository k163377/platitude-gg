//! On-demand reads answered as events: a plan's publish state,
//! signatures and file diffs.

use super::*;
use crate::eol;

/// What one read of the diff on screen established.
///
/// A completion boundary, because the ordinary answer is silence: an
/// untouched file and a read the pane has passed both send nothing, and
/// waiting cannot tell "none yet" from "none coming"
/// (core.md §非同期・並行テスト).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffReadOutcome {
    /// The repository is not open, so nothing was read.
    Unavailable,
    /// Reading the diff failed; the failure went out as one.
    Failed,
    /// The file is byte for byte the one the pane already holds — the
    /// re-read's own answer ([`RepoSession::refresh_diff`]), which a first
    /// read never gives.
    Unchanged,
    /// The pane asked for a later read while this one ran, so nothing of
    /// it went out (see [`RepoSession::diff_epoch`]).
    Overtaken,
    /// The diff went out.
    Sent,
    /// The session closed before the read could answer.
    Cancelled,
}

/// Completion of one explicitly tracked diff re-read.
pub struct DiffRefreshTask(tokio::sync::oneshot::Receiver<DiffReadOutcome>);

impl DiffRefreshTask {
    fn pending() -> (tokio::sync::oneshot::Sender<DiffReadOutcome>, Self) {
        let (send, receive) = tokio::sync::oneshot::channel();
        (send, Self(receive))
    }

    fn ready(outcome: DiffReadOutcome) -> Self {
        let (send, task) = Self::pending();
        if send.send(outcome).is_err() {
            tracing::trace!("diff re-read completion was not observed");
        }
        task
    }

    /// Waits for the re-read itself to finish; a dropped runtime reads as
    /// cancellation (no later answer can arrive).
    pub async fn outcome(self) -> DiffReadOutcome {
        self.0.await.unwrap_or(DiffReadOutcome::Cancelled)
    }
}

impl RepoSession {
    /// Asks again how many commits of a plan's range a remote already has
    /// (`rebase_plan::PlanPreview::published`), after the refs moved under
    /// the open plan. The answer names the range, so a plan since put away
    /// drops it.
    ///
    /// A plan's range starts from HEAD, which has commits, so there is no
    /// unborn case to keep off git here.
    pub fn check_plan_published(self: &Arc<Self>, range: String) {
        self.spawn_read("publish", |s, workdir, cancel| async move {
            let state = publish::state_of(&s.executor, &workdir, &range, &cancel).await?;
            Ok(SessionEvent::PlanPublished {
                range,
                published: state.published(),
            })
        });
    }

    /// Asks whether `oid` carries a signature and what git makes of it.
    ///
    /// A read of its own: verifying runs gpg or ssh-keygen, and the
    /// details pane has a 100ms budget, so the answer arrives after the
    /// commit is on screen. One at a time (`session::latest`): a selection
    /// that moves on cancels the last verification's process.
    pub fn check_signature(self: &Arc<Self>, oid: Oid) {
        self.spawn_read_latest(
            "signature",
            &self.signature_read,
            move |s, workdir, cancel| async move {
                let signature =
                    identity::verify_commit(&s.executor, &workdir, &oid.to_hex(), &cancel).await?;
                Ok(SessionEvent::SignatureChecked {
                    oid: oid.to_hex(),
                    signature,
                })
            },
        );
    }

    /// Loads a unified diff for one file, and its colours behind it.
    pub fn load_diff(self: &Arc<Self>, target: DiffTarget) {
        self.runtime.spawn(self.read_diff(target));
    }

    /// The same read aimed at another working copy: one of the files the
    /// read-only pane is listing (`RepoSession::read_carried_status`).
    ///
    /// Everything a diff is made of is asked of a working copy by path,
    /// so only the aim differs. It shares the pane's epoch with the
    /// ordinary reads, so a click either way passes the read in flight.
    ///
    /// Nothing here can be staged from: the pane has every write control
    /// down while showing another copy, and the fingerprint a partial
    /// stage is verified against is recorded under the copy it was read
    /// from (`last_diff`), so a carried selection matches nothing here.
    pub fn load_carried_diff(self: &Arc<Self>, at: String, target: DiffTarget) {
        let s = Arc::clone(self);
        let workdir = PathBuf::from(at);
        let epoch = self.claim_diff_epoch();
        self.runtime.spawn(async move {
            s.publish_diff(workdir, target, None, epoch).await;
        });
    }

    /// The same read as a future the caller drives — its completion
    /// boundary and the only way to hold one open: it is the pane's newest
    /// from the moment it is asked for, so a test can stop it inside its
    /// git and let the next one pass it (core.md §非同期・並行テスト).
    pub fn read_diff(
        self: &Arc<Self>,
        target: DiffTarget,
    ) -> impl Future<Output = DiffReadOutcome> + Send + 'static + use<> {
        // Claimed at the ask, so asking is what passes the read before —
        // and only with a working tree, so an unopened repository passes
        // nothing (`diff_epoch`).
        let started = self
            .workdir()
            .map(|workdir| (workdir, self.claim_diff_epoch()));
        let s = Arc::clone(self);
        async move {
            match started {
                None => DiffReadOutcome::Unavailable,
                Some((workdir, epoch)) => s.publish_diff(workdir, target, None, epoch).await,
            }
        }
    }

    /// Re-reads the diff the pane is holding and answers only if the bytes
    /// under it moved.
    ///
    /// Run by the poll while a working-tree file is open — the only thing
    /// that notices a file changed outside this window with its status
    /// unchanged (a conflict resolved in another tool keeps the same two
    /// stage letters). A tick that finds nothing moved costs one process:
    /// the raw diff and its fingerprint.
    pub fn refresh_diff(self: &Arc<Self>, target: DiffTarget) {
        drop(self.refresh_diff_tracked(target));
    }

    /// The same re-read with its completion boundary, for tests and for
    /// callers that have to tell "nothing moved" from "not finished".
    pub fn refresh_diff_tracked(self: &Arc<Self>, target: DiffTarget) -> DiffRefreshTask {
        let Some(workdir) = self.workdir() else {
            return DiffRefreshTask::ready(DiffReadOutcome::Unavailable);
        };
        self.start_refresh_diff(workdir, target)
    }

    /// The same re-read aimed at another working copy, for the read-only
    /// pane's own tick ([`Self::load_carried_diff`]).
    pub fn refresh_carried_diff(self: &Arc<Self>, at: String, target: DiffTarget) {
        drop(self.start_refresh_diff(PathBuf::from(at), target));
    }

    fn start_refresh_diff(
        self: &Arc<Self>,
        workdir: PathBuf,
        target: DiffTarget,
    ) -> DiffRefreshTask {
        let s = Arc::clone(self);
        let (finished, task) = DiffRefreshTask::pending();
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            let outcome = match details::file_diff_raw(&s.executor, &workdir, &target, &cancel)
                .await
            {
                Err(e) => {
                    s.fail("diff", e);
                    DiffReadOutcome::Failed
                }
                Ok(raw) if s.diff_seen(&workdir, &target) == Some(details::fingerprint(&raw)) => {
                    DiffReadOutcome::Unchanged
                }
                Ok(raw) => {
                    // Claimed only now: a tick that found nothing moved
                    // leaves the epoch to a click's read in flight.
                    let epoch = s.claim_diff_epoch();
                    s.publish_diff(workdir, target, Some(raw), epoch).await
                }
            };
            if finished.send(outcome).is_err() {
                tracing::trace!("diff re-read completion was not observed");
            }
        });
        task
    }

    /// Sends one diff and starts the colours behind it, answering how it
    /// ended. `known` is the raw diff a caller already read, so a re-read
    /// spends no second process on the same bytes.
    ///
    /// **A read the pane has passed publishes nothing**: two reads of one
    /// file overlap wherever a write answers and its status lands, and
    /// need not finish in order. The older landing last would leave the
    /// pane a fingerprint of bytes no longer there, and the next partial
    /// stage is refused against it (`stage::refusal::verify_fingerprint`).
    async fn publish_diff(
        self: &Arc<Self>,
        workdir: PathBuf,
        target: DiffTarget,
        known: Option<Vec<u8>>,
        epoch: u64,
    ) -> DiffReadOutcome {
        let cancel = self.root_cancel.clone();
        // Line-ending context is read **beside** the diff: a notice that
        // turns up later is one the reader has already scrolled past. So
        // is the source the colours are read against (`highlight`), but
        // only for a language something can be said about.
        let wants_source = crate::highlight::knows(preview::target_path(&target));
        let (diff, endings, source, embedded) = tokio::join!(
            async {
                match known {
                    Some(raw) => Ok(raw),
                    None => {
                        details::file_diff_raw(&self.executor, &workdir, &target, &cancel).await
                    }
                }
            },
            self.ending_context(&workdir, &target, &cancel),
            async {
                match wants_source {
                    true => preview::source_text(&self.executor, &workdir, &target, &cancel).await,
                    false => None,
                }
            },
            // A directory git would not open shows, in its patch's place,
            // the commit a stage of it would point at (`details::embedded`).
            async {
                match &target {
                    DiffTarget::Untracked { path } if path.ends_with('/') => {
                        details::embedded(&self.executor, &workdir, path, &cancel).await
                    }
                    _ => None,
                }
            },
        );
        match diff {
            Ok(raw) => {
                // Passed while reading: everything below (a picture written
                // to disk, a colouring walk) would be spent for nobody.
                if !self.diff_is_current(epoch) {
                    return DiffReadOutcome::Overtaken;
                }
                let patches = crate::parse::diff::parse_patch(&raw);
                let fingerprint = details::fingerprint(&raw);
                let endings = match endings {
                    EndingContext::Excluded => None,
                    EndingContext::Open(baseline) => {
                        eol::settle(eol::read_one(&raw), baseline.as_ref())
                    }
                };
                let patches = Arc::new(patches);
                let marks = Arc::new(crate::intraline::marks(&patches));
                let is_binary = patches.iter().any(|p| p.is_binary);
                let preview = preview::file_preview(
                    &self.executor,
                    &workdir,
                    &target,
                    is_binary,
                    self.preview_files.read(epoch),
                    &cancel,
                )
                .await;
                // Asked again: the pane can move on during the picture read.
                if !self.diff_is_current(epoch) {
                    // This read's files go too: nothing will name them, and
                    // the read that passed this one sweeps only below
                    // itself, possibly already (`preview::PreviewFiles`).
                    self.preview_files.sweep_before(epoch + 1);
                    return DiffReadOutcome::Overtaken;
                }
                // Earlier reads' picture files go now, whether or not this
                // one wrote any: what the pane still shows of them it has
                // already decoded (`preview::PreviewFiles`).
                self.preview_files.sweep_before(epoch);
                // Noted before the event: what the next re-read compares
                // against is what the pane was handed.
                self.note_diff(&workdir, &target, fingerprint);
                self.sink.event(SessionEvent::DiffLoaded {
                    target: target.clone(),
                    patches: Arc::clone(&patches),
                    preview,
                    fingerprint,
                    endings,
                    marks,
                    embedded,
                });
                self.paint_diff(target, patches, source, epoch);
                DiffReadOutcome::Sent
            }
            Err(e) => {
                self.fail("diff", e);
                DiffReadOutcome::Failed
            }
        }
    }

    /// Takes the next diff epoch (see [`RepoSession::diff_epoch`]).
    fn claim_diff_epoch(&self) -> u64 {
        self.diff_epoch.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// The fingerprint the pane was last handed for `target` in `workdir`,
    /// or `None` when what it holds is a diff of something else — another
    /// file, or the same path in another working copy.
    fn diff_seen(&self, workdir: &Path, target: &DiffTarget) -> Option<u64> {
        let slot = relock(&self.last_diff);
        slot.as_ref()
            .filter(|(from, held, _)| from == workdir && held == target)
            .map(|(_, _, fingerprint)| *fingerprint)
    }

    fn note_diff(&self, workdir: &Path, target: &DiffTarget, fingerprint: u64) {
        let mut slot = relock(&self.last_diff);
        *slot = Some((workdir.to_path_buf(), target.clone(), fingerprint));
    }

    /// Works out the colours for a diff already on its way to the pane and
    /// sends them after it ([`SessionEvent::DiffColoured`]).
    ///
    /// The one part of reading a diff that computes, so it runs on a
    /// blocking thread and is skipped once the reader has moved on (else a
    /// walk down a file list would leave a colouring per row running). A panic
    /// here costs the file its colours and nothing else — the rows are
    /// already out.
    fn paint_diff(
        self: &Arc<Self>,
        target: DiffTarget,
        patches: Arc<Vec<FilePatch>>,
        source: Option<String>,
        epoch: u64,
    ) {
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            if !s.diff_is_current(epoch) {
                return;
            }
            // Where the full reading is slow, a quick one (no file walked)
            // goes out first, marked unsettled: the full answer always
            // follows, so whoever waits for "the colours" waits on
            // `settled`.
            if source.is_some() && crate::highlight::deep(&patches) {
                let p = Arc::clone(&patches);
                match tokio::task::spawn_blocking(move || crate::highlight::colors_quick(&p)).await
                {
                    Ok(colors) => {
                        if !s.diff_is_current(epoch) {
                            return;
                        }
                        s.sink.event(SessionEvent::DiffColoured {
                            target: target.clone(),
                            colors,
                            settled: false,
                        });
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "quick colours failed; waiting for the full read");
                    }
                }
            }
            // The lexer states the last reading of this file left behind
            // (`RepoSession::lex_cache`) ride along and come back grown.
            let cache = relock(&s.lex_cache).take();
            let (colors, cache) = match tokio::task::spawn_blocking(move || {
                crate::highlight::colors_cached(&patches, source.as_deref(), cache)
            })
            .await
            {
                Ok(pair) => pair,
                Err(e) => {
                    tracing::warn!(error = %e, "syntax colours failed; the diff stays plain");
                    return;
                }
            };
            // Kept even when the answer is no longer wanted (the cache
            // tells a stale source apart itself), but not past the close
            // (`RepoSession::keeps_what_it_reads`).
            {
                let mut slot = relock(&s.lex_cache);
                if s.keeps_what_it_reads() {
                    *slot = cache;
                }
            }
            // Asked again: a long colouring can be overtaken.
            if !s.diff_is_current(epoch) {
                return;
            }
            s.sink.event(SessionEvent::DiffColoured {
                target,
                colors,
                settled: true,
            });
        });
    }

    /// Whether the diff read that claimed `epoch` is still the latest one.
    fn diff_is_current(&self, epoch: u64) -> bool {
        self.diff_epoch.load(std::sync::atomic::Ordering::SeqCst) == epoch
    }
}
