//! Read off real reflogs (破棄記録仕様.md §3): which lines count, what each
//! left only the old tip holding, the name a restore would take, and that a
//! move which left nothing behind is no entry.

use super::{oid, one, read, read_at, restores};
use crate::support::TestRepo;
use platitude_core::discards::{self, DiscardKind, Look, Restore};

/// A reset, an amend, a branch set by hand and a detached HEAD left behind,
/// each taking commits off; and the commits and merges in between, which
/// take nothing. Each is in its branch's reflog and HEAD's both, and is one
/// entry.
#[tokio::test]
async fn each_move_off_a_tip_is_one_entry_with_what_only_it_held() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    let c2 = repo.commit_file_id("a.txt", "2\n", "c2");
    let c3 = repo.commit_file_id("a.txt", "3\n", "c3");
    repo.git(&["reset", "--hard", "HEAD~1"]);
    repo.git(&["commit", "--amend", "-m", "c2 again"]);
    repo.git(&["branch", "topic"]);
    repo.git(&["switch", "topic"]);
    let t1 = repo.commit_file_id("b.txt", "1\n", "t1");
    repo.git(&["switch", "main"]);
    repo.git(&["branch", "--force", "topic", "main"]);
    repo.git(&["switch", "--detach", "HEAD"]);
    let d1 = repo.commit_file_id("c.txt", "1\n", "d1");
    repo.git(&["switch", "main"]);

    let found = read(&repo).await;
    assert_eq!(found.len(), 4, "{found:#?}");

    let reset = one(&found, DiscardKind::Reset);
    assert_eq!(
        (reset.name.as_str(), reset.parts[0].tip),
        ("main", oid(&c3))
    );
    assert_eq!(
        reset.lost(),
        vec![oid(&c3), oid(&c2)],
        "c2 went with the amend after it"
    );

    let amend = one(&found, DiscardKind::Amend);
    assert_eq!(
        (amend.parts[0].tip, amend.lost()),
        (oid(&c2), vec![oid(&c2)])
    );

    let moved = one(&found, DiscardKind::Moved);
    assert_eq!(
        (moved.name.as_str(), moved.lost()),
        ("topic", vec![oid(&t1)])
    );

    let left = one(&found, DiscardKind::LeftDetached);
    assert_eq!(
        (left.parts[0].tip, left.lost(), left.copy.as_str()),
        (oid(&d1), vec![oid(&d1)], "")
    );
}

/// The rebase a pull runs is a rebase like any other, though git writes it
/// behind the pull's own command line (`pull --no-edit (finish): …`).
#[tokio::test]
async fn the_rebase_a_pull_runs_is_listed() {
    let (_bare, mut work) = crate::support::remote::origin_and_clone();
    work.commit_file("t.txt", "theirs\n", "theirs");
    work.git(&["push", "--set-upstream", "origin", "main"]);
    work.git(&["reset", "--hard", "HEAD~1"]);
    let mine = work.commit_file_id("m.txt", "mine\n", "mine");
    work.git(&["-c", "pull.rebase=true", "pull", "--no-edit"]);

    let found = read(&work).await;
    let rebase = one(&found, DiscardKind::Rebase);
    assert_eq!(
        (rebase.name.as_str(), rebase.lost()),
        ("main", vec![oid(&mine)])
    );
}

/// What was done on a detached HEAD comes back under the detached name, not
/// a branch named after the folder (§4).
#[tokio::test]
async fn a_move_made_detached_comes_back_under_the_detached_name() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.git(&["switch", "--detach", "HEAD"]);
    let d1 = repo.commit_file_id("d.txt", "1\n", "d1");
    repo.git(&["commit", "--amend", "-m", "d1 again"]);
    repo.git(&["switch", "main"]);

    let found = read(&repo).await;
    let amend = one(&found, DiscardKind::Amend);
    assert_eq!((amend.name.as_str(), amend.parts[0].tip), ("", oid(&d1)));
    assert!(
        matches!(&amend.parts[0].restore, Restore::Branch { name, .. } if name == "detached-pgg-restored"),
        "{amend:#?}"
    );
}

/// An older stash entry holds the commit it stood on as the newest does: a
/// move off a tip one of them stands on took nothing.
#[tokio::test]
async fn a_tip_an_older_stash_stands_on_was_not_taken() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.commit_file("a.txt", "2\n", "c2");
    repo.write_file("w.txt", "on c2\n");
    repo.git(&["stash", "push", "--include-untracked"]);
    repo.git(&["reset", "--hard", "HEAD~1"]);
    repo.write_file("w.txt", "on c1\n");
    repo.git(&["stash", "push", "--include-untracked"]);

    let found = read(&repo).await;
    assert!(
        found.iter().all(|entry| entry.kind != DiscardKind::Reset),
        "{found:#?}"
    );
}

/// A part whose commit this repository never had is drawn nowhere and
/// waits for no walk: its restore is git's to refuse (§4).
#[tokio::test]
async fn a_tag_whose_object_is_not_here_is_absent() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    let dir = super::git_dir(&repo);
    let elsewhere = discards::TagBefore {
        name: "v9".to_string(),
        object: oid("1234567890123456789012345678901234567890"),
        commit: None,
    };
    let (executor, cancel) = crate::support::exec::env();
    discards::record_tag_delete(
        &executor,
        &repo.path,
        &dir,
        None,
        Some(("origin", &elsewhere)),
        &cancel,
    )
    .await
    .expect("record the delete");

    let found = read(&repo).await;
    let tag = one(&found, DiscardKind::DeletedRemoteTag);
    assert_eq!(tag.parts[0].look, Look::Absent);
}

/// A move's commits come back as a branch at the old tip: under the
/// branch's own name where that is free, else with the suffix — and a
/// detached HEAD's under `detached-pgg-restored` (§4).
#[tokio::test]
async fn a_move_comes_back_as_a_branch_named_clear_of_the_ones_here() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    let c2 = repo.commit_file_id("a.txt", "2\n", "c2");
    repo.git(&["reset", "--hard", "HEAD~1"]);
    repo.git(&["branch", "main-pgg-restored"]);
    repo.git(&["switch", "--detach", "HEAD"]);
    repo.commit_file("c.txt", "1\n", "d1");
    repo.git(&["switch", "main"]);

    let found = read(&repo).await;
    let reset = one(&found, DiscardKind::Reset);
    assert_eq!(
        restores(reset),
        vec![&Restore::Branch {
            name: "main-pgg-restored-1".to_string(),
            tip: oid(&c2),
            upstream: None,
        }]
    );
    let left = one(&found, DiscardKind::LeftDetached);
    assert!(
        matches!(restores(left)[..], [Restore::Branch { name, .. }] if name == "detached-pgg-restored"),
        "{left:#?}"
    );
}

/// A branch's reflog goes with the branch; HEAD's keeps its moves, named
/// for the branch by the checkout that left it — whose name is free again.
#[tokio::test]
async fn a_move_on_a_branch_deleted_since_is_read_off_heads_reflog() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.git(&["switch", "-c", "feature"]);
    let f1 = repo.commit_file_id("f.txt", "1\n", "f1");
    let f2 = repo.commit_file_id("f.txt", "2\n", "f2");
    repo.git(&["reset", "--hard", "HEAD~1"]);
    repo.git(&["switch", "main"]);
    repo.git(&["branch", "--delete", "--force", "feature"]);

    let found = read(&repo).await;
    let reset = one(&found, DiscardKind::Reset);
    assert_eq!(
        (reset.name.as_str(), reset.lost()),
        ("feature", vec![oid(&f2), oid(&f1)])
    );
    assert!(
        matches!(restores(reset)[..], [Restore::Branch { name, .. }] if name == "feature"),
        "{reset:#?}"
    );
}

/// The delete a JetBrains IDE writes into HEAD's reflog — HEAD's own
/// value, with the name and the deleted tip in the message — is an entry
/// holding what only that tip reached.
#[tokio::test]
async fn a_delete_another_tool_wrote_is_an_entry() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.git(&["switch", "-c", "spike"]);
    let s1 = repo.commit_file_id("s.txt", "1\n", "s1");
    repo.git(&["switch", "main"]);
    repo.git(&["branch", "--delete", "--force", "spike"]);
    let message = format!("delete_branch: spike origin/spike [{}]", s1.trim());
    repo.git(&["update-ref", "-m", &message, "HEAD", "HEAD"]);

    let found = read(&repo).await;
    assert_eq!(found.len(), 1, "{found:#?}");
    let deleted = one(&found, DiscardKind::Deleted);
    assert_eq!(
        (deleted.name.as_str(), deleted.lost()),
        ("spike", vec![oid(&s1)])
    );
}

/// Another working copy's HEAD is read from this one, and this one's from
/// it: what it left detached is one entry, named for it where it is not
/// the copy read from — and the commit its detached HEAD stands on, which
/// no branch holds, is no one's loss.
#[tokio::test]
async fn another_copys_detached_head_is_read_from_either_copy() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    let side = repo.path.with_file_name("side");
    repo.git(&[
        "worktree",
        "add",
        "--detach",
        &side.to_string_lossy(),
        "HEAD",
    ]);
    repo.git_in(&side, &["commit", "--allow-empty", "-m", "d1"]);
    repo.git_in(&side, &["commit", "--allow-empty", "-m", "d2"]);
    let d2 = repo.git_in(&side, &["rev-parse", "HEAD"]);
    repo.git_in(&side, &["switch", "--detach", "HEAD~1"]);

    for (from, named) in [(repo.path.clone(), "side"), (side.clone(), "")] {
        let found = read_at(&from).await;
        assert_eq!(found.len(), 1, "{found:#?}");
        let left = one(&found, DiscardKind::LeftDetached);
        assert_eq!(
            (left.copy.as_str(), left.lost()),
            (named, vec![oid(&d2)]),
            "read from {from:?}: d1 is side's HEAD"
        );
    }
}

/// A move off a tip another ref still holds took nothing, and is no entry.
#[tokio::test]
async fn a_tip_another_ref_holds_is_no_entry() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.commit_file("a.txt", "2\n", "c2");
    repo.git(&["branch", "keep"]);
    repo.git(&["reset", "--hard", "HEAD~1"]);

    let found = read(&repo).await;
    assert!(found.is_empty(), "keep still reaches c2: {found:#?}");
}
