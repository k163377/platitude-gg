//! What a ref's menu reads off the sections beside its own row: the row and
//! where it leads among the worktrees (`RowLead`), and the record its card
//! stood on handed back whole (`CardRecord`). The rules that read them are
//! core's (`offers`), asked through `GitFacts.refMenu` / `switchAction`.

use platitude_core::offers::{FarSide, Leads};
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::{BranchUpstream, NavSectionModel, Source, TagMenu};
use crate::encode::{Fields, Record, field};

/// A ref's row as its menu asks core about it: the row's two words and the
/// local branch it lands on, where it leads beside the tree the tab stands
/// in, the worktree its WORKTREE card stands on, what the worktrees say of
/// its branch, and whether new worktrees have a place at all — one answer
/// (`NavSectionModel.leadOf`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RowLead {
    /// The row's kind word (`offers::RefKind::from_word`), and its name as
    /// git knows it; empty on a commit row that draws no name.
    pub kind: String,
    pub full: String,
    /// The local branch the row lands on: a remote row's of its name, a
    /// branch row's own; empty on any other row.
    pub local: String,
    pub leads: Leads,
    /// The worktree the row leads to, as git lists it; empty where it
    /// leads nowhere.
    pub path: String,
    /// The name that worktree goes by (`name_of_worktree`) — its WORKTREES
    /// row's.
    pub called: String,
    /// That worktree is the repository's own: its marks are the house.
    pub home: bool,
    /// The worktree the row's WORKTREE card stands on, as git lists it: a
    /// worktree row's own — the tab's too —, or the one holding a local
    /// branch's; empty for a remote row, which only leads there (デザイン規約
    /// §メニュー の入れ子), and for any other.
    pub worktree: String,
    /// `rebase` / `bisect` where an operation stopped in the tree the tab
    /// stands in holds the row's branch; empty otherwise.
    pub stopped_by: String,
    /// New worktrees have a place to go (`has_place_for_worktrees`).
    pub placed: bool,
}

impl Record for RowLead {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("kind", &self.kind)
            .put("full", &self.full)
            .put("local", &self.local)
            .put("leads", self.leads.word())
            .put("path", &self.path)
            .put("called", &self.called)
            .put("home", &self.home)
            .put("worktree", &self.worktree)
            .put("stoppedBy", &self.stopped_by)
            .put("placed", &self.placed)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        Some(Self {
            kind: field(map, "kind")?,
            full: field(map, "full")?,
            local: field(map, "local")?,
            leads: Leads::from_word(&field::<String>(map, "leads")?)?,
            path: field(map, "path")?,
            called: field(map, "called")?,
            home: field(map, "home")?,
            worktree: field(map, "worktree")?,
            stopped_by: field(map, "stoppedBy")?,
            placed: field(map, "placed")?,
        })
    }
}

/// The record a ref's card stood on, handed back as the section built it
/// (`GitFacts.refMenu`): a branch's upstream (`NavSectionModel.branchUpstream`
/// — nothing over there on any row but a local branch's) or a tag's menu
/// (`tagMenu`). Told apart by the far side it carries (`encode::put_far`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardRecord {
    Branch(BranchUpstream),
    Tag(TagMenu),
}

impl CardRecord {
    /// What the card's rows read over there.
    pub fn far(&self) -> FarSide {
        match self {
            Self::Branch(upstream) => upstream.far.map_or(FarSide::Unread, FarSide::Branch),
            Self::Tag(menu) => FarSide::Tag(menu.far),
        }
    }
}

impl Record for CardRecord {
    fn to_map(&self) -> QVariantMap {
        match self {
            Self::Branch(upstream) => upstream.to_map(),
            Self::Tag(menu) => menu.to_map(),
        }
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        match field::<String>(map, "far")?.as_str() {
            "tag" => TagMenu::from_map(map).map(Self::Tag),
            _ => BranchUpstream::from_map(map).map(Self::Branch),
        }
    }
}

impl NavSectionModel {
    /// Where the row `full` of `kind` leads (contract on the `leadOf` slot):
    /// a branch's or a remote branch's holder, by `local` — the local
    /// branch the row lands on —, and a worktree on no branch itself while
    /// the tab does not stand in it. Of a local branch, also the operation
    /// stopped here that holds it. Asked of any other section, the row
    /// leads nowhere.
    pub(super) fn row_lead(&self, kind: &str, full: &str, local: &str) -> RowLead {
        let local = if matches!(kind, "branch" | "remote") {
            local
        } else {
            ""
        };
        let row = RowLead {
            kind: kind.to_string(),
            full: full.to_string(),
            local: local.to_string(),
            ..RowLead::default()
        };
        let Source::Worktrees { current, .. } = &self.all else {
            return row;
        };
        let (leads, path) = match kind {
            "branch" | "remote" => {
                let holder = self.worktree_with(local);
                let leads = if holder.is_empty() {
                    Leads::Nowhere
                } else {
                    Leads::Holder
                };
                (leads, holder)
            }
            "worktree" if platitude_core::session::same_path_key(full) != *current => {
                (Leads::Itself, full.to_string())
            }
            _ => (Leads::Nowhere, String::new()),
        };
        let stopped_by = if kind == "branch" {
            self.held_by_operation_of(local)
                .filter(|held| held.here)
                .map(|held| held.by)
                .unwrap_or_default()
        } else {
            String::new()
        };
        let worktree = match kind {
            "worktree" => full.to_string(),
            "branch" => path.clone(),
            _ => String::new(),
        };
        RowLead {
            leads,
            worktree,
            called: if path.is_empty() {
                String::new()
            } else {
                self.name_of_worktree(&path)
            },
            home: self.is_home_worktree(&path),
            path,
            stopped_by,
            placed: self.has_place_for_worktrees(),
            ..row
        }
    }
}
