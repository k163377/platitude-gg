//! Everything QML sees of the diff pane: the properties it binds to, the
//! file reads it asks for, and the feed it drains.
//!
//! One `#[qobject]` block, and it cannot be split further — QMetaInfo is
//! built per file (structure.md §分割).

use super::*;

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl DiffModel {
    qproperty!("widestNo", Member = widest_no, Notify = changed);
    // Through a getter: a value QML wrote would not reach Rust (encode::wire_qml).
    qproperty!("widestLines", Read = widest_lines, Notify = changed);
    fn widest_lines(&self) -> &Candidates {
        &self.widest_lines
    }
    qproperty!("rowsGen", Member = rows_gen, Notify = changed);
    qproperty!("commitBands", Member = commit_bands, Notify = changed);
    qproperty!("split", Member = split, Notify = changed);
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

    /// The rows on screen are about to be swapped (a re-read of the same
    /// file, or a re-lay) — emitted before any moves, so `DiffScrollPlace`
    /// takes the place off a view still standing.
    ///
    /// Handlers must not call back into this model: `drain` / `relay_rows`
    /// hold the borrow while it goes out.
    #[qsignal]
    pub(super) fn rows_replacing(&mut self);

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        let invoker = self.get_qml_method_invoker();
        self.feed = crate::hub::attach_feed(tab_id, |f| &f.diff, invoker);
    }

    /// One column or two (`relay_rows`) — the band's toggle, and the saved
    /// choice as the page opens.
    #[qslot]
    fn set_split(&mut self, split: bool) {
        if self.split == split {
            return;
        }
        self.relay_rows(split);
        self.changed();
    }

    /// How the split rows came out, for `diff-split` (a picture cannot say
    /// whether a removed line and its replacement were paired): `pairs=`
    /// del / add rows, `alone=` changed lines with an empty seat across,
    /// `both=` context rows.
    #[qslot]
    fn split_tally(&self) -> String {
        let mut pairs = 0;
        let mut alone = 0;
        let mut both = 0;
        for line in &self.lines {
            match (line.kind.as_str(), line.pair_kind.as_str()) {
                ("ctx", "ctx") => both += 1,
                ("del", "add") => pairs += 1,
                ("del", "") | ("", "add") => alone += 1,
                _ => {}
            }
        }
        format!(
            "split={} pairs={pairs} alone={alone} both={both}",
            self.split
        )
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

    /// Diff of one file between two commits (a choice of exactly two —
    /// デザイン規約 §複数のコミットを選ぶ). `from_hex` is the older side.
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
    /// (デザイン規約 §複数のコミットを選ぶ). `ids` are newest first (graph
    /// order).
    #[qslot]
    fn request_choice_file(&mut self, ids: Vec<String>, path: String, orig_path: String) {
        let mut oids = Vec::new();
        for hex in ids.iter().filter(|h| !h.is_empty()) {
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
    fn request_working_tree(&mut self, bucket: String, path: String, orig_path: String) {
        let target = bucket_target(&bucket, &path, orig_path);
        self.begin_request(path, target);
    }

    /// The same read aimed at another worktree — `at` is its path
    /// (the read-only pane; `RepoSession::load_carried_diff`).
    #[qslot]
    fn request_carried(&mut self, at: String, bucket: String, path: String, orig_path: String) {
        let target = bucket_target(&bucket, &path, orig_path);
        self.begin_request_in(at, path, target);
    }

    /// `refresh_working_tree` for a carried diff, on the other worktrees'
    /// slower tick. Aimed where the rows came from — the ordinary re-read
    /// would put this window's file of that name in a pane showing another
    /// worktree's.
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

    /// The page's tick: asks core whether the open working-tree file
    /// changed (`RepoSession::refresh_diff`, silent when not). Sets no
    /// `loading` — most ticks find nothing. Ignored for another file or
    /// while a read is out.
    ///
    /// Returns whether a read went out — what `diff-tick` reads, since the
    /// read's usual answer is silence.
    #[qslot]
    fn refresh_working_tree(&mut self, bucket: String, path: String, orig_path: String) -> bool {
        // A carried diff is re-read by `refresh_carried`.
        if self.loading || !self.current_at.is_empty() {
            return false;
        }
        let target = bucket_target(&bucket, &path, orig_path);
        if diff_key(&target) != self.current_key {
            return false;
        }
        crate::hub::from_session(self.tab_id, |s| s.refresh_diff(target)).is_some()
    }

    /// How many lines are marked `side` (`ours` / `theirs`), for
    /// `conflict-sides` — a picture cannot say every band is there.
    #[qslot]
    pub(super) fn side_count(&self, side: String) -> i32 {
        // Each side of a row names its side at most once, so this counts
        // lines (`encode::Marks`).
        let count = self
            .lines
            .iter()
            .flat_map(|line| std::iter::once(&line.marks.own).chain(line.marks.pair.as_ref()))
            .filter(|marks| marks.side == side)
            .count();
        i32::try_from(count).unwrap_or(i32::MAX)
    }

    // ---- the reader's own selection of the text (`selection`) --------

    /// Which byte of row `row`'s source line the place `at` (in
    /// `LineRuler`'s units) stands on.
    #[qslot]
    fn source_byte_at(&self, side: i32, row: i32, at: i32) -> i32 {
        self.byte_at(side, row, at)
    }

    /// A press landed: the selection starts here, empty.
    #[qslot]
    fn begin_select(&mut self, side: i32, row: i32, at: i32) {
        if self.start_select(side, row, at) {
            self.changed();
        }
    }

    /// The pointer moved during a drag (`drag_select`).
    #[qslot]
    fn extend_select(&mut self, side: i32, row: i32, at: i32) {
        if self.drag_select(side, row, at) {
            self.changed();
        }
    }

    /// One whole row becomes the selection — a right-click outside
    /// whatever was selected (デザイン規約 §diff の中身をコピーする).
    #[qslot]
    fn select_row(&mut self, side: i32, row: i32) {
        if self.select_whole_row(side, row) {
            self.changed();
        }
    }

    /// Whether a place in the text is inside the selection — what a
    /// right-click asks before taking its own row instead. A place in the
    /// other column is outside it.
    #[qslot]
    fn selection_holds(&self, side: i32, row: i32, at: i32) -> bool {
        self.holds(side, row, at)
    }

    /// Where the selection's moving end stands (`moving_end`), or nothing
    /// where none stands.
    #[qslot]
    fn selection_end(&self) -> Ended {
        self.moving_end()
    }

    #[qslot]
    fn clear_select(&mut self) {
        if self.drop_selection() {
            self.changed();
        }
    }

    /// What the plain `Copy` puts on the clipboard: the lines of the
    /// selection's column it takes (`takes`), cut at the two ends, in the
    /// file's own bytes.
    #[qslot]
    fn selection_text(&self) -> String {
        self.copied_new()
    }

    /// What `Copy removed lines` puts there (`copied_removed`).
    #[qslot]
    fn removed_text(&self) -> String {
        self.copied_removed()
    }

    #[qslot]
    fn clear(&mut self) {
        self.current_key = String::new();
        self.widest_no = 0;
        self.widest_lines = Candidates::default();
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
        // Kept past here, a closed pane holds the whole diff's heap.
        self.shown = None;
        self.shown_has_preview = false;
        self.shown_marks = Default::default();
        self.shown_colors = Default::default();
        self.apply_endings(None);
        self.apply_preview(None);
        // No next read is coming to sweep the picture files.
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
        let mine = wanted(feed.drain(), &self.current_key);
        if mine.is_empty() {
            return;
        }
        let carries_rows = mine.iter().any(|m| matches!(m, DiffMsg::Loaded { .. }));
        // Only a batch carrying rows takes it — the colours are a second
        // message about a read already answered.
        let asked = carries_rows.then(|| self.requested_at.take()).flatten();
        // Only where there is a place to lose: rows for an empty pane are
        // a pane opening.
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
                    self.shown_colors = Default::default();
                    self.coloured = false;
                    self.lay_out_rows(&Default::default());
                }
                DiffMsg::Coloured {
                    colors, settled, ..
                } => {
                    // A deep diff's quick first answer is a stand-in; only
                    // the settled colours set the flag.
                    if settled {
                        self.coloured = true;
                    }
                    self.repaint_rows(&colors);
                    self.shown_colors = colors;
                }
            }
        }
        self.changed();
        if let Some(t0) = asked {
            tracing::info!(
                elapsed_ms = t0.elapsed().as_millis() as u64,
                "diff rows applied"
            );
        }
    }
}
qml_register!(DiffModel, "DiffModel", singleton = false);
