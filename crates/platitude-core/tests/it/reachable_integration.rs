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
use crate::support::exec::env;
use platitude_core::reachable;

/// Three commits on `main` and nothing else, plus `main`'s tip — the tests
/// below move every ref but `main`, so the tip is the same commit at every
/// ask.
fn scenario() -> (TestRepo, String) {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "first");
    repo.commit_file("a.txt", "2\n", "second");
    let tip = repo.commit_file_id("a.txt", "3\n", "third");
    (repo, tip)
}

async fn reached(repo: &TestRepo, tip: &str) -> bool {
    let (executor, cancel) = env();
    reachable::reached_without_branch(&executor, &repo.path, tip, "main", &cancel)
        .await
        .expect("ask what holds the tip")
}

/// One history, the walk asked again after each move of a second branch.
/// The opening ask is the exclusion's own case — left in the walk, the
/// branch holds its own tip and every repository reads as safe — and
/// doubles as the no-stash case: a repository that never stashed has no
/// `refs/stash`, and the walk has to survive naming it anyway (that is why
/// it is spelled `--glob=refs/stash*` — core.md).
#[tokio::test]
async fn only_a_branch_at_or_beyond_the_tip_holds_it() {
    let (mut repo, tip) = scenario();
    assert!(
        !reached(&repo, &tip).await,
        "a branch alone on its tip holds it alone"
    );

    repo.git(&["branch", "keep"]);
    assert!(
        reached(&repo, &tip).await,
        "another branch on the tip holds it"
    );

    repo.git(&["switch", "keep"]);
    repo.commit_file("b.txt", "1\n", "beyond");
    repo.git(&["switch", "main"]);
    assert!(
        reached(&repo, &tip).await,
        "a branch further along holds the tip too"
    );

    // A tag is not in the walk at all (they are the bulk of the refs on a
    // large repository and the bulk of the cost). One sitting on the tip
    // is caught by the refs listing instead — see
    // `reachable::a_ref_sits_on_head` — and one strictly ahead of the tip
    // is missed, which shows the mark on a row that could have been a
    // click.
    repo.git(&["tag", "v2", "keep"]);
    repo.git(&["branch", "-D", "keep"]);
    assert!(
        !reached(&repo, &tip).await,
        "a tag beyond the tip is not what this walk looks at"
    );
    repo.git(&["tag", "-d", "v2"]);

    // The case that decides the whole rule: a branch part-way up the
    // range saves what is below it and nothing above, so the tip is
    // still lost.
    repo.git(&["branch", "keep", "HEAD~1"]);
    assert!(
        !reached(&repo, &tip).await,
        "a branch inside the range does not hold the tip"
    );
}

/// The other two kinds of ref in the walk, on the same one history.
#[tokio::test]
async fn remote_tracking_refs_and_the_stash_hold_the_tip_like_branches() {
    let (mut repo, tip) = scenario();
    let behind = repo.git(&["rev-parse", "HEAD~1"]);

    repo.git(&["update-ref", "refs/remotes/origin/main", &tip]);
    assert!(
        reached(&repo, &tip).await,
        "a remote-tracking ref on the tip holds it"
    );

    repo.git(&["update-ref", "refs/remotes/origin/main", &behind]);
    assert!(
        !reached(&repo, &tip).await,
        "a remote-tracking ref left behind holds nothing"
    );

    repo.write_file("a.txt", "dirty\n");
    repo.git(&["stash"]);
    assert!(
        reached(&repo, &tip).await,
        "a stash made on the tip holds it"
    );
}
