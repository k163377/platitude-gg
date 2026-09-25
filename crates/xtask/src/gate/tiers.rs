//! Which census lines each gate owes, and on which side — the table
//! `crates/xtask/verb-tiers.txt`, read by the gate and checked by
//! `cargo xtask verbs` (反映前テストの機械化.md §段ごとに何を回すか).
//!
//! A census line is a whole app run on each side, and the census grows
//! with the features, so a line runs before a merge only for what it alone
//! catches there: an app decision, a wiring or a timing no cheaper test
//! holds.
//!
//! The container repeats a host line before a merge only where its fonts
//! can answer differently: it runs the same QML over the same core, and
//! every change still owes its clippy, tests, QtTest and `bare` there.
//!
//! No test reads the file, so like the census it is no edge of the graph
//! (`graph::literal_paths`): it decides which steps a gate owes, not what
//! any step reads.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::census::Census;

pub(crate) const FILE: &str = "crates/xtask/verb-tiers.txt";

/// What the table says of one census line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tier {
    /// Owed by the container before a merge too: the line is judged on a
    /// size, a cut or a fold that the fonts decide.
    Linux,
    /// Owed by the full gate only. It names the pre-merge lines that walk
    /// its path, and they have to stay pre-merge lines — unless its claim
    /// is held elsewhere ([`STANDS_ALONE`]).
    Full,
    /// Owed by no gate: the census line it leans on runs the same path and
    /// judges as much. A passing run of it is not recorded, so it does not
    /// come back.
    Twin,
}

/// What a full row's reason starts with: the kind of thing that tells it
/// from the pre-merge lines it leans on.
const KINDS: [&str; 5] = ["git:", "picture:", "held:", "perf:", "other:"];

/// The kinds whose witness is no census line: a cheaper test, or the perf
/// tool's own runs.
const STANDS_ALONE: [&str; 2] = ["held:", "perf:"];

struct Entry {
    tier: Tier,
    /// The lines this one leans on: the pre-merge lines for [`Tier::Full`],
    /// the one line it is the same run as for [`Tier::Twin`].
    leans: Vec<String>,
    why: String,
}

#[derive(Default)]
pub(crate) struct Tiers {
    entries: BTreeMap<String, Entry>,
    /// Rows that could not be read, said by [`Tiers::complaints`].
    unread: Vec<String>,
}

impl Tiers {
    pub(crate) fn load(root: &Path) -> Tiers {
        Tiers::parse(&std::fs::read_to_string(root.join(FILE)).unwrap_or_default())
    }

    /// `<tier> TAB <census line> TAB <leans, ` ; `-separated, or -> TAB
    /// <why>` per row; `#` starts a comment.
    pub(crate) fn parse(text: &str) -> Tiers {
        let mut tiers = Tiers::default();
        for (at, row) in text.lines().enumerate() {
            if row.starts_with('#') || row.trim().is_empty() {
                continue;
            }
            let fields: Vec<&str> = row.split('\t').collect();
            let tier = match fields.first().copied() {
                Some("linux") => Tier::Linux,
                Some("full") => Tier::Full,
                Some("twin") => Tier::Twin,
                _ => {
                    tiers
                        .unread
                        .push(format!("row {}: no tier in {row:?}", at + 1));
                    continue;
                }
            };
            let [_, line, leans, why] = fields[..] else {
                tiers.unread.push(format!(
                    "row {}: four fields, tab-separated, in {row:?}",
                    at + 1
                ));
                continue;
            };
            if why.trim().is_empty() {
                tiers
                    .unread
                    .push(format!("row {}: no reason for {line:?}", at + 1));
            }
            let leans = leans
                .split(" ; ")
                .map(str::trim)
                .filter(|lean| !lean.is_empty() && *lean != "-")
                .map(str::to_string)
                .collect();
            let why = why.trim().to_string();
            if tiers
                .entries
                .insert(line.to_string(), Entry { tier, leans, why })
                .is_some()
            {
                tiers.unread.push(format!("row {}: {line:?} twice", at + 1));
            }
        }
        tiers
    }

    pub(crate) fn tier(&self, line: &str) -> Option<Tier> {
        self.entries.get(line).map(|entry| entry.tier)
    }

    /// The line a twin is the same run as.
    pub(crate) fn twin_of(&self, line: &str) -> Option<&str> {
        self.entries
            .get(line)
            .filter(|entry| entry.tier == Tier::Twin)
            .and_then(|entry| entry.leans.first())
            .map(String::as_str)
    }

    /// Whether a gate owes `line` at all — before a merge (`full` false)
    /// or in the full gate.
    pub(crate) fn owed(&self, line: &str, full: bool) -> bool {
        match self.tier(line) {
            None | Some(Tier::Linux) => true,
            Some(Tier::Full) => full,
            Some(Tier::Twin) => false,
        }
    }

    /// Whether the container owes `line`, given that the gate owes it.
    pub(crate) fn on_linux(&self, line: &str, full: bool) -> bool {
        full || self.tier(line) == Some(Tier::Linux)
    }

    /// The verbs a census holds no line of on purpose: every row the table
    /// has of the verb is a twin of another verb's line.
    pub(crate) fn twinned_away(&self) -> BTreeSet<String> {
        let verb = |line: &str| line.split_whitespace().next().unwrap_or("").to_string();
        let mut away = BTreeMap::new();
        for (line, entry) in &self.entries {
            let elsewhere = entry.tier == Tier::Twin
                && entry.leans.first().is_some_and(|of| verb(of) != verb(line));
            *away.entry(verb(line)).or_insert(true) &= elsewhere;
        }
        away.into_iter()
            .filter_map(|(verb, away)| away.then_some(verb))
            .collect()
    }

    /// How many rows of each tier: linux, full, twin.
    pub(crate) fn counts(&self) -> (usize, usize, usize) {
        let of = |tier| self.entries.values().filter(|e| e.tier == tier).count();
        (of(Tier::Linux), of(Tier::Full), of(Tier::Twin))
    }

    /// What is wrong with the table against `census`. The failure this is
    /// for is a lean that moved: moving the pre-merge line a full line
    /// names to the full gate too would silently leave its claim with no
    /// witness before a merge.
    pub(crate) fn complaints(&self, census: &Census) -> Vec<String> {
        let recorded: BTreeSet<&str> = census.lines.keys().map(String::as_str).collect();
        let before_merge = |line: &str| {
            recorded.contains(line) && matches!(self.tier(line), None | Some(Tier::Linux))
        };
        let mut out = self.unread.clone();
        for (line, entry) in &self.entries {
            match entry.tier {
                Tier::Linux | Tier::Full if !recorded.contains(line.as_str()) => out.push(format!(
                    "{FILE} names {line:?}, which the census does not hold — a renamed or dropped \
                     line: follow it or take the row out"
                )),
                Tier::Twin if recorded.contains(line.as_str()) => out.push(format!(
                    "{line:?} is in the census again, and {FILE} says it is the same run as {:?} \
                     — drop the census line",
                    entry.leans.first().map_or("", String::as_str)
                )),
                _ => {}
            }
            match entry.tier {
                Tier::Linux if !entry.leans.is_empty() => {
                    out.push(format!("{line:?} is a linux row and leans on nothing"));
                }
                Tier::Full => {
                    let kind = KINDS.iter().find(|kind| entry.why.starts_with(**kind));
                    match kind {
                        None => out.push(format!(
                            "{line:?}: a full row's reason starts with one of {KINDS:?}"
                        )),
                        Some(kind) if entry.leans.is_empty() && !STANDS_ALONE.contains(kind) => {
                            out.push(format!(
                                "{line:?} waits for the full gate on a {kind} reason and names no \
                                 pre-merge line walking its path — name one, or run it before a merge"
                            ));
                        }
                        Some(_) => {}
                    }
                    for lean in entry.leans.iter().filter(|lean| !before_merge(lean)) {
                        out.push(format!(
                            "{line:?} waits for the full gate leaning on {lean:?}, which is not a \
                             pre-merge line of the census — keep one of them before the merge"
                        ));
                    }
                }
                Tier::Twin => match entry.leans.as_slice() {
                    [of] if recorded.contains(of.as_str()) && self.tier(of) != Some(Tier::Twin) => {
                    }
                    _ => out.push(format!(
                        "{line:?} is a twin of {:?}, which has to be one census line that is no \
                         twin itself",
                        entry.leans
                    )),
                },
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{Census, Tier, Tiers};

    fn census(lines: &[&str]) -> Census {
        let mut census = Census::default();
        for line in lines {
            census
                .lines
                .insert((*line).to_string(), ["Main".to_string()].into());
        }
        census
    }

    const TABLE: &str = "# a comment\n\
        linux\tbadges --preset conflict\t-\tcap= reads the band's measured floor\n\
        full\tfile-menu b.txt\tdiscard-many-go c.txt\tpicture: the menu standing\n\
        twin\tdiscard-file b.txt\tfile-menu b.txt\tthe same run\n";

    /// A line the table does not name is the host's before a merge and
    /// both sides' in the full gate; each tier moves one of those four.
    #[test]
    fn each_tier_moves_what_a_gate_owes() {
        let tiers = Tiers::parse(TABLE);
        let pre_merge = "stash --preset basic";
        assert!(tiers.owed(pre_merge, false) && !tiers.on_linux(pre_merge, false));
        assert!(tiers.owed(pre_merge, true) && tiers.on_linux(pre_merge, true));
        let linux = "badges --preset conflict";
        assert_eq!(tiers.tier(linux), Some(Tier::Linux));
        assert!(tiers.owed(linux, false) && tiers.on_linux(linux, false));
        let full = "file-menu b.txt";
        assert!(!tiers.owed(full, false));
        assert!(tiers.owed(full, true) && tiers.on_linux(full, true));
        let twin = "discard-file b.txt";
        assert!(!tiers.owed(twin, false) && !tiers.owed(twin, true));
        assert_eq!(tiers.twin_of(twin), Some("file-menu b.txt"));
        assert_eq!(tiers.counts(), (1, 1, 1));
    }

    #[test]
    fn a_table_that_agrees_with_the_census_has_nothing_to_say() {
        let tiers = Tiers::parse(TABLE);
        let held = census(&[
            "badges --preset conflict",
            "file-menu b.txt",
            "discard-many-go c.txt",
        ]);
        assert_eq!(tiers.complaints(&held), Vec::<String>::new());
    }

    #[test]
    fn a_moved_lean_a_returned_twin_and_a_gone_line_are_each_named() {
        let said = |table: &str, lines: &[&str]| Tiers::parse(table).complaints(&census(lines));
        let gone = said(TABLE, &["file-menu b.txt", "discard-many-go c.txt"]);
        assert!(gone.len() == 1 && gone[0].contains("badges"), "{gone:?}");
        let back = said(
            TABLE,
            &[
                "badges --preset conflict",
                "file-menu b.txt",
                "discard-many-go c.txt",
                "discard-file b.txt",
            ],
        );
        assert!(
            back.len() == 1 && back[0].contains("in the census again"),
            "{back:?}"
        );
        let moved = said(
            &format!("{TABLE}full\tdiscard-many-go c.txt\t-\theld: a moved lean\n"),
            &[
                "badges --preset conflict",
                "file-menu b.txt",
                "discard-many-go c.txt",
            ],
        );
        assert!(
            moved.len() == 1 && moved[0].contains("keep one of them before the merge"),
            "{moved:?}"
        );
        let chained = said("twin\ta\tb\tsame\ntwin\tb\tc\tsame\n", &["c"]);
        assert!(
            chained.iter().any(|c| c.contains("no twin itself")),
            "{chained:?}"
        );
        let unread = said("full\tonly two fields\n", &[]);
        assert!(
            unread.iter().any(|c| c.contains("four fields")),
            "{unread:?}"
        );
    }

    #[test]
    fn a_full_row_names_its_pre_merge_line_unless_something_cheaper_holds_it() {
        let said = |row: &str| {
            Tiers::parse(&format!("{TABLE}{row}\n")).complaints(&census(&[
                "badges --preset conflict",
                "file-menu b.txt",
                "discard-many-go c.txt",
                "push",
            ]))
        };
        for kind in ["git", "picture", "other"] {
            let bare = said(&format!("full\tpush\t-\t{kind}: git answers"));
            assert!(
                bare.len() == 1 && bare[0].contains("names no pre-merge line"),
                "{kind}: {bare:?}"
            );
        }
        for kind in ["held", "perf"] {
            let alone = said(&format!("full\tpush\t-\t{kind}: a unit holds it"));
            assert_eq!(alone, Vec::<String>::new(), "{kind}");
        }
        let unsorted = said("full\tpush\tdiscard-many-go c.txt\tbecause");
        assert!(
            unsorted.len() == 1 && unsorted[0].contains("starts with one of"),
            "{unsorted:?}"
        );
    }

    #[test]
    fn a_verb_is_twinned_away_only_onto_other_verbs() {
        let tiers = Tiers::parse(&format!(
            "{TABLE}twin\ttab-pin 0\ttab-pin\tthe same run\n\
             full\tdiscard-many c.txt\tdiscard-many-go c.txt\tpicture: the menu\n\
             twin\tdiscard-many b.txt\tfile-menu b.txt\tthe same run\n"
        ));
        assert_eq!(
            tiers.twinned_away().into_iter().collect::<Vec<_>>(),
            ["discard-file"]
        );
    }
}
