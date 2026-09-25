//! What a range's publish state says about the remotes that hold it.

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::publish;

#[tokio::test]
async fn publish_state_distinguishes_pushed_commits() {
    let mut origin = TestRepo::init();
    origin.commit_file("a.txt", "one\n", "root");
    origin.git(&["config", "core.bare", "true"]);

    let mut work = TestRepo::init();
    work.git(&["remote", "add", "origin", &origin.file_url()]);
    work.git(&["fetch", "origin"]);
    work.git(&["checkout", "-b", "main", "origin/main"]);
    work.commit_file("b.txt", "two\n", "local one");
    work.commit_file("c.txt", "three\n", "local two");
    let (exec, cancel) = env();

    let state = publish::state_of(&exec, &work.path, "origin/main..HEAD", &cancel)
        .await
        .expect("state");
    assert_eq!((state.total, state.unpublished), (2, 2));
    assert!(!state.rewrites_published());

    work.git(&["push", "origin", "main"]);
    work.commit_file("d.txt", "four\n", "local three");

    let state = publish::state_of(&exec, &work.path, "HEAD~3..HEAD", &cancel)
        .await
        .expect("state");
    assert_eq!(state.total, 3);
    assert_eq!(state.unpublished, 1);
    assert_eq!(state.published(), 2);
    assert!(state.rewrites_published());

    let tip = publish::state_of(&exec, &work.path, &publish::only("HEAD"), &cancel)
        .await
        .expect("state");
    assert!(!tip.rewrites_published());
    let parent = publish::state_of(&exec, &work.path, &publish::only("HEAD~1"), &cancel)
        .await
        .expect("state");
    assert!(parent.rewrites_published());
}

/// With no remote `state_of` asks the same way; only git's answer to
/// `--not --remotes` naming nothing differs (rules-refs/core.md `periodic`).
mod periodic {
    use super::*;

    #[tokio::test]
    #[ignore = "duplicates publish_state_distinguishes_pushed_commits: not worth the pre-merge run"]
    async fn a_repository_without_remotes_has_nothing_published() {
        let mut repo = TestRepo::init();
        repo.commit_file("a.txt", "one\n", "root");
        repo.commit_file("b.txt", "two\n", "second");
        let (exec, cancel) = env();

        let state = publish::state_of(&exec, &repo.path, "HEAD~1..HEAD", &cancel)
            .await
            .expect("state");
        assert_eq!((state.total, state.unpublished), (1, 1));
        assert!(!state.rewrites_published());
    }
}
