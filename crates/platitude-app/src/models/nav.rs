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

// What a row of the sidebar can say, and the one row that is held rather
// than read: **a folder row**, which no section's data arrives as, so
// there is nothing to project it from. Every other row is answered field
// by field out of what arrived (`Source`), and the fields below are then
// only the declaration — `#[derive(QModelItem)]` turns them into the role
// names the delegate resolves by, and `Role` answers under those names
// (held to this list by the test at the foot of the file).
#[derive(QModelItem, Default)]
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
    /// (`FOLDED`, empty when open) — a row is one of five kinds and
    /// qtbridge's `QModelItem` allows fifteen fields, so a slot that
    /// structurally cannot be used twice at once is shared. Reading it is
    /// guarded by `folder` everywhere, as the other shared fields are
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
    /// The same source written the way the row writes names — what a
    /// delegate shows (`encode::rename_source`). A made row never carries
    /// one; the field is here because **the view's role table is one role
    /// per field of this struct** (the test at the foot of this file), and
    /// a role no field stands for cannot be asked for by name.
    orig_name: String,
    is_head: bool,
    has_remote: bool,
    /// This repository does not hold the ref, so the name greys. Only tags
    /// are ever listed that way: a tag a remote has and this one does not
    /// reaches no graph row, and the sidebar is where it can be read at
    /// all. Written as the negative of core's `here` so every other kind
    /// of row keeps it off by default.
    only_remote: bool,
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
            + self.orig_name.heap_bytes()
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
/// A row that is only ever drawn does not have to be built: every section
/// keeps what arrived, in the shape it arrived in, and answers for a row
/// as the view asks. The two a large repository fills read out of the
/// refs snapshot the session already holds and this model already points
/// at, so their whole lists cost one `Arc` and, where the shaping has
/// anything to say, an index per visible row. Measured on
/// `JetBrains/kotlin`: holding those two as rows was 13.9MB of the
/// process's Rust heap, all of it a second copy of what the snapshot says.
#[derive(Default)]
enum Source {
    /// Nothing has arrived yet.
    #[default]
    Waiting,
    /// `snapshot.locals`.
    Locals(Arc<platitude_core::session::RefsSnapshot>),
    /// `snapshot.remotes`.
    Remotes(Arc<platitude_core::session::RefsSnapshot>),
    /// `snapshot.tags`.
    Tags(Arc<platitude_core::session::RefsSnapshot>),
    Stashes(Vec<platitude_core::stash::StashEntry>),
    /// The working copies, and which of them this window is showing —
    /// the one fact about a worktree row that is not in the entry.
    Worktrees {
        list: Vec<platitude_core::worktrees::WorktreeEntry>,
        current: String,
    },
    /// The pending changes, with the order the pane shows them in. **A
    /// row is a bucket and an entry**, not an entry: an entry with both
    /// halves changed (`MM`) is one row under the index and another under
    /// the working tree.
    Files {
        status: platitude_core::status::WorkTreeStatus,
        order: Vec<FileAt>,
    },
}

impl platitude_core::mem::Footprint for Source {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::Waiting => 0,
            // Nothing of its own: the snapshot belongs to the session,
            // which is where the report counts it. A shared `Arc` added up
            // at every pointer into it is a number that means nothing.
            Self::Locals(_) | Self::Remotes(_) | Self::Tags(_) => 0,
            Self::Stashes(list) => list.heap_bytes(),
            Self::Worktrees { list, current } => list.heap_bytes() + current.heap_bytes(),
            Self::Files { status, order } => {
                status.heap_bytes() + order.capacity() * size_of::<FileAt>()
            }
        }
    }
}

impl Source {
    fn len(&self) -> usize {
        match self {
            Self::Waiting => 0,
            Self::Locals(snapshot) => snapshot.locals.len(),
            Self::Remotes(snapshot) => snapshot.remotes.len(),
            Self::Tags(snapshot) => snapshot.tags.len(),
            Self::Stashes(list) => list.len(),
            Self::Worktrees { list, .. } => list.len(),
            Self::Files { order, .. } => order.len(),
        }
    }

    fn entry(&self, at: usize) -> Option<Entry<'_>> {
        match self {
            Self::Waiting => None,
            Self::Locals(snapshot) => snapshot.locals.get(at).map(Entry::Local),
            Self::Remotes(snapshot) => snapshot.remotes.get(at).map(Entry::Remote),
            Self::Tags(snapshot) => snapshot.tags.get(at).map(Entry::Tag),
            Self::Stashes(list) => list.get(at).map(Entry::Stash),
            Self::Worktrees { list, current } => {
                list.get(at).map(|entry| Entry::Worktree { entry, current })
            }
            Self::Files { status, order } => order.get(at).and_then(|row| {
                status.items.get(row.at as usize).map(|item| Entry::File {
                    item,
                    bucket: row.bucket,
                })
            }),
        }
    }

    /// The working copies that arrived, with the pane's order worked out
    /// once rather than per row.
    fn files(status: platitude_core::status::WorkTreeStatus) -> Self {
        let mut order = Vec::new();
        for bucket in Bucket::SHOWN {
            for (at, item) in status.items.iter().enumerate() {
                if bucket.holds(item) {
                    order.push(FileAt {
                        bucket,
                        at: at as u32,
                    });
                }
            }
        }
        Self::Files { status, order }
    }
}

/// One file row: which of git's four answers it came out of, and the
/// entry in the status it came from.
struct FileAt {
    bucket: Bucket,
    at: u32,
}

/// Which of git's four answers a file row came out of.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Bucket {
    Conflicts,
    Unstaged,
    Untracked,
    Staged,
}

impl Bucket {
    /// GitKraken display order, which is also the order the pane's group
    /// runs come in.
    const SHOWN: [Self; 4] = [
        Self::Conflicts,
        Self::Unstaged,
        Self::Untracked,
        Self::Staged,
    ];

    /// Whether an entry belongs in this bucket — the same test
    /// `WorkTreeStatus` makes for its four iterators, asked one entry at a
    /// time because a row has to know which of them it came out of.
    fn holds(self, item: &platitude_core::status::StatusItem) -> bool {
        use platitude_core::status::StatusItem;
        match (self, item) {
            (Self::Conflicts, StatusItem::Unmerged { .. })
            | (Self::Untracked, StatusItem::Untracked { .. }) => true,
            (Self::Unstaged, StatusItem::Tracked { unstaged, .. }) => *unstaged != '.',
            (Self::Staged, StatusItem::Tracked { staged, .. }) => *staged != '.',
            _ => false,
        }
    }

    /// What the row routes diffs and staging by.
    fn routing(self) -> &'static str {
        match self {
            Self::Conflicts => "conflicts",
            Self::Unstaged => "unstaged",
            Self::Untracked => "untracked",
            Self::Staged => "staged",
        }
    }

    /// The run the pane shows it in — where an untracked file counts as
    /// unstaged, while `routing` keeps the real answer.
    fn run(self) -> &'static str {
        match self {
            Self::Conflicts => "conflicts",
            Self::Unstaged | Self::Untracked => "unstaged",
            Self::Staged => "staged",
        }
    }
}

/// One entry of a source, whichever kind the section has.
#[derive(Clone, Copy)]
enum Entry<'a> {
    Local(&'a platitude_core::session::BranchItem),
    Remote(&'a platitude_core::session::BranchItem),
    Tag(&'a platitude_core::session::TagItem),
    Stash(&'a platitude_core::stash::StashEntry),
    Worktree {
        entry: &'a platitude_core::worktrees::WorktreeEntry,
        /// The working copy this window is showing, spelled the way
        /// `path` is compared (forward slashes, lower case).
        current: &'a str,
    },
    File {
        item: &'a platitude_core::status::StatusItem,
        bucket: Bucket,
    },
}

impl<'a> Entry<'a> {
    /// What git calls it, before any indenting takes it apart.
    fn name(self) -> &'a str {
        match self {
            Self::Local(branch) | Self::Remote(branch) => &branch.short,
            Self::Tag(tag) => &tag.short,
            // A stash shows its message; the selector that names it to
            // git rides in `full`.
            Self::Stash(stash) => &stash.message,
            Self::Worktree { entry, .. } => leaf_of(&entry.path),
            Self::File { item, .. } => item.path(),
        }
    }

    /// What git knows the row by, where that is not what it shows. A ref
    /// has none until the tree writes one (`field`), because a name is
    /// all git needs to be given.
    fn full(self) -> &'a str {
        match self {
            Self::Local(_) | Self::Remote(_) | Self::Tag(_) => "",
            Self::Stash(stash) => &stash.name,
            Self::Worktree { entry, .. } => &entry.path,
            Self::File { item, .. } => item.path(),
        }
    }
}

/// The last segment of a path, however it is spelled. A worktree row
/// shows the folder it lives in, and git prints the path the platform's
/// way.
fn leaf_of(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// One row of the shaped list.
///
/// Sixteen bytes where a row of the source can be pointed at, against the
/// two hundred a `NavItem` costs; a folder row is the one thing no source
/// holds, so it is the one thing carried whole (behind a box, so the
/// pointed-at rows are not all widened to hold one).
///
/// `from` is where in the whole name the segment this row shows begins —
/// worked out by whichever tree placed the row, because the two do not
/// agree on it: the refs tree indents one folder per `/`, while the
/// working tree's compacts a chain of single-child folders into one row
/// and leaves the file showing only its last segment.
enum Arranged {
    At { at: u32, depth: i32, from: u32 },
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
    /// A folder row — the one kind of row no source holds, so the one
    /// kind this section had to build.
    Made(&'a NavItem),
    /// A row of the source, shown at this depth and from this point in
    /// its name.
    Shown {
        of: Entry<'a>,
        depth: i32,
        from: usize,
    },
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
    OrigName,
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
    /// Every role the view is handed, in the order their numbers run —
    /// which is the order `NavItem` declares its fields in, one for one
    /// (the test at the foot of this file holds them together).
    ///
    /// **`Upstream` is not among them.** A `QModelItem` holds fifteen
    /// fields at most, no delegate has ever asked for that one, and the
    /// row that a rename's source has to reach is a delegate. It is still
    /// answered — `upstream_of` reads it straight out of `field`, which
    /// works it out from the entry rather than from a table.
    const ALL: [Self; 15] = [
        Self::Name,
        Self::Full,
        Self::OidHex,
        Self::Change,
        Self::Bucket,
        Self::Group,
        Self::OrigPath,
        Self::OrigName,
        Self::IsHead,
        Self::HasRemote,
        Self::OnlyRemote,
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
            Self::OrigName => "orig_name",
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
    /// Whether a tree placed these rows, which is the one thing that
    /// gives a **ref** row a full name: a branch arrives knowing only
    /// what it is called, and the tree is what decides that `main` shown
    /// under `origin` is `origin/main` to git. Every other kind of row
    /// arrived with both, so this does not concern them — and a filtered
    /// list stands in no tree, which is why its refs have no full name
    /// (as they did not when the filter built them by hand).
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
        // A tree is what gives a ref row a full name to be known by, and
        // it is the shaping a filter replaces.
        self.tree_named =
            needle.is_empty() && matches!(self.section.as_str(), "branches" | "remotes");
        self.arranged = if needle.is_empty() {
            match self.section.as_str() {
                "branches" | "remotes" => Some(self.build_tree()),
                // The worktree keeps its group runs (conflicts → unstaged →
                // staged) and trees each run independently.
                "worktree" if self.tree_view => {
                    let mut out = Vec::new();
                    let mut at = 0;
                    while at < self.all.len() {
                        let run = self.run_of(at);
                        let mut end = at + 1;
                        while end < self.all.len() && self.run_of(end) == run {
                            end += 1;
                        }
                        self.wt_tree_into(at..end, run, &mut out);
                        at = end;
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
                        from: 0,
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
                Arranged::At { at, depth, from } => Some(Row::Shown {
                    of: self.all.entry(*at as usize)?,
                    depth: *depth,
                    from: *from as usize,
                }),
                Arranged::Made(item) => Some(Row::Made(item)),
            },
            None => Some(Row::Shown {
                of: self.all.entry(at)?,
                depth: 0,
                from: 0,
            }),
        }
    }

    /// What the row is called on screen: what the tree left of its whole
    /// name, which is the whole of it wherever no tree placed the row.
    fn shown_name(of: Entry<'_>, from: usize) -> &str {
        let whole = of.name();
        whole.get(from..).unwrap_or(whole)
    }

    /// What one row answers for one role.
    ///
    /// **The only place a row's fields are worked out.** The view reads
    /// it through `data`, and the slots below read it directly, so no row
    /// can show the delegate one thing and tell automation another.
    fn field<'a>(&self, row: Row<'a>, role: Role) -> Value<'a> {
        let (of, depth, from) = match row {
            // A folder row is the one kind nothing arrived as, so it is
            // the one kind held whole — and it holds what it shows.
            Row::Made(item) => {
                return match role {
                    Role::Name => Value::Said(&item.name),
                    Role::Full => Value::Said(&item.full),
                    Role::OidHex => Value::Said(&item.oid_hex),
                    Role::Change => Value::Said(&item.change),
                    Role::Bucket => Value::Said(&item.bucket),
                    Role::Group => Value::Said(&item.group),
                    Role::OrigPath => Value::Said(&item.orig_path),
                    // A folder row has no rename to write down; the slot
                    // beside this one is carrying its own path instead.
                    Role::OrigName => Value::Said(""),
                    Role::IsHead => Value::Flag(item.is_head),
                    Role::HasRemote => Value::Flag(item.has_remote),
                    Role::OnlyRemote => Value::Flag(item.only_remote),
                    // Nothing made here speaks for a remote branch: the
                    // rows that do arrive from the source, where this is
                    // read off the entry.
                    Role::Upstream => Value::Said(""),
                    Role::HasPr => Value::Flag(item.has_pr),
                    Role::EolMark => Value::Flag(item.eol_mark),
                    Role::Depth => Value::Number(item.depth),
                    Role::Folder => Value::Flag(item.folder),
                };
            }
            Row::Shown { of, depth, from } => (of, depth, from),
        };
        match role {
            Role::Name => Value::Said(Self::shown_name(of, from)),
            // A tree is what writes a full name down for a ref; every
            // other row arrived knowing what git calls it.
            Role::Full => Value::Said(if self.tree_named && of.full().is_empty() {
                of.name()
            } else {
                of.full()
            }),
            Role::Depth => Value::Number(depth),
            // A row of the source is a row of the list, never a folder.
            Role::Folder => Value::Flag(false),
            Role::OidHex => match of {
                Entry::Local(branch) | Entry::Remote(branch) => Value::Spelled(branch.oid.to_hex()),
                Entry::Tag(tag) => Value::Spelled(tag.oid.to_hex()),
                // The commit makes the row clickable: the details pane
                // then shows the stashed changes.
                Entry::Stash(stash) => Value::Spelled(stash.oid.to_hex()),
                Entry::Worktree { .. } | Entry::File { .. } => Value::Said(""),
            },
            Role::IsHead => Value::Flag(match of {
                Entry::Local(branch) | Entry::Remote(branch) => branch.is_head,
                // The working copy this window shows is marked the way the
                // current branch is.
                Entry::Worktree { entry, current } => {
                    entry.path.replace('\\', "/").to_lowercase() == current
                }
                Entry::Tag(_) | Entry::Stash(_) | Entry::File { .. } => false,
            }),
            Role::HasRemote => Value::Flag(match of {
                Entry::Local(branch) | Entry::Remote(branch) => branch.has_remote,
                // Same badge as a branch: nothing means this tag is only
                // here. The bit comes off `ls-remote --tags`, which the
                // fetch carries.
                Entry::Tag(tag) => tag.has_remote,
                Entry::Stash(_) | Entry::Worktree { .. } | Entry::File { .. } => false,
            }),
            // Written as the negative of core's `here`, so every other
            // kind of row keeps it off by default.
            Role::OnlyRemote => Value::Flag(matches!(of, Entry::Tag(tag) if !tag.here)),
            Role::Upstream => Value::Said(match of {
                Entry::Local(branch) | Entry::Remote(branch) => &branch.upstream,
                _ => "",
            }),
            Role::HasPr => Value::Flag(match of {
                Entry::Local(branch) => {
                    crate::encode::fake_pr_set().contains(branch.short.as_str())
                }
                Entry::Remote(branch) => {
                    crate::encode::fake_pr_set().contains(pr_key(&branch.short))
                }
                Entry::Worktree { entry, .. } => entry
                    .branch
                    .as_deref()
                    .is_some_and(|branch| crate::encode::fake_pr_set().contains(branch)),
                Entry::Tag(_) | Entry::Stash(_) | Entry::File { .. } => false,
            }),
            // The letters git reports for the file, which for a conflict
            // is what each side did to it.
            Role::Change => match of {
                Entry::File { item, bucket } => Value::Spelled(letters_of(item, bucket)),
                _ => Value::Said(""),
            },
            Role::Bucket => Value::Said(match of {
                Entry::File { bucket, .. } => bucket.routing(),
                // A worktree row carries its branch here (empty =
                // detached), which is what the row shows on its right.
                Entry::Worktree { entry, .. } => entry.branch.as_deref().unwrap_or(""),
                _ => "",
            }),
            Role::Group => Value::Said(match of {
                Entry::File { bucket, .. } => bucket.run(),
                _ => "",
            }),
            Role::OrigPath => Value::Said(match of {
                // Where a rename came from, which only the staged side of
                // one knows.
                Entry::File {
                    item: platitude_core::status::StatusItem::Tracked { orig_path, .. },
                    bucket: Bucket::Staged,
                } => orig_path.as_deref().unwrap_or(""),
                _ => "",
            }),
            // The same source, written the way this row writes names: the
            // tree has already spelled `from` bytes of the new path in the
            // folders above, and a source that shared them gives them up
            // too (`encode::rename_source`). The flat view cuts nothing.
            Role::OrigName => Value::Said(match of {
                Entry::File {
                    item: platitude_core::status::StatusItem::Tracked { orig_path, .. },
                    bucket: Bucket::Staged,
                } => crate::encode::rename_source(
                    orig_path.as_deref().unwrap_or(""),
                    of.name(),
                    from,
                ),
                _ => "",
            }),
            // Kept once on the model rather than on every row: the marks
            // are few, and the row only has to say that it has one (the
            // sentence is `pointEol`).
            Role::EolMark => Value::Flag(matches!(of, Entry::File { item, .. }
                if self.eol_marks.iter().any(|mark| mark.path == item.path()))),
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
            .map(|of| Row::Shown {
                of,
                depth: 0,
                from: 0,
            })
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

    /// The display run a source row sits in — what the working tree's
    /// list is built one of at a time, so that a folder of the same name
    /// under two of them folds apart.
    fn run_of(&self, at: usize) -> &'static str {
        match self.all.entry(at) {
            Some(Entry::File { bucket, .. }) => bucket.run(),
            _ => "",
        }
    }

    /// Trees one group run of file rows: single-child directory chains
    /// compact into one `a/b/c` row; fold-toggle keys are group-prefixed
    /// so equal paths in different groups fold apart.
    fn wt_tree_into(&self, run: std::ops::Range<usize>, group: &str, out: &mut Vec<Arranged>) {
        #[derive(Default)]
        struct DirNode {
            dirs: std::collections::BTreeMap<String, DirNode>,
            /// The source rows sitting in this directory, each with where
            /// in its path the file's own name begins.
            files: Vec<(u32, u32)>,
        }
        let mut root = DirNode::default();
        for at in run {
            let Some(of) = self.all.entry(at) else {
                continue;
            };
            let mut node = &mut root;
            let mut rest = of.full();
            let mut from = 0;
            while let Some((dir, tail)) = rest.split_once('/') {
                node = node.dirs.entry(dir.to_string()).or_default();
                from += dir.len() + 1;
                rest = tail;
            }
            node.files.push((at as u32, from as u32));
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
            for (at, from) in &node.files {
                out.push(Arranged::At {
                    at: *at,
                    depth,
                    from: *from,
                });
            }
        }
        emit(&root, group, "", 0, &self.folder_overrides, out);
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
                    // One folder per `/`, so the segment on show starts
                    // after the last of them.
                    from: (name.len() - segments[folder_count].len()) as u32,
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

/// The letters git reports for one file row: what each side did to a
/// conflicted file, the half of an ordinary change this row is about, or
/// the mark for a file git has never been told about.
fn letters_of(item: &platitude_core::status::StatusItem, bucket: Bucket) -> String {
    use platitude_core::status::StatusItem;
    match (item, bucket) {
        (StatusItem::Unmerged { ours, theirs, .. }, _) => format!("{ours}{theirs}"),
        (StatusItem::Tracked { unstaged, .. }, Bucket::Unstaged) => unstaged.to_string(),
        (StatusItem::Tracked { staged, .. }, Bucket::Staged) => staged.to_string(),
        (StatusItem::Untracked { .. }, _) => "?".to_string(),
        _ => String::new(),
    }
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

    /// Points the section at what just arrived, and answers whether the
    /// rows it shows moved.
    ///
    /// A poll tick republishes refs and status whether or not they moved,
    /// so most arrivals carry exactly what the section already shows.
    /// Swapping those in would still reset the Qt model — every delegate
    /// rebuilt, the inner list scrolled back — for an identical picture.
    /// The question is asked of the entries themselves, which is what a
    /// section that builds no rows has left to compare: a poll that found
    /// one branch moved is not a reason for the tag section to rebuild
    /// forty-five thousand delegates.
    ///
    /// What arrived is taken either way. It holds what the old one did,
    /// and the session has moved on to it, so keeping the old one would be
    /// a second copy on the heap saying the same thing.
    fn take(&mut self, arrived: Source) -> bool {
        let moved = match (&self.all, &arrived) {
            (Source::Locals(held), Source::Locals(fresh)) => held.locals != fresh.locals,
            (Source::Remotes(held), Source::Remotes(fresh)) => held.remotes != fresh.remotes,
            (Source::Tags(held), Source::Tags(fresh)) => held.tags != fresh.tags,
            (Source::Stashes(held), Source::Stashes(fresh)) => held != fresh,
            (
                Source::Worktrees {
                    list: held,
                    current: was,
                },
                Source::Worktrees {
                    list: fresh,
                    current,
                },
            ) => held != fresh || was != current,
            (Source::Files { status: held, .. }, Source::Files { status: fresh, .. }) => {
                held.items != fresh.items
            }
            _ => true,
        };
        self.all = arrived;
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
                        let head = snapshot.locals.iter().find(|b| b.is_head);
                        self.head_name = head.map(|b| b.short.to_string()).unwrap_or_default();
                        self.head_oid = head.map(|b| b.oid.to_hex()).unwrap_or_default();
                        self.head_has_remote = head.is_some_and(|b| b.has_remote);
                        self.head_has_pr = head.is_some_and(|b| {
                            crate::encode::fake_pr_set().contains(b.short.as_str())
                        });
                        self.take(Source::Locals(snapshot))
                    }
                    "remotes" => self.take(Source::Remotes(snapshot)),
                    _ => self.take(Source::Tags(snapshot)),
                };
            }
        }
        if let Some(feed) = self.status_feed.clone()
            && let Some(StatusMsg {
                status, eol_marks, ..
            }) = feed.drain().pop()
        {
            // The marks are the other half of what a file row shows, and
            // they can move on their own — a line-ending answer arrives
            // after the status it is about.
            let marked = self.eol_marks != eol_marks;
            self.eol_marks = eol_marks;
            arrived |= self.take(Source::files(status)) || marked;
        }
        if let Some(feed) = self.worktrees_feed.clone()
            && let Some(list) = feed.drain().pop()
        {
            // Which working copy this window is showing is the one thing
            // about a worktree row that git's list does not say, so it is
            // asked for once and kept beside the list.
            let current = Hub::with(|hub| hub.session(self.tab_id).and_then(|s| s.workdir()))
                .flatten()
                .map(|p| p.to_string_lossy().replace('\\', "/").to_lowercase())
                .unwrap_or_default();
            // A bare entry has no working copy to show.
            let list = list.into_iter().filter(|w| !w.bare).collect();
            arrived |= self.take(Source::Worktrees { list, current });
        }
        if let Some(feed) = self.stash_feed.clone()
            && let Some(stashes) = feed.drain().pop()
        {
            // The message is the whole of what a stash shows: the reflog
            // selector (stash@{0}) stays out of sight by request, but it
            // is what names the entry to git — renaming one takes the
            // selector, not the message — so `full` answers with it.
            arrived |= self.take(Source::Stashes(stashes));
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

    /// Where a renamed file came from, by path — whole, the way a diff
    /// wants it (a rename's diff is read by naming both of its sides).
    ///
    /// A hand never needs this: it reaches a diff through the row, which
    /// is holding the source already. A headless run has only the path,
    /// and without this it opens the destination alone — which git reads
    /// as a file appearing out of nowhere.
    ///
    /// The first row of that path that names a source, not the first row
    /// of that path: a file renamed and then edited again has a row on
    /// each side, and only the staged one knows where it came from.
    #[qslot]
    fn orig_of(&self, path: String) -> String {
        (0..self.all.len())
            .filter_map(|at| self.all.entry(at))
            .map(|of| Row::Shown {
                of,
                depth: 0,
                from: 0,
            })
            .filter(|row| self.field(*row, Role::Full).as_str() == path)
            .map(|row| self.field(row, Role::OrigPath).as_str().to_string())
            .find(|orig| !orig.is_empty())
            .unwrap_or_default()
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

    fn tracked(staged: char, unstaged: char, path: &str) -> platitude_core::status::StatusItem {
        platitude_core::status::StatusItem::Tracked {
            staged,
            unstaged,
            path: path.to_string(),
            orig_path: None,
        }
    }

    /// One of each kind of pending change, in the order git reports them
    /// rather than the order the pane shows them.
    fn pending() -> platitude_core::status::WorkTreeStatus {
        use platitude_core::status::StatusItem;
        platitude_core::status::WorkTreeStatus {
            items: vec![
                StatusItem::Unmerged {
                    ours: 'U',
                    theirs: 'U',
                    path: "a.txt".to_string(),
                },
                // Both halves changed: one row under each.
                tracked('M', 'M', "src/b.txt"),
                StatusItem::Untracked {
                    path: "c.txt".to_string(),
                },
                StatusItem::Tracked {
                    staged: 'R',
                    unstaged: '.',
                    path: "d.txt".to_string(),
                    orig_path: Some("old.txt".to_string()),
                },
            ],
            ..Default::default()
        }
    }

    /// Which entries a bucket holds is git's answer, and core is where it
    /// is written down — this list asks one entry at a time so a row can
    /// name the bucket it came out of, and the two must not drift.
    #[test]
    fn a_bucket_holds_what_core_says_it_holds() {
        let status = pending();
        for (bucket, theirs) in [
            (Bucket::Conflicts, status.conflicted().collect::<Vec<_>>()),
            (Bucket::Unstaged, status.unstaged().collect()),
            (Bucket::Untracked, status.untracked().collect()),
            (Bucket::Staged, status.staged().collect()),
        ] {
            let ours: Vec<_> = status.items.iter().filter(|i| bucket.holds(i)).collect();
            assert_eq!(
                ours,
                theirs,
                "{} does not hold what core puts in it",
                bucket.routing(),
            );
        }
    }

    /// The pane's own order, and the letters and buckets that go with it.
    /// An entry with both halves changed is two rows.
    #[test]
    fn a_file_row_reads_out_of_the_status() {
        let mut model = section("worktree", Source::files(pending()));
        model.tree_view = false;
        model.arrange();

        assert_eq!(model.shown_rows(), 5, "the MM entry is a row on each side");
        let row = |at| {
            (
                says(&model, at, Role::Name),
                says(&model, at, Role::Full),
                says(&model, at, Role::Change),
                says(&model, at, Role::Bucket),
                says(&model, at, Role::Group),
            )
        };
        // A conflict says what each side did to the file.
        assert_eq!(
            row(0),
            (
                "a.txt".into(),
                "a.txt".into(),
                "UU".into(),
                "conflicts".into(),
                "conflicts".into()
            )
        );
        assert_eq!(
            row(1),
            (
                "src/b.txt".into(),
                "src/b.txt".into(),
                "M".into(),
                "unstaged".into(),
                "unstaged".into()
            )
        );
        // Untracked routes as itself and shows in the unstaged run.
        assert_eq!(
            row(2),
            (
                "c.txt".into(),
                "c.txt".into(),
                "?".into(),
                "untracked".into(),
                "unstaged".into()
            )
        );
        assert_eq!(
            row(3),
            (
                "src/b.txt".into(),
                "src/b.txt".into(),
                "M".into(),
                "staged".into(),
                "staged".into()
            )
        );
        assert_eq!(says(&model, 4, Role::OrigPath), "old.txt");
        // The unstaged half is what a path asked for by name answers with,
        // as it did when the rows were built.
        assert_eq!(model.change_of("src/b.txt".to_string()), "M");
        assert_eq!(model.change_of("a.txt".to_string()), "UU");
    }

    /// The working tree's own tree: one run at a time, folders compacted,
    /// and a file showing only its last segment.
    #[test]
    fn the_file_tree_folds_each_run_of_its_own() {
        let mut model = section("worktree", Source::files(pending()));
        // What `attach_section` starts the working tree's list on.
        model.tree_view = true;
        model.arrange();

        // conflicts: a.txt / unstaged: src, b.txt, c.txt / staged: src,
        // b.txt, d.txt
        assert_eq!(model.shown_rows(), 7);
        assert_eq!(says(&model, 0, Role::Name), "a.txt");
        assert_eq!(says(&model, 1, Role::Name), "src");
        assert!(flags(&model, 1, Role::Folder));
        // A folder's fold key is prefixed with its run, so the same folder
        // under two of them folds apart; the path itself rides along for
        // the hover of an elided row.
        assert_eq!(says(&model, 1, Role::Full), "unstaged:src");
        assert_eq!(says(&model, 1, Role::OrigPath), "src");
        assert_eq!(says(&model, 2, Role::Name), "b.txt");
        assert_eq!(says(&model, 2, Role::Full), "src/b.txt");
        assert_eq!(depth_of(&model, 2), 1);
        assert_eq!(says(&model, 3, Role::Name), "c.txt");
        assert_eq!(says(&model, 4, Role::Full), "staged:src");

        // Folding one run leaves the other alone.
        model
            .folder_overrides
            .insert("unstaged:src".to_string(), false);
        model.arrange();
        assert_eq!(model.shown_rows(), 6);
        assert_eq!(says(&model, 1, Role::Change), FOLDED);
        assert_eq!(says(&model, 2, Role::Name), "c.txt");
    }

    /// A stash shows its message and answers with the selector git knows
    /// it by; a worktree row shows its folder and marks the one this
    /// window has open.
    #[test]
    fn the_short_sections_read_out_of_what_arrived() {
        let mut model = section(
            "stashes",
            Source::Stashes(vec![platitude_core::stash::StashEntry {
                name: "stash@{0}".to_string(),
                oid: oid("c"),
                time: 0,
                message: "On main: a thing".to_string(),
            }]),
        );
        model.arrange();
        assert_eq!(says(&model, 0, Role::Name), "On main: a thing");
        assert_eq!(says(&model, 0, Role::Full), "stash@{0}");
        assert_eq!(says(&model, 0, Role::OidHex), oid("c").to_hex());

        let entry = |path: &str, branch: &str| platitude_core::worktrees::WorktreeEntry {
            path: path.to_string(),
            branch: Some(branch.to_string()),
            head_hex: None,
            bare: false,
            detached: false,
            locked: false,
        };
        let mut model = section(
            "worktrees",
            Source::Worktrees {
                list: vec![
                    entry("C:\\work\\repo", "main"),
                    entry("C:\\work\\other", "topic"),
                ],
                current: "c:/work/other".to_string(),
            },
        );
        model.arrange();
        assert_eq!(says(&model, 0, Role::Name), "repo");
        assert_eq!(says(&model, 0, Role::Full), "C:\\work\\repo");
        assert_eq!(says(&model, 0, Role::Bucket), "main");
        assert!(!flags(&model, 0, Role::IsHead));
        // The one this window is showing is marked, however git spelled it.
        assert!(flags(&model, 1, Role::IsHead));
        assert_eq!(model.head_row, 1);
    }

    /// A snapshot that carries the same tags is not a reason to rebuild
    /// tens of thousands of delegates.
    #[test]
    fn a_republished_snapshot_moves_nothing() {
        let mut model = section("tags", Source::default());
        assert!(model.take(Source::Tags(snapshot(
            Vec::new(),
            vec![tag("v1.0", true, true)]
        ))));
        assert!(!model.take(Source::Tags(snapshot(
            Vec::new(),
            vec![tag("v1.0", true, true)]
        ))));
        assert!(model.take(Source::Tags(snapshot(
            Vec::new(),
            vec![tag("v1.1", true, true)]
        ))));
    }
}
