//! Fault injection for one command log's feed (`PGG_FAULT_HOLD_RECORDS`): the records the log's model holds back
//! until the run lets them through (`CommandsModel.letRecordsThrough`), so a write's answer reaches the tab ahead
//! of its commands reaching the log — an order no contract of the two feeds (`hub::Feeds`) rules out, made the
//! run's own (`xtask::verify::records_check`). The model owns the withheld records and the release: another tab
//! can neither drain them nor end its hold. From the first requested command onward, records stay in feed order
//! until that model releases them. A build without the harness lets everything through.

use crate::hub::CommandMsg;

#[derive(Default)]
pub(crate) struct HeldRecords {
    /// Raised by the knob when the model attached; lowered once, by the run.
    #[cfg(any(feature = "automation", test))]
    standing: bool,
    #[cfg(any(feature = "automation", test))]
    messages: Vec<CommandMsg>,
}

impl HeldRecords {
    /// The hold as the knob asks for it where the model attaches to its tab.
    pub(crate) fn from_knobs() -> Self {
        #[cfg(any(feature = "automation", test))]
        return Self {
            standing: crate::harness::knobs().fault_hold_records,
            messages: Vec::new(),
        };
        #[cfg(not(any(feature = "automation", test)))]
        Self::default()
    }

    /// What the log applies of what its feed brought: all of it, but where the hold stands.
    pub(crate) fn through(&mut self, messages: Vec<CommandMsg>) -> Vec<CommandMsg> {
        #[cfg(any(feature = "automation", test))]
        return self.take(messages);
        #[cfg(not(any(feature = "automation", test)))]
        messages
    }

    /// Lowers the hold for good, answering how many messages it held — the model drains them next — or -1 where
    /// none stood. This feed's only.
    pub(crate) fn release(&mut self) -> i32 {
        #[cfg(any(feature = "automation", test))]
        return self.lower();
        #[cfg(not(any(feature = "automation", test)))]
        -1
    }

    /// Rows nobody asked for (the opening's reads) are not the write's; once one asked for is held, everything
    /// behind it is.
    #[cfg(any(feature = "automation", test))]
    fn take(&mut self, mut messages: Vec<CommandMsg>) -> Vec<CommandMsg> {
        if !self.standing {
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
    fn lower(&mut self) -> i32 {
        if !self.standing {
            return -1;
        }
        self.standing = false;
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

    fn standing() -> HeldRecords {
        HeldRecords {
            standing: true,
            messages: Vec::new(),
        }
    }

    #[test]
    fn a_second_tab_cannot_take_the_first_tabs_records_or_release_its_hold() {
        let mut a = standing();
        let mut b = standing();
        assert!(a.through(write(11)).is_empty());
        assert!(b.through(write(22)).is_empty());
        assert_eq!(b.release(), 2);
        let b_records = b.through(Vec::new());
        assert!(matches!(
            b_records.as_slice(),
            [
                CommandMsg::Started { operation: 22, .. },
                CommandMsg::Settled { operation: 22 }
            ]
        ));
        assert!(a.through(Vec::new()).is_empty());
        assert_eq!(a.release(), 2);
        let a_records = a.through(Vec::new());
        assert!(matches!(
            a_records.as_slice(),
            [
                CommandMsg::Started { operation: 11, .. },
                CommandMsg::Settled { operation: 11 }
            ]
        ));
        assert_eq!(a.release(), -1);
    }

    #[test]
    fn a_hold_nobody_asked_for_lets_everything_through() {
        let mut none = HeldRecords::default();
        assert_eq!(none.through(write(11)).len(), 2);
        assert_eq!(none.release(), -1);
    }
}
