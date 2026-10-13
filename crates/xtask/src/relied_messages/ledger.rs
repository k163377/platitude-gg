//! `crates/xtask/relied-messages.txt`: the trees read, and for each form of
//! the table the lines its sources name, grouped into eras — the trees
//! that hold the same lines. One record a line, tab-separated, so a
//! change between two audits is a diff a reader can follow.

use std::path::Path;

use super::LEDGER;
use super::sources::{Cache, Held, LINES, Line, Read, Tree};
use super::table::{Form, Source, Supplier};

/// The ledger as read or about to be written.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Ledger {
    /// Each line's trees, in order, its head last.
    read: Vec<(Line, Vec<String>)>,
    /// Each line's last tree: what the weekly run compares a head with.
    heads: Vec<Head>,
    entries: Vec<Entry>,
}

#[derive(Debug, PartialEq, Eq)]
struct Head {
    line: Line,
    name: String,
    oid: String,
    /// The package version (Ubuntu) or git release (Apple) it is.
    note: String,
}

#[derive(Debug, PartialEq, Eq)]
struct Entry {
    name: String,
    value: String,
    sources: Vec<Source>,
    eras: Vec<Era>,
}

#[derive(Debug, PartialEq, Eq)]
struct Era {
    members: Vec<(Line, String)>,
    held: Held,
}

impl Ledger {
    /// The ledger the trees just read make.
    pub(super) fn of(
        forms: &[Form],
        trees: &[Tree],
        read: &[Read],
        cache: &Cache,
    ) -> Result<Self, String> {
        let read_order: Vec<(Line, Vec<String>)> = LINES
            .into_iter()
            .map(|line| {
                let names = trees
                    .iter()
                    .filter(|tree| tree.line == line)
                    .map(|tree| tree.name.clone())
                    .collect();
                (line, names)
            })
            .filter(|(_, names): &(Line, Vec<String>)| !names.is_empty())
            .collect();
        let mut heads = Vec::new();
        for line in LINES {
            if let Some(tree) = trees.iter().rfind(|tree| tree.line == line) {
                heads.push(Head {
                    line,
                    name: tree.name.clone(),
                    oid: cache.oid(tree)?,
                    note: cache.describe(tree).unwrap_or_default(),
                });
            }
        }
        let entries = forms
            .iter()
            .enumerate()
            .map(|(index, form)| Entry {
                name: form.name.clone(),
                value: form.value.clone(),
                sources: form.sources.clone(),
                eras: eras_of(form, index, read),
            })
            .collect();
        Ok(Self {
            read: read_order,
            heads,
            entries,
        })
    }

    pub(super) fn read(root: &Path) -> Result<Self, String> {
        let text =
            std::fs::read_to_string(root.join(LEDGER)).map_err(|e| format!("{LEDGER}: {e}"))?;
        parse(&text).map_err(|e| format!("{LEDGER}: {e}"))
    }

    pub(super) fn render(&self) -> String {
        let mut rows = vec![
            "# git's words the product reads (crates/platitude-core/src/relied_messages.rs), as each tree"
                .to_string(),
            format!(
                "# read writes them: `{}` writes this file, `check` holds the table to it",
                super::AUDIT.line()
            ),
            "# and `watch` holds git's sources to it. Not edited by hand.".to_string(),
        ];
        for (line, names) in &self.read {
            rows.push(format!("read\t{}\t{}", line.word(), names.join(" ")));
        }
        for head in &self.heads {
            rows.push(format!(
                "head\t{}\t{}\t{}\t{}",
                head.line.word(),
                head.name,
                head.oid,
                escape(&head.note)
            ));
        }
        for entry in &self.entries {
            rows.push(String::new());
            rows.push(format!("form\t{}\t{}", entry.name, escape(&entry.value)));
            for source in &entry.sources {
                rows.push(format!(
                    "source\t{}\t{}\t{}",
                    source.supplier.word(),
                    escape(&source.path),
                    escape(&source.words)
                ));
            }
            for era in &entry.eras {
                rows.push(format!("era\t{}", self.members(&era.members)));
                for (path, line) in &era.held {
                    rows.push(format!("held\t{}\t{}", escape(path), escape(line)));
                }
            }
        }
        rows.push(String::new());
        rows.join("\n")
    }

    /// `upstream v2.43.0..v2.53.0 v2.55.0, gfw …`: each line's trees, runs
    /// of the line's order shortened to their ends.
    fn members(&self, members: &[(Line, String)]) -> String {
        let mut parts = Vec::new();
        for (line, order) in &self.read {
            let mut runs: Vec<(usize, usize)> = Vec::new();
            for (at, name) in order.iter().enumerate() {
                if !members.iter().any(|(l, n)| l == line && n == name) {
                    continue;
                }
                match runs.last_mut() {
                    Some(run) if run.1 + 1 == at => run.1 = at,
                    _ => runs.push((at, at)),
                }
            }
            if runs.is_empty() {
                continue;
            }
            let spans: Vec<String> = runs
                .iter()
                .map(|&(from, to)| match from == to {
                    true => order[from].clone(),
                    false => format!("{}..{}", order[from], order[to]),
                })
                .collect();
            parts.push(format!("{} {}", line.word(), spans.join(" ")));
        }
        parts.join(", ")
    }

    /// The forms the table and this ledger do not agree on: written apart,
    /// or the words or sources of one changed.
    pub(super) fn differs_from(&self, forms: &[Form]) -> Vec<String> {
        let mut differing: Vec<String> = forms
            .iter()
            .filter(|form| {
                !self.entries.iter().any(|entry| {
                    entry.name == form.name
                        && entry.value == form.value
                        && entry.sources == form.sources
                })
            })
            .map(|form| form.name.clone())
            .collect();
        differing.extend(
            self.entries
                .iter()
                .filter(|entry| !forms.iter().any(|form| form.name == entry.name))
                .map(|entry| format!("{} (gone from the table)", entry.name)),
        );
        differing
    }

    /// The commit the ledger read for `line`'s last tree.
    pub(super) fn head_oid(&self, line: Line) -> Option<&str> {
        self.heads
            .iter()
            .find(|head| head.line == line)
            .map(|head| head.oid.as_str())
    }

    /// `line`'s last tree read.
    pub(super) fn newest(&self, line: Line) -> Option<&str> {
        self.read
            .iter()
            .find(|(l, _)| *l == line)
            .and_then(|(_, names)| names.last())
            .map(String::as_str)
    }

    pub(super) fn has_read(&self, line: Line, name: &str) -> bool {
        self.read
            .iter()
            .any(|(l, names)| *l == line && names.iter().any(|n| n == name))
    }

    pub(super) fn read_summary(&self) -> String {
        self.read
            .iter()
            .map(|(line, names)| {
                let head = self.heads.iter().find(|head| head.line == *line);
                let note = head
                    .filter(|head| !head.note.is_empty())
                    .map(|head| format!(" ({})", head.note))
                    .unwrap_or_default();
                match names.as_slice() {
                    [only] => format!("{} {only}{note}", line.word()),
                    [first, .., last] => format!(
                        "{} {first}..{last} ({} trees){note}",
                        line.word(),
                        names.len()
                    ),
                    [] => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// A form whose source names nothing at its supplier's development
    /// head is a source written wrong.
    pub(super) fn unwritten_at_the_heads(&self) -> Result<(), String> {
        let unwritten: Vec<String> = self
            .entries
            .iter()
            .flat_map(|entry| {
                let suppliers: Vec<Supplier> =
                    entry.sources.iter().map(|source| source.supplier).collect();
                [(Line::Upstream, "master"), (Line::OpenSsh, "master")]
                    .into_iter()
                    .filter(move |(line, _)| suppliers.contains(&line.supplier()))
                    .filter(|(line, head)| {
                        entry
                            .era_of(*line, head)
                            .is_none_or(|era| era.held.is_empty())
                    })
                    .map(|(line, head)| format!("{} at {} {head}", entry.name, line.word()))
            })
            .collect();
        if unwritten.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "the table names words its sources do not hold: {} — put its `- git:` / `- openssh:` line right",
                unwritten.join(", ")
            ))
        }
    }

    /// The forms whose lines are not the same in every tree read, each
    /// with its eras — where a release changed them.
    pub(super) fn changes_summary(&self) -> String {
        let mut rows = Vec::new();
        for entry in self.entries.iter().filter(|entry| entry.eras.len() > 1) {
            rows.push(format!("{} — {} eras:", entry.name, entry.eras.len()));
            for era in &entry.eras {
                rows.push(format!(
                    "  {} line(s): {}",
                    era.held.len(),
                    self.members(&era.members)
                ));
            }
        }
        if rows.is_empty() {
            rows.push("every form's lines are the same in every tree read".to_string());
        }
        rows.join("\n")
    }

    /// What changed against this ledger: a head whose lines are not those
    /// of the line's head as last read, and a release it had not read
    /// whose lines are none it knows.
    pub(super) fn changes_in(
        &self,
        forms: &[Form],
        heads: &[Tree],
        new_releases: &[Tree],
        read: &[Read],
    ) -> Vec<Change> {
        let mut changes = Vec::new();
        for (index, form) in forms.iter().enumerate() {
            let Some(entry) = self.entries.iter().find(|e| e.name == form.name) else {
                continue;
            };
            for at in read {
                let held = &at.held[index];
                let tree = &at.tree;
                if !form
                    .sources
                    .iter()
                    .any(|source| source.supplier == tree.line.supplier())
                {
                    continue;
                }
                // The lines the line's head held as last read, to say what
                // a change changed from.
                let recorded = || {
                    self.heads
                        .iter()
                        .find(|head| head.line == tree.line)
                        .and_then(|head| entry.era_of(head.line, &head.name))
                        .map(|era| era.held.clone())
                        .unwrap_or_default()
                };
                let changed = if heads.contains(tree) {
                    let before = recorded();
                    (&before != held).then_some(before)
                } else if new_releases.contains(tree) {
                    // No era holds these lines — none at all among them,
                    // where every era held some.
                    (!entry.eras.iter().any(|era| &era.held == held)).then(recorded)
                } else {
                    None
                };
                if let Some(before) = changed {
                    changes.push(Change {
                        form: form.name.clone(),
                        index,
                        tree: tree.clone(),
                        before,
                        after: held.clone(),
                    });
                }
            }
        }
        changes
    }
}

impl Entry {
    fn era_of(&self, line: Line, name: &str) -> Option<&Era> {
        self.eras
            .iter()
            .find(|era| era.members.iter().any(|(l, n)| *l == line && n == name))
    }
}

/// A tree whose lines for a form are not the ones the ledger has for it.
pub(super) struct Change {
    form: String,
    index: usize,
    tree: Tree,
    before: Held,
    after: Held,
}

impl Change {
    pub(super) fn form(&self) -> &str {
        &self.form
    }

    /// The change, and which releases carry its lines now.
    pub(super) fn report(&self, every: &[Tree], read: &[Read], cache: &Cache) -> String {
        let oid = cache
            .oid(&self.tree)
            .map(|oid| format!(" ({oid})"))
            .unwrap_or_default();
        let mut rows = vec![format!(
            "{} changed in {}{oid}:",
            self.form,
            self.tree.label()
        )];
        for line in self.before.iter().filter(|line| !self.after.contains(line)) {
            rows.push(format!("  - {}: {}", line.0, line.1));
        }
        for line in self.after.iter().filter(|line| !self.before.contains(line)) {
            rows.push(format!("  + {}: {}", line.0, line.1));
        }
        let carrying: Vec<(Line, String)> = read
            .iter()
            .filter(|at| at.held[self.index] == self.after)
            .map(|at| (at.tree.line, at.tree.name.clone()))
            .collect();
        let order = Ledger {
            read: LINES
                .into_iter()
                .map(|line| {
                    let names = every
                        .iter()
                        .filter(|tree| tree.line == line)
                        .map(|tree| tree.name.clone())
                        .collect();
                    (line, names)
                })
                .collect(),
            heads: Vec::new(),
            entries: Vec::new(),
        };
        rows.push(format!(
            "  the trees that hold these lines now: {}",
            match carrying.is_empty() {
                true => "none".to_string(),
                false => order.members(&carrying),
            }
        ));
        rows.join("\n")
    }
}

/// The trees grouped by the lines they hold for one form, in the order
/// the trees were read.
fn eras_of(form: &Form, index: usize, read: &[Read]) -> Vec<Era> {
    let mut eras: Vec<Era> = Vec::new();
    for at in read {
        if !form
            .sources
            .iter()
            .any(|source| source.supplier == at.tree.line.supplier())
        {
            continue;
        }
        let held = &at.held[index];
        let member = (at.tree.line, at.tree.name.clone());
        match eras.iter_mut().find(|era| &era.held == held) {
            Some(era) => era.members.push(member),
            None => eras.push(Era {
                members: vec![member],
                held: held.clone(),
            }),
        }
    }
    eras
}

fn parse(text: &str) -> Result<Ledger, String> {
    let mut ledger = Ledger {
        read: Vec::new(),
        heads: Vec::new(),
        entries: Vec::new(),
    };
    for (at, row) in text.lines().enumerate() {
        if row.is_empty() || row.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = row.split('\t').collect();
        let wrong = || format!("line {}: {row}", at + 1);
        match fields.as_slice() {
            ["read", line, names] => {
                let line = Line::from_word(line).ok_or_else(wrong)?;
                ledger
                    .read
                    .push((line, names.split_whitespace().map(str::to_string).collect()));
            }
            ["head", line, name, oid, note] => ledger.heads.push(Head {
                line: Line::from_word(line).ok_or_else(wrong)?,
                name: (*name).to_string(),
                oid: (*oid).to_string(),
                note: unescape(note),
            }),
            ["form", name, value] => ledger.entries.push(Entry {
                name: (*name).to_string(),
                value: unescape(value),
                sources: Vec::new(),
                eras: Vec::new(),
            }),
            ["source", supplier, path, words] => {
                let entry = ledger.entries.last_mut().ok_or_else(wrong)?;
                entry.sources.push(Source {
                    supplier: Supplier::from_word(supplier).ok_or_else(wrong)?,
                    path: unescape(path),
                    words: unescape(words),
                });
            }
            ["era", members] => {
                let members = expand(&ledger.read, members).ok_or_else(wrong)?;
                let entry = ledger.entries.last_mut().ok_or_else(wrong)?;
                entry.eras.push(Era {
                    members,
                    held: Held::new(),
                });
            }
            ["held", path, line] => {
                let era = ledger
                    .entries
                    .last_mut()
                    .and_then(|entry| entry.eras.last_mut())
                    .ok_or_else(wrong)?;
                era.held.push((unescape(path), unescape(line)));
            }
            _ => return Err(wrong()),
        }
    }
    Ok(ledger)
}

/// [`Ledger::members`] read back against the read order.
fn expand(read: &[(Line, Vec<String>)], members: &str) -> Option<Vec<(Line, String)>> {
    let mut out = Vec::new();
    for part in members.split(", ").filter(|part| !part.is_empty()) {
        let mut words = part.split_whitespace();
        let line = Line::from_word(words.next()?)?;
        let order = &read.iter().find(|(l, _)| *l == line)?.1;
        for span in words {
            let (from, to) = span.split_once("..").unwrap_or((span, span));
            let from = order.iter().position(|name| name == from)?;
            let to = order.iter().position(|name| name == to)?;
            out.extend(order[from..=to].iter().map(|name| (line, name.clone())));
        }
    }
    Some(out)
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

fn unescape(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Era, Head, Held, Ledger, Line, Read, Tree, parse};
    use crate::relied_messages::table::{Form, Source, Supplier};

    fn source() -> Source {
        Source {
            supplier: Supplier::Git,
            path: "lockfile.c".to_string(),
            words: "Unable to create '%s.lock'".to_string(),
        }
    }

    fn ledger() -> Ledger {
        let names = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        Ledger {
            read: vec![
                (
                    Line::Upstream,
                    names(&["v2.43.0", "v2.43.1", "v2.44.0", "master"]),
                ),
                (Line::Ubuntu, names(&["noble-updates"])),
            ],
            heads: vec![Head {
                line: Line::Ubuntu,
                name: "noble-updates".to_string(),
                oid: "d7a6".to_string(),
                note: "2.43.0-1ubuntu7.3".to_string(),
            }],
            entries: vec![super::Entry {
                name: "LOCK".to_string(),
                value: ".lock'\tFile exists\n".to_string(),
                sources: vec![source()],
                eras: vec![
                    Era {
                        members: vec![
                            (Line::Upstream, "v2.43.0".to_string()),
                            (Line::Upstream, "v2.43.1".to_string()),
                            (Line::Upstream, "master".to_string()),
                            (Line::Ubuntu, "noble-updates".to_string()),
                        ],
                        held: vec![("lockfile.c".to_string(), "die(_(\"a\\b\"));".to_string())],
                    },
                    Era {
                        members: vec![(Line::Upstream, "v2.44.0".to_string())],
                        held: Vec::new(),
                    },
                ],
            }],
        }
    }

    #[test]
    fn what_is_written_reads_back_the_same() {
        let ledger = ledger();
        let text = ledger.render();
        assert!(
            text.contains("era\tupstream v2.43.0..v2.43.1 master, ubuntu noble-updates\n"),
            "{text}"
        );
        assert_eq!(parse(&text).expect("its own ledger"), ledger);
    }

    #[test]
    fn a_form_reworded_or_resourced_is_one_the_table_and_ledger_differ_over() {
        let ledger = ledger();
        let mut form = Form {
            name: "LOCK".to_string(),
            value: ".lock'\tFile exists\n".to_string(),
            sources: vec![source()],
            witnesses: Vec::new(),
        };
        assert!(ledger.differs_from(std::slice::from_ref(&form)).is_empty());
        form.sources[0].words.push('x');
        assert_eq!(ledger.differs_from(std::slice::from_ref(&form)), ["LOCK"]);
        assert_eq!(ledger.differs_from(&[]), ["LOCK (gone from the table)"]);
    }

    /// A release the ledger has not read is a change where no era holds its
    /// lines — no lines at all among them, where every era held some; a head
    /// is one where its lines are not those its line's head held.
    #[test]
    fn a_tree_whose_lines_no_era_holds_is_a_change_even_with_none() {
        let mut ledger = ledger();
        ledger.entries[0].eras.truncate(1);
        let known = ledger.entries[0].eras[0].held.clone();
        let form = Form {
            name: "LOCK".to_string(),
            value: ".lock'\tFile exists\n".to_string(),
            sources: vec![source()],
            witnesses: Vec::new(),
        };
        let tree = |line, name: &str| Tree {
            line,
            name: name.to_string(),
            rev: String::new(),
        };
        let read = |tree: &Tree, held: &Held| Read {
            tree: tree.clone(),
            held: vec![held.clone()],
        };
        let unread = [
            tree(Line::Upstream, "v2.57.0"),
            tree(Line::Upstream, "v2.58.0"),
        ];
        let heads = [tree(Line::Ubuntu, "noble-updates")];
        let still = [
            read(&unread[0], &Held::new()),
            read(&unread[1], &known),
            read(&heads[0], &known),
        ];
        let changed: Vec<String> = ledger
            .changes_in(std::slice::from_ref(&form), &heads, &unread, &still)
            .iter()
            .map(|change| change.tree.label())
            .collect();
        assert_eq!(changed, ["upstream v2.57.0"]);
        let moved = [read(&heads[0], &Held::new())];
        assert_eq!(
            ledger
                .changes_in(std::slice::from_ref(&form), &heads, &[], &moved)
                .len(),
            1,
            "a head whose lines went"
        );
    }
}
