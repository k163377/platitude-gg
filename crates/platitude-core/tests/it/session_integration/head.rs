//! HEAD as one record: every read reports into it, the consumer hears
//! about it when it moves — and once more after a write, moved or not —
//! and the walk answers whether a remote already has the commit it is on
//! (`session::standing`).

use crate::support::TestRepo;
use crate::support::remote::origin_and_clone;
use crate::support::session::{open_unawaited, opened, write_result};
use crate::support::wait::bounded;
use platitude_core::OperationKind;
use platitude_core::session::{RefreshOutcome, SessionEvent};

/// Every `HeadObserved` the session has sent, as `(oid, seq)`.
fn heads_of(events: &[SessionEvent]) -> Vec<(Option<String>, u64)> {
    events
        .iter()
        .filter_map(|e| match e {
            SessionEvent::HeadObserved { head, seq } => Some((head.oid.map(|o| o.to_hex()), *seq)),
            _ => None,
        })
        .collect()
}

/// The number the write's answer named as the first a report after it
/// can carry.
fn owed_by(events: &[SessionEvent], kind: OperationKind) -> Option<u64> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::WriteFinished {
            kind: got,
            head_seq,
            ..
        } if *got == kind => Some(*head_seq),
        _ => None,
    })
}

/// The last `HeadPublished` about `oid`, if one has been sent.
fn published_of(events: &[SessionEvent], oid: &str) -> Option<bool> {
    events.iter().rev().find_map(|e| match e {
        SessionEvent::HeadPublished {
            oid: about,
            published,
        } if about.map(|o| o.to_hex()).as_deref() == Some(oid) => Some(*published),
        _ => None,
    })
}

/// The refs read and the status read both see HEAD, and the consumer hears
/// about it once — at opening, and again after a terminal commit moves it.
/// The walk answers from its own rows whether a remote has it.
#[tokio::test(flavor = "multi_thread")]
async fn head_is_reported_once_and_the_walk_says_whether_a_remote_has_it() {
    let (_bare, mut work) = origin_and_clone();
    let root = work.git(&["rev-parse", "HEAD"]);
    let (sink, session) = open_unawaited(&work);
    // The opening's passes must be over first: a tag-inclusive pass still
    // running would draw the commit, leaving the tick below nothing to change.
    sink.opened_graph(&session, 1).await;

    let heads = heads_of(&sink.events.lock().unwrap());
    assert_eq!(
        heads.iter().map(|(oid, _)| oid.clone()).collect::<Vec<_>>(),
        vec![Some(root.clone())],
        "two reads reported the same HEAD, and the consumer heard it once"
    );
    let published = sink
        .wait_for("the walk's answer about HEAD", |evs| {
            published_of(evs, &root)
        })
        .await;
    assert!(published, "the opening commit is on origin/main");

    // A terminal moves HEAD; the poll's two reads both see it.
    let local = work.commit_file_id("b.txt", "two\n", "local");
    let outcome = bounded("the tracked poll", session.refresh_poll_tracked().outcome()).await;
    assert_eq!(
        outcome,
        RefreshOutcome::Changed,
        "a moved ref walks again: {:?}",
        sink.events.lock().unwrap()
    );
    sink.wait_for("HEAD to move", |evs| {
        heads_of(evs)
            .into_iter()
            .find(|(oid, _)| oid.as_deref() == Some(local.as_str()))
    })
    .await;
    assert_eq!(
        heads_of(&sink.events.lock().unwrap()).len(),
        2,
        "the second read of the same tick found HEAD already there"
    );
    let published = sink
        .wait_for("the walk's answer about the new HEAD", |evs| {
            published_of(evs, &local)
        })
        .await;
    assert!(!published, "the local commit is on no remote");
    session.close();
}

/// The first read after a write reports HEAD even unmoved, under a number
/// at or above the one the write's answer named — what a consumer waiting
/// for "the repository as the write left it" waits on — with the status
/// read beside it.
#[tokio::test(flavor = "multi_thread")]
async fn the_read_after_a_write_reports_head_even_where_it_stayed() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    repo.write_file("f.txt", "1\n");
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;
    let before = heads_of(&sink.events.lock().unwrap())
        .last()
        .map(|(_, seq)| *seq)
        .expect("the opening reported HEAD");

    // Staging reads the tree and nothing else behind it (`AfterWrite::Tree`).
    session.stage_paths(vec!["f.txt".to_string()]);
    let error = write_result(&sink, OperationKind::Stage).await;
    assert_eq!(error, None);
    let owed =
        owed_by(&sink.events.lock().unwrap(), OperationKind::Stage).expect("the write answered");
    assert!(
        owed > before,
        "the answer names a number above every report before the write"
    );
    let settled = sink
        .wait_for("the read behind the write to report", |evs| {
            heads_of(evs).into_iter().find(|(_, seq)| *seq > before)
        })
        .await;
    assert!(
        settled.1 >= owed,
        "the report after the write carries the number its answer named, or one above it"
    );
    let beside = sink
        .wait_for("the status read behind the write", |evs| {
            evs.iter().rev().find_map(|e| match e {
                SessionEvent::StatusLoaded { head_seq, .. } if *head_seq >= owed => Some(*head_seq),
                _ => None,
            })
        })
        .await;
    assert_eq!(beside, settled.1, "the status stands beside that report");
    session.close();
}

/// A consumer woken for an unchanged record would be woken for nothing.
#[tokio::test(flavor = "multi_thread")]
async fn a_quiet_tick_reports_nothing_of_head() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "0\n", "root");
    let (sink, session) = opened(&repo).await;
    sink.opening_settled(&session).await;
    let before = heads_of(&sink.events.lock().unwrap()).len();

    let outcome = bounded("the tracked poll", session.refresh_poll_tracked().outcome()).await;
    assert_eq!(outcome, RefreshOutcome::Unchanged);
    assert_eq!(
        heads_of(&sink.events.lock().unwrap()).len(),
        before,
        "nothing moved, nothing said"
    );
    session.close();
}

/// Read beside the plan's rows, and again on request, echoing the range it
/// is about.
#[tokio::test(flavor = "multi_thread")]
async fn a_plan_opens_with_the_count_a_remote_already_has() {
    let (_bare, mut work) = origin_and_clone();
    let first = work.commit_file_id("b.txt", "two\n", "local one");
    work.commit_file("c.txt", "three\n", "local two");
    work.git(&["push", "origin", "main"]);
    work.commit_file("d.txt", "four\n", "local three");
    let (sink, session) = opened(&work).await;
    sink.opening_settled(&session).await;

    session.ask_rebase_plan(first.clone());
    let preview = sink
        .wait_for("the plan", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::RebasePlanLoaded { preview, .. } if preview.from == first => {
                    Some(preview.clone())
                }
                _ => None,
            })
        })
        .await;
    assert_eq!(preview.rows.len(), 3);
    assert_eq!(
        preview.published, 2,
        "two of the three were pushed before the last commit"
    );

    session.check_plan_published(preview.range.clone());
    let again = sink
        .wait_for("the count asked again", |evs| {
            evs.iter().find_map(|e| match e {
                SessionEvent::PlanPublished { range, published } if *range == preview.range => {
                    Some(*published)
                }
                _ => None,
            })
        })
        .await;
    assert_eq!(again, 2);
    session.close();
}
