//! Fault injection for one command log's feed (`PGG_FAULT_HOLD_RECORDS`).
//! Its model owns the withheld records and the release, so another tab cannot drain them or end its hold.
//! From the first requested command onward, records stay in feed order until that model releases them.

use crate::hub::CommandMsg;

#[derive(Default)]
pub(crate) struct HeldRecords {
    #[cfg(any(feature = "automation", test))]
    messages: Vec<CommandMsg>,
    #[cfg(any(feature = "automation", test))]
    lowered: bool,
}

impl HeldRecords {
    pub(crate) fn through(&mut self, messages: Vec<CommandMsg>) -> Vec<CommandMsg> {
        #[cfg(any(feature = "automation", test))]
        return self.take(messages, crate::harness::knobs().fault_hold_records);
        #[cfg(not(any(feature = "automation", test)))]
        messages
    }

    /// Releases this feed only. The model drains it immediately after this call.
    pub(crate) fn release(&mut self) -> i32 {
        #[cfg(any(feature = "automation", test))]
        return self.lower(crate::harness::knobs().fault_hold_records);
        #[cfg(not(any(feature = "automation", test)))]
        -1
    }

    #[cfg(any(feature = "automation", test))]
    fn take(&mut self, mut messages: Vec<CommandMsg>, enabled: bool) -> Vec<CommandMsg> {
        if !enabled || self.lowered {
            let mut out = std::mem::take(&mut self.messages);
            out.append(&mut messages);
            return out;
        }
        let from = if self.messages.is_empty() {
            messages
                .iter()
                .position(|msg| matches!(msg, CommandMsg::Started { asked: true, .. }))
        } else {
            Some(0)
        };
        if let Some(at) = from {
            self.messages.extend(messages.drain(at..));
        }
        messages
    }

    #[cfg(any(feature = "automation", test))]
    fn lower(&mut self, enabled: bool) -> i32 {
        if !enabled || self.lowered {
            return -1;
        }
        self.lowered = true;
        i32::try_from(self.messages.len()).unwrap_or(i32::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(operation: u64) -> Vec<CommandMsg> {
        vec![
            CommandMsg::Started {
                run: 1,
                id: 1,
                display: "git commit".into(),
                full: "git commit".into(),
                at_ms: 0,
                asked: true,
                operation,
            },
            CommandMsg::Settled { operation },
        ]
    }

    #[test]
    fn a_second_tab_cannot_take_the_first_tabs_records_or_release_its_hold() {
        let mut a = HeldRecords::default();
        let mut b = HeldRecords::default();
        assert!(a.take(write(11), true).is_empty());
        assert!(b.take(write(22), true).is_empty());
        assert_eq!(b.lower(true), 2);
        let b_records = b.take(Vec::new(), true);
        assert!(matches!(
            b_records.as_slice(),
            [
                CommandMsg::Started { operation: 22, .. },
                CommandMsg::Settled { operation: 22 }
            ]
        ));
        assert!(a.take(Vec::new(), true).is_empty());
        assert_eq!(a.lower(true), 2);
        let a_records = a.take(Vec::new(), true);
        assert!(matches!(
            a_records.as_slice(),
            [
                CommandMsg::Started { operation: 11, .. },
                CommandMsg::Settled { operation: 11 }
            ]
        ));
        assert_eq!(a.lower(true), -1);
    }
}
