//! What a run leaves behind: the stamps a green step and a green commit
//! earn, when they are reused and when they are not, and the one gate a
//! tree holds while it runs.

use crate::support::{ALWAYS, EXE, Sandbox, output_past_a_busy_image, set, without_always};

#[test]
fn a_copied_reader_reuses_the_graph_despite_its_new_timestamp() {
    let sb = Sandbox::new("graph-reader-copy");
    sb.gate_ok(&sb.repo, &["--dry-run"]);
    let copy = sb
        .root
        .join(format!("reader{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(EXE, &copy).expect("copy the same reader");
    let stamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
    std::fs::File::options()
        .write(true)
        .open(&copy)
        .expect("reader copy")
        .set_times(std::fs::FileTimes::new().set_modified(stamp))
        .expect("different timestamp");
    let run = || {
        let mut command = std::process::Command::new(&copy);
        command.args(["gate", "--dry-run", "--dir"]).arg(&sb.seat);
        sb.env(&mut command);
        let out = output_past_a_busy_image(&mut command, || {}).expect("run the copied reader");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.status.success(), "{text}");
        text
    };
    let reused = run();
    assert!(
        reused.contains(" (kept)"),
        "same bytes in another tree: {reused}"
    );

    // Bytes past the loaded image leave a PE / ELF runnable.
    {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&copy)
            .expect("reader copy");
        file.write_all(b"graph-cache-reader-probe")
            .expect("change the reader bytes");
        file.set_times(std::fs::FileTimes::new().set_modified(stamp))
            .expect("keep its timestamp");
    }
    let changed = run();
    assert!(!changed.contains(" (kept)"), "changed bytes: {changed}");
    {
        use std::io::{Seek, SeekFrom, Write};
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(&copy)
            .expect("reader copy");
        let replacement = b"Graph-cache-reader-probe";
        file.seek(SeekFrom::End(-(replacement.len() as i64)))
            .expect("same-sized trailing bytes");
        file.write_all(replacement)
            .expect("change bytes without changing length");
        file.set_times(std::fs::FileTimes::new().set_modified(stamp))
            .expect("keep timestamp again");
    }
    let same_metadata = run();
    assert!(
        !same_metadata.contains(" (kept)"),
        "same length and time, different bytes: {same_metadata}"
    );
    assert!(run().contains(" (kept)"));
}

#[test]
fn a_changed_tree_invalidates_the_graph_and_cache_io_failures_are_reported() {
    let sb = Sandbox::new("graph-verdicts");
    let first = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(first.contains("graph cache miss: entry absent"), "{first}");
    assert!(
        sb.gate_ok(&sb.seat, &["--dry-run"])
            .contains("graph cache hit;")
    );
    sb.write_refs(&sb.seat, 12);
    sb.commit_all(&sb.seat, "test: change graph input", &[]);
    let changed = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(
        changed.contains("graph cache miss: entry absent"),
        "{changed}"
    );

    let blocked = Sandbox::new("graph-cache-io");
    blocked.write(
        &blocked.repo,
        ".git/pgg-gate/graphs",
        "a file, not a directory\n",
    );
    let text = blocked.gate_ok(&blocked.seat, &["--dry-run"]);
    assert!(text.contains("not saved: cache directory:"), "{text}");
    assert!(!text.contains(" (kept)"), "{text}");
}

/// Whitespace runs collapsed, so a test asserts a row's content and not
/// its column width.
fn squeezed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn a_truncated_graph_is_rebuilt_and_hidden_untracked_files_prevent_reuse() {
    let sb = Sandbox::new("graph-reuse");
    sb.gate_ok(&sb.seat, &["--dry-run"]);
    let warm = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(warm.contains(" (kept)"), "{warm}");
    let shelf = sb.repo.join(".git/pgg-gate/graphs");
    let cached: Vec<_> = std::fs::read_dir(&shelf)
        .expect("graph shelf")
        .map(|entry| entry.expect("entry").path())
        .collect();
    assert_eq!(cached.len(), 1);
    let text = std::fs::read_to_string(&cached[0]).expect("cached graph");
    let first_edge = text.find("\nD\t").expect("graph edges");
    std::fs::write(&cached[0], &text[..first_edge + 1]).expect("truncate at record boundary");
    let rebuilt = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(!rebuilt.contains(" (kept)"), "{rebuilt}");
    assert!(
        rebuilt.contains("graph cache miss: invalid cache payload"),
        "{rebuilt}"
    );
    assert!(
        sb.gate_ok(&sb.seat, &["--dry-run"])
            .contains("graph cache hit;")
    );
    sb.git_ok(&sb.seat, &["config", "status.showUntrackedFiles", "no"]);
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/Untracked.qml",
        "Item {}\n",
    );
    let dirty = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(!dirty.contains(" (kept)"), "{dirty}");
    assert!(
        dirty.contains("graph cache not used: uncommitted or untracked files"),
        "{dirty}"
    );
}

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

    // The diff now holds both modules: the core step (two filters) is new;
    // the app's unit tests read stash, which did not move; the verbs and
    // the shipped build read the whole core, which did.
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
fn markdown_beside_code_does_not_invalidate_passed_steps() {
    let sb = Sandbox::new("markdown-cache");
    sb.write_refs(&sb.seat, 4);
    sb.commit_all(&sb.seat, "feat(core): four", &[]);
    sb.gate_ok(&sb.seat, &[]);
    assert!(!without_always(&sb.ran()).is_empty());
    for content in [Some("# First\n"), Some("# Second\n"), None] {
        let path = "crates/platitude-core/src/README.md";
        if let Some(content) = content {
            sb.write(&sb.seat, path, content);
        } else {
            std::fs::remove_file(sb.seat.join(path)).expect("delete markdown");
        }
        sb.commit_all(&sb.seat, "docs: change beside code", &[]);
        sb.gate_ok(&sb.seat, &[]);
        assert_eq!(sb.ran(), set(&ALWAYS), "markdown invalidated a passed step");
    }
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

/// `--keep-going` runs past the red, so the other side's passed steps
/// keep their stamps.
#[test]
fn a_red_step_leaves_main_where_it_was() {
    let sb = Sandbox::new("red");
    sb.write_refs(&sb.seat, 7);
    sb.commit_all(&sb.seat, "feat(core): seven", &[]);
    let (ok, text) = sb.gate(
        &sb.seat,
        &["--keep-going"],
        &[("PGG_GATE_FAKE_FAIL", "test platitude-core 1")],
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
    sb.commit_all(&sb.repo, "docs: main moved", &[("PGG_GATE_SKIP", "1")]);
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
    sb.write(&sb.repo, "crates/platitude-core/src/stash.rs", "pub fn stash() { let _ = 10; }\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn t() { stash() }\n}\n");
    sb.commit_all(
        &sb.repo,
        "feat(core): main moved",
        &[("PGG_GATE_SKIP", "1")],
    );
    sb.write_refs(&sb.seat, 11);
    sb.commit_all(&sb.seat, "feat(core): eleven", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let _ = sb.ran();
    let (ok, text) = sb.land("worktree-a");
    assert!(ok, "{text}");
    let again = without_always(&sb.ran());
    // The refs step reads refs.rs and, in the it binary, refs_integration
    // and support: none moved. clippy reads the whole crate, which did.
    assert!(
        !again.contains("test platitude-core 1"),
        "refs' inputs did not move: {again:?}"
    );
    assert!(
        again.contains("clippy platitude-core"),
        "the crate as a whole moved: {again:?}"
    );
}

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

    held.unlock().expect("let the first gate's lock go");
    drop(held);
    sb.gate_ok(&sb.seat, &[]);
    assert!(
        !target.join("gate-running").exists(),
        "the note comes down with the gate that wrote it"
    );
}

/// Why every lock here is released by unlocking and not by closing
/// (rules-refs/core.md「lock は unlock で離す」): a closed lock stays with
/// any child holding the description, and the next gate is refused.
/// Linux only: that inheritance is what `flock(2)` promises there.
#[test]
#[cfg(target_os = "linux")]
fn a_lock_frees_the_tree_when_unlocked_and_not_when_merely_closed() {
    use std::process::{Command, Stdio};

    let sb = Sandbox::new("carried");
    sb.write_refs(&sb.seat, 18);
    sb.commit_all(&sb.seat, "feat(core): eighteen", &[]);
    let target = sb.seat.join("target");
    std::fs::create_dir_all(&target).expect("the seat's target");
    let note = target.join("gate-running");
    let lock = target.join("gate-running.lock");
    let first_gate = || {
        std::fs::write(&note, "pid 424242\nsince 0\nwhat gate --all\n")
            .expect("the first gate's note");
        let held = std::fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock)
            .expect("the first gate's lock");
        held.try_lock().expect("held from here");
        held
    };
    // Holding the description as stdin, the child stands in for a
    // neighbour's forked child before its `execve`.
    let carry = |held: &std::fs::File| {
        Command::new("sleep")
            .arg("60")
            .stdin(Stdio::from(
                held.try_clone()
                    .expect("a second handle on the description"),
            ))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("a child handed the lock's description")
    };

    // The closed half goes last: after it a neighbour's forked child may
    // still hold the lock, so nothing here may take it again.
    let held = first_gate();
    let mut unlocking = carry(&held);
    held.unlock().expect("let the first gate's lock go");
    drop(held);
    sb.gate_ok(&sb.seat, &[]);
    assert!(
        unlocking.try_wait().expect("ask after the child").is_none(),
        "the child let the description go before the tree was asked for"
    );
    unlocking.kill().expect("the child that carried it");
    unlocking.wait().expect("the child that carried it");
    assert!(
        !note.exists(),
        "the note comes down with the gate that wrote it"
    );

    let held = first_gate();
    let mut closing = carry(&held);
    drop(held);
    let (ok, text) = sb.gate(&sb.seat, &[], &[]);
    assert!(
        !ok && text.contains("already running"),
        "a lock let go of by closing it was not carried by the child:\n{text}"
    );
    closing.kill().expect("the child that carried it");
    closing.wait().expect("the child that carried it");
}

/// A verb that rewrote its census line leaves a tree that is not the
/// commit, so there is nothing to stamp.
#[test]
fn a_gate_whose_verbs_rewrote_the_census_stamps_nothing_until_it_is_committed() {
    let sb = Sandbox::new("rewrote");
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/StashPane.qml",
        "Item {\n    width: 3\n    property var model: StashModel\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(app-ui): pane", &[]);
    let (ok, text) = sb.gate(
        &sb.seat,
        &[],
        &[("PGG_GATE_FAKE_REWRITE", "verify stash --preset basic")],
    );
    assert!(!ok, "{text}");
    assert!(
        text.contains("the tree moved with them") && text.contains("review the diff"),
        "{text}"
    );
    // Named by verb: when a component comes or goes the census diff is the
    // whole file, and the moved line is lost in it.
    assert!(
        squeezed(&text).contains("+Theme stash --preset basic"),
        "the record names the verb whose line moved: {text}"
    );
    assert_eq!(
        sb.git_ok(&sb.seat, &["status", "--porcelain"]),
        "M crates/xtask/verb-census.txt",
        "the rewrite stands in the tree"
    );
    let merge = sb.git(&sb.repo, &["merge", "--ff-only", "worktree-a"], &[]);
    assert!(merge.is_err(), "nothing was stamped: {merge:?}");
    let _ = sb.ran();
    sb.commit_all(&sb.seat, "chore(xtask): the verb census", &[]);
    let text = sb.gate_ok(&sb.seat, &[]);
    assert!(
        text.contains("cached verify stash --preset basic"),
        "{text}"
    );
    assert!(
        text.contains("census  no line moved"),
        "the run over the commit of it moved nothing: {text}"
    );
    assert_eq!(
        sb.ran(),
        set(&ALWAYS),
        "the commit of the census owed nothing new: the census is no step's input"
    );
    sb.git_ok(&sb.repo, &["merge", "--ff-only", "worktree-a"]);
}

/// A host-only stamp over a full one would send the landing back through
/// a container side that already passed.
#[test]
fn a_daily_run_keeps_a_full_stamp_it_finds() {
    let sb = Sandbox::new("keep-full");
    sb.write_refs(&sb.seat, 18);
    sb.commit_all(&sb.seat, "feat(core): eighteen", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let text = sb.gate_ok(&sb.seat, &["--host-only"]);
    assert!(text.contains("keeps its full stamp"), "{text}");
    sb.git_ok(&sb.repo, &["merge", "--ff-only", "worktree-a"]);
}

/// The steps that build and run the app key on the two product
/// directories whole, test targets included — which is what catches a
/// file the product opens by a path no source scan can follow (here
/// spelled across two files). Pins the absence of a narrower key
/// (反映前テストの機械化.md「テスト専用の配下を指紋から落とす案は採らない」).
#[test]
fn a_file_the_product_opens_is_owed_though_no_scan_could_find_it() {
    let sb = Sandbox::new("path-across-two-files");
    // The steps are faked, so nothing checks that the reader opens this
    // file: both are built from the same pieces so they cannot drift.
    let package = "crates/platitude-app";
    let [root, under, name] = ["tests", "qml", "Probe.qml"];
    let read = format!("{package}/{root}/{under}/{name}");
    // A product change, so the steps below are selected at all; their key
    // is what is under test.
    sb.write(
        &sb.seat,
        &format!("{package}/src/ui/StashPane.qml"),
        "Item {\n    width: 7\n    property var model: StashModel\n}\n",
    );
    sb.write(
        &sb.seat,
        &format!("{package}/src/paths.rs"),
        &format!("pub const TEST_ROOT: &str = \"{root}\";\n"),
    );
    sb.write(
        &sb.seat,
        &format!("{package}/src/probe.rs"),
        &format!(
            "use crate::paths::TEST_ROOT;\npub fn read() -> String {{\n    let p = \
             std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\"))\n        \
             .join(TEST_ROOT)\n        .join(\"{under}\")\n        .join(\"{name}\");\n    \
             std::fs::read_to_string(p).unwrap()\n}}\n"
        ),
    );
    sb.write(
        &sb.seat,
        &format!("{package}/src/main.rs"),
        "mod models;\nmod paths;\nmod probe;\nfn main() {}\n",
    );
    sb.write(&sb.seat, &read, "Item {}\n");
    sb.commit_all(&sb.seat, "feat(app): a reader of a file under tests", &[]);
    sb.gate_ok(&sb.seat, &[]);
    // Green first, so the second run is about a stamp.
    let built = ["shipped", "bare", "verify stash --preset basic"];
    let first = without_always(&sb.ran());
    for id in built {
        assert!(first.contains(id), "{id} did not run: {first:?}");
    }

    sb.write(&sb.seat, &read, "Item {\n    property int x: 1\n}\n");
    sb.commit_all(&sb.seat, "test(app-ui): the file it opens", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let again = without_always(&sb.ran());
    for id in built {
        assert!(
            again.contains(id),
            "{id} kept a stamp over a file the product opens: {again:?}"
        );
    }
}

/// What each build step owes again for a change that no graph of the
/// sources sees but every build reads.
const EVERY_BUILD: [&str; 11] = [
    "test platitude-core (all)",
    "test platitude-app (all)",
    "test xtask (all)",
    "test it (all)",
    "clippy platitude-core",
    "clippy platitude-app",
    "clippy xtask",
    "shipped",
    "verify stash --preset basic",
    "bare",
    "deny",
];

/// cargo's configuration is read by every build it configures — its `[env]`
/// reaches every C and C++ compile a build script runs — so a change to it
/// asks each build again, and not only the selection.
#[test]
fn a_cargo_config_change_asks_every_build_again() {
    let sb = Sandbox::new("cargo-config-key");
    let config = ".cargo/config.toml";
    sb.write(&sb.seat, config, "[env]\nCFLAGS = \"-DFIRST\"\n");
    sb.commit_all(&sb.seat, "build: a flag", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let first = without_always(&sb.ran());
    for id in EVERY_BUILD {
        assert!(first.contains(id), "{id} did not run: {first:?}");
    }

    sb.write(&sb.seat, config, "[env]\nCFLAGS = \"-DSECOND\"\n");
    sb.commit_all(&sb.seat, "build: another flag", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let again = without_always(&sb.ran());
    for id in EVERY_BUILD {
        assert!(
            again.contains(id),
            "{id} kept its stamp over {config}: {again:?}"
        );
    }
}

/// Stamps included: a cached path that filed nothing would leave the
/// table short by exactly the verbs it never ran. The gate checks its
/// books against the plan, so such a path is a red run.
#[test]
fn a_run_that_ran_nothing_still_files_a_row_for_every_step() {
    let sb = Sandbox::new("ledger-cached");
    // A change the census's verb answers for, so the verbs are in the plan
    // (`selection::a_core_change_owes_what_reads_it_and_nothing_beside_it`).
    sb.write(
        &sb.seat,
        "crates/platitude-core/src/stash.rs",
        "pub fn stash() { let _ = 41; }\n#[cfg(test)]\nmod tests {\n    use super::*;\n    \
         #[test]\n    fn t() { stash() }\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(core): forty-one", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let ran = sb.ran();
    assert!(
        ran.iter().any(|id| id.starts_with("verify ")),
        "the first run owes the census's verb: {ran:?}"
    );

    sb.gate_ok(&sb.seat, &[]);
    assert!(
        without_always(&sb.ran()).is_empty(),
        "the second run of one commit runs nothing a stamp answers for"
    );
    let ledger = sb.ledger(&sb.seat);
    assert!(!ledger.is_empty(), "a ledger beside the record");
    let verbs: Vec<&(String, String, String)> = ledger
        .iter()
        .filter(|(_, id, _)| id.starts_with("verify"))
        .collect();
    assert!(verbs.len() >= 2, "the verb on both sides: {ledger:?}");
    for row in &verbs {
        assert_eq!(row.2, "cached", "{row:?}");
    }
    // The always-steps carry no stamp of their own.
    assert!(
        ledger
            .iter()
            .all(|(_, id, outcome)| outcome == "cached" || ALWAYS.contains(&id.as_str())),
        "a run that ran nothing has nothing else to say: {ledger:?}"
    );
}

/// A red stops its group; the steps behind it get a `not-run` row, so the
/// books stay whole and a skipped step is told apart from a lost one.
#[test]
fn a_red_leaves_the_steps_behind_it_named_rather_than_missing() {
    let sb = Sandbox::new("ledger-red");
    sb.write_refs(&sb.seat, 43);
    sb.commit_all(&sb.seat, "feat(core): forty-three", &[]);
    let (ok, text) = sb.gate(
        &sb.seat,
        &["--keep-going"],
        &[("PGG_GATE_FAKE_FAIL", "test platitude-core 1")],
    );
    assert!(!ok, "{text}");
    assert!(
        !text.contains("side's ledger"),
        "the books are whole, so the run says nothing about them:\n{text}"
    );
    let ledger = sb.ledger(&sb.seat);
    assert!(
        ledger
            .iter()
            .any(|(_, id, outcome)| id == "test platitude-core 1" && outcome == "FAIL"),
        "{ledger:?}"
    );
    assert!(
        ledger.iter().any(|(_, _, outcome)| outcome == "not-run"),
        "the steps the red stopped it short of: {ledger:?}"
    );
}
