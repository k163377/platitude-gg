use super::*;

#[qobject(Base = QAbstractItemModel, ConvertToCamelCase, NoQmlElement)]
impl NavSectionModel {
    qproperty!("total", Member = total, Notify = changed);
    qproperty!("shownRows", Member = shown_total, Notify = changed);
    qproperty!("runFiles", Member = run_files, Notify = changed);
    qproperty!("headName", Member = head_name, Notify = changed);
    qproperty!("headOid", Member = head_oid, Notify = changed);
    qproperty!("headHasRemote", Member = head_has_remote, Notify = changed);
    qproperty!("headHasPr", Member = head_has_pr, Notify = changed);
    qproperty!("headAhead", Member = head_ahead, Notify = changed);
    qproperty!("headBehind", Member = head_behind, Notify = changed);
    qproperty!(
        "headUpstreamGone",
        Member = head_upstream_gone,
        Notify = changed
    );
    qproperty!("headRow", Member = head_row, Notify = changed);
    qproperty!("headShownName", Member = head_shown, Notify = changed);
    qproperty!("headUnderRow", Member = head_under_row, Notify = changed);
    qproperty!("headDepth", Member = head_depth, Notify = changed);
    qproperty!("refsLoaded", Member = refs_loaded, Notify = changed);
    qproperty!("carriedAt", Member = carried_at, Notify = changed);
    qproperty!("carriedName", Member = carried_name, Notify = changed);
    qproperty!("treeView", Member = tree_view, Notify = changed);
    qproperty!(
        "pointedEolPath",
        Member = pointed_eol_path,
        Notify = changed
    );
    qproperty!(
        "pointedEolKind",
        Member = pointed_eol_kind,
        Notify = changed
    );
    qproperty!(
        "pointedEolFrom",
        Member = pointed_eol_from,
        Notify = changed
    );
    qproperty!("pointedEolTo", Member = pointed_eol_to, Notify = changed);
    qproperty!(
        "pointedEolLines",
        Member = pointed_eol_lines,
        Notify = changed
    );
    qproperty!(
        "pointedEolScope",
        Member = pointed_eol_scope,
        Notify = changed
    );
    qproperty!("pointedEolExt", Member = pointed_eol_ext, Notify = changed);

    /// Names the row the pointer is on, so its line-ending sentence can be
    /// built; an empty path clears it. Automation writes it too, since
    /// hover cannot be injected headless.
    #[qslot]
    fn point_eol(&mut self, path: String) {
        let notice = self
            .eol_marks
            .iter()
            .find(|m| m.path == path)
            .map(|m| &m.notice);
        let words = crate::encode::ending_words(notice);
        if path == self.pointed_eol_path
            && words.kind == self.pointed_eol_kind
            && words.from == self.pointed_eol_from
            && words.to == self.pointed_eol_to
            && words.lines == self.pointed_eol_lines
            && words.scope == self.pointed_eol_scope
            && words.ext == self.pointed_eol_ext
        {
            return;
        }
        self.pointed_eol_path = path;
        self.pointed_eol_kind = words.kind;
        self.pointed_eol_from = words.from;
        self.pointed_eol_to = words.to;
        self.pointed_eol_lines = words.lines;
        self.pointed_eol_scope = words.scope;
        self.pointed_eol_ext = words.ext;
        self.changed();
    }

    #[qsignal]
    pub(super) fn changed(&mut self);

    // A refs snapshot arrived, whether or not it moved anything. Not
    // `changed`, which stays quiet on identical rows: a write that leaves
    // the refs as they were (a cherry-pick of a commit the branch already
    // has) still owes its landing (`RepoPage.tryPendingHeadSelect`).
    #[qsignal]
    pub(super) fn refs_settled(&mut self);

    /// A new refs snapshot arrived (a republished `Arc` stays quiet). What
    /// expensive re-derivations wait on: `refs_settled` fires every status
    /// tick, and a git spawn off it would poll at the tick rate.
    #[qsignal]
    pub(super) fn refs_moved(&mut self);

    /// A stash listing arrived, whether or not it moved anything.
    ///
    /// Separate from `refs_settled`: a write asks for the stashes only
    /// after rebuilding the graph (`session::write`), so reading the refs'
    /// arrival as theirs puts a dropped stash back on screen for the
    /// rebuild (デザイン規約 §消す操作は先に画面から消す).
    #[qsignal]
    pub(super) fn stashes_settled(&mut self);

    /// A worktree listing arrived, whether or not it moved anything — a
    /// listing of its own, as the stashes' is: the copy a removal took
    /// away waits on it.
    #[qsignal]
    pub(super) fn worktrees_settled(&mut self);

    /// Wires this instance to one section's data feed. `section`:
    /// `branches` / `remotes` / `worktrees` / `stashes` / `tags`. The
    /// working tree's changed files come through [`Self::attach_worktree`].
    #[qslot]
    fn attach_section(&mut self, tab_id: i32, section: String) {
        self.attach_section_feed(tab_id, section);
    }

    /// Wires this instance to one bucket run of the working tree's changed
    /// files — `conflicts` / `unstaged` / `staged`, each a list with its
    /// own share and scroll of the WIP pane.
    ///
    /// Only what is **shown** is one run's: every instance holds the whole
    /// status, so a page can ask any of them about any file.
    #[qslot]
    fn attach_worktree(&mut self, tab_id: i32, run: String) {
        self.attach_worktree_feed(tab_id, run);
    }

    /// Wires this instance to **another** working copy's changed files, as
    /// one list (why: `attach_carried_feed`).
    #[qslot]
    fn attach_carried(&mut self, tab_id: i32) {
        self.attach_carried_feed(tab_id);
    }

    /// The tab now stands in another working copy (`Hub::restand_tab`): a
    /// file list goes back to waiting, not to empty — no rows would read
    /// as a clean tree of a copy not yet read (`Source::Waiting`).
    ///
    /// Not for the sections a repository owns (branches, remotes, tags,
    /// stashes, worktrees): linked copies share them, so they stay drawn.
    #[qslot]
    fn restand(&mut self) {
        self.take(Source::Waiting);
        self.eol_marks = Arc::default();
        self.carried_at = String::new();
        self.carried_name = String::new();
        self.reshape();
        self.changed();
    }

    #[qslot]
    fn drain(&mut self) {
        self.take_feeds();
    }

    /// Shows a row as already gone: the name the page has just asked git
    /// to delete — one per list, a delete touching at most one of each
    /// kind — or empty to put it back (デザイン規約 §消す操作は先に画面から消す).
    /// When it goes back is the page's call (`hidden`); a name no row here
    /// carries changes nothing.
    #[qslot]
    fn set_hidden(&mut self, name: String) {
        let hidden: Vec<String> = if name.is_empty() {
            Vec::new()
        } else {
            vec![name]
        };
        if self.hidden == hidden {
            return;
        }
        self.hidden = hidden;
        self.reshape();
        self.changed();
    }

    #[qslot]
    fn set_filter(&mut self, filter: String) {
        if self.filter != filter {
            self.filter = filter;
            self.reshape();
            self.changed();
        }
    }

    /// Opens/closes one folder row (key = its path, e.g. `origin/feature`).
    #[qslot]
    fn toggle_folder(&mut self, key: String) {
        let depth = key.matches('/').count() as i32;
        let current = self.folder_expanded(&key, depth);
        self.folder_overrides.insert(key, !current);
        self.reshape();
        self.changed();
    }

    /// Switches the worktree list between tree and flat-path display.
    #[qslot]
    fn set_tree_view(&mut self, tree: bool) {
        if self.tree_view == tree {
            return;
        }
        self.tree_view = tree;
        self.reshape();
        self.changed();
    }

    /// Rows on screen, as `shownRows` counts them.
    #[qslot]
    fn shown(&self) -> i32 {
        self.shown_rows() as i32
    }

    /// Commit id of the ref with this name; empty when there is none. The
    /// ref sections answer through the snapshot's name index (tens of
    /// thousands of rows); the rest are short lists.
    #[qslot]
    pub(super) fn oid_of_name(&self, name: String) -> String {
        match &self.all {
            Source::Locals(_) | Source::Remotes(_) => self
                .all
                .branch_named(&name)
                .map(|branch| branch.oid.to_hex())
                .unwrap_or_default(),
            Source::Tags(_) => self
                .all
                .tag_named(&name)
                .map(|tag| tag.oid.to_hex())
                .unwrap_or_default(),
            _ => self.told(Role::Name, &name, Role::OidHex),
        }
    }

    /// The branches `remote` carries here, by the name half alone — what
    /// the upstream question offers (`Source::branches_on`). Empty for
    /// every section but the remotes; the whole section, whatever the
    /// sidebar filters to. `remote_names` is the configured names, for the
    /// cut `GitFacts.remoteOfRef` makes.
    #[qslot]
    pub(super) fn branches_on(&self, remote: String, remote_names: Vec<String>) -> Vec<String> {
        let names: Vec<&str> = remote_names.iter().map(String::as_str).collect();
        self.all
            .branches_on(&remote, &names)
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    /// One card of the operation panel's branch nest: the rows filed
    /// directly under `path`, the top for an empty one (`level_rows`). Read
    /// as each card opens (デザイン規約 §メニュー).
    #[qslot]
    fn card_level(&self, path: String) -> CardRows {
        CardRows::new(self.level_rows(&path))
    }

    /// The operation panel's WORKTREE card (`copy_rows`), read as it opens
    /// (デザイン規約 §メニュー).
    #[qslot]
    fn copy_card(&self) -> CopyRows {
        CopyRows::new(self.copy_rows())
    }

    /// The copy at `path` as its row says it (`copy_of`) — what the
    /// WORKTREE card of a menu stands on, read as the menu opens.
    /// `undefined` for a path not listed, or listed as going.
    #[qslot]
    fn copy_facts(&self, path: String) -> Optional<CopyRow> {
        Optional::new(self.copy_of(&path))
    }

    /// Where a new copy for `branch` would be made and whether git would
    /// make it there (`new_copy`) — what `Create worktree here…`'s box and
    /// the `worktree add` row stand on. `undefined` before the listing
    /// names the repository's own copy.
    #[qslot]
    fn new_copy_for(&self, branch: String) -> Optional<NewCopy> {
        Optional::new(self.new_copy(&branch))
    }

    /// Whether new copies have a place at all (`has_place_for_copies`) —
    /// not before the listing lands, nor for a bare repository's linked
    /// copies.
    #[qslot]
    fn copies_placed(&self) -> bool {
        self.has_place_for_copies()
    }

    /// The path of the copy a folder's chip names: `name` standing on
    /// `head` (`copy_standing`); empty where none does.
    #[qslot]
    fn copy_at(&self, name: String, head: String) -> String {
        self.copy_standing(&name, &head)
    }

    /// Which row on show this ref sits on; -1 when it is on none. What a
    /// list asks before scrolling to a row nobody clicked on.
    #[qslot]
    fn row_of_name(&self, name: String) -> i32 {
        self.row_of(&name)
    }

    /// The remote branch this one speaks for (`origin/main`); empty when
    /// it speaks for none, and for every section but the branches.
    #[qslot]
    fn upstream_of(&self, name: String) -> String {
        self.all
            .branch_named(&name)
            .map(|branch| branch.upstream.as_str().to_string())
            .unwrap_or_default()
    }

    /// The other way round: the local branch measured against this
    /// remote-tracking ref (`BranchItem::tracked_by`). Empty where
    /// nothing here names it, and for every section but the remotes.
    /// What a remote row's opened lines lead with (デザイン規約
    /// §左メニューの所作); the counts beside it ride the row (`Role::Ahead`).
    #[qslot]
    pub(super) fn tracked_by(&self, name: String) -> String {
        self.all
            .branch_named(&name)
            .map(|branch| branch.tracked_by.as_str().to_string())
            .unwrap_or_default()
    }

    /// The reading this branch is measured against and cannot reach —
    /// git's `[gone]` (`BranchItem::upstream_gone`). Empty where the branch
    /// tracks nothing or what it tracks is here, and for every section but
    /// the branches.
    ///
    /// A name, as the row's own slot carries it (`models::nav::field` の
    /// `Role::Bucket`). Asked by rows naming somebody else's branch — a
    /// working copy's row says of its branch what the BRANCHES row would
    /// (デザイン規約 §左メニューの所作).
    #[qslot]
    pub(super) fn upstream_gone_of(&self, name: String) -> String {
        self.all
            .branch_named(&name)
            .map(|branch| branch.upstream_gone.as_str().to_string())
            .unwrap_or_default()
    }

    /// How far the branch of this name stands from what it reads, as of
    /// the last fetch — the pair its own row draws (`models::nav::field` の
    /// `Role::Ahead`). Zero where level, untracked, or not in this section.
    ///
    /// For rows naming somebody else's branch, e.g. a working copy's
    /// (デザイン規約 §左メニューの所作). A slot, not a role: only this
    /// section can answer for a name, so it is read once as the row opens
    /// and a fetch landing meanwhile does not move it.
    #[qslot]
    pub(super) fn ahead_of(&self, name: String) -> i32 {
        self.all
            .branch_named(&name)
            .map_or(0, |branch| super::field::counted(branch.ahead))
    }

    /// The other half of that pair — how far it is behind.
    #[qslot]
    pub(super) fn behind_of(&self, name: String) -> i32 {
        self.all
            .branch_named(&name)
            .map_or(0, |branch| super::field::counted(branch.behind))
    }

    /// The commit this branch's configured upstream stands on, hex — the
    /// reference point `branch --delete` measures the tip against
    /// (`BranchItem::upstream_oid`); empty where there is none, and git
    /// then measures against HEAD. Handed to `GraphModel.branchDeleteMerged`
    /// as the menu opens.
    #[qslot]
    fn upstream_oid_of(&self, name: String) -> String {
        self.all
            .branch_named(&name)
            .and_then(|branch| branch.upstream_oid)
            .map(|oid| oid.to_hex())
            .unwrap_or_default()
    }

    /// Whether that reading stands on another commit
    /// (`session::BranchItem::upstream_drifted`). False where there is no
    /// pair, and for every section but the branches.
    ///
    /// Takes the remote delete rows out of the table (デザイン規約
    /// §左メニューの所作 の削除の表): a drifted reading is deleted from its
    /// own row.
    #[qslot]
    fn upstream_drifted(&self, name: String) -> bool {
        self.all
            .branch_named(&name)
            .is_some_and(|branch| branch.upstream_drifted)
    }

    /// Which sides the tag with this name stands on — `here` / `remote` /
    /// `both`, empty when this section holds no such row
    /// (`platitude_core::offers::TagSides`). A tag has no namespace, so
    /// one row carries both sides, and `tag --delete` / `push --delete`
    /// each need one of them.
    #[qslot]
    pub(super) fn tag_sides(&self, name: String) -> String {
        if name.is_empty() || self.section != "tags" {
            return String::new();
        }
        let Some(tag) = self.all.tag_named(&name) else {
            return String::new();
        };
        match (!tag.here, tag.has_remote) {
            (true, _) => "remote".to_string(),
            (false, true) => "both".to_string(),
            (false, false) => "here".to_string(),
        }
    }

    /// Where `remote` carries this tag when that is not where this
    /// repository has it; empty when they agree or that remote has never
    /// been heard to carry the name. Asked of the tags section.
    ///
    /// Decides the menu's push row as it opens: a plain push to a name the
    /// remote has elsewhere is refused, so the row becomes the leased
    /// overwrite pinned to this commit (デザイン規約 §相手の履歴を置き換える).
    #[qslot]
    pub(super) fn remote_tag_drift(&self, name: String, remote: String) -> String {
        if name.is_empty() || remote.is_empty() {
            return String::new();
        }
        self.all.tag_drift(&name, &remote)
    }

    /// Whether the reading of this tag standing on `oid` (hex) stands
    /// apart from the right one (`RefsSnapshot::tag_apart_at` — `against`
    /// decides where it carries the name, else the remotes by agreeing) —
    /// the copy here asked with its own commit (an open TAGS row), a graph
    /// row's reading with that row's (the card a chip unfolds into). False
    /// where no remote carries the name, and for an `oid` that is no commit
    /// id. Asked of the tags section.
    ///
    /// The one test behind every holder that wears the warning
    /// (デザイン規約 §左メニューの所作), the copy here included — the same one
    /// `tag_remotes` marks its carriers by, so the two agree whatever
    /// `against` is.
    #[qslot]
    pub(super) fn tag_apart_at(&self, name: String, oid: String, against: String) -> bool {
        if name.is_empty() || self.section != "tags" {
            return false;
        }
        let Ok(commit) = platitude_core::Oid::from_hex_str(oid.trim()) else {
            return false;
        };
        self.all.tag_apart_at(&name, commit, &against)
    }

    /// Who the right reading of this tag belongs to, as the sentence of a
    /// holder apart from it names them — `against` where it carries the
    /// name, else every remote agreeing on one commit (`fork, mirror`).
    /// Empty where the remotes disagree and none of them decides, and
    /// before the remotes are read. Asked of the tags section.
    #[qslot]
    pub(super) fn tag_weighed_against(&self, name: String, against: String) -> String {
        if name.is_empty() || self.section != "tags" {
            return String::new();
        }
        self.all.tag_weighed_against(&name, &against)
    }

    /// What a menu on this tag stands on, read as it opens (`RefTagMenu`):
    /// where the push goes and what it is leased to, which remote the
    /// deletes reach and why they stand greyed, and whether the row goes
    /// with them (`RefsSnapshot::tag_menu`). `aim` is a remote the reader
    /// named on its own — a carrier's line, a chip drawing that remote's
    /// reading (`tag_aim_at`) — or empty. Asked of the tags section.
    #[qslot]
    pub(super) fn tag_menu(&self, name: String, against: String, aim: String) -> One<TagMenu> {
        if name.is_empty() || self.section != "tags" {
            return One::new(TagMenu {
                push_remote: against,
                ..TagMenu::default()
            });
        }
        One::new(self.all.tag_menu(&name, &against, &aim))
    }

    /// The one remote a chip of this tag at `oid` (hex) draws the reading
    /// of — so a menu opened on that chip names it on its own. Empty for
    /// the copy here, for a reading several remotes share, and for an
    /// `oid` that is no commit id. Asked of the tags section.
    #[qslot]
    pub(super) fn tag_aim_at(&self, name: String, oid: String) -> String {
        if name.is_empty() || self.section != "tags" {
            return String::new();
        }
        let Ok(commit) = platitude_core::Oid::from_hex_str(oid.trim()) else {
            return String::new();
        };
        self.all.tag_aim_at(&name, commit)
    }

    /// The remotes carrying this tag, one record each in name order
    /// (`TagCarrier`) — what the row opens on (デザイン規約 §左メニューの所作).
    /// Empty where no remote has the name, and until a fetch has been
    /// through (`remote::tags::list_tags`). Asked of the tags section.
    ///
    /// `apart` is read against the right reading, which `against` — the
    /// remote this window's tag rows act on (`RepoTab.defaultRemote`;
    /// デザイン規約 §タグを作る・送る) — decides where it carries the name, and
    /// the remotes by agreeing where it does not; never the copy here: a
    /// local tag that is itself the odd one out would put the mark on
    /// everybody else.
    #[qslot]
    pub(super) fn tag_remotes(&self, name: String, against: String) -> TagCarriers {
        if name.is_empty() || self.section != "tags" {
            return TagCarriers::default();
        }
        TagCarriers::new(
            self.all
                .tag_carriers(&name, &against)
                .into_iter()
                .map(|(remote, apart)| TagCarrier {
                    remote: remote.to_string(),
                    apart,
                })
                .collect(),
        )
    }

    /// The other working copy holding this branch, by the path git lists
    /// it under; empty when no other one has it out.
    ///
    /// git refuses to switch to or delete a branch another worktree has
    /// checked out, locked or not (a lock stops only `worktree remove` /
    /// `move`), so the rows that would try ask this first.
    ///
    /// Held to the worktrees section: a file row keeps its bucket name in
    /// the slot a worktree row keeps its branch in.
    #[qslot]
    pub(super) fn worktree_holding(&self, branch: String) -> String {
        self.worktree_with(&branch)
    }

    /// Where the working copy at `path` is standing — the HEAD git listed
    /// with the entry, as hex; empty for a path this listing does not hold
    /// and for a bare entry.
    ///
    /// By path, not name: two copies can share a leaf name under different
    /// parents (`RepoPage.carriedPath`). What the page lands on when the
    /// copy it was reading commits (`settleCarriedAfterPass`).
    #[qslot]
    pub(super) fn head_of_copy(&self, path: String) -> String {
        self.told(Role::Full, &path, Role::OidHex)
    }

    /// What one row shows, and what git knows it by (empty out of range) —
    /// how automation reaches a row, roles being visible only to a delegate.
    #[qslot]
    pub(super) fn name_at(&self, row: i32) -> String {
        self.shows(row, Role::Name)
    }

    #[qslot]
    fn full_at(&self, row: i32) -> String {
        self.shows(row, Role::Full)
    }

    /// When the listing on screen looked (`looked`): at or above a write's
    /// `RepoTab.writeAnswerReadsFrom`, these rows saw what it left.
    #[qslot]
    fn listing_looked(&self) -> i32 {
        i32::try_from(self.looked).unwrap_or(i32::MAX)
    }

    /// The file row at `row` keyed the way the WIP pane holds its choice
    /// — `<bucket>:<path>`, empty for a folder row and past the end. How
    /// the choice reaches rows with no delegate built
    /// ([`NavSectionModel::file_key`]).
    #[qslot]
    fn file_key_at(&self, row: i32) -> String {
        usize::try_from(row)
            .map(|at| self.file_key(at))
            .unwrap_or_default()
    }

    /// The other way round: what the row git knows by this name shows —
    /// for a stash, the message behind `stash@{0}`, which a pop reads while
    /// the entry is still there. Asked of every entry, whatever the filter.
    #[qslot]
    fn name_of_full(&self, full: String) -> String {
        self.told(Role::Full, &full, Role::Name)
    }

    /// Local branch name a remote-tracking ref would take. The remotes
    /// snapshot owns both the visible row and its configured prefixes.
    #[qslot]
    fn local_name_for(&self, remote_ref: String) -> String {
        self.local_name_of(&remote_ref)
    }

    /// The two stage letters git reports for a working-tree path (`UU`,
    /// `DU`, …); empty for any path that is not in this section. By path,
    /// all the diff pane holds: a conflict git prints no patch for is
    /// described by what the two sides did.
    #[qslot]
    pub(super) fn change_of(&self, path: String) -> String {
        self.told(Role::Full, &path, Role::Change)
    }

    /// Which bucket holds a path now. Asked after a write has moved it,
    /// when it sits in exactly one of them.
    #[qslot]
    fn bucket_of(&self, path: String) -> String {
        self.told(Role::Full, &path, Role::Bucket)
    }

    /// Whether `bucket` still holds `path` (see [`Self::holds`]).
    #[qslot]
    fn holds_path(&self, bucket: String, path: String) -> bool {
        self.holds(&bucket, &path)
    }

    /// The row beside `path` under the same heading, as `<bucket>:<path>`
    /// (see [`Self::beside`]). What the diff pane reads next when the file
    /// it is on leaves the side being read.
    #[qslot]
    fn beside_path(&self, bucket: String, path: String) -> String {
        self.beside(&bucket, &path)
    }

    /// The file row the arrows land on, `way` steps from `path` in `bucket`
    /// (see [`Self::step`]). What the file list's own arrows walk.
    #[qslot]
    fn step_file(&self, bucket: String, path: String, way: i32) -> Landed {
        self.step(&bucket, &path, way)
    }

    /// Which row on screen this list is showing `path` in `bucket` on; -1
    /// when it is showing it on none (see [`Self::row_of_file`]). How the
    /// pane's walk tells which of its bucket lists it is standing in.
    #[qslot]
    fn row_of_file_in(&self, bucket: String, path: String) -> i32 {
        self.row_of_file(&bucket, &path)
    }

    /// The file row at the end of this list a walk going `way` comes in by
    /// (see [`Self::edge`]). What the arrows land on when they cross out of
    /// the bucket above or below.
    #[qslot]
    fn edge_file(&self, way: i32) -> Landed {
        self.edge(way)
    }

    /// Where a renamed file came from, by path — whole, since a rename's
    /// diff names both sides. A headless run has only the path, and the
    /// destination alone reads to git as a file out of nowhere.
    ///
    /// The first row of that path naming a source: a renamed-then-edited
    /// file has a row on each side, and only the staged one knows.
    #[qslot]
    fn orig_of(&self, path: String) -> String {
        self.orig_path_of(&path)
    }
}
qml_register!(NavSectionModel, "NavSectionModel", singleton = false);
