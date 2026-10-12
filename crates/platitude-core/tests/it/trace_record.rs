//! A traced write's record (`GitCommand::traced`) goes to a receiver of the
//! write's own — a named pipe on Windows, an AF_UNIX socket elsewhere —
//! and none of it to disk: nothing is left in the repository however its
//! gits run, a fsmonitor daemon one of them starts included, which outlives
//! the write and is not waited for. The reads after the write answer right.
//!
//! Where this git has no daemon (the minimum on Linux) the daemon's tests
//! stand down.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use crate::support::exec::env;
use crate::support::integrate::{current_op, helper};
use crate::support::remote::origin_and_clone;
use crate::support::wait::bounded;
use crate::support::{TestRepo, info};
use platitude_core::commit::{self, CommitOptions};
use platitude_core::integrate::RebaseOutcome;
use platitude_core::remote::{self, PushForce, PushSpec};
use platitude_core::report::ReportKind;
use platitude_core::sequencer;
use platitude_core::status::{self, StatusItem};
use platitude_core::worktrees::{self, WorktreeOn};

/// `git fsmonitor--daemon <verb>` in `dir`, under the suite's isolated
/// configuration.
fn daemon(dir: &Path, verb: &str) -> std::io::Result<std::process::Output> {
    std::process::Command::new("git")
        .args(["fsmonitor--daemon", verb])
        .current_dir(dir)
        .envs(crate::support::exec::isolated_env())
        .output()
}

/// Whether this git can run the daemon at all (`git version
/// --build-options`, the minimum to 2.55): Windows' and macOS's can, and
/// Linux's from 2.55 — not the minimum's.
fn daemon_built() -> bool {
    std::process::Command::new("git")
        .args(["version", "--build-options"])
        .envs(crate::support::exec::isolated_env())
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).contains("feature: fsmonitor--daemon"))
        .expect("spawn git")
}

/// Whether a daemon watches `dir` — none where there is no `dir` yet.
fn watched(dir: &Path) -> bool {
    dir.is_dir() && daemon(dir, "status").expect("spawn git").status.success()
}

/// Stops the daemon of each folder when dropped — ahead of the folders
/// themselves, so it is declared after the repositories.
struct Daemons(Vec<PathBuf>);

impl Drop for Daemons {
    fn drop(&mut self) {
        for dir in &self.0 {
            let _ = daemon(dir, "stop");
        }
    }
}

/// What the repository's scratch folder holds of a record — anything of
/// the trace's name. The folder keeps other things (a stopped reword's
/// message, for its `--continue`).
fn scratch(repo: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(repo.join(".git").join("platitude")) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("TRACE"))
        .collect()
}

/// Each path the product's status lists, with its two letters (`?` for an
/// untracked one).
async fn listed(dir: &Path) -> Vec<String> {
    let (exec, cancel) = env();
    let mut paths: Vec<String> = status::load(&exec, dir, &cancel)
        .await
        .expect("status")
        .items
        .iter()
        .map(|item| match item {
            StatusItem::Tracked {
                staged,
                unstaged,
                path,
                ..
            } => format!("{path} {staged}{unstaged}"),
            other => format!("{} ?", other.path()),
        })
        .collect();
    paths.sort();
    paths
}

/// Three commits on `main`; answers the middle one.
fn three_commits() -> (TestRepo, String) {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let middle = repo.commit_file_id("b.txt", "two\n", "middle");
    repo.commit_file("c.txt", "three\n", "last");
    (repo, middle)
}

/// Turns the daemon on for `repo` by its config file — for a closure that
/// cannot borrow the repository.
fn want_the_daemon(repo: &Path) {
    let config = repo.join(".git").join("config");
    let mut text = std::fs::read_to_string(&config).expect("config");
    text.push_str("[core]\n\tfsmonitor = true\n");
    std::fs::write(&config, text).expect("fsmonitor on");
}

/// Rewords `target` the way the session does. `ready` runs once the plan
/// and the repository are read.
async fn reword(
    repo: &TestRepo,
    target: &str,
    ready: impl FnOnce(),
) -> Result<(), platitude_core::GitError> {
    let (exec, cancel) = env();
    let edit = sequencer::Edit::Reword("reworded\n".into());
    let plan = sequencer::plan_edit(&exec, &repo.path, target, edit, &cancel)
        .await
        .expect("plan");
    let repo_info = info(repo).await;
    ready();
    sequencer::rebase_interactive(
        &exec,
        &repo_info,
        &plan.upstream,
        &plan.steps,
        &plan.options(),
        &helper(),
        &cancel,
    )
    .await
    .map(|outcome| {
        assert!(
            matches!(outcome, RebaseOutcome::Done),
            "the reword ran to its end"
        )
    })
}

async fn push_main(work: &Path) -> Result<(), platitude_core::GitError> {
    let (exec, cancel) = env();
    let spec = PushSpec {
        remote: "origin".into(),
        local: "main".into(),
        remote_branch: "main".into(),
        set_upstream: false,
        force: PushForce::None,
    };
    remote::push(&exec, work, &spec, remote::DEFAULT_NETWORK_TIMEOUT, &cancel).await
}

async fn add_worktree(repo: &TestRepo, at: &Path) -> Result<(), platitude_core::GitError> {
    let (exec, cancel) = env();
    worktrees::add(
        &exec,
        &repo.path,
        &at.to_string_lossy(),
        &WorktreeOn::NewBranch {
            name: "other".into(),
            start: "main".into(),
        },
        "other",
        &cancel,
    )
    .await
}

/// The record's receiver, seen from inside the write: a hook's git is told
/// a pipe or a socket, and the repository's scratch folder holds nothing of
/// the record while the write runs, nor after.
#[tokio::test]
async fn a_traced_write_writes_none_of_its_record_to_disk() {
    let mut repo = TestRepo::init();
    repo.commit_file("seed.txt", "seed\n", "root");
    repo.write_file("a.txt", "content\n");
    repo.git(&["add", "--", "a.txt"]);
    let seen = repo.path.with_file_name("seen");
    let slashed = |path: &Path| path.to_string_lossy().replace('\\', "/");
    repo.write_hook(
        "pre-commit",
        &format!(
            "echo \"target $GIT_TRACE2_EVENT\" > \"{seen}\"\nls -A .git/platitude >> \"{seen}\" 2>/dev/null\nexit 0\n",
            seen = slashed(&seen)
        ),
    );
    let repo_info = info(&repo).await;
    let (exec, cancel) = env();

    commit::commit(
        &exec,
        &repo_info,
        "traced",
        CommitOptions::default(),
        &cancel,
    )
    .await
    .expect("commit");

    let said = std::fs::read_to_string(&seen).expect("the hook ran");
    let mut lines = said.lines();
    let target = lines
        .next()
        .and_then(|line| line.strip_prefix("target "))
        .expect("the hook saw a target");
    assert!(
        target.starts_with(r"\\.\pipe\") || target.starts_with("af_unix:stream:"),
        "a pipe or a socket, no file: {target}"
    );
    let during: Vec<&str> = lines.filter(|name| name.starts_with("TRACE")).collect();
    assert_eq!(
        during,
        Vec::<&str>::new(),
        "nothing of the record while the write ran"
    );
    assert_eq!(scratch(&repo.path), Vec::<String>::new(), "nor after");
}

/// A write that starts the daemon, and what it leaves: the daemon running
/// on, not waited for; nothing in the repository's scratch folder; reads
/// that answer right. `watched_dir` is where the write's gits start it.
async fn leaves_nothing(
    repo: &Path,
    watched_dir: &Path,
    write: impl std::future::Future<Output = Result<(), platitude_core::GitError>>,
) {
    assert!(!watched(watched_dir), "no daemon to begin with");
    bounded("the write", write)
        .await
        .expect("the write goes through");
    assert!(watched(watched_dir), "the write started the daemon");
    assert_eq!(scratch(repo), Vec::<String>::new(), "nothing left");
    std::fs::write(watched_dir.join("after.txt"), "after\n").expect("a change");
    assert_eq!(listed(watched_dir).await, ["after.txt ?"]);
}

#[tokio::test]
async fn a_traced_commit_leaves_nothing_whatever_daemon_it_starts() {
    if !daemon_built() {
        return;
    }
    let mut repo = TestRepo::init();
    repo.commit_file("seed.txt", "seed\n", "root");
    repo.write_file("a.txt", "content\n");
    repo.git(&["add", "--", "a.txt"]);
    let repo_info = info(&repo).await;
    repo.git(&["config", "core.fsmonitor", "true"]);
    let top = repo.path.clone();
    let _daemons = Daemons(vec![top.clone()]);
    let (exec, cancel) = env();

    let write = commit::commit(
        &exec,
        &repo_info,
        "traced",
        CommitOptions::default(),
        &cancel,
    );
    leaves_nothing(&top, &top, write).await;
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "traced");
}

/// The replay's own gits — the `exec`'s `git commit` among them.
#[tokio::test]
async fn a_traced_replay_leaves_nothing_whatever_daemon_it_starts() {
    if !daemon_built() {
        return;
    }
    let (mut repo, middle) = three_commits();
    let top = repo.path.clone();
    let _daemons = Daemons(vec![top.clone()]);

    let write = reword(&repo, &middle, || want_the_daemon(&top));
    leaves_nothing(&top, &top, write).await;
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s", "HEAD~1"]),
        "reworded"
    );
}

/// A `pre-push` hook that reads the worktree: its git is the push's.
#[tokio::test]
async fn a_hook_under_a_traced_push_leaves_nothing_whatever_daemon_it_starts() {
    if !daemon_built() {
        return;
    }
    let (mut bare, mut work) = origin_and_clone();
    work.commit_file("b.txt", "ours\n", "work of our own");
    work.write_hook("pre-push", "git status --porcelain >/dev/null\n");
    work.git(&["config", "core.fsmonitor", "true"]);
    let top = work.path.clone();
    let _daemons = Daemons(vec![top.clone()]);

    leaves_nothing(&top, &top, push_main(&top)).await;
    let sent = work.git(&["rev-parse", "main"]);
    assert_eq!(bare.git(&["rev-parse", "main"]), sent, "the push went");
}

/// The checkout of the new worktree reads its index: the daemon it starts
/// is that worktree's.
#[tokio::test]
async fn a_traced_worktree_add_leaves_nothing_whatever_daemon_it_starts() {
    if !daemon_built() {
        return;
    }
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "a\n", "root");
    repo.git(&["config", "core.fsmonitor", "true"]);
    let other = repo.path.with_file_name("other");
    let _daemons = Daemons(vec![repo.path.clone(), other.clone()]);

    leaves_nothing(&repo.path, &other, add_worktree(&repo, &other)).await;
}

/// The far side of a push to a non-bare repository on this machine that
/// takes it into its worktree (`updateInstead`) is a git of the push's,
/// started without the push's configuration (connect.c `local_repo_env`):
/// the daemon it starts there writes to the push's receiver too, and the
/// push neither waits for it nor leaves anything.
#[tokio::test]
async fn a_local_push_s_far_side_daemon_holds_nothing() {
    if !daemon_built() {
        return;
    }
    let mut far = TestRepo::init();
    far.commit_file("a.txt", "one\n", "root");
    far.git(&["config", "receive.denyCurrentBranch", "updateInstead"]);
    let parent = far.path.parent().expect("the temp dir").to_path_buf();
    let work = parent.join("work");
    let far_path = far.path.to_string_lossy().into_owned();
    far.git_in(&parent, &["clone", "-q", &far_path, "work"]);
    std::fs::write(work.join("b.txt"), "two\n").expect("a change");
    far.git_in(&work, &["add", "--", "b.txt"]);
    far.git_in(&work, &["commit", "-q", "-m", "two"]);
    far.git(&["config", "core.fsmonitor", "true"]);
    let _daemons = Daemons(vec![far.path.clone(), work.clone()]);
    assert!(!watched(&far.path), "no daemon over there to begin with");

    bounded("the push", push_main(&work))
        .await
        .expect("the push goes");
    assert!(watched(&far.path), "the far side started its daemon");
    assert_eq!(scratch(&work), Vec::<String>::new(), "nothing left here");
    assert_eq!(
        far.git(&["ls-files"]),
        "a.txt\nb.txt",
        "taken into its worktree"
    );
}

/// The replay's `git commit`, turned down by its `commit-msg` hook, is
/// still named from the record where the repository wants the daemon. The
/// put-back after it may start the daemon too.
#[tokio::test]
async fn a_replays_hook_is_still_named_where_the_daemon_is_wanted() {
    let (repo, middle) = three_commits();
    repo.write_hook("commit-msg", "exit 1\n");
    let top = repo.path.clone();
    let _daemons = Daemons(vec![top.clone()]);

    let err = reword(&repo, &middle, || want_the_daemon(&top))
        .await
        .expect_err("the hook says no");
    assert_eq!(
        err.report().map(|report| report.kind),
        Some(ReportKind::RewriteCommitHook),
        "{err}"
    );
    assert_eq!(scratch(&top), Vec::<String>::new());
    assert_eq!(current_op(&repo).await, None, "put back");
}
