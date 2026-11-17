//! Delivery orders for the interactive-rebase plan, as
//! [`super::details_tests`] for commit details: the read walks
//! `from^..HEAD`, so a deep click answers later than a shallow one, and
//! the screen must still get the answer to the click it waits on.
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
        published: 0,
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
            PlanMsg::Published { range, published } => format!("published {range} {published}"),
        })
        .collect()
}

/// A deep commit is right-clicked, then a shallow one; the deep answer
/// lands after the shallow one is queued, before the drain. A replacing
/// queue would hand the model the answer it drops as stale
/// (`RebasePlanModel::take` keys on `asked_from`), and the screen would
/// wait forever.
#[test]
fn a_late_answer_to_an_older_click_cannot_eat_the_one_the_screen_waits_on() {
    let feeds = Arc::new(Feeds::default());
    let sink = BridgeSink::new(Arc::clone(&feeds), 1);
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

/// The three answers share one slot, so the watermark holds across kinds.
#[test]
fn an_older_plan_cannot_stand_in_front_of_a_newer_refusal() {
    let feeds = Arc::new(Feeds::default());
    let sink = BridgeSink::new(Arc::clone(&feeds), 1);
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

    // The other way about: a failed read answers the waiting click too.
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

#[test]
fn a_drained_answer_is_not_requeued_by_a_late_duplicate() {
    let feeds = Arc::new(Feeds::default());
    let sink = BridgeSink::new(Arc::clone(&feeds), 1);
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
