use std::collections::HashMap;
use std::sync::Arc;

use qtbridge::qtbridge_type_lib::{QByteArray, QHash, QModelIndex, QVariant};
use qtbridge::{QAbstractItemModel, QAbstractItemModelBase, QModelItem, QObjectHolder, qobject};

use crate::hub::{Feed, Hub, StatusMsg};

use super::qml_register;

// ---------------------------------------------------------------------------
// NavSectionModel: one section list (branches / remotes / worktree /
// worktrees / stashes / tags). Six QML instances share this type; each
// attaches to its section's feed and owns its inner scrolling list, so
// section headers can stay fixed while contents scroll. All render in the
// sidebar except `worktree` (the changed files), which the right pane's
// WIP view shows.
// ---------------------------------------------------------------------------

// PartialEq feeds the unchanged-drain check: refs and status are
// republished on every poll tick whether or not they moved, and rebuilding
// the section (plus the Qt model reset behind it) for identical rows is
// work the sidebar can see — a repository with tens of thousands of tags
// pays it in the view, on the main thread.
#[derive(QModelItem, Default, Clone, PartialEq)]
pub struct NavItem {
    /// Display text: the last path segment in tree mode, the full name in
    /// flat/filter mode.
    name: String,
    /// Full ref/path (tooltips; folder rows carry their folder path here,
    /// which doubles as the toggle key).
    full: String,
    oid_hex: String,
    /// git's change code for a file row (`M`, `?`, `UU`). **A folder row
    /// has no change to report and carries its fold state here instead**
    /// (`FOLDED`, empty when open) — the item is a union of five kinds of
    /// row and qtbridge's `QModelItem` allows fifteen fields, so a slot
    /// that structurally cannot be used twice at once is shared. Reading it
    /// is guarded by `folder` everywhere, as the other shared fields are
    /// (`bucket` carries a branch on a worktree row, `full` a folder key).
    change: String,
    bucket: String,
    /// Display grouping of worktree rows (GitKraken-style): untracked
    /// files count as `unstaged` here while `bucket` keeps the real
    /// routing for diffs and staging.
    group: String,
    /// The old path of a renamed working-tree file. **A folder row in
    /// the working tree's list carries its clean path here** — its
    /// `full` is the group-prefixed fold key, and the hover of an
    /// elided chain needs the path itself (a folder never uses the
    /// rename slot, the way `change` carries the fold state).
    orig_path: String,
    is_head: bool,
    has_remote: bool,
    /// This repository does not hold the ref, so the name greys. Only tags
    /// are ever listed that way: a tag a remote has and this one does not
    /// reaches no graph row, and the sidebar is where it can be read at
    /// all. Written as the negative of core's `here` so every other kind
    /// of row keeps it off by default.
    only_remote: bool,
    /// The remote branch a local one speaks for (`origin/main`), empty for
    /// every other kind of row. What a rename of this row offers to carry
    /// over, and what the badge beside it is about.
    upstream: String,
    /// PR-state badge. Real data arrives in Phase 4 (ls-remote refs/pull
    /// matching); until then PG_FAKE_PR previews the look.
    has_pr: bool,
    /// This file's pending change has something to say about its line
    /// endings. A flag, not the sentence — a `QModelItem` holds fifteen
    /// fields and this one is the fifteenth, so the words for the row the
    /// pointer is on are kept once on the model instead (`pointEol`).
    eol_mark: bool,
    depth: i32,
    folder: bool,
}

impl platitude_core::mem::Footprint for NavItem {
    fn heap_bytes(&self) -> usize {
        self.name.heap_bytes()
            + self.full.heap_bytes()
            + self.oid_hex.heap_bytes()
            + self.change.heap_bytes()
            + self.bucket.heap_bytes()
            + self.group.heap_bytes()
            + self.orig_path.heap_bytes()
            + self.upstream.heap_bytes()
    }
}

/// What a folder row puts in `change` while it is closed.
pub const FOLDED: &str = "FOLDED";

fn fold_state(expanded: bool) -> String {
    if expanded {
        String::new()
    } else {
        FOLDED.to_string()
    }
}

/// Where a section's rows come from.
///
/// A row that is only ever drawn does not have to be built. The two
/// sections a large repository fills — its tags and its remote branches —
/// read out of the refs snapshot the session already holds and this model
/// already points at, so their whole lists cost one `Arc` and, where the
/// shaping has anything to say, an index per visible row. Measured on
/// `JetBrains/kotlin`: holding them as rows was 13.9MB of the process's
/// Rust heap, all of it a second copy of what the snapshot says.
enum Source {
    /// Rows as they arrived, for the sections whose lists are short.
    Kept(Vec<NavItem>),
    /// `snapshot.tags`.
    Tags(Arc<platitude_core::session::RefsSnapshot>),
    /// `snapshot.remotes`.
    Remotes(Arc<platitude_core::session::RefsSnapshot>),
}

impl Default for Source {
    fn default() -> Self {
        Self::Kept(Vec::new())
    }
}

impl platitude_core::mem::Footprint for Source {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::Kept(items) => items.heap_bytes(),
            // Nothing of its own: the snapshot belongs to the session,
            // which is where the report counts it. A shared `Arc` added up
            // at every pointer into it is a number that means nothing.
            Self::Tags(_) | Self::Remotes(_) => 0,
        }
    }
}

impl Source {
    fn len(&self) -> usize {
        match self {
            Self::Kept(items) => items.len(),
            Self::Tags(snapshot) => snapshot.tags.len(),
            Self::Remotes(snapshot) => snapshot.remotes.len(),
        }
    }

    fn entry(&self, at: usize) -> Option<Entry<'_>> {
        match self {
            Self::Kept(items) => items.get(at).map(Entry::Item),
            Self::Tags(snapshot) => snapshot.tags.get(at).map(Entry::Tag),
            Self::Remotes(snapshot) => snapshot.remotes.get(at).map(Entry::Remote),
        }
    }

    /// The rows as they arrived, for the shaping that still copies them.
    /// Empty for a projected section, which has none to lend.
    fn kept(&self) -> &[NavItem] {
        match self {
            Self::Kept(items) => items,
            Self::Tags(_) | Self::Remotes(_) => &[],
        }
    }
}

/// One entry of a source, whichever kind the section has.
#[derive(Clone, Copy)]
enum Entry<'a> {
    Item(&'a NavItem),
    Tag(&'a platitude_core::session::TagItem),
    Remote(&'a platitude_core::session::BranchItem),
}

impl<'a> Entry<'a> {
    /// What git calls it, before any indenting takes it apart.
    fn name(self) -> &'a str {
        match self {
            Self::Item(item) => &item.name,
            Self::Tag(tag) => &tag.short,
            Self::Remote(branch) => &branch.short,
        }
    }

    /// The full name a row of this kind arrived with — which for a ref
    /// read out of the snapshot is nothing, the way `branch_nav_items`
    /// left it: only the tree writes one, and only into the rows it
    /// shapes.
    fn full(self) -> &'a str {
        match self {
            Self::Item(item) => &item.full,
            Self::Tag(_) | Self::Remote(_) => "",
        }
    }
}

/// One row of the shaped list.
///
/// Sixteen bytes where a row of the source can be pointed at, against the
/// two hundred a `NavItem` costs; a folder row is the one thing no source
/// holds, so it is the one thing carried whole (behind a box, so the
/// pointed-at rows are not all widened to hold one).
enum Arranged {
    At { at: u32, depth: i32 },
    Made(Box<NavItem>),
}

impl platitude_core::mem::Footprint for Arranged {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::At { .. } => 0,
            Self::Made(item) => item.heap_bytes(),
        }
    }
}

/// One row as the view reads it.
#[derive(Clone, Copy)]
enum Row<'a> {
    /// A row this section built: the folder rows, and the shaped copies
    /// the sections that still hold their rows make.
    Made(&'a NavItem),
    /// A row of the source, shown at this depth.
    Shown { of: Entry<'a>, depth: i32 },
}

/// What a row answers for one role, before Qt is handed it.
///
/// A step between the row and `QVariant` so that Rust can read the same
/// answer the delegate is given: automation asks for a row's name, the
/// arranging asks which row is the current entry, and none of them should
/// be reading a second opinion.
enum Value<'a> {
    /// Text the row already holds.
    Said(&'a str),
    /// Text the row has to spell out.
    Spelled(String),
    Flag(bool),
    Number(i32),
}

impl Value<'_> {
    fn variant(&self) -> QVariant {
        match self {
            Self::Said(text) => QVariant::from(*text),
            Self::Spelled(text) => QVariant::from(text),
            Self::Flag(flag) => QVariant::from(flag),
            Self::Number(number) => QVariant::from(number),
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Self::Said(text) => text,
            Self::Spelled(text) => text,
            Self::Flag(_) | Self::Number(_) => "",
        }
    }

    fn flag(&self) -> bool {
        matches!(self, Self::Flag(true))
    }
}

/// The roles a delegate reads a row by.
///
/// The numbers are `NavItem`'s fields in order, which is what
/// `#[derive(QModelItem)]` hands the view as names. The test at the foot
/// of this file holds the two together: **a role answered under a name
/// the delegate does not ask for draws nothing at all**, and says nothing
/// about it — no warning, no error, an empty row.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Name,
    Full,
    OidHex,
    Change,
    Bucket,
    Group,
    OrigPath,
    IsHead,
    HasRemote,
    OnlyRemote,
    Upstream,
    HasPr,
    EolMark,
    Depth,
    Folder,
}

impl Role {
    /// Every role, in the order their numbers run — which is the order
    /// `NavItem` declares its fields in.
    const ALL: [Self; 15] = [
        Self::Name,
        Self::Full,
        Self::OidHex,
        Self::Change,
        Self::Bucket,
        Self::Group,
        Self::OrigPath,
        Self::IsHead,
        Self::HasRemote,
        Self::OnlyRemote,
        Self::Upstream,
        Self::HasPr,
        Self::EolMark,
        Self::Depth,
        Self::Folder,
    ];

    fn of(role: i32) -> Option<Self> {
        usize::try_from(role)
            .ok()
            .and_then(|at| Self::ALL.get(at))
            .copied()
    }

    /// The name the delegate asks for this role by.
    fn spelling(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Full => "full",
            Self::OidHex => "oid_hex",
            Self::Change => "change",
            Self::Bucket => "bucket",
            Self::Group => "group",
            Self::OrigPath => "orig_path",
            Self::IsHead => "is_head",
            Self::HasRemote => "has_remote",
            Self::OnlyRemote => "only_remote",
            Self::Upstream => "upstream",
            Self::HasPr => "has_pr",
            Self::EolMark => "eol_mark",
            Self::Depth => "depth",
            Self::Folder => "folder",
        }
    }
}

#[derive(Default)]
pub struct NavSectionModel {
    section: String,
    /// What the rows are read out of.
    all: Source,
    /// The rows as shown — indented, folded, filtered — or `None` when
    /// they are the source's rows in its own order.
    ///
    /// `None` is not an optimisation of an empty list but of an identical
    /// one: a section with no tree and no filter (tags, stashes) shows the
    /// source exactly, and an index per row would say only that the rows
    /// are where they already are.
    arranged: Option<Vec<Arranged>>,
    /// Whether `arranged` is the indented form, where a row shows the
    /// segment under its folder and its whole name is what git calls it.
    /// A filtered list is not: it shows whole names and stands in no tree.
    indented: bool,
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
    /// Which row the pointer is on, so the other marked rows do not all
    /// answer with the sentence belonging to this one.
    ///
    /// The notice itself follows, taken apart into the pieces its sentence
    /// needs. Kept once here rather than on every row: only one row is
    /// under the pointer, and a `QModelItem` has no fields left (see
    /// `NavItem::eol_mark`). Flat because `qproperty!` names one member,
    /// not a path through one.
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

// A list model that answers by value rather than by reference: `data()`
// computes the field a role asks for, where `QListModel::get` would have
// handed out a borrow of a stored `NavItem` and so forced every row to
// exist. Nothing here is a tree — `parent` is always invalid and rows hang
// off the root — but `QAbstractItemModel` is the base that lets a row be
// answered instead of held (and the one CXX-Qt expects, via
// `QAbstractListModel`).
impl QAbstractItemModel for NavSectionModel {
    fn index(&self, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex {
        let out_of_list = parent.is_valid()
            || column != 0
            || usize::try_from(row).is_ok_and(|row| row >= self.shown_rows());
        if out_of_list || row < 0 {
            return QModelIndex::default();
        }
        self.create_index(row, column, 0)
    }

    /// Every row is a child of the root.
    fn parent(&self, _child: &QModelIndex) -> QModelIndex {
        QModelIndex::default()
    }

    fn row_count(&self, parent: &QModelIndex) -> i32 {
        if parent.is_valid() {
            0
        } else {
            self.shown_rows() as i32
        }
    }

    fn column_count(&self, parent: &QModelIndex) -> i32 {
        if parent.is_valid() { 0 } else { 1 }
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let (Some(role), Some(row)) = (
            Role::of(role),
            usize::try_from(index.row())
                .ok()
                .and_then(|at| self.row_at(at)),
        ) else {
            return QVariant::default();
        };
        self.field(row, role).variant()
    }

    /// The names QML resolves a role by. Spelled where the answers are
    /// (`Role`), and held to the item's own derived table by the test at
    /// the foot of this file — a role the delegate cannot name draws
    /// nothing at all, and says nothing about it.
    fn role_names(&self) -> QHash<i32, QByteArray> {
        let mut names = QHash::default();
        for (number, role) in Role::ALL.iter().enumerate() {
            names.insert(&(number as i32), &QByteArray::from(role.spelling()));
        }
        names
    }
}

impl NavSectionModel {
    /// Shapes the arrived rows into the ones on screen — indented, folded,
    /// filtered — and finds the current entry among them.
    fn arrange(&mut self) {
        let needle = self.filter.to_lowercase();
        self.indented =
            needle.is_empty() && matches!(self.section.as_str(), "branches" | "remotes");
        self.arranged = if needle.is_empty() {
            match self.section.as_str() {
                "branches" | "remotes" => Some(self.build_tree()),
                // The worktree keeps its group runs (conflicts → unstaged →
                // staged) and trees each run independently.
                "worktree" if self.tree_view => {
                    let kept = self.all.kept();
                    let mut out = Vec::new();
                    let mut i = 0;
                    while i < kept.len() {
                        let group = kept[i].group.clone();
                        let mut j = i + 1;
                        while j < kept.len() && kept[j].group == group {
                            j += 1;
                        }
                        wt_tree_into(&kept[i..j], &group, &self.folder_overrides, &mut out);
                        i = j;
                    }
                    Some(out)
                }
                // Nothing to indent, fold or leave out: these rows are the
                // ones that arrived.
                _ => None,
            }
        } else {
            // Filtering shows flat full names (folders would hide context).
            Some(
                (0..self.all.len())
                    .filter(|at| {
                        self.all
                            .entry(*at)
                            .is_some_and(|of| of.name().to_lowercase().contains(&needle))
                    })
                    .map(|at| Arranged::At {
                        at: at as u32,
                        depth: 0,
                    })
                    .collect(),
            )
        };
        self.head_row = (0..self.shown_rows())
            .find(|at| {
                self.row_at(*at).is_some_and(|row| {
                    self.field(row, Role::IsHead).flag() && !self.field(row, Role::Folder).flag()
                })
            })
            .map_or(-1, |row| row as i32);
    }

    /// Shapes the rows again and tells the view its whole list changed.
    ///
    /// The begin/end pair is written out here because the shaping is ours
    /// now: `QListModel` had a `reset` that wrapped it, and answering by
    /// value means there is no such wrapper to inherit.
    fn reshape(&mut self) {
        self.begin_reset_model();
        self.arrange();
        self.end_reset_model();
    }

    /// How many rows are on screen: the shaped list where there is one,
    /// and the source itself where the shaping had nothing to say.
    fn shown_rows(&self) -> usize {
        match &self.arranged {
            Some(arranged) => arranged.len(),
            None => self.all.len(),
        }
    }

    /// The row at one position on screen.
    fn row_at(&self, at: usize) -> Option<Row<'_>> {
        match &self.arranged {
            Some(arranged) => match arranged.get(at)? {
                Arranged::At { at, depth } => Some(Row::Shown {
                    of: self.all.entry(*at as usize)?,
                    depth: *depth,
                }),
                Arranged::Made(item) => Some(Row::Made(item)),
            },
            None => Some(Row::Shown {
                of: self.all.entry(at)?,
                depth: 0,
            }),
        }
    }

    /// What the row is called on screen: the segment under its folder
    /// while the list is indented, and the whole name everywhere else.
    fn shown_name<'a>(&self, of: Entry<'a>, depth: i32) -> &'a str {
        let whole = of.name();
        if !self.indented {
            return whole;
        }
        let mut rest = whole;
        for _ in 0..depth {
            let Some((_, tail)) = rest.split_once('/') else {
                break;
            };
            rest = tail;
        }
        rest
    }

    /// What one row answers for one role.
    ///
    /// **The only place a row's fields are worked out.** The view reads
    /// it through `data`, and the slots below read it directly, so no row
    /// can show the delegate one thing and tell automation another.
    fn field<'a>(&self, row: Row<'a>, role: Role) -> Value<'a> {
        let (of, depth) = match row {
            // A row that was built holds what it shows already.
            Row::Made(item) => {
                return match role {
                    Role::Name => Value::Said(&item.name),
                    Role::Full => Value::Said(&item.full),
                    Role::OidHex => Value::Said(&item.oid_hex),
                    Role::Change => Value::Said(&item.change),
                    Role::Bucket => Value::Said(&item.bucket),
                    Role::Group => Value::Said(&item.group),
                    Role::OrigPath => Value::Said(&item.orig_path),
                    Role::IsHead => Value::Flag(item.is_head),
                    Role::HasRemote => Value::Flag(item.has_remote),
                    Role::OnlyRemote => Value::Flag(item.only_remote),
                    Role::Upstream => Value::Said(&item.upstream),
                    Role::HasPr => Value::Flag(item.has_pr),
                    Role::EolMark => Value::Flag(item.eol_mark),
                    Role::Depth => Value::Number(item.depth),
                    Role::Folder => Value::Flag(item.folder),
                };
            }
            Row::Shown { of, depth } => (of, depth),
        };
        match role {
            Role::Name => Value::Said(self.shown_name(of, depth)),
            // The tree is what writes a full name down; a row that stands
            // in none is known by the one it arrived with.
            Role::Full => Value::Said(if self.indented { of.name() } else { of.full() }),
            Role::Depth => Value::Number(depth),
            // A row of the source is a row of the list, never a folder.
            Role::Folder => Value::Flag(false),
            Role::OidHex => match of {
                Entry::Item(item) => Value::Said(&item.oid_hex),
                Entry::Tag(tag) => Value::Spelled(tag.oid.to_hex()),
                Entry::Remote(branch) => Value::Spelled(branch.oid.to_hex()),
            },
            Role::IsHead => Value::Flag(match of {
                Entry::Item(item) => item.is_head,
                Entry::Tag(_) => false,
                Entry::Remote(branch) => branch.is_head,
            }),
            Role::HasRemote => Value::Flag(match of {
                Entry::Item(item) => item.has_remote,
                // Same badge as a branch: nothing means this tag is only
                // here. The bit comes off `ls-remote --tags`, which the
                // fetch carries.
                Entry::Tag(tag) => tag.has_remote,
                Entry::Remote(branch) => branch.has_remote,
            }),
            Role::OnlyRemote => Value::Flag(match of {
                Entry::Item(item) => item.only_remote,
                // Written as the negative of core's `here`, so every other
                // kind of row keeps it off by default.
                Entry::Tag(tag) => !tag.here,
                Entry::Remote(_) => false,
            }),
            Role::Upstream => Value::Said(match of {
                Entry::Item(item) => &item.upstream,
                Entry::Tag(_) => "",
                Entry::Remote(branch) => &branch.upstream,
            }),
            Role::HasPr => Value::Flag(match of {
                Entry::Item(item) => item.has_pr,
                Entry::Tag(_) => false,
                Entry::Remote(branch) => {
                    crate::encode::fake_pr_set().contains(pr_key(&branch.short))
                }
            }),
            Role::Change => Value::Said(match of {
                Entry::Item(item) => &item.change,
                Entry::Tag(_) | Entry::Remote(_) => "",
            }),
            Role::Bucket => Value::Said(match of {
                Entry::Item(item) => &item.bucket,
                Entry::Tag(_) | Entry::Remote(_) => "",
            }),
            Role::Group => Value::Said(match of {
                Entry::Item(item) => &item.group,
                Entry::Tag(_) | Entry::Remote(_) => "",
            }),
            Role::OrigPath => Value::Said(match of {
                Entry::Item(item) => &item.orig_path,
                Entry::Tag(_) | Entry::Remote(_) => "",
            }),
            Role::EolMark => Value::Flag(matches!(of, Entry::Item(item) if item.eol_mark)),
        }
    }

    /// What the row identified by one field answers for another.
    ///
    /// Asks the section's whole source rather than the visible rows, so
    /// an active filter or a collapsed folder does not hide the answer —
    /// and asks it undented, because a name given from outside is the
    /// whole one git knows.
    fn told(&self, known: Role, text: &str, wanted: Role) -> String {
        (0..self.all.len())
            .filter_map(|at| self.all.entry(at))
            .map(|of| Row::Shown { of, depth: 0 })
            .find(|row| self.field(*row, known).as_str() == text)
            .map(|row| self.field(row, wanted).as_str().to_string())
            .unwrap_or_default()
    }

    /// What one row on screen shows for one role (empty out of range).
    fn shows(&self, row: i32, role: Role) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|at| self.row_at(at))
            .map(|row| self.field(row, role).as_str().to_string())
            .unwrap_or_default()
    }

    /// Section default: remote roots (one per remote) start collapsed —
    /// that is the per-repository fold — everything else starts open.
    fn folder_expanded(&self, key: &str, depth: i32) -> bool {
        self.folder_overrides
            .get(key)
            .copied()
            .unwrap_or(!(self.section == "remotes" && depth == 0))
    }

    /// Turns the flat sorted name list into an indented tree with
    /// collapsible folder rows for every `/` level.
    ///
    /// The leaves are pointed at rather than copied: what a row of the
    /// tree adds to the name the source holds is its depth, and the
    /// segment it shows falls out of that (`shown_name`).
    fn build_tree(&self) -> Vec<Arranged> {
        let mut out = Vec::new();
        let mut open_path: Vec<String> = Vec::new();
        // Depth at which a collapsed folder swallows its descendants.
        let mut collapsed_at: Option<usize> = None;

        for at in 0..self.all.len() {
            let Some(leaf) = self.all.entry(at) else {
                continue;
            };
            let name = leaf.name();
            let segments: Vec<&str> = name.split('/').collect();
            let folder_count = segments.len() - 1;

            // Longest common folder prefix with the previous entry.
            let mut common = 0;
            while common < open_path.len()
                && common < folder_count
                && open_path[common] == segments[common]
            {
                common += 1;
            }
            open_path.truncate(common);
            if let Some(depth) = collapsed_at
                && depth >= open_path.len()
            {
                collapsed_at = None;
            }

            for (depth, segment) in segments.iter().enumerate().take(folder_count).skip(common) {
                open_path.push((*segment).to_string());
                if collapsed_at.is_some() {
                    continue;
                }
                let key = open_path.join("/");
                let expanded = self.folder_expanded(&key, depth as i32);
                out.push(Arranged::Made(Box::new(NavItem {
                    name: (*segment).to_string(),
                    full: key,
                    depth: depth as i32,
                    folder: true,
                    change: fold_state(expanded),
                    ..Default::default()
                })));
                if !expanded {
                    collapsed_at = Some(depth);
                }
            }
            if collapsed_at.is_none() {
                out.push(Arranged::At {
                    at: at as u32,
                    depth: folder_count as i32,
                });
            }
        }
        out
    }
}

/// A remote branch is looked up without its remote prefix, so
/// `origin/main` matches a PR on `main`.
fn pr_key(short: &str) -> &str {
    short.split_once('/').map_or(short, |(_, rest)| rest)
}

fn branch_nav_items(list: &[platitude_core::session::BranchItem]) -> Vec<NavItem> {
    list.iter()
        .map(|b| NavItem {
            name: b.short.to_string(),
            oid_hex: b.oid.to_hex(),
            is_head: b.is_head,
            has_remote: b.has_remote,
            upstream: b.upstream.to_string(),
            has_pr: crate::encode::fake_pr_set().contains(b.short.as_str()),
            ..Default::default()
        })
        .collect()
}

/// Trees one group run of worktree entries: single-child directory
/// chains compact into one `a/b/c` row; fold-toggle keys are
/// group-prefixed so equal paths in different groups fold apart.
fn wt_tree_into(
    entries: &[NavItem],
    group: &str,
    overrides: &HashMap<String, bool>,
    out: &mut Vec<Arranged>,
) {
    #[derive(Default)]
    struct DirNode {
        dirs: std::collections::BTreeMap<String, DirNode>,
        files: Vec<NavItem>,
    }
    let mut root = DirNode::default();
    for entry in entries {
        let mut node = &mut root;
        let mut rest = entry.full.as_str();
        while let Some((dir, tail)) = rest.split_once('/') {
            node = node.dirs.entry(dir.to_string()).or_default();
            rest = tail;
        }
        let mut leaf = entry.clone();
        leaf.name = rest.to_string();
        node.files.push(leaf);
    }
    fn emit(
        node: &DirNode,
        group: &str,
        prefix: &str,
        depth: i32,
        overrides: &HashMap<String, bool>,
        out: &mut Vec<Arranged>,
    ) {
        for (dir_name, child) in &node.dirs {
            let mut label = dir_name.clone();
            let mut target = child;
            while target.files.is_empty() && target.dirs.len() == 1 {
                let Some((next_name, next)) = target.dirs.iter().next() else {
                    break;
                };
                label.push('/');
                label.push_str(next_name);
                target = next;
            }
            let path = format!("{prefix}{label}");
            let key = format!("{group}:{path}");
            let expanded = overrides.get(&key).copied().unwrap_or(true);
            out.push(Arranged::Made(Box::new(NavItem {
                name: label,
                full: key.clone(),
                // The path itself, for the hover of a row the pane
                // elided: `full` is the fold key, not a path. It rides
                // in the rename slot, which a folder never uses.
                orig_path: path.clone(),
                group: group.to_string(),
                depth,
                folder: true,
                change: fold_state(expanded),
                ..Default::default()
            })));
            if expanded {
                emit(
                    target,
                    group,
                    &format!("{path}/"),
                    depth + 1,
                    overrides,
                    out,
                );
            }
        }
        for f in &node.files {
            let mut item = f.clone();
            item.depth = depth;
            out.push(Arranged::Made(Box::new(item)));
        }
    }
    emit(&root, group, "", 0, overrides, out);
}

/// Working-tree entries in GitKraken display order: conflicts → unstaged
/// (untracked files count as unstaged for display, via `group`, while
/// `bucket` keeps the real diff/staging routing) → staged. `full` always
/// carries the real path (tree leaves rename `name` to their last
/// segment).
fn status_nav_items(
    status: &platitude_core::status::WorkTreeStatus,
    eol_marks: &[platitude_core::session::EolMark],
) -> Vec<NavItem> {
    let push = |out: &mut Vec<NavItem>,
                bucket: &str,
                group: &str,
                change: String,
                path: &str,
                orig: String| {
        out.push(NavItem {
            name: path.to_string(),
            full: path.to_string(),
            change,
            bucket: bucket.into(),
            group: group.into(),
            orig_path: orig,
            eol_mark: eol_marks.iter().any(|m| m.path == path),
            ..Default::default()
        });
    };
    let mut rows = Vec::new();
    for entry in status.conflicted() {
        if let platitude_core::status::StatusItem::Unmerged { ours, theirs, path } = entry {
            push(
                &mut rows,
                "conflicts",
                "conflicts",
                format!("{ours}{theirs}"),
                path,
                String::new(),
            );
        }
    }
    for entry in status.unstaged() {
        if let platitude_core::status::StatusItem::Tracked { unstaged, path, .. } = entry {
            push(
                &mut rows,
                "unstaged",
                "unstaged",
                unstaged.to_string(),
                path,
                String::new(),
            );
        }
    }
    for entry in status.untracked() {
        push(
            &mut rows,
            "untracked",
            "unstaged",
            "?".to_string(),
            entry.path(),
            String::new(),
        );
    }
    for entry in status.staged() {
        if let platitude_core::status::StatusItem::Tracked {
            staged,
            path,
            orig_path,
            ..
        } = entry
        {
            push(
                &mut rows,
                "staged",
                "staged",
                staged.to_string(),
                path,
                orig_path.clone().unwrap_or_default(),
            );
        }
    }
    rows
}

#[qobject(Base = QAbstractItemModel, ConvertToCamelCase, NoQmlElement)]
impl NavSectionModel {
    qproperty!("total", Member = total, Notify = changed);
    qproperty!("headName", Member = head_name, Notify = changed);
    qproperty!("headOid", Member = head_oid, Notify = changed);
    qproperty!("headHasRemote", Member = head_has_remote, Notify = changed);
    qproperty!("headHasPr", Member = head_has_pr, Notify = changed);
    qproperty!("headRow", Member = head_row, Notify = changed);
    qproperty!("refsLoaded", Member = refs_loaded, Notify = changed);
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
    /// built. An empty path clears it. Hover cannot be injected headless,
    /// so this is also what the automation writes — the same one property
    /// a real pointer moves.
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
    fn changed(&mut self);

    // A refs snapshot arrived, whether or not it moved anything.
    //
    // `changed` cannot answer this: it stays deliberately quiet when the
    // rows come out identical, because rebuilding a section holding tens
    // of thousands of tags for the same picture is work the sidebar can
    // see. A page holding a landing owed by a write needs to hear it all
    // the same — a cherry-pick of a commit the branch already has
    // records nothing and leaves the refs exactly as they were, and its
    // landing would otherwise stay armed until something unrelated moved
    // them (`RepoPage.tryPendingHeadSelect`).
    #[qsignal]
    fn refs_settled(&mut self);

    /// Wires this instance to one section's data feed. `section`:
    /// `branches` / `remotes` / `worktree` / `worktrees` / `stashes` /
    /// `tags`.
    #[qslot]
    fn attach_section(&mut self, tab_id: i32, section: String) {
        self.tab_id = tab_id;
        self.section = section;
        self.tree_view = true;
        let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) else {
            return;
        };
        let invoker = self.get_qml_method_invoker();
        match self.section.as_str() {
            "branches" => {
                let feed = Arc::clone(&feeds.refs_branches);
                feed.attach(invoker);
                self.refs_feed = Some(feed);
            }
            "remotes" => {
                let feed = Arc::clone(&feeds.refs_remotes);
                feed.attach(invoker);
                self.refs_feed = Some(feed);
            }
            "tags" => {
                let feed = Arc::clone(&feeds.refs_tags);
                feed.attach(invoker);
                self.refs_feed = Some(feed);
            }
            "worktree" => {
                let feed = Arc::clone(&feeds.status_nav);
                feed.attach(invoker);
                self.status_feed = Some(feed);
            }
            "stashes" => {
                let feed = Arc::clone(&feeds.stash);
                feed.attach(invoker);
                self.stash_feed = Some(feed);
            }
            "worktrees" => {
                let feed = Arc::clone(&feeds.worktrees);
                feed.attach(invoker);
                self.worktrees_feed = Some(feed);
            }
            other => tracing::warn!(section = other, "unknown sidebar section"),
        }
    }

    /// Installs freshly built rows, and answers whether they were new.
    ///
    /// A poll tick republishes refs and status whether or not they moved,
    /// so most arrivals carry exactly what the section already shows.
    /// Swapping those in would still reset the Qt model — every delegate
    /// rebuilt, the inner list scrolled back — for an identical picture.
    fn take_rows(&mut self, rows: Vec<NavItem>) -> bool {
        if self.all.kept() == rows {
            return false;
        }
        self.all = Source::Kept(rows);
        true
    }

    /// Points the section at a newly published snapshot, and answers
    /// whether the rows it shows moved.
    ///
    /// The pointer check upstream has already said this is a different
    /// snapshot; this is the other question — whether *these* rows differ
    /// — and it is asked of the entries themselves, which is what the
    /// projection leaves to compare. A poll that found one branch moved
    /// is not a reason for the tag section to rebuild forty-five thousand
    /// delegates.
    ///
    /// The new handle is taken either way: it holds what the old one did,
    /// and the session has moved on to it, so keeping the old one alive
    /// would be a second snapshot on the heap saying the same thing.
    fn take_refs(&mut self, taken: Source) -> bool {
        let moved = match (&self.all, &taken) {
            (Source::Tags(held), Source::Tags(fresh)) => held.tags != fresh.tags,
            (Source::Remotes(held), Source::Remotes(fresh)) => held.remotes != fresh.remotes,
            _ => true,
        };
        self.all = taken;
        moved
    }

    /// The arrived rows, and the shaped ones where they are a second list.
    ///
    /// Filed apart on purpose: `arranged` being nothing is what says the
    /// view is reading the source directly, and a single number would hide
    /// the day that stops being true. A projected section reports no bytes
    /// of its own — the snapshot it reads is the session's, counted there.
    fn note_footprint(&self) {
        crate::memprobe::note_bytes(
            &format!("nav-{}-all", self.section),
            self.tab_id,
            platitude_core::mem::Footprint::heap_bytes(&self.all),
            self.all.len(),
        );
        crate::memprobe::note_bytes(
            &format!("nav-{}-arranged", self.section),
            self.tab_id,
            self.arranged
                .as_ref()
                .map_or(0, platitude_core::mem::Footprint::heap_bytes),
            self.arranged.as_ref().map_or(0, Vec::len),
        );
    }

    #[qslot]
    fn drain(&mut self) {
        // Every push queues its own `drain`, so a second call can find the
        // queue already emptied by the first. Nothing arrived means
        // nothing to rebuild.
        let mut arrived = false;
        // Whether refs were published at all, which is a different
        // question from whether they moved (see `refs_settled`).
        let mut settled = false;
        if let Some(feed) = self.refs_feed.clone()
            && let Some(snapshot) = feed.drain().pop()
        {
            settled = true;
            // The first snapshot is news whatever it holds: the default
            // selection is waiting on `refsLoaded`, and a section that is
            // legitimately empty would otherwise never say so.
            arrived |= !self.refs_loaded;
            self.refs_loaded = true;
            // Not the same snapshot means it has to be read; the same one
            // means these rows were built from it already.
            let fresh = !self
                .last_refs
                .as_ref()
                .is_some_and(|last| Arc::ptr_eq(last, &snapshot));
            if fresh {
                self.last_refs = Some(Arc::clone(&snapshot));
                arrived |= match self.section.as_str() {
                    "branches" => {
                        let items = branch_nav_items(&snapshot.locals);
                        let head = items.iter().find(|b| b.is_head);
                        self.head_name = head.map(|b| b.name.clone()).unwrap_or_default();
                        self.head_oid = head.map(|b| b.oid_hex.clone()).unwrap_or_default();
                        self.head_has_remote = head.is_some_and(|b| b.has_remote);
                        self.head_has_pr = head.is_some_and(|b| b.has_pr);
                        self.take_rows(items)
                    }
                    "remotes" => self.take_refs(Source::Remotes(snapshot)),
                    _ => self.take_refs(Source::Tags(snapshot)),
                };
            }
        }
        if let Some(feed) = self.status_feed.clone()
            && let Some(StatusMsg {
                status, eol_marks, ..
            }) = feed.drain().pop()
        {
            let rows = status_nav_items(&status, &eol_marks);
            self.eol_marks = eol_marks;
            arrived |= self.take_rows(rows);
        }
        if let Some(feed) = self.worktrees_feed.clone()
            && let Some(list) = feed.drain().pop()
        {
            // The current worktree is marked like the current branch;
            // `bucket` carries the branch (empty = detached) and `full`
            // the absolute path (tooltip + click-to-open).
            let current = Hub::with(|hub| hub.session(self.tab_id).and_then(|s| s.workdir()))
                .flatten()
                .map(|p| p.to_string_lossy().replace('\\', "/").to_lowercase())
                .unwrap_or_default();
            let rows = list
                .into_iter()
                .filter(|w| !w.bare)
                .map(|w| {
                    let norm = w.path.replace('\\', "/");
                    let name = norm.rsplit('/').next().unwrap_or(norm.as_str()).to_string();
                    let has_pr = w
                        .branch
                        .as_deref()
                        .is_some_and(|b| crate::encode::fake_pr_set().contains(b));
                    NavItem {
                        name,
                        is_head: norm.to_lowercase() == current,
                        full: w.path,
                        bucket: w.branch.unwrap_or_default(),
                        has_pr,
                        ..Default::default()
                    }
                })
                .collect();
            arrived |= self.take_rows(rows);
        }
        if let Some(feed) = self.stash_feed.clone()
            && let Some(stashes) = feed.drain().pop()
        {
            // The message is the whole of what a stash shows: the reflog
            // selector (stash@{0}) stays out of sight by request, but it
            // rides along in `full` because it is what names the entry to
            // git — renaming one takes the selector, not the message.
            // The commit id makes rows clickable: the details pane then
            // shows the stashed changes.
            let rows = stashes
                .into_iter()
                .map(|s| NavItem {
                    name: s.message,
                    full: s.name,
                    oid_hex: s.oid.to_hex(),
                    ..Default::default()
                })
                .collect();
            arrived |= self.take_rows(rows);
        }
        if arrived {
            self.total = self.all.len() as i32;
            self.reshape();
            self.changed();
        }
        if settled {
            self.refs_settled();
        }
        if crate::memprobe::enabled() {
            self.note_footprint();
        }
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
        // Folding moves rows around (and can swallow the current one).
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

    /// Filtered row count (`total` counts all rows; this counts what the
    /// filter lets through).
    #[qslot]
    fn shown(&self) -> i32 {
        self.shown_rows() as i32
    }

    /// Commit id of the ref with this name; empty when there is none.
    ///
    /// Looks in the section's whole list rather than the visible rows, so
    /// an active filter or a collapsed folder does not hide the answer.
    #[qslot]
    fn oid_of_name(&self, name: String) -> String {
        self.told(Role::Name, &name, Role::OidHex)
    }

    /// The remote branch this one speaks for (`origin/main`); empty when
    /// it speaks for none, and for every section but the branches.
    #[qslot]
    fn upstream_of(&self, name: String) -> String {
        self.told(Role::Name, &name, Role::Upstream)
    }

    /// What one row shows, and what git knows it by (empty out of range).
    ///
    /// Roles are only visible to a delegate, so this is how automation
    /// reaches a row it has to act on.
    #[qslot]
    fn name_at(&self, row: i32) -> String {
        self.shows(row, Role::Name)
    }

    #[qslot]
    fn full_at(&self, row: i32) -> String {
        self.shows(row, Role::Full)
    }

    /// The two stage letters git reports for a working-tree path (`UU`,
    /// `DU`, …); empty for any path that is not in this section.
    ///
    /// The diff pane asks for it by path because that is all it holds: it
    /// is handed a file, not the row the file came from, and a conflict
    /// git prints no patch for is a pane with nothing to say unless it can
    /// name what the two sides did.
    #[qslot]
    fn change_of(&self, path: String) -> String {
        self.told(Role::Full, &path, Role::Change)
    }
}
qml_register!(NavSectionModel, "NavSectionModel", singleton = false);

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(digit: &str) -> platitude_core::Oid {
        platitude_core::Oid::from_hex_str(&digit.repeat(40)).unwrap()
    }

    /// The names the delegate asks by and the numbers `data` is called
    /// with come from two places; a role that answers under the wrong one
    /// draws an empty row and reports nothing, so they are pinned here.
    #[test]
    fn every_role_the_view_is_handed_is_answered_by_the_same_name() {
        let handed = <NavItem as QModelItem>::role_names();
        assert_eq!(handed.len(), 15, "the view is handed one role per field");
        for (number, name) in handed {
            let role = Role::of(number);
            assert!(
                role.is_some(),
                "role {number} ({name}) has no answer at all"
            );
            assert_eq!(
                role.map(Role::spelling),
                Some(name.as_str()),
                "role {number} is answered under another name",
            );
        }
    }

    fn tag(short: &str, has_remote: bool, here: bool) -> platitude_core::session::TagItem {
        platitude_core::session::TagItem {
            short: short.into(),
            oid: oid("a"),
            annotated: false,
            created_unix: 0,
            has_remote,
            here,
        }
    }

    fn remote(short: &str) -> platitude_core::session::BranchItem {
        platitude_core::session::BranchItem {
            short: short.into(),
            full: format!("refs/remotes/{short}").into(),
            oid: oid("b"),
            has_remote: true,
            is_head: false,
            upstream: "".into(),
        }
    }

    fn snapshot(
        remotes: Vec<platitude_core::session::BranchItem>,
        tags: Vec<platitude_core::session::TagItem>,
    ) -> Arc<platitude_core::session::RefsSnapshot> {
        Arc::new(platitude_core::session::RefsSnapshot {
            locals: Vec::new(),
            remotes,
            tags,
            head: None,
            remote_names: Vec::new(),
        })
    }

    /// One section, wired to nothing: the shaping and the answers are
    /// plain Rust, so a test needs no Qt side at all.
    fn section(kind: &str, all: Source) -> NavSectionModel {
        let mut model = NavSectionModel::default();
        model.section = kind.to_string();
        model.all = all;
        model
    }

    fn says(model: &NavSectionModel, row: usize, role: Role) -> String {
        model
            .row_at(row)
            .map(|row| model.field(row, role).as_str().to_string())
            .unwrap_or_else(|| "<no row>".to_string())
    }

    fn flags(model: &NavSectionModel, row: usize, role: Role) -> bool {
        model
            .row_at(row)
            .is_some_and(|row| model.field(row, role).flag())
    }

    fn depth_of(model: &NavSectionModel, row: usize) -> i32 {
        match model.row_at(row).map(|row| model.field(row, Role::Depth)) {
            Some(Value::Number(depth)) => depth,
            _ => -1,
        }
    }

    /// A tag is drawn out of the snapshot, badges and all — the rows it
    /// used to be held as were only ever a copy of this.
    #[test]
    fn a_tag_row_reads_out_of_the_snapshot() {
        let mut model = section(
            "tags",
            Source::Tags(snapshot(
                Vec::new(),
                vec![tag("v1.0", true, true), tag("v2.0-theirs", true, false)],
            )),
        );
        model.arrange();

        assert_eq!(model.shown_rows(), 2);
        assert_eq!(says(&model, 0, Role::Name), "v1.0");
        // A tag never went through the tree, so it has no full name and
        // the sidebar keys it by what it shows.
        assert_eq!(says(&model, 0, Role::Full), "");
        assert_eq!(says(&model, 0, Role::OidHex), oid("a").to_hex());
        assert!(flags(&model, 0, Role::HasRemote));
        assert!(!flags(&model, 0, Role::OnlyRemote));
        // `here` is false: the name greys, which is the whole reason a tag
        // no local ref reaches is listed at all.
        assert!(flags(&model, 1, Role::OnlyRemote));
        assert!(!flags(&model, 1, Role::Folder));
        assert_eq!(depth_of(&model, 1), 0);
    }

    /// The remote tree indents rows it points at, and the segment each
    /// one shows falls out of its depth.
    #[test]
    fn the_remote_tree_points_at_rows_and_folds_over_them() {
        let mut model = section(
            "remotes",
            Source::Remotes(snapshot(
                vec![remote("origin/feature/one"), remote("origin/main")],
                Vec::new(),
            )),
        );
        // Remote roots start collapsed, so the folder row is all there is.
        model.arrange();
        assert_eq!(model.shown_rows(), 1);
        assert_eq!(says(&model, 0, Role::Name), "origin");
        assert!(flags(&model, 0, Role::Folder));
        assert_eq!(says(&model, 0, Role::Change), FOLDED);

        model.folder_overrides.insert("origin".to_string(), true);
        model.arrange();
        // origin / feature / one / main
        assert_eq!(model.shown_rows(), 4);
        assert_eq!(says(&model, 1, Role::Name), "feature");
        assert_eq!(says(&model, 2, Role::Name), "one");
        assert_eq!(says(&model, 2, Role::Full), "origin/feature/one");
        assert_eq!(depth_of(&model, 2), 2);
        assert_eq!(says(&model, 3, Role::Name), "main");
        assert_eq!(says(&model, 3, Role::Full), "origin/main");
        assert_eq!(depth_of(&model, 3), 1);
        assert_eq!(says(&model, 3, Role::OidHex), oid("b").to_hex());

        // A name from outside is the whole one git knows, whatever the
        // tree is showing.
        assert_eq!(
            model.oid_of_name("origin/main".to_string()),
            oid("b").to_hex()
        );
        assert_eq!(model.name_at(2), "one");
    }

    /// Filtering shows whole names and stands in no tree.
    #[test]
    fn a_filtered_remote_row_shows_its_whole_name() {
        let mut model = section(
            "remotes",
            Source::Remotes(snapshot(
                vec![remote("origin/feature/one"), remote("origin/main")],
                Vec::new(),
            )),
        );
        model.filter = "feature".to_string();
        model.arrange();

        assert_eq!(model.shown_rows(), 1);
        assert_eq!(says(&model, 0, Role::Name), "origin/feature/one");
        assert_eq!(says(&model, 0, Role::Full), "");
    }

    /// A snapshot that carries the same tags is not a reason to rebuild
    /// tens of thousands of delegates.
    #[test]
    fn a_republished_snapshot_moves_nothing() {
        let mut model = section("tags", Source::default());
        assert!(model.take_refs(Source::Tags(snapshot(
            Vec::new(),
            vec![tag("v1.0", true, true)]
        ))));
        assert!(!model.take_refs(Source::Tags(snapshot(
            Vec::new(),
            vec![tag("v1.0", true, true)]
        ))));
        assert!(model.take_refs(Source::Tags(snapshot(
            Vec::new(),
            vec![tag("v1.1", true, true)]
        ))));
    }
}
