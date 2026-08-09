//! What still holds the branch tip once the branch moves off it, against
//! real git.
//!
//! Every case here was measured before it was written: the walk answers
//! "held" far too readily if its exclusion pattern is spelled the way the
//! rest of the code spells refnames, and that failure is silent — it looks
//! exactly like a repository where everything is safe.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::expect_used)]

use crate::support::TestRepo;
use platitude_core::GitExecutor;
use platitude_core::reachable;
use tokio_util::sync::CancellationToken;

/// Three commits on `main` and nothing else.
fn scenario() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "first");
    repo.commit_file("a.txt", "2\n", "second");
    repo.commit_file("a.txt", "3\n", "third");
    repo
}

async fn reached(repo: &mut TestRepo) -> bool {
    let tip = repo.git(&["rev-parse", "HEAD"]);
    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();
    reachable::reached_without_branch(&executor, &repo.path, &tip, "main", &cancel)
        .await
        .expect("ask what holds the tip")
}

/// Doubles as the no-stash case: a repository that never stashed has no
/// `refs/stash`, and the walk has to survive naming it anyway (that is
/// why it is spelled `--glob=refs/stash*` — core.md).
#[tokio::test]
async fn a_branch_alone_on_its_tip_holds_it_alone() {
    let mut repo = scenario();
    assert!(!reached(&mut repo).await);
}

#[tokio::test]
async fn another_branch_on_the_tip_holds_it() {
    let mut repo = scenario();
    repo.git(&["branch", "keep"]);
    assert!(reached(&mut repo).await);
}

#[tokio::test]
async fn a_branch_further_along_holds_the_tip_too() {
    let mut repo = scenario();
    repo.git(&["branch", "keep"]);
    repo.git(&["switch", "keep"]);
    repo.commit_file("b.txt", "1\n", "beyond");
    repo.git(&["switch", "main"]);
    assert!(reached(&mut repo).await);
}

/// The case that decides the whole rule: a branch part-way up the range
/// saves what is below it and nothing above, so the tip is still lost.
#[tokio::test]
async fn a_branch_inside_the_range_does_not_hold_the_tip() {
    let mut repo = scenario();
    repo.git(&["branch", "keep", "HEAD~1"]);
    assert!(!reached(&mut repo).await);
}

#[tokio::test]
async fn a_remote_tracking_ref_on_the_tip_holds_it() {
    let mut repo = scenario();
    let tip = repo.git(&["rev-parse", "HEAD"]);
    repo.git(&["update-ref", "refs/remotes/origin/main", &tip]);
    assert!(reached(&mut repo).await);
}

#[tokio::test]
async fn a_remote_tracking_ref_left_behind_holds_nothing() {
    let mut repo = scenario();
    let behind = repo.git(&["rev-parse", "HEAD~1"]);
    repo.git(&["update-ref", "refs/remotes/origin/main", &behind]);
    assert!(!reached(&mut repo).await);
}

#[tokio::test]
async fn a_stash_made_on_the_tip_holds_it() {
    let mut repo = scenario();
    repo.write_file("a.txt", "dirty\n");
    repo.git(&["stash"]);
    assert!(reached(&mut repo).await);
}

/// The exclusion is what makes the question mean anything: without it the
/// branch negates its own tip and every repository reads as safe.
#[tokio::test]
async fn the_branch_being_asked_about_is_left_out_of_the_walk() {
    let mut repo = scenario();
    let tip = repo.git(&["rev-parse", "HEAD"]);
    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();

    let excluded = reachable::reached_without_branch(&executor, &repo.path, &tip, "main", &cancel)
        .await
        .expect("ask with main left out");
    let kept = reachable::reached_without_branch(&executor, &repo.path, &tip, "", &cancel)
        .await
        .expect("ask with nothing left out");

    assert!(!excluded, "main must not hold its own tip");
    assert!(kept, "with nothing excluded, main holds it");
}

/// A tag is not in the walk at all (they are the bulk of the refs on a
/// large repository and the bulk of the cost). One sitting on the tip is
/// caught by the refs listing instead — see `reachable::a_ref_sits_on_head`
/// — and one strictly ahead of the tip is missed, which shows the mark on
/// a row that could have been a click.
#[tokio::test]
async fn a_tag_beyond_the_tip_is_not_what_this_walk_looks_at() {
    let mut repo = scenario();
    repo.git(&["branch", "keep"]);
    repo.git(&["switch", "keep"]);
    repo.commit_file("b.txt", "1\n", "beyond");
    repo.git(&["tag", "v2"]);
    repo.git(&["switch", "main"]);
    repo.git(&["branch", "-D", "keep"]);
    assert!(!reached(&mut repo).await);
}
