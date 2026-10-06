//! The pane's diff reads: which one is the newest, the token its git runs
//! under, and the re-reads a poll tick or a window coming back asks for.

use super::*;

/// Which diff read is the newest ([`RepoSession::diff_epoch`]), and the
/// token its git runs under.
///
/// A read takes the next number and cancels the read it passes, git and
/// all: a walk down a file list leaves no diff, ruling or source read
/// running for the rows it passed. Number and token change under one lock,
/// so of two racing asks the larger number is the one left running.
#[derive(Default)]
pub(super) struct DiffEpoch(Mutex<Newest>);

#[derive(Default)]
struct Newest {
    epoch: u64,
    /// The newest read's token; `None` before the first read.
    cancel: Option<CancellationToken>,
}

impl DiffEpoch {
    /// The next number, for a read about to go out, and its token.
    pub(super) fn claim(&self, parent: &CancellationToken) -> (u64, CancellationToken) {
        take(&mut relock(&self.0), parent)
    }

    /// [`Self::claim`], only where no read was asked for since `seen`: a
    /// re-read that finds the file moved must not pass a click made while
    /// it read — the click's read is the newer ask.
    pub(super) fn claim_after(
        &self,
        seen: u64,
        parent: &CancellationToken,
    ) -> Option<(u64, CancellationToken)> {
        let mut newest = relock(&self.0);
        (newest.epoch == seen).then(|| take(&mut newest, parent))
    }

    /// The newest number, and a token the next claim cancels — what a
    /// re-read looks under, so a click stops it.
    pub(super) fn watch(&self, parent: &CancellationToken) -> (u64, CancellationToken) {
        let newest = relock(&self.0);
        let under = newest.cancel.as_ref().unwrap_or(parent);
        (newest.epoch, under.child_token())
    }

    pub(super) fn is_current(&self, epoch: u64) -> bool {
        relock(&self.0).epoch == epoch
    }
}

fn take(newest: &mut Newest, parent: &CancellationToken) -> (u64, CancellationToken) {
    newest.epoch += 1;
    let cancel = parent.child_token();
    if let Some(passed) = newest.cancel.replace(cancel.clone()) {
        passed.cancel();
    }
    (newest.epoch, cancel)
}

/// The re-reads out, one per file. An ask for a file whose re-read is out
/// is answered by the one after it, which every such ask shares — not by
/// the one out, which may have read the file before the ask's reason (a
/// save just before the window came back), as [`ReadFlight`] has it.
#[derive(Default)]
pub(super) struct Rereads(Mutex<Vec<Flying>>);

struct Flying {
    at: PathBuf,
    target: DiffTarget,
    /// The asks made while this re-read was out.
    owed: Vec<tokio::sync::oneshot::Sender<DiffReadOutcome>>,
    /// The newest diff read any of them had seen asked for.
    seen: u64,
}

/// What an ask for a re-read comes to ([`RepoSession::reread`]).
enum Turn {
    /// None of this file was out: this ask runs it, and any owed behind it.
    Lead(Leading),
    /// One is: answered by the next.
    Owed(tokio::sync::oneshot::Receiver<DiffReadOutcome>),
}

impl Rereads {
    /// Asks for a re-read of the file: the answer to wait on where one is
    /// out, `None` where this ask leads (the file is now held for it).
    fn ask(
        &self,
        at: &Path,
        target: &DiffTarget,
        seen: u64,
    ) -> Option<tokio::sync::oneshot::Receiver<DiffReadOutcome>> {
        let mut flying = relock(&self.0);
        if let Some(out) = flying
            .iter_mut()
            .find(|out| out.at == at && out.target == *target)
        {
            let (send, receive) = tokio::sync::oneshot::channel();
            out.owed.push(send);
            out.seen = out.seen.max(seen);
            return Some(receive);
        }
        flying.push(Flying {
            at: at.to_path_buf(),
            target: target.clone(),
            owed: Vec::new(),
            seen,
        });
        None
    }

    /// The asks the next re-read of the file answers, and the newest read
    /// they saw; `None` lets the file go, the next ask leading afresh.
    fn next(
        &self,
        at: &Path,
        target: &DiffTarget,
    ) -> Option<(Vec<tokio::sync::oneshot::Sender<DiffReadOutcome>>, u64)> {
        let mut flying = relock(&self.0);
        let index = flying
            .iter()
            .position(|out| out.at == at && out.target == *target)?;
        if flying[index].owed.is_empty() {
            flying.swap_remove(index);
            return None;
        }
        let out = &mut flying[index];
        Some((std::mem::take(&mut out.owed), out.seen))
    }

    /// Lets the file go with its owed asks unanswered, which hear
    /// `Cancelled` from the dropped senders.
    fn leave(&self, at: &Path, target: &DiffTarget) {
        relock(&self.0).retain(|out| !(out.at == at && out.target == *target));
    }
}

/// A lead's hold on its file, taken with the ask and let go if the lead
/// ends before [`Rereads::next`] does it — a lead that unwound, was
/// dropped (polled or not), or went down with the runtime leaves no ask
/// waiting for it. Not after: the file may by then be another lead's.
struct Leading {
    session: Arc<RepoSession>,
    at: PathBuf,
    target: DiffTarget,
    let_go: bool,
}

impl Drop for Leading {
    fn drop(&mut self) {
        if !self.let_go {
            self.session.rereads.leave(&self.at, &self.target);
        }
    }
}

impl RepoSession {
    /// Re-reads the diff the pane is holding and answers only if the bytes
    /// under it moved.
    ///
    /// Run by the poll while a working-tree file is open — the only thing
    /// that notices a file changed outside this window with its status
    /// unchanged (a conflict resolved in another tool keeps the same two
    /// stage letters). A tick that finds nothing moved costs one process:
    /// the raw diff and its fingerprint.
    pub fn refresh_diff(self: &Arc<Self>, target: DiffTarget) {
        if let Some(workdir) = self.workdir() {
            self.runtime.spawn(self.reread(workdir, target));
        }
    }

    /// The same re-read as a future the caller drives — its completion
    /// boundary, and the way a test holds one inside its git. Asked when
    /// called: the order of calls is the order of the asks.
    pub fn refresh_diff_tracked(
        self: &Arc<Self>,
        target: DiffTarget,
    ) -> impl Future<Output = DiffReadOutcome> + Send + 'static + use<> {
        let asked = self.workdir().map(|workdir| self.reread(workdir, target));
        async move {
            match asked {
                Some(reread) => reread.await,
                None => DiffReadOutcome::Unavailable,
            }
        }
    }

    /// The same re-read aimed at another working copy, for the read-only
    /// pane's own tick ([`Self::load_carried_diff`]).
    pub fn refresh_carried_diff(self: &Arc<Self>, at: String, target: DiffTarget) {
        self.runtime.spawn(self.reread(PathBuf::from(at), target));
    }

    /// Asks for one re-read of `target` in `workdir`: leading it where none
    /// of that file is out, else owed one by the lead ([`Rereads`]).
    fn reread(
        self: &Arc<Self>,
        workdir: PathBuf,
        target: DiffTarget,
    ) -> impl Future<Output = DiffReadOutcome> + Send + 'static + use<> {
        let (seen, cancel) = self.diff_epoch.watch(&self.root_cancel);
        let turn = match self.rereads.ask(&workdir, &target, seen) {
            Some(answer) => Turn::Owed(answer),
            None => Turn::Lead(Leading {
                session: Arc::clone(self),
                at: workdir,
                target,
                let_go: false,
            }),
        };
        let s = Arc::clone(self);
        async move {
            match turn {
                Turn::Owed(answer) => answer.await.unwrap_or(DiffReadOutcome::Cancelled),
                Turn::Lead(mut leading) => s.lead_rereads(&mut leading, seen, &cancel).await,
            }
        }
    }

    /// The lead's own re-read, then one more for each round of asks made
    /// while one was out, until a round finds none waiting.
    async fn lead_rereads(
        self: &Arc<Self>,
        leading: &mut Leading,
        seen: u64,
        cancel: &CancellationToken,
    ) -> DiffReadOutcome {
        let (workdir, target) = (leading.at.as_path(), &leading.target);
        let (mine, mut claimed) = self.reread_once(workdir, target, seen, cancel).await;
        while let Some((owed, asked_over)) = self.rereads.next(workdir, target) {
            // The flight's own publish is no click: an ask made over the
            // read it passed is an ask over its own.
            let seen = match claimed {
                Some((from, to)) if asked_over == from => to,
                _ => asked_over,
            };
            let (now, cancel) = self.diff_epoch.watch(&self.root_cancel);
            // A click since the asks is newer than all of them.
            let outcome = if now == seen {
                let (outcome, moved) = self.reread_once(workdir, target, seen, &cancel).await;
                claimed = moved.or(claimed);
                outcome
            } else {
                DiffReadOutcome::Overtaken
            };
            for ask in owed {
                if ask.send(outcome).is_err() {
                    tracing::trace!("diff re-read completion was not observed");
                }
            }
        }
        leading.let_go = true;
        mine
    }

    /// One re-read: the raw diff under `cancel` (stopped by the next
    /// click), published only where it moved and no read was asked for
    /// since `seen`. Answers with the epochs it claimed over, if it did.
    async fn reread_once(
        self: &Arc<Self>,
        workdir: &Path,
        target: &DiffTarget,
        seen: u64,
        cancel: &CancellationToken,
    ) -> (DiffReadOutcome, Option<(u64, u64)>) {
        let raw = match details::file_diff_raw(&self.executor, workdir, target, cancel).await {
            Ok(raw) => raw,
            Err(e) if e.is_cancelled() => return (self.passed_or_closing(seen), None),
            Err(e) => {
                self.fail("diff", e);
                return (DiffReadOutcome::Failed, None);
            }
        };
        if self.diff_seen(workdir, target) == Some(details::fingerprint(&raw)) {
            return (DiffReadOutcome::Unchanged, None);
        }
        // Claimed only now, and only over the read it looked under: a tick
        // that found nothing moved leaves the epoch to a click's read in
        // flight, and one that did never passes a click made while it read.
        let Some((epoch, read)) = self.diff_epoch.claim_after(seen, &self.root_cancel) else {
            return (DiffReadOutcome::Overtaken, None);
        };
        let outcome = self
            .publish_diff(
                workdir.to_path_buf(),
                target.clone(),
                Some(raw),
                epoch,
                read,
            )
            .await;
        (outcome, Some((seen, epoch)))
    }

    /// What a read stopped by its token comes to: passed by a newer read,
    /// or — still the newest — stopped by the session closing.
    pub(super) fn passed_or_closing(&self, epoch: u64) -> DiffReadOutcome {
        if self.diff_epoch.is_current(epoch) {
            DiffReadOutcome::Cancelled
        } else {
            DiffReadOutcome::Overtaken
        }
    }
}
