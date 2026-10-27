//! Which registered lines (`gate::lines`) each gate owes, and on which
//! side — the table `crates/xtask/verb-tiers.txt`, read by the gate and
//! checked by `cargo xtask verbs` (反映前テストの機械化.md §段ごとに何を回すか).
//!
//! A registered line is a whole app run on each side, and the table grows
//! with the features, so a line runs before a merge only for what it alone
//! catches there: an app decision, a wiring or a timing no cheaper test
//! holds.
//!
//! The container repeats a host line before a merge only where its fonts
//! can answer differently: it runs the same QML over the same core, and
//! every change still owes its clippy, tests, QtTest and `bare` there.
//!
//! No test reads the file, so like the table of lines it is no edge of
//! the graph (`graph::literal_paths`): it decides which steps a gate owes,
//! not what any step reads.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::lines::Lines;

pub(crate) const FILE: &str = "crates/xtask/verb-tiers.txt";

/// What the table says of one registered line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tier {
    /// Owed by the container before a merge too: the line is judged on a
    /// size, a cut or a fold that the fonts decide.
    Fonts,
    /// Owed by the whole gate only. It names the pre-merge lines that walk
    /// its path, and they have to stay pre-merge lines — unless its claim
    /// is held elsewhere ([`STANDS_ALONE`]).
    Whole,
    /// Owed by no gate: the registered line it leans on runs the same path
    /// and judges as much. Registered nowhere itself (`complaints`).
    Twin,
    /// Owed by the host alone, before a merge and in the whole gate: the
    /// form needs a git above the container's minimum (its reason starts
    /// with `git:`), so the container can never answer for it.
    Host,
}

/// What a whole row's reason starts with: the kind of thing that tells it
/// from the pre-merge lines it leans on.
const KINDS: [&str; 5] = ["git:", "picture:", "held:", "perf:", "other:"];

/// The kinds whose witness is no registered line: a cheaper test, or the
/// perf tool's own runs.
const STANDS_ALONE: [&str; 2] = ["held:", "perf:"];

struct Entry {
    tier: Tier,
    /// The lines this one leans on: the pre-merge lines for [`Tier::Whole`],
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
    /// The table as the tree holds it. A table that cannot be read — gone,
    /// not text, not readable — is an error naming the path and the
    /// cause, never an empty table: empty, every selected line would be
    /// the host's before a merge and none the container's, and the
    /// complaints, which read the rows, would have nothing to say.
    pub(crate) fn load(root: &Path) -> Result<Tiers, String> {
        let text = std::fs::read_to_string(root.join(FILE)).map_err(|e| format!("{FILE}: {e}"))?;
        Ok(Tiers::parse(&text))
    }

    /// `<tier> TAB <registered line> TAB <leans, ` ; `-separated, or -> TAB
    /// <why>` per row; `#` starts a comment.
    pub(crate) fn parse(text: &str) -> Tiers {
        let mut tiers = Tiers::default();
        for (at, row) in text.lines().enumerate() {
            if row.starts_with('#') || row.trim().is_empty() {
                continue;
            }
            let fields: Vec<&str> = row.split('\t').collect();
            let tier = match fields.first().copied() {
                Some("fonts") => Tier::Fonts,
                Some("whole") => Tier::Whole,
                Some("twin") => Tier::Twin,
                Some("host") => Tier::Host,
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
    #[cfg(test)]
    fn twin_of(&self, line: &str) -> Option<&str> {
        self.entries
            .get(line)
            .filter(|entry| entry.tier == Tier::Twin)
            .and_then(|entry| entry.leans.first())
            .map(String::as_str)
    }

    /// Whether a gate owes `line` at all — before a merge (`whole_tier` false)
    /// or in the whole gate.
    pub(crate) fn owed(&self, line: &str, whole_tier: bool) -> bool {
        match self.tier(line) {
            None | Some(Tier::Fonts | Tier::Host) => true,
            Some(Tier::Whole) => whole_tier,
            Some(Tier::Twin) => false,
        }
    }

    /// Whether the container owes `line`, given that the gate owes it: never
    /// a host row's, whose git the container does not have.
    pub(crate) fn on_linux(&self, line: &str, whole_tier: bool) -> bool {
        match self.tier(line) {
            Some(Tier::Host) => false,
            Some(Tier::Fonts) => true,
            _ => whole_tier,
        }
    }

    /// The twin rows' lines: runs of their own, registered nowhere.
    pub(crate) fn twins(&self) -> impl Iterator<Item = String> + '_ {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.tier == Tier::Twin)
            .map(|(line, _)| line.clone())
    }

    /// The verbs no line is registered for on purpose: every row the table
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

    /// How many rows of each tier: fonts, whole, twin, host.
    pub(crate) fn counts(&self) -> (usize, usize, usize, usize) {
        let of = |tier| self.entries.values().filter(|e| e.tier == tier).count();
        (
            of(Tier::Fonts),
            of(Tier::Whole),
            of(Tier::Twin),
            of(Tier::Host),
        )
    }

    /// What is wrong with the table against the registered `lines`. The
    /// failure this is for is a lean that moved: moving the pre-merge line
    /// a whole line names to the whole gate too would silently leave its
    /// claim with no witness before a merge.
    pub(crate) fn complaints(&self, lines: &Lines) -> Vec<String> {
        let recorded: BTreeSet<&str> = lines.lines().map(String::as_str).collect();
        let before_merge = |line: &str| {
            recorded.contains(line)
                && matches!(self.tier(line), None | Some(Tier::Fonts | Tier::Host))
        };
        let mut out = self.unread.clone();
        for (line, entry) in &self.entries {
            match entry.tier {
                Tier::Fonts | Tier::Whole | Tier::Host if !recorded.contains(line.as_str()) => out
                    .push(format!(
                        "{FILE} names {line:?}, which {} does not hold — a renamed or dropped \
                     line: follow it or take the row out",
                        super::lines::FILE
                    )),
                Tier::Host if !entry.why.starts_with("git:") => out.push(format!(
                    "{line:?} is a host row, whose reason starts with `git:` — the git the \
                     container lacks"
                )),
                Tier::Twin if recorded.contains(line.as_str()) => out.push(format!(
                    "{line:?} is registered in {}, and {FILE} says it is the same run as {:?} \
                     — drop one of the two rows",
                    super::lines::FILE,
                    entry.leans.first().map_or("", String::as_str)
                )),
                _ => {}
            }
            match entry.tier {
                Tier::Fonts if !entry.leans.is_empty() => {
                    out.push(format!("{line:?} is a fonts row and leans on nothing"));
                }
                Tier::Whole => {
                    let kind = KINDS.iter().find(|kind| entry.why.starts_with(**kind));
                    match kind {
                        None => out.push(format!(
                            "{line:?}: a whole row's reason starts with one of {KINDS:?}"
                        )),
                        Some(kind) if entry.leans.is_empty() && !STANDS_ALONE.contains(kind) => {
                            out.push(format!(
                                "{line:?} waits for the whole gate on a {kind} reason and names no \
                                 pre-merge line walking its path — name one, or run it before a merge"
                            ));
                        }
                        Some(_) => {}
                    }
                    for lean in entry.leans.iter().filter(|lean| !before_merge(lean)) {
                        out.push(format!(
                            "{line:?} waits for the whole gate leaning on {lean:?}, which is not a \
                             registered pre-merge line — keep one of them before the merge"
                        ));
                    }
                }
                Tier::Twin => match entry.leans.as_slice() {
                    [of] if recorded.contains(of.as_str()) && self.tier(of) != Some(Tier::Twin) => {
                    }
                    _ => out.push(format!(
                        "{line:?} is a twin of {:?}, which has to be one registered line that is \
                         no twin itself",
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
    use super::{Lines, Tier, Tiers};

    fn registered(lines: &[&str]) -> Lines {
        let mut table = Lines::default();
        for line in lines {
            table
                .entries
                .insert((*line).to_string(), Default::default());
        }
        table
    }

    const TABLE: &str = "# a comment\n\
        fonts\tbadges --preset conflict\t-\tcap= reads the band's measured floor\n\
        whole\tfile-menu b.txt\tdiscard-many-go c.txt\tpicture: the menu standing\n\
        twin\tdiscard-file b.txt\tfile-menu b.txt\tthe same run\n\
        host\tpush-lease\t-\tgit: --force-if-includes needs 2.30\n";

    /// A line the table does not name is the host's before a merge and
    /// both sides' in the whole gate; each tier moves one of those four.
    #[test]
    fn each_tier_moves_what_a_gate_owes() {
        let tiers = Tiers::parse(TABLE);
        let pre_merge = "stash --preset basic";
        assert!(tiers.owed(pre_merge, false) && !tiers.on_linux(pre_merge, false));
        assert!(tiers.owed(pre_merge, true) && tiers.on_linux(pre_merge, true));
        let fonts = "badges --preset conflict";
        assert_eq!(tiers.tier(fonts), Some(Tier::Fonts));
        assert!(tiers.owed(fonts, false) && tiers.on_linux(fonts, false));
        let whole = "file-menu b.txt";
        assert!(!tiers.owed(whole, false));
        assert!(tiers.owed(whole, true) && tiers.on_linux(whole, true));
        let twin = "discard-file b.txt";
        assert!(!tiers.owed(twin, false) && !tiers.owed(twin, true));
        assert_eq!(tiers.twin_of(twin), Some("file-menu b.txt"));
        let host = "push-lease";
        assert_eq!(tiers.tier(host), Some(Tier::Host));
        assert!(tiers.owed(host, false) && !tiers.on_linux(host, false));
        assert!(tiers.owed(host, true) && !tiers.on_linux(host, true));
        assert_eq!(tiers.counts(), (1, 1, 1, 1));
    }

    /// A host row runs before a merge, so a whole row may lean on it; its
    /// reason has to say which git the container lacks.
    #[test]
    fn a_host_row_is_a_witness_before_a_merge_and_names_the_git_it_needs() {
        let lines = registered(&[
            "badges --preset conflict",
            "file-menu b.txt",
            "discard-many-go c.txt",
            "push-lease",
            "push-lease-far",
        ]);
        let leaning =
            format!("{TABLE}whole\tpush-lease-far\tpush-lease\tgit: the far side answers\n");
        assert_eq!(
            Tiers::parse(&leaning).complaints(&lines),
            Vec::<String>::new()
        );
        let wordless = format!("{TABLE}host\tpush-lease-far\t-\tthe far side\n");
        let said = Tiers::parse(&wordless).complaints(&lines);
        assert!(
            said.len() == 1 && said[0].contains("starts with `git:`"),
            "{said:?}"
        );
    }

    #[test]
    fn a_table_that_agrees_with_the_registered_lines_has_nothing_to_say() {
        let tiers = Tiers::parse(TABLE);
        let held = registered(&[
            "badges --preset conflict",
            "file-menu b.txt",
            "discard-many-go c.txt",
            "push-lease",
        ]);
        assert_eq!(tiers.complaints(&held), Vec::<String>::new());
    }

    #[test]
    fn a_moved_lean_a_returned_twin_and_a_gone_line_are_each_named() {
        let said = |table: &str, lines: &[&str]| Tiers::parse(table).complaints(&registered(lines));
        let gone = said(
            TABLE,
            &["file-menu b.txt", "discard-many-go c.txt", "push-lease"],
        );
        assert!(gone.len() == 1 && gone[0].contains("badges"), "{gone:?}");
        let back = said(
            TABLE,
            &[
                "badges --preset conflict",
                "file-menu b.txt",
                "discard-many-go c.txt",
                "discard-file b.txt",
                "push-lease",
            ],
        );
        assert!(
            back.len() == 1 && back[0].contains("is registered"),
            "{back:?}"
        );
        let moved = said(
            &format!("{TABLE}whole\tdiscard-many-go c.txt\t-\theld: a moved lean\n"),
            &[
                "badges --preset conflict",
                "file-menu b.txt",
                "discard-many-go c.txt",
                "push-lease",
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
        let unread = said("whole\tonly two fields\n", &[]);
        assert!(
            unread.iter().any(|c| c.contains("four fields")),
            "{unread:?}"
        );
        // A row spelled with a word the table no longer has is no row of
        // any tier: it is named, never read as the host's before a merge.
        for word in ["full", "linux"] {
            let row = format!("{word}\tfile-menu b.txt\tstash\tpicture: the menu\n");
            let unknown = said(&row, &["file-menu b.txt", "stash"]);
            assert!(
                unknown.iter().any(|c| c.contains("no tier")),
                "{word}: {unknown:?}"
            );
        }
    }

    #[test]
    fn a_whole_row_names_its_pre_merge_line_unless_something_cheaper_holds_it() {
        let said = |row: &str| {
            Tiers::parse(&format!("{TABLE}{row}\n")).complaints(&registered(&[
                "badges --preset conflict",
                "file-menu b.txt",
                "discard-many-go c.txt",
                "push",
                "push-lease",
            ]))
        };
        for kind in ["git", "picture", "other"] {
            let bare = said(&format!("whole\tpush\t-\t{kind}: git answers"));
            assert!(
                bare.len() == 1 && bare[0].contains("names no pre-merge line"),
                "{kind}: {bare:?}"
            );
        }
        for kind in ["held", "perf"] {
            let alone = said(&format!("whole\tpush\t-\t{kind}: a unit holds it"));
            assert_eq!(alone, Vec::<String>::new(), "{kind}");
        }
        let unsorted = said("whole\tpush\tdiscard-many-go c.txt\tbecause");
        assert!(
            unsorted.len() == 1 && unsorted[0].contains("starts with one of"),
            "{unsorted:?}"
        );
    }

    #[test]
    fn a_verb_is_twinned_away_only_onto_other_verbs() {
        let tiers = Tiers::parse(&format!(
            "{TABLE}twin\ttab-pin 0\ttab-pin\tthe same run\n\
             whole\tdiscard-many c.txt\tdiscard-many-go c.txt\tpicture: the menu\n\
             twin\tdiscard-many b.txt\tfile-menu b.txt\tthe same run\n"
        ));
        assert_eq!(
            tiers.twinned_away().into_iter().collect::<Vec<_>>(),
            ["discard-file"]
        );
    }

    /// Each way the table fails to read is its own refusal, named by
    /// path; none is an empty table.
    #[test]
    fn a_table_that_cannot_be_read_is_refused_by_path_and_cause() {
        let root = crate::verify::claim_dir(&crate::temp::root().join("pgg-tiers"), "load")
            .expect("a root of this test's own");
        let path = root.join(super::FILE);
        std::fs::create_dir_all(path.parent().expect("crates/xtask")).expect("xtask dir");
        let said = super::FILE;

        let gone = Tiers::load(&root).err().expect("no table");
        assert!(gone.contains(said) && gone.contains("os error 2"), "{gone}");

        std::fs::write(&path, b"fonts\tstash --preset basic\t-\t\xff\n").expect("bytes");
        let not_text = Tiers::load(&root).err().expect("not text");
        assert!(
            not_text.contains(said) && not_text.contains("UTF-8"),
            "{not_text}"
        );

        std::fs::remove_file(&path).expect("rm");
        std::fs::create_dir(&path).expect("a directory in its place");
        let unreadable = Tiers::load(&root).err().expect("a directory");
        assert!(unreadable.contains(said), "{unreadable}");
        assert!(unreadable != gone && unreadable != not_text, "{unreadable}");

        std::fs::remove_dir(&path).expect("rmdir");
        std::fs::write(&path, TABLE).expect("table");
        let read = Tiers::load(&root).expect("the table");
        assert_eq!(read.counts(), (1, 1, 1, 1));
        let _ = std::fs::remove_dir_all(&root);
    }
}
