//! Latest-request ownership for the reads that answer one question at a
//! time: a new ask cancels the one still out.
//!
//! Two shapes of consumer. The commit details and the rebase plan pick
//! their answer by a number taken beside the token
//! ([`Latest::begin_numbered`]), because the answer they show is whichever
//! arrived under the largest number. The reads addressed by what they were
//! asked about — the commit whose signature, the branch whose remote
//! reading, the HEAD whose off-window mark — already tell a late answer
//! apart, and all the session has to do is stop spending a process on it
//! ([`Latest::begin`]).

use super::*;

#[derive(Default)]
pub(super) struct Latest {
    cancel: Mutex<Option<CancellationToken>>,
}

impl Latest {
    /// A token for the ask about to go out, with whatever ask held the
    /// slot before it cancelled. Called before the task is spawned, so the
    /// order is the order the asks were made, whichever turn they
    /// are scheduled in.
    pub(super) fn begin(&self, parent: &CancellationToken) -> CancellationToken {
        let cancel = parent.child_token();
        if let Some(previous) = relock(&self.cancel).replace(cancel.clone()) {
            previous.cancel();
        }
        cancel
    }

    /// [`Self::begin`] with a number for the ask off `counter`, taken
    /// under the same lock as the slot: of two asks racing here, the
    /// larger number outlives the smaller, else the consumer picking
    /// by number waits on an answer that was cancelled.
    pub(super) fn begin_numbered(
        &self,
        parent: &CancellationToken,
        counter: &AtomicU64,
    ) -> (u64, CancellationToken) {
        let cancel = parent.child_token();
        let mut slot = relock(&self.cancel);
        let generation = counter.fetch_add(1, Ordering::Relaxed);
        if let Some(previous) = slot.replace(cancel.clone()) {
            previous.cancel();
        }
        (generation, cancel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_ask_cancels_the_one_before_it_and_only_that_one() {
        let root = CancellationToken::new();
        let latest = Latest::default();
        let first = latest.begin(&root);
        let second = latest.begin(&root);
        assert!(first.is_cancelled());
        assert!(!second.is_cancelled());
        assert!(
            !root.is_cancelled(),
            "a child token cancels nothing above it"
        );
        root.cancel();
        assert!(
            second.is_cancelled(),
            "and the session's close still reaches it"
        );
    }
}
