//! A write's records in the log: its rows under its id, and the receipt
//! that says all of them are in (`CommandMsg::Settled`).

use super::*;

fn started(id: u64, operation: u64) -> CommandMsg {
    CommandMsg::Started {
        run: 1,
        id,
        display: format!("git push {id}"),
        full: format!("git push {id}"),
        at_ms: 0,
        asked: true,
        operation,
    }
}

fn finished(id: u64, code: i32) -> CommandMsg {
    CommandMsg::Finished {
        run: 1,
        id,
        code: Some(code),
        note: String::new(),
        answered: false,
        waited_ms: 0,
        elapsed_ms: 5,
        message: String::new(),
    }
}

fn settled(operation: u64) -> CommandMsg {
    CommandMsg::Settled { operation }
}

/// Every command of a write goes in ahead of its receipt, however many it
/// ran — rows do not say when the last of them is in, the receipt does.
#[test]
fn a_write_is_recorded_at_its_receipt_whatever_its_commands_number() {
    let mut log = CommandsModel::default();
    log.take_in(vec![
        started(1, 7),
        finished(1, 0),
        started(2, 7),
        finished(2, 0),
    ]);
    assert_eq!(log.rows_of(7), 2);
    assert!(
        !log.recorded(7),
        "two rows in, and a third command may still come"
    );
    log.take_in(vec![started(3, 7), finished(3, 0), settled(7)]);
    assert_eq!(log.rows_of(7), 3);
    assert!(log.recorded(7));

    log.take_in(vec![started(4, 8), finished(4, 0), settled(8)]);
    assert_eq!(log.rows_of(8), 1);
    assert!(log.recorded(8));
}

/// A write that ran nothing — refused before git, or nothing to do — is
/// recorded by its receipt alone.
#[test]
fn a_write_that_ran_no_command_is_recorded_by_its_receipt() {
    let mut log = CommandsModel::default();
    assert!(!log.recorded(9));
    log.take_in(vec![settled(9)]);
    assert!(log.recorded(9));
    assert_eq!(log.rows_of(9), 0);
}

/// A refused write's records are in at its receipt as a landed one's are;
/// the failure is the row's to show, not the receipt's to hold.
#[test]
fn a_refused_write_is_recorded_as_a_landed_one_is() {
    let mut log = CommandsModel::default();
    log.take_in(vec![
        started(1, 3),
        finished(1, 1),
        started(2, 3),
        finished(2, 0),
        settled(3),
    ]);
    assert!(log.recorded(3));
    assert_eq!(log.rows_of(3), 2);
    assert_eq!(log.rows[0].status, "failed");
    assert_eq!(log.rows[1].status, "ok");
}

/// The receipt is asked after by the write's id: another write's receipt,
/// or a row of the write's own, answers nothing for it.
#[test]
fn a_receipt_answers_for_its_own_write_only() {
    let mut log = CommandsModel::default();
    log.take_in(vec![started(1, 4), finished(1, 0), settled(5)]);
    assert!(log.recorded(5));
    assert!(!log.recorded(4), "a row of 4's, and 5's receipt");
    assert!(!log.recorded(0), "reads outside any write carry no id");
    assert!(!log.recorded(-1));
    assert_eq!(log.rows_of(-1), 0);
}

/// The reader's `Clear` empties the rows; what the log has taken in stays
/// taken in.
#[test]
fn clearing_the_rows_keeps_the_receipts() {
    let mut log = CommandsModel::default();
    log.take_in(vec![started(1, 6), finished(1, 0), settled(6)]);
    log.reset_unnotified();
    assert_eq!(log.rows_of(6), 0);
    assert!(log.recorded(6));
}

/// The log keeps a bounded run of receipts, the oldest falling off first.
#[test]
fn the_oldest_receipt_falls_off_past_the_bound() {
    let mut log = CommandsModel::default();
    let last = u64::try_from(KEEP_RECEIPTS).unwrap() + 1;
    log.take_in((1..=last).map(settled).collect());
    assert!(!log.recorded(1));
    assert!(log.recorded(2));
    assert!(log.recorded(i32::try_from(last).unwrap()));
}
