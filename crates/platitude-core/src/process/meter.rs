//! What a stretch of work spent in git: the lives of the children it
//! spawned, summed — each from the spawn to the run's end (`family`), the
//! slot wait left out.
//!
//! A proxy for how heavy the work was, not a CPU reading: a child's life
//! holds its disk waits and the time the machine kept it off a core, and
//! a `status` that stats on several threads spends more CPU than it
//! lives. It is what repeating work is paced by (`session::pace`): when
//! the children live longer, for whatever reason, the work comes round
//! less often.
//!
//! The work is the future [`Meter::over`] drives and the children spawned
//! from inside it. A task it spawns is outside — a task-local does not
//! cross into another task — unless the task's future is [`carried`] in.

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

tokio::task_local! {
    static METER: Arc<Meter>;
}

/// The sum of the children's lives over one stretch of work.
#[derive(Debug, Default)]
pub struct Meter {
    nanos: AtomicU64,
    children: AtomicU32,
}

impl Meter {
    /// Drives `work`, charging here every child spawned from inside it.
    pub async fn over<F: Future>(self: &Arc<Self>, work: F) -> F::Output {
        METER.scope(Arc::clone(self), work).await
    }

    /// The children's lives so far, summed.
    #[must_use]
    pub fn spent(&self) -> Duration {
        Duration::from_nanos(self.nanos.load(Ordering::SeqCst))
    }

    /// How many children were charged — none means the work was answered
    /// without a process of its own (another read's answer, a skip).
    #[must_use]
    pub fn children(&self) -> u32 {
        self.children.load(Ordering::SeqCst)
    }

    /// What the work weighed, where it spawned anything of its own — `None`
    /// says nothing about how heavy such work is.
    #[must_use]
    pub fn weighed(&self) -> Option<Duration> {
        (self.children() > 0).then(|| self.spent())
    }

    fn charge(&self, lived: Duration, child: bool) {
        let nanos = u64::try_from(lived.as_nanos()).unwrap_or(u64::MAX);
        self.nanos.fetch_add(nanos, Ordering::SeqCst);
        if child {
            self.children.fetch_add(1, Ordering::SeqCst);
        }
    }
}

/// `work`, charged to the metered work this is called from, if any — for a
/// task that work spawns and owns. A child it spawns is charged as its run
/// ends, or as it is told silent ([`charge_so_far`]).
pub(crate) fn carried<F: Future>(work: F) -> impl Future<Output = F::Output> {
    let meter = METER.try_with(Arc::clone).ok();
    async move {
        match meter {
            Some(meter) => meter.over(work).await,
            None => work.await,
        }
    }
}

/// Charges one child's life to the work it was spawned from, if any is
/// being metered — less what [`charge_so_far`] charged of it already.
pub(super) fn charge(lived: Duration, so_far: Option<Duration>) {
    let rest = lived.saturating_sub(so_far.unwrap_or_default());
    if METER
        .try_with(|meter| meter.charge(rest, so_far.is_none()))
        .is_err()
    {
        tracing::trace!("git child outside any metered work");
    }
}

/// Charges a child still running the life it has lived so far, counting
/// it — for a run told silent, whose waiter goes on and weighs its work
/// before the run ends (`session::walk_silence`). Its end charges the rest
/// ([`charge`]).
pub(super) fn charge_so_far(lived: Duration) {
    if METER.try_with(|meter| meter.charge(lived, true)).is_err() {
        tracing::trace!("git child outside any metered work");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::{GitCommand, GitExecutor};
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn the_children_of_the_work_are_charged_and_nothing_else() {
        let executor = GitExecutor::new(crate::process::path_git());
        let cancel = CancellationToken::new();
        let meter = Arc::new(Meter::default());
        meter
            .over(async {
                for _ in 0..2 {
                    executor
                        .run(GitCommand::new().args(["--version"]), &cancel)
                        .await
                        .expect("git answers its version");
                }
            })
            .await;
        assert_eq!(meter.children(), 2, "both children were charged");
        assert!(meter.spent() > Duration::ZERO, "and their lives counted");

        let spent = meter.spent();
        executor
            .run(GitCommand::new().args(["--version"]), &cancel)
            .await
            .expect("git answers again");
        assert_eq!(meter.children(), 2, "a child outside the work is not");
        assert_eq!(meter.spent(), spent);
    }

    #[tokio::test]
    async fn a_task_spawned_from_the_work_is_outside_it() {
        let executor = GitExecutor::new(crate::process::path_git());
        let meter = Arc::new(Meter::default());
        meter
            .over(async {
                let executor = executor.clone();
                tokio::spawn(async move {
                    let cancel = CancellationToken::new();
                    executor
                        .run(GitCommand::new().args(["--version"]), &cancel)
                        .await
                        .expect("git answers its version");
                })
                .await
                .expect("the spawned read");
            })
            .await;
        assert_eq!(meter.children(), 0);
    }

    /// A run told silent is charged as far as it has lived at the telling —
    /// a waiter that goes on then weighs it — and only the rest at its end,
    /// counted once. A budget of nothing tells before git can have ended.
    #[tokio::test]
    async fn a_run_told_silent_is_charged_at_the_telling_and_once() {
        let executor = GitExecutor::new(crate::process::path_git());
        let cancel = CancellationToken::new();
        let meter = Arc::new(Meter::default());
        let at_the_telling = std::sync::Mutex::new(None);
        let tell = || {
            *at_the_telling.lock().unwrap() = Some((meter.children(), meter.spent()));
        };
        meter
            .over(
                executor.run_streaming_watched(
                    GitCommand::new()
                        .args(["--version"])
                        .silent_after(Duration::ZERO),
                    &cancel,
                    &mut |_| {},
                    Some(&tell),
                ),
            )
            .await
            .expect("git answers its version");
        let (children, spent) = at_the_telling
            .into_inner()
            .unwrap()
            .expect("told before the run ended");
        assert_eq!(children, 1, "counted at the telling");
        assert!(spent > Duration::ZERO, "and charged its life so far");
        assert_eq!(meter.children(), 1, "counted once");
        assert!(meter.spent() >= spent, "the rest added at the end");
    }

    #[tokio::test]
    async fn a_task_spawned_carried_is_inside_it() {
        let executor = GitExecutor::new(crate::process::path_git());
        let meter = Arc::new(Meter::default());
        meter
            .over(async {
                let executor = executor.clone();
                tokio::spawn(carried(async move {
                    let cancel = CancellationToken::new();
                    executor
                        .run(GitCommand::new().args(["--version"]), &cancel)
                        .await
                        .expect("git answers its version");
                }))
                .await
                .expect("the spawned read");
            })
            .await;
        assert_eq!(meter.children(), 1);
    }
}
