//! The verb census: which QML components each verify-ui line brought to
//! life, recorded by the runs themselves.
//!
//! The harness walks the window's item tree through the run and reports
//! every QML type it met (`WindowCensus.qml`); a passing run writes them
//! here against the line that ran it. The gate then owes a verb for a
//! QML change when the verb's census names a file the change reaches —
//! a mechanical answer to "which verbs show this", with no table anyone
//! writes by hand. A file the census never names is one no verb shows,
//! and the gate says so rather than passing it.
//!
//! Only lines that are reproducible are recorded: a run against a `--repo`
//! of this machine names nothing another machine has.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::graph::{collect, stem_of};

/// Where the census lives, checked in beside the runner so a fresh clone
/// knows what the verbs show. Rewritten by runs, never by hand.
pub(crate) const FILE: &str = "crates/xtask/verb-census.txt";

const HEADER: &str = "\
# The verb census — written by `cargo xtask verify-ui`, read by `cargo xtask gate`.
#
# One line per verify-ui argument line that passed, followed by a tab and
# the QML components (ui/ and auto/, by file name) the run brought to
# life at any moment. The gate owes a change every verb whose census
# names a file the change reaches. A run adds to its line; a name whose
# file is gone is dropped on the next write. Never edited by hand: run
# the verb instead, and this file follows.
";

#[derive(Default)]
pub(crate) struct Census {
    /// verify-ui line -> the component names its run met.
    pub lines: BTreeMap<String, BTreeSet<String>>,
}

impl Census {
    pub(crate) fn load(root: &Path) -> Census {
        let text = std::fs::read_to_string(root.join(FILE)).unwrap_or_default();
        let mut census = Census::default();
        for line in text.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let Some((key, names)) = line.split_once('\t') else {
                continue;
            };
            census.lines.insert(
                key.to_string(),
                names.split_whitespace().map(str::to_string).collect(),
            );
        }
        census
    }

    /// Every recorded line whose run met one of `stems`.
    pub(crate) fn verbs_touching(&self, stems: &BTreeSet<String>) -> Vec<String> {
        self.lines
            .iter()
            .filter(|(_, names)| names.iter().any(|name| stems.contains(name)))
            .map(|(line, _)| line.clone())
            .collect()
    }

    /// Whether any run ever met `stem`.
    pub(crate) fn covers(&self, stem: &str) -> bool {
        self.lines.values().any(|names| names.contains(stem))
    }

    fn save(&self, root: &Path) -> Result<(), String> {
        let mut text = String::from(HEADER);
        for (line, names) in &self.lines {
            text.push_str(line);
            text.push('\t');
            text.push_str(&names.iter().cloned().collect::<Vec<_>>().join(" "));
            text.push('\n');
        }
        let path = root.join(FILE);
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// Records what a passing run of `line` met. Names that are no QML file
/// of the app (C++ types, inline components, files since removed) are
/// dropped; the rest join what earlier runs of the same line saw.
pub(crate) fn record(root: &Path, line: &str, names: &[String]) -> Result<usize, String> {
    let known = component_files(root)?;
    let mut census = Census::load(root);
    let entry = census.lines.entry(line.to_string()).or_default();
    entry.extend(names.iter().filter(|n| known.contains(*n)).cloned());
    entry.retain(|name| known.contains(name));
    let count = entry.len();
    for names in census.lines.values_mut() {
        names.retain(|name| known.contains(name));
    }
    census.save(root)?;
    Ok(count)
}

/// The names of every QML component file of the app, product and
/// harness alike.
fn component_files(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut files = Vec::new();
    collect(
        root,
        &root.join("crates/platitude-app/src"),
        "qml",
        &mut files,
    )?;
    Ok(files.iter().map(|f| stem_of(f)).collect())
}

/// Whether a QML file can stand in the item tree at all. A singleton or a
/// `QtObject` root never does, so no census can name it: its readers
/// carry it.
pub(crate) fn instantiable(root: &Path, file: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(root.join(file)) else {
        return false;
    };
    if text.contains("pragma Singleton") {
        return false;
    }
    let code = super::graph::strip_comments(&text);
    // The root type is the first word of the first line that opens an
    // object: `Item {`, `QtObject {`, `AppCard {`.
    let root_type = code
        .lines()
        .map(str::trim)
        .find(|line| line.ends_with('{') && !line.starts_with("import") && !line.contains(':'))
        .and_then(|line| line.split_whitespace().next())
        .unwrap_or("");
    root_type != "QtObject"
}

/// The names a run reported: the `census=` line, comma-separated.
pub(crate) fn names_in(lines: &[String]) -> Option<Vec<String>> {
    lines
        .iter()
        .rev()
        .find_map(|line| line.split("census=").nth(1))
        .map(|rest| {
            rest.trim()
                .split(',')
                .map(str::trim)
                .filter(|n| !n.is_empty())
                .map(str::to_string)
                .collect()
        })
}

#[cfg(test)]
mod tests {
    use super::{Census, names_in};
    use std::collections::BTreeSet;

    #[test]
    fn owes_the_verbs_whose_runs_met_a_reached_file() {
        let mut census = Census::default();
        census.lines.insert(
            "commit --preset basic".into(),
            ["CommitBox", "GraphPane"].map(String::from).into(),
        );
        census.lines.insert(
            "diff-file notes.txt".into(),
            ["DiffPane"].map(String::from).into(),
        );
        let reached: BTreeSet<String> = ["DiffPane".to_string()].into();
        assert_eq!(census.verbs_touching(&reached), vec!["diff-file notes.txt"]);
        assert!(census.covers("GraphPane"));
        assert!(!census.covers("SettingsDialog"));
    }

    #[test]
    fn reads_the_census_line_a_run_reported() {
        let lines = vec![
            "2026-09-02T01:18:33Z  INFO bench: auto_act ran=commit".to_string(),
            "2026-09-02T01:18:33Z  INFO bench: census=AppCard,DiffPane,Main".to_string(),
        ];
        assert_eq!(
            names_in(&lines),
            Some(vec!["AppCard".into(), "DiffPane".into(), "Main".into()])
        );
        assert_eq!(names_in(&[]), None);
    }
}
