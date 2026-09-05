use std::collections::HashMap;
use std::sync::Arc;

use platitude_core::Oid;
use platitude_core::details::DiffTarget;
use platitude_core::eol;
use platitude_core::parse::diff::FilePatch;
use platitude_core::preview::{FilePreview, PreviewSide};
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::encode::{
    DiffRow, diff_key, display_ranges, flatten_patches, has_wide, hit_byte, human_size,
    is_combined, is_new_file, is_unmerged_only, widest_columns,
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
// DiffModel: unified diff lines for one file
// ---------------------------------------------------------------------------

#[derive(QModelItem, Default, Clone)]
pub struct DiffLineItem {
    kind: String,
    old_no: i32,
    new_no: i32,
    /// What the row draws: the line, or the same line marked up in the
    /// theme's colours when `rich` (see `encode::DiffRow`).
    text: String,
    /// Whether `text` is markup. The pane reads the text format off this
    /// rather than sniffing the string — a line of C++ full of `<>` is
    /// not markup, and guessing would eventually decide it was.
    rich: bool,
    /// Display columns of what changed inside this row —
    /// `"col:width,col:width"`, empty where nothing is emphasised
    /// (see `encode::DiffRow`).
    emph: String,
    /// One of git's conflict fences. Not the same thing as `side`
    /// below: that says which side a line came from, this says the line
    /// is not the file talking at all (see `encode::DiffRow`).
    fence: bool,
    /// This line ends the file without a newline, on the side its own
    /// numbers name. git says it in a note of its own; the pane says it
    /// as a mark at the end of this line, which is the line the note was
    /// about (デザイン規約 §行末の改行が無いこと. See `encode::DiffRow`).
    no_newline: bool,
    /// Where this row sits in the patch, so staging it needs no lookup.
    hunk: i32,
    line: i32,
    /// Which of the read's patches the two above are counted within (-1
    /// where they name nothing) — hunks are numbered from zero inside
    /// each one, so it takes all three to reach a line. What the copy
    /// reads a row's own source text back off, rather than keeping a
    /// second copy of every line beside the drawn one (`selection`).
    patch: i32,
    /// The columns the reader's own selection covers on this row, in the
    /// same `"col:wides:width:wides"` spelling as `emph` — with `"*"` for
    /// a row that is in the selection from end to end, which is the shape
    /// almost every selected row has and the one the pane can draw
    /// without being told any columns at all.
    ///
    /// Empty on every row the plain `Copy` does not take: outside the
    /// selection, and on the removed lines and hunk headings inside it.
    /// **The wash is the answer** — what is not washed is not copied
    /// (デザイン規約 §diff の中身をコピーする).
    sel: String,
    /// The side a combined diff's marker columns name — `"ours"` /
    /// `"theirs"` / `""`, empty for a single-parent diff — read once as
    /// the row is built, by the parser's own rule
    /// (`platitude_core::parse::diff::side_of_markers` over
    /// `encode::DiffRow.markers`), so the rows and the tally cannot come
    /// to read the columns two ways.
    side: String,
}

impl platitude_core::mem::Footprint for DiffLineItem {
    fn heap_bytes(&self) -> usize {
        self.kind.heap_bytes()
            + self.text.heap_bytes()
            + self.side.heap_bytes()
            + self.emph.heap_bytes()
            + self.sel.heap_bytes()
    }
}

#[derive(Default)]
pub struct DiffModel {
    lines: Vec<DiffLineItem>,
    /// The largest line number the rows carry, on either side. The gutter
    /// is as wide as the widest number it will hold, so counting it is a
    /// fact about the rows rather than something QML works out.
    widest_no: i32,
    /// How many columns of the mono font the longest line needs
    /// (`encode::widest_columns`). The pane turns it into how far sideways
    /// the code may be sent; 0 is a diff with nowhere to go.
    widest_columns: i32,
    /// Whether any line carries a glyph the mono font draws two columns
    /// wide (`encode::has_wide`). The pane measures what one of those
    /// advances only where one is on screen: the ruler that measures it
    /// sets a wide glyph, and on a Latin-only mono family that loads a
    /// fallback font this process otherwise has no reason to hold.
    has_wide: bool,
    /// How many commits this reading stacked, each with a band of its own
    /// above its patch (デザイン規約 §複数のコミットを選ぶ). 0 for every
    /// diff that is of one thing, which is all the others.
    commit_bands: i32,
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
    /// no rows, and the absence is the answer rather than a failure.
    unmerged: bool,
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
    /// file is is the preview's own fact rather than something QML reads
    /// back off the URL.
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
    /// The rows of the diff on screen, kept so the colours — which arrive
    /// behind them — can be laid over the same lines without another read
    /// (`DiffMsg::Coloured`). Rebuilding all of them measured 2ms for a
    /// 6,000-line diff, which is why the colours can afford to redo the
    /// whole list rather than address rows one at a time.
    shown: Option<Arc<Vec<FilePatch>>>,
    /// What changed inside each shown row (`intraline`), kept beside
    /// `shown` for the same rebuilds.
    shown_marks: Arc<platitude_core::intraline::IntraMarks>,
    /// Whether the diff on screen is one a picture stands in for, which is
    /// the other half of what `flatten_patches` is told.
    shown_has_preview: bool,
    /// The two ends of the reader's own selection of the text: a row and
    /// a byte offset into that row's source line. `from` is where the
    /// press landed and `to` is where the pointer has reached, so the
    /// pair is in the order it was made rather than in reading order —
    /// `taken()` sorts it. Whether the four mean anything at all is
    /// `sel_active`'s to say: a fresh model reads 0,0,0,0, which is a
    /// perfectly good empty selection on row 0 and no selection at all.
    sel_from_row: i32,
    sel_from_at: i32,
    sel_to_row: i32,
    sel_to_at: i32,
    /// What the selection holds, published so the menu can leave out a
    /// row that would copy nothing (デザイン規約 §メニュー: 選べない行は消す).
    /// Read off the rows as the selection settles rather than counted
    /// again when the menu opens — the menu decides what it offers once,
    /// as it opens, and these are what it decides from.
    sel_active: bool,
    sel_has_new: bool,
    sel_removed: i32,
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
