//! What still holds the branch tip once the branch moves off it, against
//! real git.
//!
//! A misspelled exclusion (`reachable::command`) answers "held" too
//! readily, and silently — it looks like a repository where everything is
//! safe.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::expect_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::reachable;

/// Three commits on `main`, plus its tip — the tests move every ref but
/// `main`, so the tip is the same commit at every ask.
fn scenario() -> (TestRepo, String) {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "first");
    repo.commit_file("a.txt", "2\n", "second");
    let tip = repo.commit_file_id("a.txt", "3\n", "third");
    (repo, tip)
}

async fn reached(repo: &TestRepo, tip: &str) -> bool {
    reached_without(repo, tip, &["main"]).await
}

async fn reached_without(repo: &TestRepo, tip: &str, moving: &[&str]) -> bool {
    let (executor, cancel) = env();
    let moving: Vec<String> = moving.iter().map(ToString::to_string).collect();
    reachable::reached_without(&executor, &repo.path, tip, &moving, &cancel)
        .await
        .expect("ask what holds the tip")
}

/// The opening ask is the exclusion's own case and doubles as the no-stash
/// case: with no `refs/stash`, the walk has to survive naming it anyway
/// (rules-refs/core.md「`--glob=<pattern>` は階層にしか当たらない」).
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

    // Tags are not in the walk (`reachable`'s module doc): one on the tip is
    // caught by `reachable::OnTip`, one strictly ahead is missed.
    repo.git(&["tag", "v2", "keep"]);
    repo.git(&["branch", "-D", "keep"]);
    assert!(
        !reached(&repo, &tip).await,
        "a tag beyond the tip is not what this walk looks at"
    );
    repo.git(&["tag", "-d", "v2"]);

    // The case that decides the rule: a branch part-way up the range saves
    // only what is below it.
    repo.git(&["branch", "keep", "HEAD~1"]);
    assert!(
        !reached(&repo, &tip).await,
        "a branch inside the range does not hold the tip"
    );
}

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

/// Under `rebase.updateRefs` the branches on the tip move with HEAD's, and
/// are left out of the walk beside it; what is further along still holds.
#[tokio::test]
async fn the_branches_a_rewrite_takes_along_are_left_out_of_the_walk() {
    let (mut repo, tip) = scenario();
    repo.git(&["branch", "keep"]);
    assert!(reached_without(&repo, &tip, &["main"]).await);
    assert!(
        !reached_without(&repo, &tip, &["main", "keep"]).await,
        "with both left out, nothing else is there"
    );

    repo.git(&["switch", "--quiet", "--create", "ahead"]);
    repo.commit_file("b.txt", "1\n", "beyond");
    repo.git(&["switch", "--quiet", "main"]);
    assert!(
        reached_without(&repo, &tip, &["main", "keep"]).await,
        "a branch further along is not one the rewrite moves"
    );
}

/// git's own side of reading the setting; that the command line asks for a
/// boolean is held by `reachable`'s unit tests (rules-refs/core.md
/// `periodic`).
mod periodic {
    use super::*;

    /// Read as git reads a boolean; anything else answers false.
    #[tokio::test]
    #[ignore = "git's boolean spellings behind a fixed command line: not worth the pre-merge run"]
    async fn the_setting_is_read_as_git_reads_a_boolean() {
        let (mut repo, _) = scenario();
        let (executor, cancel) = env();
        let along = async |repo: &TestRepo| {
            reachable::rewrites_take_branches_along(&executor, &repo.path, &cancel)
                .await
                .expect("read the setting")
        };
        assert!(!along(&repo).await, "unset is git's default");
        for truthy in ["true", "yes", "on", "1"] {
            repo.git(&["config", "rebase.updateRefs", truthy]);
            assert!(along(&repo).await, "{truthy:?}");
        }
        for falsy in ["false", "no", "0", "banana"] {
            repo.git(&["config", "rebase.updateRefs", falsy]);
            assert!(!along(&repo).await, "{falsy:?}");
        }
    }
}
