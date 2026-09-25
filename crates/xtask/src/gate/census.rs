//! The verb census: which QML components each verify-ui line leaves
//! standing, recorded by the runs themselves.
//!
//! The harness walks the window's item tree as the run finishes and
//! reports every QML type it met (`WindowCensus.qml`); a passing run
//! writes them here against the line that ran it. The gate owes a verb
//! for a QML change when the verb's census names a file the change
//! reaches; a file the census never names is one no verb shows, and the
//! gate says so.
//!
//! A run whose page had stopped arriving rewrites its line whole: its walk
//! is a function of the build and the verb, and names no run reproduces
//! would select verbs for, and call covered, components nothing brings up
//! any more.
//!
//! A run whose page was still arriving (`WindowCensus.pageSettled`) only
//! adds to its line: its walk can say what it met, not that the rest is
//! gone, and writing it whole would move a checked-in file with the
//! machine's timing — a commit of the flutter on every gate.
//!
//! Only reproducible lines are recorded: a run against a `--repo` of this
//! machine names nothing another machine has.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::graph::{collect, stem_of};

/// Where the census lives, checked in so a fresh clone knows what the
/// verbs show. Rewritten by runs only.
pub(crate) const FILE: &str = "crates/xtask/verb-census.txt";

const HEADER: &str = "\
# The verb census — written by `cargo xtask verify-ui`, read by `cargo xtask gate`.
#
# One line per verify-ui argument line that passed, followed by a tab and
# the QML components (ui/ and auto/, by file name) the window was showing
# as the run finished. The gate owes a change every verb whose census
# names a file the change reaches. A run whose page had stopped arriving
# rewrites its own line and one whose page was still arriving adds to it,
# and a name whose file is gone leaves every line on the next write.
# Never edited by hand: run the verb instead, and this file follows.
";

#[derive(Default)]
pub(crate) struct Census {
    /// verify-ui line -> the component names its run met.
    pub lines: BTreeMap<String, BTreeSet<String>>,
}

impl Census {
    /// The census as the tree holds it. Only a file that is not there
    /// reads as empty (a tree before its first run): every writer rewrites
    /// the whole file from what it read ([`record`]), so a file read as
    /// empty or partial would be written back that way.
    pub(crate) fn load(root: &Path) -> Result<Census, String> {
        Census::read(&bytes(root)?)
    }

    /// The census the file's bytes say — refused where they are not text or
    /// hold a row no run wrote ([`Census::parse`]).
    pub(crate) fn read(bytes: &[u8]) -> Result<Census, String> {
        let text = std::str::from_utf8(bytes).map_err(|e| {
            let row = bytes[..e.valid_up_to()]
                .iter()
                .filter(|byte| **byte == b'\n')
                .count()
                + 1;
            format!("{FILE}:{row}: not UTF-8 — {PUT_BACK}")
        })?;
        Census::parse(text)
    }

    /// `<verify-ui line> TAB <names, space-separated>` per row, `#` starting
    /// a comment. A row of any other shape, and a line held twice, is named
    /// by its row: dropped or taken last, it would leave the file on the
    /// next write.
    fn parse(text: &str) -> Result<Census, String> {
        let mut census = Census::default();
        let mut first_at: BTreeMap<&str, usize> = BTreeMap::new();
        let mut wrong = Vec::new();
        for (at, row) in text.lines().enumerate() {
            let at = at + 1;
            if row.starts_with('#') || row.trim().is_empty() {
                continue;
            }
            let Some((key, names)) = row.split_once('\t').filter(|(key, _)| !key.is_empty()) else {
                wrong.push(format!(
                    "{FILE}:{at}: not `<verify-ui line> TAB <names>`: {row:?}"
                ));
                continue;
            };
            if let Some(first) = first_at.get(key) {
                wrong.push(format!("{FILE}:{at}: {key:?} again (first at row {first})"));
                continue;
            }
            first_at.insert(key, at);
            census.lines.insert(
                key.to_string(),
                names.split_whitespace().map(str::to_string).collect(),
            );
        }
        if wrong.is_empty() {
            return Ok(census);
        }
        Err(format!(
            "{FILE} holds rows no run wrote, so nothing may choose verbs off it or write it \
             back — {PUT_BACK}:\n{}",
            wrong
                .iter()
                .map(|row| format!("  {row}"))
                .collect::<Vec<_>>()
                .join("\n")
        ))
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
        // Whole or not at all: a run killed mid-write would leave half a
        // census, read as every later line gone. Staged under target/,
        // where git does not look, and renamed into place.
        let staging = root.join("target").join("verb-census.txt.part");
        if let Some(dir) = staging.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&staging, text).map_err(|e| format!("{}: {e}", staging.display()))?;
        std::fs::rename(&staging, &path).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// What to do about a census that cannot be read whole.
const PUT_BACK: &str = "the file is generated: put a committed copy of it back and let the runs \
                        write their lines again";

/// The file's bytes as the tree holds them: none for a tree with no census
/// yet, and an error for a file that is there and cannot be read.
pub(crate) fn bytes(root: &Path) -> Result<Vec<u8>, String> {
    match std::fs::read(root.join(FILE)) {
        Ok(bytes) => Ok(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("{FILE}: {e}")),
    }
}

/// How many verb lines a moved name is named by before the row counts
/// them instead ([`Shift`]).
const NAMED_AT_MOST: usize = 6;

/// What a write did to the census, read as sets. A component the app
/// gained rewrites every line, and the one line that gained a name by
/// itself (a verb landing somewhere new) is lost in that diff; so a name
/// every line moved is one row that counts them, and a name a handful of
/// lines moved is a row that names the verbs.
#[derive(Default)]
pub(crate) struct Shift {
    /// name -> the verb lines whose run met it and had not before.
    gained: BTreeMap<String, BTreeSet<String>>,
    /// name -> the verb lines that had met it and did not this time.
    lost: BTreeMap<String, BTreeSet<String>>,
    /// Verb lines the write put in, against how many names each brought;
    /// the line itself says it gained them all, so they get no rows.
    put_in: BTreeMap<String, usize>,
    /// Verb lines the write took away, whose names get no rows either.
    took_away: BTreeSet<String>,
    /// How many lines the census holds now, which the counts read against.
    lines: usize,
}

impl Shift {
    /// The sets one census holds against another's.
    pub(crate) fn between(before: &Census, after: &Census) -> Shift {
        let mut shift = Shift {
            lines: after.lines.len(),
            ..Shift::default()
        };
        for (line, names) in &after.lines {
            let Some(held) = before.lines.get(line) else {
                shift.put_in.insert(line.clone(), names.len());
                continue;
            };
            for name in names.difference(held) {
                shift
                    .gained
                    .entry(name.clone())
                    .or_default()
                    .insert(line.clone());
            }
            for name in held.difference(names) {
                shift
                    .lost
                    .entry(name.clone())
                    .or_default()
                    .insert(line.clone());
            }
        }
        for line in before.lines.keys() {
            if !after.lines.contains_key(line) {
                shift.took_away.insert(line.clone());
            }
        }
        shift
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.gained.is_empty()
            && self.lost.is_empty()
            && self.put_in.is_empty()
            && self.took_away.is_empty()
    }

    /// Every name that moved, as `(mark, name, the lines it moved on)`,
    /// fewest lines first, so the flutter reads before the bulk.
    fn moves(&self) -> Vec<(char, &String, &BTreeSet<String>)> {
        let mut moves: Vec<(char, &String, &BTreeSet<String>)> = self
            .gained
            .iter()
            .map(|(name, lines)| ('+', name, lines))
            .chain(self.lost.iter().map(|(name, lines)| ('-', name, lines)))
            .collect();
        moves.sort_by(|one, two| (one.2.len(), one.1, one.0).cmp(&(two.2.len(), two.1, two.0)));
        moves
    }

    /// Where a name moved: the verbs up to [`NAMED_AT_MOST`], else a count
    /// of the census's lines — `all` when it is every line, so a new
    /// component is told from a partial move without counting.
    fn spread(&self, lines: &BTreeSet<String>) -> Vec<String> {
        if lines.len() <= NAMED_AT_MOST {
            return lines.iter().cloned().collect();
        }
        if lines.len() == self.lines {
            return vec![format!("all {} lines", self.lines)];
        }
        vec![format!("{} of {} lines", lines.len(), self.lines)]
    }

    /// The write as a block of the gate's record: one row per name that
    /// moved, and one per line put in or taken away.
    pub(crate) fn block(&self) -> String {
        if self.is_empty() {
            // No line and nothing moved is a run that never read the
            // census (a dry run).
            if self.lines == 0 {
                return String::new();
            }
            return format!("  census  no line moved ({} line(s))\n", self.lines);
        }
        let mut rows: Vec<(String, String)> = Vec::new();
        // Past a handful the header's count is the whole answer: a tree
        // with no census yet puts every line in at once.
        if self.put_in.len() <= NAMED_AT_MOST {
            for (line, names) in &self.put_in {
                rows.push((String::from("+line"), format!("{line} ({names} name(s))")));
            }
        }
        if self.took_away.len() <= NAMED_AT_MOST {
            for line in &self.took_away {
                rows.push((String::from("-line"), line.clone()));
            }
        }
        for (mark, name, lines) in self.moves() {
            let head = format!("{mark}{name}");
            for (row, where_) in self.spread(lines).into_iter().enumerate() {
                rows.push((
                    if row == 0 {
                        head.clone()
                    } else {
                        String::new()
                    },
                    where_,
                ));
            }
        }
        let width = rows.iter().map(|(head, _)| head.len()).max().unwrap_or(0);
        let mut out = format!(
            "  census  {} name(s) moved over {} line(s){}\n",
            self.gained.len() + self.lost.len(),
            self.lines,
            self.lines_said(),
        );
        for (head, where_) in rows {
            out.push_str(&format!("          {head:width$}  {where_}\n"));
        }
        out
    }

    /// What the header says of whole lines, when there were any.
    fn lines_said(&self) -> String {
        let mut said = String::new();
        if !self.put_in.is_empty() {
            said.push_str(&format!(", {} line(s) put in", self.put_in.len()));
        }
        if !self.took_away.is_empty() {
            said.push_str(&format!(", {} taken away", self.took_away.len()));
        }
        said
    }

    /// What the write did to one verb's line, in a clause a run of that
    /// verb can print beside its own count: `(+Theme)`, `(+A -B)`, and
    /// nothing when no name moved anywhere.
    ///
    /// A name that moved on other lines too says how many, so the run can
    /// tell its own verb's move from a component the app gained or lost.
    pub(crate) fn said_for(&self, line: &str) -> String {
        let mut own: Vec<String> = Vec::new();
        let mut beside: Vec<String> = Vec::new();
        for (mark, name, lines) in self.moves() {
            let others = lines.iter().filter(|held| *held != line).count();
            if !lines.contains(line) {
                beside.push(format!("{mark}{name} on {others} other line(s)"));
            } else if others == 0 {
                own.push(format!("{mark}{name}"));
            } else {
                own.push(format!("{mark}{name} on {} lines", lines.len()));
            }
        }
        if own.is_empty() && beside.is_empty() {
            return String::new();
        }
        let own = if own.is_empty() {
            String::from("this line unchanged")
        } else {
            own.join(" ")
        };
        if beside.is_empty() {
            return format!(" ({own})");
        }
        format!(" ({own}; {})", beside.join(", "))
    }
}

/// Records what a passing run of `line` showed. Names that are no QML
/// file of the app (C++ types, inline components, files since removed)
/// are dropped from every line; what is left replaces the line — or, with
/// `page_settled` false, is added to it (module doc).
///
/// Answers how many names the line holds and what the write moved
/// ([`Shift`]), or why nothing was written: a census that cannot be read
/// whole ([`Census::load`]) is left as it is.
pub(crate) fn record(
    root: &Path,
    line: &str,
    names: &[String],
    page_settled: bool,
) -> Result<(usize, Shift), String> {
    let _turn = one_writer(root)?;
    let known = component_files(root)?;
    let before = Census::load(root)?;
    let mut census = Census {
        lines: before.lines.clone(),
    };
    let mut shown: BTreeSet<String> = names
        .iter()
        .filter(|n| known.contains(*n))
        .cloned()
        .collect();
    if !page_settled && let Some(held) = census.lines.get(line) {
        shown.extend(held.iter().cloned());
    }
    let count = shown.len();
    census.lines.insert(line.to_string(), shown);
    for names in census.lines.values_mut() {
        names.retain(|name| known.contains(name));
    }
    census.save(root)?;
    let shift = Shift::between(&before, &census);
    Ok((count, shift))
}

/// The file under one writer at a time, for as long as the guard lives:
/// the gate runs verbs several at once (`gate::sides::verbs`), and two
/// read-modify-writes at once would each lose the other's line. The lock
/// lives under target/, since a file beside the census would be a change
/// the gate refuses to run over.
fn one_writer(root: &Path) -> Result<crate::locks::Locked, String> {
    let dir = root.join("target");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join("verb-census.lock");
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    file.lock()
        .map_err(|e| format!("could not hold {}: {e}", path.display()))?;
    Ok(crate::locks::Locked::new(file))
}

/// The names of every QML component file of the app, product and
/// harness alike.
fn component_files(root: &Path) -> Result<BTreeSet<String>, String> {
    Ok(super::graph::qml_files(root)?
        .iter()
        .map(|f| stem_of(f))
        .collect())
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
    // An unreadable root still counts: only a `QtObject` root is ruled out.
    root_type(root, file).unwrap_or_default() != "QtObject"
}

/// What a component's own object is: the first word of the first line
/// that opens one — `Item {`, `QtObject {`, `AppCard {`.
pub(crate) fn root_type(root: &Path, file: &str) -> Option<String> {
    let text = std::fs::read_to_string(root.join(file)).ok()?;
    let code = super::graph::strip_comments(&text);
    code.lines()
        .map(str::trim)
        .find(|line| line.ends_with('{') && !line.starts_with("import") && !line.contains(':'))
        .and_then(|line| line.split_whitespace().next())
        .map(str::to_string)
}

/// Who wears whom: the stem of every component that is another one's root
/// type, against the stems that wear it.
///
/// The item tree answers with the outermost type only — a `GraphFind`
/// never says the `FindBar` it is one of — so a component used only as a
/// root is never named by a run, and both questions the census answers
/// (which verbs show this file, and whether any does) go through the
/// wearer, read off the sources.
pub(crate) fn worn_by(root: &Path) -> BTreeMap<String, BTreeSet<String>> {
    let mut worn: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut files = Vec::new();
    if collect(
        root,
        &root.join("crates/platitude-app/src"),
        "qml",
        &mut files,
    )
    .is_err()
    {
        return worn;
    }
    for file in &files {
        let stem = stem_of(file);
        let Some(kind) = root_type(root, file) else {
            continue;
        };
        if kind != stem {
            worn.entry(kind).or_default().insert(stem);
        }
    }
    worn
}

/// `stem` and everyone wearing it, however many layers deep.
pub(crate) fn through_wearers(
    stem: &str,
    worn: &BTreeMap<String, BTreeSet<String>>,
    out: &mut BTreeSet<String>,
) {
    if !out.insert(stem.to_string()) {
        return;
    }
    for wearer in worn.get(stem).into_iter().flatten() {
        through_wearers(wearer, worn, out);
    }
}

/// Whether the run photographed a page that had stopped arriving: the
/// `census page=` line the walk says before the names.
///
/// A run that said nothing is read as still arriving — the answer that
/// can only add names.
pub(crate) fn page_settled_in(lines: &[String]) -> bool {
    lines
        .iter()
        .rev()
        .find_map(|line| line.split("census page=").nth(1))
        .and_then(|rest| rest.split_whitespace().next())
        .is_some_and(|word| word == "settled")
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
    use super::{Census, FILE, Shift, names_in, page_settled_in, record};
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    /// A census of the lines given, written the way the file is.
    fn census_of(lines: &[(&str, &[&str])]) -> Census {
        let text: String = lines
            .iter()
            .map(|(line, names)| format!("{line}\t{}\n", names.join(" ")))
            .collect();
        Census::parse(&text).expect("a census as the file is written")
    }

    /// Eight verbs showing the same two components — more than
    /// `NAMED_AT_MOST`, so a name every one of them moved is counted.
    fn eight_lines() -> Census {
        census_of(
            &VERBS
                .iter()
                .map(|verb| (*verb, &["AppCard", "Main"][..]))
                .collect::<Vec<_>>(),
        )
    }

    /// The census those eight verbs make when each shows what `showing`
    /// gives its place.
    fn eight_showing(showing: impl Fn(usize) -> &'static [&'static str]) -> Census {
        census_of(
            &VERBS
                .iter()
                .enumerate()
                .map(|(n, verb)| (*verb, showing(n)))
                .collect::<Vec<_>>(),
        )
    }

    /// A block's rows with the column padding taken out, so a test says
    /// what a row holds and not how wide the widest name beside it was.
    fn rows(block: &str) -> Vec<String> {
        block
            .lines()
            .map(|row| row.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect()
    }

    const VERBS: [&str; 8] = [
        "commit --preset basic",
        "diff-file notes.txt",
        "graph --preset tags",
        "op-exit-go continue --preset rebase-staged",
        "stash --preset basic",
        "switch --preset branches",
        "wip --preset dirty",
        "zoom --preset wide",
    ];

    #[test]
    fn a_name_every_line_moved_is_one_row_and_a_name_few_lines_moved_names_the_verbs() {
        // A component the app gained, met by every run; one verb landing
        // somewhere new, which is the flutter; and a component two verbs
        // stopped showing.
        let after = eight_showing(|n| match n {
            3 => &["AppCard", "DiffReach", "FileRowDelegate", "Main"],
            0 | 1 => &["DiffReach", "Main"],
            _ => &["AppCard", "DiffReach", "Main"],
        });
        let block = Shift::between(&eight_lines(), &after).block();
        assert_eq!(
            rows(&block),
            [
                "census 3 name(s) moved over 8 line(s)",
                // The flutter first.
                "+FileRowDelegate op-exit-go continue --preset rebase-staged",
                "-AppCard commit --preset basic",
                "diff-file notes.txt",
                // The bulk last, as a count.
                "+DiffReach all 8 lines",
            ],
            "{block}"
        );
    }

    #[test]
    fn a_name_a_subset_moved_is_counted_against_the_whole() {
        let after = eight_showing(|n| {
            if n < 7 {
                &["Main"]
            } else {
                &["AppCard", "Main"]
            }
        });
        let block = Shift::between(&eight_lines(), &after).block();
        assert_eq!(
            rows(&block),
            [
                "census 1 name(s) moved over 8 line(s)",
                "-AppCard 7 of 8 lines",
            ],
            "past a handful the verbs are counted, and a subset is not all: {block}"
        );
    }

    #[test]
    fn every_line_put_in_at_once_is_the_headers_count_and_no_rows() {
        // A tree with no census yet.
        let block = Shift::between(&Census::default(), &eight_lines()).block();
        assert_eq!(
            rows(&block),
            ["census 0 name(s) moved over 8 line(s), 8 line(s) put in"],
            "{block}"
        );
    }

    #[test]
    fn a_line_put_in_or_taken_away_is_its_own_row_and_not_its_names() {
        let before = census_of(&[("wip --preset dirty", &["AppCard", "Main"])]);
        let after = census_of(&[("diff-file notes.txt", &["AppCard", "DiffPane", "Main"])]);
        let block = Shift::between(&before, &after).block();
        assert_eq!(
            rows(&block),
            [
                "census 0 name(s) moved over 1 line(s), 1 line(s) put in, 1 taken away",
                "+line diff-file notes.txt (3 name(s))",
                "-line wip --preset dirty",
            ],
            "a line that was not there gained all of its names, which the line itself \
             says: {block}"
        );
    }

    #[test]
    fn a_census_that_did_not_move_says_so_and_one_never_read_says_nothing() {
        let held = eight_lines();
        let still = Shift::between(&held, &held);
        assert!(still.is_empty());
        assert_eq!(still.block(), "  census  no line moved (8 line(s))\n");
        assert_eq!(still.said_for("wip --preset dirty"), "");
        assert_eq!(
            Shift::default().block(),
            "",
            "a dry run read no census and has nothing to say of one"
        );
    }

    /// A root holding nothing but the QML files a census may name: a
    /// checkout of one test's own, with no census in it yet.
    ///
    /// Claimed, not named by pid and counter: pids come round, these roots
    /// are never cleaned up, and a census an earlier holder left would be
    /// read back as this test's own.
    fn root_with(components: &[&str]) -> PathBuf {
        let root = crate::verify::claim_dir(&std::env::temp_dir().join("pgg-census"), "root")
            .expect("a root of this test's own");
        let ui = root.join("crates/platitude-app/src/ui");
        std::fs::create_dir_all(&ui).expect("ui dir");
        std::fs::create_dir_all(root.join("crates/xtask")).expect("xtask dir");
        for name in components {
            std::fs::write(ui.join(format!("{name}.qml")), "Item {}\n").expect("component");
        }
        root
    }

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
            "2026-09-02T01:18:33Z  INFO bench: census page=settled".to_string(),
            "2026-09-02T01:18:33Z  INFO bench: census=AppCard,DiffPane,Main".to_string(),
        ];
        assert_eq!(
            names_in(&lines),
            Some(vec!["AppCard".into(), "DiffPane".into(), "Main".into()]),
            "the page line is no name, and stands in front of the names"
        );
        assert!(page_settled_in(&lines));
        assert_eq!(names_in(&[]), None);
    }

    #[test]
    fn the_terms_beside_the_answer_are_neither_a_name_nor_a_second_answer() {
        // Terms ride beside the answer (`WindowCensus.terms`): the answer
        // is the first word, and the `census page=` line is no `census=`.
        let lines = vec![
            "INFO bench: census page=settled waited=true state=open finish=2 failed=false \
             stale=false wipRow=true wipRowStands=true"
                .to_string(),
            "INFO bench: census=AppCard,WipTallyRow".to_string(),
        ];
        assert!(page_settled_in(&lines));
        assert_eq!(
            names_in(&lines),
            Some(vec!["AppCard".into(), "WipTallyRow".into()])
        );
    }

    #[test]
    fn a_run_that_said_nothing_of_the_page_is_read_as_one_still_arriving() {
        let arriving = ["2026-09-02T01:18:33Z  INFO bench: census page=arriving".to_string()];
        assert!(!page_settled_in(&arriving));
        assert!(!page_settled_in(&[]));
    }

    #[test]
    fn a_run_writes_its_line_over_the_one_before_it() {
        let root = root_with(&["DiffPane", "GraphPane", "WipPane"]);
        record(
            &root,
            "wip --preset dirty",
            &["GraphPane".into(), "WipPane".into()],
            true,
        )
        .expect("first");
        // The verb stopped showing the graph and started showing the
        // diff: what it shows now is the whole of its line.
        let (count, shift) = record(
            &root,
            "wip --preset dirty",
            &["DiffPane".into(), "WipPane".into(), "QQuickText".into()],
            true,
        )
        .expect("second");
        assert_eq!(count, 2, "a name that is no QML file of the app is dropped");
        assert_eq!(
            shift.said_for("wip --preset dirty"),
            " (+DiffPane -GraphPane)",
            "the run says which names moved"
        );
        let census = Census::load(&root).expect("the census it wrote");
        assert_eq!(
            census.lines["wip --preset dirty"],
            ["DiffPane", "WipPane"].map(String::from).into()
        );
        assert!(!census.covers("GraphPane"), "no run shows it any more");
    }

    #[test]
    fn a_run_whose_page_was_still_arriving_only_adds() {
        let root = root_with(&["DiffPane", "GraphPane", "WipPane"]);
        record(
            &root,
            "band --system-title-bar",
            &["DiffPane".into(), "GraphPane".into()],
            true,
        )
        .expect("settled");
        // The same verb again, finished before the reads landed: what it
        // did not meet is still what the verb shows.
        let (count, shift) = record(
            &root,
            "band --system-title-bar",
            &["GraphPane".into(), "WipPane".into()],
            false,
        )
        .expect("arriving");
        assert_eq!(count, 3);
        assert_eq!(
            shift.said_for("band --system-title-bar"),
            " (+WipPane)",
            "a run that only adds says what it added"
        );
        let census = Census::load(&root).expect("the census it wrote");
        assert_eq!(
            census.lines["band --system-title-bar"],
            ["DiffPane", "GraphPane", "WipPane"]
                .map(String::from)
                .into()
        );
    }

    #[test]
    fn a_component_that_is_gone_leaves_every_line() {
        let root = root_with(&["DiffPane", "WipPane"]);
        record(&root, "wip --preset dirty", &["WipPane".into()], true).expect("wip");
        record(
            &root,
            "diff-file b.txt",
            &["DiffPane".into(), "WipPane".into()],
            true,
        )
        .expect("diff");
        std::fs::remove_file(root.join("crates/platitude-app/src/ui/WipPane.qml")).expect("remove");
        let (_, shift) =
            record(&root, "diff-file b.txt", &["DiffPane".into()], true).expect("again");
        assert_eq!(
            shift.said_for("diff-file b.txt"),
            " (-WipPane on 2 lines)",
            "a name the pruning took off every line says so on the line that ran"
        );
        let census = Census::load(&root).expect("the census it wrote");
        assert!(!census.covers("WipPane"));
        assert_eq!(
            census.lines["wip --preset dirty"],
            BTreeSet::new(),
            "the line the run did not touch is pruned too"
        );
    }

    #[test]
    fn a_census_that_cannot_be_read_whole_is_named_by_its_row_and_never_written() {
        let held = "wip --preset dirty\tWipPane\n";
        let mut not_text = format!("# census\n{held}").into_bytes();
        not_text.extend_from_slice(b"diff-file b.txt\tDiff\xffPane\n");
        for (what, bytes, says) in [
            ("not UTF-8", not_text, ":3: not UTF-8"),
            (
                "a row with no tab",
                format!("# census\n{held}<<<<<<< HEAD\n").into_bytes(),
                ":3: not `<verify-ui line> TAB <names>`",
            ),
            (
                "a line held twice",
                format!("{held}diff-file b.txt\tDiffPane\n{held}").into_bytes(),
                ":3: \"wip --preset dirty\" again (first at row 1)",
            ),
        ] {
            let root = root_with(&["DiffPane", "WipPane"]);
            let path = root.join(FILE);
            std::fs::write(&path, &bytes).expect("the census as found");
            let Err(read) = Census::load(&root) else {
                panic!("{what}: read as a census");
            };
            assert!(read.contains(&format!("{FILE}{says}")), "{what}: {read}");
            let Err(wrote) = record(&root, "diff-file b.txt", &["DiffPane".into()], true) else {
                panic!("{what}: a line was written over it");
            };
            assert!(wrote.contains(says), "{what}: {wrote}");
            assert_eq!(
                std::fs::read(&path).expect("the census after"),
                bytes,
                "{what}: the file is as it was found"
            );
        }
    }

    #[test]
    fn only_a_census_that_is_not_there_reads_as_empty() {
        let root = root_with(&["WipPane"]);
        assert!(
            Census::load(&root)
                .expect("no file is an empty census")
                .lines
                .is_empty()
        );
        let path = root.join(FILE);
        std::fs::create_dir(&path).expect("a directory where the census stands");
        let Err(read) = Census::load(&root) else {
            panic!("a census that cannot be opened read as an empty one");
        };
        assert!(read.contains(FILE), "{read}");
        std::fs::remove_dir(&path).expect("the directory gone");
        record(&root, "wip --preset dirty", &["WipPane".into()], true).expect("the first line");
        assert_eq!(Census::load(&root).expect("the first write").lines.len(), 1);
    }

    #[test]
    fn a_line_that_could_not_be_held_or_put_in_place_is_an_error_and_the_census_stands() {
        let root = root_with(&["DiffPane", "WipPane"]);
        record(&root, "wip --preset dirty", &["WipPane".into()], true).expect("a line before");
        let path = root.join(FILE);
        let before = std::fs::read(&path).expect("the census before");
        let staging = root.join("target").join("verb-census.txt.part");
        std::fs::create_dir(&staging).expect("a directory where the staging file goes");
        let Err(why) = record(&root, "diff-file b.txt", &["DiffPane".into()], true) else {
            panic!("a line was recorded with nowhere to stage it");
        };
        assert!(why.contains("verb-census.txt.part"), "{why}");
        assert_eq!(std::fs::read(&path).expect("the census"), before);

        std::fs::remove_dir_all(root.join("target")).expect("target gone");
        std::fs::write(root.join("target"), "").expect("a file where target/ goes");
        let Err(why) = record(&root, "diff-file b.txt", &["DiffPane".into()], true) else {
            panic!("a line was recorded with no lock to hold");
        };
        assert!(why.contains("target"), "{why}");
        assert_eq!(std::fs::read(&path).expect("the census"), before);
    }

    /// The read-modify-write under [`super::one_writer`].
    #[test]
    fn runs_recording_at_once_each_keep_their_line() {
        let root = root_with(&["WipPane"]);
        let lines: Vec<String> = (0..8).map(|n| format!("wip {n}")).collect();
        std::thread::scope(|scope| {
            for line in &lines {
                let root = &root;
                scope.spawn(move || record(root, line, &["WipPane".into()], true).expect("a line"));
            }
        });
        assert_eq!(
            Census::load(&root)
                .expect("the census")
                .lines
                .into_keys()
                .collect::<Vec<_>>(),
            lines
        );
    }
}
