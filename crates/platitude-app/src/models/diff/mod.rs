use std::sync::Arc;
use std::time::Instant;

use platitude_core::Oid;
use platitude_core::details::DiffTarget;
use platitude_core::eol;
use platitude_core::parse::diff::FilePatch;
use platitude_core::preview::{FilePreview, PreviewSide};
use qtbridge::qtbridge_type_lib::QVariantMap;
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, QmlObject, qobject};

use crate::encode::{
    Candidates, DiffRow, Fields, LineMarks, Marks, One, Optional, Record, Runs, SplitRow, diff_key,
    field, flatten_patches, human_size, is_combined, is_new_file, is_unmerged_only, pair_rows,
    source_byte, spelled_ranges, widest_lines,
};
use crate::hub::{DiffMsg, Feed};

use super::{impl_extend_notified, impl_notify_runs, push_run, qml_register};

mod qobject;
mod rows;
mod selection;
#[cfg(test)]
mod selection_tests;

use rows::bucket_target;
/// Re-exported for `models::diff_tests` (`rows` is private).
pub(super) use rows::wanted;

/// One row of the pane. As one column, a row is one line of the diff and
/// the `pair_*` roles are empty; side by side (`split`), the old side's
/// line is in the plain roles and the new side's in `pair_*`, either
/// possibly nothing (`encode::pair_rows`). `new_no` is the right side's
/// number in both readings.
///
/// Fifteen roles, all taken (`QModelItem` holds no more) — a new fact
/// about a line goes into `marks`, not a role.
#[derive(QModelItem, Default, Clone)]
pub struct DiffLineItem {
    /// `hunk` / `ctx` / `add` / `del` / `meta` / `commit` — and `""` on a
    /// split row whose old side has nothing.
    kind: String,
    old_no: i32,
    new_no: i32,
    /// The line as `Text.StyledText` markup, coloured or not
    /// (`encode::DiffRow`) — one format for every row, so a row is measured
    /// in the format it is drawn in.
    text: String,
    /// The runs of what changed inside this row (`encode::DiffRow`).
    emph: Runs,
    /// The small facts about the line — a fence, a missing final newline,
    /// the side of a conflict — and on a split row the right side's as
    /// well (`encode::Marks`).
    marks: One<Marks>,
    /// Where this row sits in the patch, so staging it needs no lookup.
    hunk: i32,
    line: i32,
    /// Which of the read's patches `hunk` / `line` count within (-1 for
    /// none) — hunks restart at zero in each. The copy reads the row's
    /// source text back through all three (`selection`).
    patch: i32,
    /// What the reader's selection covers on this row ([`Washed`]). None
    /// on every row the plain `Copy` does not take — what is washed is
    /// what is copied (デザイン規約 §diff の中身をコピーする).
    sel: Optional<Washed>,
    /// The `pair_*` roles: the new side of a split row (デザイン規約
    /// §diff を 2 列で読む). `pair_kind` is `ctx` / `add`, or `""` where the
    /// right has nothing (a removed line nothing replaced, a row that is
    /// not a line). Empty throughout as one column.
    pair_kind: String,
    pair_text: String,
    pair_emph: Runs,
    /// The right side's line of the hunk, -1 for none — a paired row is
    /// two lines of the hunk, and the right mark stages this one.
    pair_line: i32,
    /// The right side's wash. A selection is of one column, so at most one
    /// of `sel` / `pair_sel` is set.
    pair_sel: Optional<Washed>,
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

/// What the reader's selection covers on one side of a row: the line
/// end to end (`whole`, and `runs` says nothing), or the runs of places
/// a drag cut through (the shape `emph` has, laid by the same ruler).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Washed {
    pub whole: bool,
    pub runs: Runs,
}

impl Washed {
    pub fn whole() -> Self {
        Self {
            whole: true,
            runs: Runs::default(),
        }
    }

    pub fn runs(runs: Runs) -> Self {
        Self { whole: false, runs }
    }
}

impl Record for Washed {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("whole", &self.whole)
            .put("runs", &self.runs)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            whole: field(map, "whole")?,
            runs: field(map, "runs")?,
        })
    }
}

impl platitude_core::mem::Footprint for Washed {
    fn heap_bytes(&self) -> usize {
        self.runs.heap_bytes()
    }
}

#[derive(Default)]
pub struct DiffModel {
    lines: Vec<DiffLineItem>,
    /// The largest line number the rows carry on either side; the gutter
    /// is sized to it.
    widest_no: i32,
    /// Lines the pane measures for a first answer to how far sideways the
    /// code may go (`encode::widest_lines`) — lines, not a width: no column
    /// arithmetic survives the font fallback (rules-refs/app-ui.md
    /// 「diff は横へ送る」). Empty = nowhere to go.
    widest_lines: Candidates,
    /// Which layout of the rows is on screen: bumped by every
    /// `lay_out_rows`, not by an in-place change (colours, marks) — same
    /// lines, same widths. `DiffReach` drops widths filed against an older
    /// one.
    rows_gen: i32,
    /// How many commits this reading stacked, a band each (デザイン規約
    /// §複数のコミットを選ぶ); 0 for a diff of one thing.
    commit_bands: i32,
    /// Whether the rows are laid out side by side (デザイン規約
    /// §diff を 2 列で読む).
    split: bool,
    title: String,
    is_binary: bool,
    /// The file has no old side (`encode::is_new_file`).
    is_new_file: bool,
    /// git's combined diff of a conflicted path — none of it can be
    /// staged in pieces.
    is_combined: bool,
    /// git named the path unmerged and printed nothing else (one side is
    /// gone): no rows, and that is the answer.
    unmerged: bool,
    /// The path is a repository of its own in the working copy: never any
    /// rows; the pane says what a stage of it would record
    /// (`platitude_core::details::embedded`).
    embedded: bool,
    /// The commit that stage would point at, 8 characters. Empty where the
    /// repository has no commit yet (the case `git add` refuses).
    embedded_sha8: String,
    loading: bool,
    /// Whether the settled colours have been laid over the rows on screen;
    /// stays false for a language with no rules. Nothing is drawn from it —
    /// it is what a headless run waits on (`colour-place`).
    coloured: bool,
    /// "" (text diff only) / "image" / "binary".
    preview_kind: String,
    /// `file:` URLs for the image sides ("" = no renderable image there):
    /// the working-tree file, or the file core wrote the blob to
    /// (`platitude_core::preview::PreviewFiles`), stamped with the read
    /// (`rows::apply_preview`).
    preview_old_url: String,
    preview_new_url: String,
    /// Whether the previewed image is a vector one (`image/svg+xml`); the
    /// cell picks its upscale smoothing by it.
    preview_vector: bool,
    /// Human-readable sizes ("" = the side does not exist).
    preview_old_size: String,
    preview_new_size: String,
    /// Fingerprint of the shown diff's source bytes, as hex (QML numbers
    /// cannot hold a u64). Empty while loading — a selection made against
    /// no diff has nothing valid to address.
    fingerprint: String,
    /// The line-ending notice taken apart for `Words.lineEndings`: which of
    /// the four (`""` = nothing to say), the two endings in the sentence's
    /// order, how many lines, and how far the sample reached.
    ending_kind: String,
    ending_from: String,
    ending_to: String,
    ending_lines: i32,
    ending_scope: String,
    ending_ext: String,
    current_key: String,
    /// Which working copy the open diff was read from, empty for this
    /// window's own tree — whose file it is, which the key does not say
    /// (`begin_request_in`). It aims a re-read, and the pane refuses every
    /// write while it is set.
    current_at: String,
    /// The patches on screen, kept so the colours (`DiffMsg::Coloured`) and
    /// a re-lay rebuild the rows without a read — rebuilding every row is
    /// orders cheaper than colouring (ci/baseline/code-costs-windows-x64.md
    /// §着色).
    shown: Option<Arc<Vec<FilePatch>>>,
    /// What changed inside each shown row (`intraline`), kept beside
    /// `shown` for the same rebuilds.
    shown_marks: Arc<platitude_core::intraline::IntraMarks>,
    /// The colours laid over the rows on screen, kept for a re-lay
    /// (`relay_rows`); plain until they arrive.
    shown_colors: platitude_core::highlight::DiffColors,
    /// Whether a picture stands in for the diff on screen — the other half
    /// of what `flatten_patches` is told.
    shown_has_preview: bool,
    /// The selection's two ends, each a row and a byte offset into its
    /// source line: `from` the press, `to` the pointer, in the order made
    /// (`taken()` sorts). Meaningless unless `sel_active` — 0,0,0,0 is also
    /// a valid empty selection.
    sel_from_row: i32,
    sel_from_at: i32,
    sel_to_row: i32,
    sel_to_at: i32,
    /// Which column the selection is of (`selection`).
    sel_side: i32,
    /// What the selection holds, so the menu can leave out a row that
    /// would copy nothing (デザイン規約 §メニュー「選べない行は消す」);
    /// settled with the selection, read once as the menu opens.
    sel_active: bool,
    sel_has_new: bool,
    sel_removed: i32,
    /// When the file on screen was asked for, for the read / apply marks
    /// (ci/baseline/perf-windows-x64.md §操作 1 点の内訳).
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
