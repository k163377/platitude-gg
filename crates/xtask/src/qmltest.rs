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
//!
//! A run that is not green keeps its staging directory — the import tree
//! as staged, every runner's log, and a manifest of what the runners were
//! handed ([`MANIFEST`]) — under [`FAILED`], where only this module's sweep
//! takes it away ([`sweep`]).

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::command::{self, Permission, Where};
use crate::wait::{Budget, LOOK_AGAIN, Wait};

#[cfg(test)]
mod tests;

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
/// does not resolve — so it is staged all but empty, and a test reaching
/// into it fails on the name it wanted. The exception is [`STUBS`].
const APP_MODULE: &str = "platitude";
/// The stub's one type, named so a test reaching for it reads as the
/// mistake it is.
const APP_STUB_TYPE: &str = "NotTheApplication.qml";
/// Stand-ins for the module's singletons, one file each, staged into the
/// module: what the row delegates read as they are built (`GitFacts` —
/// argument-only facts, whose rules Rust tests), and what the harness's
/// machinery reads (`Harness` — the run's name, and the lines it says,
/// kept for a test to read back; `PerfProbe` — a clock). Nothing of the
/// models.
const STUBS: &str = "crates/platitude-app/tests/qml/stubs";

/// How long one `qmltestrunner`, one file or all of them, may write
/// nothing. QtTest bounds `tryCompare`, but not `when: windowShown` on a
/// window that never comes up, and such a file goes quiet. One number for
/// both roads: it waits forever on either, and costs this twice —
/// together, then alone, which is the run that names it. Silence rather
/// than length, because the run of them all grows with every file; the
/// log is flushed line by line (`QAbstractTestLogger::outputString`), so a
/// run getting on renews it.
const QUIET: Duration = Duration::from_secs(120);

/// The whole of one run: the backstop for one that keeps writing and never
/// ends. Under the gate step's own silence ceiling
/// (`watched::QUIET_CEILING`), which hears nothing from this command until
/// a run is over, so this fires first and names the runner.
const WHOLE: Duration = Duration::from_secs(15 * 60);

/// Where a run that was not green leaves its staging directory, one per
/// run, named `<second it started>-<pid>` so that the names sort by age.
const FAILED: &str = "target/qmltest/failed";
/// How many of those stand: the newest. The one-process log of a red run
/// carries every later file's compile errors, so it is the large part,
/// and a few runs are what a person compares.
const KEPT: usize = 5;
/// A staging directory with no run behind it — what a run killed before
/// its end leaves — is taken once it is this old: no run stands this long.
const LITTER_AGE: Duration = Duration::from_secs(24 * 60 * 60);
/// What a kept directory says about its run ([`manifest`]).
const MANIFEST: &str = "manifest.txt";
/// The one-process run's log, inside the staging directory.
const TOGETHER_LOG: &str = "all.txt";

/// The variables a Qt run reads that a manifest lists when inherited:
/// name prefixes, compared in upper case. Every variable set on the
/// runner's command is listed whatever its name.
const READ_BY_QT: [&str; 9] = [
    "QT_",
    "QML",
    "QV4_",
    "QSG_",
    "QTEST",
    "QQUICK",
    "FONTCONFIG",
    "LD_LIBRARY_PATH",
    "DYLD_",
];

/// What every `qmltestrunner` of one run is handed.
struct Setup<'a> {
    root: &'a Path,
    /// The staging directory, workspace-relative ([`work_rel`]).
    work: &'a str,
    import: &'a Path,
    /// PATH as composed for the runner ([`crate::qt::path_with_qt`]).
    path: &'a OsString,
    /// The runner, resolved on `path` ([`runner_on`]).
    exe: &'a Path,
}

/// One `qmltestrunner` as it came out, for the manifest.
struct Ran {
    input: String,
    /// The log's file name inside the staging directory.
    log: String,
    /// The exit status, or why there is none.
    ended: String,
}

pub fn run(args: &[String]) -> Result<(), String> {
    if let Some(unknown) = args.iter().find(|arg| arg.starts_with('-')) {
        return Err(format!(
            "unknown option {unknown:?} (qmltest takes file names alone)"
        ));
    }
    let root = crate::tree::workspace_root();
    let every = tests_in(&root)?;
    if every.is_empty() {
        return Err(format!("no tst_*.qml under {TESTS}"));
    }
    let tests = chosen(&every, args)?;
    let started = SystemTime::now();
    let work = work_rel();
    let import = stage(&root, &root.join(&work))?;
    // qmltestrunner sits in Qt's bin, which the harness shells' PATH may lack.
    let path = crate::qt::path_with_qt()?;
    let exe = runner_on(&path);
    let setup = Setup {
        root: &root,
        work: &work,
        import: &import,
        path: &path,
        exe: &exe,
    };
    let mut runs = Vec::new();
    // Named files run apart, each answering for itself — the road for one file while it is being written. The
    // gate names none.
    let verdict = if args.is_empty() {
        match all_at_once(&setup, &tests, &mut runs) {
            Ok(()) => {
                println!("qmltest: PASS ({} file(s), one process)", tests.len());
                Ok(())
            }
            Err(together) => one_at_a_time(&setup, &tests, &together, &mut runs),
        }
    } else {
        tests
            .iter()
            .try_for_each(|test| run_one(&setup, test, &mut runs))
    };
    let verdict = match verdict {
        Ok(()) => {
            let _ = std::fs::remove_dir_all(root.join(&work));
            Ok(())
        }
        Err(why) => Err(with_evidence(
            &setup,
            &tests,
            args.is_empty(),
            &runs,
            &run_name(started),
            why,
        )),
    };
    sweep(&root);
    verdict
}

/// Every file in one process — the ordinary road. A process is the price
/// (the test bodies cost next to nothing) on a step in the gate's serial
/// chain, so one process is what makes a new file here nearly free.
fn all_at_once(setup: &Setup, tests: &[String], runs: &mut Vec<Ran>) -> Result<(), String> {
    let text = runner(setup, TESTS, TOGETHER_LOG, runs)?;
    // A run that stopped part-way leaves the rest unmentioned, which a
    // count of failures reads as a pass ([`unfinished_in`]).
    short_of(setup.root, tests, &text)
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
    setup: &Setup,
    tests: &[String],
    together: &str,
    runs: &mut Vec<Ran>,
) -> Result<(), String> {
    println!("qmltest: the one-process run was not green — running the files apart to say which");
    let mut failures = Vec::new();
    for test in tests {
        if let Err(why) = run_one(setup, test, runs) {
            println!("qmltest: FAIL {test}");
            println!("{why}");
            failures.push(test.clone());
        }
    }
    // The log on disk, not `together`: a killed run's words are the kill.
    let log =
        std::fs::read_to_string(setup.root.join(setup.work).join(TOGETHER_LOG)).unwrap_or_default();
    verdict(together, &failures, &outcomes(setup.root, tests, &log))
}

/// What the two roads come to. The first run's red is the verdict either
/// way — the second is a diagnosis, not an appeal: a file that fails apart
/// is named, and files that pass apart but fail together are not
/// independent of each other, which is a red of its own. That is all the
/// two roads establish: where the one-process run first broke
/// ([`first_break`] over `together_files`), not why.
fn verdict(
    together: &str,
    apart: &[String],
    together_files: &[(String, Came)],
) -> Result<(), String> {
    if !apart.is_empty() {
        return Err(format!("qmltest failed: {}", apart.join(" / ")));
    }
    let place = match first_break(together_files) {
        Some((Some(before), first, Came::Red(why))) => {
            format!("{before} finished green, then {first} was the first to fail: {why}")
        }
        Some((None, first, Came::Red(why))) => {
            format!("{first}, the first file, failed: {why}")
        }
        Some((Some(before), first, _)) => {
            format!("{before} finished green, and the log names nothing from {first} on")
        }
        Some((None, _, _)) | None => "the log names no file at all".to_string(),
    };
    Err(format!(
        "every file passes in a process of its own and they do not pass together in one. In the \
         one-process run, {place}. That is what this establishes; the cause it does not \
         determine. The candidates it cannot tell apart: state a file leaves in the process for \
         the files after it; Qt's process-wide type registration and plugin bookkeeping, which \
         the engine each file gets shares with every other; a resource shared with whatever \
         else was running. The run of them together said: {together}"
    ))
}

/// What a log says of one file.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Came {
    /// Every case it declares finished, and none failed.
    Green,
    /// The first line that says why not.
    Red(String),
    /// The log never names it.
    Unmentioned,
    /// It had no run of its own: an earlier named file failed first.
    NotRun,
}

impl Came {
    fn said(&self) -> String {
        match self {
            Came::Green => "green".to_string(),
            Came::Red(why) => format!("red    {why}"),
            Came::Unmentioned => "never named in the log".to_string(),
            Came::NotRun => "not run".to_string(),
        }
    }
}

/// What `text` — one runner's log — says of each of `tests`, in the order
/// the log first names them; the files it never names follow, in their
/// own order. A file is named by its cases (`case_names`) and, for a file
/// that did not compile, by its stem (`tst_x::compile()`). The cases are
/// told apart by name, which no two files share
/// (`no_two_files_declare_one_case`).
fn outcomes(root: &Path, tests: &[String], text: &str) -> Vec<(String, Came)> {
    let mut whose: HashMap<String, usize> = HashMap::new();
    let mut cases: Vec<Vec<String>> = Vec::new();
    for (at, test) in tests.iter().enumerate() {
        let declared = case_names(&root.join(test));
        for case in &declared {
            whose.insert(case.clone(), at);
        }
        if let Some(stem) = Path::new(test).file_stem() {
            whose.insert(stem.to_string_lossy().into_owned(), at);
        }
        cases.push(declared);
    }
    let mut first: Vec<Option<usize>> = vec![None; tests.len()];
    let mut failed: Vec<Option<&str>> = vec![None; tests.len()];
    let mut finished: Vec<&str> = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let Some((case, function)) = spoken_for(line) else {
            continue;
        };
        let Some(&at) = whose.get(case) else {
            continue;
        };
        first[at].get_or_insert(n);
        if line.starts_with("FAIL!") {
            failed[at].get_or_insert(line);
        }
        if function.starts_with("cleanupTestCase(") {
            finished.push(case);
        }
    }
    let mut said: Vec<(usize, String, Came)> = tests
        .iter()
        .enumerate()
        .map(|(at, test)| {
            let came = match (first[at], failed[at]) {
                (None, _) => Came::Unmentioned,
                (Some(_), Some(line)) => Came::Red(line.to_string()),
                (Some(_), None) => match cases[at].iter().find(|c| !finished.contains(&c.as_str()))
                {
                    Some(case) => Came::Red(format!("{case} never finished")),
                    None => Came::Green,
                },
            };
            (first[at].unwrap_or(usize::MAX), test.clone(), came)
        })
        .collect();
    said.sort_by_key(|(at, _, _)| *at);
    said.into_iter()
        .map(|(_, test, came)| (test, came))
        .collect()
}

/// The case and the function a QtTest log line speaks for:
/// `<TAG> : qmltestrunner::<case>::<function>`.
fn spoken_for(line: &str) -> Option<(&str, &str)> {
    let (tag, rest) = line.split_once(" : qmltestrunner::")?;
    if tag.is_empty()
        || !tag
            .trim_end()
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '!')
    {
        return None;
    }
    rest.split_once("::")
}

/// The first file that did not come out green, the file just before it
/// (green, then, if there is one), and what the log says of it.
fn first_break(files: &[(String, Came)]) -> Option<(Option<&str>, &str, &Came)> {
    let at = files.iter().position(|(_, came)| *came != Came::Green)?;
    let before = at.checked_sub(1).map(|b| files[b].0.as_str());
    Some((before, files[at].0.as_str(), &files[at].1))
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
/// `gate` runs the host and container sides here at once, and a
/// shared directory has each rebuilding the tree the other resolves
/// through — a component "unavailable" for the length of one copy.
fn work_rel() -> String {
    format!("{WORK}/{}", std::process::id())
}

/// The name a kept run stands under in [`FAILED`]: the second it started,
/// then the process.
fn run_name(started: SystemTime) -> String {
    let secs = started
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    format!("{secs}-{}", std::process::id())
}

/// The files `args` name out of `every` — by stem (`tst_ink`), file name or
/// path — or all of them for no args. A name that matches none is a
/// mistake, not an empty run.
fn chosen(every: &[String], args: &[String]) -> Result<Vec<String>, String> {
    if args.is_empty() {
        return Ok(every.to_vec());
    }
    let mut picked = Vec::new();
    for arg in args {
        let want = arg.replace('\\', "/");
        let stem = want.trim_end_matches(".qml");
        let found = every
            .iter()
            .find(|test| test == &&want || test.ends_with(&format!("/{stem}.qml")))
            .ok_or_else(|| format!("no test called {arg:?} under {TESTS}"))?;
        if !picked.contains(found) {
            picked.push(found.clone());
        }
    }
    Ok(picked)
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
    let mut qmldir = format!("module {APP_MODULE}\nNotTheApplication 1.0 {APP_STUB_TYPE}\n");
    for stub in stubs_in(root)? {
        let name = stub
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        std::fs::copy(&stub, app.join(&name)).map_err(|e| format!("{name}: {e}"))?;
        let stem = name.trim_end_matches(".qml");
        qmldir.push_str(&format!("singleton {stem} 1.0 {name}\n"));
    }
    std::fs::write(app.join("qmldir"), qmldir).map_err(|e| format!("{APP_MODULE}/qmldir: {e}"))?;
    Ok(import)
}

/// The stand-in singletons under [`STUBS`], in a fixed order.
fn stubs_in(root: &Path) -> Result<Vec<PathBuf>, String> {
    let dir = root.join(STUBS);
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|e| e == "qml") {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}

fn run_one(setup: &Setup, test: &str, runs: &mut Vec<Ran>) -> Result<(), String> {
    let text = runner(setup, test, &log_of(test), runs)?;
    // A file whose case never became true runs nothing and exits green.
    short_of(setup.root, std::slice::from_ref(&test.to_string()), &text)?;
    let totals = text
        .lines()
        .find(|line| line.starts_with("Totals:"))
        .unwrap_or("no totals line");
    println!("qmltest: ok   {test} — {totals}");
    Ok(())
}

/// The log of `test`'s run of its own, inside the staging directory.
fn log_of(test: &str) -> String {
    let stem = Path::new(test)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    format!("{stem}.txt")
}

/// The runner the composed PATH resolves to, started by that path so a
/// manifest names the program that ran. Bare when the PATH holds none,
/// and the start says so.
fn runner_on(path: &OsStr) -> PathBuf {
    let name = format!("qmltestrunner{}", std::env::consts::EXE_SUFFIX);
    std::env::split_paths(path)
        .map(|dir| dir.join(&name))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| PathBuf::from("qmltestrunner"))
}

/// What every runner gets on top of what it inherits: the composed PATH
/// and the offscreen fonts. The manifest reads it back off a command
/// built here too.
fn given(cmd: &mut Command, path: &OsString) {
    cmd.env("PATH", path);
    // The fonts the app's pictures are drawn in: a test that sets a mark
    // against letters reads the face the shots show.
    crate::qt::offscreen_fonts(cmd);
}

/// One `qmltestrunner` over `input` (a file, or the directory of them all)
/// under [`QUIET`] and [`WHOLE`], writing `log` in the staging directory and
/// answering with it. The verdict is the exit code *and* the `FAIL!` lines:
/// a logger that could not be written leaves a green exit with nothing
/// behind it. Whatever it comes to goes into `runs`.
fn runner(setup: &Setup, input: &str, log: &str, runs: &mut Vec<Ran>) -> Result<String, String> {
    let at = format!("{}/{log}", setup.work);
    let _ = std::fs::remove_file(setup.root.join(&at));
    // Relative paths from the workspace root: `-o <file>,txt` splits its
    // argument on a comma, and an absolute Windows path gives it more to
    // get wrong.
    let mut cmd = Command::new(setup.exe);
    cmd.args(["-platform", "offscreen"])
        .args(["-input", input])
        .arg("-import")
        .arg(setup.import)
        .args(["-o", &format!("{at},txt")])
        .current_dir(setup.root)
        .stdin(Stdio::null());
    given(&mut cmd, setup.path);
    let mut ran = Ran {
        input: input.to_string(),
        log: log.to_string(),
        ended: String::new(),
    };
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            ran.ended = format!("never started: {e}");
            runs.push(ran);
            return Err(format!(
                "failed to run {} (it ships in Qt's bin): {e}",
                setup.exe.display()
            ));
        }
    };
    let mut wait = Wait::new(
        format!("qmltestrunner on {input}"),
        Budget::of(QUIET, WHOLE),
        LOOK_AGAIN,
    );
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        let len = std::fs::metadata(setup.root.join(&at)).map_or(0, |m| m.len());
        wait.saw(format!("{len} bytes of log"));
        if let Err(expired) = wait.look_again("its exit") {
            let _ = child.kill();
            let _ = child.wait();
            ran.ended = format!("killed — {expired}");
            runs.push(ran);
            return Err(format!(
                "killed — {expired} (a TestCase whose `when` never came true waits forever)"
            ));
        }
    };
    ran.ended = status.to_string();
    runs.push(ran);
    let text = std::fs::read_to_string(setup.root.join(&at)).unwrap_or_default();
    let refused = text.lines().any(|line| line.starts_with("FAIL!"));
    if status.success() && !refused {
        return Ok(text);
    }
    Err(if text.is_empty() {
        format!("qmltestrunner wrote no log and exited {status} (log: {at})")
    } else {
        text
    })
}

/// Keeps a red run's staging directory under [`FAILED`] with its
/// manifest, and answers `why` with where it went.
fn with_evidence(
    setup: &Setup,
    tests: &[String],
    together: bool,
    runs: &[Ran],
    name: &str,
    why: String,
) -> String {
    match keep(setup, tests, together, runs, name) {
        Ok(dir) => {
            let shown = dir
                .strip_prefix(setup.root)
                .unwrap_or(&dir)
                .display()
                .to_string()
                .replace('\\', "/");
            println!("qmltest: evidence kept at {shown}");
            format!(
                "{why}\nevidence: {shown} ({MANIFEST}, every runner's log, the import tree as \
                 staged)"
            )
        }
        Err(e) => format!("{why}\n(the evidence could not be kept: {e})"),
    }
}

/// Writes the manifest into the staging directory and moves the directory
/// under [`FAILED`], answering where it stands. A directory that will not
/// move (a file in it still held open) stands where it ran, with the
/// manifest in it, until [`LITTER_AGE`] takes it.
fn keep(
    setup: &Setup,
    tests: &[String],
    together: bool,
    runs: &[Ran],
    name: &str,
) -> Result<PathBuf, String> {
    let work = setup.root.join(setup.work);
    let said = manifest(&gather(setup, tests, together, runs, name));
    std::fs::write(work.join(MANIFEST), said)
        .map_err(|e| format!("{}: {e}", work.join(MANIFEST).display()))?;
    let failed = setup.root.join(FAILED);
    std::fs::create_dir_all(&failed).map_err(|e| format!("{}: {e}", failed.display()))?;
    let dir = failed.join(name);
    Ok(match std::fs::rename(&work, &dir) {
        Ok(()) => dir,
        Err(e) => {
            println!("qmltest: the evidence stays where it ran, not under {FAILED}: {e}");
            work
        }
    })
}

/// What a red run's manifest says, gathered ([`gather`]) apart from how
/// it is written ([`manifest`]).
struct Manifest {
    run: String,
    /// The runner's path, or why there is none.
    runner: String,
    /// The Qt this tree chose ([`crate::qt::chosen`]), or why there is none.
    qt: String,
    /// A log's `Config:` line: the Qt the runner itself reports.
    config: Option<String>,
    import: String,
    /// The import and plugin paths a log reports ([`reported_paths`]).
    reported: Option<(Vec<String>, Vec<String>)>,
    /// One line per variable ([`environment`]).
    env: Vec<String>,
    files: Vec<(String, Came)>,
    /// One line per runner, in the order they ran.
    runs: Vec<String>,
}

fn gather(setup: &Setup, tests: &[String], together: bool, runs: &[Ran], run: &str) -> Manifest {
    let work = setup.root.join(setup.work);
    let logs: Vec<String> = runs
        .iter()
        .map(|ran| std::fs::read_to_string(work.join(&ran.log)).unwrap_or_default())
        .collect();
    let files = if together {
        let all = runs
            .iter()
            .position(|ran| ran.log == TOGETHER_LOG)
            .map_or("", |at| logs[at].as_str());
        outcomes(setup.root, tests, all)
    } else {
        tests
            .iter()
            .enumerate()
            .flat_map(|(at, test)| match logs.get(at) {
                Some(text) => outcomes(setup.root, std::slice::from_ref(test), text),
                None => vec![(test.clone(), Came::NotRun)],
            })
            .collect()
    };
    let mut probe = Command::new(setup.exe);
    given(&mut probe, setup.path);
    let set = probe
        .get_envs()
        .map(|(name, value)| {
            (
                name.to_string_lossy().into_owned(),
                value.map(|v| v.to_string_lossy().into_owned()),
            )
        })
        .collect();
    let inherited = std::env::vars_os()
        .map(|(name, value)| {
            (
                name.to_string_lossy().into_owned(),
                value.to_string_lossy().into_owned(),
            )
        })
        .collect();
    Manifest {
        run: run.to_string(),
        runner: if setup.exe.is_absolute() {
            setup.exe.display().to_string()
        } else {
            format!(
                "{} (not on the PATH below; started by name)",
                setup.exe.display()
            )
        },
        qt: match crate::qt::chosen() {
            Ok(Some(qt)) => qt.describe(),
            Ok(None) => "no Qt found".to_string(),
            Err(why) => why,
        },
        config: logs.iter().find_map(|text| {
            text.lines()
                .find(|line| line.starts_with("Config:"))
                .map(str::to_string)
        }),
        import: setup.import.display().to_string(),
        reported: logs.iter().find_map(|text| reported_paths(text)),
        env: environment(inherited, set),
        files,
        runs: runs
            .iter()
            .map(|ran| format!("{} -> {}: {}", ran.input, ran.log, ran.ended))
            .collect(),
    }
}

/// The manifest's text.
fn manifest(m: &Manifest) -> String {
    let mut out = format!("qmltest run {} — not green\n\n", m.run);
    out.push_str(&format!("runner     {}\n", m.runner));
    out.push_str(&format!("qt         {}\n", m.qt));
    out.push_str(&format!(
        "qt (log)   {}\n",
        m.config.as_deref().unwrap_or("no log has a Config: line")
    ));
    out.push_str(&format!(
        "-import    {} (the tree as staged is import/ beside this file)\n",
        m.import
    ));
    match &m.reported {
        Some((imports, plugins)) => {
            out.push_str("\npaths the runner reported (it lists them beside a compile error):\n");
            for path in imports {
                out.push_str(&format!("  import  {path}\n"));
            }
            for path in plugins {
                out.push_str(&format!("  plugin  {path}\n"));
            }
        }
        None => out.push_str(
            "\npaths the runner reported: none (it lists them only beside a compile error)\n",
        ),
    }
    out.push_str(
        "\nenvironment the runner was given (every variable set on its command, and those a Qt \
         reads that it inherited):\n",
    );
    for line in &m.env {
        out.push_str(&format!("  {line}\n"));
    }
    out.push_str("\nfiles, in the order the log first names them:\n");
    for (file, came) in &m.files {
        out.push_str(&format!("  {file}  {}\n", came.said()));
    }
    out.push_str("\nrunners, in the order they ran (input -> log beside this file: exit):\n");
    for run in &m.runs {
        out.push_str(&format!("  {run}\n"));
    }
    out
}

/// The variables a runner sees that bear on what it loads, as lines: every
/// one `set` on its command (a `None` is one taken away), and those
/// `inherited` whose names a Qt reads ([`READ_BY_QT`]) and the command does
/// not set. Names compare as the platform compares them.
fn environment(
    inherited: Vec<(String, String)>,
    set: Vec<(String, Option<String>)>,
) -> Vec<String> {
    let same = |a: &str, b: &str| {
        if cfg!(windows) {
            a.eq_ignore_ascii_case(b)
        } else {
            a == b
        }
    };
    let mut lines: Vec<String> = inherited
        .into_iter()
        .filter(|(name, _)| {
            let upper = name.to_ascii_uppercase();
            READ_BY_QT.iter().any(|prefix| upper.starts_with(prefix))
        })
        .filter(|(name, _)| !set.iter().any(|(s, _)| same(s, name)))
        .map(|(name, value)| format!("{name}={value}  (inherited)"))
        .collect();
    lines.extend(set.into_iter().map(|(name, value)| match value {
        Some(value) => format!("{name}={value}  (set here)"),
        None => format!("{name}  (taken away here)"),
    }));
    lines.sort();
    lines
}

/// The import and plugin paths a runner log reports. Qt's runner lists them
/// only beside a compile error (`handleCompileErrors`, quicktest.cpp): an
/// `Import paths:` line, one quoted path per line, a `Plugin paths:` line,
/// and its paths. The first such list in the log.
fn reported_paths(text: &str) -> Option<(Vec<String>, Vec<String>)> {
    let quoted = |line: &str| {
        line.trim()
            .strip_prefix('\'')
            .and_then(|path| path.strip_suffix('\''))
            .map(str::to_string)
    };
    let mut lines = text.lines();
    lines.find(|line| line.trim_end().ends_with("Import paths:"))?;
    let mut imports = Vec::new();
    let mut next = lines.next();
    while let Some(path) = next.and_then(quoted) {
        imports.push(path);
        next = lines.next();
    }
    let mut plugins = Vec::new();
    if next.map(str::trim) == Some("Plugin paths:") {
        plugins.extend(lines.map_while(quoted));
    }
    Some((imports, plugins))
}

/// Keeps the newest [`KEPT`] runs under [`FAILED`], and takes a staging
/// directory no run finished with once it is [`LITTER_AGE`] old. Silent: a
/// directory that will not go is the next sweep's.
fn sweep(root: &Path) {
    let failed = root.join(FAILED);
    if let Ok(entries) = std::fs::read_dir(&failed) {
        let names = entries
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        for old in beyond_the_newest(names, KEPT) {
            let _ = std::fs::remove_dir_all(failed.join(old));
        }
    }
    let work = root.join(WORK);
    let now = SystemTime::now();
    if let Ok(entries) = std::fs::read_dir(&work) {
        let aged: Vec<(String, Option<Duration>)> = entries
            .flatten()
            .map(|entry| {
                let since = entry
                    .metadata()
                    .and_then(|meta| meta.modified())
                    .ok()
                    .and_then(|written| now.duration_since(written).ok());
                (entry.file_name().to_string_lossy().into_owned(), since)
            })
            .collect();
        for old in litter(&aged, LITTER_AGE) {
            let _ = std::fs::remove_dir_all(work.join(old));
        }
    }
}

/// The runs past the newest `keep`, oldest first. A run's name starts with
/// the second it started, so names sort by age (`gate::evidence` keeps its
/// red logs by the same rule).
fn beyond_the_newest(mut names: Vec<String>, keep: usize) -> Vec<String> {
    names.sort();
    names.truncate(names.len().saturating_sub(keep));
    names
}

/// The staging directories — a process id for a name — last written `age`
/// or longer ago. One whose age cannot be read stays.
fn litter(entries: &[(String, Option<Duration>)], age: Duration) -> Vec<String> {
    entries
        .iter()
        .filter(|(name, since)| {
            !name.is_empty()
                && name.bytes().all(|b| b.is_ascii_digit())
                && since.is_some_and(|since| since >= age)
        })
        .map(|(name, _)| name.clone())
        .collect()
}
