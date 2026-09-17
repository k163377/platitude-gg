//! Everything QML sees of the diff pane: the properties it binds to, the
//! file reads it asks for, and the feed it drains.
//!
//! One `#[qobject]` block, and it cannot be split further — QMetaInfo is
//! built per file (app-ui.md).

use super::*;

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl DiffModel {
    qproperty!("widestNo", Member = widest_no, Notify = changed);
    qproperty!("widestLines", Member = widest_lines, Notify = changed);
    qproperty!("rowsGen", Member = rows_gen, Notify = changed);
    qproperty!("commitBands", Member = commit_bands, Notify = changed);
    qproperty!("title", Member = title, Notify = changed);
    qproperty!("isBinary", Member = is_binary, Notify = changed);
    qproperty!("isNewFile", Member = is_new_file, Notify = changed);
    qproperty!("isCombined", Member = is_combined, Notify = changed);
    qproperty!("unmerged", Member = unmerged, Notify = changed);
    qproperty!("embedded", Member = embedded, Notify = changed);
    qproperty!("embeddedSha8", Member = embedded_sha8, Notify = changed);
    qproperty!("loading", Member = loading, Notify = changed);
    qproperty!("carriedAt", Member = current_at, Notify = changed);
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
    /// **Return without calling back into this model.** It goes out from
    /// inside `drain`, which is holding the borrow.
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

    /// One file as each of several chosen commits changed it, stacked
    /// (デザイン規約 §複数のコミットを選ぶ). `packed` is their ids newest
    /// first, joined by `\u{1f}` — the order the graph stands in.
    #[qslot]
    fn request_choice_file(&mut self, packed: String, path: String, orig_path: String) {
        let mut oids = Vec::new();
        for hex in packed.split('\u{1f}').filter(|h| !h.is_empty()) {
            let Ok(oid) = Oid::from_hex_str(hex.trim()) else {
                return;
            };
            oids.push(oid);
        }
        if oids.is_empty() {
            return;
        }
        let target = DiffTarget::Choice {
            oids,
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

    /// The same file in another working copy — `at` is that copy's path.
    /// What the read-only pane opens: the contents have to be readable,
    /// and the only thing that made the ordinary read this window's own
    /// was where it was aimed (`RepoSession::load_carried_diff`).
    #[qslot]
    fn request_carried(&mut self, at: String, bucket: String, path: String, orig_path: String) {
        let target = bucket_target(&bucket, &path, orig_path);
        self.begin_request_in(at, path, target);
    }

    /// The re-read of that file, on the copies' own slower tick — the one
    /// the ordinary `refresh_work_tree` is on the page's tick for.
    ///
    /// **Aimed where the rows came from.** Sent through the ordinary
    /// re-read it would read *this* window's file of that name and hand
    /// it to a pane showing somebody else's, which is the one way the two
    /// trees can be mixed up on screen.
    #[qslot]
    fn refresh_carried(
        &mut self,
        at: String,
        bucket: String,
        path: String,
        orig_path: String,
    ) -> bool {
        if self.loading || at != self.current_at {
            return false;
        }
        let target = bucket_target(&bucket, &path, orig_path);
        if diff_key(&target) != self.current_key {
            return false;
        }
        crate::hub::from_session(self.tab_id, |s| s.refresh_carried_diff(at, target)).is_some()
    }

    /// Asks whether the working-tree file on screen still reads the way it
    /// did — the page's tick, while a diff of one is open.
    ///
    /// An ask only: nothing is put down and nothing is said to be
    /// loading, because most ticks find the file where they left it and
    /// core answers those with silence (`RepoSession::refresh_diff`). A
    /// tick for some other file is not this pane's, and one that arrives
    /// while a read is already out has nothing to add to it.
    ///
    /// Answers whether a read went out, which is all this side of it can
    /// be asked: the usual answer to the read itself is silence, so the
    /// automation reads the ask (`diff-tick`) and takes that for its
    /// answer.
    #[qslot]
    fn refresh_work_tree(&mut self, bucket: String, path: String, orig_path: String) -> bool {
        // A diff read from another copy is re-read on the copies' tick
        // and by their aim (`refresh_carried`).
        if self.loading || !self.current_at.is_empty() {
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
    // read off the patches, because the file's own bytes are here
    // (`selection`).

    /// Which byte of row `row`'s line the place `at` of it stands on —
    /// `at` counted in the units the row's own layout counts a place in
    /// (`LineRuler`, which is what turned the reader's press into
    /// one). Pixels stop at the pane: only the row that was laid out
    /// knows where its characters are drawn, and only the file's own
    /// bytes are here.
    #[qslot]
    fn source_byte_at(&self, row: i32, at: i32) -> i32 {
        self.byte_at(row, at)
    }

    /// A press landed: the selection starts here and holds nothing yet.
    ///
    /// The three published counts ride `changed`, the way
    /// every other property of this model does — and it is
    /// said here, so that the four ways in cost one signal
    /// each and a drag that has not left the character it is
    /// on costs none.
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
        self.widest_lines = String::new();
        self.title = String::new();
        self.is_binary = false;
        self.is_new_file = false;
        self.is_combined = false;
        self.unmerged = false;
        self.embedded = false;
        self.embedded_sha8 = String::new();
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
        let carries_rows = mine.iter().any(|m| matches!(m, DiffMsg::Loaded { .. }));
        // Held for the two marks below: one says when the rows arrived
        // and the other when they are in the model, and between them is
        // what this call costs.
        // **Only a batch carrying rows takes it** — the colours behind a
        // diff are a second message about a read already answered.
        let asked = carries_rows.then(|| self.requested_at.take()).flatten();
        // Before the first of them is applied, and only where there is a
        // place to lose: rows arriving for a file with none on screen
        // are a pane opening.
        if !self.lines.is_empty() && carries_rows {
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
                    embedded,
                    ..
                } => {
                    self.loading = false;
                    if let Some(t0) = &asked {
                        // Data arrival only; PagePerfDriver separately
                        // observes the frame that draws it.
                        tracing::info!(
                            elapsed_ms = t0.elapsed().as_millis() as u64,
                            "diff request round trip"
                        );
                    }
                    self.is_binary = patches.iter().any(|p| p.is_binary);
                    self.is_new_file = is_new_file(&patches);
                    self.is_combined = is_combined(&patches);
                    self.unmerged = is_unmerged_only(&patches);
                    self.embedded = embedded.is_some();
                    // The same 8 characters every other hash on screen is
                    // shown by; empty for a repository with no commit yet.
                    self.embedded_sha8 = match embedded {
                        Some(platitude_core::details::Embedded::On(oid)) => oid.short_hex(8),
                        _ => String::new(),
                    };
                    self.fingerprint = format!("{fingerprint:016x}");
                    self.apply_endings(endings.as_ref());
                    self.apply_preview(preview.as_deref());
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
                    // anything waiting for "the colours" waits for
                    // these (app-ui.md §UI 自動化の因果性).
                    if settled {
                        self.coloured = true;
                    }
                    self.repaint_rows(&colors);
                }
            }
        }
        self.changed();
        if let Some(t0) = asked {
            // The rows are in the model and the signals are out; what is
            // left before the frame is the view and the painting.
            tracing::info!(
                elapsed_ms = t0.elapsed().as_millis() as u64,
                "diff rows applied"
            );
        }
    }
}
qml_register!(DiffModel, "DiffModel", singleton = false);
