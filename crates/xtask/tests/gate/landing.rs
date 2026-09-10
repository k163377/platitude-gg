//! `land`: the rebase, the gate and the fast-forward in order, the build
//! slot it steps out of, and the seats it refuses.

use std::process::Command;

use crate::support::{EXE, Sandbox, output_past_a_busy_image, without_always};

#[test]
fn land_keeps_the_readers_identity_when_it_leaves_the_build_slot() {
    let sb = Sandbox::new("land-graph-reader");
    sb.write_refs(&sb.seat, 8);
    sb.commit_all(&sb.seat, "feat(core): eight", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let slot = sb.seat.join("target/debug");
    std::fs::create_dir_all(&slot).expect("build slot");
    let running = slot.join(format!("xtask{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(EXE, &running).expect("the same reader in the build slot");
    let (ok, text) = sb.land_from(&running, "worktree-a");
    assert!(ok, "{text}");
    assert!(text.contains("stepped out of"), "{text}");
    assert!(text.contains("graph cache hit;"), "{text}");
    assert_eq!(sb.main_sha(), sb.head(&sb.seat));
}

#[test]
fn land_rebases_then_gates_then_fast_forwards() {
    let sb = Sandbox::new("land");
    sb.write(&sb.repo, "internal-docs/notes.md", "# notes\n\nmoved\n");
    let moved = sb.commit_all(&sb.repo, "docs: main moved", &[("PGG_GATE_SKIP", "1")]);
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
    assert!(text.contains("queue "), "{text}");
    let records = std::fs::read_dir(sb.seat.join("target/land-runs"))
        .expect("landing records")
        .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 1);
    assert!(
        records[0].contains("PASS") && records[0].contains("queue "),
        "{records:?}"
    );
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
        &[("PGG_GATE_SKIP", "1")],
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
    let failures = std::fs::read_dir(sb.seat.join("target/land-runs"))
        .expect("failed landing record")
        .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(failures.len(), 1);
    assert!(failures[0].contains("FAIL:") && failures[0].contains("uncommitted"));
    std::fs::remove_file(sb.seat.join("crates/platitude-core/src/extra.rs")).expect("clean");

    sb.write_refs(&sb.repo, 13);
    let main_before = sb.commit_all(&sb.repo, "feat(core): thirteen", &[("PGG_GATE_SKIP", "1")]);
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

/// The landing commits the census its gate rewrote and gates again,
/// rather than stopping to have a person commit a generated file.
#[test]
fn land_commits_the_census_its_gate_rewrote() {
    let sb = Sandbox::new("land-census");
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/StashPane.qml",
        "Item {\n    width: 4\n    property var model: StashModel\n}\n",
    );
    let before = sb.commit_all(&sb.seat, "feat(app-ui): pane", &[]);
    let mut command = Command::new(EXE);
    command
        .args(["land", "worktree-a", "--dir"])
        .arg(&sb.repo)
        .current_dir(&sb.repo)
        .env("PGG_GATE_FAKE_REWRITE", "verify stash --preset basic");
    sb.env(&mut command);
    let output = output_past_a_busy_image(&mut command, || {}).expect("spawn xtask");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{text}");
    assert!(text.contains("committed on worktree-a"), "{text}");
    assert!(text.contains("landed worktree-a"), "{text}");
    assert_eq!(
        sb.git_ok(&sb.seat, &["log", "-1", "--format=%s"]),
        "chore(xtask): the verb census as the land's gate rewrote it"
    );
    assert_eq!(sb.git_ok(&sb.seat, &["rev-parse", "HEAD~1"]), before);
    assert_eq!(
        sb.main_sha(),
        sb.head(&sb.seat),
        "main is the seat's tip, census and all"
    );
    let census = std::fs::read_to_string(sb.repo.join("crates/xtask/verb-census.txt"))
        .expect("the census on main");
    assert!(
        census.contains("stash --preset basic\tDriver Main StashPane Theme"),
        "{census}"
    );
}

/// A landed seat goes back to the roster: the branch is on main and the
/// tree is at main's tip, so the letter is free for whoever asks next
/// without anybody having to say the words (CLAUDE.md ビルド・テスト).
/// The session that landed takes it back at its next edit if it goes on
/// working there. A live claim naming somebody else stays, and one whose
/// process is gone goes back like the session's own.
#[test]
fn a_landed_seat_goes_back_to_the_roster() {
    let sb = Sandbox::new("claim");
    let seat = sb.seat.display().to_string().replace('\\', "/");
    let lock = |reason: &str| {
        sb.git_ok(&sb.repo, &["worktree", "lock", "--reason", reason, &seat]);
    };
    let locks = || sb.git_ok(&sb.repo, &["worktree", "list", "--porcelain"]);
    sb.write_refs(&sb.seat, 8);
    sb.commit_all(&sb.seat, "feat(core): eight", &[]);
    lock("claude-seat mine");

    let (ok, text) = sb.land_as("worktree-a", "mine");
    assert!(ok, "{text}");
    assert!(text.contains("released the seat claim"), "{text}");
    assert!(
        !locks().contains("locked"),
        "the seat the landing emptied stayed claimed"
    );

    lock("claude-seat mine");
    sb.write_refs(&sb.seat, 16);
    sb.commit_all(&sb.seat, "feat(core): sixteen", &[]);
    let (ok, text) = sb.land_as("worktree-a", "somebody-else");
    assert!(ok, "{text}");
    assert!(text.contains("is another session's and stays"), "{text}");
    assert!(
        locks().contains("locked claude-seat mine"),
        "a session that is still running is still using its tree"
    );

    // The number no process can have (`subprocess::NO_SUCH_PID`): odd,
    // and above what either kernel hands out.
    sb.git_ok(&sb.repo, &["worktree", "unlock", &seat]);
    lock("claude-seat gone pid 2147483645");
    sb.write_refs(&sb.seat, 32);
    sb.commit_all(&sb.seat, "feat(core): thirty-two", &[]);
    let (ok, text) = sb.land_as("worktree-a", "mine");
    assert!(ok, "{text}");
    assert!(text.contains("released the seat claim"), "{text}");
    assert!(
        !locks().contains("locked"),
        "the seat of a session that ended goes back to the roster"
    );
}
