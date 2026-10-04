//! The discard log's picked entry on the graph (破棄記録仕様.md): the walk
//! takes its tips, the commits only those tips reach come out
//! provisional with every edge they send dashed, and taking the entry off
//! walks them away again.

use std::collections::{HashMap, HashSet};

use crate::support::TestRepo;
use crate::support::session::open_unawaited;
use platitude_core::Oid;
use platitude_core::discards::{self, CopyOf, CopyOperation, Stands};
use platitude_core::graph::SegmentKind;
use platitude_core::session::{LogRow, SessionEvent, ShownDiscard};

fn oid(hex: &str) -> Oid {
    Oid::from_hex_str(hex.trim()).expect("an oid")
}

fn subjects(rows: &std::collections::BTreeMap<u32, LogRow>) -> Vec<String> {
    rows.values().map(|row| row.subject.clone()).collect()
}

/// The commits drawn, the uncommitted row (its id all zero) left out.
fn drawn(rows: &std::collections::BTreeMap<u32, LogRow>) -> Vec<Oid> {
    rows.values()
        .map(|row| oid(&row.oid_hex))
        .filter(|oid| !Oid::hex_is_zero(&oid.to_hex()))
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_picked_entry_walks_what_only_its_tip_reaches_dashed() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    let c2 = repo.commit_file_id("a.txt", "2\n", "c2");
    let c3 = repo.commit_file_id("a.txt", "3\n", "c3");
    repo.git(&["reset", "--hard", "HEAD~2"]);

    let (sink, session) = open_unawaited(&repo);
    sink.wait_for("the window without what the reset took", |evs| {
        let rows = crate::support::replay_rows(evs);
        (subjects(&rows) == ["c1"]).then_some(())
    })
    .await;

    let lost: HashSet<Oid> = [oid(&c3), oid(&c2)].into_iter().collect();
    session.show_discard(Some(ShownDiscard {
        tips: vec![oid(&c3)],
        lost,
        stashlike: HashSet::new(),
        stands: HashMap::new(),
    }));
    let rows = sink
        .wait_for("what the reset took, on the graph", |evs| {
            let rows = crate::support::replay_rows(evs);
            (subjects(&rows) == ["c3", "c2", "c1"]).then_some(rows)
        })
        .await;
    // The walk says it landed, for the entry it took: the screen then knows a
    // tip still undrawn is past the window.
    sink.wait_for("the entry's walk landed", |evs| {
        evs.iter()
            .any(
                |e| matches!(e, SessionEvent::DiscardWalked { tip: Some(tip) } if *tip == oid(&c3)),
            )
            .then_some(())
    })
    .await;
    for row in rows.values() {
        let taken = row.subject != "c1";
        assert_eq!(row.provisional, taken, "{}", row.subject);
        let sent: Vec<bool> = row
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::OutOfNode)
            .map(|s| s.dashed)
            .collect();
        assert!(
            sent.iter().all(|dashed| *dashed == taken),
            "{}: {sent:?}",
            row.subject
        );
    }

    // The same entry again — another pick with the same tips — is answered
    // too: the screen waits on that answer before its presses go live.
    let walked = |evs: &[SessionEvent]| {
        evs.iter()
            .filter(
                |e| matches!(e, SessionEvent::DiscardWalked { tip: Some(tip) } if *tip == oid(&c3)),
            )
            .count()
    };
    let before = walked(&sink.events.lock().expect("the events"));
    session.show_discard(Some(ShownDiscard {
        tips: vec![oid(&c3)],
        lost: [oid(&c3), oid(&c2)].into_iter().collect(),
        stashlike: HashSet::new(),
        stands: HashMap::new(),
    }));
    sink.wait_for("the same entry's walk landed again", |evs| {
        (walked(evs) > before).then_some(())
    })
    .await;

    session.show_discard(None);
    sink.wait_for("the entry put down, its commits walked away", |evs| {
        let rows = crate::support::replay_rows(evs);
        (subjects(&rows) == ["c1"]).then_some(())
    })
    .await;
    session.close();
}

/// A hunk thrown out of `f.txt`, its other hunk left: the copy, whose base
/// is the file as the discard left it, a commit made for it on HEAD.
async fn hunk_copy(repo: &mut TestRepo) -> (String, String) {
    let lines = |kept: &[usize]| -> String {
        (1..=20)
            .map(|n| {
                if kept.contains(&n) {
                    format!("edit {n}\n")
                } else {
                    format!("line {n}\n")
                }
            })
            .collect()
    };
    repo.commit_file("f.txt", &lines(&[]), "c1");
    repo.write_file("f.txt", &lines(&[3, 17]));
    let (workdir, dir) = (repo.path.clone(), repo.path.join(".git"));
    let tracked = ["f.txt".to_string()];
    let of = CopyOf {
        workdir: &workdir,
        git_dir: &dir,
        tracked: &tracked,
        untracked: &[],
        operation: CopyOperation::Hunk,
        touches_index: false,
        moved: None,
    };
    let (executor, cancel) = crate::support::exec::env();
    let copied = discards::copy_work(&executor, &of, &cancel)
        .await
        .expect("copy")
        .expect("work to copy");
    repo.write_file("f.txt", &lines(&[17]));
    assert!(
        discards::record_copy(&executor, &workdir, &dir, &copied, true, &cancel)
            .await
            .expect("record")
    );
    let copy = repo.git(&["rev-parse", discards::RECORD_REF]);
    let head = repo.git(&["rev-parse", "HEAD"]);
    (copy, head)
}

/// A copy whose base is a commit made for it draws on the commit that base
/// was made on — as the uncommitted row it was — and the base draws no row.
#[tokio::test(flavor = "multi_thread")]
async fn a_copy_on_a_base_of_its_own_draws_on_head() {
    let mut repo = TestRepo::init();
    let (copy, head) = hunk_copy(&mut repo).await;
    let made = oid(&repo.git(&["rev-parse", &format!("{copy}^1")]));
    assert_ne!(made, oid(&head), "the base is a commit of its own");

    let (sink, session) = open_unawaited(&repo);
    sink.wait_for("the window", |evs| {
        let rows = crate::support::replay_rows(evs);
        (drawn(&rows) == [oid(&head)]).then_some(())
    })
    .await;
    session.show_discard(Some(ShownDiscard {
        tips: vec![oid(&copy)],
        lost: [oid(&copy)].into_iter().collect(),
        stashlike: [oid(&copy)].into_iter().collect(),
        stands: [(
            oid(&copy),
            Stands {
                made,
                on: oid(&head),
            },
        )]
        .into_iter()
        .collect(),
    }));
    let rows = sink
        .wait_for("the copy on HEAD, its base no row", |evs| {
            let rows = crate::support::replay_rows(evs);
            drawn(&rows).contains(&oid(&copy)).then_some(rows)
        })
        .await;
    assert_eq!(drawn(&rows), vec![oid(&copy), oid(&head)]);
    let row = rows
        .values()
        .find(|row| oid(&row.oid_hex) == oid(&copy))
        .expect("the copy's row");
    assert_eq!(row.parents.to_vec(), vec![oid(&head)]);
    session.close();
}

/// A copy that went to the stashes keeps its base: the stash draws on the
/// commit the base was made on, and the base draws no row.
#[tokio::test(flavor = "multi_thread")]
async fn a_stash_on_a_base_of_its_own_draws_on_head() {
    let mut repo = TestRepo::init();
    let (copy, head) = hunk_copy(&mut repo).await;
    repo.git(&["stash", "store", "-m", "kept", &copy]);

    let (sink, session) = open_unawaited(&repo);
    let rows = sink
        .wait_for("the stash on HEAD, its base no row", |evs| {
            let rows = crate::support::replay_rows(evs);
            drawn(&rows).contains(&oid(&copy)).then_some(rows)
        })
        .await;
    assert_eq!(drawn(&rows), vec![oid(&copy), oid(&head)]);
    session.close();
}
