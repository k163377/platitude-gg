//! A delete takes the commits only its name held off the graph when it is
//! accepted, and a refused one puts them back when git answers
//! (`session::leaving`; デザイン規約 §消す操作は先に画面から消す).
//!
//! Where the order against git's answer is the claim, git is held inside
//! the delete (a hook waiting on a file), so what the graph did is seen
//! while the answer provably has not come.

use platitude_core::session::{FollowUp, PassStep, RefreshOutcome, RelaidRow, SessionEvent};

use crate::support::remote::origin_and_clone;
use crate::support::session::{
    CaptureSink, open_with_doors, opened, scenario, write_answer, write_settled,
};
use crate::support::{TestRepo, barrier_hook};

/// A branch off main with one commit nothing else holds; its id.
fn off_main(repo: &mut TestRepo, name: &str) -> String {
    repo.git(&["switch", "-c", name]);
    let tip = repo.commit_file_id(&format!("{name}.txt"), "u\n", &format!("{name} work"));
    repo.git(&["switch", "main"]);
    tip
}

/// The file a hook's barrier waits for, where no status reads it.
fn release_of(repo: &TestRepo) -> std::path::PathBuf {
    repo.path.join(".git").join("hook-release")
}

/// The rows of the `nth` graph laid out again (`LogRelaid`), by id.
async fn relaid(sink: &CaptureSink, nth: usize) -> Vec<String> {
    sink.wait_for("the graph laid out again", |evs| {
        evs.iter()
            .filter_map(|e| match e {
                SessionEvent::LogRelaid { rows, .. } => Some(rows),
                _ => None,
            })
            .nth(nth)
            .map(|rows| {
                rows.iter()
                    .map(|row| match row {
                        RelaidRow::Moved { oid, .. } => oid.to_hex(),
                        RelaidRow::Made(row) => row.oid_hex.clone(),
                    })
                    .collect()
            })
    })
    .await
}

fn answered(sink: &CaptureSink, id: platitude_core::OperationId) -> bool {
    sink.count(|e| matches!(e, SessionEvent::WriteFinished { id: got, .. } if *got == id)) > 0
}

fn replaced(sink: &CaptureSink) -> usize {
    sink.count(|e| matches!(e, SessionEvent::LogReplaced { .. }))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_forced_delete_takes_what_only_it_held_off_the_graph_before_git_answers() {
    let (mut repo, _head) = scenario();
    let tip = off_main(&mut repo, "unmerged");
    let release = release_of(&repo);
    repo.write_hook("reference-transaction", &barrier_hook(&release));
    let (sink, session) = opened(&repo).await;
    sink.opened_graph(&session, 6).await;
    let swaps = replaced(&sink);

    let id = session
        .delete_branch("unmerged".into(), true)
        .expect("accepted");
    let rows = relaid(&sink, 0).await;
    assert!(!answered(&sink, id), "git was still held inside the delete");
    assert_eq!(rows.len(), 5, "{rows:#?}");
    assert!(
        !rows.contains(&tip),
        "the branch's own commit is still drawn"
    );

    std::fs::write(&release, b"go").expect("release the hook");
    assert_eq!(write_settled(&sink, id).await, []);
    assert_eq!(
        replaced(&sink),
        swaps,
        "the walk behind the delete found the picture the stand-in drew"
    );
    session.close();
}

/// A plain delete is refused unless the tip is merged into what the graph
/// draws anyway: it takes no commit with it, and lays nothing out.
#[tokio::test(flavor = "multi_thread")]
async fn a_plain_delete_lays_nothing_out() {
    let (repo, _head) = scenario();
    let (sink, session) = opened(&repo).await;
    sink.opened_graph(&session, 5).await;

    let id = session
        .delete_branch("side".into(), false)
        .expect("accepted");
    assert_eq!(write_settled(&sink, id).await, []);
    assert_eq!(
        sink.count(|e| matches!(e, SessionEvent::LogRelaid { .. })),
        0
    );
    session.close();
}

/// Refused, the commits are back at the answer — before the reads behind
/// it, so the refusal is about rows on screen.
#[tokio::test(flavor = "multi_thread")]
async fn a_refused_delete_puts_its_commits_back_at_the_answer() {
    let (mut repo, _head) = scenario();
    let tip = off_main(&mut repo, "unmerged");
    let release = release_of(&repo);
    // Held, then aborted: git refuses the delete with the ref in place.
    let refuse = format!(
        "{}if [ \"$1\" = prepared ]; then exit 1; fi\n",
        barrier_hook(&release)
    );
    repo.write_hook("reference-transaction", &refuse);
    let (sink, session) = opened(&repo).await;
    sink.opened_graph(&session, 6).await;

    let id = session
        .delete_branch("unmerged".into(), true)
        .expect("accepted");
    assert!(!relaid(&sink, 0).await.contains(&tip));

    std::fs::write(&release, b"go").expect("release the hook");
    assert!(
        write_answer(&sink, id).await.is_some(),
        "the hook refused the delete"
    );
    let back = relaid(&sink, 1).await;
    assert_eq!(back.len(), 6, "{back:#?}");
    assert!(
        back.contains(&tip),
        "the refused delete's commit stayed away"
    );

    assert_eq!(write_settled(&sink, id).await, []);
    let events = sink.events.lock().unwrap();
    let at = |pred: &dyn Fn(&SessionEvent) -> bool| {
        events
            .iter()
            .enumerate()
            .filter(|(_, e)| pred(e))
            .map(|(at, _)| at)
            .collect::<Vec<_>>()
    };
    let answer = at(&|e| matches!(e, SessionEvent::WriteFinished { id: got, .. } if *got == id));
    let put_back = at(&|e| matches!(e, SessionEvent::LogRelaid { .. }));
    let refs = at(&|e| matches!(e, SessionEvent::RefsLoaded { .. }));
    let refs_behind = refs.iter().find(|at| **at > answer[0]);
    assert!(
        answer[0] < put_back[1] && refs_behind.is_none_or(|refs| put_back[1] < *refs),
        "the commits came back at the answer, not with the reads behind it: \
         answer {answer:?}, laid {put_back:?}, refs {refs:?}"
    );
    drop(events);
    session.close();
}

/// A walk landing while the delete is out lays the same graph: it cannot
/// bring the commits back.
#[tokio::test(flavor = "multi_thread")]
async fn a_walk_landing_while_the_delete_is_out_keeps_the_commits_off() {
    let (mut repo, _head) = scenario();
    let tip = off_main(&mut repo, "unmerged");
    let release = release_of(&repo);
    repo.write_hook("reference-transaction", &barrier_hook(&release));
    let (sink, session) = opened(&repo).await;
    sink.opened_graph(&session, 6).await;
    let swaps = replaced(&sink);

    let id = session
        .delete_branch("unmerged".into(), true)
        .expect("accepted");
    assert!(!relaid(&sink, 0).await.contains(&tip));
    let outcome = crate::support::wait::bounded(
        "a rebuild while git is held",
        session.refresh_log_tracked().outcome(),
    )
    .await;
    assert!(!answered(&sink, id), "git was still held inside the delete");
    assert_eq!(
        (outcome, replaced(&sink)),
        (RefreshOutcome::Unchanged, swaps),
        "the walk, which still sees the branch, drew it back"
    );

    std::fs::write(&release, b"go").expect("release the hook");
    assert_eq!(write_settled(&sink, id).await, []);
    session.close();
}

/// A remote branch's delete takes what only the remote-tracking ref held,
/// held on the far side until the test has looked.
#[tokio::test(flavor = "multi_thread")]
async fn a_remote_branch_delete_takes_what_only_the_remote_held() {
    let (bare, mut work) = origin_and_clone();
    let theirs = their_branch(&bare, &mut work, "theirs");
    let release = release_of(&bare);
    bare.write_hook("pre-receive", &barrier_hook(&release));
    let (sink, session) = opened(&work).await;
    sink.opening_settled(&session).await;
    crate::support::wait::bounded("the opening's graph", session.wait_for_graph_passes()).await;

    let id = session
        .delete_remote_branch("origin".into(), "theirs".into(), theirs.clone())
        .expect("accepted");
    let rows = relaid(&sink, 0).await;
    assert!(!answered(&sink, id), "the push was still held over there");
    assert!(!rows.contains(&theirs), "{rows:#?}");

    std::fs::write(&release, b"go").expect("release the hook");
    assert_eq!(write_settled(&sink, id).await, []);
    session.close();
}

/// A tag is a starting point of the walk too: deleting the one tag on a
/// commit nothing else holds takes that commit off.
#[tokio::test(flavor = "multi_thread")]
async fn a_tag_delete_takes_what_only_the_tag_held() {
    let (mut repo, _head) = scenario();
    let lone = off_main(&mut repo, "lone");
    repo.git(&["tag", "lone-tag", "lone"]);
    repo.git(&["branch", "-D", "lone"]);
    let release = release_of(&repo);
    repo.write_hook("reference-transaction", &barrier_hook(&release));
    let (sink, session) = opened(&repo).await;
    sink.opened_graph(&session, 6).await;

    let id = session.delete_tag("lone-tag".into()).expect("accepted");
    let rows = relaid(&sink, 0).await;
    assert!(!answered(&sink, id), "git was still held inside the delete");
    assert!(!rows.contains(&lone), "{rows:#?}");

    std::fs::write(&release, b"go").expect("release the hook");
    assert_eq!(write_settled(&sink, id).await, []);
    session.close();
}

/// A delete that landed while the walk behind it fell over stands on: the
/// record still holds its commit, and the graph laid out for the next
/// delete must not draw it back. The first walk to land lets it go.
#[tokio::test(flavor = "multi_thread")]
async fn a_landed_delete_whose_walk_failed_stays_off_the_graph() {
    let (mut repo, _head) = scenario();
    let first = off_main(&mut repo, "first");
    let second = off_main(&mut repo, "second");
    let (sink, session, doors) = open_with_doors(&repo);
    sink.opened_graph(&session, 7).await;

    doors.fail_every_pass(PassStep::Swapping);
    let id = session
        .delete_branch("first".into(), true)
        .expect("accepted");
    assert_eq!(
        write_settled(&sink, id).await,
        [FollowUp::Graph],
        "the walk behind the delete fell over"
    );

    let id = session
        .delete_branch("second".into(), true)
        .expect("accepted");
    let rows = relaid(&sink, 1).await;
    assert!(
        !rows.contains(&first),
        "the first delete's commit came back: {rows:#?}"
    );
    assert!(!rows.contains(&second), "{rows:#?}");
    assert_eq!(write_settled(&sink, id).await, [FollowUp::Graph]);

    doors.stop_failing();
    let outcome =
        crate::support::wait::bounded("a walk that lands", session.refresh_log_tracked().outcome())
            .await;
    assert!(outcome.landed(), "{outcome:?}");
    session.close();
}

/// A branch somebody else pushed, fetched here and held by nothing but its
/// remote-tracking ref; its commit.
fn their_branch(bare: &TestRepo, work: &mut TestRepo, name: &str) -> String {
    let mut other = TestRepo::init();
    other.git(&["remote", "add", "origin", &bare.file_url()]);
    other.git(&["fetch", "origin"]);
    other.git(&["switch", "-c", name, "origin/main"]);
    let tip = other.commit_file_id("theirs.txt", "theirs\n", "their work");
    other.git(&["push", "origin", name]);
    work.git(&["fetch", "origin"]);
    tip
}
