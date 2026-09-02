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
        command_line(&words("test -p platitude-core")),
        words("cargo test -p platitude-core")
    );
}

#[test]
fn volume_names_survive_a_windows_path() {
    assert_eq!(
        volume(
            Path::new("C:\\Users\\x\\IdeaProjects\\platitude-gg"),
            "target"
        ),
        "pg-linux-target-platitude-gg"
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
