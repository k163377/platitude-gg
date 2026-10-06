//! `git clone`, offline: the far side is a `file://` URL of a local
//! repository (実装計画 §10). git's own refusals are in [`periodic`].

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::GitError;
use platitude_core::remote;

/// A `file://` remote answers instantly; the budget just has to exist.
const NET: std::time::Duration = remote::DEFAULT_NETWORK_TIMEOUT;

#[tokio::test]
async fn a_clone_lands_in_the_folder_it_was_named() {
    let mut seed = TestRepo::init();
    seed.commit_file("a.txt", "one\n", "root");
    let dir = tempfile::tempdir().expect("create tempdir");
    let into = dir.path().join("landed");
    let (exec, cancel) = env();

    remote::clone(&exec, &seed.file_url(), &into, NET, &cancel)
        .await
        .expect("clone");

    assert_eq!(
        std::fs::read_to_string(into.join("a.txt")).unwrap(),
        "one\n",
        "the working tree is checked out"
    );
    platitude_core::repo::open(&exec, &into, &cancel)
        .await
        .expect("the clone opens as a repository");
    let remotes = remote::list(&exec, &into, &cancel).await.expect("remotes");
    assert_eq!(
        remotes.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        vec!["origin"],
        "git names the first remote itself"
    );
}

/// A destination the picker has never been to is still a destination.
#[tokio::test]
async fn the_folders_above_the_destination_are_made() {
    let mut seed = TestRepo::init();
    seed.commit_file("a.txt", "one\n", "root");
    let dir = tempfile::tempdir().expect("create tempdir");
    let into = dir.path().join("not").join("there").join("yet");
    let (exec, cancel) = env();

    remote::clone(&exec, &seed.file_url(), &into, NET, &cancel)
        .await
        .expect("clone");

    assert!(into.join("a.txt").is_file());
}

/// git refusing a clone. The failure comes back untouched, so these record
/// git's answer, not a decision of ours — the full gate runs them
/// (`-- --ignored ::periodic::`), not every change.
mod periodic {
    use super::*;

    /// The destination is git's to judge; its refusal is what the dialog
    /// shows (デザイン規約 §リポジトリを取り寄せる).
    #[tokio::test]
    #[ignore = "git's own refusal of a taken destination: not worth the pre-merge run"]
    async fn a_destination_that_is_taken_comes_back_as_gits_own_refusal() {
        let mut seed = TestRepo::init();
        seed.commit_file("a.txt", "one\n", "root");
        let dir = tempfile::tempdir().expect("create tempdir");
        let into = dir.path().join("landed");
        let (exec, cancel) = env();
        remote::clone(&exec, &seed.file_url(), &into, NET, &cancel)
            .await
            .expect("first clone");

        let refused = remote::clone(&exec, &seed.file_url(), &into, NET, &cancel).await;

        match refused {
            Err(GitError::Failed { code, stderr, .. }) => {
                assert_ne!(code, 0);
                assert!(!stderr.trim().is_empty(), "git said why");
            }
            other => panic!("a taken destination is a refusal, not {other:?}"),
        }
    }

    /// The answer the dialog stays open for.
    #[tokio::test]
    #[ignore = "git's own failure on a URL nothing answers: not worth the pre-merge run"]
    async fn a_url_that_answers_nothing_comes_back_the_same_way() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let nowhere = dir.path().join("nowhere");
        let into = dir.path().join("landed");
        let (exec, cancel) = env();

        let refused = remote::clone(
            &exec,
            &format!("file:///{}", nowhere.to_string_lossy().replace('\\', "/")),
            &into,
            NET,
            &cancel,
        )
        .await;

        assert!(
            matches!(refused, Err(GitError::Failed { .. })),
            "{refused:?}"
        );
    }
}
