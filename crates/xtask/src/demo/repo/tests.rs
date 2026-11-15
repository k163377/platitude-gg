//! What a demo repository's build leaves behind, read back from git: the
//! stamps its commits carry, and nothing of git's still running in it.

use super::DemoRepo;

/// Every git a build starts runs its automatic maintenance before it
/// answers, the receiving end of a push included. Asked the two ways git
/// decides it: the setting the build's git reads, in the worktree and in
/// the bare repository a push lands in — the oldest git starts `gc --auto`,
/// which reads it and says nothing in its arguments — and the children git
/// starts (`GIT_TRACE`), where newer git spells it out as `--detach` or
/// `--no-detach` on `maintenance run --auto`.
#[test]
fn every_git_a_build_starts_runs_its_maintenance_before_it_answers() {
    let root = crate::verify::claim_dir(&crate::temp::root().join("pgg-demo"), "maintenance")
        .expect("a directory of its own");
    let mut repo = DemoRepo::init(&root, "repo").expect("a repository");
    repo.add_origin().expect("an origin");
    let work = repo.work.clone();
    for dir in [work.clone(), root.join("origin.git")] {
        let detach = repo.git_at(&dir, &["config", "--type=bool", "--get", "gc.autoDetach"]);
        assert_eq!(
            detach.as_deref(),
            Ok("false"),
            "gc.autoDetach in {}",
            dir.display()
        );
    }
    repo.write("a.txt", "a\n").expect("a file");
    repo.git(&["add", "--", "a.txt"]).expect("staged");
    // One command that starts maintenance here, and one whose receiving end
    // starts it in the bare repository.
    let mut started = Vec::new();
    for (name, args) in [
        ("commit", &["commit", "-m", "m"][..]),
        ("push", &["push", "origin", "main"][..]),
    ] {
        let trace = root.join(format!("trace-{name}"));
        let out =
            super::output_of(repo.command(&work, args).env("GIT_TRACE", &trace)).expect("git runs");
        assert!(
            out.status.success(),
            "git {name}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let traced = std::fs::read_to_string(&trace).expect("the trace");
        let maintenance: Vec<String> = traced
            .lines()
            .filter_map(|line| line.split_once("run_command: ").map(|(_, run)| run))
            .filter(|run| run.contains("maintenance run --auto") || run.contains("gc --auto"))
            .map(str::to_string)
            .collect();
        assert!(
            !maintenance.is_empty(),
            "no maintenance started by git {name} in\n{traced}"
        );
        started.extend(maintenance);
    }
    let detached: Vec<&String> = started
        .iter()
        .filter(|run| run.split_whitespace().any(|word| word == "--detach"))
        .collect();
    assert!(
        detached.is_empty(),
        "left running past its command: {detached:?}"
    );
    std::fs::remove_dir_all(&root).expect("the directory, cleared");
}

#[test]
fn a_commit_written_earlier_is_authored_exactly_that_long_before_it_is_committed() {
    let root = crate::verify::claim_dir(&crate::temp::root().join("pgg-demo"), "earlier")
        .expect("a directory of its own");
    let earlier = 2 * 24 * 3600;
    let mut repo = DemoRepo::init(&root, "repo").expect("a repository");
    repo.commit_written_earlier("a.txt", "x\n", "m", None, earlier)
        .expect("a commit");
    let stamps = repo
        .git(&["log", "-1", "--format=%at %ct"])
        .expect("the commit's stamps");
    std::fs::remove_dir_all(&root).expect("the directory, cleared");
    let [authored, committed] = [0, 1].map(|at| {
        stamps
            .split(' ')
            .nth(at)
            .and_then(|stamp| stamp.parse::<u64>().ok())
            .unwrap_or_else(|| panic!("two stamps, not {stamps:?}"))
    });
    assert_eq!(committed - authored, earlier, "{stamps}");
}
