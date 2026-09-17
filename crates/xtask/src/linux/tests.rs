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
        command_line(&words("verify-ui commit --preset basic")),
        words("cargo xtask verify-ui commit --preset basic")
    );
    assert_eq!(
        command_line(&words("demo-repo --preset basic")),
        words("cargo xtask demo-repo --preset basic")
    );
}

/// The tree at /work is the host's own checkout, so a cargo in there
/// that resolves is a cargo that can rewrite the host's `Cargo.lock`.
/// Every one of them is spelled `--locked`, typed by hand or not.
#[test]
fn a_cargo_that_resolves_is_locked_wherever_it_was_typed() {
    assert_eq!(
        command_line(&words("test -p platitude-core")),
        words("cargo test --locked -p platitude-core")
    );
    assert_eq!(
        command_line(&words("clippy -p xtask --all-targets")),
        words("cargo clippy --locked -p xtask --all-targets")
    );
    assert_eq!(command_line(&words("tree")), words("cargo tree --locked"));
    // Cargo's own one-letter aliases are the same verbs.
    assert_eq!(
        command_line(&words("t -p platitude-core")),
        words("cargo t --locked -p platitude-core")
    );
    assert_eq!(stage_for(&words("t")), "app");
    // The gate spells its own, and one line does not carry it twice.
    assert_eq!(
        command_line(&words("test --locked -p xtask --lib")),
        words("cargo test --locked -p xtask --lib")
    );
}

/// The default is locked, so a verb nobody thought of carries it too —
/// `fetch` writes a lock file of its own where there is none, and was
/// missed by a list of the verbs that resolve.
#[test]
fn a_verb_nobody_listed_is_locked_all_the_same() {
    assert_eq!(command_line(&words("fetch")), words("cargo fetch --locked"));
    for verb in ["vendor", "package", "rustc", "fix", "publish", "install"] {
        assert_eq!(
            command_line(&words(verb)),
            words(&format!("cargo {verb} --locked")),
            "{verb}"
        );
    }
    // A third-party subcommand is locked as well: one that will not take
    // the flag says so where the person who typed it can read it, which
    // is the loud half of being wrong.
    assert_eq!(
        command_line(&words("deny check")),
        words("cargo deny --locked check")
    );
    // A line that is all options has no subcommand to spell it after,
    // and resolves nothing either.
    assert_eq!(command_line(&words("--version")), words("cargo --version"));
    assert_eq!(command_line(&words("--list")), words("cargo --list"));
}

/// **Cargo takes its own options ahead of the subcommand**, so the first
/// word is not the verb and a line that reads it as one hands the
/// container an unlocked cargo.
#[test]
fn cargos_own_options_come_before_the_subcommand() {
    assert_eq!(
        command_line(&words("--offline fetch")),
        words("cargo --offline fetch --locked")
    );
    assert_eq!(
        command_line(&words("-v test -p platitude-core")),
        words("cargo -v test --locked -p platitude-core")
    );
    // Value-bearing options in both spellings: the value is a word with
    // no dash on it, and is not the subcommand.
    assert_eq!(
        command_line(&words("--config net.retry=2 test")),
        words("cargo --config net.retry=2 test --locked")
    );
    assert_eq!(
        command_line(&words("--config=net.retry=2 test")),
        words("cargo --config=net.retry=2 test --locked")
    );
    assert_eq!(
        command_line(&words("--color always -Z unstable-options build")),
        words("cargo --color always -Z unstable-options build --locked")
    );
    // The attached spellings are one word each, and the word after them
    // is the subcommand.
    assert_eq!(
        command_line(&words("--color=always -Zscript build")),
        words("cargo --color=always -Zscript build --locked")
    );
    // The exceptions are the exceptions wherever the verb stands.
    assert_eq!(
        command_line(&words("--offline fmt --check")),
        words("cargo --offline fmt --check")
    );
    // Already locked, in either spelling — --frozen is --locked and
    // --offline in one word.
    assert_eq!(
        command_line(&words("--locked fetch")),
        words("cargo --locked fetch")
    );
    assert_eq!(
        command_line(&words("--frozen fetch")),
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
        command_line(&words("--offline verify-ui commit")),
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
        command_line(&words("fmt --all --check")),
        words("cargo fmt --all --check")
    );
    for line in [
        "update -p tracing",
        "generate-lockfile",
        "add thiserror",
        "remove thiserror",
        "clean",
    ] {
        assert_eq!(command_line(&words(line)), words(&format!("cargo {line}")));
    }
    // An xtask verb needs none of this: the alias carries it
    // (.cargo/config.toml).
    assert!(
        !command_line(&words("verify-ui commit")).contains(&"--locked".to_string()),
        "the alias already spells it"
    );
}

/// Past `--` the words are the program's.
#[test]
fn a_locked_beyond_the_separator_is_not_this_lines_own() {
    assert_eq!(
        command_line(&words("run -p xtask -- structure --locked")),
        words("cargo run --locked -p xtask -- structure --locked")
    );
}

/// The container is `--rm`: what it did not say while it ran is gone.
/// The look is taken before the command and printed only if the command
/// fails, so a green run's log is the log it always was.
#[test]
fn the_command_runs_bracketed_by_a_look_at_what_a_resolve_reads() {
    let line = watched_from_inside(&words("cargo xtask verify-ui commit"));
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
