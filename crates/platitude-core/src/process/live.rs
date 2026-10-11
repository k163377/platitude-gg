//! The git processes one owner has running, counted from the spawn to the
//! reap — what an owner waits on before it touches a folder those
//! processes stand in (`RepoSession::stand_down`).

use std::sync::Arc;

/// A count of running processes, shared by every handle an owner gives
/// out ([`super::GitExecutor::counted`]). A command waiting for a slot is
/// not running: a cancelled one ends there without spawning. One is counted
/// from just before its spawn, and the token is read again once it is: an
/// owner that cancels and then waits for none ([`Self::idle`]) is past every
/// git it had, and spawns none after.
#[derive(Debug)]
pub struct Live {
    running: tokio::sync::watch::Sender<usize>,
}

impl Default for Live {
    fn default() -> Self {
        Self {
            running: tokio::sync::watch::channel(0).0,
        }
    }
}

impl Live {
    /// One more running until the guard goes, taken just before the spawn.
    pub(super) fn enter(self: &Arc<Self>) -> Running {
        self.running.send_modify(|running| *running += 1);
        Running {
            live: Arc::clone(self),
        }
    }

    /// How many are running now.
    #[must_use]
    pub fn count(&self) -> usize {
        *self.running.borrow()
    }

    /// Waits until none is running. Says nothing of what starts after: the
    /// owner shuts its own door first.
    pub async fn idle(&self) {
        let mut running = self.running.subscribe();
        // The sender is `self`, so it outlives the wait.
        let _ = running.wait_for(|running| *running == 0).await;
    }
}

/// One process counted as running; dropped once it is reaped, or once its
/// spawn is called off or fails.
pub(super) struct Running {
    live: Arc<Live>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.live
            .running
            .send_modify(|running| *running = running.saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn idle_answers_once_the_last_one_running_has_gone() {
        let live = Arc::new(Live::default());
        live.idle().await;
        let first = live.enter();
        let second = live.enter();
        assert_eq!(live.count(), 2);
        drop(first);
        let waiting = live.idle();
        tokio::pin!(waiting);
        assert!(
            crate::wait::poll_once(&mut waiting).is_pending(),
            "one is still running"
        );
        drop(second);
        waiting.await;
        assert_eq!(live.count(), 0);
    }
}
