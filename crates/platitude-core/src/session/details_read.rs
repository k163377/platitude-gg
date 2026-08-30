//! Latest-request ownership for commit details.
use super::*;

// Feeds can outlive a closed/reopened session. An old sink must never carry
// a larger identity than a new session's request into the same consumer.
static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);

#[derive(Default)]
pub(super) struct DetailsRead {
    cancel: Option<CancellationToken>,
}

impl DetailsRead {
    fn begin(&mut self, parent: &CancellationToken) -> (u64, CancellationToken) {
        let generation = NEXT_REQUEST.fetch_add(1, Ordering::Relaxed);
        let cancel = parent.child_token();
        if let Some(previous) = self.cancel.replace(cancel.clone()) {
            previous.cancel();
        }
        (generation, cancel)
    }
}

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

impl RepoSession {
    /// Cancels the previous details read and requests this commit.
    /// Consumers must retain the generation through queuing: cancellation
    /// can race with a result that has already entered the sink.
    pub fn load_details(self: &Arc<Self>, oid: Oid) -> Option<DetailsTask> {
        let workdir = self.workdir()?;
        let (generation, cancel) = relock(&self.details_read).begin(&self.root_cancel);
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
