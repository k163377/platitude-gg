//! Everything QML sees of the diff pane: the properties it binds to, the
//! file reads it asks for, and the feed it drains.
//!
//! One `#[qobject]` block, and it cannot be split further — QMetaInfo is
//! built per file (app-ui.md).

use super::*;

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl DiffModel {
    qproperty!("widestNo", Member = widest_no, Notify = changed);
    qproperty!("widestColumns", Member = widest_columns, Notify = changed);
    qproperty!("hasWide", Member = has_wide, Notify = changed);
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
    qproperty!("previewVector", Member = preview_vector, Notify = changed);
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
    qproperty!("selActive", Member = sel_active, Notify = changed);
    qproperty!("selHasNew", Member = sel_has_new, Notify = changed);
    qproperty!("selRemoved", Member = sel_removed, Notify = changed);

    #[qsignal]
    pub(super) fn changed(&mut self);

    /// The rows on screen are about to be swapped for a re-read of the
    /// same file. Said before the first of them moves, so that whoever
    /// keeps the reader's place takes it off a view that is still standing
    /// where they left it (`DiffScrollPlace`).
    ///
    /// **Nothing on the other end may call back into this model.** It goes
    /// out from inside `drain`, which is holding the borrow.
    #[qsignal]
    fn rows_replacing(&mut self);

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

    /// Diff of one file between two commits — the file list a choice of
    /// exactly two puts up (デザイン規約 §複数のコミットを選ぶ). `from_hex`
    /// is the older side, the one the comparison is measured from.
    #[qslot]
    fn request_range_file(
        &mut self,
        from_hex: String,
        to_hex: String,
        path: String,
        orig_path: String,
    ) {
        let (Ok(from), Ok(to)) = (
            Oid::from_hex_str(from_hex.trim()),
            Oid::from_hex_str(to_hex.trim()),
        ) else {
            return;
        };
        let target = DiffTarget::Range {
            from,
            to,
            path: path.clone(),
            orig_path: (!orig_path.is_empty()).then_some(orig_path),
        };
        self.begin_request(path, target);
    }

    /// Diff of a working-tree entry (bucket: staged/unstaged/untracked/
    /// conflicts).
    #[qslot]
    fn request_work_tree(&mut self, bucket: String, path: String, orig_path: String) {
        let target = bucket_target(&bucket, &path, orig_path);
        self.begin_request(path, target);
    }

    /// Asks whether the working-tree file on screen still reads the way it
    /// did — the page's tick, while a diff of one is open.
    ///
    /// Not a request: nothing is put down and nothing is said to be
    /// loading, because most ticks find the file where they left it and
    /// core answers those with silence (`RepoSession::refresh_diff`). A
    /// tick for some other file is not this pane's, and one that arrives
    /// while a read is already out has nothing to add to it.
    ///
    /// Answers whether a read went out, which is all this side of it can
    /// be asked: the usual answer to the read itself is silence, so the
    /// automation reads the ask (`diff-tick`) rather than waiting for a
    /// reply that a file nobody touched never sends.
    #[qslot]
    fn refresh_work_tree(&mut self, bucket: String, path: String, orig_path: String) -> bool {
        if self.loading {
            return false;
        }
        let target = bucket_target(&bucket, &path, orig_path);
        if diff_key(&target) != self.current_key {
            return false;
        }
        crate::hub::from_session(self.tab_id, |s| s.refresh_diff(target)).is_some()
    }

    /// How many rows one side is named on — `side` is the row role's own
    /// word (`ours` / `theirs`). For the smoke hook (`conflict-sides`):
    /// which side a line came from is drawn as a band a few pixels wide,
    /// and a picture cannot be asked whether every band that should be
    /// there is.
    #[qslot]
    fn side_count(&self, side: String) -> i32 {
        let mut count = 0;
        for line in &self.lines {
            if line.side == side {
                count += 1;
            }
        }
        count
    }

    // ---- the reader's own selection of the text --------------------
    // The pane brings a row and a place along it; everything after that is
    // read off the patches, because the file's own bytes are here and not
    // there (`selection`).

    /// Which byte of row `row`'s line a press `x` pixels along it lands
    /// on. `char_w` and `wide_delta` are what the pane measured of the
    /// mono font (`DiffPane.charW` / `wideDelta`) — the same two numbers
    /// the emphasis wash is placed with.
    #[qslot]
    fn hit_byte_at(&self, row: i32, x: f64, char_w: f64, wide_delta: f64) -> i32 {
        self.hit_at(row, x, char_w, wide_delta)
    }

    /// A press landed: the selection starts here and holds nothing yet.
    ///
    /// The three published counts ride `changed`, the way every other
    /// property of this model does — and it is said here rather than
    /// inside the selection itself, so that the four ways in cost one
    /// signal each and a drag that has not left the character it is on
    /// costs none.
    #[qslot]
    fn begin_select(&mut self, row: i32, at: i32) {
        if self.start_select(row, at) {
            self.changed();
        }
    }

    /// The hand has moved to here.
    #[qslot]
    fn extend_select(&mut self, row: i32, at: i32) {
        if self.drag_select(row, at) {
            self.changed();
        }
    }

    /// One whole row becomes the selection — a right-click outside
    /// whatever was selected (デザイン規約 §diff の中身をコピーする).
    #[qslot]
    fn select_row(&mut self, row: i32) {
        if self.select_whole_row(row) {
            self.changed();
        }
    }

    /// Whether a place in the text is inside the selection, which is what
    /// a right-click asks before deciding whether to take its own row.
    #[qslot]
    fn selection_holds(&self, row: i32, at: i32) -> bool {
        self.holds(row, at)
    }

    #[qslot]
    fn clear_select(&mut self) {
        if self.drop_selection() {
            self.changed();
        }
    }

    /// What the plain `Copy` puts on the clipboard: the unchanged and
    /// added lines the selection covers, cut at its two ends.
    #[qslot]
    fn selection_text(&self) -> String {
        self.copied_new()
    }

    /// What `Copy removed lines` puts there: the removed lines the
    /// selection reaches over, whole.
    #[qslot]
    fn removed_text(&self) -> String {
        self.copied_removed()
    }

    #[qslot]
    fn clear(&mut self) {
        self.current_key = String::new();
        self.widest_no = 0;
        self.widest_columns = 0;
        self.has_wide = false;
        self.title = String::new();
        self.is_binary = false;
        self.is_new_file = false;
        self.is_combined = false;
        self.unmerged = false;
        self.loading = false;
        self.fingerprint = String::new();
        self.coloured = false;
        // The parsed patches go with the rows — kept past here they are a
        // closed pane still holding the whole diff's heap.
        self.shown = None;
        self.shown_has_preview = false;
        self.shown_marks = Default::default();
        self.apply_endings(None);
        self.apply_preview(None);
        // The picture files the read wrote go with the pane: nothing
        // names them any more, and no next read is coming to sweep them.
        crate::hub::with_session(self.tab_id, |s| s.release_preview());
        self.forget_selection();
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
        // Before the first of them is applied, and only where there is a
        // place to lose: rows arriving for a file with none on screen are
        // a pane opening, not a reader being moved.
        if !self.lines.is_empty() && mine.iter().any(|m| matches!(m, DiffMsg::Loaded { .. })) {
            self.rows_replacing();
        }
        for msg in mine {
            match msg {
                DiffMsg::Loaded {
                    patches,
                    preview,
                    fingerprint,
                    endings,
                    marks,
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
                    self.shown_marks = marks;
                    // Plain to begin with. The colours are a second
                    // message and may never come at all — a language the
                    // set has no rules for, a reader who has moved on.
                    self.coloured = false;
                    self.lay_out_rows(&Default::default());
                }
                DiffMsg::Coloured {
                    colors, settled, ..
                } => {
                    // Only the colours the diff ends on set the flag: a
                    // deep diff's quick first answer is a stand-in, and
                    // anything waiting for "the colours" must not latch
                    // a shot of the interim (app-ui.md §UI 自動化の因果性).
                    if settled {
                        self.coloured = true;
                    }
                    self.repaint_rows(&colors);
                }
            }
        }
        self.changed();
    }
}
qml_register!(DiffModel, "DiffModel", singleton = false);
