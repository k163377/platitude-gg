//! Refs listing and HEAD state against real git, including a local
//! file-based "remote" (no network involved).
//!
//! HEAD is read in all three states, each once: the state and how the
//! command log classifies it come off the same read (`answer_reads` holds
//! the rest of that family).

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::expect_used)]

use std::collections::HashSet;

use crate::support::TestRepo;
use crate::support::exec::{env, logged};
use platitude_core::Oid;
use platitude_core::process::CommandEnd;
use platitude_core::refs::{self, RefKind};

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
    // The bare clone took every branch, so local-only is deleted there;
    // feature/x keeps a same-named branch but never gets an upstream.
    repo.git(&["push", "origin", "--delete", "local-only"]);
    repo
}

#[tokio::test]
async fn lists_branches_tags_and_remotes() {
    let mut repo = scenario();
    let (executor, cancel) = env();

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

    // Remote-state comes from the upstream alone, to agree with git
    // (`RemoteBranches::spoken_for`): git names no upstream for feature/x
    // though `origin/feature/x` is in the listing.
    find(RefKind::RemoteBranch, "origin/feature/x");
    assert_eq!(
        repo.git(&[
            "for-each-ref",
            "--format=%(upstream)",
            "refs/heads/feature/x"
        ]),
        "",
        "git names no upstream for it"
    );
    let remotes = refs::RemoteBranches::index(&refs);
    let with_remote: HashSet<&str> = refs
        .iter()
        .filter(|r| r.kind == RefKind::LocalBranch)
        .filter(|r| remotes.has_counterpart(r))
        .map(|r| r.name.as_str())
        .collect();
    assert!(with_remote.contains("refs/heads/main"));
    assert!(!with_remote.contains("refs/heads/feature/x"));
    assert!(!with_remote.contains("refs/heads/local-only"));
}

/// `symbolic-ref -q` exits 1 for "detached" and `rev-parse --verify -q
/// HEAD` for "unborn" — unmarked, each logs a failed row on every refresh
/// (rules/core.md「終了コードで答える問い合わせは」).
#[tokio::test]
async fn head_state_on_branch_and_detached_answers_by_code() {
    let mut repo = TestRepo::init();
    let sha = repo.commit_file_id("a.txt", "1\n", "initial");
    let (executor, log, cancel) = logged();

    let on_branch = refs::head_state(&executor, &repo.path, &cancel)
        .await
        .unwrap();
    assert_eq!(on_branch.branch.as_deref(), Some("main"));
    assert!(!on_branch.detached);
    assert_eq!(on_branch.oid, Some(Oid::from_hex_str(&sha).unwrap()));
    assert_eq!(
        log.ends_of(&["symbolic-ref"]),
        vec![CommandEnd::Answered(0)],
        "a branch is the plain answer"
    );

    repo.git(&["checkout", "--detach", "HEAD"]);
    let detached = refs::head_state(&executor, &repo.path, &cancel)
        .await
        .unwrap();
    assert!(detached.detached);
    assert_eq!(detached.branch, None, "detached HEAD names no branch");
    assert_eq!(
        detached.oid,
        Some(Oid::from_hex_str(&sha).unwrap()),
        "the commit is still there"
    );
    assert_eq!(
        log.ends_of(&["symbolic-ref"]),
        vec![CommandEnd::Answered(0), CommandEnd::Answered(1)],
        "detached is an answer, not a failed row"
    );
    // Counted, not `all()`: `ends_of` drops rows that never ended, so `all()`
    // would pass with an end missing, or with no read at all.
    assert_eq!(
        log.ends_of(&["rev-parse", "--verify"]),
        vec![CommandEnd::Answered(0), CommandEnd::Answered(0)],
        "the commit resolved on both reads, and both reads ended"
    );

    let refs_list = refs::load(&executor, &repo.path, &cancel).await.unwrap();
    assert!(refs_list.iter().all(|r| !r.is_head));
}

#[tokio::test]
async fn empty_repository_has_unborn_head_and_says_so_by_code() {
    let repo = TestRepo::init();
    let (executor, log, cancel) = logged();

    let state = refs::head_state(&executor, &repo.path, &cancel)
        .await
        .unwrap();
    assert_eq!(state.branch.as_deref(), Some("main"));
    assert_eq!(state.oid, None, "unborn branch has no commit");
    assert!(!state.detached);
    assert_eq!(
        log.ends_of(&["rev-parse", "--verify"]),
        vec![CommandEnd::Answered(1)],
        "unborn is an answer, not a failed row"
    );

    let refs_list = refs::load(&executor, &repo.path, &cancel).await.unwrap();
    assert!(refs_list.is_empty());
}

/// The walk names commits, not branches, so its HEAD read skips the second
/// process `head_state` runs — on Windows the launch is most of what a read
/// costs (ci/baseline/code-costs-windows-x64.md).
#[tokio::test]
async fn the_narrow_head_read_asks_for_the_commit_and_nothing_else() {
    let mut repo = TestRepo::init();
    let (executor, log, cancel) = logged();

    let unborn = refs::head_tip(&executor, &repo.path, &cancel)
        .await
        .unwrap();
    assert_eq!(unborn, None, "no commit yet");

    let sha = repo.commit_file_id("a.txt", "1\n", "initial");
    let born = refs::head_tip(&executor, &repo.path, &cancel)
        .await
        .unwrap();
    assert_eq!(born, Some(Oid::from_hex_str(&sha).unwrap()));

    assert_eq!(
        log.ends_of(&["rev-parse", "--verify"]),
        vec![CommandEnd::Answered(1), CommandEnd::Answered(0)],
        "unborn is an answer, not a failed row"
    );
    assert!(
        log.ends_of(&["symbolic-ref"]).is_empty(),
        "the branch is nobody's question here"
    );
}

/// Against real git because a misspelled placeholder reads as an empty leg:
/// every branch would look level with its upstream while the parser passed.
#[tokio::test]
async fn a_branch_counts_how_far_it_stands_from_its_upstream() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "initial");
    for branch in ["level", "ahead", "behind", "diverged"] {
        repo.git(&["branch", branch]);
    }

    let remote_path = repo.path.parent().expect("parent").join("origin.git");
    let remote_str = remote_path.to_string_lossy().replace('\\', "/");
    let repo_dir = repo.path.clone();
    repo.git_in(
        repo_dir.parent().expect("parent"),
        &["clone", "--bare", "repo", "origin.git"],
    );
    repo.git(&["remote", "add", "origin", &remote_str]);
    repo.git(&["fetch", "origin"]);
    for branch in ["level", "ahead", "behind", "diverged"] {
        repo.git(&["branch", "-u", &format!("origin/{branch}"), branch]);
    }

    repo.git(&["switch", "ahead"]);
    repo.commit_file("a.txt", "2\n", "ahead one");
    repo.commit_file("a.txt", "3\n", "ahead two");

    repo.git(&["switch", "behind"]);
    repo.commit_file("a.txt", "4\n", "theirs");
    repo.git(&["push", "origin", "behind"]);
    repo.git(&["reset", "--hard", "HEAD~1"]);

    repo.git(&["switch", "diverged"]);
    repo.commit_file("a.txt", "5\n", "theirs");
    repo.git(&["push", "origin", "diverged"]);
    repo.git(&["reset", "--hard", "HEAD~1"]);
    repo.commit_file("a.txt", "6\n", "ours");

    let (executor, cancel) = env();
    let refs = refs::load(&executor, &repo.path, &cancel).await.unwrap();
    let counts = |short: &str| {
        refs.iter()
            .find(|r| r.kind == RefKind::LocalBranch && r.short == short)
            .map(|entry| (entry.ahead, entry.behind))
            .unwrap_or_else(|| panic!("missing branch {short}"))
    };

    assert_eq!(counts("level"), (0, 0), "level with its upstream");
    assert_eq!(counts("ahead"), (2, 0), "two commits the remote has not");
    assert_eq!(counts("behind"), (0, 1), "one commit this side has not");
    assert_eq!(counts("diverged"), (1, 1), "one each way");
}
