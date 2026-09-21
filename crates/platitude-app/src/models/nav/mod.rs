//! One section list (branches / remotes / worktree / worktrees /
//! stashes / tags). All render in the sidebar except `worktree` (the
//! changed files), which the right pane's WIP view shows.

use std::collections::HashMap;
use std::sync::Arc;

use qtbridge::qtbridge_type_lib::{QByteArray, QHash, QModelIndex, QVariant};
use qtbridge::{QAbstractItemModel, QAbstractItemModelBase, QModelItem, QObjectHolder, qobject};

use crate::encode::{Fields, Landed, Landing, Listed, Record, field};
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

mod attach;
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

//
// The folded marker is written through `fold_state` and read back where
// a fold has to be told from an open folder (`view::folded_over_head`).
use item::{FOLDED, HELD, LOCKED, MAIN, NavItem, PRUNABLE, fold_state};
use role::{Arranged, Role, Row, Value};
use source::{Bucket, Entry, Source, letters_of, pr_key};

#[derive(Default)]
pub struct NavSectionModel {
    section: String,
    /// Worktree sections only: the one bucket run this list shows —
    /// `conflicts` / `unstaged` / `staged`. Each run is a list of its own
    /// in the WIP pane, and every one of them holds the whole status, so
    /// this narrows what is **shown** and nothing else: the answers a page
    /// asks about a file (`told` / `holds` / `beside`) come out of the
    /// source and are the whole tree's, whichever list is asked.
    run: String,
    all: Source,
    /// The rows as shown — indented, folded, filtered — or `None` when
    /// they are the source's rows in its own order.
    ///
    /// `None` is an optimisation of an identical list: a section with no
    /// tree and no filter (tags, stashes) shows the source exactly, and
    /// an index per row would say only that the rows are where they
    /// already are.
    arranged: Option<Vec<Arranged>>,
    /// Whether a tree placed these rows — the one thing that gives a
    /// **ref** row a full name (a branch arrives knowing only what it is
    /// called; every other kind of row arrived with both). A filtered
    /// list stands in no tree, so its refs have no full name.
    tree_named: bool,
    filter: String,
    /// Rows this list is already showing as gone, while git is still
    /// being asked to delete them (デザイン規約 §消す操作は先に画面から消す).
    /// Keyed the way `Row::Full`-carrying rows are named to git — a
    /// stash by its selector, a ref by its short name.
    ///
    /// One of these is shown as though the delete had already
    /// landed, `total` included, where a filtered-out row is still
    /// one of the section's rows and is still counted. A name goes
    /// in when the write goes out and comes back out when that
    /// write is refused or the reading it invalidated has arrived —
    /// which of the two, and for which write, is decided away from
    /// here (`ops::StandIn`); the page only hands over what it was
    /// told to draw without.
    hidden: Vec<String>,
    total: i32,
    /// Rows on screen — what filtering, folding and the run leave shown.
    /// A property, because the pane's share of room is worked out from
    /// it, and a binding follows properties
    /// (app-ui.md 「QML バインディングはプロパティにしか反応しない」).
    shown_total: i32,
    /// Worktree sections only: files in this list's bucket run — the
    /// number its heading wears. Untracked files count as unstaged
    /// because the run does (`Bucket::run`), which is what keeps the
    /// heading's number and the rows under it one rule.
    run_files: i32,
    /// Current branch (branches section only), as the one record has it
    /// (`RefsMsg::Head`, the report every consumer at HEAD is handed —
    /// `hub::sink`). What the highlighted row is found by
    /// (`Role::IsHead`), and what the sticky row that stands in for it
    /// while its own row is scrolled off says. **The record has it**:
    /// the snapshot is one refs read's picture, and a status read that
    /// landed since may already have moved HEAD.
    head_name: String,
    head_oid: String,
    /// What that branch's own row wears, read off the snapshot by name
    /// once both are in hand (`drain::settle_head_marks`).
    head_has_remote: bool,
    head_has_pr: bool,
    /// The upstream that branch is measured against and cannot reach,
    /// off the same lookup — the name, because the stand-in draws the
    /// badge's state from it and says it in the line it opens, the way
    /// the rows read one answer out of one slot (`field::Role::Bucket`).
    head_upstream_gone: String,
    /// How far that branch stands from its upstream, off the same lookup
    /// — the pair the stand-in draws, so it and the row it stands for
    /// cannot say different numbers. **Drawn while the status read's
    /// pair** (`WorkTreeModel.ahead`) is held back: the counts are not
    /// yet about the branch HEAD is on, which is the moment after a
    /// switch when this stand-in is the one on screen.
    head_ahead: i32,
    head_behind: i32,
    /// Visible row of the current entry, or -1 when it has none (a
    /// filter or a collapsed folder hides it, or HEAD is detached). The
    /// sticky row needs it to tell whether the real row is on screen.
    head_row: i32,
    /// What the stand-in's own line says, which is the whole name
    /// everywhere but under a fold: there it begins at the folder that
    /// closed, since the folders above that one are rows on screen
    /// (`view::arrange`). The whole name is still what it opens with and
    /// what it answers to — this is the cut line alone.
    head_shown: String,
    /// The folded row the current entry is behind, when a fold is what
    /// took its row away; -1 in every other case, the filter's included.
    /// **The stand-in sits under this row** (`HeadPinRow.seatedUnder`),
    /// which is where opening that folder brings the branch back — a
    /// stand-in at the head of the list instead would say the branch is
    /// somewhere the reader can see it is not. A filter leaves nothing
    /// to sit under, so that half keeps the head of the list.
    head_under_row: i32,
    /// How far that row is folded in — what the sticky row steps itself
    /// in by, so the stand-in's name begins in the same column as the
    /// row it is reading against: its own where it has one, and the
    /// folder holding it where a fold took that row. 0 where a filter
    /// took it: the stand-in then takes a seat at the head of the list
    /// (`HeadPinRow.seated`), where nothing is nesting it.
    head_depth: i32,
    /// True once a refs snapshot arrived (distinguishes "no head yet"
    /// from "detached / no local branches" for the default selection).
    refs_loaded: bool,
    /// Worktree section only: tree vs flat-path display. The page's
    /// restore is the sole writer and `attach_*` a reader
    /// (`PageLayout.applySavedLayout` → `set_tree_view`, before anything
    /// shows — the saved default is the tree; the derive-`Default` false
    /// here is never on screen). An attach after the restore once put
    /// the saved flat view back to a tree and then wrote the tree over
    /// the saved flag.
    tree_view: bool,
    /// Which row the pointer is on; the notice follows, taken apart into
    /// the pieces its sentence needs. Kept once here — one row wears it
    /// at a time (see `NavItem::eol_mark`). Flat because `qproperty!`
    /// names one member.
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
    /// that found nothing moved republishes the very same one, so this
    /// pointer is the whole check — the rows are not rebuilt to discover
    /// they are identical.
    last_refs: Option<Arc<platitude_core::session::RefsSnapshot>>,
    refs_feed: Option<Arc<Feed<RefsMsg>>>,
    status_feed: Option<Arc<Feed<StatusMsg>>>,
    carried_feed: Option<Arc<Feed<CarriedStatusMsg>>>,
    /// Which copy the rows came from, empty for this window's own tree.
    /// **The page's own answer, read back**: a list showing one copy is
    /// handed another copy's rows only by being told to, and this is what
    /// says the telling has landed — so a pane about to refuse every write
    /// is refusing them over the files it is actually showing.
    carried_at: String,
    /// That copy's name, as the band says it.
    carried_name: String,
    stash_feed: Option<Arc<Feed<crate::hub::StashList>>>,
    worktrees_feed: Option<Arc<Feed<Vec<platitude_core::worktrees::WorktreeEntry>>>>,
    tab_id: i32,
}
