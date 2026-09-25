//! The sweep over a build directory written by hand, in the file shapes a
//! listing of a real `target/` shows
//! (ci/baseline/code-costs-windows-x64.md §build directory の世代).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::{
    Alive, CANONICAL, Gone, HARNESS, IF_MOVED, ONLY_IF_MOVED, Tail, asks, crate_of, generation,
    profiles_in, read_artifacts, stamp, stamped, sweep_profile, unit_key,
};
use crate::yard::Yard;

/// Writers only this suite needs, so they live here, not in [`crate::yard`].
impl Yard {
    fn file(&self, relative: &str, bytes: usize) -> PathBuf {
        let path = self.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the parents");
        }
        std::fs::write(&path, vec![b'x'; bytes]).expect("the file");
        path
    }

    /// A file dated `at`: the sweep dates a directory by its newest entry
    /// (`super::written_at`).
    fn file_at(&self, relative: &str, bytes: usize, at: SystemTime) -> PathBuf {
        let path = self.file(relative, bytes);
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .and_then(|file| file.set_modified(at))
            .expect("its date");
        path
    }
}

fn alive(deps: &[&str], build: &[&str]) -> Alive {
    Alive {
        deps: deps.iter().map(|s| (*s).to_string()).collect(),
        build: build.iter().map(|s| (*s).to_string()).collect(),
    }
}

fn names_under(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// `lib` goes only on the extensions rustc puts it on, and a Unix binary
/// has none: every spelling must map to one name, or keeping a unit takes
/// half of it away.
#[test]
fn every_spelling_of_one_unit_answers_to_one_name() {
    for (file, key) in [
        (
            "libplatitude_core-10e1c651a9583edf.rlib",
            "platitude_core-10e1c651a9583edf",
        ),
        (
            "libplatitude_core-10e1c651a9583edf.rmeta",
            "platitude_core-10e1c651a9583edf",
        ),
        (
            "platitude_core-10e1c651a9583edf.d",
            "platitude_core-10e1c651a9583edf",
        ),
        (
            "platitude_core-404cf966f957821d.exe",
            "platitude_core-404cf966f957821d",
        ),
        (
            "platitude_core-404cf966f957821d.pdb",
            "platitude_core-404cf966f957821d",
        ),
        (
            "platitude_core-404cf966f957821d",
            "platitude_core-404cf966f957821d",
        ),
        ("xtask.exe", "xtask"),
        ("xtask.d", "xtask"),
        // Stripping a `lib` rustc did not write would file `libc` under `c`.
        ("liblibc-7581adf255e9367e.rlib", "libc-7581adf255e9367e"),
        ("libc-7581adf255e9367e.d", "libc-7581adf255e9367e"),
    ] {
        assert_eq!(unit_key(file), key, "{file}");
    }
}

/// What an incremental session is named after; an unhashed bin is its own
/// crate.
#[test]
fn a_units_crate_is_its_name_without_the_hash() {
    assert_eq!(
        crate_of("platitude_core-10e1c651a9583edf"),
        "platitude_core"
    );
    assert_eq!(crate_of("xtask"), "xtask");
    // Not sixteen hex digits, so not a hash.
    assert_eq!(crate_of("xtask-inflight-13720-2"), "xtask-inflight-13720-2");
}

#[test]
fn the_live_set_is_read_off_the_two_message_shapes() {
    let text = [
        r#"{"reason":"compiler-artifact","filenames":["C:\\t\\c\\target\\debug\\xtask.exe","C:\\t\\c\\target\\debug\\xtask.pdb"],"executable":"C:\\t\\c\\target\\debug\\xtask.exe","fresh":true}"#,
        // A dashed bin: `deps/` holds it under the crate's name too.
        r#"{"reason":"compiler-artifact","filenames":["C:\\t\\c\\target\\debug\\pgg-todo-editor.exe"],"executable":"C:\\t\\c\\target\\debug\\pgg-todo-editor.exe","fresh":true}"#,
        r#"{"reason":"compiler-artifact","filenames":["C:\\t\\c\\target\\debug\\deps\\liblibc-7581adf255e9367e.rlib","C:\\t\\c\\target\\debug\\deps\\liblibc-7581adf255e9367e.rmeta"],"fresh":false}"#,
        r#"{"reason":"compiler-artifact","filenames":["C:\\t\\c\\target\\debug\\build\\cc-1a10f7f7bb5602bf\\build-script-build.exe"],"fresh":true}"#,
        r#"{"reason":"build-script-executed","out_dir":"C:\\t\\c\\target\\debug\\build\\cc-7801b9b6db4fe7b8\\out"}"#,
        r#"{"reason":"compiler-artifact","filenames":["C:\\t\\c\\target\\release\\deps\\libplatitude_core-8ad8598591cbaf1c.rmeta"],"fresh":true}"#,
        r#"{"reason":"compiler-artifact","filenames":["C:\\t\\c\\target\\hooks\\deps\\xtask-0000000000000000.exe"],"fresh":true}"#,
        r#"{"reason":"build-finished","success":true}"#,
    ]
    .join("\n");
    let mut live: BTreeMap<String, Alive> = BTreeMap::new();
    let compiled = read_artifacts(Path::new(r"C:\t\c"), &text, &mut live);

    assert_eq!(compiled, 1, "the one unit cargo said it had to build");
    let debug = live.get("debug").expect("the debug profile");
    assert_eq!(
        debug.deps.iter().map(String::as_str).collect::<Vec<_>>(),
        [
            "libc-7581adf255e9367e",
            "pgg-todo-editor",
            "pgg_todo_editor",
            "xtask",
        ]
    );
    assert_eq!(
        debug.build.iter().map(String::as_str).collect::<Vec<_>>(),
        ["cc-1a10f7f7bb5602bf", "cc-7801b9b6db4fe7b8"]
    );
    let release = live.get("release").expect("the release profile");
    assert_eq!(
        release.deps.iter().map(String::as_str).collect::<Vec<_>>(),
        ["platitude_core-8ad8598591cbaf1c"]
    );
    assert!(
        !live.contains_key("hooks"),
        "the hook's own slot is not swept: {live:?}"
    );
}

/// A live set read off another tree's paths would keep names nothing here
/// wrote.
#[test]
fn another_trees_paths_are_not_read() {
    let mut live: BTreeMap<String, Alive> = BTreeMap::new();
    let line = r#"{"reason":"compiler-artifact","filenames":["/t/other/target/debug/deps/libx-0000000000000000.rlib"]}"#;
    read_artifacts(Path::new("/t/c"), line, &mut live);
    assert!(live.is_empty(), "{live:?}");
}

/// Every generation of a unit but the live one goes, and every spelling
/// of the live one stays.
#[test]
fn only_what_the_live_set_names_stays() {
    let yard = Yard::new("generations");
    let debug = "target/debug";
    // waits(measured): a date handed to the files under test, awaited by nothing
    let long_ago = SystemTime::now() - Duration::from_secs(60 * 60 * 24 * 30);
    for hash in ["10e1c651a9583edf", "7fed29d0757554ac"] {
        for spelling in ["libplatitude_core-{h}.rlib", "libplatitude_core-{h}.rmeta"] {
            yard.file_at(
                &format!("{debug}/deps/{}", spelling.replace("{h}", hash)),
                8,
                long_ago,
            );
        }
        yard.file_at(
            &format!("{debug}/deps/platitude_core-{hash}.d"),
            8,
            long_ago,
        );
    }
    yard.file_at(&format!("{debug}/deps/xtask.exe"), 8, long_ago);
    yard.file_at(&format!("{debug}/deps/xtask.d"), 8, long_ago);
    // A landing's set-aside (`land::INFLIGHT`): cargo never named it.
    yard.file_at(
        &format!("{debug}/deps/xtask-inflight-13720-2.exe"),
        8,
        long_ago,
    );
    yard.file_at(
        &format!("{debug}/build/cc-1a10f7f7bb5602bf/build-script-build.exe"),
        8,
        long_ago,
    );
    yard.file_at(
        &format!("{debug}/build/cc-7801b9b6db4fe7b8/out/probe.rs"),
        8,
        long_ago,
    );
    yard.file_at(
        &format!("{debug}/build/cc-0000000000000000/out/probe.rs"),
        8,
        long_ago,
    );
    // Left alone whatever it holds: an unhashed unit keeps its bookkeeping.
    yard.file(
        &format!("{debug}/.fingerprint/platitude-core-7fed29d0757554ac/lib-platitude_core"),
        8,
    );

    let alive = alive(
        &["platitude_core-10e1c651a9583edf", "xtask"],
        &["cc-1a10f7f7bb5602bf", "cc-7801b9b6db4fe7b8"],
    );
    let freed = sweep_profile(&yard.join(debug), "debug", &alive, None, false)
        .expect("the sweep")
        .freed;

    assert_eq!(
        names_under(&yard.join(format!("{debug}/deps"))),
        [
            "libplatitude_core-10e1c651a9583edf.rlib",
            "libplatitude_core-10e1c651a9583edf.rmeta",
            "platitude_core-10e1c651a9583edf.d",
            "xtask.d",
            "xtask.exe",
        ]
    );
    assert_eq!(
        names_under(&yard.join(format!("{debug}/build"))),
        ["cc-1a10f7f7bb5602bf", "cc-7801b9b6db4fe7b8"]
    );
    assert_eq!(
        names_under(&yard.join(format!("{debug}/.fingerprint"))),
        ["platitude-core-7fed29d0757554ac"]
    );
    // Four dead files under deps and one dead build directory.
    assert_eq!(freed, 5 * 8);
}

/// Whatever the live set says, or a configuration in use would be rebuilt
/// after every sweep.
#[test]
fn what_was_written_since_the_last_sweep_stays() {
    let yard = Yard::new("recency");
    let debug = "target/debug";
    // waits(measured): a date handed to the files under test, awaited by nothing
    let floor = SystemTime::now() - Duration::from_secs(60 * 60 * 24);
    yard.file_at(
        &format!("{debug}/deps/libx-0000000000000000.rlib"),
        16,
        floor - Duration::from_secs(60 * 60),
    );
    yard.file_at(
        &format!("{debug}/deps/libx-1111111111111111.rlib"),
        16,
        floor + Duration::from_secs(60 * 60),
    );
    let freed = sweep_profile(
        &yard.join(debug),
        "debug",
        &alive(&[], &[]),
        Some(floor),
        false,
    )
    .expect("the sweep")
    .freed;
    assert_eq!(freed, 16);
    assert_eq!(
        names_under(&yard.join(format!("{debug}/deps"))),
        ["libx-1111111111111111.rlib"]
    );
}

#[test]
fn a_dry_run_leaves_the_tree_as_it_found_it() {
    let yard = Yard::new("dry");
    let debug = "target/debug";
    yard.file(&format!("{debug}/deps/libx-0000000000000000.rlib"), 16);
    yard.file(&format!("{debug}/deps/libx-1111111111111111.rlib"), 16);
    let alive = alive(&["x-0000000000000000"], &[]);
    let would = sweep_profile(&yard.join(debug), "debug", &alive, None, true)
        .expect("the dry run")
        .freed;
    assert_eq!(would, 16);
    assert_eq!(
        names_under(&yard.join(format!("{debug}/deps"))),
        ["libx-0000000000000000.rlib", "libx-1111111111111111.rlib"]
    );
}

/// No message names an incremental session, so it is judged by its date
/// against its crate's live compile.
#[test]
fn an_incremental_session_stands_or_falls_by_its_compile() {
    let yard = Yard::new("incremental");
    let debug = "target/debug";
    // waits(measured): a date handed to the files under test, awaited by nothing
    let now = SystemTime::now();
    let long_ago = now - Duration::from_secs(60 * 60 * 24);
    yard.file_at(
        &format!("{debug}/deps/libplatitude_core-10e1c651a9583edf.rmeta"),
        8,
        now,
    );

    let live = "platitude_core-0emviq0avpz9n";
    yard.file_at(
        &format!("{debug}/incremental/{live}/s-hmh4rz10dj-120hg0f/dep-graph.bin"),
        8,
        now,
    );
    // An interrupted session (`-working`): dead whatever its date.
    yard.file_at(
        &format!("{debug}/incremental/{live}/s-hmh5klwtki-0f301ir-working/x.o"),
        8,
        now,
    );
    yard.file_at(
        &format!("{debug}/incremental/platitude_core-1umsnbk8oozwe/s-old/dep-graph.bin"),
        8,
        long_ago,
    );
    yard.file_at(
        &format!("{debug}/incremental/pgg_todo_editor-017xhpis8lup9/s-new/dep-graph.bin"),
        8,
        now,
    );

    let alive = alive(&["platitude_core-10e1c651a9583edf"], &[]);
    sweep_profile(&yard.join(debug), "debug", &alive, None, false).expect("the sweep");

    assert_eq!(
        names_under(&yard.join(format!("{debug}/incremental"))),
        [live],
        "the stale session and the crate with nothing live both go"
    );
    assert_eq!(
        names_under(&yard.join(format!("{debug}/incremental/{live}"))),
        ["s-hmh4rz10dj-120hg0f"],
        "the interrupted session goes and the finished one stays"
    );
}

#[test]
fn a_directory_that_is_not_there_is_not_swept() {
    let yard = Yard::new("absent");
    let swept = sweep_profile(
        &yard.join("target/release"),
        "release",
        &alive(&["x"], &[]),
        None,
        false,
    )
    .expect("nothing to do");
    assert_eq!(swept.freed, 0);
    assert!(
        swept.walked,
        "a directory that was never built is read, not skipped: a skip is what holds the \
         generation open for the next tail"
    );
}

/// Cargo holds the profile's lock through a link, and a unit being
/// written now is live whatever an earlier reading said. The run must not
/// count as walked: a stamp over it would hold the generation closed until
/// the lock file next moves.
#[test]
fn a_profile_cargo_is_building_in_is_left_whole() {
    let yard = Yard::new("locked");
    let debug = "target/debug";
    yard.file(&format!("{debug}/deps/libx-1111111111111111.rlib"), 16);
    let lock = yard.file(&format!("{debug}/.cargo-lock"), 0);
    let held = std::fs::File::open(&lock).expect("the lock");
    held.lock().expect("this test holding it, as cargo would");

    let swept = sweep_profile(&yard.join(debug), "debug", &alive(&[], &[]), None, false)
        .expect("a sweep that took nothing");

    assert_eq!(swept.freed, 0);
    assert!(!swept.walked);
    assert_eq!(
        names_under(&yard.join(format!("{debug}/deps"))),
        ["libx-1111111111111111.rlib"]
    );
    held.unlock().expect("letting go");
}

#[test]
fn a_size_is_said_in_the_unit_it_fills() {
    assert_eq!(super::size(0), "0.0B");
    assert_eq!(super::size(1023), "1023.0B");
    assert_eq!(super::size(1024), "1.0KB");
    assert_eq!(super::size(1024 * 1024 * 3 / 2), "1.5MB");
    assert_eq!(super::size(1024 * 1024 * 1024 * 5), "5.0GB");
}

#[test]
fn what_went_is_counted_and_said() {
    let yard = Yard::new("tally");
    let file = yard.file("one", 2048);
    let mut gone = Gone::new("debug", false);
    gone.take(&file, "deps/one");
    assert_eq!(gone.done(), 2048);
    assert!(!file.exists());
}

/// A dependency added already moves the lock, hashed beside this; a member
/// or a comment moving must not rehash a tree.
#[test]
fn the_generation_reads_the_profile_sections_alone() {
    let manifest = "\
[workspace]
members = [\"crates/xtask\"]

[profile.dev.package.xtask]
opt-level = 3

[workspace.dependencies]
thiserror = \"2\"

[profile.shipped]
inherits = \"release\"
";
    assert_eq!(
        profiles_in(manifest),
        "[profile.dev.package.xtask]\nopt-level = 3\n\n[profile.shipped]\ninherits = \"release\"\n"
    );
    let moved = manifest.replace(
        "members = [\"crates/xtask\"]",
        "members = [\"crates/core\"]",
    );
    assert_eq!(profiles_in(&moved), profiles_in(manifest));
}

#[test]
fn the_generation_moves_with_the_lock() {
    let yard = Yard::new("generation");
    std::fs::write(yard.join("Cargo.lock"), "version = 4\n").expect("a lock");
    std::fs::write(yard.join("rust-toolchain.toml"), "[toolchain]\n").expect("a pin");
    std::fs::write(yard.join("Cargo.toml"), "[workspace]\n").expect("a manifest");
    let first = generation(&yard).expect("a key");
    std::fs::write(yard.join("Cargo.lock"), "version = 4\n# one more\n").expect("a lock");
    assert_ne!(generation(&yard).expect("a key"), first);
}

/// A line for a directory this does not walk would be read for nothing,
/// and a `{package}` line must match a package of this workspace.
#[test]
fn every_canonical_line_is_one_this_can_run() {
    assert!(!CANONICAL.is_empty());
    for line in CANONICAL {
        assert!(
            super::PROFILES.contains(&line.profile),
            "{:?} fills a directory this sweeps",
            line.words
        );
        assert!(
            !line.words.is_empty() && !line.words[0].starts_with('-'),
            "{:?} opens with a cargo subcommand",
            line.words
        );
        assert!(
            line.words.contains(&"--locked"),
            "{:?} is locked, like every cargo this runner starts",
            line.words
        );
        if line.words.contains(&"{package}") {
            assert!(
                ["platitude-core", "platitude-app", "xtask"]
                    .iter()
                    .any(|package| (line.packages)(package)),
                "{:?} names at least one of this workspace's packages",
                line.words
            );
        }
    }
}

/// A workspace member's bin gets no `extra-filename`, so every
/// configuration of it lands on one name under `deps/`: the line whose
/// configuration this runner's tools read must be the last to write it,
/// or the next tool relinks (`CANONICAL`).
#[test]
fn the_configuration_this_runners_tools_read_is_written_last() {
    let of = |profile: &str| -> Vec<&[&str]> {
        CANONICAL
            .iter()
            .filter(|line| line.profile == profile)
            .map(|line| line.words)
            .collect()
    };
    assert_eq!(
        of("debug").last().copied(),
        Some(["build", "--locked", "-p", "xtask"].as_slice()),
        "the task runner every gate step starts from is the last debug line"
    );
    assert_eq!(
        of("release"),
        [["build", "--locked", "--release", "--features", HARNESS].as_slice()],
        "the release verify-ui drives is the only line in that profile — one without the \
         harness writes the same bin under the same name, and the product comes from `shipped`"
    );
    // Panics on a missing line: compared as `Option`, `None` would sort
    // first and pass.
    let at = |words: &[&str]| {
        CANONICAL
            .iter()
            .position(|line| line.words == words)
            .unwrap_or_else(|| panic!("{words:?} is a line of the canonical set"))
    };
    assert!(
        at(&["test", "--locked", "--workspace", "--no-run"])
            < at(&["test", "--locked", "-p", "{package}", "--no-run"]),
        "the per-package test lines, which is how the gate runs them, leave the bin a \
         package's integration tests are handed"
    );
}

/// A sweep out here writes nothing the container's volume reads, so no
/// host-side sweep can close the volume's generation (`sweep::asks`).
#[test]
fn a_stamp_answers_for_the_directory_it_stands_in() {
    let here = Yard::new("stamp-here");
    let volume = Yard::new("stamp-volume");
    for (yard, lock) in [
        (&here, "version = 4\n"),
        (&volume, "version = 4\n# in there\n"),
    ] {
        std::fs::write(yard.join("Cargo.lock"), lock).expect("a lock");
        std::fs::write(yard.join("rust-toolchain.toml"), "[toolchain]\n").expect("a pin");
        std::fs::write(yard.join("Cargo.toml"), "[workspace]\n").expect("a manifest");
        std::fs::create_dir_all(yard.join("target/sweep")).expect("somewhere to stamp");
    }
    stamp(&here).expect("this tree's stamp");
    assert_eq!(
        stamped(&here).map(|(key, _)| key),
        Some(generation(&here).expect("this tree's key")),
        "the directory that was swept carries the key it was swept under"
    );
    assert_eq!(
        stamped(&volume),
        None,
        "and the other one carries nothing, so its next tail reads it as unswept"
    );
}

/// The volume has its own rustc, key and stamp, and the verb in there
/// compares them; a tail that let this tree's key answer for both would
/// skip the container whenever the host had already swept.
#[test]
fn a_tail_with_a_linux_side_asks_the_volume_on_the_volumes_own_terms() {
    let tier = |whatever_the_key_says, the_volume_too| {
        asks(&Tail {
            whatever_the_key_says,
            the_volume_too,
        })
    };
    assert_eq!(tier(false, true), Some(false), "gate, and the key decides");
    assert_eq!(tier(true, true), Some(true), "gate --all, on both sides");
    assert_eq!(
        tier(false, false),
        None,
        "gate --host-only starts no container, and a sweep is no reason to"
    );
    assert_eq!(asks(&Tail::after_a_landing()), Some(false));
}

/// What the container is told with, and what the verb parses, are one
/// word: the call is what `linux::sweep_the_volume` hands it.
#[test]
fn the_containers_half_is_told_with_the_option_this_verb_parses() {
    assert_eq!(IF_MOVED.options().collect::<Vec<_>>(), [ONLY_IF_MOVED]);
}
