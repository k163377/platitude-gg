//! `land`: the rebase, the gate and the fast-forward in order, the build
//! slot it steps out of, and the seats it refuses.

#[cfg(target_os = "linux")]
use std::process::Command;

#[cfg(target_os = "linux")]
use crate::support::output_past_a_busy_image;
use crate::support::{EXE, Sandbox, without_always};

#[test]
fn land_rebases_then_gates_then_fast_forwards() {
    let sb = Sandbox::new("land");
    sb.write(&sb.repo, "internal-docs/notes.md", "# notes\n\nmoved\n");
    let moved = sb.commit_all(&sb.repo, "docs: main moved", &[("PG_GATE_SKIP", "1")]);
    sb.write_refs(&sb.seat, 8);
    let before = sb.commit_all(&sb.seat, "feat(core): eight", &[]);
    assert!(
        sb.git(
            &sb.repo,
            &["merge-base", "--is-ancestor", "main", "worktree-a"],
            &[]
        )
        .is_err()
    );

    let (ok, text) = sb.land("worktree-a");
    assert!(ok, "{text}");
    assert!(text.contains("rebasing"), "{text}");
    assert!(text.contains("landed worktree-a"), "{text}");
    let after = sb.head(&sb.seat);
    assert_ne!(after, before, "the rebase rewrote the commit");
    assert_eq!(sb.main_sha(), after, "main is the seat's tip");
    assert_eq!(
        sb.git_ok(&sb.seat, &["rev-parse", "HEAD~1"]),
        moved,
        "on top of what main had"
    );
    let ran = without_always(&sb.ran());
    assert!(ran.contains("test platitude-core 1"), "{ran:?}");
    let (ok, text) = sb.land("worktree-a");
    assert!(ok && text.contains("nothing to land"), "{text}");
}

/// A landing runs cargo in the trees it has just moved — the gate's
/// steps build the seat's task runner after the rebase — and `cargo
/// xtask land` is itself the binary in that slot. Windows cannot replace
/// a running image, so the slot is freed before the first step: the name
/// is empty afterwards, and what an earlier landing could not delete
/// (its own image, still running) is swept on the way past.
#[test]
fn land_steps_out_of_the_build_slot_the_gate_builds_into() {
    let sb = Sandbox::new("slot");
    // Main moves under the seat, so the rebase brings sources the task
    // runner in the slot no longer matches.
    sb.write(
        &sb.repo,
        "crates/xtask/src/qmltest.rs",
        "pub fn run() { let _ = 15; }\n",
    );
    sb.commit_all(
        &sb.repo,
        "feat(xtask): main moved",
        &[("PG_GATE_SKIP", "1")],
    );
    sb.write_refs(&sb.seat, 16);
    sb.commit_all(&sb.seat, "feat(core): sixteen", &[]);

    let slot = sb.seat.join("target").join("debug");
    std::fs::create_dir_all(&slot).expect("the build slot");
    let suffix = std::env::consts::EXE_SUFFIX;
    let running = slot.join(format!("xtask{suffix}"));
    std::fs::copy(EXE, &running).expect("the runner in the slot");
    let left_behind = slot.join(format!("xtask-inflight-424242{suffix}"));
    std::fs::write(&left_behind, b"an earlier landing's image").expect("what Windows leaves");

    let (ok, text) = sb.land_from(&running, "worktree-a");
    assert!(ok && text.contains("landed worktree-a"), "{text}");
    assert!(text.contains("stepped out of"), "{text}");
    assert!(
        !running.exists(),
        "the slot the gate's cargo has to write is still taken:\n{text}"
    );
    assert!(
        !left_behind.exists(),
        "an earlier landing's image was left in the slot's directory:\n{text}"
    );
    assert_eq!(sb.main_sha(), sb.head(&sb.seat));
}

/// The other half of that: a runner published into a slot by `fs::copy`
/// is exactly the image a neighbour's fork can be holding open, and what
/// this suite has to survive is the refusal, not the fork — a window too
/// short to catch on purpose. Held open here on purpose instead: one
/// attempt is refused outright, and the run that keeps asking gets its
/// answer as soon as the handle goes.
///
/// Linux rather than every unix, because POSIX only says `execve` *may*
/// refuse a file open for writing — this asserts that it does, which is
/// a promise Linux makes and the container is the machine that keeps it.
#[test]
#[cfg(target_os = "linux")]
fn a_runner_held_open_for_writing_is_run_once_the_handle_goes() {
    // A sandbox for the temp root it takes away again: nothing here gates.
    let sb = Sandbox::new("busy");
    let slot = sb.root.join("slot");
    std::fs::create_dir_all(&slot).expect("the build slot");
    let running = slot.join("xtask");
    std::fs::copy(EXE, &running).expect("the runner in the slot");

    // Opened, not truncated: the file stays the runner throughout.
    let handle = std::fs::OpenOptions::new()
        .write(true)
        .open(&running)
        .expect("hold the published runner open for writing");
    let refused = Command::new(&running)
        .output()
        .expect_err("a file open for writing is not executable on linux");
    assert_eq!(refused.kind(), std::io::ErrorKind::ExecutableFileBusy);

    let (busy, saw_busy) = std::sync::mpsc::channel();
    let runner = std::thread::spawn(move || {
        // No command: the runner prints its usage and touches nothing.
        let mut command = Command::new(&running);
        output_past_a_busy_image(&mut command, move || busy.send(()).expect("report ETXTBSY"))
    });
    saw_busy.recv().expect("the first execution was refused");
    drop(handle);
    let out = runner
        .join()
        .expect("the runner")
        .expect("run the published runner");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("cargo xtask <command>"),
        "the published file is not the runner: {out:?}"
    );
}

#[test]
fn land_refuses_a_dirty_seat_and_a_rebase_that_stops_is_walked_back() {
    let sb = Sandbox::new("refusals");
    sb.write_refs(&sb.seat, 12);
    let tip = sb.commit_all(&sb.seat, "feat(core): twelve", &[]);
    sb.write(
        &sb.seat,
        "crates/platitude-core/src/extra.rs",
        "// unsaved\n",
    );
    let (ok, text) = sb.land("worktree-a");
    assert!(!ok && text.contains("uncommitted"), "{text}");
    std::fs::remove_file(sb.seat.join("crates/platitude-core/src/extra.rs")).expect("clean");

    sb.write_refs(&sb.repo, 13);
    let main_before = sb.commit_all(&sb.repo, "feat(core): thirteen", &[("PG_GATE_SKIP", "1")]);
    let (ok, text) = sb.land("worktree-a");
    assert!(!ok && text.contains("walked back"), "{text}");
    assert_eq!(sb.head(&sb.seat), tip, "the seat stands where it did");
    assert!(
        sb.git(
            &sb.seat,
            &["rev-parse", "--verify", "--quiet", "REBASE_HEAD"],
            &[]
        )
        .is_err(),
        "no rebase left in flight"
    );
    assert_eq!(sb.main_sha(), main_before);
}
