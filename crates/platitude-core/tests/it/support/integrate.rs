//! What the integrate tests run git with, and what they read the
//! repository's state back through.

use std::path::PathBuf;

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::integrate::InProgress;
use platitude_core::opstate;

pub fn helper() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_pgg-todo-editor"))
}

/// Runs whatever `plan_edit` produced, the way the session does.
pub async fn apply(repo: &TestRepo, plan: &platitude_core::sequencer::EditPlan) {
    let (exec, cancel) = env();
    let repo_info = crate::support::info(repo).await;
    platitude_core::sequencer::rebase_interactive(
        &exec,
        &repo_info,
        &plan.upstream,
        &plan.steps,
        &plan.options(),
        &helper(),
        &cancel,
    )
    .await
    .expect("run the plan");
}

/// main and side both change the same line of `f.txt`.
pub fn conflicting_branches() -> TestRepo {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "base\n", "root");
    repo.git(&["checkout", "-b", "side"]);
    repo.commit_file("f.txt", "side\n", "side change");
    repo.git(&["checkout", "main"]);
    repo.commit_file("f.txt", "main\n", "main change");
    repo
}

/// HEAD on `main` with no commits yet, beside a branch `other` fetched from
/// another repository; `other`'s commit.
pub fn unborn_beside_a_commit() -> (TestRepo, String) {
    let mut source = TestRepo::init();
    let tip = source.commit_file_id("a.txt", "one\n", "root");
    let mut repo = TestRepo::init();
    repo.git(&["fetch", &source.file_url(), "main:refs/heads/other"]);
    (repo, tip)
}

pub async fn current_op(repo: &TestRepo) -> Option<InProgress> {
    let (exec, cancel) = env();
    let state = opstate::detect(&exec, &repo.path, &cancel)
        .await
        .expect("op state");
    InProgress::from_state(&state)
}
