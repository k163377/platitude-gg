use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use platitude_core::Oid;
use platitude_core::details::DiffTarget;
use platitude_core::eol;
use platitude_core::parse::diff::FilePatch;
use platitude_core::preview::{FilePreview, PreviewSide};
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::encode::{
    DiffRow, SplitRow, diff_key, flatten_patches, human_size, is_combined, is_new_file,
    is_unmerged_only, pair_rows, source_byte, spelled_ranges, widest_lines,
};
use crate::hub::{DiffMsg, Feed};

use super::{impl_extend_notified, impl_notify_runs, push_run, qml_register};

mod qobject;
mod rows;
mod selection;
#[cfg(test)]
mod selection_tests;

use rows::bucket_target;
/// Filed here because the test beside `models` names it: a `pub(super)`
/// written in `rows` would only reach as far as this module (structure.md).
pub(super) use rows::wanted;

// ---------------------------------------------------------------------------
// DiffModel: the diff's lines for one file, as one column or as two
// ---------------------------------------------------------------------------

/// One row of the pane. Read as one column, a row is one line of the diff
/// and the `pair_*` roles are empty. Read side by side (`split`), a row
/// holds the old side's line in the plain roles and the new side's in the
/// `pair_*` ones, either of which can be nothing (`encode::pair_rows`);
/// `hunk` and `patch` are the row's, and `new_no` is the right side's
/// number in both readings.
///
/// **Fifteen roles, and all of them taken** (`QModelItem` holds no more).
/// That is why the three small facts about a line — fence, no newline,
/// which side of a conflict — are spelled together in `marks` rather
/// than as a role each: two sides of them would be six.
#[derive(QModelItem, Default, Clone)]
pub struct DiffLineItem {
    /// `hunk` / `ctx` / `add` / `del` / `meta` / `commit` — and `""` on a
    /// split row whose old side has nothing.
    kind: String,
    old_no: i32,
    new_no: i32,
    /// What the row draws: the line marked up for `Text.StyledText`,
    /// coloured or not (see `encode::DiffRow`). One format for every row,
    /// so the pane never has to say which this one is — and so a row is
    /// never measured in a format it is not about to be drawn in.
    text: String,
    /// Display columns of what changed inside this row —
    /// `"col:width,col:width"`, empty where nothing is emphasised
    /// (see `encode::DiffRow`).
    emph: String,
    /// The small facts about the line, as letters: `f` for one of git's
    /// conflict fences, `n` for a line that ends the file without a
    /// newline (デザイン規約 §行末の改行が無いこと), `o` / `t` for the
    /// side of a conflict it came from (`side_of_markers`). On a split
    /// row the right side's letters follow a `|` — `"n|"`, `"fo|ft"` —
    /// and as one column there is no `|` at all (`rows::marks_of`). The
    /// row decodes it once (`DiffRowDelegate`).
    marks: String,
    /// Where this row sits in the patch, so staging it needs no lookup.
    hunk: i32,
    line: i32,
    /// Which of the read's patches the two above are counted within (-1
    /// where they name nothing) — hunks are numbered from zero inside
    /// each one, so it takes all three to reach a line. What the copy
    /// reads a row's own source text back off, with the drawn line the
    /// only one kept (`selection`).
    patch: i32,
    /// The columns the reader's own selection covers on this row, in the
    /// same `"col:wides:width:wides"` spelling as `emph` — with `"*"` for
    /// a row that is in the selection from end to end, which is the shape
    /// almost every selected row has and the one the pane can draw
    /// without being told any columns at all.
    ///
    /// Empty on every row the plain `Copy` does not take: outside the
    /// selection, and on the removed lines and hunk headings inside it.
    /// **The wash is the answer** — what is washed is what is copied
    /// (デザイン規約 §diff の中身をコピーする).
    sel: String,
    /// The new side of a split row — the same roles again, for the line
    /// on the right (デザイン規約 §diff を 2 列で読む). `pair_kind` is
    /// `ctx` / `add`, or `""` where the right has nothing: a removed line
    /// nothing replaced, and every row that is not a line at all. Empty
    /// throughout while the diff is read as one column.
    pair_kind: String,
    pair_text: String,
    pair_emph: String,
    /// The right side's line of the hunk, -1 for none. Its own: the
    /// two sides of a paired row are two lines of the hunk, and a press
    /// on the right's mark stages this one.
    pair_line: i32,
    /// The wash on the right side, spelled the way `sel` is. A selection
    /// is of one column (`selection`), so at most one of the two is ever
    /// written.
    pair_sel: String,
}

impl platitude_core::mem::Footprint for DiffLineItem {
    fn heap_bytes(&self) -> usize {
        self.kind.heap_bytes()
            + self.text.heap_bytes()
            + self.emph.heap_bytes()
            + self.marks.heap_bytes()
            + self.sel.heap_bytes()
            + self.pair_kind.heap_bytes()
            + self.pair_text.heap_bytes()
            + self.pair_emph.heap_bytes()
            + self.pair_sel.heap_bytes()
    }
}

#[derive(Default)]
pub struct DiffModel {
    lines: Vec<DiffLineItem>,
    /// The largest line number the rows carry, on either side. The gutter
    /// is as wide as the widest number it will hold, so counting it is a
    /// fact about the rows.
    widest_no: i32,
    /// The lines the pane measures for a first answer to how far sideways
    /// the code may be sent, packed (`encode::widest_lines`). Lines:
    /// the pane owns the font, and on the fallback a Latin-only mono
    /// family hands a wide glyph to there is no arithmetic over
    /// columns that arrives at what is drawn. Empty is a diff with
    /// nowhere to go.
    widest_lines: String,
    /// Which reading of the rows the ones on screen are from — one up
    /// every time they are laid out again. The pane files a width per row
    /// as the rows are drawn (`DiffReach`), and a width measured on a
    /// line that is gone is not an answer about this diff: this is how it
    /// knows which reading it is holding. The rows changing in place —
    /// the colours arriving, a mark going out — is not a new reading:
    /// same lines, same widths.
    rows_gen: i32,
    /// How many commits this reading stacked, each with a band of its own
    /// above its patch (デザイン規約 §複数のコミットを選ぶ). 0 for every
    /// diff that is of one thing, which is all the others.
    commit_bands: i32,
    /// Whether the rows are laid out side by side — the old side on the
    /// left, the new on the right — rather than as one column
    /// (デザイン規約 §diff を 2 列で読む). Set from the machine's saved
    /// choice as the page opens, and by the band's toggle after that; the
    /// rows are laid out again from `shown` on every change, so nothing
    /// is read twice for it.
    split: bool,
    title: String,
    is_binary: bool,
    /// The file has no old side: everything in the diff was added by it
    /// being there at all (see `encode::is_new_file`).
    is_new_file: bool,
    /// The diff is the combined form git prints for a conflicted path: it
    /// compares the working tree against both stages at once, its rows
    /// carry marker columns, and none of it can be staged in pieces.
    is_combined: bool,
    /// git named the path unmerged and printed nothing else — one of the
    /// two sides is gone, so there is no third thing to compare. There are
    /// no rows, and the absence is the answer.
    unmerged: bool,
    /// The path is a repository of its own sitting in the working copy.
    /// git will not open it, so there are no rows and never will be; what
    /// the pane says instead is what a stage of it would record
    /// (`platitude_core::details::embedded`).
    embedded: bool,
    /// The commit that stage would point at, as the 8 characters every
    /// other hash on screen is shown by. Empty where the repository has
    /// no commit yet — which is also the case `git add` refuses.
    embedded_sha8: String,
    loading: bool,
    /// Whether the colours for the rows on screen have arrived and been
    /// laid over them. False from the moment a file is asked for, and it
    /// stays false for a language the set has no rules for — nothing was
    /// coming. Nothing in the pane is drawn from it: it is how a headless
    /// run can wait for the second half of a diff (`colour-place`).
    coloured: bool,
    /// "" (text diff only) / "image" / "binary".
    preview_kind: String,
    /// `file:` URLs for the image sides ("" = no renderable image there):
    /// the working-tree file itself, or the file core wrote the blob to
    /// (`platitude_core::preview::PreviewFiles`), stamped with the read
    /// they were made at (`rows::apply_preview`).
    preview_old_url: String,
    preview_new_url: String,
    /// Whether the previewed image is a vector one (`image/svg+xml`).
    /// Which smoothing an upscale gets is the cell's choice, but what the
    /// file is is the preview's own fact, said here once for the cell
    /// to read.
    preview_vector: bool,
    /// Human-readable sizes ("" = the side does not exist).
    preview_old_size: String,
    preview_new_size: String,
    /// Fingerprint of the shown diff's source bytes, as hex (QML numbers
    /// cannot hold a u64). Empty while loading — a selection made against
    /// no diff has nothing valid to address.
    fingerprint: String,
    /// What the diff said about line endings, taken apart into the pieces
    /// one sentence needs: which of the four it is (`""` = nothing to
    /// say), the two endings in the order the sentence names them, how
    /// many lines it is about, and how far the sample behind it reached.
    /// The sentence itself is `Words.lineEndings` — the pieces are here
    /// because working them out is not QML's job.
    ending_kind: String,
    ending_from: String,
    ending_to: String,
    ending_lines: i32,
    ending_scope: String,
    ending_ext: String,
    current_key: String,
    /// Which working copy the open diff was read from, empty for this
    /// window's own tree. Beside the key, because the two are asked
    /// different questions: the key says which file an answer is about
    /// (and a copy's file is the same file by that name), this says whose
    /// it is — which is what settles a re-read's aim and what the pane
    /// refuses every write over.
    current_at: String,
    /// The rows of the diff on screen, kept so the colours — which arrive
    /// behind them — can be laid over the same lines without another read
    /// (`DiffMsg::Coloured`). Rebuilding all of them costs orders of
    /// magnitude less than colouring them did
    /// (ci/baseline/code-costs-windows-x64.md §着色), which is why the
    /// colours can afford to redo the whole list, every row of it, in
    /// one pass.
    shown: Option<Arc<Vec<FilePatch>>>,
    /// What changed inside each shown row (`intraline`), kept beside
    /// `shown` for the same rebuilds.
    shown_marks: Arc<platitude_core::intraline::IntraMarks>,
    /// The colours laid over the rows on screen, kept for the one rebuild
    /// that is neither a read nor a repaint: the rows laid out the other
    /// way round (`set_split`). Plain until the second message arrives.
    shown_colors: platitude_core::highlight::DiffColors,
    /// Whether the diff on screen is one a picture stands in for, which is
    /// the other half of what `flatten_patches` is told.
    shown_has_preview: bool,
    /// The two ends of the reader's own selection of the text: a row and
    /// a byte offset into that row's source line. `from` is where the
    /// press landed and `to` is where the pointer has reached, so the
    /// pair is in the order it was made — `taken()` sorts it into
    /// reading order. Whether the four mean anything at all is
    /// `sel_active`'s to say: a fresh model reads 0,0,0,0, which is a
    /// perfectly good empty selection on row 0 and no selection at all.
    sel_from_row: i32,
    sel_from_at: i32,
    sel_to_row: i32,
    sel_to_at: i32,
    /// Which column the selection is of: 0 for the rows' own lines —
    /// the only column while the diff is one — and 1 for the new side of
    /// a split row (`selection`). A drag is of one column, so its two
    /// ends share this.
    sel_side: i32,
    /// What the selection holds, published so the menu can leave out a
    /// row that would copy nothing (デザイン規約 §メニュー: 選べない行は消す).
    /// Read off the rows as the selection settles — the menu decides
    /// what it offers once, as it opens, and these are what it decides
    /// from.
    sel_active: bool,
    sel_has_new: bool,
    sel_removed: i32,
    /// When the file on screen was asked for, for the two numbers the
    /// frame the pane draws cannot give: how much of the wait was the
    /// read, how much was building the model, and what is left is the
    /// drawing (`perf`, ci/baseline/perf-windows-x64.md §操作 1 点の内訳).
    /// Taken by the rows arriving, so a re-read that finds nothing moved
    /// and the colours behind a diff report nothing.
    requested_at: Option<Instant>,
    feed: Option<Arc<Feed<crate::hub::DiffMsg>>>,
    tab_id: i32,
}

impl QListModel for DiffModel {
    type Item = DiffLineItem;

    fn len(&self) -> usize {
        self.lines.len()
    }
    fn get(&self, index: usize) -> Option<&DiffLineItem> {
        self.lines.get(index)
    }
    fn reset_unnotified(&mut self) {
        self.lines.clear();
    }
}

impl_extend_notified!(DiffModel, lines, DiffLineItem);
impl_notify_runs!(DiffModel);
