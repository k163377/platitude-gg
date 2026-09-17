//! The commit details read: one at a time, and numbered
//! ([`Latest::begin_numbered`]).
use super::*;

// Feeds can outlive a closed/reopened session. A new session's request
// outranks whatever an old sink carries into the same consumer.
static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);

/// The task has finished, including delivery to its sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailsOutcome {
    Sent,
    Failed,
    Cancelled,
}

/// Its generation is assigned synchronously, before the worker can run.
pub struct DetailsTask {
    generation: u64,
    done: tokio::sync::oneshot::Receiver<DetailsOutcome>,
}

impl DetailsTask {
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub async fn outcome(self) -> DetailsOutcome {
        self.done.await.unwrap_or(DetailsOutcome::Cancelled)
    }
}

fn completed(sender: tokio::sync::oneshot::Sender<DetailsOutcome>, outcome: DetailsOutcome) {
    if sender.send(outcome).is_err() {
        tracing::trace!("details completion was not observed");
    }
}

/// What a choice of several commits asks the right pane to show
/// (デザイン規約 §複数のコミットを選ぶ).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionRead {
    /// Exactly two commits: what differs between them.
    Compare,
    /// Three or more: what all of them changed, merged into one list.
    Union,
}

impl RepoSession {
    /// Cancels the previous right-pane read and asks what this choice of
    /// commits changed.
    ///
    /// **The same slot the one-commit read uses.** The two answer the
    /// same pane, so a choice has to cancel a commit read and a commit
    /// read has to cancel a choice — two slots would let the slower of
    /// them land last and describe something nobody is pointing at.
    /// `oids` come newest first, the order the graph stands in.
    pub fn load_selection(
        self: &Arc<Self>,
        oids: Vec<Oid>,
        mode: SelectionRead,
    ) -> Option<DetailsTask> {
        let workdir = self.workdir()?;
        let (generation, cancel) = self
            .details_read
            .begin_numbered(&self.root_cancel, &NEXT_REQUEST);
        let (send, done) = tokio::sync::oneshot::channel();
        let task = DetailsTask { generation, done };
        let session = Arc::clone(self);
        self.runtime.spawn(async move {
            let outcome = session
                .read_selection(&workdir, &oids, mode, generation, &cancel)
                .await;
            completed(send, outcome);
        });
        Some(task)
    }

    async fn read_selection(
        &self,
        workdir: &Path,
        oids: &[Oid],
        mode: SelectionRead,
        generation: u64,
        cancel: &CancellationToken,
    ) -> DetailsOutcome {
        if cancel.is_cancelled() {
            return DetailsOutcome::Cancelled;
        }
        let result = match mode {
            // Oldest side first: the choice arrives newest first, and a
            // comparison is read from the older of the two.
            SelectionRead::Compare => match (oids.last(), oids.first()) {
                (Some(from), Some(to)) => {
                    details::compare_files(&self.executor, workdir, from, to, cancel).await
                }
                _ => Ok(Vec::new()),
            },
            SelectionRead::Union => {
                details::union_files(&self.executor, workdir, oids, cancel).await
            }
        };
        if cancel.is_cancelled() {
            return DetailsOutcome::Cancelled;
        }
        match result {
            Ok(files) => {
                self.sink
                    .event(SessionEvent::SelectionLoaded { generation, files });
                DetailsOutcome::Sent
            }
            Err(error) => {
                self.sink.event(SessionEvent::DetailsFailed {
                    generation,
                    // Named by the newest of the choice, which is what
                    // the reader last pressed. An empty choice reads
                    // nothing and so cannot fail.
                    oid: oids.first().copied().unwrap_or_else(Oid::zero_unsized),
                    error,
                });
                DetailsOutcome::Failed
            }
        }
    }

    /// Cancels the previous details read and requests this commit.
    /// Consumers must retain the generation through queuing: cancellation
    /// can race with a result that has already entered the sink.
    pub fn load_details(self: &Arc<Self>, oid: Oid) -> Option<DetailsTask> {
        let workdir = self.workdir()?;
        let (generation, cancel) = self
            .details_read
            .begin_numbered(&self.root_cancel, &NEXT_REQUEST);
        let (send, done) = tokio::sync::oneshot::channel();
        let task = DetailsTask { generation, done };
        let session = Arc::clone(self);
        self.runtime.spawn(async move {
            let outcome = session
                .read_details(&workdir, oid, generation, &cancel)
                .await;
            completed(send, outcome);
        });
        Some(task)
    }

    async fn read_details(
        &self,
        workdir: &Path,
        oid: Oid,
        generation: u64,
        cancel: &CancellationToken,
    ) -> DetailsOutcome {
        if cancel.is_cancelled() {
            return DetailsOutcome::Cancelled;
        }
        let result = details::commit_details(&self.executor, workdir, &oid, cancel).await;
        if cancel.is_cancelled() {
            return DetailsOutcome::Cancelled;
        }
        match result {
            Ok(details) => {
                self.sink.event(SessionEvent::DetailsLoaded {
                    generation,
                    details,
                });
                DetailsOutcome::Sent
            }
            Err(error) => {
                // The error travels with the generation too. A global OpFailed
                // here could surface an old failure after a newer selection.
                self.sink.event(SessionEvent::DetailsFailed {
                    generation,
                    oid,
                    error,
                });
                DetailsOutcome::Failed
            }
        }
    }
}
