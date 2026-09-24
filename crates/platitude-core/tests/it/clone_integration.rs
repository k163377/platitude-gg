//! `git clone`, exercised entirely offline: the far side is a `file://`
//! URL of a second local repository (実装計画 §11.3). git's own refusals
//! of a clone are in [`periodic`].

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
        "the working copy is checked out"
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

/// git makes the folders it is short of, so a destination the picker has
/// never been to is still a destination.
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

/// **What the pre-merge run leaves out**: git refusing a clone — into a
/// destination that holds anything, and from a URL nothing answers. The
/// command is handed to the executor and its failure comes back untouched,
/// so what these record is git's answer rather than a decision of ours.
/// So the full gate runs them (`-- --ignored ::periodic::`) rather than
/// every change.
mod periodic {
    use super::*;

    /// The destination is git's to judge: git refuses one that holds
    /// anything, and its answer is what the dialog shows
    /// (デザイン規約 §リポジトリを取り寄せる).
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

    /// A URL nothing answers is the same shape of answer — the one the dialog
    /// stays open for.
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
