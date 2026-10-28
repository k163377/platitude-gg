//! What the QML tests are owed by, and what keeps their stamp: a change at
//! any boundary a run of them reads across — the runner as built, Qt's
//! selection and its pin, what it stages and hands `qmltestrunner`, what
//! the tests load past that, and on the Linux side the image and what
//! starts the run in it — plans both steps that read it, moves their key
//! and runs them; a change to nothing a run reads leaves their green.

use std::collections::BTreeMap;

use crate::support::{Ran, Sandbox};

const HOST: &str = "qmltest";
const LINUX: &str = "qmltest-linux";
const BOTH: &[&str] = &[HOST, LINUX];
const TEST: &str = "crates/platitude-app/tests/qml/tst_census.qml";
const QT: &str = "crates/xtask/src/qt.rs";

/// The miniature's QML test run: the runner, which stages the product's
/// module and hands the tests' directory over (named in its strings, as
/// the real one names them) and finds Qt through `qt.rs`, whose own tests
/// read the seat module; a test that loads the harness's module and a
/// helper by relative import; a stub; and the module that starts a run in
/// the container.
fn with_the_qml_runner(name: &str) -> (Sandbox, String) {
    let sb = Sandbox::new(name);
    let files: &[(&str, &str)] = &[
        (
            "crates/xtask/src/main.rs",
            "mod demo;\nmod linux;\nmod qmltest;\nmod qt;\nmod seats;\nmod verify;\nfn main() {}\n",
        ),
        (
            "crates/xtask/src/qmltest.rs",
            "use crate::qt;\nconst TESTS: &str = \"crates/platitude-app/tests/qml\";\n\
             const UI: &str = \"crates/platitude-app/src/ui\";\n\
             pub fn run() { let _ = (TESTS, UI); }\n",
        ),
        (QT, &qt("C:\\\\Windows\\\\Fonts", "")),
        ("crates/xtask/src/seats.rs", "pub fn seat() {}\n"),
        (
            "crates/xtask/src/linux.rs",
            "mod container;\npub fn run() { container::exec_in(); }\n",
        ),
        (
            "crates/xtask/src/linux/container.rs",
            "pub fn exec_in() { let _ = \"PGG_IN_CONTAINER=1\"; }\n",
        ),
        (TEST, &census("")),
        (
            "crates/platitude-app/tests/qml/painted.js",
            ".pragma library\nfunction ink() { return 1 }\n",
        ),
        (
            "crates/platitude-app/tests/qml/stubs/GitFacts.qml",
            "pragma Singleton\nQtObject {}\n",
        ),
    ];
    for (path, text) in files {
        sb.write(&sb.seat, path, text);
    }
    let base = sb.commit_all(&sb.seat, "test: the QML runner", &[]);
    // Stamped once, so every later key is weighed against a green.
    sb.gate_ok(&sb.seat, &[]);
    let _ = sb.ran();
    (sb, base)
}

/// Qt's selection: the variable the offscreen fonts are found by, the
/// pin, and a test of its own that reads the seat module; `note` a
/// comment over it.
fn qt(fonts: &str, note: &str) -> String {
    format!(
        "{note}pub const PIN: &str = \".qt-version\";\npub fn path_with_qt() {{}}\n\
         pub fn offscreen_fonts() -> (&'static str, &'static str) {{ (\"QT_QPA_FONTDIR\", \"{fonts}\") }}\n\
         #[cfg(test)]\nmod tests {{\n    #[test]\n    fn t() {{ crate::seats::seat(); }}\n}}\n"
    )
}

/// The test file, `body` inside its case.
fn census(body: &str) -> String {
    format!(
        "import QtTest\nimport platitude.ui\nimport \"../../src/auto\" as Automation\n\
         import \"painted.js\" as Painted\nItem {{\n    Automation.Driver {{}}\n    \
         TestCase {{\n        name: \"Census\"\n{body}    }}\n}}\n"
    )
}

/// Each planned step's key, by id, off a dry run against main: on this
/// branch the QML tests are always planned.
fn keys(sb: &Sandbox) -> BTreeMap<String, String> {
    sb.gate_ok(&sb.seat, &["--dry-run"])
        .lines()
        .filter_map(|line| {
            let (row, key) = line.split_once("  key ")?;
            let (_, id) = row.split_once("] ")?;
            Some((id.to_string(), key.trim().to_string()))
        })
        .collect()
}

/// One change at each boundary, in turn: the file, its new text, the
/// steps that read it, and what it is to a run.
type Boundary = (&'static str, String, &'static [&'static str], &'static str);

fn boundaries() -> [Boundary; 11] {
    [
        (
            TEST,
            census("        function test_it() { compare(Painted.ink(), 1) }\n"),
            BOTH,
            "a test's body, which the runner hands over and reads for its cases",
        ),
        (
            "crates/platitude-app/src/ui/StashPane.qml",
            "Item {\n    property var model: StashModel\n    property int rows: 1\n}\n".into(),
            BOTH,
            "the product's QML, staged as the module the tests import",
        ),
        (
            "crates/platitude-app/src/auto/Driver.qml",
            "Item {\n    width: 1\n}\n".into(),
            BOTH,
            "the harness's QML, which a test loads by relative import",
        ),
        (
            "crates/platitude-app/tests/qml/stubs/GitFacts.qml",
            "pragma Singleton\nQtObject {\n    property bool ready: true\n}\n".into(),
            BOTH,
            "a stub, staged in place of the app's own module",
        ),
        (
            "crates/platitude-app/tests/qml/painted.js",
            ".pragma library\nfunction ink() { return 2 }\n".into(),
            BOTH,
            "a JS helper the tests import",
        ),
        (
            "crates/xtask/src/qmltest.rs",
            "use crate::qt;\nconst TESTS: &str = \"crates/platitude-app/tests/qml\";\n\
             const UI: &str = \"crates/platitude-app/src/ui\";\n\
             pub fn run() { let _ = (TESTS, UI, \"-platform\"); }\n"
                .into(),
            BOTH,
            "the runner's own code",
        ),
        (
            QT,
            qt("C:\\\\Fonts", ""),
            BOTH,
            "Qt's selection and the fonts' variable, one file both sides compile",
        ),
        (
            ".qt-version",
            "6.11.0\n".into(),
            BOTH,
            "the Qt pin the host's selection holds the Qt to and the image is built at",
        ),
        (
            ".cargo/config.toml",
            "[env]\nCFLAGS = \"-DPROBE\"\n".into(),
            BOTH,
            "cargo's [env], which the runner's run inherits",
        ),
        (
            "ci/linux/Dockerfile",
            "FROM ubuntu\nENV LANG=ja_JP.UTF-8\n".into(),
            &[LINUX],
            "the image's environment, the container's alone",
        ),
        (
            "crates/xtask/src/linux/container.rs",
            "pub fn exec_in() { let _ = \"PGG_IN_CONTAINER=2\"; }\n".into(),
            &[LINUX],
            "what starts the run in the container and hands it its environment",
        ),
    ]
}

#[test]
fn a_change_at_each_boundary_of_a_qml_test_run_owes_it_and_runs_it() {
    let (sb, mut base) = with_the_qml_runner("qmltest-boundaries");
    for (file, text, owed, why) in boundaries() {
        let before = keys(&sb);
        sb.write(&sb.seat, file, &text);
        let head = sb.commit_all(&sb.seat, &format!("feat: {file}"), &[]);
        let plan = sb.gate_ok(&sb.seat, &["--main", &base, "--dry-run"]);
        let after = keys(&sb);
        for id in BOTH {
            let row = format!("] {id}  key ");
            if owed.contains(id) {
                let planned = plan
                    .lines()
                    .any(|line| line.contains(&row) && line.trim_start().starts_with("run "));
                assert!(planned, "{why}: {id} is owed and unstamped\n{plan}");
                assert_ne!(before.get(*id), after.get(*id), "{why}: {id}'s key");
            } else {
                assert!(!plan.contains(&row), "{why}: {id} reads none of it\n{plan}");
                assert_eq!(before.get(*id), after.get(*id), "{why}: {id}'s key");
            }
        }
        sb.gate_ok(&sb.seat, &["--main", &base]);
        let ran = sb.ran();
        for id in BOTH {
            assert_eq!(
                ran.contains(*id),
                owed.contains(id),
                "{why}: {id}; ran {ran:?}"
            );
        }
        base = head;
    }
}

/// What no run reads plans no QML test by itself, and leaves the green
/// the branch's own reach still asks for: a file only the runner's tests
/// read, the product's Rust (a stub stands in for the app's module), and
/// the tests' prose. A comment in the runner's Rust is planned and keeps
/// the stamp; a comment in a test is part of the text the runner reads
/// for its cases, and runs both again.
#[test]
fn what_a_qml_test_run_does_not_read_keeps_its_green() {
    let (sb, _) = with_the_qml_runner("qmltest-unread");
    let unread: [(&str, String, &str); 4] = [
        (
            "crates/xtask/src/seats.rs",
            "pub fn seat() { let _ = 1; }\n".into(),
            "read by the runner's tests alone",
        ),
        (
            "crates/platitude-core/src/refs.rs",
            "pub fn refs() { let _ = 3; }\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n"
                .into(),
            "the product's Rust",
        ),
        (
            "crates/platitude-app/tests/qml/README.md",
            "# How they are run\n".into(),
            "the tests' prose",
        ),
        (
            QT,
            qt("C:\\\\Windows\\\\Fonts", "/// Where Qt is found.\n"),
            "a comment in the runner's Rust",
        ),
    ];
    for (file, text, why) in unread {
        let base = sb.head(&sb.seat);
        sb.write(&sb.seat, file, &text);
        sb.commit_all(&sb.seat, &format!("chore: {file}"), &[]);
        let plan = sb.gate_ok(&sb.seat, &["--main", &base, "--dry-run"]);
        let planned = plan.contains("] qmltest  key ");
        assert_eq!(planned, file == QT, "{why}\n{plan}");
        // The branch's reach owes both; the stamps answer for them.
        sb.gate_ok(&sb.seat, &[]);
        sb.ran().lacks(BOTH);
        let ledger = sb.ledger(&sb.seat);
        for id in BOTH {
            let row = ledger.iter().find(|(_, step, _)| step == id);
            assert!(
                row.is_some_and(|(_, _, outcome)| outcome == "cached"),
                "{why}: {id} {row:?}"
            );
        }
    }
    sb.write(
        &sb.seat,
        TEST,
        &census("        // The census, read from the window it takes.\n"),
    );
    sb.commit_all(&sb.seat, "docs(app-ui): the census", &[]);
    sb.gate_ok(&sb.seat, &[]);
    sb.ran().holds(BOTH);
}
