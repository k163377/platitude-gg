//! On-demand reads answered as events: publish state, history reach,
//! signatures, commit details and file diffs.

use super::*;
use crate::eol;

/// What one read of the diff on screen established.
///
/// A completion boundary rather than a convenience, because the ordinary
/// answer is silence: a file nobody has touched sends no event at all, a
/// read the pane has passed sends none either, and waiting cannot tell
/// "none yet" from "none coming" (core.md §非同期・並行テスト).
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

    /// Waits for the re-read itself to finish. A dropped runtime is the
    /// same observable result as cancellation: no later answer from this
    /// read can arrive.
    pub async fn outcome(self) -> DiffReadOutcome {
        self.0.await.unwrap_or(DiffReadOutcome::Cancelled)
    }
}

impl RepoSession {
    /// Asks again how many commits of a plan's range a remote already has
    /// — the count the plan's preview arrived with
    /// (`rebase_plan::PlanPreview::published`), re-read after the refs
    /// moved under the open plan. A read, not a write; the answer names
    /// the range, so a plan that has since been put away drops it.
    ///
    /// A range a plan asked about always starts from HEAD and HEAD has
    /// commits, so — unlike the one-commit question the walk answers off
    /// its rows — there is no unborn case to keep off git here.
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
    /// Kept out of [`Self::load_details`] on purpose: verifying runs gpg or
    /// ssh-keygen, and the details pane has a 100ms budget. The answer
    /// arrives on its own, after the commit is already on screen.
    ///
    /// One at a time: a selection that moves on cancels the verification
    /// the last one started rather than letting two race — the consumer
    /// already drops the answer about the row left behind, and this
    /// spares it the process (`session::latest`).
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

    /// The same read as a future the caller drives, which is both its
    /// completion boundary and the only way to hold one open: the read
    /// this returns is the pane's newest from the moment it is asked for,
    /// so a test can stop it inside its git and let the next one pass it
    /// (core.md §非同期・並行テスト).
    pub fn read_diff(
        self: &Arc<Self>,
        target: DiffTarget,
    ) -> impl Future<Output = DiffReadOutcome> + Send + 'static + use<> {
        // Claimed here rather than inside the read, so that asking for a
        // read is what passes the one before it, whenever either of them
        // runs — and only where there is something to read, so a repository
        // nobody has opened passes nothing (see `diff_epoch`).
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
    /// What the poll runs while a working-tree file is open, and the only
    /// thing that notices a file changed outside this window without also
    /// changing its status. A conflict is the shape that shows it: git
    /// reports the same two stage letters whether or not the markers are
    /// still in the file, so a conflict resolved in another tool left the
    /// pane drawing the conflict it was opened on.
    ///
    /// One process on a tick that finds nothing moved — the raw diff, and
    /// the fingerprint that says it is the one already on screen. Only a
    /// file that really moved pays for the rest of a read.
    pub fn refresh_diff(self: &Arc<Self>, target: DiffTarget) {
        drop(self.start_refresh_diff(target));
    }

    /// The same re-read with its completion boundary, for tests and for
    /// callers that have to tell "nothing moved" from "not finished".
    pub fn refresh_diff_tracked(self: &Arc<Self>, target: DiffTarget) -> DiffRefreshTask {
        self.start_refresh_diff(target)
    }

    fn start_refresh_diff(self: &Arc<Self>, target: DiffTarget) -> DiffRefreshTask {
        let Some(workdir) = self.workdir() else {
            return DiffRefreshTask::ready(DiffReadOutcome::Unavailable);
        };
        let s = Arc::clone(self);
        let (finished, task) = DiffRefreshTask::pending();
        self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            let outcome =
                match details::file_diff_raw(&s.executor, &workdir, &target, &cancel).await {
                    Err(e) => {
                        s.fail("diff", e);
                        DiffReadOutcome::Failed
                    }
                    Ok(raw) if s.diff_seen(&target) == Some(details::fingerprint(&raw)) => {
                        DiffReadOutcome::Unchanged
                    }
                    Ok(raw) => {
                        // Claimed only now, and not before the read above: a
                        // tick that found the file where it left it must not
                        // take the epoch from the read a click has in flight
                        // (see `diff_epoch`).
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
    /// ended. `known` is the raw diff a caller has already read, so a
    /// re-read that found it moved spends no second process on the same
    /// bytes.
    ///
    /// **A read the pane has passed publishes nothing.** Two reads of one
    /// file overlap wherever a write answers and the status behind it
    /// lands — both ask the pane to re-read what it holds — and they need
    /// not finish in the order they were asked. The older landing last
    /// would leave the pane holding rows and a fingerprint taken from
    /// bytes that are no longer there, and the next partial stage is
    /// refused against exactly that fingerprint
    /// (`stage::refusal::verify_fingerprint`).
    async fn publish_diff(
        self: &Arc<Self>,
        workdir: PathBuf,
        target: DiffTarget,
        known: Option<Vec<u8>>,
        epoch: u64,
    ) -> DiffReadOutcome {
        let cancel = self.root_cancel.clone();
        // What git's settings say, and the neighbours if they are the
        // only answer, are read **beside** the diff rather than after
        // it: a notice that turns up a moment later is one the reader
        // has already scrolled past.
        // The file the diff is of, fetched beside it rather than
        // after: the colours are read against it (`highlight`), and a
        // second round trip would land after the rows are on screen.
        // Only for a language something can be said about — otherwise
        // it is a process spent on a file nobody will colour.
        let wants_source = crate::highlight::knows(preview::target_path(&target));
        let (diff, endings, source) = tokio::join!(
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
        );
        match diff {
            Ok(raw) => {
                // Passed while the bytes were being read: everything below
                // is spent for a pane that has moved — a picture written
                // to disk, a file walked for its colours — and none of it
                // is the read the pane is waiting for.
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
                // Asked again, because the picture above is read out of
                // git and the pane can be moved on across it.
                if !self.diff_is_current(epoch) {
                    // This read's own files go with the ones before it:
                    // nothing will ever name them, and the read that
                    // passed this one sweeps only below itself — which
                    // it may already have done (`preview::PreviewFiles`).
                    self.preview_files.sweep_before(epoch + 1);
                    return DiffReadOutcome::Overtaken;
                }
                // The picture files of the reads before this one go now,
                // whether or not this one wrote any: the pane is about
                // to be handed this read, and what it still shows of the
                // last it has already decoded (`preview::PreviewFiles`).
                self.preview_files.sweep_before(epoch);
                // Noted before the event and not after it: what the next
                // re-read compares against is what the pane was handed.
                self.note_diff(&target, fingerprint);
                self.sink.event(SessionEvent::DiffLoaded {
                    target: target.clone(),
                    patches: Arc::clone(&patches),
                    preview,
                    fingerprint,
                    endings,
                    marks,
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

    /// The fingerprint the pane was last handed for `target`, or `None`
    /// when what it holds is a diff of something else.
    fn diff_seen(&self, target: &DiffTarget) -> Option<u64> {
        let slot = relock(&self.last_diff);
        slot.as_ref()
            .filter(|(held, _)| held == target)
            .map(|(_, fingerprint)| *fingerprint)
    }

    fn note_diff(&self, target: &DiffTarget, fingerprint: u64) {
        let mut slot = relock(&self.last_diff);
        *slot = Some((target.clone(), fingerprint));
    }

    /// Works out the colours for a diff already on its way to the pane and
    /// sends them after it ([`SessionEvent::DiffColoured`]).
    ///
    /// Two things keep this off the reader's path. It is the one part of
    /// reading a diff that computes rather than waits, so it goes to a
    /// blocking thread rather than holding a runtime worker for as long as
    /// a large file takes; and it is skipped outright once the reader has
    /// moved on, which is what stops a walk down a commit's file list from
    /// leaving a colouring per row running behind it.
    ///
    /// It is also somebody else's code. If it goes down, the diff does not
    /// go with it — the rows are already gone out, and a panic here costs
    /// the file its colours and nothing else.
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
            // Where the full reading will keep the reader waiting, a
            // quick one goes out first: no file walked, a fraction of
            // the budget, on screen in tens of milliseconds. Marked
            // unsettled — the full answer follows it whatever it says,
            // so whoever waits for "the colours" has a true to wait on.
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
            // Kept even where the answer is not wanted any more: the
            // states are about the file, not about who asked, and the
            // cache tells a stale source apart on its own.
            *relock(&s.lex_cache) = cache;
            // Asked again: a long colouring can be overtaken while it runs,
            // and the pane would drop the answer anyway.
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
