//! The run itself: which files are read as what, what each is held to,
//! and the verdict.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::source::{self, Purpose};
use super::{Exception, Finding, qml, rust};

/// The suites' common budgets, where a clock is read on purpose. Named by
/// the tail of the path: a whole path in a string here would be an edge to
/// that file in the gate's graph.
const BUDGETS: [&str; 2] = ["/tests/it/support/wait.rs", "/src/wait.rs"];

/// This scanner's own directory, by the same tail: its tests are
/// fixtures of the very shapes it looks for.
const SELF: &str = "/src/waits/";

/// The directory of a crate's `src` that holds the app's automation
/// harness, read as test code beside the window it drives.
const HARNESS: &str = "auto";

/// The Rust half of that same harness (the `PGG_*` record, the deadline
/// thread, the drivers' clock). It is behind the same feature as
/// [`HARNESS`] and ships in no window, so its body is read the way this
/// runner's own is.
const HARNESS_RUST: &str = "harness";

/// How much was read: said beside the verdict, so that a PASS names the
/// range it holds for.
#[derive(Default)]
struct Read {
    rust_files: usize,
    qml_files: usize,
    /// Counted apart from the QtTest files, so that neither going to zero
    /// hides behind the other.
    harness_qml_files: usize,
    /// Rust statements read as test code.
    test_statements: usize,
    /// The files of this runner's own crate whose tool bodies were read,
    /// and the statements of those bodies.
    tool_files: usize,
    tool_statements: usize,
    /// The same for the harness's Rust half ([`HARNESS_RUST`]), counted
    /// apart so that neither group's guard stands in for the other's.
    harness_rust_files: usize,
    harness_rust_statements: usize,
}

pub fn run(args: &[String]) -> Result<(), String> {
    if let Some(unknown) = args.first() {
        return Err(format!("unknown option {unknown:?} (waits takes none)"));
    }
    let root = crate::tree::workspace_root();
    let runner = runner_prefix(&root)?;
    let mut files = Vec::new();
    collect(&root.join("crates"), &mut files)?;
    files.sort();

    let mut sources = Vec::new();
    for file in &files {
        let relative = file
            .strip_prefix(&root)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        let Some(lang) = language_of(&relative) else {
            continue;
        };
        let text = std::fs::read_to_string(file).map_err(|e| format!("{relative}: {e}"))?;
        sources.push((relative, lang, text));
    }
    let declared = declared_test_files(&sources);

    let mut findings = Vec::new();
    let mut exceptions = Vec::new();
    let mut read = Read::default();
    for (relative, lang, text) in &sources {
        match lang {
            source::Lang::Rust => {
                read.rust_files += 1;
                let scope = scope_of(relative, &declared, &runner);
                let scanned = rust::scan(relative, text, scope);
                if scope == rust::Scope::Tool {
                    read.tool_files += 1;
                    read.tool_statements += scanned.body_statements;
                } else if scope == rust::Scope::Harness {
                    read.harness_rust_files += 1;
                    read.harness_rust_statements += scanned.body_statements;
                }
                read.test_statements += scanned.test_statements;
                findings.extend(scanned.findings);
                exceptions.extend(scanned.exceptions);
            }
            source::Lang::Qml => {
                read.qml_files += 1;
                let kind = qml_kind(relative);
                if kind == qml::Kind::Harness {
                    read.harness_qml_files += 1;
                }
                let (found, allowed) = qml::scan(relative, text, kind);
                findings.extend(found);
                exceptions.extend(allowed);
            }
        }
    }
    // Otherwise the body rules name nothing and still say PASS.
    if read.tool_files == 0 {
        return Err(format!(
            "none of the files read is this runner's own ({runner}/src) — its tool bodies went \
             unread"
        ));
    }
    // The same guard for each half of the harness.
    for (what, dir, found) in [
        ("the beats it spells", HARNESS, read.harness_qml_files),
        ("the clocks it keeps", HARNESS_RUST, read.harness_rust_files),
    ] {
        if found == 0 {
            return Err(format!(
                "none of the files read is an app harness (a crate's src/{dir}) — {what} went \
                 unread"
            ));
        }
    }
    report(&findings, &exceptions, &read)
}

/// The files another file declares as test modules (`#[cfg(test)] mod
/// x;`): test code from top to bottom, whatever they are called.
fn declared_test_files(sources: &[(String, source::Lang, String)]) -> BTreeSet<String> {
    let mut declared = BTreeSet::new();
    for (relative, lang, text) in sources {
        if *lang != source::Lang::Rust {
            continue;
        }
        let code = source::code_view(text, source::Lang::Rust);
        for module in source::declared_test_modules(&code, text) {
            declared.extend(module_files(relative, &module));
        }
    }
    declared
}

/// Where a module declared in `declaring` puts its file: the one its
/// `#[path]` names, relative to the declaring file's own directory — or,
/// by its name alone, beside a `mod.rs` / `lib.rs` / `main.rs` and under
/// a directory of the file's own stem otherwise, as `name.rs` or as
/// `name/mod.rs`.
fn module_files(declaring: &str, module: &source::Declared) -> Vec<String> {
    let (dir, file) = declaring.rsplit_once('/').unwrap_or(("", declaring));
    match module {
        source::Declared::Path(path) => vec![joined(dir, path)],
        source::Declared::Named(name) => {
            let stem = file.strip_suffix(".rs").unwrap_or(file);
            let base = if matches!(stem, "mod" | "lib" | "main") {
                dir.to_string()
            } else {
                format!("{dir}/{stem}")
            };
            vec![format!("{base}/{name}.rs"), format!("{base}/{name}/mod.rs")]
        }
    }
}

/// `path`, as the tree spells the file it names from `dir`: separators
/// one way, and `.` and `..` walked off.
fn joined(dir: &str, path: &str) -> String {
    let path = path.replace('\\', "/");
    let mut parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

fn report(findings: &[Finding], exceptions: &[Exception], read: &Read) -> Result<(), String> {
    for finding in findings {
        println!(
            "waits: {}:{}: [{}] {}",
            finding.file, finding.line, finding.rule, finding.excerpt
        );
    }
    // Named one by one, so that a reviewer sees the list grow.
    for exception in exceptions {
        println!(
            "waits: {}:{}: stands as {}",
            exception.file,
            exception.line,
            exception.purpose.name()
        );
    }
    let counted = Purpose::ALL
        .iter()
        .map(|p| {
            let n = exceptions.iter().filter(|e| e.purpose == *p).count();
            format!("{} {n}", p.name())
        })
        .collect::<Vec<_>>()
        .join(", ");
    if findings.is_empty() {
        println!(
            "waits: {} Rust and {} QML files read ({} of them the app's harness) — {} \
             statement(s) of test code, and the bodies of {} file(s) of this runner ({} \
             statement(s)) and {} of the harness's Rust half ({} statement(s)); {} statement(s) \
             standing on a marker ({counted}) — PASS",
            read.rust_files,
            read.qml_files,
            read.harness_qml_files,
            read.test_statements,
            read.tool_files,
            read.tool_statements,
            read.harness_rust_files,
            read.harness_rust_statements,
            exceptions.len()
        );
        return Ok(());
    }
    Err(format!(
        "{} wait(s) that spend the clock or read no answer: end a wait on the answer \
         (a completion handle, an event, a hand-driven poll), take the budget from the suite \
         (`bounded`) — or, in this runner's own body, the wait from `crate::wait` (`Wait`, \
         `receive`, `stood`) — read what a wait answered, or say on the line why this one \
         stands (`// waits(<purpose>): <reason>`, purposes paced / ceiling / measured / timed)",
        findings.len()
    ))
}

/// Which scanner reads a file, or none. The budgets' own implementation
/// and this scanner's fixtures are none.
fn language_of(relative: &str) -> Option<source::Lang> {
    if BUDGETS.iter().any(|tail| relative.ends_with(tail)) || relative.contains(SELF) {
        return None;
    }
    let mut parts = relative.split('/');
    let (Some("crates"), Some(_crate), Some(kind)) = (parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    let name = relative.rsplit('/').next().unwrap_or_default();
    if name.ends_with(".rs") && matches!(kind, "src" | "tests") {
        return Some(source::Lang::Rust);
    }
    if !name.ends_with(".qml") {
        return None;
    }
    if kind == "tests" && name.starts_with("tst_") {
        return Some(source::Lang::Qml);
    }
    // Every file of the harness directory: reading only some would say
    // PASS over the rest.
    if kind == "src" && parts.next() == Some(HARNESS) {
        return Some(source::Lang::Qml);
    }
    None
}

fn qml_kind(relative: &str) -> qml::Kind {
    if in_src_dir(relative, HARNESS) {
        qml::Kind::Harness
    } else {
        qml::Kind::Test
    }
}

/// How much of a Rust file is read ([`rust::Scope`]); `runner` is
/// [`runner_prefix`]'s.
fn scope_of(relative: &str, declared: &BTreeSet<String>, runner: &str) -> rust::Scope {
    if whole_test_file(relative) || declared.contains(relative) {
        rust::Scope::Whole
    } else if relative.starts_with(&format!("{runner}/src/")) {
        rust::Scope::Tool
    } else if in_src_dir(relative, HARNESS_RUST) {
        rust::Scope::Harness
    } else {
        rust::Scope::Tests
    }
}

/// Whether `relative` stands in `dir` of some crate's `src` — a
/// `tests/<dir>` is a test file and not this.
fn in_src_dir(relative: &str, dir: &str) -> bool {
    let mut parts = relative.split('/').skip(2);
    (parts.next(), parts.next()) == (Some("src"), Some(dir))
}

/// This runner's own crate relative to `root`, from the manifest it is
/// compiled from, so that no path is spelled here.
fn runner_prefix(root: &Path) -> Result<String, String> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let relative = manifest.strip_prefix(root).map_err(|_| {
        format!(
            "this runner is compiled from {}, outside the tree it reads ({})",
            manifest.display(),
            root.display()
        )
    })?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

/// Whether a Rust file is test code from top to bottom by where it stands
/// or what it is called (structure.md §分割). A test file under another
/// name is known by the `#[cfg(test)] mod` that declares it
/// ([`declared_test_files`]).
fn whole_test_file(relative: &str) -> bool {
    let name = relative.rsplit('/').next().unwrap_or_default();
    relative.split('/').nth(2) == Some("tests") || name == "tests.rs" || name.ends_with("_tests.rs")
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            collect(&path, out)?;
        } else if path
            .extension()
            .is_some_and(|ext| ext == "rs" || ext == "qml")
        {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        language_of, module_files, qml, qml_kind, runner_prefix,
        rust::Scope,
        scope_of,
        source::{Declared, Lang},
        whole_test_file,
    };

    fn named(declaring: &str, name: &str) -> Vec<String> {
        module_files(declaring, &Declared::Named(name.to_string()))
    }

    // The paths below are under a crate that does not exist: the gate's
    // graph reads a real path in a string as an edge to that file, and
    // refuses a crate root outright.

    #[test]
    fn a_declared_module_resolves_beside_its_declaring_file() {
        assert_eq!(
            named("crates/x/src/graph/mod.rs", "testkit")[0],
            "crates/x/src/graph/testkit.rs"
        );
        assert_eq!(
            named("crates/x/src/offers.rs", "tests")[0],
            "crates/x/src/offers/tests.rs"
        );
        assert_eq!(
            named("crates/x/src/lib.rs", "wait"),
            [
                "crates/x/src/wait.rs".to_string(),
                "crates/x/src/wait/mod.rs".to_string()
            ]
        );
    }

    #[test]
    fn a_modules_own_path_stands_from_the_declaring_files_directory() {
        let path = |declaring: &str, path: &str| {
            module_files(declaring, &Declared::Path(path.to_string()))
        };
        assert_eq!(
            path("crates/x/src/process/executor.rs", "tests.rs"),
            ["crates/x/src/process/tests.rs".to_string()],
            "beside the declaring file"
        );
        assert_eq!(
            path("crates/x/src/avatar.rs", "avatar_tests.rs"),
            ["crates/x/src/avatar_tests.rs".to_string()]
        );
        assert_eq!(
            path("crates/x/tests/gate/main.rs", "../../src/wait.rs"),
            ["crates/x/src/wait.rs".to_string()],
            "`..` walks the declaring file's directory off"
        );
        assert_eq!(
            path("crates/x/src/lib.rs", ".\\sub\\kit.rs"),
            ["crates/x/src/sub/kit.rs".to_string()],
            "a separator of the other kind names the same file"
        );
    }

    #[test]
    fn every_crates_test_code_is_read_and_the_budgets_are_not() {
        assert_eq!(
            language_of("crates/x/tests/it/session_integration/auto_fetch.rs"),
            Some(Lang::Rust)
        );
        assert_eq!(language_of("crates/x/src/lanes.rs"), Some(Lang::Rust));
        assert_eq!(
            language_of("crates/x/tests/qml/tst_ink.qml"),
            Some(Lang::Qml)
        );
        assert_eq!(language_of("crates/x/tests/it/support/wait.rs"), None);
        assert_eq!(language_of("crates/x/src/wait.rs"), None);
        assert_eq!(language_of("crates/x/src/waits/rust.rs"), None);
        assert_eq!(language_of("crates/x/src/ui/Main.qml"), None);
        assert_eq!(language_of("internal-docs/x.rs"), None);
    }

    #[test]
    fn the_harness_is_read_whatever_its_files_are_called_and_the_window_is_not() {
        assert_eq!(
            language_of("crates/x/src/auto/WindowDialogActs.qml"),
            Some(Lang::Qml),
            "the driver is a file per verb group, and every one of them is read"
        );
        assert_eq!(
            language_of("crates/x/src/ui/SampleTimer.qml"),
            None,
            "the cadence stands in the window beside the harness, which is the product"
        );
        assert_eq!(language_of("crates/x/src/auto/qmldir"), None);
        assert_eq!(
            language_of("crates/x/src/auto/driver.rs"),
            Some(Lang::Rust),
            "a Rust file there is read for its test blocks like any other"
        );
    }

    #[test]
    fn a_qml_file_is_judged_by_the_rules_of_where_it_stands() {
        assert_eq!(
            qml_kind("crates/x/src/auto/WindowCensus.qml"),
            qml::Kind::Harness
        );
        assert_eq!(qml_kind("crates/x/tests/qml/tst_ink.qml"), qml::Kind::Test);
        assert_eq!(
            qml_kind("crates/x/tests/auto/tst_ink.qml"),
            qml::Kind::Test,
            "a directory of that name under tests is still a QtTest file"
        );
    }

    #[test]
    fn a_whole_test_file_is_named_by_where_it_stands_or_what_it_is_called() {
        assert!(whole_test_file("crates/x/tests/gate/support.rs"));
        assert!(whole_test_file("crates/x/src/session/state_tests.rs"));
        assert!(whole_test_file("crates/x/src/corpus/stream/tests.rs"));
        assert!(!whole_test_file("crates/x/src/session/read_flight.rs"));
    }

    #[test]
    fn a_test_file_is_read_whole_a_products_source_for_its_tests_and_the_runners_for_its_body() {
        let declared = BTreeSet::from(["crates/x/src/graph/testkit.rs".to_string()]);
        let runner = "crates/x";
        assert_eq!(
            scope_of("crates/x/tests/gate/support.rs", &declared, runner),
            Scope::Whole
        );
        assert_eq!(
            scope_of("crates/x/src/graph/testkit.rs", &declared, runner),
            Scope::Whole,
            "a declared test file is test code and nothing else"
        );
        assert_eq!(
            scope_of("crates/x/src/corpus/tests.rs", &declared, runner),
            Scope::Whole
        );
        assert_eq!(
            scope_of("crates/x/src/lanes.rs", &declared, runner),
            Scope::Tool,
            "the runner's source is read for its body too"
        );
        assert_eq!(
            scope_of("crates/y/src/lanes.rs", &declared, runner),
            Scope::Tests,
            "a product crate's source is read for its tests alone"
        );
        assert_eq!(
            scope_of("crates/y/src/harness/deadline.rs", &declared, runner),
            Scope::Harness,
            "the harness's Rust half is a body, in whichever crate carries it"
        );
        assert_eq!(
            scope_of("crates/y/tests/harness/support.rs", &declared, runner),
            Scope::Whole,
            "a `tests/harness` of somebody's is a test file and not the app's"
        );
    }

    #[test]
    fn the_runner_is_named_relative_to_the_tree_it_reads() {
        let root = crate::tree::workspace_root();
        let prefix = runner_prefix(&root).expect("compiled inside the tree");
        assert!(!prefix.is_empty() && !prefix.contains('\\'), "{prefix}");
        assert!(
            root.join(&prefix).is_dir(),
            "{prefix} is a crate of the tree"
        );
        assert!(runner_prefix(&root.join("elsewhere")).is_err());
    }
}
