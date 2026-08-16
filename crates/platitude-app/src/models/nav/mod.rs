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

mod field;
mod item;
mod qobject;
mod role;
mod source;
#[cfg(test)]
mod testkit;
mod tree;
mod view;

// The siblings reach these through `use super::*`, which sees what this
// module can see -- so the bindings stay private and nothing leaks out of
// `nav` but the model itself (`models::mod`).
//
// Only the tests read the folded marker itself; production writes it
// through `fold_state`.
#[cfg(test)]
use item::FOLDED;
use item::{NavItem, fold_state};
use role::{Arranged, Role, Row, Value};
use source::{Bucket, Entry, Source, letters_of, pr_key};

#[derive(Default)]
pub struct NavSectionModel {
    section: String,
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
    total: i32,
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
    /// Worktree section only: tree (default) vs flat-path display.
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
