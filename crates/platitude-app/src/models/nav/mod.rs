//! One section list (branches / remotes / files / worktrees /
//! stashes / tags). All render in the sidebar except `files` (the
//! changed files), which the right pane's WIP view shows.

use std::collections::HashMap;
use std::sync::Arc;

use qtbridge::qtbridge_type_lib::{
    QByteArray, QHash_i32_QByteArray, QModelIndex, QString, QVariant,
};
use qtbridge::{QAbstractItemModel, QAbstractItemModelBase, QModelItem, qobject};

use crate::encode::{Fields, Landed, Landing, Listed, One, Optional, Record, field};
use crate::hub::{CarriedStatusMsg, Feed, Hub, RefsMsg, StatusMsg, attached};

use super::pathtree::DirNode;
use super::{edit_between, push_run, qml_register};

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

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Option<Self> {
        Some(Self {
            remote: field(map, "remote")?,
            apart: field(map, "apart")?,
        })
    }
}

/// Why git would not take a name beside the branches or tags a section
/// holds (`NavSectionModel::name_clash`): `kind` is `taken`, `holds`,
/// `under` or `case`, and `with` the name in the way (empty for `taken`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameClashed {
    pub kind: String,
    pub with: String,
}

impl Record for NameClashed {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        Fields::new()
            .put("kind", &self.kind)
            .put("with", &self.with)
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Option<Self> {
        Some(Self {
            kind: field(map, "kind")?,
            with: field(map, "with")?,
        })
    }
}

impl From<platitude_core::session::NameClash> for NameClashed {
    fn from(clash: platitude_core::session::NameClash) -> Self {
        use platitude_core::session::NameClash;
        let (kind, with) = match clash {
            NameClash::Taken => ("taken", String::new()),
            NameClash::Holds(with) => ("holds", with),
            NameClash::Under(with) => ("under", with),
            NameClash::Case(with) => ("case", with),
        };
        Self {
            kind: kind.to_string(),
            with,
        }
    }
}

/// The worktree a stopped rebase or bisect holds a branch in
/// (`NavSectionModel::held_by_operation`): `by` is `rebase` or `bisect`,
/// `path` the worktree as git lists it, `here` whether it is the one this
/// window stands on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeldMark {
    pub by: String,
    pub path: String,
    pub here: bool,
}

impl Record for HeldMark {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        Fields::new()
            .put("by", &self.by)
            .put("path", &self.path)
            .put("here", &self.here)
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Option<Self> {
        Some(Self {
            by: field(map, "by")?,
            path: field(map, "path")?,
            here: field(map, "here")?,
        })
    }
}

/// What a tag's menu stands on (`NavSectionModel::tag_menu`,
/// `platitude_core::session::TagMenuFacts`), in the words the card reads,
/// with what its rows read over there flat beside them (`encode::put_far`:
/// `sides`, the `pushRemote` its pushes go to and the `heldBack` the card
/// words its greyed rows by) — the record QML hands `GitFacts.refMenu` as
/// it is (`CardRecord`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagMenu {
    /// Hex; empty for the plain push.
    pub lease: String,
    pub reach: String,
    /// The remotes carrying the name, as a sentence lists them.
    pub carriers: String,
    pub row_goes: bool,
    pub far: platitude_core::offers::TagFar,
}

impl Record for TagMenu {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        let far = platitude_core::offers::FarSide::Tag(self.far.clone());
        crate::encode::put_far(Fields::new(), &far)
            .put("lease", &self.lease)
            .put("reach", &self.reach)
            .put("carriers", &self.carriers)
            .put("rowGoes", &self.row_goes)
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Option<Self> {
        let platitude_core::offers::FarSide::Tag(far) = crate::encode::read_far(map)? else {
            return None;
        };
        Some(Self {
            lease: field(map, "lease")?,
            reach: field(map, "reach")?,
            carriers: field(map, "carriers")?,
            row_goes: field(map, "rowGoes")?,
            far,
        })
    }
}

impl From<platitude_core::session::TagMenuFacts> for TagMenu {
    fn from(facts: platitude_core::session::TagMenuFacts) -> Self {
        Self {
            far: facts.far,
            lease: crate::encode::oid_text(facts.lease),
            reach: facts.reach,
            carriers: facts.carriers.join(", "),
            row_goes: facts.row_goes,
        }
    }
}

/// What a branch's card reads of its upstream, read as the menu opens and
/// as it stands again (`NavSectionModel::branch_upstream`): the remote
/// reading it speaks for and that reading's commit — what the card's
/// remote deletes name and are leased to —, the upstream it is configured
/// with, where that stands and the commit it is on, and core's part flat
/// beside them (`encode::put_far`), off the same row
/// (`RefsSnapshot::branch_upstream`). QML hands the record to
/// `GitFacts.refMenu` as it is (`CardRecord`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BranchUpstream {
    /// `None` on a row that is no local branch's: it reads nothing over
    /// there.
    pub far: Option<platitude_core::offers::BranchFar>,
    /// Empty where no reading is listed.
    pub counterpart: String,
    /// Hex; empty with no reading.
    pub counterpart_oid: String,
    /// Listed or gone, a branch here too (`BranchItem::upstream_named`);
    /// empty where it has none.
    pub upstream: String,
    /// That upstream is a branch here (`BranchItem::upstream_here`).
    pub upstream_here: bool,
    /// Hex; the commit that upstream stands on — what the plain delete is
    /// measured against (`BranchItem::upstream_oid`). Empty where there is
    /// none, and git then measures against HEAD.
    pub upstream_oid: String,
}

impl Record for BranchUpstream {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        use platitude_core::offers::FarSide;
        let far = self.far.map_or(FarSide::Unread, FarSide::Branch);
        crate::encode::put_far(Fields::new(), &far)
            .put("remoteCounterpart", &self.counterpart)
            .put("remoteCounterpartOid", &self.counterpart_oid)
            .put("upstream", &self.upstream)
            .put("upstreamHere", &self.upstream_here)
            .put("upstreamOid", &self.upstream_oid)
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Option<Self> {
        use platitude_core::offers::FarSide;
        let far = match crate::encode::read_far(map)? {
            FarSide::Unread => None,
            FarSide::Branch(far) => Some(far),
            FarSide::Tag(_) => return None,
        };
        Some(Self {
            far,
            counterpart: field(map, "remoteCounterpart")?,
            counterpart_oid: field(map, "remoteCounterpartOid")?,
            upstream: field(map, "upstream")?,
            upstream_here: field(map, "upstreamHere")?,
            upstream_oid: field(map, "upstreamOid")?,
        })
    }
}

impl From<platitude_core::session::BranchUpstreamFacts> for BranchUpstream {
    fn from(facts: platitude_core::session::BranchUpstreamFacts) -> Self {
        Self {
            far: Some(facts.far),
            counterpart: facts.counterpart,
            counterpart_oid: crate::encode::oid_text(facts.counterpart_oid),
            upstream: facts.upstream,
            upstream_here: facts.upstream_here,
            upstream_oid: crate::encode::oid_text(facts.upstream_oid),
        }
    }
}

mod attach;
mod card;
mod drain;
mod field;
mod item;
mod menu;
mod qmodel;
mod qobject;
mod role;
mod source;
#[cfg(test)]
mod testkit;
#[cfg(test)]
mod tests;
mod tree;
mod view;
mod walk;
mod worktree_card;

use card::CardRows;
use item::{FOLDED, HELD, NavItem, fold_state};
/// The worktree rows' state words, which `GitFacts` hands to core's card rule.
pub(crate) use item::{LOCKED, MAIN, PRUNABLE};
pub(crate) use menu::{CardRecord, RowLead};
use role::{Arranged, Role, Row, Value};
use source::{Bucket, Entry, Source, letters_of, pr_key};
pub(crate) use worktree_card::{NewWorktree, WorktreeName, WorktreeRow, WorktreeRows};

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
    /// One key per row on screen, in screen order (`view::row_key`): what
    /// the next reshape weighs its rows against, so that a read moving a
    /// few rows tells Qt of those alone and every other row keeps its
    /// delegate (`view::tell_rows`).
    shown_keys: Vec<u64>,
    /// The rows as Qt holds them while a reshape is being told — each one
    /// a row of the new arrangement, or `None` for one on its way out — so
    /// that `rowCount` and `data` answer every step of the telling. `None`
    /// outside it.
    transit: Option<Vec<Option<u32>>>,
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
    /// Branches section only: the branch whose worktree is being removed,
    /// drawn as nobody's while git is still asked (`HELD` left off — the
    /// worktree's own row is the worktrees section's `hidden`). Empty for
    /// none; when it comes back is `ops::StandIn`'s, as for `hidden`.
    released: String,
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
    /// The commit HEAD is on (`HeadMsg::oid_hex`), detached too; empty
    /// before the first commit. QML reads where HEAD is from
    /// `WorktreeModel.headOid`.
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
    /// one in flight as it ended. A file list's is its status's. A session's
    /// own count: the tab stood in another worktree counts from the start
    /// again, and this holds the old session's number until the new one's
    /// first listing lands.
    looked: u64,
    /// File sections of this window's tree: the status as git last reported
    /// it, while the rows on screen are drawn with stage presses still out
    /// over it (`ops::StageAhead`) — kept to draw again when those move.
    /// `None` while the rows are that status itself, which `all` then holds:
    /// a second copy would cost a whole status.
    staged_from: Option<StatusBase>,
    /// File sections only: tree vs flat-path display. The page's
    /// restore (`PageLayout.applySavedLayout` → `set_tree_view`) is the
    /// sole writer: an `attach_*` that set it would overwrite the restore,
    /// and that value would be written back into the saved layout. The
    /// derive-`Default` false is never on screen.
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
    /// Moves with every snapshot this section takes (`refsMoved`), rows
    /// moved or not — what a binding that asks a slot about the snapshot
    /// follows: a slot is followed by nothing, and `changed` is said only
    /// for rows that moved, while what a slot reads can move with none of
    /// them — the remotes' names (`PublishFlow.pushFacts`).
    refs_revision: i32,
    refs_feed: Option<Arc<Feed<RefsMsg>>>,
    status_feed: Option<Arc<Feed<StatusMsg>>>,
    carried_feed: Option<Arc<Feed<CarriedStatusMsg>>>,
    /// Which worktree the rows came from, empty for this window's own tree.
    /// Read back by the page as proof its switch of worktree has landed, so a
    /// pane refusing writes refuses them over the files it actually shows.
    carried_at: String,
    /// That worktree's name, as the band says it.
    carried_name: String,
    /// The branch the repository's own worktree keeps, where that worktree is
    /// not this window's — checked out, or kept by a rebase or bisect
    /// standing there (`WorktreeEntry::held`). A branch it keeps wears the
    /// house where the others wear the tree (デザイン規約 §ref の種別).
    /// Empty otherwise, and on every section but WORKTREES.
    home_branch: String,
    stash_feed: Option<Arc<Feed<crate::hub::StashList>>>,
    worktrees_feed: Option<Arc<Feed<crate::hub::WorktreeList>>>,
    tab_id: i32,
}

/// A status as git reported it, with the line-ending marks that came with it
/// (`NavSectionModel::staged_from`).
struct StatusBase {
    status: Arc<platitude_core::status::WorktreeStatus>,
    marks: Arc<Vec<platitude_core::session::EolMark>>,
}
