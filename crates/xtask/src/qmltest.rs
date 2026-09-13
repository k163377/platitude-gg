//! `cargo xtask qmltest` — the QML tests, run by Qt's own runner.
//!
//! `tst_*.qml` under crates/platitude-app/tests/qml holds what only QML
//! can be asked: a Canvas that owes the screenshot a paint is a fact
//! about the item tree, and no Rust test reaches it. QtTest answers those,
//! and `qmltestrunner` is the binary Qt ships to run it.
//!
//! **The import tree is staged, not pointed at.** `import platitude.ui`
//! resolves by directory name, and the product's directory is called `ui`,
//! so no directory under crates/ can serve as an import path. The whole of
//! src/ui is copied under `platitude/ui` — with the shipped qmldir rather
//! than a subset written out here, so the runs read the same singleton
//! declarations the app does, and a component the closure grows into is
//! already standing there. (It grows: the test names `InkCanvas`, which
//! reads `Ink`, which reads `Metrics`.)
//!
//! **The runner's `-o` log is the only output that survives.** Redirected
//! on Windows its plain text comes back empty — a verdict with an exit
//! code and nothing behind it — so what a caller reads is written to a
//! file and printed from there.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::command::{self, Permission, Where};
use crate::wait::{Budget, LOOK_AGAIN, Wait};

pub(crate) static QMLTEST: command::Command = command::Command {
    id: "qmltest.run",
    call: "qmltest",
    purpose: "the QtTest files, run by Qt's own runner",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&QMLTEST];

/// The tests, the module they import, and where both ends are staged.
const TESTS: &str = "crates/platitude-app/tests/qml";
const UI: &str = "crates/platitude-app/src/ui";
const WORK: &str = "target/qmltest";
/// `import platitude.ui` looks for `<import path>/platitude/ui/qmldir`.
const MODULE: &str = "platitude/ui";

/// The ceiling for one file. QtTest bounds its own waits (`tryCompare`),
/// but `when: windowShown` on a window that never comes up is not one of
/// them, and an unbounded wait is a hang nothing can name.
const CEILING: Duration = Duration::from_secs(120);

pub fn run(args: &[String]) -> Result<(), String> {
    if let Some(unknown) = args.first() {
        return Err(format!("unknown option {unknown:?} (qmltest takes none)"));
    }
    let root = crate::tree::workspace_root();
    let tests = tests_in(&root)?;
    if tests.is_empty() {
        return Err(format!("no tst_*.qml under {TESTS}"));
    }
    let work = work_rel();
    let import = stage(&root, &root.join(&work))?;
    // qmltestrunner sits in Qt's bin beside the libraries it loads, and
    // the harness shells do not inherit the user's PATH edits.
    let path = crate::qt::path_with_qt()?;
    let mut failures = Vec::new();
    for test in &tests {
        if let Err(why) = run_one(&root, &work, &import, test, &path) {
            println!("qmltest: FAIL {test}");
            println!("{why}");
            failures.push(test.clone());
        }
    }
    let _ = std::fs::remove_dir_all(root.join(&work));
    if failures.is_empty() {
        println!("qmltest: PASS ({} file(s))", tests.len());
        return Ok(());
    }
    Err(format!("qmltest failed: {}", failures.join(" / ")))
}

/// This run's own staging directory, named after the process — as a
/// workspace-relative path, because that is how the log reaches `-o`.
///
/// Two runs are in flight at once whenever this machine is the Linux one:
/// `gate` and `check` both put the host side and the container side in
/// parallel, and on Linux the container side runs right here
/// (`crate::linux`). A directory shared between them would have each
/// rebuilding the tree the other is resolving through — a component
/// "unavailable" for the length of one copy, and green again on the
/// re-run that goes looking for it.
fn work_rel() -> String {
    format!("{WORK}/{}", std::process::id())
}

/// The `tst_*.qml` files, in a fixed order so two runs read alike.
fn tests_in(root: &Path) -> Result<Vec<String>, String> {
    let dir = root.join(TESTS);
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let name = entry.map_err(|e| e.to_string())?.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("tst_") && name.ends_with(".qml") {
            found.push(format!("{TESTS}/{name}"));
        }
    }
    found.sort();
    Ok(found)
}

/// Builds the import tree and answers with the directory to hand `-import`.
fn stage(root: &Path, work: &Path) -> Result<PathBuf, String> {
    let import = work.join("import");
    let module = import.join(MODULE);
    // Rebuilt rather than topped up: what a killed run left behind holds
    // components since renamed, and they would resolve here and nowhere
    // else.
    if module.exists() {
        std::fs::remove_dir_all(&module).map_err(|e| format!("{}: {e}", module.display()))?;
    }
    std::fs::create_dir_all(&module).map_err(|e| format!("{}: {e}", module.display()))?;
    let ui = root.join(UI);
    for entry in std::fs::read_dir(&ui).map_err(|e| format!("{}: {e}", ui.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if name != "qmldir" && !name.ends_with(".qml") {
            continue;
        }
        std::fs::copy(&path, module.join(&name)).map_err(|e| format!("{name}: {e}"))?;
    }
    if !module.join("qmldir").is_file() {
        return Err(format!("{UI}/qmldir is gone — nothing declares the module"));
    }
    Ok(import)
}

/// One file: the runner under a ceiling, then its log. The verdict is the
/// exit code *and* the `FAIL!` lines — a logger that could not be written
/// would otherwise leave a green exit with nothing behind it.
fn run_one(
    root: &Path,
    work: &str,
    import: &Path,
    test: &str,
    path: &OsString,
) -> Result<(), String> {
    let stem = Path::new(test)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let log = format!("{work}/{stem}.txt");
    let _ = std::fs::remove_file(root.join(&log));
    // Relative paths, run from the workspace root: `-o <file>,txt` splits
    // its argument on a comma, and an absolute Windows path is one more
    // thing for that to be wrong about.
    let mut child = Command::new("qmltestrunner")
        .args(["-platform", "offscreen"])
        .args(["-input", test])
        .arg("-import")
        .arg(import)
        .args(["-o", &format!("{log},txt")])
        .current_dir(root)
        .env("PATH", path)
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to run qmltestrunner (it ships in Qt's bin): {e}"))?;
    let mut wait = Wait::new(
        format!("qmltestrunner on {test}"),
        Budget::whole(CEILING),
        LOOK_AGAIN,
    );
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if let Err(expired) = wait.look_again("its exit") {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "killed at the ceiling — {expired} (a TestCase whose `when` never came true \
                 waits forever)"
            ));
        }
    };
    let text = std::fs::read_to_string(root.join(&log)).unwrap_or_default();
    let refused = text.lines().any(|line| line.starts_with("FAIL!"));
    if status.success() && !refused {
        let totals = text
            .lines()
            .find(|line| line.starts_with("Totals:"))
            .unwrap_or("no totals line");
        println!("qmltest: ok   {test} — {totals}");
        return Ok(());
    }
    Err(if text.is_empty() {
        format!("qmltestrunner wrote no log and exited {status} (log: {log})")
    } else {
        text
    })
}
