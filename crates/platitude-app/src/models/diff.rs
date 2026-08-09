use std::collections::HashMap;
use std::sync::Arc;

use platitude_core::Oid;
use platitude_core::details::DiffTarget;
use platitude_core::eol;
use platitude_core::preview::{FilePreview, PreviewSide};
use qtbridge::{QListModel, QListModelBase, QModelItem, QObjectHolder, qobject};

use crate::encode::{
    DiffRow, diff_key, flatten_patches, human_size, image_data_url, is_combined, is_new_file,
    is_unmerged_only,
};
use crate::hub::{Feed, Hub};

use super::{impl_extend_notified, qml_register};

// ---------------------------------------------------------------------------
// DiffModel: unified diff lines for one file
// ---------------------------------------------------------------------------

#[derive(QModelItem, Default, Clone)]
pub struct DiffLineItem {
    kind: String,
    old_no: i32,
    new_no: i32,
    text: String,
    /// Where this row sits in the patch, so staging it needs no lookup.
    hunk: i32,
    line: i32,
    /// One marker column per side of a combined diff, empty otherwise —
    /// the only place "which side is this line from" is written down (see
    /// `encode::DiffRow`).
    markers: String,
}

#[derive(Default)]
pub struct DiffModel {
    lines: Vec<DiffLineItem>,
    title: String,
    is_binary: bool,
    /// The file has no old side: everything in the diff was added by it
    /// being there at all. What the pane does with that is its business
    /// (see `encode::is_new_file`).
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
    qproperty!("title", Member = title, Notify = changed);
    qproperty!("isBinary", Member = is_binary, Notify = changed);
    qproperty!("isNewFile", Member = is_new_file, Notify = changed);
    qproperty!("isCombined", Member = is_combined, Notify = changed);
    qproperty!("unmerged", Member = unmerged, Notify = changed);
    qproperty!("loading", Member = loading, Notify = changed);
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
        if let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) {
            let feed = Arc::clone(&feeds.diff);
            feed.attach(self.get_qml_method_invoker());
            self.feed = Some(feed);
        }
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
        let Some(msg) = feed.drain().pop() else {
            return;
        };
        if diff_key(&msg.target) != self.current_key {
            return; // stale response
        }
        self.loading = false;
        self.is_binary = msg.patches.iter().any(|p| p.is_binary);
        self.is_new_file = is_new_file(&msg.patches);
        self.is_combined = is_combined(&msg.patches);
        self.unmerged = is_unmerged_only(&msg.patches);
        self.fingerprint = format!("{:016x}", msg.fingerprint);
        self.apply_endings(msg.endings.as_ref());
        self.apply_preview(msg.preview.as_ref());
        self.reset();
        let rows = flatten_patches(&msg.patches, msg.preview.is_none())
            .into_iter()
            .map(|r: DiffRow| DiffLineItem {
                kind: r.kind.to_string(),
                old_no: r.old_no,
                new_no: r.new_no,
                text: r.text,
                hunk: r.hunk,
                line: r.line,
                markers: r.markers,
            })
            .collect();
        self.extend_notified(rows);
        self.changed();
    }
}
qml_register!(DiffModel, "DiffModel", singleton = false);

impl DiffModel {
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
        if !same_file {
            // Goes down with the rows it describes: while they are still
            // on screen the pane must keep offering — or keep withholding
            // — exactly what they are.
            self.is_new_file = false;
            self.is_combined = false;
            self.unmerged = false;
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
