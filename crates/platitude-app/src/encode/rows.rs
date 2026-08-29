//! Parsed patches flattened into the rows the diff pane draws.

use platitude_core::highlight::DiffColors;
use platitude_core::intraline::IntraMarks;
use platitude_core::parse::diff::{DiffLineKind, FilePatch};

use super::markup::{display_ranges, styled};

/// One flattened row of the diff pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRow {
    /// `hunk` / `ctx` / `add` / `del` / `meta`.
    pub kind: &'static str,
    /// -1 when the side has no line number.
    pub old_no: i32,
    pub new_no: i32,
    /// What the row draws — plain text, or the same line marked up for
    /// `Text.StyledText` when [`DiffRow::rich`]. Only one of the two is
    /// ever held: keeping both doubles what a long diff costs.
    pub text: String,
    /// Whether [`DiffRow::text`] is markup. False wherever the theme had
    /// nothing to say — a language the set has never heard of, a hunk
    /// heading, the binary note, a conflict marker.
    pub rich: bool,
    /// Display columns of what changed inside this row —
    /// `"col:wides:width:wides,…"`: where each run starts and how far it
    /// runs, said in the mono font's columns and in the wide glyphs
    /// standing in them. Empty where nothing is emphasised
    /// (`platitude_core::intraline`, laid out by [`display_ranges`]). The
    /// pane draws these as the stronger wash under the text
    /// (デザイン規約 §シンタックスハイライト), and needs the second number
    /// to place the first — a wide glyph is drawn from a fallback font
    /// whose advance need not be two of the mono one's.
    pub emph: String,
    /// One of git's conflict fences (`<<<<<<<` / `|||||||` / `=======` /
    /// `>>>>>>>`); the pane drops its voice for these
    /// (デザイン規約 §シンタックスハイライト).
    pub fence: bool,
    /// This line is the last of its side and ends without a newline —
    /// git's `\ No newline at end of file`, folded onto the row it is
    /// about instead of standing as a row of its own
    /// (デザイン規約 §行末の改行が無いこと). The note is never on both
    /// sides of one line at once: git prints one per side, after that
    /// side's own last line, so the row it lands on already says which
    /// side is being talked about.
    pub no_newline: bool,
    /// Which hunk of the file this row belongs to, and which line of that
    /// hunk it is (-1 on the hunk header). These are the same indices
    /// [`platitude_core::patch::HunkSelect`] addresses, so a row can be
    /// staged straight from what the pane is showing.
    pub hunk: i32,
    pub line: i32,
    /// Which of the read's patches the two above are counted within (-1
    /// where they name nothing). Hunks are numbered from zero inside each
    /// patch, so this is what makes the pair above an address: the
    /// selection reads a row's own source line back off
    /// `patches[patch].hunks[hunk].lines[line]` rather than keeping a
    /// second copy of the text (`DiffModel::source_line`).
    pub patch: i32,
    /// One marker column per side of a combined diff (`" +"`, `"++"`,
    /// `"- "`), empty on every ordinary row. Which side a line came from
    /// is in here and nowhere else: the colour cannot carry it, since git
    /// paints our side and theirs the same green.
    pub markers: String,
}

/// Whether the patch has a new side and no old one — a file the repository
/// is seeing for the first time (untracked, or newly added to the index).
///
/// The pane reads this to drop the piecemeal staging it would otherwise
/// offer (デザイン規約 §diff の中のステージ). A deleted file is not one
/// of these — its lines still exist on the old side, and a part of them
/// can still be staged.
///
/// The new side has to be named, not merely inferred from a missing old
/// one: a patch that carries no `diff --git` header at all parses with
/// both sides empty ([`platitude_core::parse::diff::parse_patch`]
/// synthesizes the file entry), and that is a diff whose shape is unknown
/// rather than one that is known to be new.
pub fn is_new_file(patches: &[FilePatch]) -> bool {
    !patches.is_empty()
        && patches.iter().all(|p| {
            // A conflicted path is never one of these: `AA` — both
            // branches invented the file — has no old side by
            // construction, and an unmerged entry has neither side
            // because git printed no patch at all.
            !p.is_combined && !p.unmerged && p.old_path.is_none() && p.new_path.is_some()
        })
}

/// Whether the diff compares its file against **more than one** side —
/// the form git prints for a conflicted path. Nothing in it can be staged
/// or thrown away piecemeal (`platitude_core::patch::is_combined` is the
/// floor under that), and its rows carry [`DiffRow::markers`].
pub fn is_combined(patches: &[FilePatch]) -> bool {
    patches.iter().any(|p| p.is_combined)
}

/// Whether git named the path as unmerged and printed no patch for it:
/// one of the two sides does not exist, so there is nothing to compare
/// (`DU` / `UD` / `AU` / `UA`). The pane has no rows to show and says what
/// the two sides did instead.
pub fn is_unmerged_only(patches: &[FilePatch]) -> bool {
    !patches.is_empty() && patches.iter().all(|p| p.unmerged)
}

/// Flattens parsed patches into displayable rows (hunk headers inline).
/// `binary_note` inserts the "(binary file)" meta row; the caller turns it
/// off when a preview (image / size summary) already covers that file.
/// `marks` is `None` on the repaint that lays colours over rows already
/// on screen — that pass keeps every row's `emph` as it is, so working
/// the columns out again would be thrown away.
pub fn flatten_patches(
    patches: &[FilePatch],
    binary_note: bool,
    colors: &DiffColors,
    marks: Option<&IntraMarks>,
) -> Vec<DiffRow> {
    let mut rows = Vec::new();
    for (patch_index, patch) in patches.iter().enumerate() {
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
                    text: String::from("(binary file)"),
                    rich: false,
                    emph: String::new(),
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
                text: hunk_header(hunk, &heading),
                rich: false,
                emph: String::new(),
                fence: false,
                no_newline: false,
                hunk: hunk_no,
                line: -1,
                patch: patch_no,
                markers: String::new(),
            });
            for (line_index, line) in hunk.lines.iter().enumerate() {
                // git's note about the line above it rather than a line
                // of the file, so it rides that row instead of taking one
                // (デザイン規約 §行末の改行が無いこと). A note with no line
                // in front of it is git talking about nothing: there is
                // no row to carry it and none is invented.
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
                let rich = !markup.is_empty();
                rows.push(DiffRow {
                    kind,
                    old_no: line.old_no.map_or(-1, |n| n as i32),
                    new_no: line.new_no.map_or(-1, |n| n as i32),
                    text: if rich { markup } else { line.text.clone() },
                    rich,
                    emph: marks.map_or_else(String::new, |marks| {
                        display_ranges(&line.text, marks.line(patch_index, hunk_index, line_index))
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
