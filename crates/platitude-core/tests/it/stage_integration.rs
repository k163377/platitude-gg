//! Staging against real repositories: whole files, hunks and single lines.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use platitude_core::details::DiffTarget;
use platitude_core::patch::HunkSelect;
use platitude_core::process::GitExecutor;
use platitude_core::repo::RepoInfo;
use platitude_core::{stage, status};
use tokio_util::sync::CancellationToken;

fn env() -> (GitExecutor, CancellationToken) {
    (GitExecutor::new(), CancellationToken::new())
}

async fn info(repo: &TestRepo) -> RepoInfo {
    let (exec, cancel) = env();
    platitude_core::repo::open(&exec, &repo.path, &cancel)
        .await
        .expect("open repo")
}

/// Staged / unstaged / untracked paths of the current status.
async fn buckets(repo: &TestRepo) -> (Vec<String>, Vec<String>, Vec<String>) {
    let (exec, cancel) = env();
    let s = status::load(&exec, &repo.path, &cancel)
        .await
        .expect("status");
    let collect = |it: &mut dyn Iterator<Item = &status::StatusItem>| {
        let mut v: Vec<String> = it.map(|i| i.path().to_string()).collect();
        v.sort();
        v
    };
    (
        collect(&mut s.staged()),
        collect(&mut s.unstaged()),
        collect(&mut s.untracked()),
    )
}

/// Content of a path in the index (what a commit would record).
fn indexed(repo: &mut TestRepo, path: &str) -> String {
    repo.git(&["show", &format!(":{path}")])
}

#[tokio::test]
async fn stage_and_unstage_whole_files() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("a.txt", "one changed\n");
    repo.write_file("new.txt", "fresh\n");
    let (exec, cancel) = env();

    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert!(staged.is_empty());
    assert_eq!(unstaged, vec!["a.txt"]);
    assert_eq!(untracked, vec!["new.txt"]);

    stage::stage_paths(
        &exec,
        &repo.path,
        &["a.txt".into(), "new.txt".into()],
        &cancel,
    )
    .await
    .expect("stage");
    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert_eq!(staged, vec!["a.txt", "new.txt"]);
    assert!(unstaged.is_empty() && untracked.is_empty());

    stage::unstage_paths(&exec, &repo.path, &["a.txt".into()], &cancel)
        .await
        .expect("unstage");
    let (staged, unstaged, _) = buckets(&repo).await;
    assert_eq!(staged, vec!["new.txt"]);
    assert_eq!(unstaged, vec!["a.txt"], "worktree edit survives unstaging");
}

#[tokio::test]
async fn unstage_works_on_an_unborn_branch() {
    let mut repo = TestRepo::init();
    repo.write_file("first.txt", "hello\n");
    repo.git(&["add", "--", "first.txt"]);
    let (exec, cancel) = env();

    stage::unstage_paths(&exec, &repo.path, &["first.txt".into()], &cancel)
        .await
        .expect("unstage on unborn HEAD");
    let (staged, _, untracked) = buckets(&repo).await;
    assert!(staged.is_empty());
    assert_eq!(untracked, vec!["first.txt"], "file is back to untracked");
}

#[tokio::test]
async fn discard_and_clean_reset_the_working_tree() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("a.txt", "broken\n");
    repo.write_file("junk.txt", "junk\n");
    let (exec, cancel) = env();

    stage::discard_worktree(&exec, &repo.path, &["a.txt".into()], &cancel)
        .await
        .expect("discard");
    stage::remove_untracked(&exec, &repo.path, &["junk.txt".into()], &cancel)
        .await
        .expect("clean");

    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert!(staged.is_empty() && unstaged.is_empty() && untracked.is_empty());
    assert!(!repo.path.join("junk.txt").exists());
}

/// Discarding from the staged side takes both sides with it, whatever
/// shape the staged change has: an edit goes back to HEAD, a file staged
/// as new leaves the disk, and a rename needs both of its names to be
/// undone in one go.
#[tokio::test]
async fn discard_to_head_undoes_every_staged_shape() {
    let mut repo = TestRepo::init();
    repo.commit_file("kept.txt", "one\n", "root");
    repo.commit_file("moved.txt", "move me\n", "second");
    repo.write_file("kept.txt", "staged\n");
    repo.git(&["add", "--", "kept.txt"]);
    // Staged on both sides: the worktree has gone on past the index.
    repo.write_file("kept.txt", "and dirty\n");
    repo.write_file("fresh.txt", "brand new\n");
    repo.git(&["add", "--", "fresh.txt"]);
    repo.git(&["mv", "moved.txt", "elsewhere.txt"]);
    let (exec, cancel) = env();

    stage::discard_to_head(
        &exec,
        &repo.path,
        &[
            "kept.txt".into(),
            "fresh.txt".into(),
            // Both names of the rename, or the old one stays staged as a
            // deletion.
            "elsewhere.txt".into(),
            "moved.txt".into(),
        ],
        &cancel,
    )
    .await
    .expect("discard to head");

    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert!(
        staged.is_empty() && unstaged.is_empty() && untracked.is_empty(),
        "nothing is left over: {staged:?} {unstaged:?} {untracked:?}"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path.join("kept.txt")).unwrap(),
        "one\n"
    );
    assert!(repo.path.join("moved.txt").exists(), "the rename is undone");
    assert!(!repo.path.join("elsewhere.txt").exists());
    assert!(
        !repo.path.join("fresh.txt").exists(),
        "HEAD has no such file, so discarding it takes the file"
    );
}

#[tokio::test]
async fn discard_to_head_works_on_an_unborn_branch() {
    let mut repo = TestRepo::init();
    repo.write_file("first.txt", "hello\n");
    repo.git(&["add", "--", "first.txt"]);
    let (exec, cancel) = env();

    stage::discard_to_head(&exec, &repo.path, &["first.txt".into()], &cancel)
        .await
        .expect("discard to head on unborn HEAD");
    let (staged, _, untracked) = buckets(&repo).await;
    assert!(staged.is_empty() && untracked.is_empty());
    assert!(!repo.path.join("first.txt").exists());
}

/// Two well-separated edits produce two hunks; staging only the second
/// must leave the first out of the index.
#[tokio::test]
async fn stage_a_single_hunk() {
    let mut repo = TestRepo::init();
    let base: String = (1..=20).map(|n| format!("line {n}\n")).collect();
    repo.commit_file("f.txt", &base, "root");
    let edited = base
        .replace("line 2\n", "line 2 EDITED\n")
        .replace("line 18\n", "line 18 EDITED\n");
    repo.write_file("f.txt", &edited);
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    let target = DiffTarget::Unstaged {
        path: "f.txt".into(),
    };
    assert_eq!(
        stage::hunk_count(&exec, &repo.path, &target, &cancel)
            .await
            .expect("hunk count"),
        2
    );

    stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(1)],
        fp(&repo_info, &target).await,
        &cancel,
    )
    .await
    .expect("stage hunk");

    let staged_content = indexed(&mut repo, "f.txt");
    assert!(
        staged_content.contains("line 18 EDITED"),
        "second hunk staged"
    );
    assert!(
        !staged_content.contains("line 2 EDITED"),
        "first hunk untouched: {staged_content}"
    );
    // The file is now both staged and unstaged.
    let (staged, unstaged, _) = buckets(&repo).await;
    assert_eq!(staged, vec!["f.txt"]);
    assert_eq!(unstaged, vec!["f.txt"]);
}

/// One hunk, several changed lines, only one of them staged.
#[tokio::test]
async fn stage_a_single_line() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "a\nb\nc\n", "root");
    repo.write_file("f.txt", "a\nB\nc\nD\n");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;
    let target = DiffTarget::Unstaged {
        path: "f.txt".into(),
    };

    // Body: 0 " a", 1 "-b", 2 "+B", 3 " c", 4 "+D".
    stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::lines(0, [4])],
        fp(&repo_info, &target).await,
        &cancel,
    )
    .await
    .expect("stage one line");

    assert_eq!(
        indexed(&mut repo, "f.txt"),
        "a\nb\nc\nD",
        "only the appended line reached the index"
    );
}

/// Throwing away one hunk of an unstaged diff leaves the other hunk on
/// disk and the index where it was.
#[tokio::test]
async fn discard_a_single_hunk() {
    let mut repo = TestRepo::init();
    let base: String = (1..=20).map(|n| format!("line {n}\n")).collect();
    repo.commit_file("f.txt", &base, "root");
    // Staged first, so the discard has an index side it must not touch.
    repo.write_file("f.txt", &base.replace("line 10\n", "line 10 STAGED\n"));
    repo.git(&["add", "--", "f.txt"]);
    let staged_content = indexed(&mut repo, "f.txt");
    repo.write_file(
        "f.txt",
        &base
            .replace("line 2\n", "line 2 EDITED\n")
            .replace("line 10\n", "line 10 STAGED\n")
            .replace("line 18\n", "line 18 EDITED\n"),
    );
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    stage::discard_partial(
        &exec,
        &repo_info,
        &DiffTarget::Unstaged {
            path: "f.txt".into(),
        },
        &[HunkSelect::whole(0)],
        fp(
            &repo_info,
            &DiffTarget::Unstaged {
                path: "f.txt".into(),
            },
        )
        .await,
        &cancel,
    )
    .await
    .expect("discard hunk");

    let on_disk = std::fs::read_to_string(repo.path.join("f.txt")).unwrap();
    assert!(
        !on_disk.contains("line 2 EDITED"),
        "the chosen hunk is gone"
    );
    assert!(on_disk.contains("line 18 EDITED"), "the other hunk stays");
    assert!(on_disk.contains("line 10 STAGED"), "the staged edit stays");
    assert_eq!(
        indexed(&mut repo, "f.txt"),
        staged_content,
        "the index is untouched"
    );
}

#[tokio::test]
async fn discard_a_single_line() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "a\nb\nc\n", "root");
    repo.write_file("f.txt", "a\nB\nc\nD\n");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    // Body: 0 " a", 1 "-b", 2 "+B", 3 " c", 4 "+D".
    stage::discard_partial(
        &exec,
        &repo_info,
        &DiffTarget::Unstaged {
            path: "f.txt".into(),
        },
        &[HunkSelect::lines(0, [4])],
        fp(
            &repo_info,
            &DiffTarget::Unstaged {
                path: "f.txt".into(),
            },
        )
        .await,
        &cancel,
    )
    .await
    .expect("discard one line");

    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).unwrap(),
        "a\nB\nc\n",
        "only the appended line went"
    );
}

/// Staging a deletion alone (its replacement line stays unstaged).
#[tokio::test]
async fn stage_only_a_deletion() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "a\nb\nc\n", "root");
    repo.write_file("f.txt", "a\nB\nc\n");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    stage::apply_partial(
        &exec,
        &repo_info,
        &DiffTarget::Unstaged {
            path: "f.txt".into(),
        },
        &[HunkSelect::lines(0, [1])],
        fp(
            &repo_info,
            &DiffTarget::Unstaged {
                path: "f.txt".into(),
            },
        )
        .await,
        &cancel,
    )
    .await
    .expect("stage deletion");

    assert_eq!(indexed(&mut repo, "f.txt"), "a\nc");
}

/// Two adjacent lines replaced at once: a diff lists both deletions before
/// both additions, so staging only the first must not float the untouched
/// line above its own replacement.
#[tokio::test]
async fn stage_the_first_line_of_a_two_line_replacement() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\ntwo\ntail\n", "root");
    repo.write_file("f.txt", "ONE\nTWO\ntail\n");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    // Body: 0 "-one", 1 "-two", 2 "+ONE", 3 "+TWO".
    stage::apply_partial(
        &exec,
        &repo_info,
        &DiffTarget::Unstaged {
            path: "f.txt".into(),
        },
        &[HunkSelect::lines(0, [0, 2])],
        fp(
            &repo_info,
            &DiffTarget::Unstaged {
                path: "f.txt".into(),
            },
        )
        .await,
        &cancel,
    )
    .await
    .expect("stage the first replacement");

    assert_eq!(indexed(&mut repo, "f.txt"), "ONE\ntwo\ntail");
}

/// The mirror case: unstaging the second line of a staged replacement.
#[tokio::test]
async fn unstage_the_second_line_of_a_two_line_replacement() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\ntwo\ntail\n", "root");
    repo.write_file("f.txt", "ONE\nTWO\ntail\n");
    repo.git(&["add", "--", "f.txt"]);
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    // Body: 0 "-one", 1 "-two", 2 "+ONE", 3 "+TWO".
    stage::apply_partial(
        &exec,
        &repo_info,
        &DiffTarget::Staged {
            path: "f.txt".into(),
            orig_path: None,
        },
        &[HunkSelect::lines(0, [1, 3])],
        fp(
            &repo_info,
            &DiffTarget::Staged {
                path: "f.txt".into(),
                orig_path: None,
            },
        )
        .await,
        &cancel,
    )
    .await
    .expect("unstage the second replacement");

    assert_eq!(indexed(&mut repo, "f.txt"), "ONE\ntwo\ntail");
}

/// A file whose last line has no newline. Staging the line above it leaves
/// that last line as context, and the patch only applies while the context
/// line keeps the `\ No newline at end of file` marker that described it.
#[tokio::test]
async fn stage_a_line_above_a_missing_trailing_newline() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\ntwo", "root");
    repo.write_file("f.txt", "ONE\nTWO");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    // Body: 0 "-one", 1 "-two", 2 marker, 3 "+ONE", 4 "+TWO", 5 marker.
    stage::apply_partial(
        &exec,
        &repo_info,
        &DiffTarget::Unstaged {
            path: "f.txt".into(),
        },
        &[HunkSelect::lines(0, [0, 3])],
        fp(
            &repo_info,
            &DiffTarget::Unstaged {
                path: "f.txt".into(),
            },
        )
        .await,
        &cancel,
    )
    .await
    .expect("stage the first line");

    assert_eq!(
        repo.git_raw(&["show", ":f.txt"]),
        b"ONE\ntwo",
        "the line left alone still ends the file without a newline"
    );
}

/// Unstaging one line of a fully staged change (`git apply -R`).
#[tokio::test]
async fn unstage_a_single_line() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "a\nb\nc\n", "root");
    repo.write_file("f.txt", "a\nB\nc\nD\n");
    repo.git(&["add", "--", "f.txt"]);
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    // Staged diff body: 0 " a", 1 "-b", 2 "+B", 3 " c", 4 "+D".
    // Unstage the b→B replacement, keep the appended D staged.
    stage::apply_partial(
        &exec,
        &repo_info,
        &DiffTarget::Staged {
            path: "f.txt".into(),
            orig_path: None,
        },
        &[HunkSelect::lines(0, [1, 2])],
        fp(
            &repo_info,
            &DiffTarget::Staged {
                path: "f.txt".into(),
                orig_path: None,
            },
        )
        .await,
        &cancel,
    )
    .await
    .expect("unstage lines");

    assert_eq!(indexed(&mut repo, "f.txt"), "a\nb\nc\nD");
    // The working tree is untouched by unstaging.
    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).unwrap(),
        "a\nB\nc\nD\n"
    );
}

/// Partially staging an untracked file goes through intent-to-add. The
/// file sits in a brand-new directory on purpose: `status -uall` names it
/// per file rather than folding the directory, which is what gives it a
/// diff to select from.
#[tokio::test]
async fn stage_part_of_an_untracked_file() {
    let mut repo = TestRepo::init();
    repo.commit_file("seed.txt", "seed\n", "root");
    repo.write_file("newdir/new.txt", "keep\ndrop\n");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    let (_, _, untracked) = buckets(&repo).await;
    assert_eq!(untracked, vec!["newdir/new.txt"]);

    stage::apply_partial(
        &exec,
        &repo_info,
        &DiffTarget::Untracked {
            path: "newdir/new.txt".into(),
        },
        &[HunkSelect::lines(0, [0])],
        fp(
            &repo_info,
            &DiffTarget::Untracked {
                path: "newdir/new.txt".into(),
            },
        )
        .await,
        &cancel,
    )
    .await
    .expect("stage part of a new file");

    assert_eq!(indexed(&mut repo, "newdir/new.txt"), "keep");
    let (staged, unstaged, _) = buckets(&repo).await;
    assert_eq!(staged, vec!["newdir/new.txt"]);
    assert_eq!(unstaged, vec!["newdir/new.txt"], "the rest stays unstaged");
}

/// CRLF content must round-trip byte-for-byte through the rebuilt patch.
#[tokio::test]
async fn stage_a_hunk_of_a_crlf_file() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "a\r\nb\r\nc\r\n", "root");
    repo.write_file("f.txt", "a\r\nB\r\nc\r\n");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    stage::apply_partial(
        &exec,
        &repo_info,
        &DiffTarget::Unstaged {
            path: "f.txt".into(),
        },
        &[HunkSelect::whole(0)],
        fp(
            &repo_info,
            &DiffTarget::Unstaged {
                path: "f.txt".into(),
            },
        )
        .await,
        &cancel,
    )
    .await
    .expect("stage crlf hunk");

    let blob = repo.git_raw(&["show", ":f.txt"]);
    assert_eq!(blob, b"a\r\nB\r\nc\r\n", "line endings preserved");
}

/// A file without a trailing newline: staging the whole hunk must keep
/// the missing newline. (Selecting only an addition after the unterminated
/// line is the one shape a partial patch cannot express — the line before
/// it would gain a newline — and no workaround quietly widens the
/// selection: P3-確認事項 触らないと決めたもの.)
#[tokio::test]
async fn stage_a_file_without_a_trailing_newline() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "keep\nold", "root");
    repo.write_file("f.txt", "keep\nold\nnew");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;
    let target = DiffTarget::Unstaged {
        path: "f.txt".into(),
    };

    // Body: 0 " keep", 1 "-old", 2 "\ No newline", 3 "+old", 4 "+new",
    // 5 "\ No newline" — git rewrites the last line as a replacement.
    stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        fp(&repo_info, &target).await,
        &cancel,
    )
    .await
    .expect("stage the whole hunk");
    assert_eq!(repo.git_raw(&["show", ":f.txt"]), b"keep\nold\nnew");
}

/// Paths are pathspecs to git, and pathspecs glob by default. A file whose
/// name contains glob characters must only ever stage itself.
#[tokio::test]
async fn a_glob_shaped_filename_stages_only_itself() {
    let mut repo = TestRepo::init();
    repo.commit_file("seed.txt", "seed\n", "root");
    repo.write_file("[ab].txt", "bracketed\n");
    repo.write_file("a.txt", "plain\n");
    let (exec, cancel) = env();

    stage::stage_paths(&exec, &repo.path, &["[ab].txt".into()], &cancel)
        .await
        .expect("stage");

    let (staged, _, untracked) = buckets(&repo).await;
    assert_eq!(staged, vec!["[ab].txt"]);
    assert_eq!(untracked, vec!["a.txt"], "the glob did not expand");
}

#[tokio::test]
async fn staging_a_commit_diff_is_rejected() {
    let mut repo = TestRepo::init();
    let oid = repo.commit_file("f.txt", "x\n", "root");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;

    let err = stage::apply_partial(
        &exec,
        &repo_info,
        &DiffTarget::Commit {
            oid: platitude_core::Oid::from_hex_str(&oid).unwrap(),
            parent: None,
            path: "f.txt".into(),
            orig_path: None,
        },
        &[HunkSelect::whole(0)],
        0,
        &cancel,
    )
    .await
    .expect_err("committed diffs are not stageable");
    assert!(err.to_string().contains("cannot be staged"));
}

/// A fresh `git init` has no HEAD and an empty index; emptying that index
/// is a no-op, not a fatal (実測 2026-08-07: without --ignore-unmatch,
/// `git rm --cached -r -- .` exits 128 on "did not match any files").
#[tokio::test]
async fn unstage_all_on_an_unborn_empty_index_succeeds() {
    let repo = TestRepo::init();
    let (exec, cancel) = env();
    stage::unstage_all(&exec, &repo.path, &cancel)
        .await
        .expect("unstaging nothing succeeds at its job");
}

/// A selection that indexes a diff the file no longer produces is a
/// refusal, not a write that quietly did nothing: the graph refresh
/// after a "successful" no-op would show the user nothing happened,
/// with no words saying why.
#[tokio::test]
async fn a_vanished_selection_is_an_error_not_a_silent_success() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("a.txt", "one changed\n");
    let repo_info = info(&repo).await;
    let (exec, cancel) = env();

    let target = DiffTarget::Unstaged {
        path: "a.txt".into(),
    };
    let err = stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(99)],
        fp(&repo_info, &target).await,
        &cancel,
    )
    .await
    .expect_err("hunk 99 is not in the diff");
    assert!(format!("{err}").contains("no longer"), "{err}");

    let err = stage::discard_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(99)],
        fp(&repo_info, &target).await,
        &cancel,
    )
    .await
    .expect_err("same refusal on the discarding side");
    assert!(format!("{err}").contains("no longer"), "{err}");
}

/// A failed partial stage of an untracked file must not leave the
/// intent-to-add mark behind — the file would silently change buckets
/// (and with it, which discard the row offers).
#[tokio::test]
async fn a_failed_untracked_partial_stage_leaves_the_file_untracked() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("new.txt", "fresh\n");
    let repo_info = info(&repo).await;
    let (exec, cancel) = env();

    let target = DiffTarget::Untracked {
        path: "new.txt".into(),
    };
    stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(99)],
        fp(&repo_info, &target).await,
        &cancel,
    )
    .await
    .expect_err("hunk 99 is not in the diff");

    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert!(staged.is_empty(), "no half-staged leftovers: {staged:?}");
    assert!(unstaged.is_empty(), "{unstaged:?}");
    assert_eq!(untracked, vec!["new.txt"], "back in the untracked bucket");
}

/// The fingerprint the UI would carry: taken from the same diff the
/// selection addresses, before anything changes it.
async fn fp(repo_info: &RepoInfo, target: &DiffTarget) -> u64 {
    let (exec, cancel) = env();
    platitude_core::details::file_diff_with_fingerprint(&exec, &repo_info.workdir, target, &cancel)
        .await
        .expect("diff for fingerprint")
        .1
}

/// A selection carried from an older diff is refused once the file
/// changes: the fingerprint the UI saw no longer matches the re-run
/// bytes — on both the staging and the discarding side. This is the
/// formatter-on-save case: the file moves on after the diff was read
/// but before the queued write runs, and positional indices would land
/// on the wrong hunk.
#[tokio::test]
async fn a_selection_from_a_stale_diff_is_refused() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\ntwo\nthree\n", "root");
    repo.write_file("a.txt", "one\ntwo changed\nthree\n");
    let repo_info = info(&repo).await;
    let (exec, cancel) = env();
    let target = DiffTarget::Unstaged {
        path: "a.txt".into(),
    };
    let seen = fp(&repo_info, &target).await;

    repo.write_file("a.txt", "prelude\none\ntwo changed\nthree\n");

    let err = stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        seen,
        &cancel,
    )
    .await
    .expect_err("stale fingerprint is refused");
    assert!(format!("{err}").contains("changed since"), "{err}");

    let err = stage::discard_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        seen,
        &cancel,
    )
    .await
    .expect_err("stale fingerprint refuses the discard too");
    assert!(format!("{err}").contains("changed since"), "{err}");

    let (staged, unstaged, _) = buckets(&repo).await;
    assert!(staged.is_empty(), "nothing was staged: {staged:?}");
    assert_eq!(unstaged, vec!["a.txt"], "nothing was discarded");
}

/// An untracked partial stage checks the fingerprint against the same
/// `--no-index` bytes the UI read — before the intent-to-add mark, which
/// changes what the diff command even is. A stale one leaves the file
/// fully untracked, mark and all.
#[tokio::test]
async fn a_stale_untracked_selection_is_refused_before_the_mark() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.write_file("new.txt", "fresh\n");
    let repo_info = info(&repo).await;
    let (exec, cancel) = env();
    let target = DiffTarget::Untracked {
        path: "new.txt".into(),
    };
    let seen = fp(&repo_info, &target).await;

    repo.write_file("new.txt", "fresh\nand more\n");

    let err = stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        seen,
        &cancel,
    )
    .await
    .expect_err("stale fingerprint is refused");
    assert!(format!("{err}").contains("changed since"), "{err}");

    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert!(
        staged.is_empty() && unstaged.is_empty(),
        "no intent-to-add mark left: {staged:?} {unstaged:?}"
    );
    assert_eq!(untracked, vec!["new.txt"]);
}

/// A conflicted file's diff is the combined form, which has no single old
/// side for a rebuilt patch to sit on — `git apply` refuses the shape
/// outright. The pane withholds the pieces there, so nothing should ask;
/// this is the floor under that, and it must say so in this app's words
/// rather than let git complain about a fragment nobody wrote.
#[tokio::test]
async fn no_part_of_a_conflicted_file_can_be_taken_or_thrown_away() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\ntwo\nthree\n", "base");
    repo.git(&["checkout", "-b", "side"]);
    repo.write_file("f.txt", "one\nTHEIRS\nthree\n");
    repo.git(&["commit", "-am", "their side"]);
    repo.git(&["checkout", "main"]);
    repo.write_file("f.txt", "one\nOURS\nthree\n");
    repo.git(&["commit", "-am", "our side"]);
    repo.git_expect_failure(&["merge", "--no-edit", "side"]);

    let (exec, cancel) = env();
    let repo_info = info(&repo).await;
    let target = DiffTarget::Unstaged {
        path: "f.txt".into(),
    };
    let seen = fp(&repo_info, &target).await;

    let err = stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        seen,
        &cancel,
    )
    .await
    .expect_err("a combined diff cannot be staged in pieces");
    assert!(format!("{err}").contains("still conflicted"), "{err}");

    let err = stage::discard_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        seen,
        &cancel,
    )
    .await
    .expect_err("nor thrown away in pieces");
    assert!(format!("{err}").contains("still conflicted"), "{err}");

    // Refused, not half-done: the path is still exactly as git left it.
    let s = status::load(&exec, &repo.path, &cancel)
        .await
        .expect("status");
    assert_eq!(s.conflicted().count(), 1, "still one unmerged path");
    assert!(
        std::fs::read_to_string(repo.path.join("f.txt"))
            .unwrap()
            .contains("<<<<<<<"),
        "the markers are untouched"
    );
}
