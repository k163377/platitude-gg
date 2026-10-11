//! Parts of the files whose diff is not a plain edit: one git converts on
//! its way in and out (`text eol=crlf`), one staged as new, and one marked
//! intent-to-add — each a header the rebuilt patch has to say right.

use crate::support::exec::env;
use crate::support::stage::{buckets, fp, indexed};
use crate::support::{TestRepo, info};
use platitude_core::details::DiffTarget;
use platitude_core::patch::HunkSelect;
use platitude_core::stage;

fn on_disk(repo: &TestRepo, rel: &str) -> String {
    String::from_utf8(std::fs::read(repo.path.join(rel)).expect("read")).expect("utf-8")
}

/// Twenty numbered lines ended by `eol`, lines 2 and 18 edited where
/// `edited` says so — two hunks.
fn numbered(eol: &str, edited: [bool; 2]) -> String {
    (1..=20)
        .map(|n| {
            let mark = match n {
                2 if edited[0] => " EDITED",
                18 if edited[1] => " EDITED",
                _ => "",
            };
            format!("line {n}{mark}{eol}")
        })
        .collect()
}

/// A file its attributes keep CRLF on disk and LF in the index: the diff is
/// in index space, so a part staged lands LF in the index, and a part thrown
/// away comes back CRLF like every line around it.
#[tokio::test]
async fn a_part_of_a_file_its_attributes_convert_keeps_both_sides_endings() {
    let mut repo = TestRepo::init();
    repo.commit_file(".gitattributes", "f.txt text eol=crlf\n", "attributes");
    repo.write_file("f.txt", &numbered("\r\n", [false; 2]));
    repo.git(&["add", "f.txt"]);
    repo.git(&["commit", "-m", "root"]);
    assert_eq!(
        indexed(&mut repo, "f.txt"),
        numbered("\n", [false; 2]).trim_end()
    );
    repo.write_file("f.txt", &numbered("\r\n", [true; 2]));
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;
    let target = DiffTarget::Unstaged {
        path: "f.txt".into(),
    };

    stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        fp(&repo_info, &target).await,
        &cancel,
    )
    .await
    .expect("stage the first hunk");
    assert_eq!(
        indexed(&mut repo, "f.txt"),
        numbered("\n", [true, false]).trim_end(),
        "the index takes the line, LF as it holds every line"
    );
    assert_eq!(on_disk(&repo, "f.txt"), numbered("\r\n", [true; 2]));

    stage::discard_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        fp(&repo_info, &target).await,
        &cancel,
    )
    .await
    .expect("throw the other hunk away");
    assert_eq!(
        on_disk(&repo, "f.txt"),
        numbered("\r\n", [true, false]),
        "back to the index's line, CRLF as every line on disk"
    );
}

/// A new file staged whole, part of it taken back out: the file stays
/// staged as new with the lines left, the working tree as it was.
#[tokio::test]
async fn unstage_part_of_a_staged_new_file() {
    let mut repo = TestRepo::init();
    repo.commit_file("seed.txt", "seed\n", "root");
    repo.write_file("new.txt", "keep\ndrop\n");
    repo.git(&["add", "new.txt"]);
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;
    let target = DiffTarget::Staged {
        path: "new.txt".into(),
        orig_path: None,
    };

    // Staged diff body: 0 "+keep", 1 "+drop".
    stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::lines(0, [1])],
        fp(&repo_info, &target).await,
        &cancel,
    )
    .await
    .expect("unstage a line of a new file");

    assert_eq!(indexed(&mut repo, "new.txt"), "keep");
    assert_eq!(on_disk(&repo, "new.txt"), "keep\ndrop\n");
    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert_eq!(staged, vec!["new.txt"], "still staged as new");
    assert_eq!(unstaged, vec!["new.txt"], "the line taken out is unstaged");
    assert!(untracked.is_empty(), "{untracked:?}");
    assert_eq!(
        repo.git(&["diff", "--cached", "--name-status"]),
        "A\tnew.txt"
    );
}

/// A file marked intent-to-add, part of its lines thrown away: they go from
/// the disk, the mark stays, and the rest is still there to stage.
#[tokio::test]
async fn discard_part_of_an_intent_to_add_file() {
    let mut repo = TestRepo::init();
    repo.commit_file("seed.txt", "seed\n", "root");
    repo.write_file("new.txt", "keep\ndrop\n");
    repo.git(&["add", "--intent-to-add", "new.txt"]);
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;
    let target = DiffTarget::Unstaged {
        path: "new.txt".into(),
    };

    // Unstaged diff body: 0 "+keep", 1 "+drop".
    stage::discard_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::lines(0, [1])],
        fp(&repo_info, &target).await,
        &cancel,
    )
    .await
    .expect("throw a line of an intent-to-add file away");

    assert_eq!(on_disk(&repo, "new.txt"), "keep\n");
    let (_, unstaged, untracked) = buckets(&repo).await;
    assert_eq!(unstaged, vec!["new.txt"], "still marked, the rest to stage");
    assert!(untracked.is_empty(), "{untracked:?}");
}

/// An untracked file with CRLF endings under `core.autocrlf=true` (Git for
/// Windows' own default): the read the selection was made on (`--no-index`)
/// and the read once it is marked intent-to-add must give the same hunks,
/// or a part of it could never be staged.
#[tokio::test]
async fn a_part_of_an_untracked_file_git_converts_is_staged() {
    let mut repo = TestRepo::init_autocrlf();
    repo.commit_file("seed.txt", "seed\n", "root");
    repo.write_file("newdir/new.txt", "keep\r\ndrop\r\n");
    let (exec, cancel) = env();
    let repo_info = info(&repo).await;
    let target = DiffTarget::Untracked {
        path: "newdir/new.txt".into(),
    };

    stage::apply_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::lines(0, [0])],
        fp(&repo_info, &target).await,
        &cancel,
    )
    .await
    .expect("stage part of a new file git converts");
    assert_eq!(indexed(&mut repo, "newdir/new.txt"), "keep");
    assert_eq!(on_disk(&repo, "newdir/new.txt"), "keep\r\ndrop\r\n");
    let (staged, unstaged, _) = buckets(&repo).await;
    assert_eq!(staged, vec!["newdir/new.txt"]);
    assert_eq!(unstaged, vec!["newdir/new.txt"], "the rest stays unstaged");
}
