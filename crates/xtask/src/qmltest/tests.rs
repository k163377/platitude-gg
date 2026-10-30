//! `qmltest`'s own tests, in a file of their own (structure.md §分割).

use super::{
    Came, FAILED, KEPT, LITTER_AGE, MANIFEST, Manifest, Ran, Setup, TESTS, beyond_the_newest,
    case_names, environment, keep, litter, manifest, outcomes, reported_paths, run_each, short_of,
    sweep, tests_in, unfinished_in, work_base,
};
use crate::yard::Yard;
use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

/// A red run's staging directory goes under [`FAILED`] whole, its log
/// beside a manifest that says how its runner ended; of the kept runs,
/// the sweep leaves the newest.
#[test]
fn a_red_run_is_kept_with_its_manifest_and_only_the_newest_stand() {
    let root = Yard::new("qmltest-kept");
    let work = "target/qmltest/4242";
    let staged = root.join(work);
    std::fs::create_dir_all(staged.join("import").join("platitude")).expect("a staged tree");
    let log = "Config: Using QtTest library 6.12.0\n\
               FAIL!  : qmltestrunner::tst_x::compile() Cannot install element 'X'\n";
    std::fs::write(staged.join("tst_x.txt"), log).expect("the runner's log");
    let import = staged.join("import");
    let path = OsString::new();
    let setup = Setup {
        root: &root,
        work,
        import: &import,
        path: &path,
        exe: Path::new("qmltestrunner"),
    };
    let runs = [Ran {
        input: format!("{TESTS}/tst_x.qml"),
        log: "tst_x.txt".to_string(),
        ended: "exit code: 1".to_string(),
    }];
    let tests = [format!("{TESTS}/tst_x.qml")];
    let dir = keep(&setup, &tests, true, &runs, "1793102832-4242").expect("kept");
    let group = root.join(work_base()).join(FAILED).join("suite");
    assert_eq!(dir, group.join("1793102832-4242"));
    assert!(!staged.exists(), "moved, not copied");
    assert_eq!(
        std::fs::read_to_string(dir.join("tst_x.txt")).expect("the log"),
        log
    );
    assert!(dir.join("import").join("platitude").is_dir(), "the tree");
    let said = std::fs::read_to_string(dir.join(MANIFEST)).expect("the manifest");
    for part in [
        "qt (log)   Config: Using QtTest library 6.12.0",
        &format!("{TESTS}/tst_x.qml -> tst_x.txt: exit code: 1"),
        &format!("{TESTS}/tst_x.qml  red    FAIL!  : qmltestrunner::tst_x::compile()"),
        "(not on the PATH below; started by name)",
    ] {
        assert!(said.contains(part), "{part:?} is not in:\n{said}");
    }

    for older in 0..KEPT {
        std::fs::create_dir_all(group.join(format!("17931028{older:02}-1")))
            .expect("an older kept run");
    }
    let selected = root.join(work_base()).join(FAILED).join("selected");
    for newer in 0..KEPT + 2 {
        std::fs::create_dir_all(selected.join(format!("17932028{newer:02}-1")))
            .expect("a newer selected run");
    }
    sweep(&root);
    assert_eq!(std::fs::read_dir(selected).unwrap().count(), KEPT);
    let mut left: Vec<String> = std::fs::read_dir(group)
        .expect("the kept runs")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    left.sort();
    assert_eq!(left.len(), KEPT, "{left:?}");
    assert_eq!(left[0], "1793102801-1", "the oldest went: {left:?}");
    assert!(left.contains(&"1793102832-4242".to_string()), "{left:?}");
}

#[test]
fn every_file_runs_once_and_all_failures_remain_red() {
    let tests: Vec<String> = ["a", "b", "c"].map(String::from).into();
    let mut visited = Vec::new();
    let why = run_each(&tests, |test| {
        visited.push(test.to_string());
        if test == "b" {
            Ok(())
        } else {
            Err(format!("failure of {test}"))
        }
    })
    .unwrap_err();
    assert_eq!(visited, tests);
    assert!(why.contains("a: failure of a"), "{why}");
    assert!(why.contains("c: failure of c"), "{why}");
    assert!(run_each(&tests, |_| Ok(())).is_ok());
}

/// A log as the one-process run writes it: two files finish, the third
/// does not compile, and the fourth is never reached.
#[test]
fn a_log_says_each_file_in_the_order_it_names_them() {
    let root = crate::tree::workspace_root();
    let tests = vec![
        format!("{TESTS}/tst_graphrowwalk.qml"),
        format!("{TESTS}/tst_graphhover.qml"),
        format!("{TESTS}/tst_handwatch.qml"),
        format!("{TESTS}/tst_navfacts.qml"),
    ];
    let [walk, hover, hand] = ["GraphRowWalk", "GraphHover", "HandWatch"];
    for (test, case) in tests.iter().zip([walk, hover, hand]) {
        assert_eq!(
            case_names(&root.join(test)),
            [case],
            "the case these lines speak for"
        );
    }
    let log = format!(
        "********* Start testing of qmltestrunner *********\n\
         Config: Using QtTest library 6.12.0, Qt 6.12.0\n\
         PASS   : qmltestrunner::{hover}::initTestCase()\n\
         PASS   : qmltestrunner::{hover}::cleanupTestCase()\n\
         PASS   : qmltestrunner::{hand}::initTestCase()\n\
         QDEBUG : qmltestrunner::{hand}::test_x() said: qmltestrunner::{walk}::y\n\
         PASS   : qmltestrunner::{hand}::cleanupTestCase()\n\
         QWARN  : qmltestrunner::tst_graphrowwalk::compile() \n\
         \x20   tst_graphrowwalk.qml: Cannot install element 'X' into protected module 'QtQuick' version '6'\n\
         FAIL!  : qmltestrunner::tst_graphrowwalk::compile() Cannot install element 'X' into protected module 'QtQuick' version '6'\n\
         Totals: 4 passed, 1 failed, 0 skipped, 0 blacklisted, 5ms\n"
    );
    let said = outcomes(&root, &tests, &log);
    assert_eq!(
        said,
        vec![
            (tests[1].clone(), Came::Green),
            (tests[2].clone(), Came::Green),
            (
                tests[0].clone(),
                Came::Red(
                    "FAIL!  : qmltestrunner::tst_graphrowwalk::compile() Cannot install \
                     element 'X' into protected module 'QtQuick' version '6'"
                        .to_string()
                )
            ),
            (tests[3].clone(), Came::Unmentioned),
        ],
        "{log}"
    );
    // A case opened and not closed is a file that fell short.
    let cut = log.replace(
        &format!("PASS   : qmltestrunner::{hand}::cleanupTestCase()\n"),
        "",
    );
    assert_eq!(
        outcomes(&root, &tests[2..3], &cut),
        vec![(
            tests[2].clone(),
            Came::Red(format!("{hand} never finished"))
        )]
    );
}

/// [`outcomes`] tells files apart by their cases' names.
#[test]
fn no_two_files_declare_one_case() {
    let root = crate::tree::workspace_root();
    let mut seen = std::collections::HashMap::new();
    for test in tests_in(&root).expect("the tst_*.qml files") {
        for case in case_names(&root.join(&test)) {
            if let Some(other) = seen.insert(case.clone(), test.clone()) {
                panic!("{case} is declared by {other} and by {test}");
            }
        }
    }
}

#[test]
fn the_paths_a_compile_error_lists_are_read_back() {
    let log = "QWARN  : qmltestrunner::tst_x::compile() \n\
               \x20 Working directory: C:\\w\n\
               \x20 View: QQuickView, Import paths:\n\
               \x20   'C:\\w\\target\\qmltest\\1\\import'\n\
               \x20   'qrc:\\qt-project.org\\imports'\n\
               \x20 Plugin paths:\n\
               \x20   'C:\\Qt\\plugins'\n\
               \x20   '.'\n\
               \n\
               FAIL!  : qmltestrunner::tst_x::compile() no\n\
               \x20 Import paths:\n\
               \x20   'a later list'\n";
    let (imports, plugins) = reported_paths(log).expect("a list");
    assert_eq!(
        imports,
        [
            "C:\\w\\target\\qmltest\\1\\import",
            "qrc:\\qt-project.org\\imports"
        ]
    );
    assert_eq!(plugins, ["C:\\Qt\\plugins", "."]);
    assert_eq!(reported_paths("PASS   : qmltestrunner::A::b()\n"), None);
}

/// What the command sets wins over what it inherits; of the rest, only
/// what a Qt reads is listed.
#[test]
fn the_environment_listed_is_what_the_runner_was_given() {
    let inherited = vec![
        ("PATH".to_string(), "/usr/bin".to_string()),
        ("QT_QPA_FONTDIR".to_string(), "/elsewhere".to_string()),
        ("QT_QPA_PLATFORM".to_string(), "windows".to_string()),
        ("QML_IMPORT_TRACE".to_string(), "1".to_string()),
        ("HOME".to_string(), "/home/someone".to_string()),
    ];
    let set = vec![
        ("PATH".to_string(), Some("/qt/bin:/usr/bin".to_string())),
        ("QT_QPA_FONTDIR".to_string(), Some("/fonts".to_string())),
        ("QT_SCALE_FACTOR".to_string(), None),
    ];
    assert_eq!(
        environment(inherited, set),
        vec![
            "PATH=/qt/bin:/usr/bin  (set here)".to_string(),
            "QML_IMPORT_TRACE=1  (inherited)".to_string(),
            "QT_QPA_FONTDIR=/fonts  (set here)".to_string(),
            "QT_QPA_PLATFORM=windows  (inherited)".to_string(),
            "QT_SCALE_FACTOR  (taken away here)".to_string(),
        ]
    );
}

#[test]
fn a_manifest_says_what_ran_with_what_and_how_it_ended() {
    let m = Manifest {
        run: "1793102832-28260".to_string(),
        runner: "C:\\Qt\\6.12.0\\msvc2022_64\\bin\\qmltestrunner.exe".to_string(),
        qt: "Qt 6.12.0 at C:/Qt/6.12.0/msvc2022_64".to_string(),
        config: Some("Config: Using QtTest library 6.12.0, Qt 6.12.0".to_string()),
        import: "C:\\w\\target\\qmltest\\28260\\import".to_string(),
        reported: Some((vec!["qrc:\\qt\\qml".to_string()], vec![".".to_string()])),
        env: vec!["QT_QPA_FONTDIR=C:\\Windows\\Fonts  (set here)".to_string()],
        files: vec![
            ("tst_a.qml".to_string(), Came::Green),
            ("tst_b.qml".to_string(), Came::Red("FAIL!  : b".to_string())),
            ("tst_c.qml".to_string(), Came::NotRun),
        ],
        runs: vec!["crates/x/tests/qml -> all.txt: exit code: 67".to_string()],
    };
    let said = manifest(&m);
    for part in [
        "qmltest run 1793102832-28260",
        "runner     C:\\Qt\\6.12.0\\msvc2022_64\\bin\\qmltestrunner.exe",
        "qt         Qt 6.12.0 at C:/Qt/6.12.0/msvc2022_64",
        "qt (log)   Config: Using QtTest library 6.12.0",
        "-import    C:\\w\\target\\qmltest\\28260\\import",
        "  import  qrc:\\qt\\qml\n  plugin  .\n",
        "  QT_QPA_FONTDIR=C:\\Windows\\Fonts  (set here)\n",
        "  tst_a.qml  green\n  tst_b.qml  red    FAIL!  : b\n  tst_c.qml  not run\n",
        "  crates/x/tests/qml -> all.txt: exit code: 67\n",
    ] {
        assert!(said.contains(part), "{part:?} is not in:\n{said}");
    }
    let bare = manifest(&Manifest {
        config: None,
        reported: None,
        ..m
    });
    assert!(bare.contains("no log has a Config: line"), "{bare}");
    assert!(bare.contains("paths the runner reported: none"), "{bare}");
}

/// The newest runs stand, by the second their names start with.
#[test]
fn only_the_newest_kept_runs_stand() {
    let names: Vec<String> = [
        "1793102900-4",
        "1793102800-1",
        "1793102850-3",
        "1793102820-2",
    ]
    .map(str::to_string)
    .to_vec();
    assert_eq!(
        beyond_the_newest(names.clone(), 2),
        ["1793102800-1", "1793102820-2"]
    );
    assert!(beyond_the_newest(names, 4).is_empty());
}

/// A staging directory goes once it is old; the kept runs' directory,
/// a fresh run's, and one whose age cannot be read stay.
#[test]
fn only_old_staging_directories_are_litter() {
    let old = Some(LITTER_AGE + Duration::from_secs(1));
    let fresh = Some(Duration::from_secs(60));
    let entries = vec![
        ("28260".to_string(), old),
        ("31337".to_string(), fresh),
        ("4242".to_string(), None),
        ("failed".to_string(), old),
    ];
    assert_eq!(litter(&entries, LITTER_AGE), ["28260"]);
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
