//! What a run leaves behind: the stamps a green step and a green commit
//! earn, when they are reused and when they are not, and the one gate a
//! tree holds while it runs.

use crate::support::{ALWAYS, Sandbox, set, without_always};

#[test]
fn a_step_is_asked_again_only_when_what_it_reads_changed() {
    let sb = Sandbox::new("cache");
    sb.write(&sb.seat, "crates/platitude-core/src/stash.rs", "pub fn stash() { let _ = 3; }\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn t() { stash() }\n}\n");
    sb.commit_all(&sb.seat, "feat(core): three", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let first = without_always(&sb.ran());
    assert!(
        first.contains("test platitude-core 1") && first.contains("test platitude-app 1"),
        "{first:?}"
    );

    let text = sb.gate_ok(&sb.seat, &[]);
    assert!(text.contains("cached test platitude-core 1"), "{text}");
    assert_eq!(
        sb.ran(),
        set(&ALWAYS),
        "a second run of one commit runs nothing twice"
    );

    // A refs commit on top: the branch's diff now holds both modules, so
    // the core step is a new one (two filters) and runs; the app's unit
    // tests read stash and its readers, none of which moved, so that
    // step stays green. The verbs and the shipped build read the whole
    // app and core — the core moved, so they run again.
    sb.write_refs(&sb.seat, 4);
    sb.commit_all(&sb.seat, "feat(core): four", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let second = without_always(&sb.ran());
    assert!(second.contains("test platitude-core 2"), "{second:?}");
    assert!(
        !second.contains("test platitude-app 1"),
        "the app reads stash, not refs: {second:?}"
    );
    assert!(
        second.contains("shipped") && second.contains("verify stash --preset basic"),
        "{second:?}"
    );
}

#[test]
fn a_host_only_run_stamps_half_and_the_full_run_reuses_it() {
    let sb = Sandbox::new("host-only");
    sb.write_refs(&sb.seat, 5);
    sb.commit_all(&sb.seat, "feat(core): five", &[]);
    let text = sb.gate_ok(&sb.seat, &["--host-only"]);
    assert!(text.contains("host-only"), "{text}");
    let daily = sb.ran();
    assert!(
        daily.contains("test platitude-core 1") && !daily.contains("test platitude-core 1 linux"),
        "{daily:?}"
    );
    let refused = sb.git(&sb.repo, &["merge", "--ff-only", "worktree-a"], &[]);
    assert!(
        refused.as_ref().is_err_and(|e| e.contains("host-only")),
        "{refused:?}"
    );
    sb.gate_ok(&sb.seat, &[]);
    let rest = without_always(&sb.ran());
    assert!(
        rest.contains("test platitude-core 1 linux") && !rest.contains("test platitude-core 1"),
        "{rest:?}"
    );
    sb.git_ok(&sb.repo, &["merge", "--ff-only", "worktree-a"]);
}

#[test]
fn a_red_step_leaves_main_where_it_was() {
    let sb = Sandbox::new("red");
    sb.write_refs(&sb.seat, 7);
    sb.commit_all(&sb.seat, "feat(core): seven", &[]);
    let (ok, text) = sb.gate(
        &sb.seat,
        &[],
        &[("PG_GATE_FAKE_FAIL", "test platitude-core 1")],
    );
    assert!(
        !ok && text.contains("FAIL   test platitude-core 1") && text.contains("nothing stamped"),
        "{text}"
    );
    let merge = sb.git(&sb.repo, &["merge", "--ff-only", "worktree-a"], &[]);
    assert!(merge.is_err(), "a red gate stamped nothing: {merge:?}");
    let _ = sb.ran();
    sb.gate_ok(&sb.seat, &[]);
    let again = without_always(&sb.ran());
    assert!(again.contains("test platitude-core 1"), "{again:?}");
    assert!(
        !again.contains("test platitude-core 1 linux"),
        "the linux side was green and stays so: {again:?}"
    );
}

#[test]
fn a_pseudo_run_off_main_is_reused_after_the_rebase() {
    let sb = Sandbox::new("pseudo");
    sb.write(&sb.repo, "internal-docs/notes.md", "# notes\n\nmoved\n");
    sb.commit_all(&sb.repo, "docs: main moved", &[("PG_GATE_SKIP", "1")]);
    sb.write_refs(&sb.seat, 9);
    let tip = sb.commit_all(&sb.seat, "feat(core): nine", &[]);
    let text = sb.gate_ok(&sb.seat, &[]);
    assert!(text.contains("off main"), "{text}");
    let pseudo = without_always(&sb.ran());
    assert!(pseudo.contains("test platitude-core 1"), "{pseudo:?}");
    let update = sb.git(&sb.seat, &["update-ref", "refs/heads/main", &tip], &[]);
    assert!(
        update.as_ref().is_err_and(|e| e.contains("off main")),
        "{update:?}"
    );

    let (ok, text) = sb.land("worktree-a");
    assert!(
        ok && text.contains("rebasing") && text.contains("landed"),
        "{text}"
    );
    assert_eq!(sb.ran(), set(&ALWAYS), "the rebase owed nothing new");
    assert_eq!(sb.main_sha(), sb.head(&sb.seat));
}

#[test]
fn a_rebase_that_touches_a_step_s_inputs_reruns_that_step_only() {
    let sb = Sandbox::new("rebase-inputs");
    // Main's move touches the stash module.
    sb.write(&sb.repo, "crates/platitude-core/src/stash.rs", "pub fn stash() { let _ = 10; }\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn t() { stash() }\n}\n");
    sb.commit_all(&sb.repo, "feat(core): main moved", &[("PG_GATE_SKIP", "1")]);
    sb.write_refs(&sb.seat, 11);
    sb.commit_all(&sb.seat, "feat(core): eleven", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let _ = sb.ran();
    let (ok, text) = sb.land("worktree-a");
    assert!(ok, "{text}");
    let again = without_always(&sb.ran());
    // The refs tests read refs.rs only: same objects, still green. The it
    // binary's inputs include support and both modules' readers — its
    // refs filter reads refs_integration, which reads support: unchanged
    // too. clippy reads the whole crate: main's move changed that.
    assert!(
        !again.contains("test platitude-core 1"),
        "refs' inputs did not move: {again:?}"
    );
    assert!(
        again.contains("clippy platitude-core"),
        "the crate as a whole moved: {again:?}"
    );
}

/// One gate per tree: while one holds the tree a second is refused with
/// the first's pid and runs nothing, a dry run is not held (it runs
/// nothing either), and the tree is free again the moment the first is
/// done — or gone.
#[test]
fn a_second_gate_in_the_same_tree_is_refused_while_the_first_holds_it() {
    let sb = Sandbox::new("one-gate");
    sb.write_refs(&sb.seat, 17);
    sb.commit_all(&sb.seat, "feat(core): seventeen", &[]);
    let target = sb.seat.join("target");
    std::fs::create_dir_all(&target).expect("the seat's target");
    std::fs::write(
        target.join("gate-running"),
        "pid 424242\nsince 0\nwhat gate --all\n",
    )
    .expect("the first gate's note");
    let held = std::fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(target.join("gate-running.lock"))
        .expect("the first gate's lock");
    held.try_lock()
        .expect("held from here, as the first gate holds it");

    let (ok, text) = sb.gate(&sb.seat, &[], &[]);
    assert!(!ok, "a second gate ran beside the first:\n{text}");
    assert!(
        text.contains("already running") && text.contains("gate --all (pid 424242"),
        "{text}"
    );
    assert!(sb.ran().is_empty(), "the refused gate ran a step");
    let (ok, text) = sb.gate(&sb.seat, &["--dry-run"], &[]);
    assert!(
        ok,
        "a dry run holds nothing and is held by nothing:\n{text}"
    );

    drop(held);
    sb.gate_ok(&sb.seat, &[]);
    assert!(
        !target.join("gate-running").exists(),
        "the note comes down with the gate that wrote it"
    );
}
