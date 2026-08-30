//! Adversarial delivery orders for the interactive-rebase plan, the way
//! [`super::details_tests`] holds the line for commit details.
//!
//! The plan is asked for by a right-click, and the read behind it walks
//! `from^..HEAD` — so a click deep in the history takes far longer to
//! answer than a click near the tip, and the two can land in either
//! order. Whichever way round they land, what reaches the screen has to
//! be the answer to the click it is waiting on.
use super::*;
use platitude_core::rebase_plan::{PlanPreview, PlanRefusal, PlanRow};
use sink::BridgeSink;

fn preview(from: &str) -> PlanPreview {
    PlanPreview {
        from: from.to_string(),
        upstream: "base".to_string(),
        root: false,
        range: format!("base..{from}"),
        rows: vec![PlanRow {
            oid: from.to_string(),
            subject: format!("subject of {from}"),
            ..PlanRow::default()
        }],
        onto: None,
        onto_ref: String::new(),
    }
}

fn asked(feeds: &Feeds) -> Vec<String> {
    feeds
        .plan
        .drain()
        .iter()
        .map(|message| match message {
            PlanMsg::Loaded { preview } => format!("loaded {}", preview.from),
            PlanMsg::Refused { from, .. } => format!("refused {from}"),
            PlanMsg::Failed { from } => format!("failed {from}"),
        })
        .collect()
}

/// The report this test was written from: a deep commit is right-clicked,
/// then a shallow one. The shallow answer is queued first, and the deep
/// one lands before the Qt thread gets round to draining. Replacing the
/// queue there hands the model the answer to a click it has already left
/// behind, which it drops as stale (`RebasePlanModel::take` keys on
/// `asked_from`) — and nothing is then coming for the click that is
/// waiting, so the screen never opens and never says why.
#[test]
fn a_late_answer_to_an_older_click_cannot_eat_the_one_the_screen_waits_on() {
    let feeds = Arc::new(Feeds::default());
    let sink = BridgeSink {
        feeds: Arc::clone(&feeds),
    };
    sink.event(SessionEvent::RebasePlanLoaded {
        generation: 2,
        preview: preview("shallow"),
    });
    sink.event(SessionEvent::RebasePlanLoaded {
        generation: 1,
        preview: preview("deep"),
    });
    assert_eq!(asked(&feeds), vec!["loaded shallow"]);
}

/// The three answers share one slot, so the watermark has to hold across
/// their kinds: a refusal for the newest click is as much its answer as a
/// plan is, and an older click's plan must not stand in front of it.
#[test]
fn an_older_plan_cannot_stand_in_front_of_a_newer_refusal() {
    let feeds = Arc::new(Feeds::default());
    let sink = BridgeSink {
        feeds: Arc::clone(&feeds),
    };
    sink.event(SessionEvent::RebasePlanRefused {
        generation: 4,
        from: "shallow".to_string(),
        refusal: PlanRefusal::AcrossMerge,
    });
    sink.event(SessionEvent::RebasePlanLoaded {
        generation: 3,
        preview: preview("deep"),
    });
    assert_eq!(asked(&feeds), vec!["refused shallow"]);

    // And the same the other way about: a read that failed answers the
    // click that is waiting, so the model can put its waiting state down.
    sink.event(SessionEvent::RebasePlanLoaded {
        generation: 5,
        preview: preview("deep"),
    });
    sink.event(SessionEvent::RebasePlanFailed {
        generation: 6,
        from: "shallow".to_string(),
    });
    assert_eq!(asked(&feeds), vec!["failed shallow"]);
}

/// A generation already drained is spent: an answer arriving under it
/// again must not wake the consumer for a plan it has finished with.
#[test]
fn a_drained_answer_is_not_requeued_by_a_late_duplicate() {
    let feeds = Arc::new(Feeds::default());
    let sink = BridgeSink {
        feeds: Arc::clone(&feeds),
    };
    sink.event(SessionEvent::RebasePlanLoaded {
        generation: 7,
        preview: preview("shallow"),
    });
    assert_eq!(asked(&feeds), vec!["loaded shallow"]);
    sink.event(SessionEvent::RebasePlanLoaded {
        generation: 7,
        preview: preview("shallow"),
    });
    assert_eq!(feeds.plan.depth(), 0);
}
