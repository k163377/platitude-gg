//! The branches as the operation panel's card offers them: every one this
//! section holds, filed the way the section files them — **whatever the
//! section is folding or filtering to**. The card is a way to move, not
//! the list somebody is reading, so a branch the reader folded away or
//! filtered out of the left menu is still somewhere to go
//! (デザイン規約 §操作パネル).

use super::*;

/// One row of the card: a folder, which the card opens as a card of its
/// own, or a branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardRow {
    /// What the row says — the segment under its folders, the way the
    /// section's own row says it.
    pub name: String,
    /// What git knows it by: a folder's path, a branch's whole name.
    pub full: String,
    /// How many folders it is filed under — which card of the nest it
    /// goes on.
    pub depth: i32,
    pub folder: bool,
    /// The branch HEAD is on. **On the list and not on offer**: the card
    /// offers what can be chosen, and a folder holding nothing else takes
    /// its own row with it.
    pub head: bool,
    /// Whether choosing from the row gets anywhere: a branch that is not
    /// the one HEAD is on, a folder with such a branch filed anywhere
    /// under it. **Said of a folder before its card is made** — the card
    /// makes a folder's rows as it opens (`OpsBranchMenu`), and a folder
    /// with nothing to offer takes its row away before anyone opens it.
    pub offers: bool,
    /// Another working copy has it out (`item::HELD`).
    pub held: bool,
    pub ahead: i32,
    pub behind: i32,
    /// The badge the section's row ends with (`NavRowBody.remoteSeat`):
    /// a remote carries it, a pull request is open for it, and the
    /// upstream it is measured against has gone.
    pub remote: bool,
    pub pr: bool,
    pub gone: bool,
}

/// The card, in the order the section draws it: a folder before what is
/// filed under it, each at its depth.
pub type CardRows = Listed<CardRow>;

impl Record for CardRow {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        Fields::new()
            .put("name", &self.name)
            .put("full", &self.full)
            .put("depth", &self.depth)
            .put("folder", &self.folder)
            .put("head", &self.head)
            .put("offers", &self.offers)
            .put("held", &self.held)
            .put("ahead", &self.ahead)
            .put("behind", &self.behind)
            .put("remote", &self.remote)
            .put("pr", &self.pr)
            .put("gone", &self.gone)
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            name: field(map, "name")?,
            full: field(map, "full")?,
            depth: field(map, "depth")?,
            folder: field(map, "folder")?,
            head: field(map, "head")?,
            offers: field(map, "offers")?,
            held: field(map, "held")?,
            ahead: field(map, "ahead")?,
            behind: field(map, "behind")?,
            remote: field(map, "remote")?,
            pr: field(map, "pr")?,
            gone: field(map, "gone")?,
        })
    }
}

impl NavSectionModel {
    /// The card's rows. Empty for every section but the branches.
    ///
    /// **Every folder open**, and no filter: the tree is the section's own
    /// (`build_tree_with`), so a name is filed under the same folders on
    /// both sides of the window. What each row says is the section's own
    /// answer too (`field`) — the card cannot tell the left menu's row
    /// one thing and draw another.
    pub(super) fn card(&self) -> Vec<CardRow> {
        if self.section != "branches" {
            return Vec::new();
        }
        let tree = self.build_tree_with(|_, _| true);
        let mut rows: Vec<CardRow> = tree
            .iter()
            .filter_map(|arranged| self.read_arranged(arranged))
            .map(|row| {
                // The whole name off the row itself: the section answers
                // a branch's `full` only while a tree named it, and a
                // filter turns that off for its own list.
                let full = match row {
                    Row::Made(item) => item.full.clone(),
                    Row::Shown { of, .. } => of.name().to_string(),
                };
                let folder = self.field(row, Role::Folder).flag();
                let head = self.field(row, Role::IsHead).flag();
                CardRow {
                    name: self.field(row, Role::Name).as_str().to_string(),
                    full,
                    depth: self.field(row, Role::Depth).number(),
                    folder,
                    head,
                    offers: !folder && !head,
                    held: self.field(row, Role::Change).as_str() == HELD,
                    ahead: self.field(row, Role::Ahead).number(),
                    behind: self.field(row, Role::Behind).number(),
                    remote: self.field(row, Role::HasRemote).flag(),
                    pr: self.field(row, Role::HasPr).flag(),
                    gone: !self.field(row, Role::Bucket).as_str().is_empty(),
                }
            })
            .collect();
        // A folder offers what any branch filed under it offers: every
        // folder on the way down to a branch on offer is one that does.
        let offering: std::collections::HashSet<String> = rows
            .iter()
            .filter(|row| row.offers)
            .flat_map(|row| {
                let mut above = Vec::new();
                let mut at = row.full.as_str();
                while let Some((parent, _)) = at.rsplit_once('/') {
                    above.push(parent.to_string());
                    at = parent;
                }
                above
            })
            .collect();
        for row in rows.iter_mut().filter(|row| row.folder) {
            row.offers = offering.contains(&row.full);
        }
        rows
    }

    /// One card of the nest: the rows filed directly under the folder at
    /// `path`, or the top of it for an empty one. **One level at a time**
    /// — the card makes an item for every row it is handed, so a whole tree
    /// made on the press costs the press an item per local branch
    /// (ci/baseline/code-costs-windows-x64.md), and a folder's rows are
    /// made as its card opens (`OpsBranchMenu`).
    pub(super) fn level_rows(&self, path: &str) -> Vec<CardRow> {
        self.card()
            .into_iter()
            .filter(|row| row.full.rsplit_once('/').map_or("", |(parent, _)| parent) == path)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::testkit::*;
    use super::*;

    fn branches() -> NavSectionModel {
        let mut held = local("worktree-a", false);
        held.held_elsewhere = true;
        let mut tracked = local("feature/tracked", false);
        tracked.has_remote = true;
        tracked.ahead = 1;
        tracked.behind = 2;
        let mut gone = local("feature/zz-gone", false);
        gone.upstream_gone = "origin/feature/zz-gone".into();
        let mut model = section(
            "branches",
            Source::Locals(locals(vec![
                tracked,
                gone,
                local("main", true),
                local("topic/deep/one", false),
                local("topic/two", false),
                held,
            ])),
        );
        model.head_name = "main".to_string();
        model
    }

    fn names(model: &NavSectionModel) -> Vec<(String, i32, bool)> {
        model
            .card()
            .into_iter()
            .map(|row| (row.full, row.depth, row.folder))
            .collect()
    }

    /// The card files every branch under its folders, a folder before
    /// what it holds — the section's own tree, one row per `/`.
    #[test]
    fn the_card_is_the_sections_tree_with_every_folder_open() {
        let model = branches();
        assert_eq!(
            names(&model),
            vec![
                ("feature".to_string(), 0, true),
                ("feature/tracked".to_string(), 1, false),
                ("feature/zz-gone".to_string(), 1, false),
                ("main".to_string(), 0, false),
                ("topic".to_string(), 0, true),
                ("topic/deep".to_string(), 1, true),
                ("topic/deep/one".to_string(), 2, false),
                ("topic/two".to_string(), 1, false),
                ("worktree-a".to_string(), 0, false),
            ]
        );
        let rows = model.card();
        assert_eq!(
            rows[1].name, "tracked",
            "a row says the segment under its folders"
        );
        assert_eq!(rows[5].name, "deep");
    }

    /// **The left menu's folds and filter are the reader's, not the
    /// card's**: a branch folded away or filtered out is still offered.
    #[test]
    fn the_card_ignores_what_the_section_is_folding_or_filtering_to() {
        let mut model = branches();
        model.arrange();
        let whole = names(&model);

        model.folder_overrides.insert("topic".to_string(), false);
        model.arrange();
        assert!(
            model.shown_rows() < whole.len(),
            "the fold took rows off the list"
        );
        assert_eq!(names(&model), whole);

        model.filter = "main".to_string();
        model.arrange();
        assert_eq!(model.shown_rows(), 1, "the filter left one row on the list");
        assert_eq!(names(&model), whole);
    }

    /// Each row carries what the section's own row draws — the current
    /// branch, the copy holding one, the counts and the badge.
    #[test]
    fn a_card_row_says_what_the_sections_row_draws() {
        let rows = branches().card();
        let row = |full: &str| {
            rows.iter()
                .find(|row| row.full == full)
                .cloned()
                .unwrap_or_else(|| panic!("no row for {full}"))
        };
        assert!(row("main").head);
        assert!(!row("topic/two").head);
        assert!(row("worktree-a").held);
        assert!(!row("main").held);
        let tracked = row("feature/tracked");
        assert_eq!(
            (tracked.ahead, tracked.behind, tracked.remote),
            (1, 2, true)
        );
        assert!(row("feature/zz-gone").gone);
        assert!(!tracked.gone);
    }

    /// A card of the nest is the rows filed directly under one folder,
    /// the top of it for the empty path — nothing deeper, nothing beside.
    #[test]
    fn a_level_is_what_is_filed_directly_under_the_folder() {
        let model = branches();
        let level = |path: &str| -> Vec<String> {
            model
                .level_rows(path)
                .into_iter()
                .map(|row| row.full)
                .collect()
        };
        assert_eq!(level(""), vec!["feature", "main", "topic", "worktree-a"]);
        assert_eq!(level("topic"), vec!["topic/deep", "topic/two"]);
        assert_eq!(level("topic/deep"), vec!["topic/deep/one"]);
        assert!(level("nowhere").is_empty());
    }

    /// A folder offers what the branches filed anywhere under it offer —
    /// said before its card is made, so a folder holding only the current
    /// branch is taken off the card that holds it.
    #[test]
    fn a_folder_offers_what_the_branches_under_it_offer() {
        let mut model = section(
            "branches",
            Source::Locals(locals(vec![
                local("only/here", true),
                local("topic/deep/one", false),
                local("rig", false),
            ])),
        );
        model.head_name = "only/here".to_string();
        let row = |full: &str| {
            model
                .card()
                .into_iter()
                .find(|row| row.full == full)
                .unwrap_or_else(|| panic!("no row for {full}"))
        };
        assert!(
            !row("only").offers,
            "a folder holding only the current branch"
        );
        assert!(!row("only/here").offers);
        assert!(
            row("topic").offers,
            "a branch two folders down reaches the top"
        );
        assert!(row("topic/deep").offers);
        assert!(row("rig").offers);
    }

    /// Only the branches have a card: every other section answers with
    /// nothing rather than with rows nobody asked for.
    #[test]
    fn only_the_branches_have_a_card() {
        let model = section(
            "remotes",
            Source::Remotes(named(vec![remote("origin/main")], &["origin"])),
        );
        assert!(model.card().is_empty());
    }
}
