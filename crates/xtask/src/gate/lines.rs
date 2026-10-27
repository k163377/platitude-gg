//! The registered verify-ui lines — what the gate runs — and what each
//! brings up beyond the window's core: the one table the verb selection,
//! the coverage question and the whole gate's enumeration read, held in
//! the tree and written by people.
//!
//! A row is `<verify-ui line> TAB <subjects>`: the line as typed (its
//! canonical spelling, `verify::canonical_line`), then the components the
//! run stands up that the core does not — the popups it opens, the parts
//! a Loader builds for it, the rows a list has only for its preset — or
//! `-` for a run that shows the core alone. The `*` row names what every
//! line stands up besides `Main`: the page, and the rows every preset has.
//! What a subject composes in turn is read off the sources
//! ([`super::shown`]), so a row names the surfaces, not every part.
//!
//! No run writes this file. A run says what it met beside what its row
//! declares (`verify::observed`), for a person to move the row by.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::shown::{Composition, MAIN};

pub(crate) const FILE: &str = "crates/xtask/verb-lines.txt";

/// The row that names what every line stands up.
const EVERY: &str = "*";

#[derive(Default)]
pub(crate) struct Lines {
    /// What every line stands up besides `Main` (the `*` row).
    pub defaults: BTreeSet<String>,
    /// verify-ui line -> the subjects its run brings up.
    pub entries: BTreeMap<String, BTreeSet<String>>,
}

/// What to do about a table that cannot be read whole.
const PUT_BACK: &str = "the file is kept by hand: fix the row and run again";

impl Lines {
    /// The table as the tree holds it. A table that cannot be read — gone,
    /// not text, a row of another shape — is an error naming the row:
    /// read as empty, the gate would owe no verb at all and call every
    /// component unshown.
    pub(crate) fn load(root: &Path) -> Result<Lines, String> {
        let bytes = std::fs::read(root.join(FILE)).map_err(|e| format!("{FILE}: {e}"))?;
        let text = std::str::from_utf8(&bytes).map_err(|e| {
            let row = bytes[..e.valid_up_to()]
                .iter()
                .filter(|byte| **byte == b'\n')
                .count()
                + 1;
            format!("{FILE}:{row}: not UTF-8 — {PUT_BACK}")
        })?;
        Lines::parse(text)
    }

    /// `<line> TAB <subjects, space-separated, or ->` per row, `#` a
    /// comment, `*` the row of every line. A row of any other shape, and
    /// a line held twice, is named by its row.
    pub(crate) fn parse(text: &str) -> Result<Lines, String> {
        let mut lines = Lines::default();
        let mut first_at: BTreeMap<&str, usize> = BTreeMap::new();
        let mut wrong = Vec::new();
        for (at, row) in text.lines().enumerate() {
            let at = at + 1;
            if row.starts_with('#') || row.trim().is_empty() {
                continue;
            }
            let Some((key, subjects)) = row.split_once('\t').filter(|(key, _)| !key.is_empty())
            else {
                wrong.push(format!(
                    "{FILE}:{at}: not `<verify-ui line> TAB <subjects>`: {row:?}"
                ));
                continue;
            };
            if let Some(first) = first_at.get(key) {
                wrong.push(format!("{FILE}:{at}: {key:?} again (first at row {first})"));
                continue;
            }
            first_at.insert(key, at);
            let subjects: BTreeSet<String> = subjects
                .split_whitespace()
                .filter(|word| *word != "-")
                .map(str::to_string)
                .collect();
            if key == EVERY {
                lines.defaults = subjects;
            } else {
                lines.entries.insert(key.to_string(), subjects);
            }
        }
        if wrong.is_empty() {
            return Ok(lines);
        }
        Err(format!(
            "{FILE} holds rows that are no line's, so nothing may choose verbs off it — {PUT_BACK}:\n{}",
            wrong
                .iter()
                .map(|row| format!("  {row}"))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }

    pub(crate) fn contains(&self, line: &str) -> bool {
        self.entries.contains_key(line)
    }

    /// What a registered line's run promises to show: the `*` row and its
    /// own subjects — held against the census at its picture
    /// (`verify::contract`). `None` for a line the table does not hold.
    pub(crate) fn declared(&self, line: &str) -> Option<BTreeSet<String>> {
        self.entries
            .get(line)
            .map(|subjects| self.defaults.union(subjects).cloned().collect())
    }

    /// Every part some registered line promises to show: what a green
    /// line is evidence of. A component outside it is verified by no run
    /// (`plan::uncovered`).
    pub(crate) fn promised(&self) -> BTreeSet<String> {
        self.defaults
            .iter()
            .chain(self.entries.values().flatten())
            .cloned()
            .collect()
    }

    /// Every registered line, sorted.
    pub(crate) fn lines(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    /// What is wrong with the table against the sources: a subject that is
    /// no component, one no run could meet, or one every line shows already
    /// and that opens onto nothing.
    pub(crate) fn complaints(&self, composition: &Composition) -> Vec<String> {
        let mut out = Vec::new();
        for subject in &self.defaults {
            if !composition.is_component(subject) {
                out.push(format!(
                    "{FILE}: the `*` row names {subject:?}, which is no QML component of the app"
                ));
            } else if !composition.instantiable(subject) {
                out.push(format!(
                    "{FILE}: the `*` row names {subject:?}, a singleton no run can meet in the \
                     window — every line would then lack it"
                ));
            }
        }
        for (line, subjects) in &self.entries {
            for subject in subjects {
                if !composition.is_component(subject) {
                    out.push(format!(
                        "{FILE}: {line:?} names {subject:?}, which is no QML component of the \
                         app — a renamed or dropped file: follow it or take the name out"
                    ));
                } else if !composition.instantiable(subject) {
                    out.push(format!(
                        "{FILE}: {line:?} names {subject:?}, a singleton no run can meet in the \
                         window — name what shows it instead"
                    ));
                } else if self.defaults.contains(subject) {
                    out.push(format!(
                        "{FILE}: {line:?} names {subject:?}, which the `*` row promises for every \
                         line already — take the name out"
                    ));
                }
            }
        }
        out
    }

    /// The window's core: `Main`, the `*` row, and all they compose.
    pub(crate) fn core(&self, composition: &Composition) -> BTreeSet<String> {
        composition.standing(std::iter::once(MAIN).chain(self.defaults.iter().map(String::as_str)))
    }

    /// What each line shows, closed over the sources.
    pub(crate) fn shown(&self, composition: &Composition) -> Shown {
        let core = self.core(composition);
        let lines = self
            .entries
            .iter()
            .map(|(line, subjects)| {
                let mut shown = core.clone();
                shown.extend(composition.brought(subjects.iter().map(String::as_str)));
                (line.clone(), shown)
            })
            .collect();
        Shown { core, lines }
    }
}

/// What the registered lines show, by line.
pub(crate) struct Shown {
    core: BTreeSet<String>,
    lines: BTreeMap<String, BTreeSet<String>>,
}

impl Shown {
    pub(crate) fn core(&self) -> &BTreeSet<String> {
        &self.core
    }

    /// What `line` shows: the core and what its subjects bring up. Empty
    /// for a line the table does not hold.
    pub(crate) fn of(&self, line: &str) -> Option<&BTreeSet<String>> {
        self.lines.get(line)
    }
}

#[cfg(test)]
mod tests {
    use super::{FILE, Lines};
    use crate::gate::shown::Composition;
    use std::collections::BTreeSet;

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|n| (*n).to_string()).collect()
    }

    fn composition() -> Composition {
        Composition::from_sources(&[
            (
                "crates/platitude-app/src/ui/Main.qml".into(),
                "Item {\n    Page {}\n    SettingsDialog {}\n}\n".into(),
            ),
            (
                "crates/platitude-app/src/ui/Page.qml".into(),
                "Item {\n    GraphPane {}\n    Loader {\n        active: page.open\n        sourceComponent: PlanPane {}\n    }\n}\n".into(),
            ),
            (
                "crates/platitude-app/src/ui/GraphPane.qml".into(),
                "Item {\n    ListView {\n        delegate: GraphRow {}\n    }\n}\n".into(),
            ),
            (
                "crates/platitude-app/src/ui/GraphRow.qml".into(),
                "Item {}\n".into(),
            ),
            (
                "crates/platitude-app/src/ui/PlanPane.qml".into(),
                "Item {}\n".into(),
            ),
            (
                "crates/platitude-app/src/ui/SettingsDialog.qml".into(),
                "Dialog {\n    Section {}\n}\n".into(),
            ),
            (
                "crates/platitude-app/src/ui/Section.qml".into(),
                "Item {}\n".into(),
            ),
            (
                "crates/platitude-app/src/ui/Theme.qml".into(),
                "pragma Singleton\nQtObject {}\n".into(),
            ),
        ])
    }

    #[test]
    fn the_core_is_main_and_the_every_row_and_a_line_adds_what_it_brings_up() {
        let lines = Lines::parse(
            "# lines\n*\tGraphRow\nsettings\tSettingsDialog\nplan --preset plan\tPlanPane\ngraph\t-\n",
        )
        .expect("a table as written");
        let comp = composition();
        let shown = lines.shown(&comp);
        assert_eq!(
            *shown.core(),
            set(&["Main", "Page", "GraphPane", "GraphRow"]),
            "the dialog and the plan stand on a condition"
        );
        assert_eq!(
            *shown.of("settings").expect("registered"),
            set(&[
                "Main",
                "Page",
                "GraphPane",
                "GraphRow",
                "SettingsDialog",
                "Section"
            ])
        );
        assert_eq!(
            *shown.of("plan --preset plan").expect("registered"),
            set(&["Main", "Page", "GraphPane", "GraphRow", "PlanPane"])
        );
        assert_eq!(*shown.of("graph").expect("registered"), *shown.core());
        // What the rows promise is the `*` row and each line's own subjects,
        // never the closure the sources give them.
        assert_eq!(
            lines.declared("settings").expect("registered"),
            set(&["GraphRow", "SettingsDialog"])
        );
        assert_eq!(
            lines.promised(),
            set(&["GraphRow", "SettingsDialog", "PlanPane"])
        );
        assert!(lines.complaints(&comp).is_empty());
    }

    #[test]
    fn a_subject_that_is_no_component_a_singleton_or_the_star_rows_is_complained_of() {
        let lines = Lines::parse(
            "*\tGraphRow\nsettings\tSettingsDialog Gone\ntheme\tTheme\ngraph\tGraphRow\n\
             plan\tPlanPane\n",
        )
        .expect("a table as written");
        let said = lines.complaints(&composition());
        assert_eq!(said.len(), 3, "{said:#?}");
        assert!(
            said[0].contains("\"GraphRow\"") && said[0].contains("`*` row promises"),
            "{said:?}"
        );
        assert!(
            said[1].contains("\"Gone\"") && said[1].contains("no QML component"),
            "{said:?}"
        );
        assert!(
            said[2].contains("\"Theme\"") && said[2].contains("singleton"),
            "{said:?}"
        );
        let star = Lines::parse("*\tTheme\ngraph\tGraphRow\n").expect("a table as written");
        let said = star.complaints(&composition());
        assert!(
            said.len() == 1
                && said[0].contains("`*` row names \"Theme\"")
                && said[0].contains("singleton"),
            "{said:?}"
        );
    }

    #[test]
    fn a_table_that_cannot_be_read_whole_is_named_by_its_row() {
        for (what, text, says) in [
            (
                "a row with no tab",
                "graph\t-\n<<<<<<< HEAD\n",
                ":2: not `<verify-ui line> TAB <subjects>`",
            ),
            (
                "a line held twice",
                "graph\t-\nsettings\t-\ngraph\tPage\n",
                ":3: \"graph\" again (first at row 1)",
            ),
        ] {
            let Err(read) = Lines::parse(text) else {
                panic!("{what}: read as a table");
            };
            assert!(read.contains(&format!("{FILE}{says}")), "{what}: {read}");
        }
        assert!(Lines::parse("").expect("an empty table").entries.is_empty());
    }
}
