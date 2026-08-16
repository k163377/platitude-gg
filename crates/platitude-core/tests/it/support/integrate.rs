//! What the integrate tests run git with, and what they read the
//! repository's state back through.

use std::path::PathBuf;

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::integrate::InProgress;
use platitude_core::opstate;
use platitude_core::repo::RepoInfo;

pub async fn info(repo: &TestRepo) -> RepoInfo {
    let (exec, cancel) = env();
    platitude_core::repo::open(&exec, &repo.path, &cancel)
        .await
        .expect("open repo")
}

pub fn helper() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_pg-todo-editor"))
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

pub async fn current_op(repo: &TestRepo) -> Option<InProgress> {
    let (exec, cancel) = env();
    let state = opstate::detect(&exec, &repo.path, &cancel)
        .await
        .expect("op state");
    InProgress::from_state(&state)
}
