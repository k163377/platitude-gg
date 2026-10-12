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
    let dir = crate::support::temp::dir();
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

/// A repository git will not open for its owner (`safe.directory`, made
/// here by git's own test switch) is no "not a repository": git said
/// something else, and its words are the answer.
#[tokio::test]
async fn open_leaves_a_folder_git_will_not_trust_to_gits_words() {
    let repo = TestRepo::init();
    let executor = crate::support::exec::isolated_with(&[("GIT_TEST_ASSUME_DIFFERENT_OWNER", "1")]);
    let cancel = tokio_util::sync::CancellationToken::new();
    let err = bounded("repo open", repo::open(&executor, &repo.path, &cancel))
        .await
        .unwrap_err();
    match err {
        GitError::Failed { stderr, .. } => {
            assert!(stderr.contains("dubious ownership"), "{stderr}");
        }
        other => panic!("git's own refusal, not a reading of it: {other:?}"),
    }
}

/// The screen has a line of its own for a bare repository, and this flag
/// is the only thing it may read to choose it.
#[tokio::test]
async fn open_rejects_a_bare_repository_as_bare() {
    let dir = crate::support::temp::dir();
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

/// The two halves the strip needs to tell "another worktree of what is
/// already open" from "another repository" (デザイン規約 §タブの所作).
#[tokio::test]
async fn place_names_the_worktree_and_the_repository_it_hangs_off() {
    let mut repo_dir = TestRepo::init();
    repo_dir.commit_file("a.txt", "hello\n", "initial");
    let linked = repo_dir.path.with_file_name("linked");
    repo_dir.git(&["worktree", "add", "-b", "topic", &linked.to_string_lossy()]);

    let (executor, cancel) = env();
    let main = bounded(
        "place the worktree",
        repo::place(&executor, &repo_dir.path, &cancel),
    )
    .await
    .unwrap();
    let hung = bounded(
        "place the worktree",
        repo::place(&executor, &linked, &cancel),
    )
    .await
    .unwrap();

    let real = |path: &std::path::Path| std::fs::canonicalize(path).unwrap();
    assert_eq!(real(&main.info.workdir), real(&repo_dir.path));
    assert_eq!(real(&hung.info.workdir), real(&linked));
    assert_eq!(
        real(&main.repo),
        real(&repo_dir.path),
        "the repository's own worktree names it"
    );
    assert_eq!(
        real(&hung.repo),
        real(&repo_dir.path),
        "and the linked worktree answers with the same one"
    );
}

#[tokio::test]
async fn place_from_a_subdirectory_answers_for_the_worktree_around_it() {
    let mut repo_dir = TestRepo::init();
    repo_dir.commit_file("sub/dir/file.txt", "x\n", "nested");
    let linked = repo_dir.path.with_file_name("linked");
    repo_dir.git(&["worktree", "add", "-b", "topic", &linked.to_string_lossy()]);

    let (executor, cancel) = env();
    let deep = linked.join("sub").join("dir");
    let place = bounded("place the worktree", repo::place(&executor, &deep, &cancel))
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

/// A code a command answers by is a failure after all where git says the
/// words the command named, and a failed row of a command that keeps its
/// stdout carries it ahead of stderr. A shell alias stands in for git
/// writing to both streams and exiting with the code.
#[tokio::test]
async fn named_words_and_kept_stdout_reach_the_row() {
    use crate::support::Said;
    use platitude_core::process::CommandEnd;
    use std::sync::Arc;

    let repo_dir = TestRepo::init();
    let said = Arc::new(Said::default());
    let (executor, cancel) = observed_env(said.clone(), Kept::Asked);
    let probe = || {
        GitCommand::new().cwd(&repo_dir.path).args([
            "-c",
            "alias.pggprobe=!echo on stdout; echo refused here >&2; exit 3",
            "pggprobe",
        ])
    };

    for cmd in [
        probe().answers_by_code(3),
        probe()
            .answers_by_code(3)
            .except_when_it_says(&["refused here"]),
        probe().logs_stdout(),
        probe().answers_by_code(3).logs_stdout(),
    ] {
        let out = bounded("the probe", executor.run_unchecked(cmd, &cancel))
            .await
            .expect("spawn");
        assert_eq!(out.code, 3);
    }

    let seen = said.0.lock().unwrap().clone();
    assert_eq!(
        seen,
        vec![
            (CommandEnd::Answered(3), "refused here".to_string()),
            (CommandEnd::Exited(3), "refused here".to_string()),
            (CommandEnd::Exited(3), "on stdout\nrefused here".to_string()),
            (CommandEnd::Answered(3), "refused here".to_string(),),
        ],
        "an answer keeps its row quiet, stdout or none"
    );
}

/// A read must leave `.git/index.lock` alone: a write dies where it cannot
/// take it, and reads run beside writes (the queue orders only writes,
/// `session::write`). A working-tree `git diff` refreshes the index under
/// that lock unless the fixed arguments say not to; `--no-optional-locks`
/// does not reach `diff`.
#[tokio::test]
async fn a_working_tree_read_leaves_the_index_untouched() {
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
        "the working tree diff",
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

/// What a counted handle's count was as the observer heard each command
/// asked for and ended — the end is heard after the reap, inside the run.
struct CountsAtEnds {
    live: std::sync::Arc<platitude_core::process::Live>,
    seen: std::sync::Mutex<Vec<(&'static str, usize)>>,
}

impl platitude_core::process::CommandObserver for CountsAtEnds {
    fn records(&self, _kept: Kept) -> bool {
        true
    }
    fn started(
        &self,
        _display: &str,
        _full: &str,
        _kept: Kept,
        _operation: Option<platitude_core::OperationId>,
    ) -> u64 {
        if let Ok(mut seen) = self.seen.lock() {
            seen.push(("asked", self.live.count()));
        }
        0
    }
    fn finished(
        &self,
        _id: u64,
        _end: platitude_core::process::CommandEnd,
        _waited: u64,
        _elapsed: u64,
        _message: &str,
    ) {
        if let Ok(mut seen) = self.seen.lock() {
            seen.push(("ended", self.live.count()));
        }
    }
}

/// A counted handle counts a git from the spawn to the reap: none when it is
/// asked for, one still as its end is heard (the child reaped), none once the
/// run has answered — and none at all for a command that never spawned. What
/// a session standing down waits out before its folder is renamed
/// (`RepoSession::stand_down`); the guard sits in the one function every
/// command runs through, so a cancelled one is counted by the same lines.
#[tokio::test]
async fn a_counted_git_runs_until_it_is_reaped_and_no_longer() {
    let live = std::sync::Arc::new(platitude_core::process::Live::default());
    let observer = std::sync::Arc::new(CountsAtEnds {
        live: std::sync::Arc::clone(&live),
        seen: std::sync::Mutex::new(Vec::new()),
    });
    let counted = |executor: GitExecutor| {
        executor
            .counted(std::sync::Arc::clone(&live))
            .observed(std::sync::Arc::clone(&observer) as _, Kept::Asked)
    };

    let ran = counted(crate::support::exec::isolated());
    bounded(
        "git version",
        ran.run(GitCommand::new().arg("version"), &CancellationToken::new()),
    )
    .await
    .expect("git answers");
    assert_eq!(live.count(), 0, "reaped and let go by the time it answers");

    let cancelled = CancellationToken::new();
    cancelled.cancel();
    let err = bounded(
        "a cancelled ask",
        ran.run(GitCommand::new().arg("version"), &cancelled),
    )
    .await
    .expect_err("nothing runs");
    assert!(err.is_cancelled(), "{err:?}");

    let missing = counted(GitExecutor::with_program("pgg-no-such-program"));
    let err = bounded(
        "a program that is not there",
        missing.run(GitCommand::new().arg("version"), &CancellationToken::new()),
    )
    .await
    .expect_err("nothing spawns");
    assert!(matches!(err, GitError::GitNotFound { .. }), "{err:?}");

    assert_eq!(
        *observer.seen.lock().unwrap(),
        [
            ("asked", 0),
            ("ended", 1),
            ("asked", 0),
            ("ended", 0),
            ("asked", 0),
            ("ended", 0),
        ],
        "counted from the spawn to the reap, and only a spawn"
    );
    assert_eq!(live.count(), 0);
    bounded("idle", live.idle()).await.expect("every end told");
}

/// A shell loop that holds until `release` exists, started where git starts
/// an alias: the top of the tree. A spin, so nothing it needs is looked up
/// on the PATH git hands its shell.
#[cfg(windows)]
fn held_until(release: &std::path::Path) -> String {
    let release = release.to_string_lossy().replace('\\', "/");
    format!(
        "i=0; until [ -f \"{release}\" ]; do i=$((i+1)); [ \"$i\" -gt 200000000 ] && exit 1; done"
    )
}

/// A run held where Windows would refuse its folder: a `git commit` an
/// alias starts, waiting on its hook, standing in the tree's folder — as
/// `git lfs version` leaves `git-lfs` when it is killed. (A shell does not
/// hold its folder: MSYS opens it shared.) Counted, on a task of its own,
/// and handed over once the hook has spoken, so that git is provably
/// standing there.
#[cfg(windows)]
struct HeldOnItsHook {
    repo: TestRepo,
    live: std::sync::Arc<platitude_core::process::Live>,
    cancel: CancellationToken,
    run: tokio::task::JoinHandle<Result<platitude_core::process::GitOutput, GitError>>,
}

#[cfg(windows)]
#[expect(
    clippy::expect_used,
    reason = "test scaffolding; `allow-*-in-tests` only covers `#[test]` fns"
)]
impl HeldOnItsHook {
    async fn start() -> Self {
        let repo = TestRepo::init();
        repo.write_hook(
            "pre-commit",
            &format!(
                "printf 'started\\n'\n{}\n",
                held_until(&repo.path.join("never-released"))
            ),
        );
        // The hook's words reach stdout: git sends them to stderr.
        let held = GitCommand::new().cwd(&repo.path).args([
            "-c",
            "alias.held=!git commit --allow-empty -m held 2>&1",
            "held",
        ]);
        let live = std::sync::Arc::new(platitude_core::process::Live::default());
        let exec = crate::support::exec::isolated().counted(std::sync::Arc::clone(&live));
        let cancel = CancellationToken::new();
        let (word, spoken) = tokio::sync::oneshot::channel();
        let run = tokio::spawn({
            let cancel = cancel.clone();
            async move {
                let mut word = Some(word);
                let mut said = Vec::new();
                exec.run_streaming(held, &cancel, &mut |chunk| {
                    said.extend_from_slice(chunk);
                    if said.windows(7).any(|w| w == b"started")
                        && let Some(word) = word.take()
                    {
                        let _ = word.send(());
                    }
                })
                .await
            }
        });
        bounded("the hook's word", spoken)
            .await
            .expect("the hook speaks");
        assert_eq!(live.count(), 1, "counted while git waits on its hook");
        Self {
            repo,
            live,
            cancel,
            run,
        }
    }

    /// Waits for the count's end, then renames the folder straight away:
    /// nothing of the run may stand there by then. Judged off the count,
    /// not the run's answer — the count is what an owner standing down
    /// waits on (`RepoSession::stand_down`).
    async fn gone_with_the_count(&self) {
        bounded("the count's end", self.live.idle())
            .await
            .expect("every end told");
        let moved = self.repo.path.with_file_name("moved");
        std::fs::rename(&self.repo.path, &moved)
            .expect("nothing of the run stands in the tree's folder");
        std::fs::rename(&moved, &self.repo.path).expect("and back");
    }
}

/// A cancel ends what git started for the run, not git alone, and the
/// owner's count of running gits comes down only once that is gone too.
#[cfg(windows)]
#[tokio::test]
async fn a_cancel_ends_what_git_started_for_the_run_before_it_answers() {
    let held = HeldOnItsHook::start().await;
    held.cancel.cancel();
    held.gone_with_the_count().await;
    let ran = bounded("the run's answer", held.run)
        .await
        .expect("the run's task ends");
    assert!(ran.as_ref().is_err_and(GitError::is_cancelled), "{ran:?}");
}

/// A run dropped mid-way — its task gone, as a runtime going down takes it
/// — is ended whole all the same, and counted until it is gone.
#[cfg(windows)]
#[tokio::test]
async fn a_run_dropped_mid_way_ends_what_git_started_before_the_count_comes_down() {
    let held = HeldOnItsHook::start().await;
    held.run.abort();
    held.gone_with_the_count().await;
}

/// A run that ends by itself ends nothing it started: the fsmonitor daemon
/// git starts and leaves running on purpose runs on after the run that
/// started it, and stops when it is told to. It never holds the tree's
/// folder — it moves to `$HOME` and watches the tree shared for deletion
/// (git's `fsmonitor--daemon.c`, `fsm-listen-win32.c`) — so `stop`'s
/// answer is all the folder waits on.
#[cfg(windows)]
#[tokio::test]
async fn a_run_that_ends_by_itself_leaves_the_daemon_it_started() {
    let repo = TestRepo::init();
    let (exec, cancel) = env();
    let daemon = |verb: &str| {
        GitCommand::new()
            .cwd(&repo.path)
            .args(["fsmonitor--daemon", verb])
    };
    bounded("the daemon's start", exec.run(daemon("start"), &cancel))
        .await
        .expect("the daemon starts");
    let running = bounded("the daemon's status", exec.run(daemon("status"), &cancel)).await;
    // Told to stop before anything is judged, so a red leaves no daemon.
    let stopped = bounded("the daemon's stop", exec.run(daemon("stop"), &cancel)).await;
    assert!(
        running.is_ok(),
        "the daemon the run left behind still runs: {running:?}"
    );
    stopped.expect("the daemon stops when told");
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
        assert!(
            v.supported(),
            "dev/CI machines must have git >= {}",
            version::minimum_string()
        );
    }
}
