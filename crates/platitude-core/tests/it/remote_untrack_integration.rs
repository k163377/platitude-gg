//! The upstream a remote branch delete takes off the branches here
//! (`remote::untrack_deleted`): every one tracking the name on the remote
//! the delete went to, as far as this repository's own file holds it — and
//! a refusal there is the command log's, not the write's.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::remote::origin_and_clone;
use crate::support::session::{opened, write_answer};
use platitude_core::process::CommandEnd;
use platitude_core::remote;
use platitude_core::session::SessionEvent;

/// A `file://` remote answers instantly; the budget just has to exist.
const NET: std::time::Duration = remote::DEFAULT_NETWORK_TIMEOUT;

/// `topic` pushed to `origin` and tracked here, plus `copy` tracking the
/// same name and `main` tracking its own; the tip as the screen shows it.
fn tracked_topic(work: &mut TestRepo) -> String {
    work.git(&["switch", "-c", "topic"]);
    work.commit_file("topic.txt", "ours\n", "our work");
    work.git(&["push", "-u", "origin", "topic"]);
    work.git(&["switch", "main"]);
    work.git(&["branch", "--track", "copy", "origin/topic"]);
    work.git(&["branch", "--set-upstream-to=origin/main", "main"]);
    work.git(&["rev-parse", "origin/topic"])
}

/// Deletes `origin/topic` and takes the upstreams off, as the session does.
async fn delete_and_untrack(work: &TestRepo, shown: &str) -> Vec<String> {
    let (exec, cancel) = env();
    remote::delete_remote_branch(&exec, &work.path, "origin", "topic", shown, NET, &cancel)
        .await
        .expect("delete remote branch");
    let mut untracked = remote::untrack_deleted(&exec, &work.path, "origin", "topic", &cancel)
        .await
        .expect("untrack");
    untracked.sort();
    untracked
}

fn tracks(work: &mut TestRepo, name: &str) -> bool {
    work.git_ok(&["config", &format!("branch.{name}.merge")])
}

/// Every branch here that tracked the deleted name loses its upstream, so
/// none is left warning that it is gone; one tracking another name keeps
/// its own.
#[tokio::test]
async fn a_delete_takes_the_upstream_off_what_tracked_it() {
    let (_bare, mut work) = origin_and_clone();
    let shown = tracked_topic(&mut work);

    assert_eq!(delete_and_untrack(&work, &shown).await, ["copy", "topic"]);
    assert!(!tracks(&mut work, "copy") && !tracks(&mut work, "topic"));
    assert_eq!(
        work.git(&["config", "branch.main.merge"]),
        "refs/heads/main"
    );
}

/// Judged by the push alone: a `pushurl` naming another repository than
/// the fetch URL takes the delete there, which is where these branches
/// push, so they lose the upstream although the fetch side still holds
/// the name.
#[tokio::test]
async fn a_delete_where_the_push_goes_elsewhere_still_untracks() {
    let (mut bare, mut work) = origin_and_clone();
    let (elsewhere, _seed) = origin_and_clone();
    let shown = tracked_topic(&mut work);
    work.git(&["push", &elsewhere.file_url(), "topic"]);
    work.git(&[
        "remote",
        "set-url",
        "--push",
        "origin",
        &elsewhere.file_url(),
    ]);

    assert_eq!(delete_and_untrack(&work, &shown).await, ["copy", "topic"]);
    assert_eq!(
        bare.git(&["rev-parse", "topic"]),
        shown,
        "the fetch side was not reached"
    );
}

/// What this repository's file says is taken off only where git goes by
/// it: an upstream inherited from an included file is left, and so is one
/// written here that an include read after it points at another remote —
/// its upstream is that one. The rest still come off.
#[tokio::test]
async fn only_the_upstream_this_file_holds_and_git_goes_by_comes_off() {
    let (_bare, mut work) = origin_and_clone();
    let shown = tracked_topic(&mut work);
    work.git(&["branch", "inherited", "main"]);
    work.git(&["branch", "overridden", "main"]);
    work.git(&["config", "branch.overridden.remote", "origin"]);
    work.git(&["config", "branch.overridden.merge", "refs/heads/topic"]);
    std::fs::write(
        work.path.join(".git").join("inherited.cfg"),
        "[branch \"inherited\"]\n\tremote = origin\n\tmerge = refs/heads/topic\n\
         [branch \"overridden\"]\n\tremote = elsewhere\n",
    )
    .expect("write the included file");
    work.git(&["config", "--local", "include.path", "inherited.cfg"]);

    assert_eq!(delete_and_untrack(&work, &shown).await, ["copy", "topic"]);
    assert!(tracks(&mut work, "inherited") && tracks(&mut work, "overridden"));
}

/// A `[branch "HEAD"]` section tracking the deleted name loses its keys,
/// and nothing else does: `branch --unset-upstream HEAD` would have taken
/// the checked-out branch's own upstream instead.
#[tokio::test]
async fn a_section_named_head_is_not_the_checked_out_branch() {
    let (_bare, mut work) = origin_and_clone();
    let shown = tracked_topic(&mut work);
    work.git(&["config", "branch.HEAD.remote", "origin"]);
    work.git(&["config", "branch.HEAD.merge", "refs/heads/topic"]);

    assert_eq!(
        delete_and_untrack(&work, &shown).await,
        ["HEAD", "copy", "topic"]
    );
    assert_eq!(
        work.git(&["rev-parse", "--abbrev-ref", "main@{upstream}"]),
        "origin/main",
        "the branch checked out keeps its own"
    );
}

/// `Delete both` takes the upstream off the other branches that tracked
/// the remote half, as the remote delete alone does.
#[tokio::test(flavor = "multi_thread")]
async fn delete_both_takes_the_upstream_off_the_branches_left_here() {
    let (_bare, mut work) = origin_and_clone();
    let shown = tracked_topic(&mut work);
    let (sink, session) = opened(&work).await;
    sink.opening_settled(&session).await;

    let id = session
        .delete_branch_everywhere("topic".into(), "origin".into(), "topic".into(), true, shown)
        .expect("accepted");
    assert_eq!(write_answer(&sink, id).await, None);
    session.close_and_wait().await;

    assert!(!tracks(&mut work, "copy"));
    assert_eq!(
        work.git(&["config", "branch.main.merge"]),
        "refs/heads/main"
    );
}

/// The upstream can refuse to come off after the delete went — the config
/// file held by another program. The write still answers as done, and the
/// refused command stands in the log as one the reader asked for, under
/// the write's id.
#[tokio::test(flavor = "multi_thread")]
async fn an_upstream_that_will_not_come_off_is_the_logs_not_the_writes() {
    let (mut bare, mut work) = origin_and_clone();
    let shown = tracked_topic(&mut work);
    let (sink, session) = opened(&work).await;
    sink.opening_settled(&session).await;
    let lock = work.path.join(".git").join("config.lock");
    std::fs::write(&lock, "").expect("hold the config file");

    let id = session
        .delete_remote_branch("origin".into(), "topic".into(), shown)
        .expect("accepted");
    let answer = write_answer(&sink, id).await;
    std::fs::remove_file(&lock).expect("let the config file go");

    assert_eq!(answer, None, "the delete went");
    assert!(!bare.git_ok(&["rev-parse", "--verify", "refs/heads/topic"]));
    assert!(tracks(&mut work, "topic"), "the upstream stayed");
    let (asked, operation, end) = {
        let events = sink.events.lock().unwrap();
        let (unset, asked, operation) = events
            .iter()
            .find_map(|e| match e {
                SessionEvent::CommandStarted {
                    id: command,
                    display,
                    asked,
                    operation,
                    ..
                } if display.contains("--unset-all") => Some((*command, *asked, *operation)),
                _ => None,
            })
            .expect("the unset was tried");
        let end = events.iter().find_map(|e| match e {
            SessionEvent::CommandFinished {
                id: command, end, ..
            } if *command == unset => Some(*end),
            _ => None,
        });
        (asked, operation, end)
    };
    assert_eq!((asked, operation), (true, Some(id)));
    assert!(
        matches!(end, Some(CommandEnd::Exited(code)) if code != 0),
        "{end:?}"
    );
    session.close_and_wait().await;
}
