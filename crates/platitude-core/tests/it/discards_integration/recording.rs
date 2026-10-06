//! Platitude GG's own record (破棄記録仕様.md §2), written by the recorders
//! and read back as entries: the copy of thrown-away work in the shape of a
//! stash, and the notes of everything git keeps no line for.

use super::{begin, copy_and_record, finish, git_dir, oid, one, read, restores, same_dir};
use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::discards::{
    self, CopyOf, CopyOperation, DiscardKind, Look, RemoteTip, Restore,
};

/// What a discard of `tracked` and `untracked` would copy.
fn discard_of<'a>(
    repo: &'a TestRepo,
    git_dir: &'a std::path::Path,
    tracked: &'a [String],
    untracked: &'a [String],
) -> CopyOf<'a> {
    CopyOf {
        workdir: &repo.path,
        git_dir,
        tracked,
        untracked,
        operation: CopyOperation::Discard,
        touches_index: false,
        moved: None,
    }
}

fn paths(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

/// Gives `path` the modification time `at`.
fn dated(path: &std::path::Path, at: std::time::SystemTime) {
    std::fs::File::options()
        .write(true)
        .open(path)
        .and_then(|file| file.set_modified(at))
        .expect("date the file");
}

/// A change made in the second the index was written, to the same size,
/// is copied as the working tree holds it: git looks into such a file
/// rather than trust its stat ("racy git"), and the copy of the index
/// keeps the index's own date for it to. `core.trustctime=false` leaves
/// the date and the size the only stat git reads.
#[tokio::test]
async fn a_change_in_the_second_the_index_was_written_is_copied() {
    let mut repo = TestRepo::init();
    repo.git(&["config", "core.trustctime", "false"]);
    repo.commit_file("f.txt", "aaaa\n", "c1");
    let second = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
    repo.write_file("f.txt", "bbbb\n");
    dated(&repo.path.join("f.txt"), second);
    repo.git(&["add", "f.txt"]);
    dated(&git_dir(&repo).join("index"), second);
    repo.write_file("f.txt", "cccc\n");
    dated(&repo.path.join("f.txt"), second);
    let dir = git_dir(&repo);
    let tracked = paths(&["f.txt"]);

    assert!(copy_and_record(&discard_of(&repo, &dir, &tracked, &[])).await);

    let copy = repo.git(&["rev-parse", discards::RECORD_REF]);
    assert_eq!(repo.git(&["show", &format!("{copy}:f.txt")]), "cccc");
    assert_eq!(repo.git(&["show", &format!("{copy}^2:f.txt")]), "bbbb");
}

/// The copy holds what the index had, what the working tree had and the
/// untracked files apart, as a stash would — and nothing the copy is made
/// in moves: not the index, the working tree, nor the stash.
#[tokio::test]
async fn a_copy_holds_the_staged_the_unstaged_and_the_untracked_apart() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.commit_file("b.txt", "1\n", "c2");
    repo.write_file("a.txt", "2\n");
    repo.git(&["add", "a.txt"]);
    repo.write_file("a.txt", "3\n");
    repo.write_file("b.txt", "2\n");
    repo.write_file("u.txt", "1\n");
    let status_before = repo.git(&["status", "--porcelain=v1"]);
    let dir = git_dir(&repo);
    let (tracked, untracked) = (paths(&["a.txt", "b.txt"]), paths(&["u.txt"]));

    assert!(copy_and_record(&discard_of(&repo, &dir, &tracked, &untracked)).await);

    assert_eq!(repo.git(&["status", "--porcelain=v1"]), status_before);
    assert!(!repo.git_ok(&["rev-parse", "--verify", "--quiet", "refs/stash"]));
    let copy = repo.git(&["rev-parse", discards::RECORD_REF]);
    assert_eq!(repo.git(&["show", &format!("{copy}^2:a.txt")]), "2");
    assert_eq!(repo.git(&["show", &format!("{copy}:a.txt")]), "3");
    assert_eq!(repo.git(&["show", &format!("{copy}:b.txt")]), "2");
    assert_eq!(repo.git(&["show", &format!("{copy}^3:u.txt")]), "1");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%an <%ae> %cn <%ce>", &copy]),
        "Platitude GG <> Platitude GG <>"
    );
    assert_eq!(
        repo.git(&[
            "log",
            "-g",
            "-1",
            "--format=%gn <%ge>",
            discards::RECORD_REF
        ]),
        "Platitude GG <>"
    );

    let found = read(&repo).await;
    let discarded = one(&found, DiscardKind::Discarded);
    assert_eq!(
        (discarded.name.as_str(), discarded.worktree.as_str()),
        ("main", "")
    );
    let part = &discarded.parts[0];
    assert_eq!(
        (part.look, part.files, part.tip),
        (Look::Uncommitted, 3, oid(&copy))
    );
    assert!(
        matches!(
            &part.restore,
            Restore::Changes { copy: back, path, git_dir: dir } if *back == oid(&copy)
                && same_dir(path, &repo.path) && same_dir(dir, &git_dir(&repo))
        ),
        "{part:?}"
    );
}

/// Nothing is written before the first commit (§2.1), nor for paths that
/// hold nothing HEAD does not.
#[tokio::test]
async fn nothing_is_copied_unborn_or_when_nothing_differs() {
    let mut repo = TestRepo::init();
    repo.write_file("a.txt", "1\n");
    repo.git(&["add", "a.txt"]);
    let dir = git_dir(&repo);
    let tracked = paths(&["a.txt"]);

    assert!(
        !copy_and_record(&discard_of(&repo, &dir, &tracked, &[])).await,
        "unborn"
    );
    repo.git(&["commit", "-m", "c1"]);
    assert!(
        !copy_and_record(&discard_of(&repo, &dir, &tracked, &[])).await,
        "nothing differs"
    );

    assert!(!repo.git_ok(&["rev-parse", "--verify", "--quiet", discards::RECORD_REF]));
}

/// The user's settings neither stop a copy nor sign it: no identity at all
/// under `user.useConfigOnly`, a `reference-transaction` hook refusing every
/// ref, and `core.safecrlf` refusing a file of mixed line endings (§2.1).
#[tokio::test]
async fn the_users_settings_do_not_stop_a_copy() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.git(&["config", "--unset", "user.name"]);
    repo.git(&["config", "--unset", "user.email"]);
    repo.git(&["config", "user.useConfigOnly", "true"]);
    repo.git(&["config", "core.safecrlf", "true"]);
    repo.git(&["config", "core.autocrlf", "true"]);
    repo.write_hook("reference-transaction", "exit 1\n");
    repo.write_file("a.txt", "mixed\r\nendings\n");
    let dir = git_dir(&repo);
    let tracked = paths(&["a.txt"]);

    assert!(copy_and_record(&discard_of(&repo, &dir, &tracked, &[])).await);

    let found = read(&repo).await;
    let discarded = one(&found, DiscardKind::Discarded);
    assert_eq!(discarded.parts[0].files, 1);
    assert!(discarded.parts[0].not_copied.is_empty());
}

/// A file git will not take — its required filter has no driver here — is
/// left out and named; the rest is copied (§2.1).
#[tokio::test]
async fn a_file_git_will_not_take_is_named_and_the_rest_copied() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.commit_file(".gitattributes", "*.bin filter=absent\n", "attributes");
    repo.git(&["config", "filter.absent.required", "true"]);
    repo.git(&[
        "config",
        "filter.absent.clean",
        "absent-filter-that-is-not-here",
    ]);
    repo.write_file("a.txt", "2\n");
    repo.write_file("big.bin", "data\n");
    let dir = git_dir(&repo);
    let (tracked, untracked) = (paths(&["a.txt"]), paths(&["big.bin"]));

    assert!(copy_and_record(&discard_of(&repo, &dir, &tracked, &untracked)).await);

    let found = read(&repo).await;
    let part = &one(&found, DiscardKind::Discarded).parts[0];
    assert_eq!(part.not_copied, vec!["big.bin".to_string()]);
    let copy = repo.git(&["rev-parse", discards::RECORD_REF]);
    assert_eq!(repo.git(&["show", &format!("{copy}:a.txt")]), "2");
}

/// The same filter not required: git takes the file whole where the filter
/// fails, so it is copied and nothing is named — the line the screen draws
/// between a discard brought back and one that is not (`lfs::required`).
#[tokio::test]
async fn a_failing_filter_that_is_not_required_is_copied_whole() {
    let mut repo = TestRepo::init();
    repo.commit_file(".gitattributes", "*.bin filter=absent\n", "attributes");
    repo.git(&[
        "config",
        "filter.absent.clean",
        "absent-filter-that-is-not-here",
    ]);
    repo.write_file("big.bin", "data\n");
    let dir = git_dir(&repo);
    let untracked = paths(&["big.bin"]);

    assert!(copy_and_record(&discard_of(&repo, &dir, &[], &untracked)).await);

    let found = read(&repo).await;
    let part = &one(&found, DiscardKind::Discarded).parts[0];
    assert_eq!((part.files, part.not_copied.clone()), (1, Vec::new()));
}

/// Work none of which git will take still goes on the record: a copy of
/// nothing, the paths named (§2.1) — the record says what went.
#[tokio::test]
async fn work_git_takes_none_of_is_still_on_the_record() {
    let mut repo = TestRepo::init();
    repo.commit_file(".gitattributes", "*.bin filter=absent\n", "attributes");
    repo.git(&["config", "filter.absent.required", "true"]);
    repo.git(&[
        "config",
        "filter.absent.clean",
        "absent-filter-that-is-not-here",
    ]);
    repo.write_file("big.bin", "data\n");
    let dir = git_dir(&repo);
    let untracked = paths(&["big.bin"]);

    assert!(copy_and_record(&discard_of(&repo, &dir, &[], &untracked)).await);

    let found = read(&repo).await;
    let part = &one(&found, DiscardKind::Discarded).parts[0];
    assert_eq!(
        (part.files, part.not_copied.clone()),
        (0, vec!["big.bin".to_string()])
    );
}

/// A staged file where HEAD has a folder of its name, and a staged folder's
/// file where HEAD has a file, are copied as the index holds them — and
/// thrown away, both sides back to HEAD, the copy is what went.
#[tokio::test]
async fn a_file_and_a_folder_trading_places_are_copied() {
    let mut repo = TestRepo::init();
    repo.commit_file("lib", "file\n", "c1");
    repo.commit_file("d/x", "x\n", "c2");
    repo.git(&["rm", "--quiet", "--cached", "lib", "d/x"]);
    std::fs::remove_file(repo.path.join("lib")).expect("take the file away");
    std::fs::remove_dir_all(repo.path.join("d")).expect("take the folder away");
    repo.write_file("lib/x", "in a folder\n");
    repo.write_file("d", "a file\n");
    repo.git(&["add", "lib/x", "d"]);
    let dir = git_dir(&repo);
    let tracked = paths(&["d", "lib/x"]);
    let workdir = repo.path.clone();
    let of = CopyOf {
        workdir: &workdir,
        git_dir: &dir,
        tracked: &tracked,
        untracked: &[],
        operation: CopyOperation::Discard,
        touches_index: true,
        moved: None,
    };

    let copied = begin(&of).await.expect("work to copy");
    repo.git(&["reset", "--hard", "--quiet"]);
    assert!(finish(&of, &copied, true).await);

    let copy = repo.git(&["rev-parse", discards::RECORD_REF]);
    assert_eq!(
        repo.git(&["show", &format!("{copy}^2:lib/x")]),
        "in a folder"
    );
    assert_eq!(repo.git(&["show", &format!("{copy}^2:d")]), "a file");
}

/// In a sparse checkout, files outside its definition — a tracked one
/// written back by hand, an untracked one — are copied like any other.
#[tokio::test]
async fn files_outside_a_sparse_checkout_are_copied() {
    let mut repo = TestRepo::init();
    repo.commit_file("a/1", "1\n", "c1");
    repo.commit_file("b/2", "2\n", "c2");
    repo.git(&["sparse-checkout", "set", "a"]);
    repo.write_file("b/2", "changed\n");
    repo.write_file("b/new", "new\n");
    let dir = git_dir(&repo);
    let (tracked, untracked) = (paths(&["b/2"]), paths(&["b/new"]));

    assert!(copy_and_record(&discard_of(&repo, &dir, &tracked, &untracked)).await);

    let found = read(&repo).await;
    let part = &one(&found, DiscardKind::Discarded).parts[0];
    assert!(part.not_copied.is_empty(), "{part:?}");
    let copy = repo.git(&["rev-parse", discards::RECORD_REF]);
    assert_eq!(repo.git(&["show", &format!("{copy}:b/2")]), "changed");
    assert_eq!(repo.git(&["show", &format!("{copy}^3:b/new")]), "new");
}

/// The work a `reset --hard` threw away joins the reset as one entry: the
/// branch at the old tip first, then the work.
#[tokio::test]
async fn a_reset_hards_copy_and_its_reset_are_one_entry() {
    let mut repo = TestRepo::init();
    let c1 = repo.commit_file_id("a.txt", "1\n", "c1");
    let c2 = repo.commit_file_id("a.txt", "2\n", "c2");
    repo.write_file("a.txt", "3\n");
    let dir = git_dir(&repo);
    let tracked = paths(&["a.txt"]);
    let of = CopyOf {
        operation: CopyOperation::ResetHard,
        moved: Some(("main", oid(&c2), oid(&c1))),
        ..discard_of(&repo, &dir, &tracked, &[])
    };

    assert!(copy_and_record(&of).await);
    repo.git(&["reset", "--hard", "HEAD~1"]);

    let found = read(&repo).await;
    assert_eq!(found.len(), 1, "{found:#?}");
    let reset = one(&found, DiscardKind::Reset);
    assert!(
        matches!(
            restores(reset)[..],
            [Restore::Branch { name, tip, .. }, Restore::Changes { .. }]
                if name == "main-pgg-restored" && *tip == oid(&c2)
        ),
        "{reset:#?}"
    );
}

/// A branch deleted comes back with the upstream it was measured against;
/// one deleted on a remote holding another tip has that tip as a second
/// part, which takes the next free name — and a part brought back is read
/// no more.
#[tokio::test]
async fn a_deleted_branch_comes_back_with_its_upstream_and_the_remotes_tip() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.git(&["switch", "-c", "spike"]);
    let s1 = repo.commit_file_id("s.txt", "1\n", "s1");
    let s2 = repo.commit_file_id("s.txt", "2\n", "s2");
    repo.git(&["config", "branch.spike.remote", "origin"]);
    repo.git(&["config", "branch.spike.merge", "refs/heads/spike"]);
    repo.git(&["switch", "main"]);
    let (executor, cancel) = env();
    let dir = git_dir(&repo);
    let before = discards::branch_before(&executor, &repo.path, "spike", &cancel)
        .await
        .expect("read the branch")
        .expect("the branch is there");
    repo.git(&["branch", "--delete", "--force", "spike"]);
    let remote = RemoteTip {
        remote: "origin".to_string(),
        branch: "spike".to_string(),
        tip: oid(&s1),
    };

    discards::record_branch_delete(&executor, &repo.path, &dir, &before, Some(&remote), &cancel)
        .await
        .expect("note the delete");

    let found = read(&repo).await;
    let deleted = one(&found, DiscardKind::Deleted);
    assert_eq!(deleted.remote, "origin");
    assert_eq!(
        restores(deleted),
        vec![
            &Restore::Branch {
                name: "spike".to_string(),
                tip: oid(&s2),
                upstream: Some(("origin".to_string(), "spike".to_string())),
            },
            &Restore::Branch {
                name: "spike-pgg-restored".to_string(),
                tip: oid(&s1),
                upstream: None,
            },
        ]
    );
    assert_eq!(deleted.lost(), vec![oid(&s2), oid(&s1)]);

    let note = repo.git(&["rev-parse", discards::RECORD_REF]);
    discards::record_restored(&executor, &repo.path, &dir, &oid(&note), 0, &cancel)
        .await
        .expect("note the restore");
    let found = read(&repo).await;
    let left = one(&found, DiscardKind::Deleted);
    assert_eq!(left.parts.len(), 1);
    assert_eq!(left.parts[0].tip, oid(&s1));
}

/// A remote branch deleted, or pushed over, comes back as a branch here
/// under the remote's name of it, with no upstream (§4).
#[tokio::test]
async fn a_remote_branch_deleted_or_pushed_over_comes_back_as_a_branch_here() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.git(&["switch", "-c", "gone"]);
    let g1 = repo.commit_file_id("g.txt", "1\n", "g1");
    repo.git(&["switch", "main"]);
    repo.git(&["branch", "--delete", "--force", "gone"]);
    let (executor, cancel) = env();
    let dir = git_dir(&repo);
    let tip = RemoteTip {
        remote: "origin".to_string(),
        branch: "main".to_string(),
        tip: oid(&g1),
    };

    discards::record_force_push(&executor, &repo.path, &dir, &tip, &cancel)
        .await
        .expect("note the push");
    let gone = RemoteTip {
        branch: "gone".to_string(),
        ..tip.clone()
    };
    discards::record_remote_delete(&executor, &repo.path, &dir, &gone, &cancel)
        .await
        .expect("note the delete");

    let found = read(&repo).await;
    let pushed = one(&found, DiscardKind::ForcePushed);
    assert_eq!(
        (pushed.name.as_str(), pushed.remote.as_str()),
        ("main", "origin")
    );
    assert!(
        matches!(&restores(pushed)[..], [Restore::Branch { name, upstream: None, .. }] if name == "main-pgg-restored")
    );
    let deleted = one(&found, DiscardKind::DeletedRemoteBranch);
    assert!(matches!(&restores(deleted)[..], [Restore::Branch { name, .. }] if name == "gone"));
    assert_eq!(deleted.lost(), vec![oid(&g1)]);
}

/// A tag deleted comes back on the object it named — an annotated tag's
/// own, so it is the same tag — and stands on its commit on the graph.
#[tokio::test]
async fn a_deleted_tag_comes_back_on_its_own_object() {
    let mut repo = TestRepo::init();
    let c1 = repo.commit_file_id("a.txt", "1\n", "c1");
    repo.git(&["tag", "--annotate", "-m", "first", "v1"]);
    let object = repo.git(&["rev-parse", "refs/tags/v1"]);
    let (executor, cancel) = env();
    let dir = git_dir(&repo);
    let before = discards::tag_before(&executor, &repo.path, "v1", "refs/tags/v1", &cancel)
        .await
        .expect("read the tag")
        .expect("the tag is there");
    repo.git(&["tag", "--delete", "v1"]);

    discards::record_tag_delete(&executor, &repo.path, &dir, Some(&before), None, &cancel)
        .await
        .expect("note the delete");

    let found = read(&repo).await;
    let deleted = one(&found, DiscardKind::DeletedTag);
    assert_eq!(deleted.parts[0].tip, oid(&c1));
    assert_eq!(
        restores(deleted),
        vec![&Restore::Tag {
            name: "v1".to_string(),
            object: oid(&object),
        }]
    );
}

/// A worktree removed comes back where it was, on the branch it had
/// out.
#[tokio::test]
async fn a_removed_worktree_comes_back_where_it_was() {
    let mut repo = TestRepo::init();
    let c1 = repo.commit_file_id("a.txt", "1\n", "c1");
    let side = repo.path.with_file_name("side");
    let side_path = side.to_string_lossy().into_owned();
    repo.git(&["worktree", "add", "-b", "side-work", &side_path]);
    repo.git(&["worktree", "remove", &side_path]);
    let (executor, cancel) = env();
    let dir = git_dir(&repo);

    discards::record_worktree_remove(
        &executor,
        &repo.path,
        &dir,
        &side_path,
        Some("side-work"),
        &oid(&c1),
        &cancel,
    )
    .await
    .expect("note the remove");

    let found = read(&repo).await;
    let removed = one(&found, DiscardKind::RemovedWorktree);
    assert_eq!(removed.name, "side");
    assert_eq!(
        restores(removed),
        vec![&Restore::Worktree {
            path: side_path,
            branch: Some("side-work".to_string()),
            head: oid(&c1),
        }]
    );
}

/// A stash dropped, and one popped, are each the stash itself, named by its
/// message and counted by the paths it holds.
#[tokio::test]
async fn a_dropped_or_popped_stash_is_the_entry_itself() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.write_file("a.txt", "2\n");
    repo.write_file("u.txt", "1\n");
    repo.git(&["stash", "push", "--include-untracked", "-m", "try again"]);
    let (executor, cancel) = env();
    let dir = git_dir(&repo);
    let before = discards::stash_before(&executor, &repo.path, "stash@{0}", &cancel)
        .await
        .expect("read the stash")
        .expect("the entry is there");
    repo.git(&["stash", "drop"]);

    discards::record_stash(&executor, &repo.path, &dir, &before, false, &cancel)
        .await
        .expect("note the drop");

    let found = read(&repo).await;
    let dropped = one(&found, DiscardKind::DroppedStash);
    assert_eq!(dropped.name, "On main: try again");
    assert_eq!(
        (
            dropped.parts[0].look,
            dropped.parts[0].files,
            dropped.lost()
        ),
        (Look::Stash, 2, vec![before.commit])
    );
    assert_eq!(
        restores(dropped),
        vec![&Restore::Stash {
            commit: before.commit,
            message: "On main: try again".to_string(),
        }]
    );
}

/// The branches one rebase moved together are one entry, one part each.
#[tokio::test]
async fn the_branches_one_rebase_moved_are_one_entry() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.git(&["switch", "-c", "base"]);
    let b1 = repo.commit_file_id("b.txt", "1\n", "b1");
    repo.git(&["switch", "-c", "top"]);
    let t1 = repo.commit_file_id("t.txt", "1\n", "t1");
    repo.git(&["switch", "main"]);
    repo.commit_file("m.txt", "1\n", "m1");
    repo.git(&["switch", "top"]);
    repo.git(&["rebase", "--update-refs", "main"]);
    let (b2, t2) = (
        repo.git(&["rev-parse", "base"]),
        repo.git(&["rev-parse", "top"]),
    );
    let (executor, cancel) = env();
    let dir = git_dir(&repo);
    let moved = vec![
        ("top".to_string(), oid(&t1), oid(&t2)),
        ("base".to_string(), oid(&b1), oid(&b2)),
    ];

    discards::record_moves(&executor, &repo.path, &dir, &moved, &cancel)
        .await
        .expect("note the moves");

    let found = read(&repo).await;
    assert_eq!(found.len(), 1, "{found:#?}");
    let rebase = one(&found, DiscardKind::Rebase);
    assert_eq!(rebase.parts.len(), 2);
    assert_eq!(rebase.lost(), vec![oid(&t1), oid(&b1)]);
}
