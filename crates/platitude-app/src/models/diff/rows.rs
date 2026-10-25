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

    fn lay_out(&mut self, colors: &platitude_core::highlight::DiffColors, before: Option<&[u64]>) {
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
                    &row_keys(&patches, !self.shown_has_preview, self.split),
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

/// One key per row [`line_items`] lays out of these patches — what a re-read
/// of the same file keeps a row by (`DiffModel::lay_out_kept`): its kind and
/// its line as git printed it, and for a hunk's header its heading alone, the
/// numbers moving with every line above. The colours and the marks laid
/// over a line come and go on their own.
pub(super) fn row_keys(patches: &[FilePatch], binary_note: bool, split: bool) -> Vec<u64> {
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
    let rows = flatten_patches(patches, binary_note, &Default::default(), None);
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

/// Which diff a worktree bucket asks for — shared by the click's
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::notify::runs_of;
    use platitude_core::parse::diff::parse_patch;

    /// How a re-read moved the rows, by the keys the model keeps them by.
    fn edit(before: &str, after: &str, split: bool) -> crate::models::notify::Edit {
        let keys = |patch: &str| row_keys(&parse_patch(patch.as_bytes()), true, split);
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
                    lines: line_items(&was, true, &Default::default(), None, split),
                    split,
                    ..DiffModel::default()
                };
                model.shown = Some(std::sync::Arc::new(parse_patch(after.as_bytes())));
                model.lay_out_kept(&row_keys(&was, true, split));
                let fresh = line_items(
                    &parse_patch(after.as_bytes()),
                    true,
                    &Default::default(),
                    None,
                    split,
                );
                assert_eq!(
                    said(&model.lines),
                    said(&fresh),
                    "{before:?} read again as {after:?}, two columns: {split}"
                );
            }
        }
    }
}
