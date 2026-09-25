//! Handing a stopped merge to the configured mergetool.

use crate::support::TestRepo;
use crate::support::exec::{env, logged_global};
use crate::support::integrate::conflicting_branches;
use platitude_core::integrate::{self, MergeOptions};
use platitude_core::process::CommandEnd;
use platitude_core::{conflict, status};

/// `side` merged into `main`, stopped on the conflict in `f.txt`.
async fn stopped_merge() -> TestRepo {
    let repo = conflicting_branches();
    let (exec, cancel) = env();
    integrate::merge(&exec, &repo.path, "side", &MergeOptions::default(), &cancel)
        .await
        .expect("conflict");
    repo
}

/// The two keys name different tools, so the mark in the file says which
/// key `--gui` reached. `mergetool.writeToTemp` defaults to false, which
/// puts `_LOCAL_` / `_REMOTE_` / `_BASE_` / `_BACKUP_` beside the file,
/// untracked and in the pane, while the tool is open.
#[tokio::test]
async fn mergetool_launches_the_gui_tool_and_keeps_its_scratch_out_of_the_tree() {
    let mut repo = stopped_merge().await;
    for (name, mark) in [("pgguitool", "gui"), ("pgclitool", "cli")] {
        let key = format!("mergetool.{name}.cmd");
        let cmd = format!("ls > listing.txt; printf {mark} > \"$MERGED\"");
        repo.git(&["config", &key, &cmd]);
        // Left off, git compares mtimes instead of the exit code (`periodic`).
        let trust = format!("mergetool.{name}.trustExitCode");
        repo.git(&["config", &trust, "true"]);
    }
    repo.git(&["config", "merge.guitool", "pgguitool"]);
    repo.git(&["config", "merge.tool", "pgclitool"]);

    let (exec, cancel) = env();
    conflict::mergetool(&exec, &repo.path, &["f.txt".into()], &cancel)
        .await
        .expect("run the tool");

    assert_eq!(
        std::fs::read_to_string(repo.path.join("f.txt")).unwrap(),
        "gui",
        "merge.guitool is the key --gui reaches"
    );
    let listing = std::fs::read_to_string(repo.path.join("listing.txt")).unwrap();
    assert!(
        !listing.contains("_LOCAL_") && !listing.contains("_BACKUP_"),
        "scratch landed in the working tree: {listing}"
    );

    // git stages what the tool resolved, so the caller only has to refresh.
    let s = status::load(&exec, &repo.path, &cancel)
        .await
        .expect("status");
    assert!(conflict::conflicted(&s).is_empty());
}

#[tokio::test]
async fn mergetool_refuses_when_no_tool_is_configured() {
    let repo = stopped_merge().await;
    let (exec, cancel) = env();

    let err = conflict::mergetool(&exec, &repo.path, &["f.txt".into()], &cancel)
        .await
        .expect_err("nothing to launch");
    assert!(err.to_string().contains("merge.guitool"), "{err}");

    // Nothing ran: git would have guessed a tool, asked on the closed
    // stdin, and failed the file.
    let f = std::fs::read_to_string(repo.path.join("f.txt")).unwrap();
    assert!(f.contains("<<<<<<<"), "{f}");
    assert!(!repo.path.join("listing.txt").exists());
}

#[tokio::test]
async fn user_defined_tools_come_from_their_keys() {
    let mut repo = TestRepo::init();
    let (exec, cancel) = env();
    assert!(
        conflict::user_defined_tools(&exec, &repo.path, &cancel)
            .await
            .expect("none configured")
            .is_empty(),
        "no keys is an empty answer, not a failure"
    );

    repo.git(&["config", "mergetool.alpha.cmd", "true"]);
    repo.git(&["config", "mergetool.beta.cmd", "true"]);
    // Neither names a tool: one is a non-`cmd` key on `alpha`, the other is
    // git's own choice of which to launch.
    repo.git(&["config", "mergetool.alpha.trustExitCode", "true"]);
    repo.git(&["config", "merge.guitool", "alpha"]);

    let mut names = conflict::user_defined_tools(&exec, &repo.path, &cancel)
        .await
        .expect("read the keys");
    names.sort();
    assert_eq!(names, vec!["alpha".to_string(), "beta".to_string()]);
}

#[tokio::test]
async fn setting_the_tool_writes_the_gui_key_and_an_empty_one_clears_it() {
    let repo = TestRepo::init();
    let (exec, log, cancel) = logged_global(repo.global_config());

    conflict::set_merge_tool(&exec, &repo.path, "pgtool", &cancel)
        .await
        .expect("write the choice");
    assert_eq!(
        conflict::configured_tool(&exec, &repo.path, &cancel)
            .await
            .expect("read it back"),
        Some("pgtool".to_string()),
    );

    conflict::set_merge_tool(&exec, &repo.path, "", &cancel)
        .await
        .expect("clear the choice");
    assert_eq!(
        conflict::configured_tool(&exec, &repo.path, &cancel)
            .await
            .expect("read it back"),
        None,
    );
    assert_eq!(
        log.ends_of(&["--unset"]),
        vec![CommandEnd::Answered(0)],
        "the key was there to be taken out"
    );
}

/// `git config --unset` exits 5 when there was nothing to unset; unmarked,
/// that is a failed row and the command panel opens over the settings card.
#[tokio::test]
async fn clearing_a_tool_that_was_never_set_answers_by_code() {
    let repo = TestRepo::init();
    let (exec, log, cancel) = logged_global(repo.global_config());

    conflict::set_merge_tool(&exec, &repo.path, "", &cancel)
        .await
        .expect("nothing to unset is not a failure");

    assert_eq!(log.ends_of(&["--unset"]), vec![CommandEnd::Answered(5)]);
}

/// This machine's tools, and git's own check on a tool that saved nothing
/// after the launch the test above runs (rules-refs/core.md `periodic`).
mod periodic {
    use super::*;

    /// For an untrusted tool git compares mtimes; an unmoved file makes it
    /// ask on stdin, closed here, so the file fails and keeps its markers.
    #[tokio::test]
    #[ignore = "git's own mtime check on an untrusted tool: not worth the pre-merge run"]
    async fn a_tool_that_saves_nothing_fails_and_leaves_the_markers() {
        let mut repo = stopped_merge().await;
        repo.git(&["config", "mergetool.pgnoop.cmd", "true"]);
        repo.git(&["config", "merge.tool", "pgnoop"]);

        let (exec, cancel) = env();
        conflict::mergetool(&exec, &repo.path, &["f.txt".into()], &cancel)
            .await
            .expect_err("nothing was saved");

        let f = std::fs::read_to_string(repo.path.join("f.txt")).unwrap();
        assert!(f.contains("<<<<<<<"), "the conflict is still there: {f}");
    }

    /// The parsing is unit-tested on captured output; this checks that the
    /// real `--tool-help` still answers in that shape.
    #[tokio::test]
    #[ignore = "this machine's merge tools: seconds on Windows, not worth the pre-merge run"]
    #[mry::lock(conflict::available_tools)]
    async fn available_tools_never_offers_one_that_needs_a_terminal() {
        conflict::mock_available_tools().calls_real_impl();
        let repo = TestRepo::init();
        let (exec, cancel) = env();
        let names = conflict::available_tools(&exec, &repo.path, &cancel)
            .await
            .expect("ask git what is installed");
        // Git for Windows ships vim, so vimdiff is always "available" — and
        // always unusable here, since the subprocess gets no console.
        assert!(
            !names.iter().any(|n| n.starts_with("vimdiff")),
            "a terminal tool got offered: {names:?}"
        );
    }
}
