//! The execution layer against the real system git.
//!
//! These await the executor directly (no `CaptureSink`, so no `Patience`,
//! and test executors carry no stock timeout), so `bounded` is the
//! backstop that names a wedged git.

use crate::support::TestRepo;
use crate::support::exec::{env, observed_env};
use crate::support::wait::bounded;
use platitude_core::process::Kept;
use platitude_core::{GitCommand, GitError, GitExecutor, repo, version};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn missing_binary_maps_to_git_not_found() {
    let executor = GitExecutor::with_program("definitely-not-a-real-git-binary");
    let cancel = CancellationToken::new();
    let err = bounded("version detect", version::detect(&executor, &cancel))
        .await
        .unwrap_err();
    assert!(matches!(err, GitError::GitNotFound { .. }), "got {err:?}");
}

/// Settings holds the empty path for "whichever git this machine resolves",
/// and typed-in paths are checked with this probe, so it must keep that.
#[tokio::test]
async fn probing_the_empty_path_asks_the_git_on_path() {
    let cancel = CancellationToken::new();
    let probe = bounded("probe PATH", version::probe("", &cancel)).await;
    assert!(probe.answered(), "dev/CI machines must have git: {probe:?}");
    assert!(!probe.version().is_empty(), "{probe:?}");
}

/// Told apart from one that ran and said something else: a reader who
/// mistyped a path needs a different sentence.
#[tokio::test]
async fn probing_a_path_with_nothing_at_it_answers_missing() {
    let cancel = CancellationToken::new();
    let probe = bounded(
        "probe a missing binary",
        version::probe("definitely-not-a-real-git-binary", &cancel),
    )
    .await;
    assert_eq!(probe, version::Probe::Missing, "{probe:?}");
    assert!(!probe.answered());
    assert_eq!(probe.version(), "");
}

#[tokio::test]
async fn probing_something_that_is_not_git_carries_its_own_words() {
    let repo_dir = TestRepo::init();
    // A file that exists and is not a program: the spawn fails with the
    // OS's own reason on every platform this ships to.
    let not_git = repo_dir.path.join("not-git.txt");
    std::fs::write(&not_git, "I am not git\n").unwrap();

    let cancel = CancellationToken::new();
    let probe = bounded(
        "probe a file that is not git",
        version::probe(&not_git.to_string_lossy(), &cancel),
    )
    .await;
    match probe {
        version::Probe::Failed { message } => assert!(!message.is_empty()),
        other => panic!("expected a failure carrying a reason, got {other:?}"),
    }
}

/// A wrong git directory that is still a directory fails none of the
/// scratch writes that go there (partial stage, commit message, rebase
/// plan) — this is where it is named.
#[tokio::test]
async fn opens_a_valid_repository() {
    let mut repo_dir = TestRepo::init();
    repo_dir.commit_file("a.txt", "hello\n", "initial");

    let (executor, cancel) = env();
    let info = bounded("repo open", repo::open(&executor, &repo_dir.path, &cancel))
        .await
        .unwrap();

    // git prints forward-slash paths; compare canonicalized forms.
    let expected = std::fs::canonicalize(&repo_dir.path).unwrap();
    let actual = std::fs::canonicalize(&info.workdir).unwrap();
    assert_eq!(actual, expected);
    assert!(
        std::fs::canonicalize(&info.git_dir)
            .unwrap()
            .ends_with(".git")
    );
}

#[tokio::test]
async fn open_rejects_a_non_repository() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nope");
    let (executor, cancel) = env();
    for path in [dir.path(), missing.as_path()] {
        let err = bounded("repo open", repo::open(&executor, path, &cancel))
            .await
            .unwrap_err();
        assert!(
            matches!(err, GitError::NotARepository { bare: false, .. }),
            "{}: got {err:?}",
            path.display()
        );
    }
}

/// The screen has a line of its own for a bare repository, and this flag
/// is the only thing it may read to choose it.
#[tokio::test]
async fn open_rejects_a_bare_repository_as_bare() {
    let dir = tempfile::tempdir().unwrap();
    let bare = dir.path().join("origin.git");
    let status = std::process::Command::new("git")
        .args(["init", "--bare", "--quiet"])
        .arg(&bare)
        .status()
        .expect("git init --bare");
    assert!(status.success());

    let (executor, cancel) = env();
    let err = bounded("repo open", repo::open(&executor, &bare, &cancel))
        .await
        .unwrap_err();
    assert!(
        matches!(err, GitError::NotARepository { bare: true, .. }),
        "got {err:?}"
    );
}

/// The two halves the strip needs to tell "another copy of what is already
/// open" from "another repository" (デザイン規約 §タブの所作).
#[tokio::test]
async fn place_names_the_copy_and_the_repository_it_hangs_off() {
    let mut repo_dir = TestRepo::init();
    repo_dir.commit_file("a.txt", "hello\n", "initial");
    let linked = repo_dir.path.with_file_name("linked");
    repo_dir.git(&["worktree", "add", "-b", "topic", &linked.to_string_lossy()]);

    let (executor, cancel) = env();
    let main = bounded(
        "place the copy",
        repo::place(&executor, &repo_dir.path, &cancel),
    )
    .await
    .unwrap();
    let hung = bounded("place the copy", repo::place(&executor, &linked, &cancel))
        .await
        .unwrap();

    let real = |path: &std::path::Path| std::fs::canonicalize(path).unwrap();
    assert_eq!(real(&main.info.workdir), real(&repo_dir.path));
    assert_eq!(real(&hung.info.workdir), real(&linked));
    assert_eq!(
        real(&main.repo),
        real(&repo_dir.path),
        "the repository's own copy names it"
    );
    assert_eq!(
        real(&hung.repo),
        real(&repo_dir.path),
        "and the linked copy answers with the same one"
    );
}

#[tokio::test]
async fn place_from_a_subdirectory_answers_for_the_copy_around_it() {
    let mut repo_dir = TestRepo::init();
    repo_dir.commit_file("sub/dir/file.txt", "x\n", "nested");
    let linked = repo_dir.path.with_file_name("linked");
    repo_dir.git(&["worktree", "add", "-b", "topic", &linked.to_string_lossy()]);

    let (executor, cancel) = env();
    let deep = linked.join("sub").join("dir");
    let place = bounded("place the copy", repo::place(&executor, &deep, &cancel))
        .await
        .unwrap();

    let real = |path: &std::path::Path| std::fs::canonicalize(path).unwrap();
    assert_eq!(real(&place.info.workdir), real(&linked));
    assert_eq!(real(&place.repo), real(&repo_dir.path));
}

#[tokio::test]
async fn failed_commands_surface_gits_stderr() {
    let mut repo_dir = TestRepo::init();
    repo_dir.commit_file("a.txt", "hello\n", "initial");

    let (executor, cancel) = env();
    let cmd =
        GitCommand::new()
            .cwd(&repo_dir.path)
            .args(["rev-parse", "--verify", "does-not-exist"]);
    let err = bounded("the failing command", executor.run(cmd, &cancel))
        .await
        .unwrap_err();
    match err {
        GitError::Failed { code, stderr, .. } => {
            assert_ne!(code, 0);
            assert!(!stderr.is_empty());
        }
        other => panic!("expected Failed, got {other:?}"),
    }
}

/// A fatal 128 from the same command stays a plain exit, which keeps a
/// broken repository off an answered, ok-looking row.
#[tokio::test]
async fn answers_by_code_reports_only_zero_and_one_as_answers() {
    use crate::support::Ends;
    use platitude_core::process::CommandEnd;
    use std::sync::Arc;

    let mut repo_dir = TestRepo::init();
    let c1 = repo_dir.commit_file_id("a.txt", "1\n", "one");
    let c2 = repo_dir.commit_file_id("a.txt", "2\n", "two");

    let ends = Arc::new(Ends::default());
    let (executor, cancel) = observed_env(ends.clone(), Kept::Asked);
    let ancestor = |a: String, b: String| {
        GitCommand::new()
            .cwd(&repo_dir.path)
            .args(["merge-base", "--is-ancestor"])
            .arg(a)
            .arg(b)
            .answers_by_code(1)
    };

    for cmd in [
        ancestor(c1.clone(), c2.clone()), // yes → 0
        ancestor(c2, c1.clone()),         // no → 1
        ancestor("0".repeat(40), c1),     // fatal → 128
    ] {
        let _out = bounded("the ancestry answer", executor.run_unchecked(cmd, &cancel))
            .await
            .expect("spawn");
    }

    let seen = ends.0.lock().unwrap().clone();
    assert_eq!(
        seen,
        vec![
            CommandEnd::Answered(0),
            CommandEnd::Answered(1),
            CommandEnd::Exited(128),
        ]
    );
}

/// A read must leave `.git/index.lock` alone: a write dies where it cannot
/// take it, and reads run beside writes (the queue orders only writes,
/// `session::write`). A work-tree `git diff` refreshes the index under
/// that lock unless the fixed arguments say not to; `--no-optional-locks`
/// does not reach `diff`.
#[tokio::test]
async fn a_work_tree_read_leaves_the_index_untouched() {
    let mut repo_dir = TestRepo::init();
    repo_dir.commit_file("a.txt", "one\n", "add a");

    // Same bytes under a stat the index has not seen: the entry a refresh
    // would rewrite. Over a matching index a faulty read writes nothing.
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(repo_dir.path.join("a.txt"))
        .expect("open the tracked file");
    // A fixed date differs from the recorded stat without reading a clock.
    file.set_times(std::fs::FileTimes::new().set_modified(std::time::SystemTime::UNIX_EPOCH))
        .expect("give it a stat the index has not seen");
    drop(file);

    let index = repo_dir.path.join(".git").join("index");
    let before = std::fs::read(&index).expect("the index the reader starts on");

    let (executor, cancel) = env();
    let out = bounded(
        "the work tree diff",
        executor.run(
            GitCommand::new()
                .cwd(&repo_dir.path)
                .args(["diff", "--no-ext-diff"]),
            &cancel,
        ),
    )
    .await
    .expect("the diff answers");

    assert_eq!(
        std::fs::read(&index).expect("the index the reader left"),
        before,
        "a read rewrote the index, so it held the lock a write dies on"
    );
    assert_eq!(out.stdout_utf8(), "", "a stat-only change is not a change");
}

/// Whether this machine's git meets the minimum is about the machine, not
/// the code (the parse and boundary are `version::tests`), so the full gate
/// runs it (`-- --ignored ::periodic::`), not every change.
mod periodic {
    use super::*;

    #[tokio::test]
    #[ignore = "this machine's git version: not worth the pre-merge run"]
    async fn detects_a_supported_git_version() {
        let (executor, cancel) = env();
        let v = bounded("version detect", version::detect(&executor, &cancel))
            .await
            .unwrap();
        assert!(v.supported(), "dev/CI machines must have git >= 2.43");
    }
}
