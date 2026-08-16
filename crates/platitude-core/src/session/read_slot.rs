//! Single-flight ownership for snapshot readers.

use super::*;

/// One read of a snapshot at a time, with at most one repeat booked
/// behind it.
///
/// Nothing coordinates the places that ask for a re-read — opening a
/// repository asks, and so does the window becoming active a moment
/// later, which at startup is the same moment. Granting both has
/// [`OpGate`] throw the older answer away: two `for-each-ref` and two
/// `status -uall` for one snapshot, which on `JetBrains/kotlin` is about
/// a second of disk work landing exactly where the first click goes.
///
/// The second caller does not start its own read and does not lose its
/// request either — it books the repeat, and the read in flight goes
/// round again when it lands. That distinction is the whole point: a
/// dropped request would leave a write's own refresh reading the
/// repository as it was *before* the write, with the correction waiting
/// on the next poll tick.
pub(super) struct ReadSlot {
    state: std::sync::atomic::AtomicU8,
    changed: tokio::sync::watch::Sender<u64>,
}

/// Nobody is reading.
const SLOT_IDLE: u8 = 0;
/// A read is in flight.
const SLOT_RUNNING: u8 = 1;
/// A read is in flight and somebody asked for another behind it.
const SLOT_AGAIN: u8 = 2;

impl Default for ReadSlot {
    fn default() -> Self {
        let (changed, _) = tokio::sync::watch::channel(0);
        Self {
            state: std::sync::atomic::AtomicU8::new(SLOT_IDLE),
            changed,
        }
    }
}

impl ReadSlot {
    /// Whether this caller is the one that runs. `false` = a read is
    /// already in flight and has been booked to repeat.
    pub(super) fn claim(&self) -> bool {
        let mut seen = self.state.load(Ordering::SeqCst);
        loop {
            let next = if seen == SLOT_IDLE {
                SLOT_RUNNING
            } else {
                SLOT_AGAIN
            };
            match self
                .state
                .compare_exchange(seen, next, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => return seen == SLOT_IDLE,
                Err(actual) => seen = actual,
            }
        }
    }

    /// Called by the reader when its pass lands. `true` = somebody asked
    /// while it was running, so it goes round once more.
    fn finish(&self) -> bool {
        loop {
            if self
                .state
                .compare_exchange(SLOT_RUNNING, SLOT_IDLE, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                self.changed
                    .send_modify(|generation| *generation = generation.wrapping_add(1));
                return false;
            }
            if self
                .state
                .compare_exchange(SLOT_AGAIN, SLOT_RUNNING, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return true;
            }
        }
    }

    /// Opens the slot from wherever it was left.
    ///
    /// For the reader that never reached [`Self::finish`] — its task was
    /// dropped with the runtime, or it unwound. Without this, a slot left
    /// running turns one lost pass into a section of the window that
    /// never updates again and never says why, which is a far worse
    /// failure than the duplicate read the slot exists to stop.
    fn abandon(&self) {
        self.state.store(SLOT_IDLE, Ordering::SeqCst);
        self.changed
            .send_modify(|generation| *generation = generation.wrapping_add(1));
    }

    /// Waits until the reader that currently owns this slot, including a
    /// booked repeat, has returned it. A later caller may claim it again;
    /// this closes the work already in flight rather than reserving silence.
    pub(super) async fn wait_idle(&self) {
        let mut changed = self.changed.subscribe();
        while self.state.load(Ordering::SeqCst) != SLOT_IDLE {
            if changed.changed().await.is_err() {
                return;
            }
        }
    }
}

/// Opens a [`ReadSlot`] when the reader holding it goes away without
/// finishing (see [`ReadSlot::abandon`]). A normal final `finish` disarms
/// it before another reader can claim the returned slot.
pub(super) struct SlotHeld<'a> {
    slot: &'a ReadSlot,
    armed: bool,
}

impl<'a> SlotHeld<'a> {
    pub(super) fn new(slot: &'a ReadSlot) -> Self {
        Self { slot, armed: true }
    }

    /// Lands one pass and keeps the guard armed only when a booked repeat
    /// still owns the same slot.
    pub(super) fn finish(&mut self) -> bool {
        let again = self.slot.finish();
        if !again {
            self.armed = false;
        }
        again
    }
}

impl Drop for SlotHeld<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.slot.abandon();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_caller_reads_and_the_second_books_one_more_pass() {
        let slot = ReadSlot::default();
        assert!(slot.claim(), "nothing was running");
        assert!(!slot.claim(), "a read is in flight, so this one waits");
        assert!(!slot.claim(), "and so does the next");
        let mut held = SlotHeld::new(&slot);
        assert!(held.finish(), "somebody asked while it ran");
        assert!(!held.finish(), "nobody asked during the repeat");
        assert!(slot.claim(), "and the slot is free again");
    }

    #[test]
    fn a_reader_that_never_finishes_does_not_take_the_slot_with_it() {
        let slot = ReadSlot::default();
        assert!(slot.claim());
        assert!(!slot.claim(), "and somebody is waiting behind it");
        drop(SlotHeld::new(&slot));
        assert!(
            slot.claim(),
            "the next ask runs rather than waiting on a reader that is gone"
        );
    }

    #[test]
    fn a_request_that_arrives_as_a_read_lands_is_not_lost() {
        let slot = ReadSlot::default();
        assert!(slot.claim());
        assert!(!slot.claim());
        let mut held = SlotHeld::new(&slot);
        assert!(held.finish());
        // Mid-repeat, a third caller: still exactly one more pass.
        assert!(!slot.claim());
        assert!(held.finish());
        assert!(!held.finish());
    }

    #[test]
    fn a_finished_guard_does_not_abandon_the_next_reader() {
        let slot = ReadSlot::default();
        assert!(slot.claim());
        let mut first = SlotHeld::new(&slot);
        assert!(!first.finish());

        assert!(slot.claim(), "a new reader owns the returned slot");
        drop(first);
        assert!(
            !slot.claim(),
            "dropping the completed guard did not clear the new owner"
        );
        let mut second = SlotHeld::new(&slot);
        assert!(second.finish(), "the last claim booked one repeat");
        assert!(!second.finish());
    }
}
