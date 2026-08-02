//! Refs listing and HEAD state against real git, including a local
//! file-based "remote" (no network involved).

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::expect_used)]

mod support;

use platitude_core::refs::{self, RefKind};
use platitude_core::{GitExecutor, Oid};
use support::TestRepo;
use tokio_util::sync::CancellationToken;

/// main (upstream: origin/main) + feature (no upstream, same name on
/// origin) + local-only + annotated & lightweight tags.
fn scenario() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "initial");
    repo.commit_file("a.txt", "2\n", "second");
    repo.git(&["tag", "-a", "v1.0", "-m", "release v1.0"]);
    repo.git(&["tag", "light"]);
    repo.git(&["branch", "feature/x"]);
    repo.git(&["branch", "local-only"]);

    // A bare clone acts as origin via file:// semantics (path remote).
    let remote_path = repo.path.parent().expect("parent").join("origin.git");
    let remote_str = remote_path.to_string_lossy().replace('\\', "/");
    let repo_dir = repo.path.clone();
    repo.git_in(
        repo_dir.parent().expect("parent"),
        &["clone", "--bare", "repo", "origin.git"],
    );
    repo.git(&["remote", "add", "origin", &remote_str]);
    repo.git(&["fetch", "origin"]);
    repo.git(&["branch", "-u", "origin/main", "main"]);
    // Delete local-only's counterpart scenario: origin never had it, and
    // remove feature/x's upstream config so only the name matches.
    repo.git(&["push", "origin", "--delete", "local-only"]);
    repo
}

#[tokio::test]
async fn lists_branches_tags_and_remotes() {
    let mut repo = scenario();
    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();

    let refs = refs::load(&executor, &repo.path, &cancel).await.unwrap();

    let find = |kind: RefKind, short: &str| {
        refs.iter()
            .find(|r| r.kind == kind && r.short == short)
            .unwrap_or_else(|| panic!("missing {kind:?} {short}"))
    };

    let main = find(RefKind::LocalBranch, "main");
    assert!(main.is_head);
    assert_eq!(main.upstream.as_deref(), Some("refs/remotes/origin/main"));

    let annotated = find(RefKind::Tag, "v1.0");
    assert!(
        annotated.peeled.is_some(),
        "annotated tag peels to a commit"
    );
    let light = find(RefKind::Tag, "light");
    assert!(light.peeled.is_none(), "lightweight tag needs no peel");
    assert_eq!(
        annotated.commit_oid(),
        light.commit_oid(),
        "both tags designate HEAD's commit"
    );

    let head_sha = Oid::from_hex_str(&repo.git(&["rev-parse", "HEAD"])).unwrap();
    assert_eq!(main.target, head_sha);

    find(RefKind::RemoteBranch, "origin/main");
    assert!(
        !refs
            .iter()
            .any(|r| r.kind == RefKind::RemoteBranch && r.short.ends_with("/HEAD")),
        "origin/HEAD symref must be hidden"
    );

    // Remote-state: main via upstream, feature/x via name match,
    // local-only stays local (was deleted on origin).
    let with_remote = refs::branches_with_remote(&refs);
    assert!(with_remote.contains("refs/heads/main"));
    assert!(with_remote.contains("refs/heads/feature/x"));
    assert!(!with_remote.contains("refs/heads/local-only"));
}

#[tokio::test]
async fn head_state_on_branch_and_detached() {
    let mut repo = TestRepo::init();
    let sha = repo.commit_file("a.txt", "1\n", "initial");
    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();

    let on_branch = refs::head_state(&executor, &repo.path, &cancel)
        .await
        .unwrap();
    assert_eq!(on_branch.branch.as_deref(), Some("main"));
    assert!(!on_branch.detached);
    assert_eq!(on_branch.oid, Some(Oid::from_hex_str(&sha).unwrap()));

    repo.git(&["checkout", "--detach", "HEAD"]);
    let detached = refs::head_state(&executor, &repo.path, &cancel)
        .await
        .unwrap();
    assert!(detached.detached);
    assert_eq!(detached.branch, None);
    assert_eq!(detached.oid, Some(Oid::from_hex_str(&sha).unwrap()));

    // No ref carries the HEAD marker while detached.
    let refs_list = refs::load(&executor, &repo.path, &cancel).await.unwrap();
    assert!(refs_list.iter().all(|r| !r.is_head));
}

#[tokio::test]
async fn empty_repository_has_unborn_head() {
    let repo = TestRepo::init();
    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();

    let state = refs::head_state(&executor, &repo.path, &cancel)
        .await
        .unwrap();
    assert_eq!(state.branch.as_deref(), Some("main"));
    assert_eq!(state.oid, None, "unborn branch has no commit");
    assert!(!state.detached);

    let refs_list = refs::load(&executor, &repo.path, &cancel).await.unwrap();
    assert!(refs_list.is_empty());
}
