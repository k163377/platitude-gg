//! A branch goes out under its full name: beside a tag of the same name
//! the bare name is ambiguous, which git refuses (`src refspec <name>
//! matches more than one`), and the tag must not ride along either. Every
//! door that sends a branch here — the first push's answer, the toolbar's
//! push, a branch row's — against real git, with a `file://` remote so
//! nothing leaves the machine (実装計画 §10). A send of a remote-tracking
//! ref keeps the source its caller names (`remote_integration`'s
//! replacements).

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::remote::origin_and_clone;
use platitude_core::remote::{self, PushForce};

/// A `file://` remote answers instantly; the budget just has to exist.
const NET: std::time::Duration = remote::DEFAULT_NETWORK_TIMEOUT;

/// `topic` with a commit of its own, beside a tag `topic` on the root —
/// another commit, so a send that took the tag shows in where the far
/// side's names stand.
fn topic_beside_its_tag() -> (TestRepo, TestRepo) {
    let (bare, mut work) = origin_and_clone();
    work.git(&["tag", "topic"]);
    work.git(&["switch", "-c", "topic"]);
    work.commit_file("b.txt", "b\n", "topic work");
    (bare, work)
}

/// `topic` already sent and tracked, with one more commit to send.
fn topic_tracked_beside_its_tag() -> (TestRepo, TestRepo) {
    let (bare, mut work) = topic_beside_its_tag();
    work.git(&[
        "push",
        "--set-upstream",
        "origin",
        "refs/heads/topic:refs/heads/topic",
    ]);
    work.commit_file("c.txt", "c\n", "more topic work");
    (bare, work)
}

/// The commits the far side holds under the branch's name and the tag's;
/// empty for a name it does not hold.
fn far_side(bare: &mut TestRepo) -> (String, String) {
    let path = bare.path.clone();
    let mut at = |full: &str| bare.git_in(&path, &["for-each-ref", "--format=%(objectname)", full]);
    (at("refs/heads/topic"), at("refs/tags/topic"))
}

/// The commit the branch here stands on.
fn branch_tip(work: &mut TestRepo) -> String {
    work.git(&["rev-parse", "refs/heads/topic"])
}

#[tokio::test]
async fn a_first_push_sends_the_branch_and_not_the_tag_of_its_name() {
    let (mut bare, mut work) = topic_beside_its_tag();
    let (exec, cancel) = env();

    let spec = remote::publish_spec("topic", "origin", "topic", "");
    remote::push(&exec, &work.path, &spec, NET, &cancel)
        .await
        .expect("publish");

    assert_eq!(far_side(&mut bare), (branch_tip(&mut work), String::new()));
    assert_eq!(
        work.git(&["config", "branch.topic.merge"]),
        "refs/heads/topic",
        "the answer is recorded as the upstream"
    );
}

/// The branch HEAD is on is named the same way: beside the tag, its short
/// name is `heads/topic`, which would plan the push of a branch that does
/// not exist and record it as the upstream.
#[tokio::test]
async fn the_toolbars_push_sends_the_branch_and_not_the_tag_of_its_name() {
    let (mut bare, mut work) = topic_tracked_beside_its_tag();
    let (exec, cancel) = env();

    let spec = remote::plan_current_push(&exec, &work.path, "origin", PushForce::None, &cancel)
        .await
        .expect("a plan");
    remote::push(&exec, &work.path, &spec, NET, &cancel)
        .await
        .expect("push");

    assert_eq!(far_side(&mut bare), (branch_tip(&mut work), String::new()));
    assert_eq!(
        work.git(&["config", "branch.topic.merge"]),
        "refs/heads/topic",
        "the upstream is left as it was"
    );
}
