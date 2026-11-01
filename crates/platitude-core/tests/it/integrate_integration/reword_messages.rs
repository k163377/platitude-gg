//! Where a reword's message waits for its `exec` line and when it goes: in
//! the rebase's own state folder (`rebase-merge`), which git takes away when
//! the rebase ends — run to the end, aborted or quit, by this application or
//! any other git — so the application removes none, and nothing another run
//! holds is touched.

use std::path::{Path, PathBuf};

use crate::support::TestRepo;
use crate::support::exec::env;
use crate::support::info;
use crate::support::integrate::helper;
use platitude_core::integrate::{self, Continuation, RebaseOptions, RebaseOutcome};
use platitude_core::repo::RepoInfo;
use platitude_core::sequencer::{self, RebaseStep, TodoAction};

/// The message files waiting in a worktree's rebase state, by name.
fn waiting(git_dir: &Path) -> Vec<String> {
    names_in(&git_dir.join("rebase-merge"))
        .into_iter()
        .filter(|name| name.starts_with("platitude-reword-"))
        .collect()
}

/// What stands under a worktree's scratch folder.
fn scratch(git_dir: &Path) -> Vec<String> {
    names_in(&git_dir.join("platitude"))
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// A reword behind a pick that conflicts with `side`: the replay stops
/// before the reword's `exec` line has run.
struct BehindAConflict {
    repo: TestRepo,
    info: RepoInfo,
}

impl BehindAConflict {
    async fn stop() -> Self {
        let mut repo = TestRepo::init();
        repo.commit_file("f.txt", "base\n", "base");
        repo.git(&["switch", "-c", "side"]);
        repo.commit_file("f.txt", "side\n", "their line");
        repo.git(&["switch", "main"]);
        let conflicting = repo.commit_file_id("f.txt", "ours\n", "our line");
        let reworded = repo.commit_file_id("g.txt", "g\n", "old words");
        let info = info(&repo).await;
        let (exec, cancel) = env();
        let steps = [
            RebaseStep::pick(conflicting, "our line"),
            RebaseStep {
                action: TodoAction::Reword,
                oid: reworded,
                subject: "old words".into(),
                message: Some("new words\r\n\r\n".into()),
            },
        ];
        let outcome = sequencer::rebase_interactive(
            &exec,
            &info,
            "side",
            &steps,
            &RebaseOptions::default(),
            &helper(),
            &cancel,
        )
        .await
        .expect("run the plan");
        assert!(
            matches!(outcome, RebaseOutcome::Stopped),
            "the pick conflicts: {outcome:?}"
        );
        Self { repo, info }
    }

    fn git_dir(&self) -> &Path {
        &self.info.git_dir
    }

    /// The conflict settled and staged, ready to carry on.
    fn settle(&mut self) {
        self.repo.write_file("f.txt", "settled\n");
        self.repo.git(&["add", "--", "f.txt"]);
    }
}

/// Stopped, the message waits where the rebase keeps its state, as the
/// editor's text came normalized; the scratch folder holds nothing, the
/// plan included.
#[tokio::test]
async fn a_stopped_reword_keeps_its_message_where_the_rebase_keeps_its_state() {
    let stopped = BehindAConflict::stop().await;
    let git_dir = stopped.git_dir();
    assert_eq!(waiting(git_dir), ["platitude-reword-1"]);
    assert_eq!(
        std::fs::read_to_string(git_dir.join("rebase-merge").join("platitude-reword-1"))
            .expect("the message"),
        "new words\n"
    );
    assert_eq!(scratch(git_dir), Vec::<String>::new());
}

/// Nothing of the run that stopped is needed to carry it on: a run with
/// none of its handles — the application started again — continues it, the
/// message lands, and the rebase's end takes the message with it.
#[tokio::test]
async fn a_later_run_carries_the_stop_on_and_its_end_takes_the_message() {
    let mut stopped = BehindAConflict::stop().await;
    stopped.settle();
    let (exec, cancel) = env();
    let reopened = info(&stopped.repo).await;
    integrate::resolve_current(&exec, &reopened.workdir, Continuation::Continue, &cancel)
        .await
        .expect("carry on");

    assert_eq!(stopped.repo.git(&["log", "-1", "--format=%B"]), "new words");
    assert!(!reopened.git_dir.join("rebase-merge").exists(), "done");
    assert_eq!(scratch(&reopened.git_dir), Vec::<String>::new());
}

/// An abort or a quit from another git ends the rebase, and git takes the
/// message with its state: no later rebase here has to come.
#[tokio::test]
async fn another_gits_abort_or_quit_takes_the_message_with_the_rebase() {
    for ending in ["--abort", "--quit"] {
        let mut stopped = BehindAConflict::stop().await;
        stopped.repo.git(&["rebase", ending]);
        let git_dir = stopped.git_dir().to_path_buf();
        assert!(!git_dir.join("rebase-merge").exists(), "{ending}: ended");
        assert_eq!(waiting(&git_dir), Vec::<String>::new(), "{ending}");
        assert_eq!(scratch(&git_dir), Vec::<String>::new(), "{ending}");
    }
}

/// The same from here: the application's own abort.
#[tokio::test]
async fn this_applications_abort_takes_the_message_with_the_rebase() {
    let stopped = BehindAConflict::stop().await;
    let (exec, cancel) = env();
    integrate::resolve_current(&exec, &stopped.info.workdir, Continuation::Abort, &cancel)
        .await
        .expect("abort");
    assert!(!stopped.git_dir().join("rebase-merge").exists());
    assert_eq!(scratch(stopped.git_dir()), Vec::<String>::new());
}

/// A plan that runs to the end, and one git turns down at its first
/// checkout after the helper has put the message down, both leave nothing.
/// Reordered, so the replay has to check out where it starts (picks
/// already in place are passed over without one).
#[tokio::test]
async fn a_reword_run_to_the_end_or_turned_down_leaves_nothing() {
    let mut repo = TestRepo::init();
    repo.commit_file("old.txt", "kept once\n", "root");
    let root = repo.git(&["rev-parse", "HEAD"]);
    repo.git(&["rm", "-q", "old.txt"]);
    let dropping = repo.commit_file_id("a.txt", "a\n", "drops old");
    let last = repo.commit_file_id("b.txt", "b\n", "last");
    let info = info(&repo).await;
    let (exec, cancel) = env();
    let steps = [
        RebaseStep::pick(last, "last"),
        RebaseStep {
            action: TodoAction::Reword,
            oid: dropping,
            subject: "drops old".into(),
            message: Some("drops the old file".into()),
        },
    ];
    let (options, helper) = (RebaseOptions::default(), helper());
    let run =
        || sequencer::rebase_interactive(&exec, &info, &root, &steps, &options, &helper, &cancel);

    repo.write_file("old.txt", "untracked now\n");
    run()
        .await
        .expect_err("the first checkout would overwrite old.txt");
    assert!(!info.git_dir.join("rebase-merge").exists(), "turned down");
    assert_eq!(scratch(&info.git_dir), Vec::<String>::new(), "turned down");

    std::fs::remove_file(repo.path.join("old.txt")).expect("out of the way");
    let outcome = run().await.expect("the plan runs");
    assert!(matches!(outcome, RebaseOutcome::Done), "{outcome:?}");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]),
        "drops the old file"
    );
    assert!(!info.git_dir.join("rebase-merge").exists(), "done");
    assert_eq!(scratch(&info.git_dir), Vec::<String>::new(), "done");
}

/// A rebase that runs to the end here touches nothing another run holds:
/// files another process keeps in the scratch folder, one named as a reword
/// message and one as a patch, and the stopped reword of another worktree,
/// which still carries on to its message.
#[tokio::test]
async fn a_rebase_run_to_the_end_leaves_what_other_runs_hold() {
    let mut here = BehindAConflict::stop().await;
    here.repo.git(&["rebase", "--abort"]);
    let there: PathBuf = here.repo.path.with_file_name("linked");
    here.repo.git(&[
        "worktree",
        "add",
        "-q",
        "--detach",
        &there.to_string_lossy(),
    ]);
    let (exec, cancel) = env();
    let there_info = platitude_core::repo::open(&exec, &there, &cancel)
        .await
        .expect("open the linked worktree");
    let head = here.repo.git_in(&there, &["rev-parse", "HEAD"]);
    let below = here.repo.git_in(&there, &["rev-parse", "HEAD~1"]);
    let stopped = sequencer::rebase_interactive(
        &exec,
        &there_info,
        "side",
        &[
            RebaseStep::pick(below, "our line"),
            RebaseStep {
                action: TodoAction::Reword,
                oid: head,
                subject: "old words".into(),
                message: Some("words from there".into()),
            },
        ],
        &RebaseOptions::default(),
        &helper(),
        &cancel,
    )
    .await
    .expect("run the plan there");
    assert!(matches!(stopped, RebaseOutcome::Stopped), "{stopped:?}");

    let held = here.git_dir().join("platitude");
    std::fs::create_dir_all(&held).expect("the scratch folder");
    let others = ["REWORD_MSG-4194303-0", "stage.patch-4194303-1"];
    for name in others {
        std::fs::write(held.join(name), b"another run's\n").expect("another run's file");
    }
    let tip = here.repo.git(&["rev-parse", "HEAD"]);
    let outcome = sequencer::rebase_interactive(
        &exec,
        &here.info,
        "HEAD~1",
        &[RebaseStep {
            action: TodoAction::Reword,
            oid: tip,
            subject: "old words".into(),
            message: Some("words from here".into()),
        }],
        &RebaseOptions::default(),
        &helper(),
        &cancel,
    )
    .await
    .expect("run the plan here");
    assert!(matches!(outcome, RebaseOutcome::Done), "{outcome:?}");
    assert_eq!(
        here.repo.git(&["log", "-1", "--format=%s"]),
        "words from here"
    );
    assert_eq!(scratch(here.git_dir()), others, "another run's files stay");

    assert_eq!(waiting(&there_info.git_dir), ["platitude-reword-1"]);
    std::fs::write(there.join("f.txt"), "settled\n").expect("settle there");
    here.repo.git_in(&there, &["add", "--", "f.txt"]);
    here.repo.git_in(&there, &["rebase", "--continue"]);
    assert_eq!(
        here.repo.git_in(&there, &["log", "-1", "--format=%s"]),
        "words from there"
    );
}
