//! What the staging tests read the repository with: the status buckets,
//! the index's content and the selection fingerprint (the opened info is
//! [`super::info`], shared by every write-API test).

use platitude_core::details::DiffTarget;
use platitude_core::repo::RepoInfo;
use platitude_core::status;

use super::TestRepo;
use super::exec::env;

/// Staged / unstaged / untracked paths of the current status.
pub async fn buckets(repo: &TestRepo) -> (Vec<String>, Vec<String>, Vec<String>) {
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
pub fn indexed(repo: &mut TestRepo, path: &str) -> String {
    repo.git(&["show", &format!(":{path}")])
}

/// The fingerprint the UI would carry: taken from the same diff the
/// selection addresses, before anything changes it.
pub async fn fp(repo_info: &RepoInfo, target: &DiffTarget) -> u64 {
    let (exec, cancel) = env();
    platitude_core::details::file_diff_with_fingerprint(&exec, &repo_info.workdir, target, &cancel)
        .await
        .expect("diff for fingerprint")
        .1
}
