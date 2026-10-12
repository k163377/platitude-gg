//! The git runs one owner has going, counted from the spawn to the run's
//! end (`super::family`) — what an owner waits on before it touches a
//! folder those runs stand in (`RepoSession::stand_down`).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A count of running processes, shared by every handle an owner gives
/// out ([`super::GitExecutor::counted`]). A command waiting for a slot is
/// not running: a cancelled one ends there without spawning. One is counted
/// from just before its spawn, and the token is read again once it is: an
/// owner that cancels and then waits for none ([`Self::idle`]) is past every
/// git it had, and spawns none after.
#[derive(Debug)]
pub struct Live {
    running: tokio::sync::watch::Sender<usize>,
    /// Runs let go with their end not told, since a wait last answered for
    /// them ([`Self::idle`]).
    untold: AtomicUsize,
}

/// Runs an owner let go with their end not told: git, or what it started
/// for the run, may still stand where it ran. Counted down like the rest,
/// so a wait for none still answers — with this, not with a clean end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{count} git run(s) let go with their end not told")]
pub struct UntoldEnds {
    pub count: usize,
}

impl Default for Live {
    fn default() -> Self {
        Self {
            running: tokio::sync::watch::channel(0).0,
            untold: AtomicUsize::new(0),
        }
    }
}

impl Live {
    /// One more running until the guard goes, taken just before the spawn.
    pub(super) fn enter(self: &Arc<Self>) -> Running {
        self.running.send_modify(|running| *running += 1);
        Running {
            live: Arc::clone(self),
            told: true,
        }
    }

    /// How many are running now.
    #[must_use]
    pub fn count(&self) -> usize {
        *self.running.borrow()
    }

    /// Waits until none is running. Says nothing of what starts after: the
    /// owner shuts its own door first.
    ///
    /// # Errors
    ///
    /// The runs let go meanwhile with their end not told, named once: to
    /// the wait that answers next.
    pub async fn idle(&self) -> Result<(), UntoldEnds> {
        let mut running = self.running.subscribe();
        // The sender is `self`, so it outlives the wait.
        let _ = running.wait_for(|running| *running == 0).await;
        match self.untold.swap(0, Ordering::SeqCst) {
            0 => Ok(()),
            count => Err(UntoldEnds { count }),
        }
    }
}

/// One run counted as going; dropped once the run is gone — git reaped,
/// and on Windows a stopped run's processes ended (`super::family`) — or
/// once its spawn is called off or fails.
pub(super) struct Running {
    live: Arc<Live>,
    told: bool,
}

impl Running {
    /// The run is let go with its end not told: the wait that answers next
    /// says so ([`Live::idle`]).
    pub(super) fn untold(&mut self) {
        self.told = false;
    }

    /// The end was seen after all (`super::family`'s dropped runs).
    #[cfg_attr(
        not(windows),
        expect(dead_code, reason = "only Windows waits a dropped run out")
    )]
    pub(super) fn told(&mut self) {
        self.told = true;
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        // Before the count comes down, so the wait it wakes reads it.
        if !self.told {
            self.live.untold.fetch_add(1, Ordering::SeqCst);
        }
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
        assert_eq!(live.idle().await, Ok(()));
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
        assert_eq!(waiting.await, Ok(()));
        assert_eq!(live.count(), 0);
    }

    /// A run let go untold still comes off the count — the wait answers —
    /// but the answer says so, once.
    #[tokio::test]
    async fn a_run_let_go_untold_is_named_to_the_next_wait() {
        let live = Arc::new(Live::default());
        let told = live.enter();
        let mut untold = live.enter();
        untold.untold();
        drop(untold);
        let waiting = live.idle();
        tokio::pin!(waiting);
        assert!(
            crate::wait::poll_once(&mut waiting).is_pending(),
            "the told one is still running"
        );
        drop(told);
        assert_eq!(waiting.await, Err(UntoldEnds { count: 1 }));
        assert_eq!(live.idle().await, Ok(()), "named once");
    }
}
