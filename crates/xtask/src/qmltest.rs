//! `cargo xtask qmltest` — the `tst_*.qml` tests (what only the item tree
//! can answer), run by Qt's own `qmltestrunner`.
//!
//! The import tree is staged: `import platitude.ui` resolves by directory
//! name and the product's directory is called `ui`, so no directory under
//! crates/ can serve as an import path. src/ui is copied whole under
//! `platitude/ui` with the shipped qmldir, so the runs read the app's
//! singleton declarations and whatever the tests' closure grows into.
//!
//! The runner's `-o` log is the only output that survives: its redirected
//! output on Windows comes back empty.

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

const TESTS: &str = "crates/platitude-app/tests/qml";
const UI: &str = "crates/platitude-app/src/ui";
const WORK: &str = "target/qmltest";
const MODULE: &str = "platitude/ui";
/// The app's Rust-backed module, which several `ui` components import
/// whether or not they reach into it (`AppCombo`). Only the running app
/// registers it, and a file whose import does not resolve is a type that
/// does not resolve — so it is staged empty, and a test reaching into it
/// fails on the name it wanted.
const APP_MODULE: &str = "platitude";
/// The stub's one type, named so a test reaching for it reads as the
/// mistake it is.
const APP_STUB_TYPE: &str = "NotTheApplication.qml";

/// The ceiling for one `qmltestrunner`, one file or all of them. QtTest
/// bounds `tryCompare`, but not `when: windowShown` on a window that never
/// comes up. One number for both roads: such a file waits forever on
/// either, and costs the ceiling twice — together, then alone, which is
/// the run that names it.
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
    // qmltestrunner sits in Qt's bin, which the harness shells' PATH may lack.
    let path = crate::qt::path_with_qt()?;
    let outcome = all_at_once(&root, &work, &import, &tests, &path);
    let verdict = match outcome {
        Ok(()) => {
            println!("qmltest: PASS ({} file(s), one process)", tests.len());
            Ok(())
        }
        Err(together) => one_at_a_time(&root, &work, &import, &tests, &path, &together),
    };
    let _ = std::fs::remove_dir_all(root.join(&work));
    verdict
}

/// Every file in one process — the ordinary road. A process is the price
/// (the test bodies cost next to nothing) on a step in the gate's serial
/// chain, so one process is what makes a new file here nearly free.
fn all_at_once(
    root: &Path,
    work: &str,
    import: &Path,
    tests: &[String],
    path: &OsString,
) -> Result<(), String> {
    let text = runner(root, TESTS, &format!("{work}/all.txt"), import, path)?;
    // A run that stopped part-way leaves the rest unmentioned, which a
    // count of failures reads as a pass ([`unfinished_in`]).
    short_of(root, tests, &text)
}

/// The cases the log does not answer for, as the words a red run comes
/// back with.
fn short_of(root: &Path, tests: &[String], text: &str) -> Result<(), String> {
    let missing: Vec<String> = tests
        .iter()
        .flat_map(|test| unfinished_in(text, test, root))
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "{} case(s) the run never finished: {}",
        missing.len(),
        missing.join(" / ")
    ))
}

/// One process per file, after a red together: one process cannot say
/// which file failed.
fn one_at_a_time(
    root: &Path,
    work: &str,
    import: &Path,
    tests: &[String],
    path: &OsString,
    together: &str,
) -> Result<(), String> {
    println!("qmltest: the one-process run was not green — running the files apart to say which");
    let mut failures = Vec::new();
    for test in tests {
        if let Err(why) = run_one(root, work, import, test, path) {
            println!("qmltest: FAIL {test}");
            println!("{why}");
            failures.push(test.clone());
        }
    }
    verdict(together, &failures)
}

/// What the two roads come to. The first run's red is the verdict either
/// way — the second is a diagnosis, not an appeal: a file that fails apart
/// is named, and files that pass apart but fail together are not
/// independent of each other, which is a red of its own.
fn verdict(together: &str, apart: &[String]) -> Result<(), String> {
    if !apart.is_empty() {
        return Err(format!("qmltest failed: {}", apart.join(" / ")));
    }
    Err(format!(
        "every file passes on its own and they do not pass together: one of them is leaving \
         something behind for the next. The run of them together said: {together}"
    ))
}

/// The cases of `test` the log does not hold finished, named. Every
/// `TestCase` a file declares is owed (a file may declare several), and
/// finished means its `cleanupTestCase` is logged: QtTest logs one pass or
/// fail, so an `initTestCase` alone is a process that went down inside the
/// case. A `skip()` is logged and closed like any other, so an absent case
/// is always a run that fell short.
fn unfinished_in(text: &str, test: &str, root: &Path) -> Vec<String> {
    let cases = case_names(&root.join(test));
    if cases.is_empty() {
        return vec![format!("{test} (no TestCase this could name)")];
    }
    cases
        .into_iter()
        .filter(|case| !text.contains(&format!("qmltestrunner::{case}::cleanupTestCase(")))
        .map(|case| format!("{test} :: {case}"))
        .collect()
}

/// The `TestCase` names one file declares, read off its source: the first
/// `name:` line after each `TestCase {`, at any indent, perhaps after an
/// `id:` (the attribute order of rules/structure.md §QML 整理). A second
/// `TestCase {` ends the search for the first one's.
fn case_names(file: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(file) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("TestCase {") {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("name:") {
            inside = false;
            if let Some(name) = rest
                .trim()
                .strip_prefix('"')
                .and_then(|n| n.strip_suffix('"'))
            {
                names.push(name.to_string());
            }
        }
    }
    names
}

/// This run's own staging directory, named after the process and
/// workspace-relative (that is how the log reaches `-o`). On a Linux host
/// `gate` / `check` run the host and container sides here at once, and a
/// shared directory has each rebuilding the tree the other resolves
/// through — a component "unavailable" for the length of one copy.
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
    // Rebuilt from scratch: a killed run's leftovers hold components since
    // renamed, which would resolve here and nowhere else.
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
    let app = import.join(APP_MODULE);
    std::fs::create_dir_all(&app).map_err(|e| format!("{}: {e}", app.display()))?;
    // One type: a module with none has no version for the unversioned
    // `import platitude` to resolve to, and reads as not installed.
    std::fs::write(app.join(APP_STUB_TYPE), "import QtQuick\n\nQtObject {}\n")
        .map_err(|e| format!("{APP_MODULE}/{APP_STUB_TYPE}: {e}"))?;
    std::fs::write(
        app.join("qmldir"),
        format!("module {APP_MODULE}\nNotTheApplication 1.0 {APP_STUB_TYPE}\n"),
    )
    .map_err(|e| format!("{APP_MODULE}/qmldir: {e}"))?;
    Ok(import)
}

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
    let text = runner(root, test, &format!("{work}/{stem}.txt"), import, path)?;
    // A file whose case never became true runs nothing and exits green.
    short_of(root, std::slice::from_ref(&test.to_string()), &text)?;
    let totals = text
        .lines()
        .find(|line| line.starts_with("Totals:"))
        .unwrap_or("no totals line");
    println!("qmltest: ok   {test} — {totals}");
    Ok(())
}

/// One `qmltestrunner` over `input` (a file, or the directory of them all)
/// under the ceiling, answering with its log. The verdict is the exit code
/// *and* the `FAIL!` lines: a logger that could not be written leaves a
/// green exit with nothing behind it.
fn runner(
    root: &Path,
    input: &str,
    log: &str,
    import: &Path,
    path: &OsString,
) -> Result<String, String> {
    let _ = std::fs::remove_file(root.join(log));
    // Relative paths from the workspace root: `-o <file>,txt` splits its
    // argument on a comma, and an absolute Windows path gives it more to
    // get wrong.
    let mut child = Command::new("qmltestrunner")
        .args(["-platform", "offscreen"])
        .args(["-input", input])
        .arg("-import")
        .arg(import)
        .args(["-o", &format!("{log},txt")])
        .current_dir(root)
        .env("PATH", path)
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to run qmltestrunner (it ships in Qt's bin): {e}"))?;
    let mut wait = Wait::new(
        format!("qmltestrunner on {input}"),
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
    let text = std::fs::read_to_string(root.join(log)).unwrap_or_default();
    let refused = text.lines().any(|line| line.starts_with("FAIL!"));
    if status.success() && !refused {
        return Ok(text);
    }
    Err(if text.is_empty() {
        format!("qmltestrunner wrote no log and exited {status} (log: {log})")
    } else {
        text
    })
}

#[cfg(test)]
mod tests {
    use super::{TESTS, case_names, short_of, tests_in, unfinished_in, verdict};
    use std::path::Path;

    /// The second run only names which file; where it names none, the red
    /// carries the first run's own words so a reader has them.
    #[test]
    fn a_red_together_stays_red_when_every_file_is_green_apart() {
        let together = "FAIL!  : qmltestrunner::Ink::test_a_mark_is_owed_once()";
        let why = verdict(together, &[]).expect_err("green apart is not a pass");
        assert!(why.contains("do not pass together"), "{why}");
        assert!(
            why.contains(together),
            "the first run's words are kept: {why}"
        );

        let named = verdict(together, &["tests/qml/tst_ink.qml".to_string()])
            .expect_err("a file of its own");
        assert!(named.contains("tst_ink.qml"), "{named}");
    }

    /// The count is the claim: a block this reader missed would be a case
    /// nothing is owed for, and the guard would pass a run that never
    /// reached it.
    #[test]
    fn every_test_case_in_the_tree_is_one_this_can_name() {
        let root = crate::tree::workspace_root();
        let tests = tests_in(&root).expect("the tst_*.qml files");
        assert!(tests.len() > 20, "found {} of them", tests.len());
        let mut several = 0;
        for test in &tests {
            let text = std::fs::read_to_string(root.join(test)).expect("the file");
            let blocks = text
                .lines()
                .filter(|line| line.trim().starts_with("TestCase {"))
                .count();
            let names = case_names(&root.join(test));
            assert_eq!(
                names.len(),
                blocks,
                "{test} declares {blocks} case(s) and this reader found {names:?}"
            );
            assert!(!names.is_empty(), "{test} declares none at all");
            if blocks > 1 {
                several += 1;
            }
        }
        assert!(
            several > 0,
            "a file with several cases is what the whole-file rule is for"
        );
    }

    /// Every case of a file is owed, and an opened case is not a finished
    /// one.
    #[test]
    fn a_case_left_part_way_is_read_as_one_the_run_never_finished() {
        let root = crate::tree::workspace_root();
        let four = format!("{TESTS}/tst_refstack.qml");
        let whole: String = case_names(&root.join(&four))
            .iter()
            .map(|case| format!("PASS   : qmltestrunner::{case}::cleanupTestCase()\n"))
            .collect();
        assert!(unfinished_in(&whole, &four, &root).is_empty(), "{whole}");

        // The same run with the last case opened and not closed.
        let part = whole.replace(
            "PASS   : qmltestrunner::RefChipReading::cleanupTestCase()\n",
            "PASS   : qmltestrunner::RefChipReading::initTestCase()\n",
        );
        assert_eq!(
            unfinished_in(&part, &four, &root),
            [format!("{four} :: RefChipReading")],
            "started is not finished"
        );
        assert_eq!(
            unfinished_in("", &four, &root).len(),
            4,
            "and a log that says nothing owes all four"
        );
    }

    #[test]
    fn a_run_short_of_its_cases_says_how_many_and_which() {
        let root = crate::tree::workspace_root();
        let tests = vec![format!("{TESTS}/tst_navfacts.qml")];
        let whole = "PASS   : qmltestrunner::NavFacts::cleanupTestCase()\n";
        assert!(short_of(&root, &tests, whole).is_ok());
        let why = short_of(&root, &tests, "").expect_err("nothing ran");
        assert!(
            why.contains("1 case(s) the run never finished") && why.contains("NavFacts"),
            "{why}"
        );
    }

    /// The files carry plenty of `name:` outside `TestCase` blocks (a
    /// row's own `name` role among them).
    #[test]
    fn a_name_that_is_not_a_test_cases_is_not_read_as_one() {
        let root = crate::tree::workspace_root();
        let names = case_names(&root.join(Path::new(TESTS)).join("tst_navrowlight.qml"));
        assert_eq!(names, ["NavRowLight"]);
    }
}
