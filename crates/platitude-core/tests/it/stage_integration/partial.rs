//! Staging and discarding hunk and line selections.

use crate::support::exec::env;
use crate::support::stage::{buckets, fp, indexed};
use crate::support::{TestRepo, info};
use platitude_core::details::DiffTarget;
use platitude_core::patch::HunkSelect;
use platitude_core::stage;

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
    // Staged first, so the discard has an index side to leave alone.
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

/// Two adjacent lines replaced at once: a diff lists both deletions
/// before both additions, so staging only the first has to keep the
/// untouched line under its own replacement.
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
/// file sits in a brand-new directory on purpose: `status -uall` names
/// it per file, which is what gives it a diff to select
/// from.
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
/// it would gain a newline — and the selection stands as asked:
/// P3-確認事項 触らないと決めたもの.)
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
