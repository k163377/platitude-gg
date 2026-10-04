//! What a delete reads behind itself: the refs and the graph. Not the tree,
//! the stash list or the working copies, which no delete reaches — unless
//! the name was what HEAD is measured against (`AfterWrite::Name`).

use platitude_core::session::{Recording, SessionEvent};

use crate::support::TestRepo;
use crate::support::remote::origin_and_clone;
use crate::support::session::{CaptureSink, opened, scenario, write_settled};

/// The commands started from event `from` on, as the log shows them.
fn commands_since(sink: &CaptureSink, from: usize) -> Vec<String> {
    sink.events.lock().unwrap()[from..]
        .iter()
        .filter_map(|e| match e {
            SessionEvent::CommandStarted { display, .. } => Some(display.clone()),
            _ => None,
        })
        .collect()
}

fn times(commands: &[String], needle: &str) -> usize {
    commands.iter().filter(|c| c.contains(needle)).count()
}

/// The newest status reading the session published.
fn last_status(sink: &CaptureSink) -> platitude_core::status::WorkTreeStatus {
    sink.events
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find_map(|e| match e {
            SessionEvent::StatusLoaded { status, .. } => Some(status.clone()),
            _ => None,
        })
        .expect("the opening read the status")
}

/// Both flags, and every read behind them counted: the refs once, the walk
/// once with the stash list it walks with, and nothing else.
///
/// `branch --delete` rewrites `.git/config` whether or not the branch had a
/// section there, and the rewrite is the delete's own
/// (`RepoSession::own_config_rewrite`): the first delete reads no remotes.
/// The second does, for the ref the first one moved (`publish_refs`).
#[tokio::test(flavor = "multi_thread")]
async fn a_branch_delete_reads_the_refs_and_the_graph_and_nothing_else() {
    let (mut repo, _head) = scenario();
    repo.git(&["switch", "-c", "unmerged"]);
    repo.commit_file("u.txt", "u\n", "unmerged work");
    repo.git(&["switch", "main"]);
    let (sink, session) = opened(&repo).await;
    sink.opened_graph(&session, 6).await;
    session.set_recording(Recording::WithBackground);

    for (name, force) in [("side", false), ("unmerged", true)] {
        let from = sink.events.lock().unwrap().len();
        let id = session.delete_branch(name.into(), force).expect("accepted");
        assert_eq!(
            write_settled(&sink, id).await,
            [],
            "every read behind the delete of {name} landed"
        );
        let ran = commands_since(&sink, from);
        for (needle, expected) in [
            // `status --branch` carries the word too.
            ("git branch ", 1),
            ("for-each-ref", 1),
            ("log -z", 1),
            ("stash list", 1),
            ("status", 0),
            ("rev-parse --git-path", 0),
            ("worktree list", 0),
        ] {
            assert_eq!(
                times(&ran, needle),
                expected,
                "`{needle}` behind the delete of {name}: {ran:#?}"
            );
        }
        if name == "side" {
            // The remotes' listing; the delete's own read of the branch's
            // upstream, for the discard record, is the write's.
            assert_eq!(times(&ran, "^remote"), 0, "{ran:#?}");
        }
    }
    session.close();
}

/// Under `remote = .` HEAD is measured against a branch here: deleting it
/// takes the `# branch.ab` line away, and the status says so at once.
#[tokio::test(flavor = "multi_thread")]
async fn deleting_the_branch_head_is_measured_against_reads_the_status_again() {
    let (mut repo, _head) = scenario();
    repo.git(&["branch", "base", "HEAD~1"]);
    repo.git(&["branch", "--set-upstream-to=base"]);
    let (sink, session) = opened(&repo).await;
    sink.opened_graph(&session, 5).await;
    assert!(
        last_status(&sink).upstream_tracked,
        "the fixture measures main against base"
    );
    session.set_recording(Recording::WithBackground);

    let from = sink.events.lock().unwrap().len();
    let id = session
        .delete_branch("base".into(), true)
        .expect("accepted");
    assert_eq!(write_settled(&sink, id).await, []);

    let ran = commands_since(&sink, from);
    assert_eq!(times(&ran, "status"), 1, "{ran:#?}");
    assert!(
        !last_status(&sink).upstream_tracked,
        "the status behind the delete no longer counts against base"
    );
    session.close();
}

/// A remote branch is the same question: HEAD's own upstream is read for,
/// any other is not.
#[tokio::test(flavor = "multi_thread")]
async fn a_remote_branch_delete_reads_the_status_only_for_heads_upstream() {
    let (_bare, mut work) = origin_and_clone();
    let other = pushed(&mut work, "other");
    let topic = pushed(&mut work, "topic");
    work.git(&["switch", "topic"]);
    let (sink, session) = opened(&work).await;
    sink.opening_settled(&session).await;
    session.set_recording(Recording::WithBackground);

    for (branch, shown, reads) in [("other", other, 0), ("topic", topic, 1)] {
        let from = sink.events.lock().unwrap().len();
        let id = session
            .delete_remote_branch("origin".into(), branch.into(), shown)
            .expect("accepted");
        assert_eq!(write_settled(&sink, id).await, []);
        let ran = commands_since(&sink, from);
        assert_eq!(
            times(&ran, "status"),
            reads,
            "the delete of origin/{branch}: {ran:#?}"
        );
    }
    session.close();
}

/// A branch of its own on `origin`, tracked here; the commit it went up
/// on.
fn pushed(work: &mut TestRepo, name: &str) -> String {
    work.git(&["switch", "-c", name, "main"]);
    work.commit_file(&format!("{name}.txt"), "ours\n", name);
    work.git(&["push", "-u", "origin", name]);
    work.git(&["switch", "main"]);
    work.git(&["rev-parse", &format!("origin/{name}")])
}
