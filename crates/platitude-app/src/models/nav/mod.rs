//! One section list (branches / remotes / worktree / worktrees /
//! stashes / tags). All render in the sidebar except `worktree` (the
//! changed files), which the right pane's WIP view shows.

use std::collections::HashMap;
use std::sync::Arc;

use qtbridge::qtbridge_type_lib::{QByteArray, QHash, QModelIndex, QVariant};
use qtbridge::{QAbstractItemModel, QAbstractItemModelBase, QModelItem, QObjectHolder, qobject};

use crate::hub::{Feed, Hub, StatusMsg, attached};

use super::pathtree::DirNode;
use super::qml_register;

mod attach;
mod drain;
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

// The siblings reach these through `use super::*`, which sees what this
// module can see -- so the bindings stay private and nothing leaks out of
// `nav` but the model itself (`models::mod`).
//
// Only the tests read the folded marker itself; production writes it
// through `fold_state`.
#[cfg(test)]
use item::FOLDED;
use item::{HELD, LOCKED, NavItem, PRUNABLE, fold_state};
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
    /// `None` is not an optimisation of an empty list but of an identical
    /// one: a section with no tree and no filter (tags, stashes) shows the
    /// source exactly, and an index per row would say only that the rows
    /// are where they already are.
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
    /// Not the same thing as `filter`: a filtered-out row is still one of
    /// the section's rows and is still counted, where one of these is
    /// being shown as though the delete had already landed, `total`
    /// included. The page puts a name in when the write goes out and
    /// takes it back out when the write is refused or the refs it moved
    /// have arrived (`RepoPage.showGone` / `RepoPage.showBack`).
    hidden: Vec<String>,
    total: i32,
    /// Rows on screen — what filtering, folding and the run leave shown.
    /// A property rather than the slot beside it because the pane's share
    /// of room is worked out from it, and a binding follows properties
    /// (app-ui.md 「QML バインディングはプロパティにしか反応しない」).
    shown_total: i32,
    /// Worktree sections only: files in this list's bucket run — the
    /// number its heading wears. Untracked files count as unstaged
    /// because the run does (`Bucket::run`), which is what keeps the
    /// heading's number and the rows under it one rule.
    run_files: i32,
    /// Current branch (branches section only) — feeds the sticky row
    /// that stands in for it while its own row is scrolled off.
    head_name: String,
    head_oid: String,
    head_has_remote: bool,
    head_has_pr: bool,
    /// Visible row of the current entry, or -1 when it has none (a
    /// filter or a collapsed folder hides it, or HEAD is detached). The
    /// sticky row needs it to tell whether the real row is on screen.
    head_row: i32,
    /// True once a refs snapshot arrived (distinguishes "no head yet"
    /// from "detached / no local branches" for the default selection).
    refs_loaded: bool,
    /// Worktree section only: tree vs flat-path display. The page's
    /// restore is the only writer (`PageLayout.applySavedLayout` →
    /// `set_tree_view`, before anything shows — the saved default is the
    /// tree; the derive-`Default` false here is never on screen):
    /// `attach_*` must not touch it — an attach after the restore once
    /// put the saved flat view back to a tree and then wrote the tree
    /// over the saved flag.
    tree_view: bool,
    /// Which row the pointer is on; the notice follows, taken apart into
    /// the pieces its sentence needs. Kept once here rather than on every
    /// row — a `QModelItem` has no fields left (see `NavItem::eol_mark`).
    /// Flat because `qproperty!` names one member, not a path through one.
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
    refs_feed: Option<Arc<Feed<Arc<platitude_core::session::RefsSnapshot>>>>,
    status_feed: Option<Arc<Feed<StatusMsg>>>,
    stash_feed: Option<Arc<Feed<Vec<platitude_core::stash::StashEntry>>>>,
    worktrees_feed: Option<Arc<Feed<Vec<platitude_core::worktrees::WorktreeEntry>>>>,
    tab_id: i32,
}
