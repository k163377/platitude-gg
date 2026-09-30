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
    assert!(
        sb.git_ok(&sb.repo, &["tag", "-l"]).is_empty(),
        "a landing that moves no version is tagged: {text}"
    );
    let (ok, text) = sb.land("worktree-a");
    assert!(ok && text.contains("nothing to land"), "{text}");
}

/// A lock whose app is built with `qtbridge` at `version`.
fn lock_with_qtbridge(version: &str) -> String {
    format!(
        "version = 4\n\n[[package]]\nname = \"platitude-app\"\nversion = \"0.0.0\"\n\
         dependencies = [\n \"qtbridge\",\n]\n\n[[package]]\nname = \"qtbridge\"\n\
         version = \"{version}\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n"
    )
}

/// A landing that moves a version the shipped build is made from lands
/// between two tags, pushed where main goes: the last main on the old
/// versions, and the main the update and its follow-up arrived at. A
/// second update the same day counts on.
#[test]
fn a_version_move_lands_between_two_pushed_tags() {
    let sb = Sandbox::new("land-tags");
    sb.git_ok(&sb.root, &["init", "-q", "--bare", "origin.git"]);
    let origin = sb.root.join("origin.git");
    let url = origin.display().to_string().replace('\\', "/");
    sb.git_ok(&sb.repo, &["remote", "add", "origin", &url]);
    sb.git_ok(&sb.repo, &["config", "branch.main.remote", "origin"]);
    let tags_of = |dir: &std::path::Path| {
        sb.git_ok(dir, &["tag", "-l"])
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let at = |tag: &str| sb.git_ok(&sb.repo, &["rev-parse", &format!("{tag}^{{commit}}")]);
    let said = |tag: &str| sb.git_ok(&sb.repo, &["tag", "-l", "--format=%(contents)", tag]);
    let day = || {
        sb.git_ok(
            &sb.repo,
            &["log", "-1", "--format=%cd", "--date=short", "main"],
        )
    };

    let seed = sb.main_sha();
    sb.write(&sb.seat, "Cargo.lock", &lock_with_qtbridge("0.2.1"));
    sb.commit_all(&sb.seat, "chore: build with qtbridge", &[]);
    let (ok, text) = sb.land("worktree-a");
    assert!(ok, "{text}");
    let first = day();
    let [before, after] = [
        format!("deps/{first}/before"),
        format!("deps/{first}/after"),
    ];
    assert!(
        text.contains(&format!("pushed {before} and {after} to origin")),
        "{text}"
    );
    assert_eq!(at(&before), seed);
    assert_eq!(at(&after), sb.main_sha());
    assert!(
        said(&before).contains("\n+ qtbridge 0.2.1"),
        "{}",
        said(&before)
    );
    assert!(
        said(&after).starts_with(&format!(
            "Update done: 1 commit(s) of the update and its follow-up since {before}"
        )),
        "{}",
        said(&after)
    );
    assert_eq!(tags_of(&origin), tags_of(&sb.repo));

    let old_main = sb.main_sha();
    sb.write(&sb.seat, "Cargo.lock", &lock_with_qtbridge("0.3.0"));
    sb.commit_all(&sb.seat, "chore: qtbridge 0.3", &[]);
    sb.write_refs(&sb.seat, 8);
    sb.commit_all(&sb.seat, "perf(core): after the update", &[]);
    let (ok, text) = sb.land("worktree-a");
    assert!(ok, "{text}");
    // Two landings a midnight apart are two days, and neither counts on.
    let second = day();
    let stem = if second == first {
        format!("deps/{second}-2")
    } else {
        format!("deps/{second}")
    };
    let [before, after] = [format!("{stem}/before"), format!("{stem}/after")];
    assert!(
        text.contains(&format!("pushed {before} and {after} to origin")),
        "{text}"
    );
    assert_eq!(at(&before), old_main);
    assert_eq!(at(&after), sb.main_sha());
    assert!(
        said(&after).contains("\nqtbridge 0.2.1 -> 0.3.0"),
        "{}",
        said(&after)
    );
    assert!(said(&after).contains(": 2 commit(s)"), "{}", said(&after));
    assert_eq!(tags_of(&origin).len(), 4);
    assert_eq!(tags_of(&origin), tags_of(&sb.repo));
}

/// Main has moved by the time the tags go out, so a push that fails
/// leaves the landing green, the tags here, and the command that
/// finishes the push said.
#[test]
fn an_unpushed_pair_stays_here_and_says_how_to_push_it() {
    let sb = Sandbox::new("land-tags-unpushed");
    let nowhere = sb.root.join("no-such-remote.git");
    let url = nowhere.display().to_string().replace('\\', "/");
    sb.git_ok(&sb.repo, &["remote", "add", "origin", &url]);
    sb.git_ok(&sb.repo, &["config", "branch.main.remote", "origin"]);
    sb.write(&sb.seat, "Cargo.lock", &lock_with_qtbridge("0.2.1"));
    sb.commit_all(&sb.seat, "chore: build with qtbridge", &[]);
    let (ok, text) = sb.land("worktree-a");
    assert!(ok, "{text}");
    assert_eq!(sb.main_sha(), sb.head(&sb.seat));
    let day = sb.git_ok(
        &sb.repo,
        &["log", "-1", "--format=%cd", "--date=short", "main"],
    );
    assert!(
        text.contains(&format!(
            "the version tags were not pushed to origin, so they stand here only — push them with \
             `git push --atomic origin refs/tags/deps/{day}/before refs/tags/deps/{day}/after`"
        )),
        "{text}"
    );
    assert_eq!(
        sb.git_ok(&sb.repo, &["tag", "-l"]),
        format!("deps/{day}/after\ndeps/{day}/before")
    );
}

/// The gate's steps rebuild the seat's task runner after the rebase, and
/// `land` is itself the binary in that slot. Windows cannot replace a
/// running image, so the slot is freed before the first step, and an
/// earlier landing's undeletable image is swept on the way past.
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

    // As cargo leaves it: the image under `deps/` with a hashed name, and
    // the slot a hard link to it. A running image is refused under either
    // name, so freeing only the one started from stops the gate's first
    // `cargo build -p xtask` at the other.
    let slot = sb.seat.join("target").join("debug");
    let deps = slot.join("deps");
    std::fs::create_dir_all(&deps).expect("the build slot");
    let suffix = std::env::consts::EXE_SUFFIX;
    let linked = deps.join(format!("xtask-a1b2c3d4e5f60718{suffix}"));
    std::fs::copy(EXE, &linked).expect("the runner cargo wrote");
    let running = slot.join(format!("xtask{suffix}"));
    std::fs::hard_link(&linked, &running).expect("the slot cargo links to it");
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
        !linked.exists(),
        "the name under `deps/` the slot was linked to is still the running image's, and the \
         link step of the gate's first build is refused there:\n{text}"
    );
    assert!(
        !left_behind.exists(),
        "an earlier landing's image was left in the slot's directory:\n{text}"
    );
    assert_eq!(sb.main_sha(), sb.head(&sb.seat));
}

/// A runner published by `fs::copy` can be held open by a neighbour's
/// fork, too briefly to catch, so it is held open here on purpose.
///
/// Linux only: POSIX says `execve` *may* refuse a file open for writing,
/// and Linux does.
#[test]
#[cfg(target_os = "linux")]
fn a_runner_held_open_for_writing_is_run_once_the_handle_goes() {
    // Only for the temp root: nothing here gates.
    let sb = Sandbox::new("busy");
    let slot = sb.root.join("slot");
    std::fs::create_dir_all(&slot).expect("the build slot");
    let running = slot.join("xtask");
    std::fs::copy(EXE, &running).expect("the runner in the slot");

    // Opened as it is: the file stays the runner throughout.
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

/// After committing it, the landing gates again on its own.
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

/// A seat dirty with only that generated file is not refused.
#[test]
fn land_commits_a_census_the_seat_was_holding_dirty() {
    let sb = Sandbox::new("land-census-dirty");
    sb.write_refs(&sb.seat, 22);
    let before = sb.commit_all(&sb.seat, "feat(core): twenty-two", &[]);
    let census = std::fs::read_to_string(sb.seat.join("crates/xtask/verb-census.txt"))
        .expect("the census in the seat");
    sb.write(
        &sb.seat,
        "crates/xtask/verb-census.txt",
        &format!("{census}stash --preset extra\tDriver Main\n"),
    );
    let (ok, text) = sb.land("worktree-a");
    assert!(ok && text.contains("before this landing"), "{text}");
    assert_eq!(
        sb.git_ok(&sb.seat, &["log", "-1", "--format=%s"]),
        "chore(xtask): the verb census as the land's gate rewrote it"
    );
    assert_eq!(sb.git_ok(&sb.seat, &["rev-parse", "HEAD~1"]), before);
    assert_eq!(sb.main_sha(), sb.head(&sb.seat));
}

/// Whoever's claim was on the seat comes off (CLAUDE.md §ビルド・テスト):
/// a claim is never lifted for its process being gone, so a letter the
/// landing left claimed would be one nobody hands back.
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
    assert!(text.contains("this session's own"), "{text}");
    assert!(
        !locks().contains("locked"),
        "the seat the landing emptied stayed claimed"
    );

    lock("claude-seat mine pid 4242");
    sb.write_refs(&sb.seat, 16);
    sb.commit_all(&sb.seat, "feat(core): sixteen", &[]);
    let (ok, text) = sb.land_as("worktree-a", "somebody-else");
    assert!(ok, "{text}");
    assert!(text.contains("released the seat claim"), "{text}");
    assert!(text.contains("held by session mine (pid 4242)"), "{text}");
    assert!(
        !locks().contains("locked"),
        "the user had this branch landed, so the letter is theirs to hand out"
    );

    // A lock a person wrote is nobody's to lift.
    lock("parked by hand");
    sb.write_refs(&sb.seat, 32);
    sb.commit_all(&sb.seat, "feat(core): thirty-two", &[]);
    let (ok, text) = sb.land_as("worktree-a", "mine");
    assert!(ok, "{text}");
    assert!(!text.contains("released the seat claim"), "{text}");
    assert!(
        locks().contains("locked parked by hand"),
        "a person's lock stays through a landing"
    );
}

#[test]
fn landing_an_already_merged_seat_releases_every_clean_claim() {
    let sb = Sandbox::new("land-again");
    let seat = sb.seat.display().to_string();
    let before = sb.main_sha();
    for (reason, dirty, released) in [
        ("claude-seat mine", false, true),
        ("claude-seat gone pid 2147483645", false, true),
        ("claude-seat theirs", false, true),
        ("claude-seat", false, true),
        ("parked by hand", false, false),
        ("claude-seat mine", true, false),
    ] {
        sb.git_ok(&sb.repo, &["worktree", "lock", "--reason", reason, &seat]);
        if dirty {
            sb.write_refs(&sb.seat, 64);
        }
        let (ok, text) = sb.land_as("worktree-a", "mine");
        assert!(ok && text.contains("nothing to land"), "{text}");
        let listing = sb.git_ok(&sb.repo, &["worktree", "list", "--porcelain"]);
        assert_eq!(!listing.contains("locked"), released, "{reason}: {text}");
        assert_eq!(sb.main_sha(), before);
        if !released {
            sb.git_ok(&sb.repo, &["worktree", "unlock", &seat]);
        }
    }
}

#[test]
fn codex_identity_releases_a_claim_without_a_claude_environment() {
    let sb = Sandbox::new("codex-claim");
    let seat = sb.seat.display().to_string();
    sb.git_ok(
        &sb.repo,
        &[
            "worktree",
            "lock",
            "--reason",
            "claude-seat codex-test",
            &seat,
        ],
    );
    let mut command = Command::new(EXE);
    sb.env(&mut command);
    command
        .args(["land", "worktree-a", "--dir"])
        .arg(&sb.repo)
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env_remove("CLAUDE_PID")
        .env("CODEX_THREAD_ID", "codex-test");
    let output = output_past_a_busy_image(&mut command, || {}).expect("spawn xtask");
    assert!(output.status.success(), "{output:?}");
    let listing = sb.git_ok(&sb.repo, &["worktree", "list", "--porcelain"]);
    assert!(!listing.contains("locked"), "{listing}");
}
