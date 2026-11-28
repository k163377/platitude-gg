//! What a ref's menu reads off the sections beside its own row: the row and
//! where it leads among the worktrees (`RowLead`), and the record its card
//! stood on handed back whole (`CardRecord`). What reads them is core's —
//! the menu type and the move's rule —, asked through `GitFacts.menuOn` /
//! `switchMove`.

use platitude_core::menu::Placing;
use platitude_core::offers::{FarSide, Leads};
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::{BranchUpstream, NavSectionModel, NewWorktree, Source, TagMenu, WorktreeRow};
use crate::encode::{Fields, Record, field, optional_record};

/// A ref's row as its menu asks core about it: the row's two words and the
/// local branch it lands on, where it leads beside the tree the tab stands
/// in, the worktree its WORKTREE card stands on and whether the tab stands
/// in that one, and where new worktrees go — one answer
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
    /// The worktree the row's WORKTREE card stands on, as its row says it:
    /// the one the menu was opened on, else a worktree row's own — the
    /// tab's too —, or the one holding a local branch's; none for a remote
    /// row, which only leads there (デザイン規約 §メニュー の入れ子), for any
    /// other, and for a worktree not listed or listed as going.
    pub card: Option<WorktreeRow>,
    /// The tab stands in that worktree.
    pub card_here: bool,
    /// Where new worktrees go — `Unread` until the worktree listing has
    /// arrived, when what the rest says of worktrees is read off it and
    /// every row leads nowhere —, the row's own branch's folder being
    /// taken or not (`worktrees_nowhere`, `place`).
    pub placing: Placing,
    /// Where the row's own branch would go out in a worktree of its own
    /// (`new_worktree`); none with no local branch, or nowhere to go.
    pub place: Option<NewWorktree>,
}

impl Record for RowLead {
    fn to_map(&self) -> QVariantMap {
        // A card and a place are left out where there is none, which QML
        // reads as `undefined`.
        Fields::new()
            .put("kind", &self.kind)
            .put("full", &self.full)
            .put("local", &self.local)
            .put("leads", self.leads.word())
            .put("path", &self.path)
            .put("called", &self.called)
            .put("home", &self.home)
            .put("cardHere", &self.card_here)
            .put("placing", self.placing.word())
            .put_record("card", self.card.as_ref())
            .put_record("place", self.place.as_ref())
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
            card: optional_record(map, "card")?,
            card_here: field(map, "cardHere")?,
            placing: Placing::from_word(&field::<String>(map, "placing")?)?,
            place: optional_record(map, "place")?,
        })
    }
}

/// The record a ref's card stood on, handed back as the section built it
/// (`GitFacts.menuOn`, picked by the row's kind in `card_record`): a branch's upstream
/// — nothing over there on any row but a local branch's — or a tag's menu
/// (`tag_card`). Told apart by the far side it carries (`encode::put_far`).
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
            Self::Tag(menu) => FarSide::Tag(menu.far.clone()),
        }
    }
}

impl NavSectionModel {
    /// The record the card of the row `name` of `kind` stands on — the one
    /// place a row's kind picks it (`card_of`): a local branch's upstream
    /// (`RefsSnapshot::branch_upstream`); the same record empty on a remote
    /// branch's row, a reading with no upstream of its own — and its name
    /// may be a local branch's as well; a tag's menu (`tag_card`); none on
    /// a row with no card.
    pub(super) fn card_record(
        &self,
        kind: &str,
        name: &str,
        against: String,
        aim: &str,
    ) -> Option<CardRecord> {
        use platitude_core::offers::RefKind;
        match RefKind::from_word(kind)? {
            RefKind::Branch => Some(CardRecord::Branch(self.all.branch_upstream(name))),
            RefKind::Remote => Some(CardRecord::Branch(BranchUpstream::default())),
            RefKind::Tag => Some(CardRecord::Tag(self.tag_card(name, against, aim))),
            RefKind::Stash | RefKind::Worktree => None,
        }
    }

    /// What a menu on tag `name` stands on (`RefTagMenu`): where the push
    /// goes and what it is leased to, which remote the deletes reach and
    /// why they stand greyed, and whether the row goes with them
    /// (`RefsSnapshot::tag_menu`), and the sides the name stands on. With
    /// no name, or before the refs are read, it reaches nothing beyond
    /// `against`.
    pub(super) fn tag_card(&self, name: &str, against: String, aim: &str) -> TagMenu {
        let found = if name.is_empty() {
            None
        } else {
            self.all.tag_menu(name, &against, aim)
        };
        found.unwrap_or_else(|| {
            platitude_core::session::TagMenuFacts {
                far: platitude_core::offers::TagFar {
                    push_remote: against,
                    ..Default::default()
                },
                ..Default::default()
            }
            .into()
        })
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
    /// the tab does not stand in it — a branch an operation stopped here
    /// keeps is HEAD's reading's to say (`TreeStanding::kept`). `opened` is
    /// the worktree the menu was
    /// opened on (a WORKTREES row's path), which the WORKTREE card stands on
    /// over the row's own; empty for none. Asked of any other section, the
    /// row leads nowhere.
    pub(super) fn row_lead(&self, kind: &str, full: &str, local: &str, opened: &str) -> RowLead {
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
        // A branch out in this window's own worktree stands its card there,
        // as that worktree's row does: the same branch, the same rows.
        let here = if kind == "branch" && path.is_empty() {
            self.worktree_here_with(local)
        } else {
            String::new()
        };
        let worktree = if !opened.is_empty() {
            opened
        } else {
            match kind {
                "worktree" => full,
                "branch" if path.is_empty() => here.as_str(),
                "branch" => path.as_str(),
                _ => "",
            }
        };
        let card = self.worktree_of(worktree);
        let card_here = card
            .as_ref()
            .is_some_and(|card| platitude_core::session::same_path_key(&card.full) == *current);
        let place = if local.is_empty() {
            None
        } else {
            self.new_worktree(local)
        };
        let named = if path.is_empty() {
            super::WorktreeName::default()
        } else {
            self.called_worktree(&path)
        };
        let placing = match self.worktrees_nowhere() {
            Some(nowhere) => Placing::Nowhere(nowhere),
            None => Placing::Beside {
                taken: place.as_ref().is_some_and(|place| !place.taken.is_empty()),
            },
        };
        RowLead {
            leads,
            card,
            card_here,
            place,
            called: named.called,
            home: named.home,
            path,
            placing,
            ..row
        }
    }
}
