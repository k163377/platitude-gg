//! What one run met in the window, kept beside its pictures as a
//! diagnostic and said against its registered row — never written back
//! to the table (`gate::lines`), which people keep.
//!
//! The harness walks the window's item tree at the app's picture
//! (`WindowCensus.qml`) and says it as `census run=… page=…` and
//! `census=A,B,…`. What the row promises and the picture lacks is the
//! run's breach (`verify::contract`); what the picture had beyond the row
//! is said here, and is what a person widens a row by.

use std::collections::BTreeSet;
use std::path::Path;

use crate::gate::{Composition, Lines};

/// The file a run leaves beside its pictures.
pub(crate) const FILE: &str = "census.txt";

/// What the run's lines say it met.
pub(crate) struct Observed {
    /// Whether the page had stopped arriving at the picture.
    pub settled: bool,
    /// The terms beside the answer, as the walk said them.
    pub terms: String,
    /// Every name met at the picture.
    pub names: BTreeSet<String>,
}

impl Observed {
    /// Read off a run's lines: `None` for a run that never walked (ended
    /// before its picture).
    pub(crate) fn in_lines(lines: &[String]) -> Option<Observed> {
        let names: BTreeSet<String> = lines
            .iter()
            .rev()
            .filter(|line| !line.contains("census run="))
            .find_map(|line| line.split("bench: census=").nth(1))?
            .trim()
            .split(',')
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(str::to_string)
            .collect();
        let page = lines
            .iter()
            .rev()
            .find_map(|line| line.split("census run=").nth(1))
            .unwrap_or_default();
        let mut words = page.split_whitespace().skip(1);
        let settled = words.next() == Some("page=settled");
        Some(Observed {
            settled,
            terms: words.collect::<Vec<_>>().join(" "),
            names,
        })
    }

    /// What the run met that its row does not declare, and what the row
    /// declares that the run did not meet — among the components the
    /// table could name. The first is a row to widen; the second is a
    /// part that stood only part of the time, or a row to narrow.
    pub(crate) fn against(
        &self,
        shown: &BTreeSet<String>,
        composition: &Composition,
    ) -> (Vec<String>, Vec<String>) {
        let components = |name: &&String| composition.is_component(name);
        // A singleton the walk met is no subject a row may name
        // (`lines::complaints`), so it is never offered as one.
        let undeclared: Vec<String> = self
            .names
            .iter()
            .filter(components)
            .filter(|name| !shown.contains(*name) && composition.instantiable(name))
            .cloned()
            .collect();
        // A root another file wears is met under the wearer's name, never
        // its own, so it is no part the row names and the run lacked.
        let unmet: Vec<String> = shown
            .iter()
            .filter(|name| !self.names.contains(*name))
            .filter(components)
            .filter(|name| composition.instantiable(name) && !composition.is_worn(name))
            .cloned()
            .collect();
        (undeclared, unmet)
    }
}

/// Where a run stood: the side, the tree and the commit, so a file is
/// never read as another run's.
pub(crate) struct Standing {
    pub side: &'static str,
    pub commit: String,
    pub dirty: bool,
}

impl Standing {
    pub(crate) fn here(root: &Path) -> Standing {
        let here = root.display().to_string();
        let commit = crate::subprocess::git_query(&here, &["rev-parse", "--short", "HEAD"])
            .unwrap_or_else(|| "unknown".to_string());
        let dirty = crate::subprocess::git_query(&here, &["status", "--porcelain"])
            .is_some_and(|status| !status.trim().is_empty());
        Standing {
            side: if std::env::var_os(crate::linux::IN_CONTAINER).is_some() {
                "container"
            } else {
                "host"
            },
            commit,
            dirty,
        }
    }
}

/// Keeps what the run met beside its pictures and says it against the
/// run's row. `line` is the run's typable line, or `None` for one nobody
/// could type again. Answers the lines said, for the run's output; a file
/// that could not be written is said there too and fails nothing — the
/// verdict is the judgment's.
pub(crate) fn note(
    root: &Path,
    shot_dir: &Path,
    line: Option<&str>,
    verdict: &str,
    said: &[String],
) -> Vec<String> {
    let Some(observed) = Observed::in_lines(said) else {
        return vec!["shown: the run ended before it walked the window".to_string()];
    };
    let standing = Standing::here(root);
    let mut out = Vec::new();
    let mut text = format!(
        "# what this run met in the window (diagnostic; the gate reads {})\n\
         line={}\nside={} os={} commit={}{}\nverdict={verdict}\npage={} {}\ncensus={}\n",
        crate::gate::LINES_FILE,
        line.unwrap_or("-"),
        standing.side,
        std::env::consts::OS,
        standing.commit,
        if standing.dirty { " dirty" } else { "" },
        if observed.settled {
            "settled"
        } else {
            "arriving"
        },
        observed.terms,
        observed.names.iter().cloned().collect::<Vec<_>>().join(","),
    );
    match (line, Lines::load(root), Composition::read(root)) {
        (Some(line), Ok(lines), Ok(composition)) => {
            let shown = lines.shown(&composition);
            match shown.of(line) {
                None => out.push(format!(
                    "shown: {line} is not registered in {} — no gate runs it until it is",
                    crate::gate::LINES_FILE
                )),
                Some(shown) => {
                    let (undeclared, unmet) = observed.against(shown, &composition);
                    text.push_str(&format!(
                        "undeclared={}\nunmet={}\n",
                        undeclared.join(","),
                        unmet.join(",")
                    ));
                    out.push(format!(
                        "shown: {} component(s) met{}{}",
                        observed.names.len(),
                        if undeclared.is_empty() {
                            String::new()
                        } else {
                            format!(
                                "; beside the row: {} (met, not declared — a row to widen if \
                                 the run brings them up by design)",
                                undeclared.join(" ")
                            )
                        },
                        if unmet.is_empty() {
                            String::new()
                        } else {
                            format!(
                                "; declared, not met: {} (stood only part of the time, or a \
                                 row to narrow)",
                                unmet.join(" ")
                            )
                        },
                    ));
                }
            }
        }
        (None, _, _) => {}
        (_, Err(why), _) | (_, _, Err(why)) => out.push(format!("shown: not compared — {why}")),
    }
    let path = shot_dir.join(FILE);
    if let Err(e) = std::fs::write(&path, text) {
        out.push(format!(
            "shown: {} could not be written: {e}",
            path.display()
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::Observed;
    use crate::gate::Composition;
    use std::collections::BTreeSet;

    fn said(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|l| (*l).to_string()).collect()
    }

    #[test]
    fn reads_the_walk_a_run_reported() {
        let observed = Observed::in_lines(&said(&[
            "2026-09-02T01:18:33Z  INFO bench: auto_act ran=commit",
            "INFO bench: census run=commit-1-2-0 page=settled state=open finish=2",
            "INFO bench: census=AppCard,DiffPane,Main",
        ]))
        .expect("a run that walked");
        assert!(observed.settled);
        assert_eq!(observed.terms, "state=open finish=2");
        assert_eq!(
            observed.names,
            ["AppCard", "DiffPane", "Main"].map(String::from).into()
        );
        // The terms beside the answer are no second answer.
        let arriving = Observed::in_lines(&said(&[
            "INFO bench: census run=commit-1-2-0 page=arriving state=open",
            "INFO bench: census=Main",
        ]))
        .expect("a run that walked");
        assert!(!arriving.settled);
        assert!(Observed::in_lines(&said(&["INFO bench: auto_act ran=commit"])).is_none());
    }

    #[test]
    fn says_what_was_met_beside_the_row_and_what_the_row_names_unmet() {
        let composition = Composition::from_sources(&[
            (
                "crates/platitude-app/src/ui/Main.qml".into(),
                "Item {\n    Pane {}\n    SettingsDialog {}\n}\n".into(),
            ),
            (
                "crates/platitude-app/src/ui/Pane.qml".into(),
                "Item {}\n".into(),
            ),
            (
                "crates/platitude-app/src/ui/SettingsDialog.qml".into(),
                "Dialog {}\n".into(),
            ),
            (
                "crates/platitude-app/src/ui/State.qml".into(),
                "QtObject {}\n".into(),
            ),
            (
                "crates/platitude-app/src/ui/Bar.qml".into(),
                "Item {}\n".into(),
            ),
            (
                "crates/platitude-app/src/ui/GraphBar.qml".into(),
                "Bar {}\n".into(),
            ),
        ]);
        let observed = Observed::in_lines(&said(&[
            "INFO bench: census run=r1 page=settled state=open",
            "INFO bench: census=Main,SettingsDialog,QQuickText,State,GraphBar",
        ]))
        .expect("walked");
        let shown: BTreeSet<String> = ["Main", "Pane", "State", "Bar", "GraphBar"]
            .map(String::from)
            .into();
        let (undeclared, unmet) = observed.against(&shown, &composition);
        assert_eq!(
            undeclared,
            ["SettingsDialog"],
            "a C++ type is no component to declare"
        );
        assert_eq!(
            unmet,
            ["Pane"],
            "a root another file wears is met under the wearer's name, so it is not unmet"
        );
    }
}
