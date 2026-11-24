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
        let painted = line_items(
            &patches,
            !self.shown_has_preview,
            colors,
            None,
            self.split,
            self.laid.as_deref(),
        );
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
    /// colours — no read. The rows come back as a whole new list, so the
    /// reader's place is held across it (`rows_replacing`, `DiffScrollPlace`).
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
        self.notify_runs(std::iter::once(0..=rows - 1));
    }

    pub(super) fn lay_out_rows(&mut self, colors: &platitude_core::highlight::DiffColors) {
        self.lay_out(colors, None);
    }

    /// Lays out a re-read of the file on screen over its rows, telling the
    /// view only the rows that went, came or changed (`edit_notified`): a
    /// row that stays keeps its delegate — the tip the hand is in, the place
    /// the reader scrolled to (デザイン規約 §hover のツールチップ「外乱では閉じ
    /// ない」). `before`: the rows on screen, keyed off the patches they were
    /// laid out from (`row_keys`).
    pub(super) fn lay_out_kept(&mut self, before: &[u64]) {
        self.lay_out(&Default::default(), Some(before));
    }

    /// The keys of the rows on screen (`row_keys`), as laid out — the parts
    /// in them too.
    pub(super) fn keys_on_screen(&self) -> Option<Vec<u64>> {
        (!self.lines.is_empty())
            .then_some(self.shown.as_ref())
            .flatten()
            .map(|shown| {
                row_keys(
                    shown,
                    !self.shown_has_preview,
                    self.split,
                    self.laid.as_deref(),
                )
            })
    }

    /// The patches the rows on screen were laid out of: the read with the
    /// parts pressed on it in it, or the read itself. What a row's place
    /// (`hunk` / `line` / `patch`) is a place in.
    pub(super) fn laid_patches(&self) -> Option<&[FilePatch]> {
        match &self.laid {
            Some(laid) => Some(&laid.patches),
            None => self.shown.as_deref().map(Vec::as_slice),
        }
    }

    /// Takes `parts` as the ones drawn over the read, laying it out with
    /// them (`ahead::laid`).
    pub(super) fn take_parts(&mut self, parts: Vec<crate::ops::DiffPart>) {
        self.laid = self
            .shown
            .as_deref()
            .and_then(|read| super::ahead::laid(read, &parts))
            .map(Arc::new);
        self.shown_parts = parts;
    }

    /// The parts pressed on the diff on screen, as they stand now
    /// (`ops::StageAhead::diff_parts`): none over another worktree's file,
    /// which nothing here writes.
    pub(super) fn parts_now(&self) -> Vec<crate::ops::DiffPart> {
        match self.shown_print {
            Some(print) if self.current_at.is_empty() => {
                crate::hub::diff_parts(self.tab_id, &self.current_key, print)
            }
            _ => Vec::new(),
        }
    }

    /// Draws the rows again where the parts over them moved — pressed, put
    /// back by a refusal (`restage`). The colours stay: the read is the same.
    /// Answers whether the rows moved.
    pub(super) fn restage_rows(&mut self) -> bool {
        let parts = self.parts_now();
        // Landing moves no row: the layout is the parts' places.
        let placed = |parts: &[crate::ops::DiffPart]| -> Vec<(u64, platitude_core::parts::Part)> {
            parts.iter().map(|p| (p.id, p.part)).collect()
        };
        let moved = placed(&parts) != placed(&self.shown_parts);
        let Some(before) = self.keys_on_screen().filter(|_| moved) else {
            self.take_parts(parts);
            return false;
        };
        self.take_parts(parts);
        self.rows_arriving();
        let colors = std::mem::take(&mut self.shown_colors);
        self.lay_out(&colors, Some(&before));
        self.shown_colors = colors;
        true
    }

    pub(super) fn lay_out(
        &mut self,
        colors: &platitude_core::highlight::DiffColors,
        before: Option<&[u64]>,
    ) {
        let Some(patches) = self.shown.clone() else {
            return;
        };
        // A selection left standing would point by position into the new
        // rows.
        self.forget_selection();
        if before.is_none() {
            self.reset();
        }
        let rows = line_items(
            &patches,
            !self.shown_has_preview,
            colors,
            Some(&self.shown_marks),
            self.split,
            self.laid.as_deref(),
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
        match before {
            Some(before) => {
                let edit = edit_between(
                    before,
                    &row_keys(
                        &patches,
                        !self.shown_has_preview,
                        self.split,
                        self.laid.as_deref(),
                    ),
                );
                self.edit_notified(&edit, rows);
            }
            None => self.extend_notified(rows),
        }
        // A query standing over the old rows stands over these.
        self.refind();
        if crate::harness::memprobe::enabled() {
            crate::harness::memprobe::note("diff-lines", self.tab_id, &self.lines);
        }
    }

    /// A read is out: the pane waits, and an earlier failure no longer
    /// stands for it.
    pub(super) fn waiting(&mut self) {
        self.loading = true;
        self.unread = false;
    }

    /// The read failed (`DiffMsg::Unread`): the pane stops waiting and says
    /// so, keeping the rows a re-read had shown — but not rows relabelled
    /// from the other side (`relabel_to`), which this side's read was to
    /// replace.
    pub(super) fn went_unread(&mut self) {
        if self.relabelled {
            let key = std::mem::take(&mut self.current_key);
            let title = std::mem::take(&mut self.title);
            self.blank();
            self.current_key = key;
            self.title = title;
        }
        self.loading = false;
        self.unread = true;
        self.requested_at = None;
    }

    /// Nothing of a file left in the pane, nothing waited for.
    pub(super) fn blank(&mut self) {
        self.current_key = String::new();
        self.widest_no = 0;
        self.widest_lines = Candidates::default();
        self.title = String::new();
        self.is_binary = false;
        self.is_new_file = false;
        self.whole_why = String::new();
        self.mode_from = String::new();
        self.mode_to = String::new();
        self.is_combined = false;
        self.unmerged = false;
        self.embedded = false;
        self.embedded_sha8 = String::new();
        self.loading = false;
        self.relabelled = false;
        self.unread = false;
        self.fingerprint = String::new();
        self.coloured = false;
        // Kept past here, a closed pane holds the whole diff's heap.
        self.shown = None;
        self.shown_has_preview = false;
        self.shown_print = None;
        self.shown_asked = 0;
        self.drawn_from = 0;
        self.shown_parts = Vec::new();
        self.laid = None;
        crate::hub::diff_drawn(self.tab_id, None, None, 0);
        self.shown_marks = Default::default();
        self.shown_colors = Default::default();
        self.apply_endings(None);
        // Lets go of the pictures held for it: no next read is coming.
        self.apply_preview(None);
        self.forget_selection();
        self.reset();
    }

    /// Whether the rows on screen are this window's read of the file and the
    /// last one asked — nothing out, none asked since: what a press may
    /// carry to the file's other side as they are (`relabel_to`).
    pub(super) fn holds_latest_read(&self) -> bool {
        self.shown_print.is_some()
            && self.current_at.is_empty()
            && !self.loading
            && self.asks > 0
            && self.shown_asked == self.asks
    }

    /// Whether the answer to ask `asked` stands under a wait
    /// (`drawn_from`): not drawn.
    pub(super) fn waits_over(&self, asked: u64) -> bool {
        asked < self.drawn_from
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
        if !same_file {
            // Nothing of the file left is drawn, parts included.
            self.shown_print = None;
            self.shown_parts = Vec::new();
            self.laid = None;
            crate::hub::diff_drawn(self.tab_id, None, None, 0);
        }
        self.requested_at = Some(std::time::Instant::now());
        self.current_key = key;
        self.current_at = at.clone();
        self.title = title;
        self.is_binary = false;
        self.fingerprint = String::new();
        // Like the fingerprint, about bytes not read yet.
        self.apply_endings(None);
        self.waiting();
        // Likewise: the colours on screen are the last read's.
        self.coloured = false;
        if !same_file {
            // Kept while the rows stay: the pane offers what the rows on
            // screen are.
            self.is_new_file = false;
            self.whole_why = String::new();
            self.mode_from = String::new();
            self.mode_to = String::new();
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
            // Numbered, so the read that answers it is told from one asked
            // before (`ops::StageAhead::diff_asked`).
            true => {
                self.asks += 1;
                let asked = self.asks;
                crate::hub::diff_asked(self.tab_id, asked);
                crate::hub::with_session(self.tab_id, |s| s.load_diff_asked(target, asked));
            }
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

    /// Maps the core preview onto the QML-facing strings, holding a blob
    /// side for the pane's image provider (`preview_images`). `None`
    /// resets. What the last preview held is let go: each read holds its
    /// pictures under ids of its own, so the URLs the pane is handed name
    /// none of those.
    pub(super) fn apply_preview(&mut self, preview: Option<&FilePreview>) {
        let before = std::mem::take(&mut self.preview_held);
        self.show_preview(preview);
        for id in before {
            crate::preview_images::release(&id);
        }
    }

    fn show_preview(&mut self, preview: Option<&FilePreview>) {
        let Some(p) = preview else {
            self.preview_kind.clear();
            self.preview_old_url.clear();
            self.preview_new_url.clear();
            self.preview_vector = false;
            self.preview_old_size.clear();
            self.preview_new_size.clear();
            return;
        };
        self.preview_kind = if p.format.is_some() {
            "image".to_string()
        } else {
            "binary".to_string()
        };
        self.preview_vector = p.format.is_some_and(ImageFormat::is_vector);
        // An `Image` reloads only when its source string changes. A held
        // picture is a new id for every read; the working-tree side keeps
        // its path, so the read's fingerprint rides its URL — Qt ignores a
        // `file:` URL's query.
        let stamp = &self.fingerprint;
        let tab = self.tab_id;
        // A blob side is read by the session that read its diff, when the
        // `Image` asks (`preview_images`).
        let reader = match p.format {
            Some(_) => crate::hub::from_session(tab, |s| s.picture_reader()).flatten(),
            None => None,
        };
        let mut held = Vec::new();
        let mut url = |side: &Option<PreviewSide>| -> String {
            match side.as_ref().and_then(|s| s.picture.as_ref()) {
                None => String::new(),
                Some(Picture::File(file)) => {
                    let url = crate::urlpath::file_url(file);
                    if url.is_empty() {
                        return url;
                    }
                    format!("{url}?read={stamp}")
                }
                Some(Picture::Blob(picture)) => {
                    let id = crate::preview_images::hold(tab, picture.clone(), reader.clone());
                    let url = crate::preview_images::url(&id);
                    held.push(id);
                    url
                }
            }
        };
        let old_url = url(&p.old);
        let new_url = url(&p.new);
        let size = |side: &Option<PreviewSide>| -> String {
            side.as_ref()
                .map(|s| human_size(s.size))
                .unwrap_or_default()
        };
        self.preview_old_url = old_url;
        self.preview_new_url = new_url;
        self.preview_held = held;
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
    laid: Option<&platitude_core::parts::After>,
) -> Vec<DiffLineItem> {
    let rows = match laid {
        Some(laid) => crate::encode::flatten_after(laid, binary_note, colors, marks),
        None => flatten_patches(patches, binary_note, colors, marks),
    };
    if split {
        pair_rows(rows).into_iter().map(split_item).collect()
    } else {
        rows.into_iter().map(line_item).collect()
    }
}

/// One key per row [`line_items`] lays out of these patches — what a re-read
/// of the same file keeps a row by (`DiffModel::lay_out_kept`): its kind and
/// its line as git printed it, and for a hunk's header its heading alone, the
/// numbers moving with every line above. The colours and the marks laid
/// over a line come and go on their own.
pub(super) fn row_keys(
    patches: &[FilePatch],
    binary_note: bool,
    split: bool,
    laid: Option<&platitude_core::parts::After>,
) -> Vec<u64> {
    fn said(row: &DiffRow) -> (&'static str, &str) {
        let text = if row.kind == "hunk" {
            row.text.splitn(3, "@@").nth(2).unwrap_or("")
        } else {
            row.text.as_str()
        };
        (row.kind, text)
    }
    fn key_of(parts: impl std::hash::Hash) -> u64 {
        use std::hash::Hasher;
        let mut key = std::collections::hash_map::DefaultHasher::new();
        parts.hash(&mut key);
        key.finish()
    }
    let rows = match laid {
        Some(laid) => crate::encode::flatten_after(laid, binary_note, &Default::default(), None),
        None => flatten_patches(patches, binary_note, &Default::default(), None),
    };
    if split {
        pair_rows(rows)
            .iter()
            .map(|row| key_of((said(&row.left), row.right.as_ref().map(said))))
            .collect()
    } else {
        rows.iter().map(|row| key_of(said(row))).collect()
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

/// Which diff a worktree bucket asks for (`encode::Routing::diff_target`)
/// — shared by the click's request and the tick's re-read, so they cannot
/// address different files under one name. A word no bucket goes by reads
/// the working tree's, as a conflicted file does.
pub(super) fn bucket_target(bucket: &str, path: &str, orig_path: String) -> DiffTarget {
    crate::encode::Routing::of_word(bucket)
        .unwrap_or(crate::encode::Routing::Unstaged)
        .diff_target(path, &orig_path)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::notify::runs_of;
    use platitude_core::Oid;
    use platitude_core::parse::diff::parse_patch;
    use platitude_core::preview::PictureBlob;

    /// A failed read ends the wait and says so; the next ask waits again
    /// with the failure gone, so it never stands for a read it is not about.
    #[test]
    fn an_unread_diff_ends_the_wait_until_the_next_ask() {
        let mut model = DiffModel::default();
        model.waiting();
        // waits(measured): only has to be some stamp — the test reads that it goes, never its time
        model.requested_at = Some(std::time::Instant::now());
        model.went_unread();
        assert!(!model.loading);
        assert!(model.unread);
        assert!(model.requested_at.is_none(), "no round trip to log");
        model.waiting();
        assert!(model.loading);
        assert!(!model.unread);
    }

    /// A press carries the rows on screen to the file's other side only where
    /// they are the file's last read asked: not another file's left from
    /// before the open, not with a read out or asked since, not another
    /// worktree's (`relabel_to`).
    #[test]
    fn only_the_last_read_asked_is_carried_to_the_other_side() {
        let mut model = DiffModel {
            asks: 2,
            shown_asked: 2,
            shown_print: Some(7),
            ..DiffModel::default()
        };
        assert!(model.holds_latest_read());
        model.loading = true;
        assert!(!model.holds_latest_read(), "a read is out");
        model.loading = false;
        model.shown_asked = 1;
        assert!(
            !model.holds_latest_read(),
            "the last one asked has not landed"
        );
        model.shown_asked = 0;
        assert!(
            !model.holds_latest_read(),
            "a tick's re-read, numbered by nobody"
        );
        model.shown_asked = 2;
        model.shown_print = None;
        assert!(!model.holds_latest_read(), "another file's rows");
        model.shown_print = Some(7);
        model.current_at = "elsewhere".to_string();
        assert!(!model.holds_latest_read(), "another worktree's file");
    }

    /// A wait draws no read asked before it — that read shows the side as it
    /// was before the press (`wait_for`); the next one asked is drawn.
    #[test]
    fn a_wait_draws_no_read_asked_before_it() {
        let model = DiffModel {
            asks: 3,
            drawn_from: 4,
            ..DiffModel::default()
        };
        assert!(model.waits_over(3));
        assert!(model.waits_over(0), "a tick's re-read");
        assert!(!model.waits_over(4));
        assert!(!DiffModel::default().waits_over(0), "no wait stands");
    }

    /// The read of the side rows were relabelled as failing: none of them
    /// stays, as they were never that side's — the pane says the read failed
    /// over nothing, and stops waiting.
    #[test]
    fn a_failed_read_takes_the_relabelled_rows_away() {
        let mut model = DiffModel {
            relabelled: true,
            loading: true,
            current_key: "staged:notes.txt".to_string(),
            title: "notes.txt".to_string(),
            shown: Some(std::sync::Arc::new(parse_patch(BEFORE.as_bytes()))),
            shown_print: Some(7),
            ..DiffModel::default()
        };
        model.went_unread();
        assert!(!model.relabelled);
        assert!(model.unread && !model.loading);
        assert!(model.shown.is_none());
        assert_eq!(model.current_key, "staged:notes.txt");
        assert_eq!(model.title, "notes.txt");
    }

    /// A part of a shape not drawn ahead leaves its hunk as read: the rows are
    /// not the parts' exact layout, and the marks wait for the read after it
    /// (`laid_exact`).
    #[test]
    fn a_part_left_as_read_is_no_exact_layout() {
        let split = "--- a/x.txt\n+++ b/x.txt\n@@ -1,12 +1,15 @@\n a\n b\n c\n+X\n d\n e\n f\n+Y\n g\n h\n i\n+Z\n j\n k\n l\n";
        let read = parse_patch(split.as_bytes());
        let line = |line| platitude_core::parts::Part {
            hunk: 0,
            line: Some(line),
            moves: platitude_core::parts::Moves::Old,
        };
        let mut model = DiffModel::default();
        assert!(model.laid_exact(), "nothing laid over the read");
        model.laid = Some(std::sync::Arc::new(platitude_core::parts::after(
            &read,
            &[line(3)],
        )));
        assert!(model.laid_exact());
        model.laid = Some(std::sync::Arc::new(platitude_core::parts::after(
            &read,
            &[line(7)],
        )));
        assert!(
            !model.laid_exact(),
            "seven lines of context between the changes left"
        );
    }

    /// How a re-read moved the rows, by the keys the model keeps them by.
    fn edit(before: &str, after: &str, split: bool) -> crate::models::notify::Edit {
        let keys = |patch: &str| row_keys(&parse_patch(patch.as_bytes()), true, split, None);
        edit_between(&keys(before), &keys(after))
    }

    const BEFORE: &str = "\
--- a/notes.txt
+++ b/notes.txt
@@ -1,3 +1,3 @@ fn main
 one
-two
+deux
 three
";
    /// A line written in at the top of the hunk: the hunk's numbers move, the
    /// rest stays (`DiffModel::lay_out_kept`).
    const LINE_ABOVE: &str = "\
--- a/notes.txt
+++ b/notes.txt
@@ -1,3 +1,4 @@ fn main
+zero
 one
-two
+deux
 three
";

    #[test]
    fn a_line_written_in_above_comes_and_the_rows_below_and_the_heading_stay() {
        let moved = edit(BEFORE, LINE_ABOVE, false);
        assert_eq!(moved.come, runs_of([1]), "the new line, under the header");
        assert!(moved.gone.is_empty());
        assert_eq!(moved.kept, vec![(0, 0), (1, 2), (2, 3), (3, 4), (4, 5)]);
    }

    #[test]
    fn side_by_side_the_rows_are_kept_by_both_their_sides() {
        let moved = edit(BEFORE, LINE_ABOVE, true);
        // header, one, two|deux, three — then header, |zero, one, two|deux, three.
        assert_eq!(moved.come, runs_of([1]));
        assert_eq!(moved.kept, vec![(0, 0), (1, 2), (2, 3), (3, 4)]);
    }

    /// The change staged out of the hunk: `two` / `deux` left as `deux`.
    const STAGED: &str = "\
--- a/notes.txt
+++ b/notes.txt
@@ -1,3 +1,3 @@ fn main
 one
 deux
 three
";

    #[test]
    fn a_line_staged_out_of_the_hunk_goes_alone() {
        let moved = edit(BEFORE, STAGED, false);
        // `two` / `deux` went and `deux` came back as context: a line of
        // another kind is another row.
        assert_eq!(moved.gone, runs_of([2, 3]));
        assert_eq!(moved.come, runs_of([2]));
        assert_eq!(moved.kept, vec![(0, 0), (1, 1), (4, 3)]);
    }

    /// The rows a read of the same file leaves are the rows a fresh layout
    /// of it lays out — either way round, one column or two: the runs that
    /// went and came spliced at their places, the rows that stayed
    /// rewritten (`impl_edit_notified!`; no QObject here, so the rows
    /// alone).
    #[test]
    fn a_read_laid_out_over_the_rows_leaves_the_rows_a_fresh_layout_lays_out() {
        let said = |rows: &[DiffLineItem]| {
            rows.iter()
                .map(|row| {
                    let sides = (row.kind.clone(), row.text.clone(), row.pair_kind.clone());
                    (
                        sides,
                        row.pair_text.clone(),
                        row.old_no,
                        row.new_no,
                        row.hunk,
                        row.line,
                    )
                })
                .collect::<Vec<_>>()
        };
        let pairs = [
            (BEFORE, LINE_ABOVE),
            (LINE_ABOVE, BEFORE),
            (BEFORE, STAGED),
            (STAGED, LINE_ABOVE),
        ];
        for split in [false, true] {
            for (before, after) in pairs {
                let was = parse_patch(before.as_bytes());
                let mut model = DiffModel {
                    lines: line_items(&was, true, &Default::default(), None, split, None),
                    split,
                    ..DiffModel::default()
                };
                model.shown = Some(std::sync::Arc::new(parse_patch(after.as_bytes())));
                model.lay_out_kept(&row_keys(&was, true, split, None));
                let fresh = line_items(
                    &parse_patch(after.as_bytes()),
                    true,
                    &Default::default(),
                    None,
                    split,
                    None,
                );
                assert_eq!(
                    said(&model.lines),
                    said(&fresh),
                    "{before:?} read again as {after:?}, two columns: {split}"
                );
            }
        }
    }

    /// A blob side is held for the pane under an `image://` URL of its
    /// tab's; the working-tree side stays a `file:` URL stamped with the
    /// read. The next read lets the last one's pictures go, and the pane
    /// closing lets go of the rest.
    #[test]
    fn a_pane_holds_its_pictures_until_the_next_read_or_its_close() {
        let oid = Oid::from_hex_str("0123456789abcdef0123456789abcdef01234567").expect("an id");
        let picture = |size: u64| PreviewSide {
            size,
            picture: Some(Picture::Blob(PictureBlob::new(oid, size, "logo.png"))),
        };
        let file = std::env::temp_dir().join("logo.png");
        let read = |old: u64| FilePreview {
            format: Some(ImageFormat::Png),
            old: Some(picture(old)),
            new: Some(PreviewSide {
                size: 1,
                picture: Some(Picture::File(file.clone())),
            }),
        };
        let mut model = DiffModel {
            tab_id: 41,
            fingerprint: "00ff".to_string(),
            ..DiffModel::default()
        };

        model.apply_preview(Some(&read(5)));
        let first = model.preview_held.clone();
        assert_eq!(first.len(), 1, "the blob side, not the file");
        assert_eq!(model.preview_old_url, crate::preview_images::url(&first[0]));
        assert!(first[0].starts_with("t41-"), "{first:?}");
        assert!(crate::preview_images::is_held(&first[0]));
        assert_eq!(
            model.preview_new_url,
            format!("{}?read=00ff", crate::urlpath::file_url(&file))
        );

        model.apply_preview(Some(&read(6)));
        let second = model.preview_held.clone();
        assert_ne!(model.preview_old_url, crate::preview_images::url(&first[0]));
        assert!(
            !crate::preview_images::is_held(&first[0]),
            "the last read's pictures go"
        );
        assert!(crate::preview_images::is_held(&second[0]));

        model.apply_preview(None);
        assert!(model.preview_held.is_empty());
        assert!(
            !crate::preview_images::is_held(&second[0]),
            "the pane closed"
        );
        assert_eq!(model.preview_old_url, "");
    }
}
