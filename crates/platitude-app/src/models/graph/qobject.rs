//! Everything QML sees of the graph: the properties it binds to, the
//! feed it drains, and the questions it asks of the loaded rows.
//!
//! One `#[qobject]` block, and it cannot be split further
//! (structure.md「1 型 1 ファイルから動かせない」).

use super::*;

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl GraphModel {
    qproperty!("loading", Member = loading, Notify = stats_changed);
    // Keys (`encode::Chip::key`) of chips shown as gone while git deletes
    // their refs (デザイン規約 §消す操作は先に画面から消す). Each list holds its
    // own, as the sidebar's sections do (`NavSectionModel::set_hidden`); the
    // rows keep the names until the walk after the delete lands.
    qproperty!("goneChips", Member = gone_chips, Notify = stats_changed);
    qproperty!("rowTotal", Member = row_total, Notify = stats_changed);
    qproperty!(
        "carriedRevision",
        Member = carried_revision,
        Notify = stats_changed
    );
    qproperty!("walkedTotal", Member = walked_total, Notify = stats_changed);
    qproperty!("maxLanes", Member = max_lanes, Notify = stats_changed);
    qproperty!(
        "firstChunkMs",
        Member = first_chunk_ms,
        Notify = stats_changed
    );
    qproperty!("totalMs", Member = total_ms, Notify = stats_changed);
    qproperty!("truncated", Member = truncated, Notify = stats_changed);
    // Properties for the reason `matchCount` is: a background refresh
    // moves them unasked (its landing takes the wait back off).
    qproperty!("windowStep", Member = window_step, Notify = stats_changed);
    qproperty!("growing", Member = growing, Notify = stats_changed);
    qproperty!("finishCount", Member = finish_count, Notify = stats_changed);
    qproperty!("wipRow", Member = wip_row, Notify = stats_changed);
    qproperty!("carriedTop", Member = carried_top, Notify = stats_changed);
    qproperty!("resetCount", Member = reset_count, Notify = stats_changed);
    // How many loaded rows the find bar's line is in. A property because a
    // background refresh re-marks the rows with nobody typing
    // (app-ui.md「QML バインディングはプロパティにしか反応しない」).
    qproperty!("matchCount", Member = match_count, Notify = stats_changed);
    // Properties for the reason `matchCount` is.
    qproperty!("searching", Member = searching, Notify = stats_changed);
    qproperty!(
        "shortOfAnOid",
        Member = short_of_an_oid,
        Notify = stats_changed
    );
    qproperty!(
        "firstMatched",
        Member = first_matched,
        Notify = stats_changed
    );
    // The three wire values through getters: a value QML wrote would not
    // reach Rust (encode::wire_qml).
    qproperty!("tailGeometry", Read = tail_geometry, Notify = stats_changed);
    fn tail_geometry(&self) -> &crate::encode::Lanes {
        &self.tail_geometry
    }
    qproperty!("tailMatched", Member = tail_matched, Notify = stats_changed);
    // What the stand-in draws — properties for the reason `head.rs` gives.
    qproperty!("headRow", Member = head_row, Notify = stats_changed);
    qproperty!("headLabels", Read = head_labels, Notify = stats_changed);
    fn head_labels(&self) -> &crate::encode::Chips {
        &self.head_labels
    }
    qproperty!("headSubject", Member = head_subject, Notify = stats_changed);
    qproperty!("headColor", Member = head_color, Notify = stats_changed);
    qproperty!("headLane", Member = head_lane, Notify = stats_changed);
    qproperty!("headGeometry", Read = head_geometry, Notify = stats_changed);
    fn head_geometry(&self) -> &crate::encode::Lanes {
        &self.head_geometry
    }
    qproperty!("headAvatar", Member = head_avatar, Notify = stats_changed);
    qproperty!("headMatched", Member = head_matched, Notify = stats_changed);
    qproperty!(
        "headAvatarUrl",
        Member = head_avatar_url,
        Notify = stats_changed
    );
    qproperty!("error", Member = error, Notify = stats_changed);
    // One badge stands for both (`BandStateGroup.staleBadgeShown`).
    qproperty!("failed", Member = failed, Notify = stats_changed);
    qproperty!("stale", Member = stale, Notify = stats_changed);
    // The discard log's picked entry on the graph (`provisional.rs`): whether
    // one is shown, the first and last of the rows it would bring back and
    // its old tip's row (-1 for none loaded), and the revision a delegate
    // reads beside `provisionalAt`.
    qproperty!(
        "provisionalOn",
        Member = provisional_on,
        Notify = stats_changed
    );
    qproperty!(
        "provisionalFirst",
        Member = provisional_first,
        Notify = stats_changed
    );
    qproperty!(
        "provisionalLast",
        Member = provisional_last,
        Notify = stats_changed
    );
    qproperty!(
        "provisionalTipRow",
        Member = provisional_tip_row,
        Notify = stats_changed
    );
    qproperty!(
        "provisionalRevision",
        Member = provisional_revision,
        Notify = stats_changed
    );
    // A walk that took the entry's tips has answered: a tip row still -1 is
    // past the window.
    qproperty!(
        "provisionalWalked",
        Member = provisional_walked,
        Notify = stats_changed
    );

    #[qsignal]
    pub(super) fn stats_changed(&mut self);

    /// Puts the discard log's picked entry on the graph: its parts' tips,
    /// the first the one it lands on, how each draws (`uncommitted` /
    /// `stash` / empty for a commit), the commits only they reach, and where
    /// a copy on a base made for it draws — `<tip> <base> <commit>`, full
    /// hex (`DiscardModel.tipsAt` / `looksAt` / `lostAt` / `standsAt`).
    #[qslot]
    fn show_discard(
        &mut self,
        tips: Vec<String>,
        looks: Vec<String>,
        lost: Vec<String>,
        stands: Vec<String>,
    ) {
        self.show_provisional(tips, looks, lost, &stands);
    }

    /// Takes the entry off the graph.
    #[qslot]
    fn hide_discard(&mut self) {
        self.hide_provisional();
    }

    /// Whether the row is one the shown entry would bring back. Read beside
    /// `provisionalRevision`, for the reason `carriedName` is read beside
    /// `carriedRevision`.
    #[qslot]
    fn provisional_at(&self, row: i32) -> bool {
        usize::try_from(row).is_ok_and(|row| self.provisional_row(row))
    }

    /// Whether the row is the shown entry's copy of thrown-away work, which
    /// draws as an uncommitted row does. Read beside `provisionalRevision`.
    #[qslot]
    fn provisional_wip_at(&self, row: i32) -> bool {
        usize::try_from(row).is_ok_and(|row| self.provisional_tip_drawn_as(row, "uncommitted"))
    }

    /// Whether the row is the shown entry's dropped stash, which draws as a
    /// stash does. Read beside `provisionalRevision`.
    #[qslot]
    fn provisional_stash_at(&self, row: i32) -> bool {
        usize::try_from(row).is_ok_and(|row| self.provisional_tip_drawn_as(row, "stash"))
    }

    /// Names the refs whose chips to leave undrawn, at most one per kind
    /// (a delete touches at most one of each); empty ones put theirs back,
    /// as a refused delete does. The keys are built here — what a chip
    /// answers to is not the page's business.
    #[qslot]
    fn set_gone(&mut self, branch: String, remote: String, tag: String) {
        let keys = crate::encode::gone_keys(&branch, &remote, &tag);
        if self.gone_chips == keys {
            return;
        }
        self.gone_chips = keys;
        self.stats_changed();
    }

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        let invoker = self.get_qml_method_invoker();
        self.feed = crate::hub::attach_feed(tab_id, |f| &f.graph, invoker);
    }

    /// The tab now stands in another working copy of the same repository,
    /// with a swapped session (`Hub::restand_tab`).
    ///
    /// The rows stay: linked copies share every commit and ref, and the
    /// new session's first pass arrives as one replacement that keeps the
    /// reader's place ([`FirstPass::Swapped`]). So does `generation`: the
    /// new session counts on from this graph's record
    /// (`platitude_core::session::DrawnGraph`).
    ///
    /// [`FirstPass::Swapped`]: platitude_core::session::FirstPass::Swapped
    #[qslot]
    fn restand(&mut self) {
        // A grow still out was the old session's to answer, and it never
        // will: the footer would wait for as long as the page stands.
        self.growing = false;
        // The discard log's entry was the old session's to walk: the new one
        // walks without it, and every row would read as passed over.
        self.drop_provisional();
        self.stats_changed();
    }

    #[qslot]
    fn drain(&mut self) {
        self.take_feed();
    }

    /// The tail was pressed: load the next step of history. Refused for an
    /// uncut window and while a press is out — a second would cancel the
    /// first's walk and land the same commits.
    #[qslot]
    fn grow_window(&mut self) {
        if !self.truncated || self.growing {
            return;
        }
        self.growing = true;
        self.stats_changed();
        crate::hub::with_session(self.tab_id, |s| s.grow_log_window());
    }

    /// Automation: graph passes reaching `step` fail where they would
    /// have walked, and one is asked for in the same call. The fault goes
    /// in at the walk so the pass leaves by its ordinary reporting arm —
    /// how `STALE GRAPH` gets photographed (`graph-stopped` /
    /// `graph-stale`).
    ///
    /// The slot stands in every build and does nothing without the harness
    /// (rules-refs/app-ui.md「`#[qslot]` に `#[cfg]` は効かない」).
    #[qslot]
    fn fail_graph_pass(&mut self, step: String) {
        crate::harness::fail_graph_pass(self.tab_id, &step);
    }

    /// Automation: passes stop walking as if this window's first status
    /// had not arrived, and one is asked for in the same call. Answers
    /// whether the hold had been up, so a run cannot take a landing it
    /// never arranged for its own. Stands in every build, as
    /// [`Self::fail_graph_pass`] does.
    #[qslot]
    fn let_the_working_tree_row_through(&mut self) -> bool {
        crate::harness::let_the_working_tree_row_through(self.tab_id)
    }

    /// Automation: one more pass while the hold stays up; answers whether
    /// the hold was up. Ask it after a write: the hold keeps this window's
    /// row from appearing, which is what ordinarily brings the next pass (a
    /// stopped replay moves no branch), so a stopped operation's landing
    /// would get no pass to turn down. Stands in every build, as
    /// [`Self::fail_graph_pass`] does.
    #[qslot]
    fn walk_again_while_held(&mut self) -> bool {
        crate::harness::walk_again_while_held(self.tab_id)
    }

    /// Row index of a commit (sidebar jump); -1 when absent.
    #[qslot]
    fn row_of(&self, oid_hex: String) -> i32 {
        self.row_of_hex(&oid_hex).map_or(-1, |i| i as i32)
    }

    /// The ids of the rows between two places, ends included, in walk
    /// order — what a Shift click reaches. One crossing: a range can be the
    /// whole window, and asking per row would put a walk of the history on
    /// a click (CLAUDE.md §性能予算).
    #[qslot]
    fn oids_between(&self, from: i32, to: i32) -> Vec<String> {
        let (lo, hi) = if from <= to { (from, to) } else { (to, from) };
        let lo = usize::try_from(lo).unwrap_or(0);
        let Ok(hi) = usize::try_from(hi) else {
            return Vec::new();
        };
        self.rows
            .get(lo..=hi.min(self.rows.len().saturating_sub(1)))
            .unwrap_or_default()
            .iter()
            .map(|r| r.oid_hex.clone())
            .collect()
    }

    /// The ones of `ids` this graph still has, in walk order — a choice
    /// (held by id) after a background pass rewrote the rows. Walk order
    /// because both readings of a choice depend on it: a comparison
    /// measures from the older end, and a merged file list takes a path's
    /// status from the newest commit that touched it
    /// (`details::union_files`).
    #[qslot]
    fn present_oids(&self, ids: Vec<String>) -> Vec<String> {
        let mut held: Vec<(usize, String)> = ids
            .into_iter()
            .filter_map(|hex| Some((self.row_of_hex(&hex)?, hex)))
            .collect();
        held.sort_unstable();
        held.into_iter().map(|(_, hex)| hex).collect()
    }

    /// The rows a choice of commits names (`item::ChosenRow`), off the
    /// rows with no git run. The order handed in is the order handed back
    /// — [`Self::present_oids`]'s walk order, whatever order the presses
    /// came in. One index lookup per id: filtering the loaded rows would
    /// put a walk of the history on a click.
    #[qslot]
    fn chosen_rows(&self, ids: Vec<String>) -> ChosenRows {
        ChosenRows::new(
            ids.iter()
                .filter_map(|hex| self.rows.get(self.row_of_hex(hex)?))
                .map(ChosenRow::of)
                .collect(),
        )
    }

    /// The row "the newest commit" names, or -1 where the window holds
    /// none (`item::newest_commit_row` has the rule), for readers with only
    /// row numbers to go by.
    #[qslot]
    fn newest_commit_row(&self) -> i32 {
        super::item::newest_commit_row(&self.rows).map_or(-1, |i| i as i32)
    }

    /// What another working copy's uncommitted row shows beside its words:
    /// its six tallies, or none for every other row. By row index, not off
    /// the view — the view's tallies are this window's own.
    #[qslot]
    fn carried_tally(&self, row: i32) -> Optional<Tally> {
        usize::try_from(row)
            .ok()
            .and_then(|row| self.carried.get(&row))
            .map(|carried| carried.tally.clone())
            .unwrap_or_default()
    }

    /// Where the working copy a row is about lives — what a double-click
    /// on it opens in a tab of its own. Empty for every other row.
    #[qslot]
    fn carried_path(&self, row: i32) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|row| self.carried.get(&row))
            .map(|carried| carried.path.clone())
            .unwrap_or_default()
    }

    /// The name of the working copy a row is about, or empty where the
    /// row is not one of theirs — what the read-only gestures branch on.
    #[qslot]
    fn carried_name(&self, row: i32) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|row| self.carried.get(&row))
            .map(|carried| carried.name.clone())
            .unwrap_or_default()
    }

    /// Which row a working copy's uncommitted work stands on now, or -1
    /// (the copy has gone clean). Every such row carries git's all-zero id,
    /// as this window's own does, so a pane on one follows it across a
    /// rebuild by path (`RepoPage.settleCarriedAfterPass`).
    #[qslot]
    fn carried_row_of(&self, path: String) -> i32 {
        self.carried
            .iter()
            .find(|(_, carried)| carried.path == path)
            .and_then(|(row, _)| i32::try_from(*row).ok())
            .unwrap_or(-1)
    }

    /// Reflog selector when the commit is a stash row (empty otherwise).
    #[qslot]
    fn stash_ref_of(&self, oid_hex: String) -> String {
        self.row_of_hex(&oid_hex)
            .and_then(|i| self.rows.get(i))
            .map(|r| r.stash_ref.clone())
            .unwrap_or_default()
    }

    /// Whether a remote already has this commit — what the "rewrites
    /// published history" warnings read. Off the row's walk mark
    /// (`session::published`), so a menu has it in its opening frame; a
    /// `git rev-list` would land after the card and move its edge under the
    /// hand (デザイン規約 §メニュー). False for a commit no row carries —
    /// a menu only opens on a row.
    #[qslot]
    fn published_at(&self, oid_hex: String) -> bool {
        let Ok(oid) = Oid::from_hex_str(oid_hex.trim()) else {
            return false;
        };
        self.row_at(&oid).is_some_and(|row| self.published_row(row))
    }

    /// Whether rebasing HEAD onto this commit would rewrite one a remote
    /// already has — the `rewrites pushed commits` note the ref menu's
    /// `rebase` row wears. `<ref>..HEAD` is a set no single mark answers,
    /// so it is walked off the rows' order and marks; git would answer
    /// after the card is up (デザイン規約 §行が読む答えはどこから来るか).
    /// `head` comes from the asker off the one record
    /// (`WorkingTreeModel.headOid`), the HEAD the rest of the card is about.
    #[qslot]
    fn rebase_rewrites_published(&self, onto_oid_hex: String, head_oid_hex: String) -> bool {
        let (Ok(onto), Ok(head)) = (
            Oid::from_hex_str(onto_oid_hex.trim()),
            Oid::from_hex_str(head_oid_hex.trim()),
        ) else {
            return false;
        };
        platitude_core::publish::range_rewrites_published(self.walked_rows(), head, onto)
    }

    /// Whether `branch --delete` would go through for the branch whose
    /// tip is `tip_hex`, off the drawn rows: `yes` / `no`, or empty where
    /// the window cannot say — the menu then asks git
    /// (`RepoTab.checkBranchDelete`). Measured from git's reference
    /// (`publish::delete_reference`): `upstream_hex` where the listing
    /// resolves one, else `head_hex` (the one record's; empty before the
    /// first read).
    #[qslot]
    fn branch_delete_merged(
        &self,
        tip_hex: String,
        upstream_hex: String,
        head_hex: String,
    ) -> String {
        let Ok(tip) = Oid::from_hex_str(tip_hex.trim()) else {
            return String::new();
        };
        let reference = platitude_core::publish::delete_reference(
            Oid::from_hex_str(upstream_hex.trim()).ok(),
            Oid::from_hex_str(head_hex.trim()).ok(),
        );
        let Some(reference) = reference else {
            return String::new();
        };
        match self.reaches_between(reference, tip) {
            Some(true) => "yes".to_string(),
            Some(false) => "no".to_string(),
            None => String::new(),
        }
    }

    /// The lane colour of the row a ref sits on, as a graph palette index;
    /// -1 when no loaded row carries that name. A conflicted file's diff
    /// paints each side in its branch's graph colour.
    #[qslot]
    fn color_of_ref(&self, name: String) -> i32 {
        if name.is_empty() {
            return -1;
        }
        self.rows
            .iter()
            .find(|r| r.labels.iter().any(|chip| chip.name == name))
            .map_or(-1, |r| r.node_color)
    }

    /// The colour each side of a conflict is drawn in
    /// (`encode::conflict_side_colors` decides); two slots because a slot
    /// cannot hand back a pair.
    #[qslot]
    fn conflict_color_ours(&self, ours: String, theirs: String) -> i32 {
        self.conflict_colors(&ours, &theirs).0
    }

    #[qslot]
    fn conflict_color_theirs(&self, ours: String, theirs: String) -> i32 {
        self.conflict_colors(&ours, &theirs).1
    }

    /// Re-reads the assigned pictures onto the rows already loaded, in
    /// place as `remark_notified` does — not by re-walking: git knows
    /// nothing of an assignment.
    #[qslot]
    fn refresh_avatars(&mut self) {
        let avatars = crate::hub::AvatarUrls::current();
        let mut ranges: Vec<(usize, usize)> = Vec::new();
        for i in 0..self.rows.len() {
            let url = avatars.url_of(&self.rows[i].author_email);
            if self.rows[i].avatar_url != url {
                self.rows[i].avatar_url = url;
                crate::models::notify::push_run(&mut ranges, i);
            }
        }
        self.notify_runs(ranges);
    }

    /// How many loaded rows carry a picture. Automation only: QML cannot
    /// walk this model's rows, so a count there reads zero.
    #[qslot]
    fn avatar_row_count(&self) -> i32 {
        self.rows
            .iter()
            .filter(|row| !row.avatar_url.is_empty())
            .count() as i32
    }

    /// The authors of the loaded rows as the settings card lists them —
    /// `Name <address>`, one per address, sorted; a prefill goes first and
    /// is not repeated. Read when the card opens, off the rows — no git
    /// runs.
    #[qslot]
    fn author_choices(&self, prefill_name: String, prefill_email: String) -> Vec<String> {
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut listed: Vec<String> = Vec::new();
        for row in &self.rows {
            if row.author_email.is_empty()
                || row.author_email == prefill_email
                || !seen.insert(&row.author_email)
            {
                continue;
            }
            listed.push(format!("{} <{}>", row.author, row.author_email));
        }
        listed.sort();
        let mut out: Vec<String> = Vec::new();
        if !prefill_email.is_empty() {
            out.push(format!("{prefill_name} <{prefill_email}>"));
        }
        out.extend(listed);
        out
    }

    /// Puts the find bar's line to the rows, lighting the ones it is in.
    /// An empty or whitespace line is no search: the marks come off and
    /// `matchCount` goes to zero.
    #[qslot]
    fn set_find(&mut self, text: String) {
        let next = Query::new(&text);
        if next == self.query {
            return;
        }
        self.query = next;
        self.searching = self.query.is_some();
        self.short_of_an_oid = self.query.as_ref().is_some_and(Query::short_of_an_oid);
        self.remark_notified();
        self.stats_changed();
    }

    /// Row of the first match at or after `from`, wrapping to the first
    /// match of all when there is none below; -1 when nothing matches.
    /// Where an incremental search lands.
    #[qslot]
    fn match_from(&self, from: i32) -> i32 {
        self.match_rows()
            .find(|row| *row >= from)
            .or_else(|| self.match_rows().next())
            .unwrap_or(-1)
    }

    /// Row of the next match after `row`, wrapping past the end.
    #[qslot]
    fn match_after(&self, row: i32) -> i32 {
        self.match_rows()
            .find(|r| *r > row)
            .or_else(|| self.match_rows().next())
            .unwrap_or(-1)
    }

    /// Row of the previous match before `row`, wrapping past the start.
    #[qslot]
    fn match_before(&self, row: i32) -> i32 {
        self.match_rows()
            .rfind(|r| *r < row)
            .or_else(|| self.match_rows().next_back())
            .unwrap_or(-1)
    }

    /// Which match this row is, counting from 1; 0 when it is not one.
    /// The left half of the bar's count.
    #[qslot]
    fn match_ordinal(&self, row: i32) -> i32 {
        self.match_rows()
            .position(|r| r == row)
            .map_or(0, |i| i as i32 + 1)
    }

    /// Full commit id at a row.
    #[qslot]
    fn oid_at(&self, row: i32) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|i| self.rows.get(i))
            .map(|r| r.oid_hex.clone())
            .unwrap_or_default()
    }

    /// The chips of a row, as its delegate hands them to the chip column —
    /// read when a menu opens on the row (`CommitRowMenu`).
    #[qslot]
    fn labels_at(&self, row: i32) -> Chips {
        usize::try_from(row)
            .ok()
            .and_then(|i| self.rows.get(i))
            .map(|r| r.labels.clone())
            .unwrap_or_default()
    }

    /// The lanes of a row, spelled, for the smoke hooks: whether a lane is
    /// dotted is not a question a screenshot answers.
    #[qslot]
    fn geometry_at(&self, row: i32) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|i| self.rows.get(i))
            .map(|r| crate::encode::spell_lanes(&r.geometry))
            .unwrap_or_default()
    }
}
impl GraphModel {
    fn conflict_colors(&self, ours: &str, theirs: &str) -> (i32, i32) {
        crate::encode::conflict_side_colors(
            (self.color_of_ref(ours.to_string()), ours),
            (self.color_of_ref(theirs.to_string()), theirs),
        )
    }
}
