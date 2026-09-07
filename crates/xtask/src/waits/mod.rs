//! `cargo xtask waits` — no test in this workspace takes a stretch of
//! clock for an answer.
//!
//! What it reads is every statement of test code — the integration
//! suites with their support modules, the `#[cfg(test)]` blocks and
//! `*_tests.rs` files of every crate, and the QtTest files — and what it
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
//! What a test may still do it says on the line: `// waits(<purpose>):
//! <reason>` above or beside the statement, with a purpose of `paced`
//! (a sleep spacing a retry whose completion is causal), `ceiling` (a
//! wall-clock ceiling that only names a failure), `measured` (a clock
//! read handed to the code under test or printed, judged by nothing) or
//! `timed` (a real-time property of the product, judged as a bound no
//! load can break). A marker that covers nothing is a finding of its
//! own, so a rewritten test sheds its marker.

mod qml;
mod rust;
mod source;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use source::Purpose;

/// A statement of test code that breaks a rule.
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

pub fn run(args: &[String]) -> Result<(), String> {
    if let Some(unknown) = args.first() {
        return Err(format!("unknown option {unknown:?} (waits takes none)"));
    }
    let root = crate::tree::workspace_root();
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
    let (mut rust_files, mut qml_files) = (0usize, 0usize);
    for (relative, lang, text) in &sources {
        let (found, allowed) = match lang {
            source::Lang::Rust => {
                rust_files += 1;
                let whole = whole_test_file(relative) || declared.contains(relative);
                rust::scan(relative, text, whole)
            }
            source::Lang::Qml => {
                qml_files += 1;
                qml::scan(relative, text)
            }
        };
        findings.extend(found);
        exceptions.extend(allowed);
    }
    report(&findings, &exceptions, rust_files, qml_files)
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
        for name in source::declared_test_modules(&code) {
            declared.extend(module_files(relative, &name));
        }
    }
    declared
}

/// Where `mod name;` in `declaring` puts its file: beside a `mod.rs` /
/// `lib.rs` / `main.rs`, and under a directory of the file's own stem
/// otherwise — as `name.rs`, or as `name/mod.rs`.
fn module_files(declaring: &str, name: &str) -> [String; 2] {
    let (dir, file) = declaring.rsplit_once('/').unwrap_or(("", declaring));
    let stem = file.strip_suffix(".rs").unwrap_or(file);
    let base = if matches!(stem, "mod" | "lib" | "main") {
        dir.to_string()
    } else {
        format!("{dir}/{stem}")
    };
    [format!("{base}/{name}.rs"), format!("{base}/{name}/mod.rs")]
}

fn report(
    findings: &[Finding],
    exceptions: &[Exception],
    rust_files: usize,
    qml_files: usize,
) -> Result<(), String> {
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
            "waits: {rust_files} Rust and {qml_files} QML test files read, {} statement(s) \
             standing on a marker ({counted}) — PASS",
            exceptions.len()
        );
        return Ok(());
    }
    Err(format!(
        "{} wait(s) that spend the clock or read no answer: end a wait on the answer \
         (a completion handle, an event, a hand-driven poll), take the budget from the suite \
         (`bounded`), read what a wait answered — or say on the line why this one stands \
         (`// waits(<purpose>): <reason>`, purposes paced / ceiling / measured / timed)",
        findings.len()
    ))
}

/// Which scanner reads a file, or none: Rust under a crate's `src` or
/// `tests`, QtTest files under `tests/qml`, and never the budgets'
/// own implementation or this scanner's fixtures.
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
    if kind == "tests" && name.starts_with("tst_") && name.ends_with(".qml") {
        return Some(source::Lang::Qml);
    }
    None
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
    use super::{language_of, module_files, source::Lang, whole_test_file};

    // The paths below are fixtures under a crate that does not exist: a
    // real path in a string here is read by the gate's graph as this file
    // reading that one, and a crate root among them is refused outright.

    #[test]
    fn a_declared_module_resolves_beside_its_declaring_file() {
        assert_eq!(
            module_files("crates/x/src/graph/mod.rs", "testkit")[0],
            "crates/x/src/graph/testkit.rs"
        );
        assert_eq!(
            module_files("crates/x/src/offers.rs", "tests")[0],
            "crates/x/src/offers/tests.rs"
        );
        assert_eq!(
            module_files("crates/x/src/lib.rs", "wait"),
            [
                "crates/x/src/wait.rs".to_string(),
                "crates/x/src/wait/mod.rs".to_string()
            ]
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
    fn a_whole_test_file_is_named_by_where_it_stands_or_what_it_is_called() {
        assert!(whole_test_file("crates/x/tests/gate/support.rs"));
        assert!(whole_test_file("crates/x/src/session/state_tests.rs"));
        assert!(whole_test_file("crates/x/src/corpus/stream/tests.rs"));
        assert!(!whole_test_file("crates/x/src/session/read_flight.rs"));
    }
}
