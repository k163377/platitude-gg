//! Staging and discarding whole files.
//!
//! The pre-merge part holds our choices: which command each side and each HEAD
//! takes, and the names a staged rename is undone by. The periodic part
//! records what git's commands then do to the index and the disk.
//!
//! An empty selection never reaches git; that is pinned in `stage::whole`.

use crate::support::TestRepo;
use crate::support::exec::{env, logged};
use crate::support::stage::buckets;
use platitude_core::stage::{self, DiscardSide};

#[tokio::test]
async fn unstage_works_on_an_unborn_branch() {
    let mut repo = TestRepo::init();
    repo.write_file("first.txt", "hello\n");
    repo.git(&["add", "--", "first.txt"]);
    let (exec, cancel) = env();

    stage::unstage_paths(&exec, &repo.path, &["first.txt".into()], &cancel)
        .await
        .expect("unstage on unborn HEAD");
    let (staged, _, untracked) = buckets(&repo).await;
    assert!(staged.is_empty());
    assert_eq!(untracked, vec!["first.txt"], "file is back to untracked");
}

#[tokio::test]
async fn discard_to_head_works_on_an_unborn_branch() {
    let mut repo = TestRepo::init();
    repo.write_file("first.txt", "hello\n");
    repo.git(&["add", "--", "first.txt"]);
    let (exec, cancel) = env();

    stage::discard_to_head(&exec, &repo.path, &["first.txt".into()], &cancel)
        .await
        .expect("discard to head on unborn HEAD");
    let (staged, _, untracked) = buckets(&repo).await;
    assert!(staged.is_empty() && untracked.is_empty());
    assert!(!repo.path.join("first.txt").exists());
}

/// One call over rows from every bucket: each side goes by its own
/// command — one `restore --worktree`, one `clean`, one
/// `restore --staged --worktree`, in that order, however many rows each
/// side had (デザイン規約 §その他の操作).
#[tokio::test]
async fn discard_chosen_takes_each_side_by_its_own_command() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.commit_file("b.txt", "two\n", "second");
    repo.write_file("a.txt", "unstaged edit\n");
    repo.write_file("b.txt", "staged edit\n");
    repo.git(&["add", "--", "b.txt"]);
    repo.write_file("junk.txt", "junk\n");
    let (exec, log, cancel) = logged();

    stage::discard_chosen(
        &exec,
        &repo.path,
        &[
            ("a.txt".into(), DiscardSide::Unstaged),
            ("junk.txt".into(), DiscardSide::Untracked),
            ("b.txt".into(), DiscardSide::Staged),
        ],
        &cancel,
    )
    .await
    .expect("discard chosen");

    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert!(
        staged.is_empty() && unstaged.is_empty() && untracked.is_empty(),
        "something is left over: {staged:?} {unstaged:?} {untracked:?}"
    );
    assert!(!repo.path.join("junk.txt").exists());
    let writes: Vec<String> = log
        .0
        .lock()
        .unwrap()
        .iter()
        .map(|(display, _)| display.clone())
        .filter(|d| d.contains("restore") || d.contains("clean"))
        .collect();
    assert_eq!(writes.len(), 3, "one command per side: {writes:?}");
    assert!(writes[0].contains("--worktree") && !writes[0].contains("--staged"));
    assert!(writes[1].contains("clean"));
    assert!(writes[2].contains("--staged") && writes[2].contains("--worktree"));
}

/// The staged row of a rename is chosen by its new name alone: the old
/// name rides along from status, read inside the write, or its staged
/// deletion would be left standing.
#[tokio::test]
async fn discard_chosen_pulls_a_staged_renames_old_name_from_status() {
    let mut repo = TestRepo::init();
    repo.commit_file("moved.txt", "move me\n", "root");
    repo.git(&["mv", "moved.txt", "elsewhere.txt"]);
    let (exec, cancel) = env();

    stage::discard_chosen(
        &exec,
        &repo.path,
        &[("elsewhere.txt".into(), DiscardSide::Staged)],
        &cancel,
    )
    .await
    .expect("discard chosen");

    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert!(
        staged.is_empty() && unstaged.is_empty() && untracked.is_empty(),
        "the old name stayed staged: {staged:?} {unstaged:?} {untracked:?}"
    );
    assert!(repo.path.join("moved.txt").exists(), "the rename is undone");
    assert!(!repo.path.join("elsewhere.txt").exists());
}

/// A staged copy carries `orig_path` exactly like a rename (`2 C.` under
/// `status.renames=copies`), and its source is left alone: pulling it in
/// would reset work the user never chose.
#[tokio::test]
async fn discard_chosen_leaves_a_staged_copys_source_alone() {
    let mut repo = TestRepo::init();
    repo.commit_file("src.txt", "line one\nline two\n", "root");
    repo.git(&["config", "status.renames", "copies"]);
    repo.write_file("src.txt", "line one\nline two\nline three\n");
    std::fs::copy(repo.path.join("src.txt"), repo.path.join("dup.txt")).expect("copy the file");
    repo.git(&["add", "--", "src.txt", "dup.txt"]);
    let (exec, cancel) = env();

    stage::discard_chosen(
        &exec,
        &repo.path,
        &[("dup.txt".into(), DiscardSide::Staged)],
        &cancel,
    )
    .await
    .expect("discard chosen");

    let (staged, unstaged, untracked) = buckets(&repo).await;
    assert!(!repo.path.join("dup.txt").exists(), "the copy is gone");
    assert_eq!(
        staged,
        vec!["src.txt"],
        "the source keeps its own staged change: {unstaged:?} {untracked:?}"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path.join("src.txt")).unwrap(),
        "line one\nline two\nline three\n",
        "and its bytes on disk"
    );
}

/// Left out of the pre-merge run: git's own `add` / `restore` semantics
/// behind the command lines pinned above. Run by the full gate
/// (`-- --ignored ::periodic::`).
mod periodic {
    use super::*;

    #[tokio::test]
    #[ignore = "git's own add and restore --staged: not worth the pre-merge run"]
    async fn stage_and_unstage_whole_files() {
        let mut repo = TestRepo::init();
        repo.commit_file("a.txt", "one\n", "root");
        repo.write_file("a.txt", "one changed\n");
        repo.write_file("new.txt", "fresh\n");
        let (exec, cancel) = env();

        let (staged, unstaged, untracked) = buckets(&repo).await;
        assert!(staged.is_empty());
        assert_eq!(unstaged, vec!["a.txt"]);
        assert_eq!(untracked, vec!["new.txt"]);

        stage::stage_paths(
            &exec,
            &repo.path,
            &["a.txt".into(), "new.txt".into()],
            &cancel,
        )
        .await
        .expect("stage");
        let (staged, unstaged, untracked) = buckets(&repo).await;
        assert_eq!(staged, vec!["a.txt", "new.txt"]);
        assert!(unstaged.is_empty() && untracked.is_empty());

        stage::unstage_paths(&exec, &repo.path, &["a.txt".into()], &cancel)
            .await
            .expect("unstage");
        let (staged, unstaged, _) = buckets(&repo).await;
        assert_eq!(staged, vec!["new.txt"]);
        assert_eq!(unstaged, vec!["a.txt"], "worktree edit survives unstaging");
    }

    /// Discarding from the staged side takes both sides with it, whatever
    /// shape the staged change has: an edit goes back to HEAD, a file staged
    /// as new leaves the disk, and a rename needs both of its names to be
    /// undone in one go.
    #[tokio::test]
    #[ignore = "git's own restore --staged --worktree: not worth the pre-merge run"]
    async fn discard_to_head_undoes_every_staged_shape() {
        let mut repo = TestRepo::init();
        repo.commit_file("kept.txt", "one\n", "root");
        repo.commit_file("moved.txt", "move me\n", "second");
        repo.write_file("kept.txt", "staged\n");
        repo.git(&["add", "--", "kept.txt"]);
        // Staged on both sides: the worktree has gone on past the index.
        repo.write_file("kept.txt", "and dirty\n");
        repo.write_file("fresh.txt", "brand new\n");
        repo.git(&["add", "--", "fresh.txt"]);
        repo.git(&["mv", "moved.txt", "elsewhere.txt"]);
        let (exec, cancel) = env();

        stage::discard_to_head(
            &exec,
            &repo.path,
            &[
                "kept.txt".into(),
                "fresh.txt".into(),
                // Both names of the rename, or the old one stays staged as a
                // deletion.
                "elsewhere.txt".into(),
                "moved.txt".into(),
            ],
            &cancel,
        )
        .await
        .expect("discard to head");

        let (staged, unstaged, untracked) = buckets(&repo).await;
        assert!(
            staged.is_empty() && unstaged.is_empty() && untracked.is_empty(),
            "nothing is left over: {staged:?} {unstaged:?} {untracked:?}"
        );
        assert_eq!(
            std::fs::read_to_string(repo.path.join("kept.txt")).unwrap(),
            "one\n"
        );
        assert!(repo.path.join("moved.txt").exists(), "the rename is undone");
        assert!(!repo.path.join("elsewhere.txt").exists());
        assert!(
            !repo.path.join("fresh.txt").exists(),
            "HEAD has no such file, so discarding it takes the file"
        );
    }

    /// A file changed on both sides has a row in each bucket, and the side
    /// rides with the chosen row: the unstaged row's discard stays the
    /// unstaged row's.
    #[tokio::test]
    #[ignore = "git's own restore --worktree: not worth the pre-merge run"]
    async fn discard_chosen_on_the_unstaged_row_keeps_what_is_staged() {
        let mut repo = TestRepo::init();
        repo.commit_file("kept.txt", "one\n", "root");
        repo.write_file("kept.txt", "staged\n");
        repo.git(&["add", "--", "kept.txt"]);
        repo.write_file("kept.txt", "and dirty\n");
        let (exec, cancel) = env();

        stage::discard_chosen(
            &exec,
            &repo.path,
            &[("kept.txt".into(), DiscardSide::Unstaged)],
            &cancel,
        )
        .await
        .expect("discard chosen");

        let (staged, unstaged, _) = buckets(&repo).await;
        assert_eq!(staged, vec!["kept.txt"], "the staged half survives");
        assert!(unstaged.is_empty());
        assert_eq!(
            std::fs::read_to_string(repo.path.join("kept.txt")).unwrap(),
            "staged\n",
            "the disk goes back to the index, not to HEAD"
        );
    }
}
