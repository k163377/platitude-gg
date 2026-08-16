use std::collections::HashMap;
use std::sync::Arc;

use platitude_core::Oid;
use platitude_core::details::DiffTarget;
use platitude_core::eol;
use platitude_core::parse::diff::FilePatch;
use platitude_core::preview::{FilePreview, PreviewSide};
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::encode::{
    DiffRow, diff_key, flatten_patches, human_size, image_data_url, is_combined, is_new_file,
    is_unmerged_only,
};
use crate::hub::{DiffMsg, Feed, Hub};

use super::{impl_extend_notified, qml_register};

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
    /// One of git's conflict fences. Not the same thing as `markers`
    /// below: that says which side a line came from, this says the line
    /// is not the file talking at all (see `encode::DiffRow`).
    fence: bool,
    /// Where this row sits in the patch, so staging it needs no lookup.
    hunk: i32,
    line: i32,
    /// One marker column per side of a combined diff, empty otherwise —
    /// the only place "which side is this line from" is written down (see
    /// `encode::DiffRow`).
    markers: String,
}

impl platitude_core::mem::Footprint for DiffLineItem {
    fn heap_bytes(&self) -> usize {
        self.kind.heap_bytes() + self.text.heap_bytes() + self.markers.heap_bytes()
    }
}

#[derive(Default)]
pub struct DiffModel {
    lines: Vec<DiffLineItem>,
    /// The largest line number the rows carry, on either side. The gutter
    /// is as wide as the widest number it will hold, so counting it is a
    /// fact about the rows rather than something QML works out.
    widest_no: i32,
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
    /// data: URLs for the image sides ("" = no renderable image there).
    preview_old_url: String,
    preview_new_url: String,
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
    /// Whether the diff on screen is one a picture stands in for, which is
    /// the other half of what `flatten_patches` is told.
    shown_has_preview: bool,
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

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl DiffModel {
    qproperty!("widestNo", Member = widest_no, Notify = changed);
    qproperty!("title", Member = title, Notify = changed);
    qproperty!("isBinary", Member = is_binary, Notify = changed);
    qproperty!("isNewFile", Member = is_new_file, Notify = changed);
    qproperty!("isCombined", Member = is_combined, Notify = changed);
    qproperty!("unmerged", Member = unmerged, Notify = changed);
    qproperty!("loading", Member = loading, Notify = changed);
    qproperty!("coloured", Member = coloured, Notify = changed);
    qproperty!("previewKind", Member = preview_kind, Notify = changed);
    qproperty!("previewOldUrl", Member = preview_old_url, Notify = changed);
    qproperty!("previewNewUrl", Member = preview_new_url, Notify = changed);
    qproperty!(
        "previewOldSize",
        Member = preview_old_size,
        Notify = changed
    );
    qproperty!(
        "previewNewSize",
        Member = preview_new_size,
        Notify = changed
    );
    qproperty!("fingerprint", Member = fingerprint, Notify = changed);
    qproperty!("endingKind", Member = ending_kind, Notify = changed);
    qproperty!("endingFrom", Member = ending_from, Notify = changed);
    qproperty!("endingTo", Member = ending_to, Notify = changed);
    qproperty!("endingLines", Member = ending_lines, Notify = changed);
    qproperty!("endingScope", Member = ending_scope, Notify = changed);
    qproperty!("endingExt", Member = ending_ext, Notify = changed);

    #[qsignal]
    fn changed(&mut self);

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        let invoker = self.get_qml_method_invoker();
        self.feed = crate::hub::attach_feed(tab_id, |f| &f.diff, invoker);
    }

    /// Diff of one file of a commit (vs its first parent).
    #[qslot]
    fn request_commit_file(
        &mut self,
        oid_hex: String,
        parent_hex: String,
        path: String,
        orig_path: String,
    ) {
        let Ok(oid) = Oid::from_hex_str(oid_hex.trim()) else {
            return;
        };
        let parent = Oid::from_hex_str(parent_hex.trim()).ok();
        let target = DiffTarget::Commit {
            oid,
            parent,
            path: path.clone(),
            orig_path: (!orig_path.is_empty()).then_some(orig_path),
        };
        self.begin_request(path, target);
    }

    /// Diff of a working-tree entry (bucket: staged/unstaged/untracked/
    /// conflicts).
    #[qslot]
    fn request_work_tree(&mut self, bucket: String, path: String, orig_path: String) {
        let target = match bucket.as_str() {
            "staged" => DiffTarget::Staged {
                path: path.clone(),
                orig_path: (!orig_path.is_empty()).then_some(orig_path),
            },
            "untracked" => DiffTarget::Untracked { path: path.clone() },
            // Conflicted files show their working-tree state.
            _ => DiffTarget::Unstaged { path: path.clone() },
        };
        self.begin_request(path, target);
    }

    #[qslot]
    fn clear(&mut self) {
        self.current_key = String::new();
        self.widest_no = 0;
        self.title = String::new();
        self.is_binary = false;
        self.is_new_file = false;
        self.is_combined = false;
        self.unmerged = false;
        self.loading = false;
        self.apply_endings(None);
        self.apply_preview(None);
        self.reset();
        self.changed();
    }

    #[qslot]
    fn drain(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        // Everything for the open file, in arrival order: the rows and the
        // colours behind them are two messages about one diff, and taking
        // only one of them would leave whichever came second unread until
        // something else woke the slot.
        let mine = wanted(feed.drain(), &self.current_key);
        if mine.is_empty() {
            return;
        }
        for msg in mine {
            match msg {
                DiffMsg::Loaded {
                    patches,
                    preview,
                    fingerprint,
                    endings,
                    ..
                } => {
                    self.loading = false;
                    self.is_binary = patches.iter().any(|p| p.is_binary);
                    self.is_new_file = is_new_file(&patches);
                    self.is_combined = is_combined(&patches);
                    self.unmerged = is_unmerged_only(&patches);
                    self.fingerprint = format!("{fingerprint:016x}");
                    self.apply_endings(endings.as_ref());
                    self.apply_preview(preview.as_ref());
                    self.shown_has_preview = preview.is_some();
                    self.shown = Some(patches);
                    // Plain to begin with. The colours are a second
                    // message and may never come at all — a language the
                    // set has no rules for, a reader who has moved on.
                    self.coloured = false;
                    self.lay_out_rows(&Default::default());
                }
                DiffMsg::Coloured { colors, .. } => {
                    self.coloured = true;
                    self.repaint_rows(&colors);
                }
            }
        }
        self.changed();
    }
}
qml_register!(DiffModel, "DiffModel", singleton = false);

impl DiffModel {
    /// Lays the colours over the rows already on screen.
    ///
    /// Their words change and nothing else does — same rows, same order,
    /// same numbers — so this rewrites the two fields that carry the
    /// markup and says `dataChanged` over the lot. Swapping the list
    /// instead would be no slower to build (2ms for 6,000 rows), but a
    /// `ListView` handed a new list starts again at the top: measured
    /// `at=0` where the reader had scrolled to 400 (2026-08-13 実測,
    /// `PG_AUTO_ACT=colour-place`). The colours arrive a second after the
    /// rows do, which is exactly long enough to have started reading.
    fn repaint_rows(&mut self, colors: &platitude_core::highlight::DiffColors) {
        let Some(patches) = self.shown.clone() else {
            return;
        };
        let painted = flatten_patches(&patches, !self.shown_has_preview, colors);
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
    #[expect(unsafe_code)]
    fn rows_changed(&mut self) {
        let Some(proxy) = self.try_get_rust_proxy_ptr() else {
            return;
        };
        let Ok(last) = i32::try_from(self.lines.len()) else {
            return;
        };
        if last == 0 {
            return;
        }
        let root = qtbridge::qtbridge_type_lib::QModelIndex::default();
        // SAFETY: the same pattern as `extend_notified` — the proxy
        // pointer stays valid while the QObject side is attached, and we
        // are on the Qt main thread inside a slot.
        let proxy = unsafe { &mut *proxy };
        let top = proxy.base_index(&*self, 0, 0, &root);
        let bottom = proxy.base_index(&*self, last - 1, 0, &root);
        proxy.base_data_changed(&mut *self, &top, &bottom);
    }

    /// Builds the row list from the diff on screen and the colours given.
    fn lay_out_rows(&mut self, colors: &platitude_core::highlight::DiffColors) {
        let Some(patches) = self.shown.clone() else {
            return;
        };
        self.reset();
        let rows: Vec<DiffLineItem> = flatten_patches(&patches, !self.shown_has_preview, colors)
            .into_iter()
            .map(|r: DiffRow| DiffLineItem {
                kind: r.kind.to_string(),
                old_no: r.old_no,
                new_no: r.new_no,
                text: r.text,
                rich: r.rich,
                fence: r.fence,
                hunk: r.hunk,
                line: r.line,
                markers: r.markers,
            })
            .collect();
        // Both sides at once: the two columns are laid out to one width,
        // and a file whose old side ran further than its new one would
        // otherwise hand the wider number to the narrower column.
        self.widest_no = rows
            .iter()
            .map(|r| r.old_no.max(r.new_no))
            .max()
            .unwrap_or(0);
        self.extend_notified(rows);
        if crate::memprobe::enabled() {
            crate::memprobe::note("diff-lines", self.tab_id, &self.lines);
        }
    }

    fn begin_request(&mut self, title: String, target: DiffTarget) {
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
            self.apply_preview(None);
            self.reset();
        }
        self.changed();
        if let Some(Some(session)) = Hub::with(|hub| hub.session(self.tab_id)) {
            session.load_diff(target);
        }
    }

    /// Takes a line-ending notice apart into the pieces its sentence needs.
    /// `None` resets — which is also what "nothing to say" looks like.
    fn apply_endings(&mut self, notice: Option<&eol::Notice>) {
        let words = crate::encode::ending_words(notice);
        self.ending_kind = words.kind;
        self.ending_from = words.from;
        self.ending_to = words.to;
        self.ending_lines = words.lines;
        self.ending_scope = words.scope;
        self.ending_ext = words.ext;
    }

    /// Maps the core preview onto the QML-facing strings. `None` resets.
    fn apply_preview(&mut self, preview: Option<&FilePreview>) {
        let Some(p) = preview else {
            self.preview_kind.clear();
            self.preview_old_url.clear();
            self.preview_new_url.clear();
            self.preview_old_size.clear();
            self.preview_new_size.clear();
            return;
        };
        self.preview_kind = if p.image_mime.is_some() {
            "image".to_string()
        } else {
            "binary".to_string()
        };
        let url = |side: &Option<PreviewSide>| -> String {
            let (Some(mime), Some(s)) = (p.image_mime, side.as_ref()) else {
                return String::new();
            };
            s.bytes
                .as_ref()
                .map(|b| image_data_url(mime, b))
                .unwrap_or_default()
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

/// Everything about the file that is open, in arrival order.
///
/// Not simply the newest arrival, for two reasons. One diff now sends two
/// messages — its rows, then its colours — and taking only the last would
/// leave the other unread. And two diffs started a moment apart need not
/// finish in that order: colouring a file of source takes most of a second
/// where a plain one takes none (`highlight::colors`), so a slower
/// *earlier* request can land after the one the reader is waiting for.
/// Taking the last and testing it left the pane on the file it was on
/// before, with no second chance — nothing else was ever going to arrive
/// for that click (2026-08-13 実測).
///
/// In order, so a file asked for twice — which every partial write does —
/// ends on its latest answer rather than the one it superseded, and so
/// colours never overtake the rows they belong to. Everything else is
/// dropped: those are answers to questions nobody is asking any more.
fn wanted(msgs: Vec<DiffMsg>, key: &str) -> Vec<DiffMsg> {
    msgs.into_iter()
        .filter(|msg| diff_key(msg.target()) == key)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(path: &str) -> DiffTarget {
        DiffTarget::Unstaged {
            path: path.to_string(),
        }
    }

    fn rows(path: &str, fingerprint: u64) -> DiffMsg {
        DiffMsg::Loaded {
            target: target(path),
            patches: Arc::new(Vec::new()),
            preview: None,
            fingerprint,
            endings: None,
        }
    }

    fn colours(path: &str) -> DiffMsg {
        DiffMsg::Coloured {
            target: target(path),
            colors: platitude_core::highlight::DiffColors::default(),
        }
    }

    fn key(path: &str) -> String {
        diff_key(&target(path))
    }

    fn paths(msgs: &[DiffMsg]) -> Vec<String> {
        msgs.iter().map(|m| diff_key(m.target())).collect()
    }

    #[test]
    fn the_open_file_is_found_under_a_slower_earlier_read() {
        // What the pane sees when a coloured file asked for first lands
        // after the plain one asked for second: both are in hand at once,
        // and only one of them is the file on screen. Taking the last
        // arrival would leave the reader looking at the previous file.
        let picked = wanted(
            vec![rows("plain.txt", 0), rows("slow.rs", 0)],
            &key("plain.txt"),
        );
        assert_eq!(paths(&picked), vec![key("plain.txt")]);
    }

    #[test]
    fn the_rows_and_the_colours_behind_them_are_both_taken() {
        // Both halves of one diff, waiting together because the slot was
        // woken once for each and ran once.
        let picked = wanted(vec![rows("a.rs", 0), colours("a.rs")], &key("a.rs"));
        assert_eq!(picked.len(), 2);
        assert!(matches!(picked[0], DiffMsg::Loaded { .. }));
        assert!(matches!(picked[1], DiffMsg::Coloured { .. }));
    }

    #[test]
    fn the_latest_answer_for_the_same_file_comes_last() {
        // A partial write re-reads the file it just changed, so the same
        // key arrives twice and the one with the write in it is second.
        // Applied in order, that is the one left on screen.
        let picked = wanted(vec![rows("a.rs", 1), rows("a.rs", 7)], &key("a.rs"));
        let last = picked.last().expect("two were kept");
        assert!(matches!(last, DiffMsg::Loaded { fingerprint: 7, .. }));
    }

    #[test]
    fn answers_for_files_nobody_is_on_are_dropped() {
        assert!(wanted(vec![rows("a.rs", 0), colours("b.rs")], &key("c.rs")).is_empty());
        assert!(wanted(Vec::new(), &key("a.rs")).is_empty());
    }
}
