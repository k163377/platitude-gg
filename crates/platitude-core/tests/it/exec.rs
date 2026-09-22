//! Integration tests for the execution layer against the real system git.
//!
//! These await the executor directly — no `CaptureSink`, so no `Patience`
//! arms itself — and the test executors carry no stock timeout. `bounded`
//! is the backstop that turns a wedged git into a named failure here.

use crate::support::TestRepo;
use crate::support::exec::{env, observed_env};
use crate::support::wait::bounded;
use platitude_core::process::Kept;
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

/// The empty path is the git on `PATH`, and it is the one the settings
/// file holds for "whichever one this machine resolves". The probe is
/// what a typed-in path is checked with, so this is the reading its
/// default has to keep.
#[tokio::test]
async fn probing_the_empty_path_asks_the_git_on_path() {
    let cancel = CancellationToken::new();
    let probe = bounded("probe PATH", version::probe("", &cancel)).await;
    assert!(probe.answered(), "dev/CI machines must have git: {probe:?}");
    assert!(!probe.version().is_empty(), "{probe:?}");
}

/// A path with nothing at the end of it is told apart from one that ran
/// and said something else — the two need different sentences, and it is
/// the only thing a reader who mistyped a path has to go on.
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

/// Something that runs and is not git is a failure carrying its own
/// words: the reader pointed at a real file and has to be told what
/// it said.
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

/// A folder with no repository in it and a path that is not there at all
/// are the same refusal: nothing to show here, and not bare.
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

/// Every copy of one repository answers with the same repository, and
/// each answers with the copy the folder is actually in — the two halves
/// the strip needs to tell "another copy of what is already open" from
/// "another repository" (デザイン規約 §タブの所作).
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

/// A folder deeper in the tree opens the copy above it, and that copy is
/// filed under the same repository — the same fold the tab strip makes,
/// asked of one answer.
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

/// Two repositories sitting side by side are two, however alike their
/// folders look.
#[tokio::test]
async fn place_keeps_two_repositories_apart() {
    let mut one = TestRepo::init();
    one.commit_file("a.txt", "one\n", "initial");
    let mut two = TestRepo::init();
    two.commit_file("a.txt", "two\n", "initial");

    let (executor, cancel) = env();
    let first = bounded("place the copy", repo::place(&executor, &one.path, &cancel))
        .await
        .unwrap();
    let second = bounded("place the copy", repo::place(&executor, &two.path, &cancel))
        .await
        .unwrap();
    assert_ne!(first.repo, second.repo);
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
    let c1 = repo_dir.commit_file_id("a.txt", "1\n", "one");
    let c2 = repo_dir.commit_file_id("a.txt", "2\n", "two");
    let c3 = repo_dir.commit_file_id("a.txt", "3\n", "three");

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
/// must be reported as a plain exit — which keeps a broken repository
/// off an answered, ok-looking row.
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

/// **A read leaves `.git/index.lock` alone.** The lock a write takes is
/// fatal — `hold_locked_index` dies where it cannot create it — so a
/// read that takes the same lock kills a write running beside it, and
/// the reads run beside the writes: the queue orders the writes against
/// each other (`session::write`). A `git diff` against the work tree
/// refreshes the index, and locks it to do so, unless the fixed
/// arguments say otherwise; `--no-optional-locks` is not that say-so,
/// since `status` asks the flag before it locks and `diff` never
/// asks.
///
/// The index a reader leaves behind is the whole of the evidence, so the
/// stat has to be one a refresh would want to fix: over an index that
/// already matches, a read with the fault and a read without it write the
/// same nothing.
#[tokio::test]
async fn a_work_tree_read_leaves_the_index_untouched() {
    let mut repo_dir = TestRepo::init();
    repo_dir.commit_file("a.txt", "one\n", "add a");

    // The same bytes under a stat the index has not seen — the entry a
    // refresh exists to fix, and the file a reader has nothing to say
    // about.
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(repo_dir.path.join("a.txt"))
        .expect("open the tracked file");
    // The epoch: the stat only has to differ from the one the index
    // recorded, and a fixed date says so without reading a
    // clock.
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
    // And it is still a reader: a file whose stat alone moved is compared
    // by content, and comes out unchanged.
    assert_eq!(out.stdout_utf8(), "", "a stat-only change is not a change");
}
