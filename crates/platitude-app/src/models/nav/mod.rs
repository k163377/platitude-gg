//! One section list (branches / remotes / files / worktrees /
//! stashes / tags). All render in the sidebar except `files` (the
//! changed files), which the right pane's WIP view shows.

use std::collections::HashMap;
use std::sync::Arc;

use qtbridge::qtbridge_type_lib::{
    QByteArray, QHash_i32_QByteArray, QModelIndex, QString, QVariant,
};
use qtbridge::{QAbstractItemModel, QAbstractItemModelBase, QModelItem, QmlObject, qobject};

use crate::encode::{Fields, Landed, Landing, Listed, One, Optional, Record, field};
use crate::hub::{CarriedStatusMsg, Feed, Hub, RefsMsg, StatusMsg, attached};

use super::pathtree::DirNode;
use super::qml_register;

/// One remote carrying a tag's name, and whether it stands somewhere
/// other than where the remote this window acts on has it
/// (`NavSectionModel::tag_remotes`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagCarrier {
    pub remote: String,
    pub apart: bool,
}

/// Every remote carrying the name, in name order.
pub type TagCarriers = Listed<TagCarrier>;

impl Record for TagCarrier {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        Fields::new()
            .put("remote", &self.remote)
            .put("apart", &self.apart)
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            remote: field(map, "remote")?,
            apart: field(map, "apart")?,
        })
    }
}

/// What a tag's menu stands on (`NavSectionModel::tag_menu`,
/// `platitude_core::session::TagMenuFacts`), in the words the card reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagMenu {
    pub push_remote: String,
    /// Hex; empty for the plain push.
    pub lease: String,
    pub reach: String,
    /// `drift` / `unnamed`, empty where the rows reaching over there are
    /// not held back.
    pub held_back: String,
    /// The remotes carrying the name, as a sentence lists them.
    pub carriers: String,
    pub row_goes: bool,
}

impl Record for TagMenu {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        Fields::new()
            .put("pushRemote", &self.push_remote)
            .put("lease", &self.lease)
            .put("reach", &self.reach)
            .put("heldBack", &self.held_back)
            .put("carriers", &self.carriers)
            .put("rowGoes", &self.row_goes)
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            push_remote: field(map, "pushRemote")?,
            lease: field(map, "lease")?,
            reach: field(map, "reach")?,
            held_back: field(map, "heldBack")?,
            carriers: field(map, "carriers")?,
            row_goes: field(map, "rowGoes")?,
        })
    }
}

impl From<platitude_core::session::TagMenuFacts> for TagMenu {
    fn from(facts: platitude_core::session::TagMenuFacts) -> Self {
        use platitude_core::session::TagDeleteHeld;
        Self {
            push_remote: facts.push_remote,
            lease: facts.lease.map(|oid| oid.to_hex()).unwrap_or_default(),
            reach: facts.reach,
            held_back: match facts.held_back {
                TagDeleteHeld::No => "",
                TagDeleteHeld::Drifted => "drift",
                TagDeleteHeld::Unnamed => "unnamed",
            }
            .to_string(),
            carriers: facts.carriers.join(", "),
            row_goes: facts.row_goes,
        }
    }
}

mod attach;
mod card;
mod drain;
#[cfg(test)]
mod drain_tests;
mod field;
#[cfg(test)]
mod field_tests;
mod item;
mod qmodel;
mod qobject;
mod role;
mod source;
#[cfg(test)]
mod testkit;
mod tree;
mod view;
#[cfg(test)]
mod view_tests;
mod walk;
#[cfg(test)]
mod walk_tests;
mod worktree_card;

use card::CardRows;
use item::{FOLDED, HELD, NavItem, PRUNABLE, fold_state};
/// The worktree rows' state words, which `GitFacts` hands to core's card rule.
pub(crate) use item::{LOCKED, MAIN};
use role::{Arranged, Role, Row, Value};
use source::{Bucket, Entry, Source, letters_of, pr_key};
use worktree_card::{NewWorktree, WorktreeRow, WorktreeRows};

#[derive(Default)]
pub struct NavSectionModel {
    section: String,
    /// File sections only: the one bucket run this list shows —
    /// `conflicts` / `unstaged` / `staged`. It narrows what is **shown**
    /// and nothing else: every run's list holds the whole status, so the
    /// answers a page asks about a file (`told` / `holds` / `beside`) are
    /// the whole tree's, whichever list is asked.
    run: String,
    all: Source,
    /// The rows as shown — indented, folded, filtered — or `None` when
    /// they are the source's rows in its own order (a section with no
    /// tree and no filter, e.g. tags, skips an index per row).
    arranged: Option<Vec<Arranged>>,
    /// Whether a tree placed these rows — the one thing that gives a
    /// **ref** row a full name (every other kind of row arrives with
    /// one). A filtered list stands in no tree, so its refs have none.
    tree_named: bool,
    filter: String,
    /// Rows this list is already showing as gone, while git is still
    /// being asked to delete them (デザイン規約 §消す操作は先に画面から消す).
    /// Keyed by what git is asked about — a stash by its selector, a ref
    /// by its short name (`Source::is_named`).
    ///
    /// Shown as though the delete had landed, `total` included (a
    /// filtered-out row is still counted). When a name comes back out is
    /// decided by `ops::StandIn`; the page only hands over what to draw
    /// without.
    hidden: Vec<String>,
    total: i32,
    /// Rows on screen — what filtering, folding and the run leave shown.
    /// A property because the pane's share of room is bound to it.
    shown_total: i32,
    /// File sections only: files in this list's bucket run — the
    /// number its heading wears, counted by the rule the rows follow
    /// (`Bucket::run`: untracked as unstaged).
    run_files: i32,
    /// Current branch (branches section only), as the one record has it
    /// (`RefsMsg::Head`) and not the snapshot: a status read that landed
    /// since the refs read may already have moved HEAD. Finds the
    /// highlighted row (`Role::IsHead`) and names the sticky stand-in.
    head_name: String,
    head_oid: String,
    /// What that branch's own row wears, read off the snapshot by name
    /// once both are in hand (`drain::settle_head_marks`).
    head_has_remote: bool,
    head_has_pr: bool,
    /// The upstream that branch is measured against and cannot reach, off
    /// the same lookup — a name, not a flag, as the rows carry it
    /// (`models::nav::field` の `Role::Bucket`).
    head_upstream_gone: String,
    /// How far that branch stands from its upstream, off the same lookup,
    /// so the stand-in and its row cannot say different numbers. Drawn
    /// while the status read's pair (`WorktreeModel.ahead`) is held back
    /// after a switch, not yet being about the branch HEAD is on.
    head_ahead: i32,
    head_behind: i32,
    /// Visible row of the current entry, or -1 when it has none (a
    /// filter or a collapsed folder hides it, or HEAD is detached).
    head_row: i32,
    /// What the stand-in's own line says: the whole name, except under a
    /// fold, where it begins at the folder that closed (the folders above
    /// are rows on screen — `view::arrange`). Display only; it still opens
    /// with and answers to the whole name.
    head_shown: String,
    /// The folded row the current entry is behind, when a fold took its
    /// row away; -1 otherwise, a filter included. The stand-in sits under
    /// this row (`HeadPinRow.seatedUnder`), where opening the folder
    /// brings the branch back; a filter leaves it at the head of the list.
    head_under_row: i32,
    /// The indent the sticky row steps in by, so the stand-in's name lines
    /// up with its row — or with the folder holding it where a fold took
    /// that row. 0 where a filter took it (seated at the head of the list,
    /// `HeadPinRow.seated`).
    head_depth: i32,
    /// True once a refs snapshot arrived (distinguishes "no head yet"
    /// from "detached / no local branches" for the default selection).
    refs_loaded: bool,
    /// When the last listing this section applied looked at the
    /// repository (`session::Standing::stamp`) — measured against a
    /// write's `reads_from`, which tells a listing that saw the write from
    /// one in flight as it ended. A session's own count: the tab stood in
    /// another worktree counts from the start again, and this holds the old
    /// session's number until the new one's first listing lands.
    looked: u64,
    /// File sections only: tree vs flat-path display. The page's
    /// restore (`PageLayout.applySavedLayout` → `set_tree_view`) is the
    /// sole writer — rules-refs/app-ui.md「保存フラグの復元は 1 書き手」;
    /// the derive-`Default` false is never on screen.
    tree_view: bool,
    /// The row the pointer is on and its notice, in the pieces its
    /// sentence needs — kept once here, since one row wears it at a time.
    /// Flat because `qproperty!` names one member.
    pointed_eol_path: String,
    pointed_eol_kind: String,
    pointed_eol_from: String,
    pointed_eol_to: String,
    pointed_eol_lines: i32,
    pointed_eol_scope: String,
    pointed_eol_ext: String,
    /// The marks as they arrived, so pointing at a row can find its words
    /// without the rows having carried them.
    eol_marks: Arc<Vec<platitude_core::session::EolMark>>,
    /// Explicit folder open/close choices (key = folder path); anything
    /// absent uses the section default.
    folder_overrides: HashMap<String, bool>,
    /// The snapshot this section last built its rows from. A poll tick
    /// that found nothing moved republishes the same `Arc`, so pointer
    /// equality is the whole check.
    last_refs: Option<Arc<platitude_core::session::RefsSnapshot>>,
    refs_feed: Option<Arc<Feed<RefsMsg>>>,
    status_feed: Option<Arc<Feed<StatusMsg>>>,
    carried_feed: Option<Arc<Feed<CarriedStatusMsg>>>,
    /// Which worktree the rows came from, empty for this window's own tree.
    /// Read back by the page as proof its switch of worktree has landed, so a
    /// pane refusing writes refuses them over the files it actually shows.
    carried_at: String,
    /// That worktree's name, as the band says it.
    carried_name: String,
    stash_feed: Option<Arc<Feed<crate::hub::StashList>>>,
    worktrees_feed: Option<Arc<Feed<crate::hub::WorktreeList>>>,
    tab_id: i32,
}
