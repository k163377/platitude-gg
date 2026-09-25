use super::*;
use crate::wait::poll_once;

fn slots(total: usize, reserve: usize) -> Arc<Slots> {
    Arc::new(Slots::new(Limits { total, reserve }))
}

/// Kept as a future: a pending ask is a place in the queue, and dropping
/// it leaves.
type Ask = std::pin::Pin<Box<dyn std::future::Future<Output = Option<Slot>> + Send>>;

fn ask(slots: &Arc<Slots>, priority: Priority, pace: Pace) -> Ask {
    let slots = Arc::clone(slots);
    Box::pin(async move { slots.acquire(priority, pace).await })
}

fn granted(ask: &mut Ask) -> Option<Slot> {
    match poll_once(ask) {
        std::task::Poll::Ready(slot) => slot,
        std::task::Poll::Pending => None,
    }
}

#[tokio::test]
async fn the_cap_holds_and_a_slot_given_back_admits_the_next() {
    let slots = slots(2, 1);
    let mut first = ask(&slots, Priority::Interactive, Pace::Here);
    let mut second = ask(&slots, Priority::Interactive, Pace::Here);
    let mut third = ask(&slots, Priority::Interactive, Pace::Here);
    let held_first = granted(&mut first).expect("the first fits");
    let _held_second = granted(&mut second).expect("the second fits");
    assert!(granted(&mut third).is_none(), "the third waits for a slot");
    assert_eq!(slots.report().running, 2);
    assert_eq!(slots.report().queued_interactive, 1);

    drop(held_first);
    let _held_third = granted(&mut third).expect("the slot given back admits the third");
    assert_eq!(slots.report().running, 2);
    assert_eq!(slots.report().queued_interactive, 0);
    assert_eq!(slots.report().admitted, 3);
}

#[tokio::test]
async fn background_reads_are_kept_out_of_the_reserve_and_a_click_still_fits() {
    let slots = slots(4, 3);
    let mut first = ask(&slots, Priority::Background, Pace::Here);
    let mut second = ask(&slots, Priority::Background, Pace::Here);
    let _held = granted(&mut first).expect("one background read runs");
    assert!(
        granted(&mut second).is_none(),
        "the second background read waits under the background cap"
    );
    let mut click = ask(&slots, Priority::Interactive, Pace::Here);
    let _clicked = granted(&mut click).expect("the reserve is the click's");
    let report = slots.report();
    assert_eq!(report.running, 2);
    assert_eq!(report.running_background, 1);
    assert_eq!(report.queued_background, 1);
}

#[tokio::test]
async fn an_interactive_ask_goes_ahead_of_a_background_one_queued_before_it() {
    let slots = slots(1, 0);
    let mut held = ask(&slots, Priority::Interactive, Pace::Here);
    let holding = granted(&mut held).expect("the pool is one slot");
    let mut background = ask(&slots, Priority::Background, Pace::Here);
    let mut click = ask(&slots, Priority::Interactive, Pace::Here);
    assert!(granted(&mut background).is_none());
    assert!(granted(&mut click).is_none());

    drop(holding);
    let _clicked = granted(&mut click).expect("the click was served first");
    assert!(
        granted(&mut background).is_none(),
        "the background read is still waiting behind it"
    );
}

/// Served next even with a click waiting beside it.
#[tokio::test]
async fn a_background_read_overtaken_often_enough_is_served_next() {
    let slots = slots(1, 0);
    let mut held = ask(&slots, Priority::Interactive, Pace::Here);
    let mut holding = granted(&mut held).expect("the pool is one slot");
    let mut background = ask(&slots, Priority::Background, Pace::Here);
    assert!(granted(&mut background).is_none());

    for nth in 0..OVERTAKEN_LIMIT {
        let mut click = ask(&slots, Priority::Interactive, Pace::Here);
        assert!(
            granted(&mut click).is_none(),
            "click {nth} waits for the slot"
        );
        drop(holding);
        holding = granted(&mut click).expect("the click overtakes the background read");
        assert!(
            granted(&mut background).is_none(),
            "overtaken {} times, the background read still waits",
            nth + 1
        );
    }
    let mut one_more = ask(&slots, Priority::Interactive, Pace::Here);
    assert!(granted(&mut one_more).is_none());
    drop(holding);
    let _read = granted(&mut background).expect("overtaken enough, it goes next");
    assert!(
        granted(&mut one_more).is_none(),
        "and the click behind it waits its turn"
    );
}

#[tokio::test]
async fn a_wait_dropped_leaves_the_queue_and_holds_nothing() {
    let slots = slots(1, 0);
    let mut held = ask(&slots, Priority::Interactive, Pace::Here);
    let holding = granted(&mut held).expect("the pool is one slot");
    let mut left = ask(&slots, Priority::Interactive, Pace::Here);
    let mut stays = ask(&slots, Priority::Interactive, Pace::Here);
    assert!(granted(&mut left).is_none());
    assert!(granted(&mut stays).is_none());
    assert_eq!(slots.report().queued_interactive, 2);

    // The token fired: the executor's `select!` drops the wait.
    drop(left);
    assert_eq!(slots.report().queued_interactive, 1);
    assert_eq!(slots.report().left_waiting, 1);

    drop(holding);
    let _next = granted(&mut stays).expect("the one that stayed is served, not the ghost");
    assert_eq!(slots.report().running, 1);
}

#[tokio::test]
async fn a_grant_that_crosses_the_leaving_is_given_straight_back() {
    let slots = slots(1, 0);
    let mut held = ask(&slots, Priority::Interactive, Pace::Here);
    let holding = granted(&mut held).expect("the pool is one slot");
    let mut waiting = ask(&slots, Priority::Interactive, Pace::Here);
    assert!(granted(&mut waiting).is_none());

    // Granted into the channel before the wait is polled again; then the
    // wait is dropped with the slot inside.
    drop(holding);
    assert_eq!(slots.report().running, 1, "granted, unpolled");
    drop(waiting);
    assert_eq!(
        slots.report().running,
        0,
        "the slot in the dropped channel came back"
    );
    let mut next = ask(&slots, Priority::Interactive, Pace::Here);
    assert!(granted(&mut next).is_some());
}

#[tokio::test]
async fn what_is_paced_elsewhere_and_what_nobody_waits_on_share_the_slots_outside_the_reserve() {
    let slots = slots(4, 2);
    let mut fetch = ask(&slots, Priority::Interactive, Pace::Elsewhere);
    let mut read = ask(&slots, Priority::Background, Pace::Here);
    let _fetching = granted(&mut fetch).expect("a fetch runs");
    let _reading = granted(&mut read).expect("beside a background read");
    let mut push = ask(&slots, Priority::Interactive, Pace::Elsewhere);
    let mut another = ask(&slots, Priority::Background, Pace::Here);
    assert!(
        granted(&mut push).is_none(),
        "a second round trip waits: what is shared outside the reserve is full"
    );
    assert!(
        granted(&mut another).is_none(),
        "and so does a second background read"
    );
    let mut click = ask(&slots, Priority::Interactive, Pace::Here);
    let mut second_click = ask(&slots, Priority::Interactive, Pace::Here);
    let mut third_click = ask(&slots, Priority::Interactive, Pace::Here);
    let _clicked = granted(&mut click).expect("the reserve is the click's");
    let _clicked_again = granted(&mut second_click).expect("all of it");
    assert!(
        granted(&mut third_click).is_none(),
        "and the pool as a whole is four"
    );
    let report = slots.report();
    assert_eq!(report.running, 4);
    assert_eq!(report.running_elsewhere, 1);
    assert_eq!(report.running_background, 1);
    assert_eq!(report.queued_interactive, 2, "the push and the third click");
    assert_eq!(report.queued_background, 1);
}

#[tokio::test]
async fn wider_limits_admit_what_is_waiting_and_narrower_ones_admit_nothing_more() {
    let slots = slots(1, 0);
    let mut first = ask(&slots, Priority::Interactive, Pace::Here);
    let mut second = ask(&slots, Priority::Interactive, Pace::Here);
    let _held = granted(&mut first).expect("one runs");
    assert!(granted(&mut second).is_none());

    slots.set_limits(Limits {
        total: 2,
        reserve: 1,
    });
    let held_second = granted(&mut second).expect("the wider pool admits the waiter");

    slots.set_limits(Limits {
        total: 1,
        reserve: 0,
    });
    let mut third = ask(&slots, Priority::Interactive, Pace::Here);
    assert!(
        granted(&mut third).is_none(),
        "two are running over a pool of one"
    );
    drop(held_second);
    assert!(
        granted(&mut third).is_none(),
        "one given back still leaves the pool full"
    );
    assert_eq!(slots.report().running, 1);
}

#[tokio::test]
async fn unbounded_slots_admit_everything_and_still_count() {
    let slots = Arc::new(Slots::unbounded());
    let mut asks: Vec<Ask> = (0..16)
        .map(|_| ask(&slots, Priority::Background, Pace::Here))
        .collect();
    let held: Vec<Slot> = asks
        .iter_mut()
        .map(|a| granted(a).expect("nothing waits"))
        .collect();
    assert_eq!(slots.report().running_background, 16);
    drop(held);
    assert_eq!(slots.report().running, 0);
}

#[test]
fn the_limits_one_number_stands_for() {
    assert_eq!(
        Limits::of(1),
        Limits {
            total: 1,
            reserve: 0
        },
        "one process: the background read has to be able to run at all"
    );
    assert_eq!(
        Limits::of(2),
        Limits {
            total: 2,
            reserve: 1
        }
    );
    assert_eq!(
        Limits::of(3),
        Limits {
            total: 3,
            reserve: 2
        },
        "a quarter rounds down for what is shared"
    );
    assert_eq!(
        Limits::of(4),
        Limits {
            total: 4,
            reserve: 3
        },
        "four slots is where the click's three commands all fit"
    );
    assert_eq!(
        Limits::of(8),
        Limits {
            total: 8,
            reserve: 6
        }
    );
    assert_eq!(Limits::of(8).shared(), 2);
    assert_eq!(
        Limits::of(0).total,
        1,
        "zero is the nearest number that runs anything"
    );
    assert_eq!(
        Limits::of(MAX_CONCURRENCY + 1).total,
        MAX_CONCURRENCY as usize
    );
}

#[tokio::test]
async fn an_identical_ask_follows_a_queued_leader_and_leads_after_a_spawned_one() {
    let slots = Arc::new(Slots::unbounded());
    let key = GroupKey {
        program: "git".into(),
        cwd: None,
        args: vec!["status".into()],
        env: Vec::new(),
    };
    let WayIn::Lead(mut lead) = slots.lead_or_follow(key.clone()) else {
        panic!("the first ask leads");
    };
    let WayIn::Follow(mut follows) = slots.lead_or_follow(key.clone()) else {
        panic!("the second ask follows the queued leader");
    };
    lead.spawning();
    assert!(lead.has_followers(), "the follower joined before the spawn");
    let WayIn::Lead(_late) = slots.lead_or_follow(key) else {
        panic!("an ask after the spawn leads for itself");
    };
    drop(lead);
    assert!(
        poll_once(&mut follows).is_ready(),
        "followers dropped unanswered are told so, and ask again"
    );
    assert_eq!(slots.report().shared, 1);
}

#[tokio::test]
async fn a_leader_that_leaves_before_spawning_closes_its_group() {
    let slots = Arc::new(Slots::unbounded());
    let key = GroupKey {
        program: "git".into(),
        cwd: None,
        args: vec!["status".into()],
        env: Vec::new(),
    };
    let WayIn::Lead(lead) = slots.lead_or_follow(key.clone()) else {
        panic!("the first ask leads");
    };
    let WayIn::Follow(mut follows) = slots.lead_or_follow(key.clone()) else {
        panic!("the second follows");
    };
    drop(lead);
    assert!(
        poll_once(&mut follows).is_ready(),
        "the follower is told the leader left"
    );
    let WayIn::Lead(_) = slots.lead_or_follow(key) else {
        panic!("with the group closed, the next ask leads");
    };
}

// --- waiting for the queue to move (`settled`) ------------------------

#[tokio::test]
async fn a_wait_for_what_already_stands_answers_at_once() {
    let slots = slots(1, 0);
    let mut first = ask(&slots, Priority::Interactive, Pace::Here);
    let _held = granted(&mut first).expect("the first fits");
    let mut second = ask(&slots, Priority::Interactive, Pace::Here);
    assert!(granted(&mut second).is_none(), "the second queues");

    let mut waiting = Box::pin(slots.settled(|report| report.queued_interactive == 1));
    assert!(
        poll_once(&mut waiting).is_ready(),
        "the queue already answers, so nothing is waited for"
    );
}

#[tokio::test]
async fn a_wait_begun_first_is_woken_by_the_ask_that_follows() {
    let slots = slots(1, 0);
    let mut first = ask(&slots, Priority::Interactive, Pace::Here);
    let _held = granted(&mut first).expect("the first fits");

    let mut waiting = Box::pin(slots.settled(|report| report.queued_interactive == 1));
    assert!(
        poll_once(&mut waiting).is_pending(),
        "nothing is queued yet, so it registers and waits"
    );

    let mut second = ask(&slots, Priority::Interactive, Pace::Here);
    assert!(granted(&mut second).is_none(), "the second queues");
    assert!(
        poll_once(&mut waiting).is_ready(),
        "and the wait it was registered for woke it"
    );
}

/// The window [`Slots::settled`] takes its `Notified` out ahead of, driven
/// through the predicate: it runs on a `Report` already read, so an ask
/// joining from inside it stirs exactly there (and it answers `false`). A
/// `settled` that took `Notified` out after the look would miss this stir
/// and wait forever.
#[tokio::test]
async fn a_stir_from_inside_the_look_still_ends_the_wait() {
    let slots = slots(1, 0);
    let mut first = ask(&slots, Priority::Interactive, Pace::Here);
    let _held = granted(&mut first).expect("the first fits");
    let mut second = ask(&slots, Priority::Interactive, Pace::Here);

    let looks = std::cell::Cell::new(0);
    let mut waiting = Box::pin(slots.settled(|report| {
        looks.set(looks.get() + 1);
        if looks.get() == 1 {
            assert_eq!(report.queued_interactive, 0, "read before the join");
            assert!(
                granted(&mut second).is_none(),
                "the second joins the queue from inside the look"
            );
        }
        report.queued_interactive == 1
    }));

    assert!(
        poll_once(&mut waiting).is_ready(),
        "the stir made inside the look was held, and the look taken again answered"
    );
    drop(waiting);
    assert_eq!(
        looks.get(),
        2,
        "it looked again rather than answering on the stir"
    );
}

#[tokio::test]
async fn a_stir_is_not_the_answer_and_leaves_none_behind() {
    let slots = slots(1, 0);
    let mut first = ask(&slots, Priority::Interactive, Pace::Here);
    let _held = granted(&mut first).expect("the first fits");
    // Two stirs with nobody registered: one queues, one leaves.
    let mut early = ask(&slots, Priority::Interactive, Pace::Here);
    assert!(granted(&mut early).is_none(), "it queues");
    drop(early);
    assert_eq!(slots.report().queued_interactive, 0, "and it leaves");

    let mut waiting = Box::pin(slots.settled(|report| report.queued_interactive == 1));
    assert!(
        poll_once(&mut waiting).is_pending(),
        "the stirs before it began left nothing it could take for an answer"
    );

    // A stir from the background queue: woken, looked, still not the answer.
    let mut background = ask(&slots, Priority::Background, Pace::Elsewhere);
    assert!(granted(&mut background).is_none(), "the pool is full");
    assert!(
        poll_once(&mut waiting).is_pending(),
        "it was woken and looked, and the interactive queue is still empty"
    );

    let mut second = ask(&slots, Priority::Interactive, Pace::Here);
    assert!(granted(&mut second).is_none(), "the second queues");
    assert!(poll_once(&mut waiting).is_ready());
}
