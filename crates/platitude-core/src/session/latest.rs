//! Latest-request ownership for the reads that answer one question at a
//! time: a new ask cancels the one still out.
//!
//! Consumers that pick their answer by number (details, rebase plan) use
//! [`Latest::begin_numbered`]. Reads addressed by what they asked about
//! already tell a late answer apart and use [`Latest::begin`], which only
//! stops spending a process on it.

use super::*;

#[derive(Default)]
pub(super) struct Latest {
    cancel: Mutex<Option<CancellationToken>>,
}

impl Latest {
    /// A token for the ask about to go out, cancelling the one before it.
    /// Call before spawning the task, so the order is the order the asks
    /// were made.
    pub(super) fn begin(&self, parent: &CancellationToken) -> CancellationToken {
        let cancel = parent.child_token();
        if let Some(previous) = relock(&self.cancel).replace(cancel.clone()) {
            previous.cancel();
        }
        cancel
    }

    /// [`Self::begin`] with a number off `counter`, taken under the slot's
    /// lock so of two racing asks the larger number is the one left
    /// running — else the consumer picking by number waits on a cancelled
    /// answer.
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
