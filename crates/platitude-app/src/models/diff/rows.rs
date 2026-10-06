//! What a read answers turned into the rows on screen: the request that
//! starts one, the rows it lays out, the colours laid over them, and which
//! of the arriving messages are about the file being shown.

use super::*;

impl DiffModel {
    /// Lays the colours over the rows on screen: rewrites only the markup
    /// fields and says `dataChanged` over the lot. Swapping the list would
    /// send the `ListView` back to the top under a reader who has started
    /// scrolling (`colour-place`).
    pub(super) fn repaint_rows(&mut self, colors: &platitude_core::highlight::DiffColors) {
        let Some(patches) = self.shown.clone() else {
            return;
        };
        // No marks: only the text is kept.
        let painted = line_items(&patches, !self.shown_has_preview, colors, None, self.split);
        if painted.len() != self.lines.len() {
            // Cannot happen (same patches), but rewriting by position is
            // only safe while it holds.
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
            row.pair_text = fresh.pair_text;
        }
        self.rows_changed();
    }

    /// Re-lays the rows as one column or two from the same patches and
    /// colours — no read; the reader's place is kept as a re-read keeps it
    /// (`rows_replacing`).
    pub(super) fn relay_rows(&mut self, split: bool) {
        self.split = split;
        if self.shown.is_none() {
            return;
        }
        if !self.lines.is_empty() {
            self.rows_replacing();
        }
        // Taken out: `lay_out_rows` borrows all of `self`.
        let colors = std::mem::take(&mut self.shown_colors);
        self.lay_out_rows(&colors);
        self.shown_colors = colors;
    }

    /// `dataChanged` over every row — nothing added or removed, so the view
    /// keeps its place.
    fn rows_changed(&mut self) {
        let rows = self.lines.len();
        if rows == 0 {
            return;
        }
        self.notify_runs([(0, rows - 1)]);
    }

    pub(super) fn lay_out_rows(&mut self, colors: &platitude_core::highlight::DiffColors) {
        let Some(patches) = self.shown.clone() else {
            return;
        };
        // A selection left standing would point by position into the new
        // rows.
        self.forget_selection();
        self.reset();
        let rows = line_items(
            &patches,
            !self.shown_has_preview,
            colors,
            Some(&self.shown_marks),
            self.split,
        );
        // Both sides: the two columns share one gutter width.
        self.widest_no = rows
            .iter()
            .map(|r| r.old_no.max(r.new_no))
            .max()
            .unwrap_or(0);
        // From the patches: a coloured row's markup length is not its
        // drawn length.
        self.widest_lines = widest_lines(&patches, colors);
        self.rows_gen = self.rows_gen.wrapping_add(1);
        self.commit_bands = patches.iter().filter(|p| !p.from_commit.is_empty()).count() as i32;
        self.extend_notified(rows);
        if crate::harness::memprobe::enabled() {
            crate::harness::memprobe::note("diff-lines", self.tab_id, &self.lines);
        }
    }

    pub(super) fn begin_request(&mut self, title: String, target: DiffTarget) {
        self.begin_request_in(String::new(), title, target);
    }

    /// The same, for a file in another worktree — `at` is that worktree's
    /// path, empty for this window's own tree.
    ///
    /// `at` is part of "the same file" (`same_file`): another worktree's file shares
    /// its path with this window's, so without it a step between them
    /// would read as a re-read and keep the rows and the reader's place.
    pub(super) fn begin_request_in(&mut self, at: String, title: String, target: DiffTarget) {
        let key = diff_key(&target);
        // A re-read of the same file (every partial write ends with one)
        // keeps the rows until the new ones arrive — emptying would flash
        // the pane and drop the view to the top.
        let same_file = key == self.current_key && at == self.current_at;
        self.requested_at = Some(std::time::Instant::now());
        self.current_key = key;
        self.current_at = at.clone();
        self.title = title;
        self.is_binary = false;
        self.fingerprint = String::new();
        // Like the fingerprint, about bytes not read yet.
        self.apply_endings(None);
        self.loading = true;
        // Likewise: the colours on screen are the last read's.
        self.coloured = false;
        if !same_file {
            // Kept while the rows stay: the pane offers what the rows on
            // screen are.
            self.is_new_file = false;
            self.is_combined = false;
            self.unmerged = false;
            self.embedded = false;
            self.embedded_sha8 = String::new();
            self.widest_no = 0;
            self.widest_lines = Candidates::default();
            self.forget_selection();
            self.shown_marks = Default::default();
            self.apply_preview(None);
            self.reset();
        }
        self.changed();
        match at.is_empty() {
            true => crate::hub::with_session(self.tab_id, |s| s.load_diff(target)),
            false => crate::hub::with_session(self.tab_id, |s| s.load_carried_diff(at, target)),
        }
    }

    /// Takes a line-ending notice apart for its sentence; `None` resets
    /// (= nothing to say).
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
        // The read's fingerprint rides the URL: an `Image` reloads only when
        // its source string changes, and the working-tree side keeps its
        // path. Qt ignores a `file:` URL's query.
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

/// The rows a read's patches make, one line a row or side by side
/// (`split`). Apart from [`DiffModel::lay_out_rows`], which notifies the
/// QObject side, so unit tests can build rows without one.
pub(super) fn line_items(
    patches: &[FilePatch],
    binary_note: bool,
    colors: &platitude_core::highlight::DiffColors,
    marks: Option<&platitude_core::intraline::IntraMarks>,
    split: bool,
) -> Vec<DiffLineItem> {
    let rows = flatten_patches(patches, binary_note, colors, marks);
    if split {
        pair_rows(rows).into_iter().map(split_item).collect()
    } else {
        rows.into_iter().map(line_item).collect()
    }
}

/// One line as one row; `pair_line` -1 says the right side is empty.
fn line_item(r: DiffRow) -> DiffLineItem {
    DiffLineItem {
        kind: r.kind.to_string(),
        old_no: r.old_no,
        new_no: r.new_no,
        marks: One::new(Marks {
            own: LineMarks::of(&r),
            pair: None,
        }),
        text: r.text,
        emph: r.emph,
        hunk: r.hunk,
        line: r.line,
        patch: r.patch,
        sel: Optional::none(),
        pair_line: -1,
        ..DiffLineItem::default()
    }
}

fn split_item(r: SplitRow) -> DiffLineItem {
    let mut item = line_item(r.left);
    if let Some(right) = r.right {
        item.new_no = right.new_no;
        item.pair_kind = right.kind.to_string();
        item.marks = One::new(Marks {
            own: item.marks.own.clone(),
            pair: Some(LineMarks::of(&right)),
        });
        item.pair_text = right.text;
        item.pair_emph = right.emph;
        item.pair_line = right.line;
    }
    item
}

/// Which diff a working-tree bucket asks for — shared by the click's
/// request and the tick's re-read, so they cannot address different files
/// under one name.
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

/// Every message about the open file, in arrival order; the rest are
/// dropped.
///
/// Not just the last: a diff sends its rows, then its colours, and a
/// slower earlier request (colouring, `highlight::colors`) can land after
/// the one the reader waits for. In order: a file asked for twice (every
/// partial write) ends on its latest answer, and colours never overtake
/// their rows.
pub(crate) fn wanted(msgs: Vec<DiffMsg>, key: &str) -> Vec<DiffMsg> {
    msgs.into_iter()
        .filter(|msg| diff_key(msg.target()) == key)
        .collect()
}
