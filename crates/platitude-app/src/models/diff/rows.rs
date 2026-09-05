//! What a read answers turned into the rows on screen: the request that
//! starts one, the rows it lays out, the colours laid over them, and which
//! of the arriving messages are about the file being shown.

use super::*;

impl DiffModel {
    /// Lays the colours over the rows already on screen.
    ///
    /// Their words change and nothing else does — same rows, same order,
    /// same numbers — so this rewrites the two fields that carry the
    /// markup and says `dataChanged` over the lot. Swapping the list
    /// instead would be no slower to build (2ms for 6,000 rows), but a
    /// `ListView` handed a new list starts again at the top: measured
    /// `at=0` where the reader had scrolled to 400 (measured,
    /// `PG_AUTO_ACT=colour-place`). The colours arrive a second after the
    /// rows do, which is exactly long enough to have started reading.
    pub(super) fn repaint_rows(&mut self, colors: &platitude_core::highlight::DiffColors) {
        let Some(patches) = self.shown.clone() else {
            return;
        };
        // `None` for the marks: this pass rewrites `text`/`rich` only,
        // so the emphasis columns it would compute go straight to waste.
        let painted = flatten_patches(&patches, !self.shown_has_preview, colors, None);
        if painted.len() != self.lines.len() {
            // The same patches were walked both times, so this cannot
            // happen — but addressing rows by position is only safe while
            // it holds, and rebuilding is correct if slightly ruder.
            tracing::warn!(
                was = self.lines.len(),
                now = painted.len(),
                "colours came back a different length; rebuilding the rows"
            );
            self.lay_out_rows(colors);
            return;
        }
        for (row, fresh) in self.lines.iter_mut().zip(painted) {
            row.text = fresh.text;
            row.rich = fresh.rich;
        }
        self.rows_changed();
    }

    /// Tells the view that every row's contents have been rewritten in
    /// place. Nothing was added or removed, so the view keeps its place,
    /// and QML's own bindings on the rows re-read themselves.
    fn rows_changed(&mut self) {
        let rows = self.lines.len();
        if rows == 0 {
            return;
        }
        self.notify_runs([(0, rows - 1)]);
    }

    /// Builds the row list from the diff on screen and the colours given.
    pub(super) fn lay_out_rows(&mut self, colors: &platitude_core::highlight::DiffColors) {
        let Some(patches) = self.shown.clone() else {
            return;
        };
        // The rows these numbers addressed are about to go. A selection
        // left standing would point into the new ones by position, which
        // is a different part of a file that has just been written to.
        self.forget_selection();
        self.reset();
        let rows = line_items(
            &patches,
            !self.shown_has_preview,
            colors,
            Some(&self.shown_marks),
        );
        // Both sides at once: the two columns are laid out to one width,
        // and a file whose old side ran further than its new one would
        // otherwise hand the wider number to the narrower column.
        self.widest_no = rows
            .iter()
            .map(|r| r.old_no.max(r.new_no))
            .max()
            .unwrap_or(0);
        // Read off the patches rather than the rows: a coloured row holds
        // markup, and the length of `<font color="#…">` is not the length
        // of anything on screen.
        self.widest_columns = widest_columns(&patches);
        // Read off the patches for the same reason: the wide glyph the
        // pane's ruler stands for is one of the file's own characters,
        // and a coloured row spells it inside markup.
        self.has_wide = has_wide(&patches);
        // How many commits this reading stacked. Every other diff is of
        // one thing and answers 0 (デザイン規約 §複数のコミットを選ぶ).
        self.commit_bands = patches.iter().filter(|p| !p.from_commit.is_empty()).count() as i32;
        self.extend_notified(rows);
        if crate::harness::memprobe::enabled() {
            crate::harness::memprobe::note("diff-lines", self.tab_id, &self.lines);
        }
    }

    pub(super) fn begin_request(&mut self, title: String, target: DiffTarget) {
        let key = diff_key(&target);
        // Reading the same file again — which is what every partial write
        // ends with — keeps the rows that are on screen until the new ones
        // arrive. Emptying here would blank the pane for the length of the
        // round trip and drop the view to the top, and on a diff of any
        // size that reads as a flash rather than as an update. `drain`
        // swaps the whole list inside one call, so the exchange is never
        // seen half done.
        let same_file = key == self.current_key;
        self.current_key = key;
        self.title = title;
        self.is_binary = false;
        self.fingerprint = String::new();
        // Goes with the fingerprint rather than with the rows: it is a
        // statement about bytes that have not been read yet, and a notice
        // held over from the last file would be about that file.
        self.apply_endings(None);
        self.loading = true;
        // Goes with the fingerprint rather than with the rows, for the
        // same reason: the colours that are on screen are the last file's.
        self.coloured = false;
        if !same_file {
            // Goes down with the rows it describes: while they are still
            // on screen the pane must keep offering — or keep withholding
            // — exactly what they are.
            self.is_new_file = false;
            self.is_combined = false;
            self.unmerged = false;
            self.widest_no = 0;
            self.widest_columns = 0;
            self.has_wide = false;
            self.forget_selection();
            self.shown_marks = Default::default();
            self.apply_preview(None);
            self.reset();
        }
        self.changed();
        crate::hub::with_session(self.tab_id, |s| s.load_diff(target));
    }

    /// Takes a line-ending notice apart into the pieces its sentence needs.
    /// `None` resets — which is also what "nothing to say" looks like.
    pub(super) fn apply_endings(&mut self, notice: Option<&eol::Notice>) {
        let words = crate::encode::ending_words(notice);
        self.ending_kind = words.kind;
        self.ending_from = words.from;
        self.ending_to = words.to;
        self.ending_lines = words.lines;
        self.ending_scope = words.scope;
        self.ending_ext = words.ext;
    }

    /// Maps the core preview onto the QML-facing strings. `None` resets.
    pub(super) fn apply_preview(&mut self, preview: Option<&FilePreview>) {
        let Some(p) = preview else {
            self.preview_kind.clear();
            self.preview_old_url.clear();
            self.preview_new_url.clear();
            self.preview_vector = false;
            self.preview_old_size.clear();
            self.preview_new_size.clear();
            return;
        };
        self.preview_kind = if p.image_mime.is_some() {
            "image".to_string()
        } else {
            "binary".to_string()
        };
        self.preview_vector = p.image_mime == Some("image/svg+xml");
        // The URL names the read as well as the file. An `Image` reloads
        // on a source that changed and on nothing else, and the
        // working-tree side keeps its path from one read to the next —
        // so the fingerprint of the bytes the read was made at rides on
        // the URL: a file that moved under the pane is decoded again, and
        // one that did not is not. Qt reads a `file:` URL's path and
        // leaves its query alone.
        let stamp = &self.fingerprint;
        let url = |side: &Option<PreviewSide>| -> String {
            let Some(file) = side.as_ref().and_then(|s| s.file.as_deref()) else {
                return String::new();
            };
            let url = crate::urlpath::file_url(file);
            if url.is_empty() {
                return url;
            }
            format!("{url}?read={stamp}")
        };
        let size = |side: &Option<PreviewSide>| -> String {
            side.as_ref()
                .map(|s| human_size(s.size))
                .unwrap_or_default()
        };
        self.preview_old_url = url(&p.old);
        self.preview_new_url = url(&p.new);
        self.preview_old_size = size(&p.old);
        self.preview_new_size = size(&p.new);
    }
}

/// The rows a read's patches make, in the shape the list holds them.
///
/// Apart from [`DiffModel::lay_out_rows`] so that what a row carries can
/// be checked without a view to hang it on: laying them out tells the
/// QObject side, and there is none in a unit test (`selection`'s cases).
pub(super) fn line_items(
    patches: &[FilePatch],
    binary_note: bool,
    colors: &platitude_core::highlight::DiffColors,
    marks: Option<&platitude_core::intraline::IntraMarks>,
) -> Vec<DiffLineItem> {
    flatten_patches(patches, binary_note, colors, marks)
        .into_iter()
        .map(|r: DiffRow| DiffLineItem {
            kind: r.kind.to_string(),
            old_no: r.old_no,
            new_no: r.new_no,
            text: r.text,
            rich: r.rich,
            emph: r.emph,
            fence: r.fence,
            no_newline: r.no_newline,
            hunk: r.hunk,
            line: r.line,
            patch: r.patch,
            sel: String::new(),
            side: platitude_core::parse::diff::side_of_markers(&r.markers).to_string(),
        })
        .collect()
}

/// Which diff one of the working tree's four buckets asks for. Read by
/// both the request a click makes and the re-read the tick makes, so the
/// two can never address different files under one name.
pub(super) fn bucket_target(bucket: &str, path: &str, orig_path: String) -> DiffTarget {
    match bucket {
        "staged" => DiffTarget::Staged {
            path: path.to_string(),
            orig_path: (!orig_path.is_empty()).then_some(orig_path),
        },
        "untracked" => DiffTarget::Untracked {
            path: path.to_string(),
        },
        // Conflicted files show their working-tree state.
        _ => DiffTarget::Unstaged {
            path: path.to_string(),
        },
    }
}

/// Everything about the file that is open, in arrival order.
///
/// Not simply the newest arrival, for two reasons. One diff now sends two
/// messages — its rows, then its colours — and taking only the last would
/// leave the other unread. And two diffs started a moment apart need not
/// finish in that order: colouring a file of source can take hundreds of
/// milliseconds where a plain one takes none (`highlight::colors`), so a
/// slower *earlier* request can land after the one the reader is waiting
/// for.
///
/// In order, so a file asked for twice — which every partial write does —
/// ends on its latest answer rather than the one it superseded, and so
/// colours never overtake the rows they belong to. Everything else is
/// dropped: those are answers to questions nobody is asking any more.
pub(crate) fn wanted(msgs: Vec<DiffMsg>, key: &str) -> Vec<DiffMsg> {
    msgs.into_iter()
        .filter(|msg| diff_key(msg.target()) == key)
        .collect()
}
