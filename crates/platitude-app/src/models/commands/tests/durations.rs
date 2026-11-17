use super::super::*;
#[test]
fn durations_read_in_the_unit_they_belong_to() {
    assert_eq!(humanize(0), "0 ms");
    assert_eq!(humanize(84), "84 ms");
    assert_eq!(humanize(999), "999 ms");
    assert_eq!(humanize(1000), "1.00 s");
    assert_eq!(humanize(1425), "1.42 s");
    assert_eq!(humanize(63_500), "63.50 s");
}

#[test]
fn a_wait_for_a_slot_is_shown_beside_the_run_and_only_when_there_was_one() {
    assert_eq!(spent(84, 0), "84 ms");
    assert_eq!(spent(84, 120), "84 ms (queued 120 ms)");
    assert_eq!(spent(1425, 2000), "1.42 s (queued 2.00 s)");
}
