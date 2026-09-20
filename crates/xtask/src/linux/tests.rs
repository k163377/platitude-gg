//! `linux`'s own tests, in a file of their own (structure.md §分割).

use super::*;

fn words(line: &str) -> Vec<String> {
    line.split_whitespace().map(String::from).collect()
}

#[test]
fn the_small_image_takes_everything_that_holds_no_qt() {
    assert_eq!(
        stage_for(&words("test -p platitude-core --test it")),
        "core"
    );
    assert_eq!(stage_for(&words("test -p xtask -p platitude-core")), "core");
    assert_eq!(stage_for(&words("fmt --all --check")), "core");
    assert_eq!(stage_for(&[]), "core");
}

#[test]
fn anything_that_can_reach_the_app_takes_the_other_one() {
    assert_eq!(stage_for(&words("test")), "app");
    assert_eq!(stage_for(&words("build --workspace")), "app");
    assert_eq!(stage_for(&words("clippy -p platitude-app")), "app");
    assert_eq!(stage_for(&words("verify-ui commit --preset basic")), "app");
    // qmltestrunner and the QtTest QML module both ship with Qt.
    assert_eq!(stage_for(&words("qmltest")), "app");
}

#[test]
fn an_xtask_verb_is_run_through_the_task_runner() {
    assert_eq!(
        command_line(&words("verify-ui commit --preset basic"), None),
        words("cargo xtask verify-ui commit --preset basic")
    );
    assert_eq!(
        command_line(&words("demo-repo --preset basic"), None),
        words("cargo xtask demo-repo --preset basic")
    );
}

/// The tree at /work is the host's own checkout, so a cargo in there
/// that resolves is a cargo that can rewrite the host's `Cargo.lock`.
/// Every one of them is spelled `--locked`, typed by hand or not.
#[test]
fn a_cargo_that_resolves_is_locked_wherever_it_was_typed() {
    assert_eq!(
        command_line(&words("test -p platitude-core"), None),
        words("cargo test --locked -p platitude-core")
    );
    assert_eq!(
        command_line(&words("clippy -p xtask --all-targets"), None),
        words("cargo clippy --locked -p xtask --all-targets")
    );
    assert_eq!(
        command_line(&words("tree"), None),
        words("cargo tree --locked")
    );
    // Cargo's own one-letter aliases are the same verbs.
    assert_eq!(
        command_line(&words("t -p platitude-core"), None),
        words("cargo t --locked -p platitude-core")
    );
    assert_eq!(stage_for(&words("t")), "app");
    // The gate spells its own, and one line does not carry it twice.
    assert_eq!(
        command_line(&words("test --locked -p xtask --lib"), None),
        words("cargo test --locked -p xtask --lib")
    );
}

/// The default is locked, so a verb nobody thought of carries it too —
/// `fetch` writes a lock file of its own where there is none, and was
/// missed by a list of the verbs that resolve.
#[test]
fn a_verb_nobody_listed_is_locked_all_the_same() {
    assert_eq!(
        command_line(&words("fetch"), None),
        words("cargo fetch --locked")
    );
    for verb in ["vendor", "package", "rustc", "fix", "publish", "install"] {
        assert_eq!(
            command_line(&words(verb), None),
            words(&format!("cargo {verb} --locked")),
            "{verb}"
        );
    }
    // A third-party subcommand is locked as well: one that will not take
    // the flag says so where the person who typed it can read it, which
    // is the loud half of being wrong.
    assert_eq!(
        command_line(&words("deny check"), None),
        words("cargo deny --locked check")
    );
    // A line that is all options has no subcommand to spell it after,
    // and resolves nothing either.
    assert_eq!(
        command_line(&words("--version"), None),
        words("cargo --version")
    );
    assert_eq!(command_line(&words("--list"), None), words("cargo --list"));
}

/// **Cargo takes its own options ahead of the subcommand**, so the first
/// word is not the verb and a line that reads it as one hands the
/// container an unlocked cargo.
#[test]
fn cargos_own_options_come_before_the_subcommand() {
    assert_eq!(
        command_line(&words("--offline fetch"), None),
        words("cargo --offline fetch --locked")
    );
    assert_eq!(
        command_line(&words("-v test -p platitude-core"), None),
        words("cargo -v test --locked -p platitude-core")
    );
    // Value-bearing options in both spellings: the value is a word with
    // no dash on it, and is not the subcommand.
    assert_eq!(
        command_line(&words("--config net.retry=2 test"), None),
        words("cargo --config net.retry=2 test --locked")
    );
    assert_eq!(
        command_line(&words("--config=net.retry=2 test"), None),
        words("cargo --config=net.retry=2 test --locked")
    );
    assert_eq!(
        command_line(&words("--color always -Z unstable-options build"), None),
        words("cargo --color always -Z unstable-options build --locked")
    );
    // The attached spellings are one word each, and the word after them
    // is the subcommand.
    assert_eq!(
        command_line(&words("--color=always -Zscript build"), None),
        words("cargo --color=always -Zscript build --locked")
    );
    // The exceptions are the exceptions wherever the verb stands.
    assert_eq!(
        command_line(&words("--offline fmt --check"), None),
        words("cargo --offline fmt --check")
    );
    // Already locked, in either spelling — --frozen is --locked and
    // --offline in one word.
    assert_eq!(
        command_line(&words("--locked fetch"), None),
        words("cargo --locked fetch")
    );
    assert_eq!(
        command_line(&words("--frozen fetch"), None),
        words("cargo --frozen fetch")
    );
    // The image is chosen off the same word.
    assert_eq!(
        stage_for(&words("--offline test -p platitude-core")),
        "core"
    );
    assert_eq!(stage_for(&words("--offline test")), "app");
    assert_eq!(stage_for(&words("--config net.retry=2 build")), "app");
    // An xtask verb behind cargo's options is still ours.
    assert_eq!(
        command_line(&words("--offline verify-ui commit"), None),
        words("cargo --offline xtask verify-ui commit")
    );
    assert_eq!(stage_for(&words("--offline verify-ui commit")), "app");
}

/// Left alone on purpose: a verb that resolves nothing, and the verbs
/// whose whole purpose is to move the lock — `--locked` would refuse
/// the second kind outright.
#[test]
fn the_verbs_that_do_not_resolve_are_handed_over_as_typed() {
    assert_eq!(
        command_line(&words("fmt --all --check"), None),
        words("cargo fmt --all --check")
    );
    for line in [
        "update -p tracing",
        "generate-lockfile",
        "add thiserror",
        "remove thiserror",
        "clean",
    ] {
        assert_eq!(
            command_line(&words(line), None),
            words(&format!("cargo {line}"))
        );
    }
    // An xtask verb needs none of this: the alias carries it
    // (.cargo/config.toml).
    assert!(
        !command_line(&words("verify-ui commit"), None).contains(&"--locked".to_string()),
        "the alias already spells it"
    );
}

/// Past `--` the words are the program's.
#[test]
fn a_locked_beyond_the_separator_is_not_this_lines_own() {
    assert_eq!(
        command_line(&words("run -p xtask -- structure --locked"), None),
        words("cargo run --locked -p xtask -- structure --locked")
    );
}

/// A line that names this run's prepared copy starts from it and says
/// nothing about cargo: no `cargo xtask`, no alias, no resolve
/// (`linux::runner`).
#[test]
fn a_prepared_copy_starts_the_verb_and_no_cargo_does() {
    let copy = "/work/target/gate-runner/xtask-1758-40".to_string();
    assert_eq!(
        command_line(
            &words("verify-ui commit --preset basic"),
            Some(copy.clone())
        ),
        vec![
            copy.clone(),
            "verify-ui".into(),
            "commit".into(),
            "--preset".into(),
            "basic".into()
        ]
    );
    let line = command_line(&words("verify-ui wip --no-build"), Some(copy));
    assert!(!line.iter().any(|word| word == "cargo"), "{line:?}");
}

/// A copy that is not there is a preparation that did not happen. The
/// script says so and stops — the one thing that must never follow is a
/// quiet road back to cargo, and the script that guards a copy holds no
/// cargo on any of its roads, the red one included.
#[test]
fn a_line_that_names_a_copy_stops_when_the_copy_is_not_there() {
    let copy = "/work/target/gate-runner/xtask-1758-40";
    let line = watched_from_inside(&[copy.to_string(), "verify-ui".into()], Some(copy));
    let script = &line[2];
    assert!(
        !script.contains("cargo --version") && !script.contains("sha256sum"),
        "nothing in there resolves, so nothing looks at what a resolve reads: {script}"
    );
    assert!(script.contains("pgg-probe ran"), "a red line is still said");
    assert!(
        script.starts_with(&format!("if [ ! -x {copy} ]; then")),
        "{script}"
    );
    assert!(script.contains("exit 127"), "{script}");
    assert!(
        script.contains("nothing here falls back to cargo"),
        "the failure names what it refuses to do: {script}"
    );
    // The guard is the copy's alone: a cargo line is the line it was.
    let cargo = watched_from_inside(&words("cargo test --locked -p platitude-core"), None);
    assert!(cargo[2].starts_with("look()"), "{}", cargo[2]);
}

/// The container is `--rm`: what it did not say while it ran is gone.
/// The look is taken before the command and printed only if the command
/// fails, so a green run's log is the log it always was.
#[test]
fn the_command_runs_bracketed_by_a_look_at_what_a_resolve_reads() {
    let line = watched_from_inside(&words("cargo xtask verify-ui commit"), None);
    assert_eq!(line[..2], words("sh -c")[..]);
    // The command arrives as arguments.
    assert_eq!(line[3], IMAGE);
    assert_eq!(line[4..], words("cargo xtask verify-ui commit")[..]);
    let script = &line[2];
    for file in READ_TO_RESOLVE {
        assert!(script.contains(&format!("{WORK}/{file}")), "{file}");
    }
    // Every member's manifest, as a glob the shell in there expands —
    // a member added to the workspace is not one this forgets.
    assert!(
        script.contains("/work/crates/*/Cargo.toml"),
        "the members are globbed: {script}"
    );
    assert!(script.contains("before=$(look before)"), "{script}");
    assert!(script.contains("exit \"$code\""), "{script}");
}

/// **Nothing here names a container and nothing here reaps one.** A
/// name would be for finding an interrupted container to take away, and
/// that is a `docker rm` on this side with no ceiling over it. What it
/// was for is gone: an interrupted container can no longer reach this
/// checkout's copies (`linux::runner::SCRIPT`), and the cargo lock it
/// still holds is waited out under the step's own ceiling. The one
/// container that is named and removed is the gate's own, and both
/// happen under a ceiling in `container` (`remove_container`).
#[test]
fn no_container_started_here_is_named_or_reaped() {
    assert!(
        !args_of(&carried()).iter().any(|arg| arg == "--name"),
        "a named container is one something has to come back and remove"
    );
    let source = include_str!("../linux.rs");
    for reaping in [
        "docker\", \"rm",
        "\"rm\", \"--force\"",
        "reap_an_interrupted",
    ] {
        assert!(
            !source.contains(reaping),
            "linux.rs removes a container ({reaping}) outside any ceiling"
        );
    }
}

/// `--container` is a prepared copy's road and a marked one: a line
/// with either flag and not its partner, or a container without a
/// copy, is one nobody spelled on purpose — and so is `--rebuild`
/// beside a container already running from an image.
#[test]
fn a_container_line_needs_a_copy_and_a_mark_and_nothing_to_rebuild() {
    assert!(options(&words("--container c verify-ui wip")).is_err());
    assert!(options(&words("--runner r --container c verify-ui wip")).is_err());
    assert!(options(&words("--runner r --step m verify-ui wip")).is_err());
    assert!(
        options(&words(
            "--rebuild --runner r --container c --step m verify-ui wip"
        ))
        .is_err()
    );
    let all = options(&words("--runner r --container c --step m verify-ui wip"))
        .expect("the copy, the container and the mark");
    assert_eq!(all.copy.as_deref(), Some("r"));
    assert_eq!(all.container.as_deref(), Some("c"));
    assert_eq!(all.step.as_deref(), Some("m"));
    assert_eq!(all.at, 6, "the command begins after the options");
    // Without a container the line is what it was.
    let copy = options(&words("--runner r verify-ui wip")).expect("a copy alone");
    assert!(copy.container.is_none() && copy.step.is_none());
    // `stop` is the one line that names a container and no copy: it is
    // about what is already in there.
    let stop = options(&words("--container c --step m stop")).expect("a stop needs no copy");
    assert_eq!(stop.container.as_deref(), Some("c"));
    assert_eq!(stop.step.as_deref(), Some("m"));
    assert!(
        options(&words("--container c stop")).is_err(),
        "a stop with no mark"
    );
}

#[test]
fn volume_names_survive_a_windows_path() {
    assert_eq!(
        volume(
            Path::new("C:\\Users\\x\\IdeaProjects\\platitude-gg"),
            "target"
        ),
        "pgg-linux-target-platitude-gg"
    );
}

#[test]
fn mount_paths_are_forward_slashed() {
    assert_eq!(
        mount_path(Path::new("C:\\Users\\x\\platitude-gg")),
        "C:/Users/x/platitude-gg"
    );
}

#[test]
fn the_qt_version_comes_from_the_workflow() {
    let root = crate::tree::workspace_root();
    let version = qt_version(&root).expect("QT_VERSION in ci.yml");
    assert!(
        version.split('.').all(|part| part.parse::<u32>().is_ok()),
        "{version:?} does not look like a version"
    );
}

/// Every container is under the launcher's announcement to a measurement
/// and under the launcher's ticket, and takes neither of its own. Both
/// marks or neither: with only the first, a `linux verify-ui` holds a
/// compile's weight out here while the verb inside queues for weight of
/// its own, and enough of those pairs fill the machine with halves that
/// cannot move (`crate::budget`, `crate::still`).
#[test]
fn a_container_is_started_carrying_both_marks() {
    assert_eq!(
        args_of(&carried()),
        vec![
            "run".to_string(),
            "--rm".to_string(),
            "--env".to_string(),
            format!("{}=1", crate::still::UNDER),
            "--env".to_string(),
            format!("{}=1", crate::budget::HELD),
        ]
    );
}

/// The arguments as the process would see them.
pub(super) fn args_of(cmd: &Command) -> Vec<String> {
    cmd.get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}
