//! Bringing a part back (破棄記録仕様.md §4): each as git's own write for the
//! thing, and thrown-away work put back as what went and nothing else — the
//! copy's difference from what the discard left — the first way it goes on.

use super::{begin, finish, git_dir, oid, read, same_dir};
use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::discards::{self, CopyOf, CopyOperation, DiscardKind, Restore, Restored};

/// What a discard of `tracked` and `untracked` copies when `discard` throws
/// it away as the write does (`touches_index` for a staged row's): the copy
/// that goes on the record.
async fn discarded(
    repo: &mut TestRepo,
    (tracked, untracked): (&[&str], &[&str]),
    operation: CopyOperation,
    touches_index: bool,
    discard: impl FnOnce(&mut TestRepo),
) -> String {
    let (workdir, dir) = (repo.path.clone(), git_dir(repo));
    let tracked: Vec<String> = tracked.iter().map(|p| (*p).to_string()).collect();
    let untracked: Vec<String> = untracked.iter().map(|p| (*p).to_string()).collect();
    let of = CopyOf {
        workdir: &workdir,
        git_dir: &dir,
        tracked: &tracked,
        untracked: &untracked,
        operation,
        touches_index,
        moved: None,
    };
    let copied = begin(&of).await.expect("work to copy");
    discard(repo);
    assert!(
        finish(&of, &copied, true).await,
        "the discard took something"
    );
    repo.git(&["rev-parse", discards::RECORD_REF])
}

/// Whole unstaged files and untracked ones thrown away, as `restore` and
/// `clean` leave them.
async fn copied(repo: &mut TestRepo, tracked: &[&str], untracked: &[&str]) -> String {
    let (tracked_paths, untracked_paths) = (tracked.to_vec(), untracked.to_vec());
    let operation = if tracked.is_empty() {
        CopyOperation::Untracked
    } else {
        CopyOperation::Discard
    };
    discarded(repo, (tracked, untracked), operation, false, move |repo| {
        if !tracked_paths.is_empty() {
            let mut args = vec!["restore", "--worktree", "--"];
            args.extend(tracked_paths.iter());
            repo.git(&args);
        }
        for path in untracked_paths {
            std::fs::remove_file(repo.path.join(path)).expect("throw the file away");
        }
    })
    .await
}

fn part_of(repo: &TestRepo, copy: &str) -> Restore {
    Restore::Changes {
        copy: oid(copy),
        path: repo.path.to_string_lossy().into_owned(),
        git_dir: git_dir(repo).to_string_lossy().into_owned(),
    }
}

async fn restored(repo: &TestRepo, part: &Restore) -> Restored {
    let (executor, cancel) = env();
    discards::restore(&executor, &repo.path, part, &cancel)
        .await
        .expect("restore")
}

/// The same work thrown away again in the second its copy came back is
/// listed again: the note that the first copy is back names that line
/// alone, though every write here falls in one second.
#[tokio::test]
async fn work_thrown_away_again_in_the_second_it_came_back_is_listed() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    let executor = crate::support::exec::dated("1700000000 +0000");
    let cancel = tokio_util::sync::CancellationToken::new();
    let (workdir, dir) = (repo.path.clone(), git_dir(&repo));
    let tracked = vec!["a.txt".to_string()];
    let of = CopyOf {
        workdir: &workdir,
        git_dir: &dir,
        tracked: &tracked,
        untracked: &[],
        operation: CopyOperation::Discard,
        touches_index: false,
        moved: None,
    };
    for round in 0..2 {
        repo.write_file("a.txt", "2\n");
        let copied = discards::copy_work(&executor, &of, &cancel)
            .await
            .expect("copy")
            .expect("work to copy");
        repo.git(&["restore", "--worktree", "--", "a.txt"]);
        assert!(
            discards::record_copy(&executor, &repo.path, &dir, &copied, true, &cancel)
                .await
                .expect("record")
        );
        let found = read(&repo).await;
        let entry = found
            .iter()
            .find(|entry| entry.kind == DiscardKind::Discarded)
            .unwrap_or_else(|| panic!("round {round}: the discard is listed: {found:#?}"));
        if round == 0 {
            let part = &entry.parts[0];
            discards::restore(&executor, &repo.path, &part.restore, &cancel)
                .await
                .expect("restore");
            let (record, at) = part.record.expect("a part of the record");
            discards::record_restored(&executor, &repo.path, &dir, &record, at, &cancel)
                .await
                .expect("note the restore");
            assert!(read(&repo).await.is_empty(), "the first copy is back");
        }
    }
}

/// Thrown-away work comes back as it was: what was staged staged, what was
/// not in the working tree alone, and the untracked files untracked.
#[tokio::test]
async fn work_comes_back_as_it_was_staged() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.commit_file("b.txt", "1\n", "c2");
    repo.write_file("a.txt", "2\n");
    repo.git(&["add", "a.txt"]);
    repo.write_file("a.txt", "3\n");
    repo.write_file("b.txt", "2\n");
    repo.write_file("u.txt", "1\n");
    let status_before = repo.git(&["status", "--porcelain=v1"]);
    let copy = discarded(
        &mut repo,
        (&["a.txt", "b.txt"], &["u.txt"]),
        CopyOperation::Discard,
        true,
        |repo| {
            repo.git(&["restore", "--staged", "--worktree", "a.txt", "b.txt"]);
            repo.git(&["clean", "-f", "u.txt"]);
        },
    )
    .await;

    assert_eq!(
        restored(&repo, &part_of(&repo, &copy)).await,
        Restored::Whole
    );

    assert_eq!(repo.git(&["status", "--porcelain=v1"]), status_before);
    assert_eq!(repo.git(&["show", ":a.txt"]), "2");
}

/// The copy names the worktree it came from as git does, so work
/// thrown out of a repository that moved since goes back into it where it
/// stands now — and into nowhere, said so, once the worktree is gone.
#[tokio::test]
async fn work_goes_back_into_its_worktree_where_it_stands_now() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.write_file("a.txt", "2\n");
    copied(&mut repo, &["a.txt"], &[]).await;
    let moved = repo.path.with_file_name("moved");
    std::fs::rename(&repo.path, &moved).expect("move the repository");
    repo.path = moved;

    let found = super::read(&repo).await;
    let part = &found[0].parts[0];
    assert!(
        matches!(&part.restore, Restore::Changes { path, .. } if same_dir(path, &repo.path)),
        "{part:?}"
    );
    assert_eq!(found[0].worktree, "", "the worktree read from");
    assert_eq!(restored(&repo, &part.restore).await, Restored::Whole);
    assert_eq!(repo.git(&["status", "--porcelain=v1"]), "M a.txt");

    let gone = Restore::Changes {
        copy: part.tip,
        path: repo
            .path
            .with_file_name("nowhere")
            .to_string_lossy()
            .into_owned(),
        git_dir: String::new(),
    };
    let (executor, cancel) = env();
    let refused = discards::restore(&executor, &repo.path, &gone, &cancel)
        .await
        .expect_err("nowhere to go");
    assert!(refused.to_string().contains("is gone"), "{refused}");
}

/// The main worktree is named apart from every linked one — one in a
/// folder `main` too — so its work goes back into it.
#[tokio::test]
async fn the_main_worktrees_work_goes_back_into_it_beside_a_worktree_in_main() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    let linked = repo.path.with_file_name("main");
    repo.git(&["worktree", "add", "--detach", &linked.to_string_lossy()]);
    repo.write_file("a.txt", "2\n");
    copied(&mut repo, &["a.txt"], &[]).await;

    let found = super::read(&repo).await;
    let part = &found[0].parts[0];
    assert!(
        matches!(&part.restore, Restore::Changes { path, .. } if same_dir(path, &repo.path)),
        "{part:?}"
    );
}

/// A staged row thrown away whose working tree had gone back to HEAD's
/// version holds nothing past its staged half; once that half no longer
/// stages over what is staged now, it waits as a stash entry holding it —
/// not "back unstaged" with nothing put back.
#[tokio::test]
async fn a_staged_half_that_will_not_stage_waits_in_the_stash() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n", "c1");
    repo.write_file("f.txt", "staged\n");
    repo.git(&["add", "f.txt"]);
    repo.write_file("f.txt", "1\n");
    let copy = discarded(
        &mut repo,
        (&["f.txt"], &[]),
        CopyOperation::Discard,
        true,
        |repo| {
            repo.git(&["restore", "--staged", "--worktree", "--", "f.txt"]);
        },
    )
    .await;
    repo.write_file("f.txt", "other\n");
    repo.git(&["add", "f.txt"]);

    assert_eq!(
        restored(&repo, &part_of(&repo, &copy)).await,
        Restored::AsStash
    );
    assert_eq!(repo.git(&["show", "stash@{0}^2:f.txt"]), "staged");
    assert_eq!(repo.git(&["status", "--porcelain=v1"]), "M  f.txt");
}

/// Where the staged half no longer stages but the working tree's side holds
/// the same at every staged path, the work comes back unstaged: only that
/// it was staged is lost.
#[tokio::test]
async fn a_staged_half_the_working_tree_held_comes_back_unstaged() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "1\n2\n3\n4\n5\n6\n7\n8\n", "c1");
    repo.write_file("f.txt", "1\nstaged\n3\n4\n5\n6\n7\n8\n");
    repo.git(&["add", "f.txt"]);
    let copy = discarded(
        &mut repo,
        (&["f.txt"], &[]),
        CopyOperation::Discard,
        true,
        |repo| {
            repo.git(&["restore", "--staged", "--worktree", "--", "f.txt"]);
        },
    )
    .await;
    // Staged since on the very line, the working tree back at HEAD's: the
    // working tree's patch goes on, the index's does not.
    repo.write_file("f.txt", "1\nX\n3\n4\n5\n6\n7\n8\n");
    repo.git(&["add", "f.txt"]);
    repo.write_file("f.txt", "1\n2\n3\n4\n5\n6\n7\n8\n");

    assert_eq!(
        restored(&repo, &part_of(&repo, &copy)).await,
        Restored::Unstaged
    );
    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).expect("read f.txt"),
        "1\nstaged\n3\n4\n5\n6\n7\n8\n"
    );
    assert_eq!(repo.git(&["show", ":f.txt"]), "1\nX\n3\n4\n5\n6\n7\n8");
}

/// A folder left where a worktree was removed is no worktree: work
/// thrown away there is refused, not "put back" into the repository the
/// folder sits in — which would answer for the folder and take nothing.
#[tokio::test]
async fn work_into_a_folder_left_by_a_removed_worktree_is_refused() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    let inner = repo.path.join("inner");
    let at = inner.to_string_lossy().into_owned();
    repo.git(&["worktree", "add", "--detach", &at]);
    std::fs::write(inner.join("a.txt"), "2\n").expect("change a");
    let (executor, cancel) = env();
    let tracked = vec!["a.txt".to_string()];
    let inner_git =
        std::path::PathBuf::from(repo.git(&["-C", &at, "rev-parse", "--absolute-git-dir"]));
    let of = CopyOf {
        workdir: &inner,
        git_dir: &inner_git,
        tracked: &tracked,
        untracked: &[],
        operation: CopyOperation::Discard,
        touches_index: false,
        moved: None,
    };
    let copied = discards::copy_work(&executor, &of, &cancel)
        .await
        .expect("copy")
        .expect("work to copy");
    repo.git(&["-C", &at, "restore", "--worktree", "--", "a.txt"]);
    assert!(
        discards::record_copy(&executor, &inner, &inner_git, &copied, true, &cancel)
            .await
            .expect("record")
    );
    repo.git(&["worktree", "remove", &at]);
    std::fs::create_dir_all(&inner).expect("a folder left behind");
    std::fs::write(inner.join("a.txt"), "1\n").expect("a file left behind");

    let found = read(&repo).await;
    let part = &found[0].parts[0];
    assert!(
        matches!(&part.restore, Restore::Changes { path, git_dir, .. }
            if same_dir(path, &inner) && git_dir.is_empty()),
        "{part:?}"
    );
    let refused = discards::restore(&executor, &repo.path, &part.restore, &cancel).await;
    assert!(refused.is_err(), "{refused:?}");
}

/// Work that would not go on over what the worktree holds now is not
/// put in it: it waits as a stash entry holding the same differences — the
/// same tree over the same bases — the working tree untouched.
#[tokio::test]
async fn work_that_would_conflict_waits_in_the_stash() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.write_file("a.txt", "2\n");
    let copy = copied(&mut repo, &["a.txt"], &[]).await;
    repo.commit_file("a.txt", "other\n", "c2");

    assert_eq!(
        restored(&repo, &part_of(&repo, &copy)).await,
        Restored::AsStash
    );

    assert_eq!(repo.git(&["status", "--porcelain=v1"]), "");
    assert_eq!(repo.git(&["show", "stash@{0}:a.txt"]), "2");
    for side in ["^{tree}", "^1", "^2"] {
        assert_eq!(
            repo.git(&["rev-parse", &format!("stash@{{0}}{side}")]),
            repo.git(&["rev-parse", &format!("{copy}{side}")]),
            "{side}"
        );
    }
    assert_eq!(
        repo.git(&["log", "-g", "-1", "--format=%gs", "refs/stash"]),
        repo.git(&["log", "-1", "--format=%s", &copy]),
    );
}

/// An untracked file of the copy's standing where it would go keeps the
/// work out of the working tree too.
#[tokio::test]
async fn an_untracked_file_in_the_way_sends_the_work_to_the_stash() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.write_file("u.txt", "mine\n");
    let copy = copied(&mut repo, &[], &["u.txt"]).await;
    repo.write_file("u.txt", "another\n");

    assert_eq!(
        restored(&repo, &part_of(&repo, &copy)).await,
        Restored::AsStash
    );
    let held = std::fs::read_to_string(repo.path.join("u.txt")).expect("read the file");
    assert_eq!(held, "another\n");
}

/// Twenty lines, `changed` read where its number is.
fn lines(changed: &[(usize, &str)]) -> String {
    (1..=20)
        .map(|n| {
            changed
                .iter()
                .find(|(at, _)| *at == n)
                .map_or_else(|| format!("line {n}\n"), |(_, text)| format!("{text}\n"))
        })
        .collect()
}

/// Staging done since in a path the copy changes stays as it is, and the
/// work comes back beside it, its staged half staged.
#[tokio::test]
async fn staging_in_the_copys_paths_stays_and_the_work_comes_back_beside_it() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.commit_file("b.txt", &lines(&[]), "c2");
    repo.write_file("a.txt", "2\n");
    repo.git(&["add", "a.txt"]);
    repo.write_file("b.txt", &lines(&[(18, "copy")]));
    let copy = discarded(
        &mut repo,
        (&["a.txt", "b.txt"], &[]),
        CopyOperation::Discard,
        true,
        |repo| {
            repo.git(&["restore", "--staged", "--worktree", "a.txt", "b.txt"]);
        },
    )
    .await;
    repo.write_file("b.txt", &lines(&[(2, "mine")]));
    repo.git(&["add", "b.txt"]);

    assert_eq!(
        restored(&repo, &part_of(&repo, &copy)).await,
        Restored::Whole
    );

    assert_eq!(repo.git(&["show", ":a.txt"]), "2");
    assert_eq!(repo.git(&["show", ":b.txt"]), lines(&[(2, "mine")]).trim());
    let held = std::fs::read_to_string(repo.path.join("b.txt")).expect("read b.txt");
    assert_eq!(held, lines(&[(2, "mine"), (18, "copy")]));
}

/// Lines the copy changes changed since keep the whole copy out — its
/// untracked files too: the working tree's half goes on all at once or not
/// at all.
#[tokio::test]
async fn a_changed_line_keeps_the_untracked_files_out_too() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", &lines(&[]), "c1");
    repo.write_file("a.txt", &lines(&[(18, "copy")]));
    repo.write_file("u.txt", "1\n");
    let copy = copied(&mut repo, &["a.txt"], &["u.txt"]).await;
    repo.write_file("a.txt", &lines(&[(18, "mine")]));

    assert_eq!(
        restored(&repo, &part_of(&repo, &copy)).await,
        Restored::AsStash
    );
    assert!(!repo.path.join("u.txt").exists(), "nothing put down");
    let held = std::fs::read_to_string(repo.path.join("a.txt")).expect("read a.txt");
    assert_eq!(held, lines(&[(18, "mine")]));
}

/// A change made since elsewhere in the copy's file stays, and the work
/// comes back beside it, the untracked files with it.
#[tokio::test]
async fn work_comes_back_beside_a_change_made_since() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", &lines(&[]), "c1");
    repo.write_file("a.txt", &lines(&[(18, "copy")]));
    repo.write_file("u.txt", "1\n");
    let copy = copied(&mut repo, &["a.txt"], &["u.txt"]).await;
    repo.write_file("a.txt", &lines(&[(2, "mine")]));

    assert_eq!(
        restored(&repo, &part_of(&repo, &copy)).await,
        Restored::Whole
    );
    let held = std::fs::read_to_string(repo.path.join("a.txt")).expect("read a.txt");
    assert_eq!(held, lines(&[(2, "mine"), (18, "copy")]));
    assert_eq!(
        std::fs::read_to_string(repo.path.join("u.txt")).expect("read u.txt"),
        "1\n"
    );
}

/// A file where one of the copy's folders would go is in the way as much
/// as a file at the path itself: nothing of the copy goes on, its tracked
/// half included.
#[tokio::test]
async fn a_file_where_a_folder_would_go_keeps_the_work_out() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.write_file("a.txt", "2\n");
    repo.write_file("d/u.txt", "1\n");
    let copy = copied(&mut repo, &["a.txt"], &["d/u.txt"]).await;
    std::fs::remove_dir_all(repo.path.join("d")).expect("take the folder away");
    repo.write_file("d", "a file now\n");

    assert_eq!(
        restored(&repo, &part_of(&repo, &copy)).await,
        Restored::AsStash
    );
    let held = std::fs::read_to_string(repo.path.join("d")).expect("read the file");
    assert_eq!(held, "a file now\n");
    assert_eq!(
        repo.git(&["status", "--porcelain=v1"]),
        "?? d",
        "a.txt as it was"
    );
}

/// A merge standing in the worktree keeps the work out: its staged half would
/// go into the merge's commit.
#[tokio::test]
async fn a_merge_standing_keeps_the_work_out() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.git(&["switch", "-c", "side"]);
    repo.commit_file("s.txt", "1\n", "side");
    repo.git(&["switch", "main"]);
    repo.write_file("a.txt", "2\n");
    repo.git(&["add", "a.txt"]);
    let copy = discarded(
        &mut repo,
        (&["a.txt"], &[]),
        CopyOperation::Discard,
        true,
        |repo| {
            repo.git(&["restore", "--staged", "--worktree", "a.txt"]);
        },
    )
    .await;
    repo.git(&["merge", "--no-commit", "--no-ff", "side"]);

    assert_eq!(
        restored(&repo, &part_of(&repo, &copy)).await,
        Restored::AsStash
    );
    assert_eq!(repo.git(&["diff", "--cached", "--name-only"]), "s.txt");
}

/// Part of a file's diff: `keep` written over the copy's paths after the
/// hunks went, as the discard leaves the file.
async fn hunk_discarded(repo: &mut TestRepo, path: &str, keep: &str) -> String {
    let (path, keep) = (path.to_string(), keep.to_string());
    let paths = [path.clone()];
    let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
    discarded(
        repo,
        (&paths, &[]),
        CopyOperation::Hunk,
        false,
        move |repo| {
            repo.write_file(&path, &keep);
        },
    )
    .await
}

/// A hunk thrown out of a file whose other hunk stayed comes back beside
/// it: the copy holds the hunk that went, not the file.
#[tokio::test]
async fn a_hunk_comes_back_beside_the_hunks_left() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", &lines(&[]), "c1");
    repo.write_file("f.txt", &lines(&[(3, "A"), (17, "B")]));
    let copy = hunk_discarded(&mut repo, "f.txt", &lines(&[(17, "B")])).await;
    assert_eq!(
        repo.git(&["diff", &format!("{copy}^1"), &copy, "--stat=200"]),
        "f.txt | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)",
        "the copy is the hunk"
    );

    assert_eq!(
        restored(&repo, &part_of(&repo, &copy)).await,
        Restored::Whole
    );
    let held = std::fs::read_to_string(repo.path.join("f.txt")).expect("read f.txt");
    assert_eq!(held, lines(&[(3, "A"), (17, "B")]));
}

/// A hunk comes back alone where the file's other hunk went since: what
/// was not thrown away with it stays gone.
#[tokio::test]
async fn a_hunk_comes_back_alone_after_the_others_went() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", &lines(&[]), "c1");
    repo.write_file("f.txt", &lines(&[(3, "A"), (17, "B")]));
    let first = hunk_discarded(&mut repo, "f.txt", &lines(&[(17, "B")])).await;
    let second = hunk_discarded(&mut repo, "f.txt", &lines(&[])).await;

    assert_eq!(
        restored(&repo, &part_of(&repo, &first)).await,
        Restored::Whole
    );
    let held = std::fs::read_to_string(repo.path.join("f.txt")).expect("read f.txt");
    assert_eq!(held, lines(&[(3, "A")]), "B stayed thrown away");

    assert_eq!(
        restored(&repo, &part_of(&repo, &second)).await,
        Restored::Whole
    );
    let held = std::fs::read_to_string(repo.path.join("f.txt")).expect("read f.txt");
    assert_eq!(held, lines(&[(3, "A"), (17, "B")]));
}

/// A hunk whose neighbouring lines changed since does not go on: the stash
/// it waits in holds that hunk alone, over the file as the discard left it
/// — applied where the file is so again, it puts back the hunk and nothing
/// else.
#[tokio::test]
async fn a_hunk_that_does_not_go_on_waits_as_itself() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", &lines(&[]), "c1");
    repo.write_file("f.txt", &lines(&[(3, "A"), (4, "B")]));
    let copy = hunk_discarded(&mut repo, "f.txt", &lines(&[(4, "B")])).await;
    repo.write_file("f.txt", &lines(&[(4, "B2")]));

    assert_eq!(
        restored(&repo, &part_of(&repo, &copy)).await,
        Restored::AsStash
    );
    let held = std::fs::read_to_string(repo.path.join("f.txt")).expect("read f.txt");
    assert_eq!(held, lines(&[(4, "B2")]), "the working tree untouched");
    assert_eq!(
        repo.git(&["diff", "stash@{0}^1", "stash@{0}", "--stat=200"]),
        "f.txt | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)"
    );
    // git's own `stash apply` merges over the index, and refuses a file
    // with unstaged changes: the hunk left beside it goes in first.
    repo.write_file("f.txt", &lines(&[(4, "B")]));
    repo.git(&["add", "f.txt"]);
    repo.git(&["stash", "apply", "--index"]);
    let held = std::fs::read_to_string(repo.path.join("f.txt")).expect("read f.txt");
    assert_eq!(held, lines(&[(3, "A"), (4, "B")]));
    assert_eq!(repo.git(&["show", ":f.txt"]), lines(&[(4, "B")]).trim());
}

/// Staging elsewhere, before the discard and after, is no part of the
/// copy: it stays staged through the restore.
#[tokio::test]
async fn staging_elsewhere_is_no_part_of_the_copy() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.commit_file("c.txt", "1\n", "c2");
    repo.write_file("c.txt", "staged\n");
    repo.git(&["add", "c.txt"]);
    repo.write_file("a.txt", "2\n");
    let copy = copied(&mut repo, &["a.txt"], &[]).await;
    assert_eq!(
        repo.git(&["diff", "--name-only", &format!("{copy}^1"), &copy]),
        "a.txt"
    );
    repo.git(&["restore", "--staged", "c.txt"]);
    repo.git(&["restore", "c.txt"]);

    assert_eq!(
        restored(&repo, &part_of(&repo, &copy)).await,
        Restored::Whole
    );
    assert_eq!(repo.git(&["status", "--porcelain=v1"]), "M a.txt");
}

/// A binary file and one under `core.autocrlf` come back as they were: the
/// differences go on as git writes them.
#[tokio::test]
async fn binary_and_converted_files_come_back_as_they_were() {
    let mut repo = TestRepo::init_autocrlf();
    repo.commit_file("t.txt", &lines(&[]).replace('\n', "\r\n"), "c1");
    std::fs::write(repo.path.join("b.bin"), [0u8, 1, 2, 3, 255]).expect("write b.bin");
    repo.git(&["add", "b.bin"]);
    repo.git(&["commit", "-m", "c2"]);
    repo.write_file(
        "t.txt",
        &lines(&[(3, "A"), (17, "B")]).replace('\n', "\r\n"),
    );
    std::fs::write(repo.path.join("b.bin"), [9u8, 0, 8, 255]).expect("write b.bin");
    let t = hunk_discarded(
        &mut repo,
        "t.txt",
        &lines(&[(17, "B")]).replace('\n', "\r\n"),
    )
    .await;
    let b = copied(&mut repo, &["b.bin"], &[]).await;

    assert_eq!(restored(&repo, &part_of(&repo, &t)).await, Restored::Whole);
    assert_eq!(restored(&repo, &part_of(&repo, &b)).await, Restored::Whole);
    let held = std::fs::read_to_string(repo.path.join("t.txt")).expect("read t.txt");
    assert_eq!(held, lines(&[(3, "A"), (17, "B")]).replace('\n', "\r\n"));
    assert_eq!(
        std::fs::read(repo.path.join("b.bin")).expect("read b.bin"),
        vec![9u8, 0, 8, 255]
    );
}

/// A tag comes back as the very object it was — an annotated one as itself,
/// not a tag made over it, whatever `tag.gpgSign` says — and not over a
/// name that stands again.
#[tokio::test]
async fn a_tag_comes_back_as_its_own_object() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "1\n", "c1");
    repo.git(&["tag", "--annotate", "--message", "first", "v1"]);
    let object = repo.git(&["rev-parse", "refs/tags/v1"]);
    repo.git(&["tag", "--delete", "v1"]);
    repo.git(&["config", "tag.gpgSign", "true"]);

    let part = Restore::Tag {
        name: "v1".to_string(),
        object: oid(&object),
    };
    assert_eq!(restored(&repo, &part).await, Restored::Whole);
    assert_eq!(repo.git(&["rev-parse", "refs/tags/v1"]), object);

    let (executor, cancel) = env();
    discards::restore(&executor, &repo.path, &part, &cancel)
        .await
        .expect_err("the name stands");
}

/// A branch with its upstream, a tag on its own object, a stash entry with
/// its message and a worktree on its branch — each git's own write.
#[tokio::test]
async fn names_a_stash_and_a_worktree_come_back_as_git_writes_them() {
    let mut repo = TestRepo::init();
    let c1 = repo.commit_file_id("a.txt", "1\n", "c1");
    repo.git(&["tag", "--annotate", "-m", "first", "v1"]);
    let object = repo.git(&["rev-parse", "refs/tags/v1"]);
    repo.git(&["tag", "--delete", "v1"]);
    repo.write_file("a.txt", "2\n");
    repo.git(&["stash", "push", "-m", "keep me"]);
    let stash = repo.git(&["rev-parse", "stash@{0}"]);
    repo.git(&["stash", "drop"]);
    let side = repo.path.with_file_name("side");
    let side_path = side.to_string_lossy().into_owned();
    repo.git(&["branch", "side-work"]);

    let parts = [
        Restore::Branch {
            name: "spike".to_string(),
            tip: oid(&c1),
            upstream: Some(("origin".to_string(), "spike".to_string())),
        },
        Restore::Tag {
            name: "v1".to_string(),
            object: oid(&object),
        },
        Restore::Stash {
            commit: oid(&stash),
            message: "On main: keep me".to_string(),
        },
        Restore::Worktree {
            path: side_path.clone(),
            branch: Some("side-work".to_string()),
            head: oid(&c1),
        },
    ];
    for part in &parts {
        assert_eq!(restored(&repo, part).await, Restored::Whole, "{part:?}");
    }

    assert_eq!(repo.git(&["config", "branch.spike.remote"]), "origin");
    assert_eq!(
        repo.git(&["config", "branch.spike.merge"]),
        "refs/heads/spike"
    );
    assert_eq!(repo.git(&["rev-parse", "refs/tags/v1"]), object);
    assert_eq!(repo.git(&["rev-parse", "stash@{0}"]), stash);
    assert_eq!(
        repo.git(&["stash", "list", "--format=%gs"]),
        "On main: keep me"
    );
    assert_eq!(
        repo.git_in(&side, &["branch", "--show-current"]),
        "side-work"
    );
}
