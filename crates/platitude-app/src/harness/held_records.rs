//! The command log's records the harness holds back
//! (`PGG_FAULT_HOLD_RECORDS`): from the first row a reader asked for,
//! everything the log's feed brings stays off the log until the run lets
//! it through. A write's answer then reaches the tab ahead of its commands
//! reaching the log — an order the two feeds (`hub::Feeds`) are free to
//! drain in, made the run's own (`xtask::verify::records_check`). A build
//! without the harness lets everything through.

use crate::hub::CommandMsg;

#[cfg(feature = "automation")]
mod hold {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, MutexGuard};

    use crate::hub::CommandMsg;

    /// Lowered once, by the run; raised from the knob until then.
    static LOWERED: AtomicBool = AtomicBool::new(false);
    /// What the hold took off the feed, in the feed's order.
    static HELD: Mutex<Vec<CommandMsg>> = Mutex::new(Vec::new());

    fn held() -> MutexGuard<'static, Vec<CommandMsg>> {
        match HELD.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn standing() -> bool {
        crate::harness::knobs().fault_hold_records && !LOWERED.load(Ordering::SeqCst)
    }

    pub(super) fn through(mut messages: Vec<CommandMsg>) -> Vec<CommandMsg> {
        let mut held = held();
        if !standing() {
            if held.is_empty() {
                return messages;
            }
            let mut out = std::mem::take(&mut *held);
            out.append(&mut messages);
            return out;
        }
        // Rows nobody asked for (the opening's reads) are not the write's;
        // once one asked for is held, everything behind it is.
        let from = if held.is_empty() {
            messages
                .iter()
                .position(|msg| matches!(msg, CommandMsg::Started { asked: true, .. }))
        } else {
            Some(0)
        };
        if let Some(at) = from {
            held.extend(messages.drain(at..));
        }
        messages
    }

    pub(super) fn let_through() -> i32 {
        if !crate::harness::knobs().fault_hold_records || LOWERED.swap(true, Ordering::SeqCst) {
            return -1;
        }
        i32::try_from(held().len()).unwrap_or(i32::MAX)
    }
}

/// What the log applies of what its feed brought: all of it, but where
/// the hold stands.
#[cfg(feature = "automation")]
pub(crate) fn records_through(messages: Vec<CommandMsg>) -> Vec<CommandMsg> {
    hold::through(messages)
}

#[cfg(not(feature = "automation"))]
pub(crate) fn records_through(messages: Vec<CommandMsg>) -> Vec<CommandMsg> {
    messages
}

/// Lowers the hold for good, answering how many messages it held — the
/// log drains them next — or -1 where none stood
/// (`CommandsModel.letRecordsThrough`).
#[cfg(feature = "automation")]
pub(crate) fn let_records_through() -> i32 {
    hold::let_through()
}

#[cfg(not(feature = "automation"))]
pub(crate) fn let_records_through() -> i32 {
    -1
}
