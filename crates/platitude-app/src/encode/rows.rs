//! Parsed patches flattened into the rows the diff pane draws.

use platitude_core::highlight::DiffColors;
use platitude_core::intraline::IntraMarks;
use platitude_core::parse::diff::{DiffLineKind, FilePatch};

use qtbridge::qtbridge_type_lib::QVariantMap;

use super::markup::{Runs, spelled_ranges, styled};
use super::wire::{Fields, Record, field, has_field};

/// One flattened row of the diff pane.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffRow {
    /// `hunk` / `ctx` / `add` / `del` / `meta` / `commit` — and `""` for
    /// the empty side of a split row.
    pub kind: &'static str,
    /// -1 when the side has no line number.
    pub old_no: i32,
    pub new_no: i32,
    /// What the row draws: the line as `markup::styled` marks it up,
    /// coloured or not. Only the markup is kept: a copy reads the source
    /// (`models::diff::selection`), and holding both doubles what a long
    /// diff costs.
    pub text: String,
    /// What changed inside this row (`platitude_core::intraline`), as runs
    /// of places in the line as spelled ([`spelled_ranges`]); the pane
    /// lays the stronger wash there (デザイン規約 §シンタックスハイライト).
    pub emph: Runs,
    /// One of git's conflict fences (`<<<<<<<` / `|||||||` / `=======` /
    /// `>>>>>>>`); the pane drops its voice for these
    /// (デザイン規約 §シンタックスハイライト).
    pub fence: bool,
    /// The line ends its side without a newline — git's
    /// `\ No newline at end of file`, folded onto this row
    /// (デザイン規約 §行末の改行が無いこと). git prints one note per side,
    /// so the row it lands on says which side.
    pub no_newline: bool,
    /// The hunk and the line within it (-1 on the hunk header) — the
    /// indices [`platitude_core::patch::HunkSelect`] addresses.
    pub hunk: i32,
    pub line: i32,
    /// Which of the read's patches the two above count within (-1 where
    /// they name nothing); together, the address
    /// `patches[patch].hunks[hunk].lines[line]` (`DiffModel::source_line`).
    pub patch: i32,
    /// One marker column per side of a combined diff (`" +"`, `"++"`,
    /// `"- "`), empty on every ordinary row. Which side a line came from
    /// is here only: git paints both sides the same green.
    pub markers: String,
}

/// One line's marks as the row reads them. `side` is `ours` / `theirs` /
/// `""` by the parser's own rule (`side_of_markers`), so the rows and the
/// tally cannot read the marker columns two ways.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LineMarks {
    pub fence: bool,
    pub no_newline: bool,
    pub side: String,
}

impl LineMarks {
    pub fn of(row: &DiffRow) -> Self {
        Self {
            fence: row.fence,
            no_newline: row.no_newline,
            side: platitude_core::parse::diff::side_of_markers(&row.markers).to_string(),
        }
    }
}

impl Record for LineMarks {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("fence", &self.fence)
            .put("noNewline", &self.no_newline)
            .put("side", &self.side)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            fence: field(map, "fence")?,
            no_newline: field(map, "noNewline")?,
            side: field(map, "side")?,
        })
    }
}

/// The marks of one row of the diff: its own line's, and — on a split
/// row that has a line on the right — that line's as well
/// (デザイン規約 §diff を 2 列で読む).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Marks {
    pub own: LineMarks,
    pub pair: Option<LineMarks>,
}

impl Record for Marks {
    fn to_map(&self) -> QVariantMap {
        let own = Fields::new().put("own", &self.own.to_map());
        match &self.pair {
            Some(pair) => own.put("pair", &pair.to_map()),
            None => own,
        }
        .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        let own = LineMarks::from_map(&field(map, "own")?)?;
        // A right side that is absent is a row with one line; one that
        // is there but does not read is a refusal, like any field.
        let pair = if has_field(map, "pair") {
            Some(LineMarks::from_map(&field(map, "pair")?)?)
        } else {
            None
        };
        Ok(Self { own, pair })
    }
}

impl platitude_core::mem::Footprint for Marks {
    fn heap_bytes(&self) -> usize {
        self.own.side.heap_bytes() + self.pair.as_ref().map_or(0, |p| p.side.heap_bytes())
    }
}

/// Whether the patch has a new side and no old one (untracked, or newly
/// added to the index); the pane drops piecemeal staging for it
/// (デザイン規約 §diff の中のステージ).
///
/// The new side has to be named: a headerless patch parses with both
/// sides empty ([`platitude_core::parse::diff::parse_patch`]), and
/// unknown is not new.
pub fn is_new_file(patches: &[FilePatch]) -> bool {
    !patches.is_empty()
        && patches.iter().all(|p| {
            // Never a conflicted path: `AA` has no old side by
            // construction, and an unmerged entry has no patch at all.
            !p.is_combined && !p.unmerged && p.old_path.is_none() && p.new_path.is_some()
        })
}

/// Whether the diff compares its file against more than one side (a
/// conflicted path). Nothing in it is staged or discarded piecemeal
/// (`platitude_core::patch::is_combined` is the floor), and its rows
/// carry [`DiffRow::markers`].
pub fn is_combined(patches: &[FilePatch]) -> bool {
    patches.iter().any(|p| p.is_combined)
}

/// Whether git named the path unmerged and printed no patch
/// (`DU` / `UD` / `AU` / `UA`: one side does not exist). The pane says
/// what the two sides did instead.
pub fn is_unmerged_only(patches: &[FilePatch]) -> bool {
    !patches.is_empty() && patches.iter().all(|p| p.unmerged)
}

/// Flattens parsed patches into rows (hunk headers inline).
/// `binary_note` adds the "(binary file)" row — off where a preview
/// covers the file. `marks` is `None` on the colour repaint, which keeps
/// every row's `emph`.
pub fn flatten_patches(
    patches: &[FilePatch],
    binary_note: bool,
    colors: &DiffColors,
    marks: Option<&IntraMarks>,
) -> Vec<DiffRow> {
    let mut rows = Vec::new();
    for (patch_index, patch) in patches.iter().enumerate() {
        // Each of several commits' patches of one file names its commit
        // (デザイン規約 §複数のコミットを選ぶ), above everything — the
        // binary note included.
        if !patch.from_commit.is_empty() {
            rows.push(DiffRow {
                kind: "commit",
                old_no: -1,
                new_no: -1,
                text: styled(&patch.from_commit, &[]),
                emph: Runs::default(),
                fence: false,
                no_newline: false,
                hunk: -1,
                line: -1,
                patch: i32::try_from(patch_index).unwrap_or(-1),
                markers: String::new(),
            });
        }
        if patch.unmerged {
            // No rows: the pane builds the unmerged sentence from the
            // stage letters (`Words.conflict`).
            continue;
        }
        if patch.is_binary {
            if binary_note {
                rows.push(DiffRow {
                    kind: "meta",
                    old_no: -1,
                    new_no: -1,
                    text: styled("(binary file)", &[]),
                    emph: Runs::default(),
                    fence: false,
                    no_newline: false,
                    hunk: -1,
                    line: -1,
                    patch: -1,
                    markers: String::new(),
                });
            }
            continue;
        }
        for (hunk_index, hunk) in patch.hunks.iter().enumerate() {
            let heading = if hunk.heading.is_empty() {
                String::new()
            } else {
                format!(" {}", hunk.heading)
            };
            let hunk_no = i32::try_from(hunk_index).unwrap_or(-1);
            let patch_no = i32::try_from(patch_index).unwrap_or(-1);
            rows.push(DiffRow {
                kind: "hunk",
                old_no: -1,
                new_no: -1,
                text: styled(&hunk_header(hunk, &heading), &[]),
                emph: Runs::default(),
                fence: false,
                no_newline: false,
                hunk: hunk_no,
                line: -1,
                patch: patch_no,
                markers: String::new(),
            });
            for (line_index, line) in hunk.lines.iter().enumerate() {
                // The note rides the row above (デザイン規約 §行末の改行が無いこと);
                // after a hunk header there is none, and none is invented.
                if line.kind == DiffLineKind::NoNewline {
                    if let Some(row) = rows.last_mut()
                        && row.kind != "hunk"
                    {
                        row.no_newline = true;
                    }
                    continue;
                }
                let kind = match line.kind {
                    DiffLineKind::Context => "ctx",
                    DiffLineKind::Addition => "add",
                    DiffLineKind::Deletion => "del",
                    // Folded above; a note never reaches here.
                    DiffLineKind::NoNewline => continue,
                };
                let read = colors.line(patch_index, hunk_index, line_index);
                let markup = styled(&line.text, &read.spans);

                rows.push(DiffRow {
                    kind,
                    old_no: line.old_no.map_or(-1, |n| n as i32),
                    new_no: line.new_no.map_or(-1, |n| n as i32),
                    text: markup,

                    emph: marks.map_or_else(Runs::default, |marks| {
                        spelled_ranges(&line.text, marks.line(patch_index, hunk_index, line_index))
                    }),
                    fence: read.fence,
                    no_newline: false,
                    hunk: hunk_no,
                    line: i32::try_from(line_index).unwrap_or(-1),
                    patch: patch_no,
                    markers: line.markers.clone(),
                });
            }
        }
    }
    rows
}

/// One row of a diff read side by side (デザイン規約 §diff を 2 列で読む).
///
/// A run of removals followed by a run of additions pairs k-th with k-th —
/// the pairing the in-line emphasis was worked out by
/// (`platitude_core::intraline::hunk_marks`); the longer run's tail stands
/// alone. Rows that are not lines keep the left and have no right.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SplitRow {
    /// The old side, or `kind == ""` for nothing there.
    pub left: DiffRow,
    /// The new side, or `None` for nothing there — and for every row that
    /// is not a line of the file.
    pub right: Option<DiffRow>,
}

/// Lays the flattened rows out side by side. A row's two sides share
/// `hunk` and `patch` (the pairing never crosses a heading); each keeps
/// its own `line`, which a press on its mark stages.
pub fn pair_rows(rows: Vec<DiffRow>) -> Vec<SplitRow> {
    let mut out = Vec::with_capacity(rows.len());
    let mut rows = rows.into_iter().peekable();
    while let Some(row) = rows.next() {
        match row.kind {
            "ctx" => out.push(SplitRow {
                right: Some(row.clone()),
                left: row,
            }),
            "del" => {
                let mut dels = vec![row];
                while let Some(next) = rows.next_if(|r| r.kind == "del") {
                    dels.push(next);
                }
                let mut adds = Vec::new();
                while let Some(next) = rows.next_if(|r| r.kind == "add") {
                    adds.push(next);
                }
                let mut adds = adds.into_iter();
                for del in dels {
                    out.push(SplitRow {
                        left: del,
                        right: adds.next(),
                    });
                }
                out.extend(adds.map(right_only));
            }
            "add" => out.push(right_only(row)),
            _ => out.push(SplitRow {
                left: row,
                right: None,
            }),
        }
    }
    out
}

/// An added line with nothing across from it. The empty seat keeps the
/// address — a press lands on rows, and a row's hunk is one number.
fn right_only(add: DiffRow) -> SplitRow {
    SplitRow {
        left: DiffRow {
            hunk: add.hunk,
            patch: add.patch,
            old_no: -1,
            new_no: -1,
            line: -1,
            ..DiffRow::default()
        },
        right: Some(add),
    }
}

/// The `@@` line as git writes it: one range per old side, and a run of
/// `@` one longer than that count on both ends.
fn hunk_header(hunk: &platitude_core::parse::diff::DiffHunk, heading: &str) -> String {
    let ats = "@".repeat(hunk.extra_old.len() + 2);
    let mut out = format!("{ats} -{},{}", hunk.old_start, hunk.old_count);
    for (start, count) in &hunk.extra_old {
        out.push_str(&format!(" -{start},{count}"));
    }
    out.push_str(&format!(
        " +{},{} {ats}{heading}",
        hunk.new_start, hunk.new_count
    ));
    out
}
