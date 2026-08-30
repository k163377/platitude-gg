//! `cargo xtask waits` — every await in the integration suite that the
//! session will not answer with an event must sit under the suite's own
//! backstop (`support::wait::bounded`, or an explicit `timeout`).
//!
//! What it guards against is not slowness but silence: a tracked
//! completion (`outcome()`), a session boundary (`wait_for_graph_passes`,
//! `wait_for_snapshot_reads`) and a hand-stepped tick all resolve through
//! channels no `Patience` is watching, so a naked await on one has
//! nothing under it — the binary sits until the CI kill with no failing
//! test named. Each of those has hung a real run (a starved timer task in
//! 2026-08, its stopped twin over a whole 900s budget in 2026-08-30), and
//! the fix each time included the backstop this count now holds in place.

use std::path::{Path, PathBuf};

/// The completions that resolve through a channel nothing else is
/// watching. A statement that holds one *and* an `.await` is a silent
/// wait; without the await it is just a handle changing hands.
const SILENT_WAITS: [&str; 4] = [
    ".outcome()",
    "wait_for_graph_passes()",
    "wait_for_snapshot_reads()",
    ".tick()",
];

/// What discharges one, looked for in the same statement — the unit that
/// survives however rustfmt wraps the call around its wrapper.
const BACKSTOPS: [&str; 2] = ["bounded(", "timeout("];

pub fn run(args: &[String]) -> Result<(), String> {
    if let Some(unknown) = args.first() {
        return Err(format!("unknown option {unknown:?} (waits takes none)"));
    }
    let root = crate::workspace_root();
    let suite = root
        .join("crates")
        .join("platitude-core")
        .join("tests")
        .join("it");
    let mut files = Vec::new();
    collect(&suite, &mut files)?;
    files.sort();

    let mut naked: Vec<String> = Vec::new();
    let mut scanned = 0usize;
    for file in &files {
        // The support modules implement the backstops; the tests are what
        // must go through them.
        if file.components().any(|c| c.as_os_str() == "support") {
            continue;
        }
        scanned += 1;
        let text = std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
        let relative = file
            .strip_prefix(&root)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        for (line, at) in naked_waits(&text) {
            naked.push(format!("{relative}:{at}: {}", line.trim()));
        }
    }

    if naked.is_empty() {
        println!("waits: {scanned} test files scanned, every silent wait under a backstop — PASS");
        Ok(())
    } else {
        for finding in &naked {
            println!("waits: {finding}");
        }
        Err(format!(
            "{} naked wait(s): these resolve through channels no Patience watches, so wrap \
             them in support::wait::bounded (or an explicit timeout) to fail by name instead \
             of hanging the binary",
            naked.len()
        ))
    }
}

/// The statements of `text` that await a silent completion with no
/// backstop in the same statement, as (first line of the statement,
/// 1-based line number).
///
/// A statement is what sits between `;`, `{` and `}`, comments stripped —
/// coarse, but exact where it matters: rustfmt may wrap a call across any
/// number of lines and never across a statement, so the wrapper and the
/// wait always share one. String literals are not parsed; a `;` inside
/// one splits a statement in two, which can only turn a wrapped wait into
/// a finding to look at, never hide a naked one.
fn naked_waits(text: &str) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    let mut statement = String::new();
    let mut opened_at = 1;
    let mut line = 1;
    let mut fresh = true;
    let code = text
        .lines()
        .map(|l| l.split("//").next().unwrap_or(l))
        .collect::<Vec<_>>()
        .join("\n");
    for ch in code.chars() {
        if fresh && !ch.is_whitespace() {
            opened_at = line;
            fresh = false;
        }
        if ch == '\n' {
            line += 1;
        }
        if matches!(ch, ';' | '{' | '}') {
            judge(&statement, opened_at, &mut found);
            statement.clear();
            fresh = true;
        } else {
            statement.push(ch);
        }
    }
    judge(&statement, opened_at, &mut found);
    found
}

fn judge(statement: &str, opened_at: usize, found: &mut Vec<(String, usize)>) {
    if !statement.contains(".await") {
        return;
    }
    if !SILENT_WAITS.iter().any(|wait| statement.contains(wait)) {
        return;
    }
    if BACKSTOPS.iter().any(|stop| statement.contains(stop)) {
        return;
    }
    let first = statement
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or_default();
    found.push((first.to_string(), opened_at));
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::naked_waits;

    #[test]
    fn a_wrapped_wait_passes_and_a_naked_one_is_named() {
        let text = "\
let a = bounded(\"x\", task.outcome()).await;
let b = task.outcome().await;
";
        let found = naked_waits(text);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].1, 2);
    }

    #[test]
    fn the_backstop_is_seen_across_rustfmts_wrapping() {
        let text = "\
let a = crate::support::wait::bounded(
    \"the tracked poll\",
    session.refresh_poll_tracked().outcome(),
)
.await;
let b = session
    .refresh_poll_tracked()
    .outcome()
    .await;
";
        let found = naked_waits(text);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].1, 6, "the naked one is named at its own statement");
    }

    #[test]
    fn boundary_waits_and_ticks_are_counted_too() {
        let text = "\
session.wait_for_graph_passes().await;
let took = ticker.tick().await;
tokio::time::timeout(BUDGET, session.wait_for_snapshot_reads()).await;
let handle = session.refresh_poll_tracked();
";
        let found = naked_waits(text);
        assert_eq!(
            found.len(),
            2,
            "an unawaited handle is not a wait: {found:?}"
        );
    }

    #[test]
    fn a_wait_described_in_a_comment_is_not_a_wait() {
        let text = "\
// waiting on task.outcome() here would .await forever
let a = 1;
";
        assert!(naked_waits(text).is_empty());
    }
}
