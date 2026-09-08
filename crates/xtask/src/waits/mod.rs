//! `cargo xtask waits` — no test in this workspace takes a stretch of
//! clock for an answer, and no verb of this runner takes a wait from
//! anywhere but `crate::wait`.
//!
//! What it reads is every statement of test code — the integration
//! suites with their support modules, the `#[cfg(test)]` blocks (a
//! `mod`, a `fn` or an `impl`) and `*_tests.rs` files of every crate,
//! the QtTest files, and the app's own automation harness — and what it
//! names is four shapes (.claude/rules/core.md §非同期・並行テスト):
//!
//! - `sleep`: a sleep or a `yield_now` spent in place of an answer. A
//!   scheduler's turn proves no order and a stretch of quiet proves no
//!   silence; both only pass for as long as the machine has spare time.
//! - `deadline`: a clock read, or a `timeout` given a budget of its own.
//!   A deadline is a diagnostic and belongs to the suite's one budget,
//!   so that no test is handed a head start on noticing a hang.
//! - `ignored`: a wait whose answer is thrown away. A rendering wait that
//!   answered `false` is a paint that never happened.
//! - `naked`: a tracked completion (`outcome()`), a session boundary
//!   (`wait_for_graph_passes`, `wait_for_snapshot_reads`) or a
//!   hand-stepped tick awaited with no backstop under it. These resolve
//!   through channels no `Patience` watches, and a naked one hung a real
//!   run until the CI kill with no failing test named.
//!
//! Of this runner's own crate it reads the tool bodies too — every
//! statement outside the test blocks — and names there what
//! `crate::wait` is the one place for (.claude/rules-refs/core.md: xtask
//! の待ちは `crate::wait` 1 本から取る): a `sleep`, in the code or in a
//! PowerShell script the code writes, and a `deadline` — a read of the
//! monotonic clock, or a receive under a budget of its own. A wait that
//! goes through that module (`Wait`, `receive`, `stood`, its paces)
//! names none of these. The product crates' bodies are not read: the
//! product's clocks are the product's business.
//!
//! The one part of a product crate that is read is the app's harness
//! ([`HARNESS`]) — the driver `PG_AUTO_ACT` runs the product through,
//! which is test code that happens to ship inside the window. It runs on
//! beats rather than on QtTest calls, so it is named by rules of its own
//! ([`qml::Kind::Harness`]): every span of time it spells for itself,
//! and every count of milliseconds it keeps. The window beside it
//! (`src/ui`) is the product and stays unread.
//!
//! What a test — or a verb — may still do it says on the line:
//! `// waits(<purpose>): <reason>` above or beside the statement, with a
//! purpose of `paced` (a sleep spacing a retry whose completion is
//! causal), `ceiling` (a wall-clock ceiling that only names a failure),
//! `measured` (a clock read handed to the code under test or printed,
//! judged by nothing) or `timed` (a real-time property of the product,
//! judged as a bound no load can break). A marker that covers nothing is
//! a finding of its own, so a rewritten wait sheds its marker.

mod qml;
mod rust;
mod source;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use source::Purpose;

/// A statement that breaks a rule.
#[derive(Debug)]
struct Finding {
    file: String,
    line: usize,
    rule: &'static str,
    /// The line of the statement, as the file has it.
    excerpt: String,
}

/// A statement a marker lets stand, by the purpose it names.
#[derive(Debug)]
struct Exception {
    file: String,
    line: usize,
    purpose: Purpose,
}

/// A statement that breaks a rule before its markers are read.
struct Candidate {
    first: usize,
    last: usize,
    /// The line it is named at and quoted from: its first, or the line
    /// of a script's sleep inside it.
    shown: usize,
    rule: &'static str,
}

/// The code that implements the suites' common budgets: what every other
/// clock read is measured against, and the one place a clock is read on
/// purpose. Named by the tail of the path — a whole path in a string
/// here would be an edge from this file to that one in the gate's graph,
/// and this file reads neither.
const BUDGETS: [&str; 2] = ["/tests/it/support/wait.rs", "/src/wait.rs"];

/// This scanner's own directory, by the same tail: its tests are
/// fixtures of the very shapes it looks for.
const SELF: &str = "/src/waits/";

/// The directory of a crate's `src` that holds the app's automation
/// harness, read as test code beside the window it drives.
const HARNESS: &str = "auto";

/// How much was read: said beside the verdict, so that a PASS names the
/// range it holds for.
#[derive(Default)]
struct Read {
    rust_files: usize,
    qml_files: usize,
    /// How many of the QML files were the app's harness rather than
    /// QtTest files: the two are read by different rules, and a count of
    /// them together would hide either going to zero.
    harness_files: usize,
    /// Rust statements read as test code.
    test_statements: usize,
    /// The files of this runner's own crate whose tool bodies were read,
    /// and the statements of those bodies.
    tool_files: usize,
    tool_statements: usize,
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
                if scope == rust::Scope::Tool {
                    read.tool_files += 1;
                }
                let scanned = rust::scan(relative, text, scope);
                read.test_statements += scanned.test_statements;
                read.tool_statements += scanned.tool_statements;
                findings.extend(scanned.findings);
                exceptions.extend(scanned.exceptions);
            }
            source::Lang::Qml => {
                read.qml_files += 1;
                let kind = qml_kind(relative);
                if kind == qml::Kind::Harness {
                    read.harness_files += 1;
                }
                let (found, allowed) = qml::scan(relative, text, kind);
                findings.extend(found);
                exceptions.extend(allowed);
            }
        }
    }
    // A runner whose own files were not among those read has a rule
    // that names nothing, and would say PASS over it.
    if read.tool_files == 0 {
        return Err(format!(
            "none of the files read is this runner's own ({runner}/src) — its tool bodies went \
             unread"
        ));
    }
    // The same guard for the harness: its rules are the only ones that
    // read a beat, so a harness directory that fell out of the scope
    // above would leave every clock in the window unnamed and still say
    // PASS.
    if read.harness_files == 0 {
        return Err(format!(
            "none of the files read is an app harness (a crate's src/{HARNESS}) — the beats it \
             spells went unread"
        ));
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
    // Named one by one: what stands on a marker is what a reviewer reads
    // the reasons of, and a list that only grows is the thing to notice.
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
             statement(s) of test code, and the tool bodies of {} file(s) of this runner ({} \
             statement(s)); {} statement(s) standing on a marker ({counted}) — PASS",
            read.rust_files,
            read.qml_files,
            read.harness_files,
            read.test_statements,
            read.tool_files,
            read.tool_statements,
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

/// Which scanner reads a file, or none: Rust under a crate's `src` or
/// `tests`, QtTest files under `tests/qml`, the app's harness under a
/// crate's `src/auto` ([`HARNESS`]), and never the budgets' own
/// implementation or this scanner's fixtures.
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
    // Every file of the harness directory, not a name inside it: the
    // driver is spread over a file per verb group, and a rule that read
    // only some of them would say PASS over the rest.
    if kind == "src" && parts.next() == Some(HARNESS) {
        return Some(source::Lang::Qml);
    }
    None
}

/// Which QML rules a file is read by: the harness's where it stands in
/// a crate's `src/auto`, and QtTest's everywhere else a QML file is
/// read. Decided from the same two segments [`language_of`] admitted it
/// on, so that a `tests/auto` of somebody's is still a QtTest file.
fn qml_kind(relative: &str) -> qml::Kind {
    let mut parts = relative.split('/').skip(2);
    if (parts.next(), parts.next()) == (Some("src"), Some(HARNESS)) {
        qml::Kind::Harness
    } else {
        qml::Kind::Test
    }
}

/// How much of a Rust file is read ([`rust::Scope`]): the whole of a
/// test file, the test blocks of a product crate's source, and of this
/// runner's own source (under `runner`, [`runner_prefix`]) the tool body
/// too.
fn scope_of(relative: &str, declared: &BTreeSet<String>, runner: &str) -> rust::Scope {
    if whole_test_file(relative) || declared.contains(relative) {
        rust::Scope::Whole
    } else if relative.starts_with(&format!("{runner}/src/")) {
        rust::Scope::Tool
    } else {
        rust::Scope::Tests
    }
}

/// This runner's own crate, relative to `root` the way every file read
/// here is — the crate this is compiled from, so that no path is spelled
/// here.
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
/// or what it is called: anything under a crate's `tests`, and the files
/// `src` names as tests (structure.md §分割: テストだけ巨大なら同ディレクトリの
/// 専用ファイルへ). A test file under another name is known by the
/// `#[cfg(test)] mod` that declares it ([`declared_test_files`]).
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

    // The paths below are fixtures under a crate that does not exist: a
    // real path in a string here is read by the gate's graph as this file
    // reading that one, and a crate root among them is refused outright.

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
            "beside the declaring file, not under a directory of its stem"
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
