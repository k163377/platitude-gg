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

/// `--gui` decides which config key git reads, so the two keys name
/// different tools and whichever runs writes its own mark into the file.
///
/// The listing is the other half: `mergetool.writeToTemp` defaults to
/// false, which puts `_LOCAL_` / `_REMOTE_` / `_BASE_` / `_BACKUP_` beside
/// the conflicted file for as long as the tool is open — every one of them
/// untracked, and every one of them in the pane.
#[tokio::test]
async fn mergetool_launches_the_gui_tool_and_keeps_its_scratch_out_of_the_tree() {
    let mut repo = stopped_merge().await;
    for (name, mark) in [("pgguitool", "gui"), ("pgclitool", "cli")] {
        let key = format!("mergetool.{name}.cmd");
        let cmd = format!("ls > listing.txt; printf {mark} > \"$MERGED\"");
        repo.git(&["config", &key, &cmd]);
        // git only trusts a user-defined tool's exit code when told to.
        // Left off, it compares mtimes instead — pinned separately below.
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

    // Nothing ran: git would otherwise have guessed a tool and then asked
    // on a stdin that is closed, reporting the file as failed.
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
    // Neither of these names a tool: one is another setting on a tool that
    // has no `cmd`, the other is git's own choice of which to launch.
    repo.git(&["config", "mergetool.alpha.trustExitCode", "true"]);
    repo.git(&["config", "merge.guitool", "alpha"]);

    let mut names = conflict::user_defined_tools(&exec, &repo.path, &cancel)
        .await
        .expect("read the keys");
    names.sort();
    assert_eq!(names, vec!["alpha".to_string(), "beta".to_string()]);
}

/// What the settings card writes lands on `merge.guitool`, and an empty
/// field takes the key back out.
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

/// Clearing a tool nothing had set is the outcome that was asked for:
/// `git config --unset` says "there was nothing to unset" with exit 5.
/// Unmarked, that code is a failed row and the command panel opens
/// itself over it — which is what closing the settings card did on a
/// machine with no merge tool configured.
#[tokio::test]
async fn clearing_a_tool_that_was_never_set_answers_by_code() {
    let repo = TestRepo::init();
    let (exec, log, cancel) = logged_global(repo.global_config());

    conflict::set_merge_tool(&exec, &repo.path, "", &cancel)
        .await
        .expect("nothing to unset is not a failure");

    assert_eq!(log.ends_of(&["--unset"]), vec![CommandEnd::Answered(5)]);
}

/// **What the pre-merge run leaves out**: this machine's own answer, and git's
/// own check on a tool that saved nothing — the launch it follows is the
/// same command line the pre-merge launch test runs. Run by the full gate
/// (`-- --ignored ::periodic::`) rather than by every change.
mod periodic {
    use super::*;

    /// Closing the editor without saving comes back as a failed file, and
    /// the markers are put back.
    ///
    /// git touches a backup before running an untrusted tool and compares
    /// mtimes afterwards; a file that did not move makes it ask on stdin
    /// whether the merge worked. stdin is closed here, so the question is
    /// answered by failing. The wording reaches the person, so it is pinned.
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

    /// `--tool-help` sources every tool definition twice and probes the
    /// registry for each. The parsing it feeds is covered by unit tests
    /// against captured output; this only checks that the command still
    /// answers in the shape they assume.
    #[tokio::test]
    #[ignore = "this machine's merge tools: seconds on Windows, not worth the pre-merge run"]
    #[mry::lock(conflict::available_tools)]
    async fn available_tools_never_offers_one_that_needs_a_terminal() {
        // Held and told to call through, so no pre-merge test's machine can
        // stand in for this one.
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
