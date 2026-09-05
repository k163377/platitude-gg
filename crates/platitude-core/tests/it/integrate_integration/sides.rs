//! What a stopped operation says about the two sides it sits between.

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::integrate::conflicting_branches;
use platitude_core::conflict;
use platitude_core::integrate::{self, InProgress, MergeOptions, RebaseOptions};

/// What each stopped operation leaves behind to name its two sides by.
/// The UI calls them by branch name, and this is what there is to build
/// those names out of — measured, because the answer differs per
/// operation and per rebase backend.
#[tokio::test]
async fn what_a_stopped_operation_says_about_its_two_sides() {
    // ---- merge: HEAD is ours, MERGE_HEAD is theirs ----
    let mut repo = conflicting_branches();
    repo.git_expect_failure(&["merge", "side"]);
    let merge_head = repo.git(&["rev-parse", "MERGE_HEAD"]);
    assert!(!merge_head.is_empty());
    assert_eq!(
        repo.git(&[
            "name-rev",
            "--name-only",
            "--refs=refs/heads/*",
            &merge_head
        ]),
        "side",
        "the branch merged in is named by its tip"
    );
    assert_eq!(repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");

    // ---- rebase, merge backend: head-name is theirs, onto is ours ----
    let mut repo = conflicting_branches();
    repo.git(&["checkout", "side"]);
    repo.git_expect_failure(&["rebase", "main"]);
    // The branch being replayed, as a full ref — the one side that comes
    // back already named.
    assert_eq!(
        read_git_file(&mut repo, "rebase-merge/head-name"),
        "refs/heads/side"
    );
    // The side being landed on is a bare object name, so it has to be
    // asked for by name separately.
    let onto = read_git_file(&mut repo, "rebase-merge/onto");
    assert_eq!(onto.len(), 40, "a raw sha, not a ref: {onto}");
    assert_eq!(
        repo.git(&["name-rev", "--name-only", "--refs=refs/heads/*", &onto]),
        "main"
    );

    // ---- rebase, apply backend: same answers, its own directory ----
    let mut repo = conflicting_branches();
    repo.git(&["checkout", "side"]);
    repo.git_expect_failure(&["rebase", "--apply", "main"]);
    assert_eq!(
        read_git_file(&mut repo, "rebase-apply/head-name"),
        "refs/heads/side",
        "the apply backend keeps the same name under its own directory"
    );

    // ---- cherry-pick: the side coming in is a commit, not a branch ----
    let mut repo = conflicting_branches();
    repo.git_expect_failure(&["cherry-pick", "side"]);
    let picked = repo.git(&["rev-parse", "CHERRY_PICK_HEAD"]);
    assert!(!picked.is_empty());
    assert_eq!(
        repo.git(&["name-rev", "--name-only", "--refs=refs/heads/*", &picked]),
        "side",
        "a tip picks up its branch's name; an older commit would be side~N"
    );
}

/// And what `sides` makes of all that — including the reversal, which is
/// the whole reason the names are read rather than assumed.
#[tokio::test]
async fn sides_names_each_side_by_what_it_actually_is() {
    let (exec, cancel) = env();

    // A merge lands the other branch on this one.
    let repo = conflicting_branches();
    integrate::merge(&exec, &repo.path, "side", &MergeOptions::default(), &cancel)
        .await
        .expect("conflict");
    let s = conflict::sides(&exec, &repo.path, InProgress::Merge, Some("main"), &cancel)
        .await
        .expect("sides");
    assert_eq!(s.ours, "main", "the branch the status read HEAD on");
    assert_eq!(s.theirs, "side");

    // A rebase swaps them: `side` is the one being replayed, so it is
    // `theirs`, and `main` is what it is landing on.
    let mut repo = conflicting_branches();
    repo.git(&["checkout", "side"]);
    integrate::rebase(
        &exec,
        &repo.path,
        "main",
        &RebaseOptions::default(),
        &cancel,
    )
    .await
    .expect("a stop is an answer, not a failure");
    // Handed the branch being replayed, which is what a status reads of
    // HEAD mid-rebase; the side is git's own file, not that.
    let s = conflict::sides(&exec, &repo.path, InProgress::Rebase, Some("side"), &cancel)
        .await
        .expect("sides");
    assert_eq!(s.ours, "main", "the upstream being landed on");
    assert_eq!(s.theirs, "side", "the branch being replayed");

    // A cherry-pick brings one commit onto the current branch.
    let repo = conflicting_branches();
    integrate::cherry_pick(&exec, &repo.path, &["side".into()], &cancel)
        .await
        .expect("a conflict is a landing, not a failure");
    let s = conflict::sides(
        &exec,
        &repo.path,
        InProgress::CherryPick,
        Some("main"),
        &cancel,
    )
    .await
    .expect("sides");
    assert_eq!(s.ours, "main");
    assert_eq!(s.theirs, "side");
}

/// Nothing to name a side by is answered with nothing, not with a guess
/// or an error: a commit no branch reaches comes back empty and the UI
/// falls back to its own wording.
#[tokio::test]
async fn a_side_no_branch_reaches_is_left_unnamed() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "gone"]);
    let orphan = repo.commit_file_id("f.txt", "orphan\n", "off on its own");
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "main\n", "main change");
    // The branch goes; the commit stays reachable only by its hash.
    repo.git(&["branch", "-D", "gone"]);
    let (exec, cancel) = env();

    integrate::cherry_pick(&exec, &repo.path, &[orphan], &cancel)
        .await
        .expect("a conflict is a landing, not a failure");
    let s = conflict::sides(&exec, &repo.path, InProgress::CherryPick, None, &cancel)
        .await
        .expect("sides");
    assert_eq!(s.ours, "", "detached names no side of its own");
    assert_eq!(s.theirs, "", "git says `undefined`, which is not a name");
}

/// Reads a file in the git directory by the name git knows it by.
fn read_git_file(repo: &mut TestRepo, rel: &str) -> String {
    let path = repo.git(&["rev-parse", "--git-path", rel]);
    std::fs::read_to_string(repo.path.join(path.trim()))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}
