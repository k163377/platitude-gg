//! Integration tests for the execution layer against the real system git.
//!
//! These await the executor directly — no `CaptureSink`, so no `Patience`
//! arms itself — and the test executors carry no stock timeout. `bounded`
//! is the backstop that turns a wedged git into a named failure here.

use crate::support::TestRepo;
use crate::support::exec::{env, observed_env};
use crate::support::wait::bounded;
use platitude_core::{GitCommand, GitError, GitExecutor, repo, version};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn detects_a_supported_git_version() {
    let (executor, cancel) = env();
    let v = bounded("version detect", version::detect(&executor, &cancel))
        .await
        .unwrap();
    assert!(v.supported(), "dev/CI machines must have git >= 2.43");
}

#[tokio::test]
async fn missing_binary_maps_to_git_not_found() {
    let executor = GitExecutor::with_program("definitely-not-a-real-git-binary");
    let cancel = CancellationToken::new();
    let err = bounded("version detect", version::detect(&executor, &cancel))
        .await
        .unwrap_err();
    assert!(matches!(err, GitError::GitNotFound { .. }), "got {err:?}");
}

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
async fn open_from_a_subdirectory_resolves_the_root() {
    let mut repo_dir = TestRepo::init();
    repo_dir.commit_file("sub/dir/file.txt", "x\n", "nested");

    let (executor, cancel) = env();
    let sub = repo_dir.path.join("sub").join("dir");
    let info = bounded("repo open", repo::open(&executor, &sub, &cancel))
        .await
        .unwrap();

    let expected = std::fs::canonicalize(&repo_dir.path).unwrap();
    assert_eq!(std::fs::canonicalize(&info.workdir).unwrap(), expected);
}

#[tokio::test]
async fn open_rejects_a_non_repository() {
    let dir = tempfile::tempdir().unwrap();
    let (executor, cancel) = env();
    let err = bounded("repo open", repo::open(&executor, dir.path(), &cancel))
        .await
        .unwrap_err();
    assert!(
        matches!(err, GitError::NotARepository { bare: false, .. }),
        "got {err:?}"
    );
}

#[tokio::test]
async fn open_rejects_a_missing_path() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nope");
    let (executor, cancel) = env();
    let err = bounded("repo open", repo::open(&executor, &missing, &cancel))
        .await
        .unwrap_err();
    assert!(
        matches!(err, GitError::NotARepository { bare: false, .. }),
        "got {err:?}"
    );
}

/// A bare repository is refused like any other folder that cannot be
/// shown, but it says so about itself: there is a repository here, and it
/// has no work tree. The screen has a line of its own for that, and this
/// flag is the only thing it may read to choose it.
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

#[tokio::test]
async fn streaming_delivers_all_stdout_chunks() {
    let mut repo_dir = TestRepo::init();
    let c1 = repo_dir.commit_file("a.txt", "1\n", "one");
    let c2 = repo_dir.commit_file("a.txt", "2\n", "two");
    let c3 = repo_dir.commit_file("a.txt", "3\n", "three");

    let (executor, cancel) = env();
    let cmd = GitCommand::new()
        .cwd(&repo_dir.path)
        .args(["log", "--format=%H"]);

    let mut collected = Vec::new();
    bounded(
        "the streamed log",
        executor.run_streaming(cmd, &cancel, &mut |chunk| {
            collected.extend_from_slice(chunk)
        }),
    )
    .await
    .unwrap();

    let text = String::from_utf8(collected).unwrap();
    let shas: Vec<&str> = text.lines().collect();
    assert_eq!(shas, vec![c3.as_str(), c2.as_str(), c1.as_str()]);
}

#[tokio::test]
async fn pre_cancelled_token_short_circuits() {
    let mut repo_dir = TestRepo::init();
    repo_dir.commit_file("a.txt", "hello\n", "initial");

    let (executor, cancel) = env();
    cancel.cancel();
    let cmd = GitCommand::new().cwd(&repo_dir.path).args(["status"]);
    let err = bounded("the pre-cancelled command", executor.run(cmd, &cancel))
        .await
        .unwrap_err();
    assert!(err.is_cancelled(), "got {err:?}");
}

/// `answers_by_code` marks 0 and 1 as answers for the command log; any
/// other exit from the same command (a fatal 128) is still a failure and
/// must be reported as a plain exit — a broken repository must not show
/// up as an answered, ok-looking row.
#[tokio::test]
async fn answers_by_code_reports_only_zero_and_one_as_answers() {
    use crate::support::Ends;
    use platitude_core::process::CommandEnd;
    use std::sync::Arc;

    let mut repo_dir = TestRepo::init();
    let c1 = repo_dir.commit_file("a.txt", "1\n", "one");
    let c2 = repo_dir.commit_file("a.txt", "2\n", "two");

    let ends = Arc::new(Ends::default());
    let (executor, cancel) = observed_env(ends.clone(), true);
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
