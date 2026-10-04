//! The working copies as the operation panel's WORKTREE card offers
//! them: every one this section holds, **whatever the section is
//! filtering to** — as on the branches' card (`card`).

use super::*;

/// One row of the card: a working copy, said the way the section's own
/// row says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyRow {
    /// The copy's folder — what the section's row says.
    pub name: String,
    /// Where it is, as git lists it — what the row is pressed for.
    pub full: String,
    /// The branch it has out; empty where it is detached.
    pub branch: String,
    /// The state of the checkout, in the section's own word for it —
    /// `MAIN`, `LOCKED`, `PRUNABLE`, or empty (`item::MAIN`).
    pub change: String,
    /// Why it is in that state, where git said: a lock's reason, or why it
    /// would be pruned (the section's `orig_path`).
    pub reason: String,
    /// The commit it stands on, as hex — what tells two copies of one
    /// folder name apart on the graph (`copy_standing`).
    pub head: String,
}

/// The card, in the order the section draws it.
pub type CopyRows = Listed<CopyRow>;

impl Record for CopyRow {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        Fields::new()
            .put("name", &self.name)
            .put("full", &self.full)
            .put("branch", &self.branch)
            .put("change", &self.change)
            .put("reason", &self.reason)
            .put("head", &self.head)
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            name: field(map, "name")?,
            full: field(map, "full")?,
            branch: field(map, "branch")?,
            change: field(map, "change")?,
            reason: field(map, "reason")?,
            head: field(map, "head")?,
        })
    }
}

/// Where a new working copy for a branch would go, and whether something
/// is already there — what the two rows that make one stand on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewCopy {
    /// git's spelling of the folder (`platitude_core::worktrees::new_copy_path`).
    pub path: String,
    /// Its last segment — the copy's name on screen once it is made.
    pub name: String,
    /// Why git would not make it there: `listed` (git still lists a copy at
    /// that path — its folder may be gone), `folder` (something is in the
    /// folder), or empty where nothing is.
    pub taken: String,
}

impl Record for NewCopy {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        Fields::new()
            .put("path", &self.path)
            .put("name", &self.name)
            .put("taken", &self.taken)
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            path: field(map, "path")?,
            name: field(map, "name")?,
            taken: field(map, "taken")?,
        })
    }
}

impl NavSectionModel {
    /// Where a new copy for `branch` goes, beside the repository's own copy
    /// (`MAIN` row). `None` before the listing names that copy, and for an
    /// empty name.
    ///
    /// The folder is looked at on disk here, once per ask: the box asks per
    /// keystroke and the menu as it opens, which is when the answer is read.
    pub(super) fn new_copy(&self, branch: &str) -> Option<NewCopy> {
        let rows = self.copy_rows();
        let main = rows.iter().find(|row| row.change == MAIN)?;
        let path = platitude_core::worktrees::new_copy_path(&main.full, branch);
        if path.is_empty() {
            return None;
        }
        let key = platitude_core::session::same_path_key(&path);
        let taken = if rows
            .iter()
            .any(|row| platitude_core::session::same_path_key(&row.full) == key)
        {
            "listed"
        } else if !platitude_core::worktrees::folder_free(std::path::Path::new(&path)) {
            "folder"
        } else {
            ""
        };
        Some(NewCopy {
            name: crate::urlpath::path_leaf(&path).to_string(),
            taken: taken.to_string(),
            path,
        })
    }

    /// Whether new copies have a place at all: the listing names the
    /// repository's own copy, and that copy has a parent to go beside it in
    /// (`copies_folder`). Nothing on disk is looked at.
    pub(super) fn has_place_for_copies(&self) -> bool {
        self.copy_rows().iter().any(|row| {
            row.change == MAIN && platitude_core::worktrees::copies_folder(&row.full).is_some()
        })
    }

    /// The card's rows. Empty for every section but the working copies.
    ///
    /// No filter, and no row the section is showing as gone — the copies
    /// `total` counts — each said by the section's own `field`.
    pub(super) fn copy_rows(&self) -> Vec<CopyRow> {
        if self.section != "worktrees" {
            return Vec::new();
        }
        (0..self.all.len())
            .filter(|at| !self.hidden_at(*at))
            .filter_map(|at| self.all.entry(at))
            .map(|of| {
                let row = Row::Shown {
                    of,
                    depth: 0,
                    from: 0,
                };
                CopyRow {
                    name: self.field(row, Role::Name).as_str().to_string(),
                    full: self.field(row, Role::Full).as_str().to_string(),
                    branch: self.field(row, Role::Bucket).as_str().to_string(),
                    change: self.field(row, Role::Change).as_str().to_string(),
                    reason: self.field(row, Role::OrigPath).as_str().to_string(),
                    head: self.field(row, Role::OidHex).as_str().to_string(),
                }
            })
            .collect()
    }

    /// The copy at `path`, as its row says it — what a WORKTREE card stands
    /// on, whichever entrance raised it. `None` for a path this listing
    /// does not hold, or holds as gone.
    pub(super) fn copy_of(&self, path: &str) -> Option<CopyRow> {
        self.copy_rows().into_iter().find(|row| row.full == path)
    }

    /// The path of the copy called `name` standing on `head` — what a
    /// folder's chip on the graph names, which carries only the folder:
    /// two copies of one name under different parents sit on different
    /// commits, or are the one chip. Empty where none does.
    pub(super) fn copy_standing(&self, name: &str, head: &str) -> String {
        self.copy_rows()
            .into_iter()
            .find(|row| row.name == name && row.head == head)
            .map(|row| row.full)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::testkit::*;
    use super::*;

    fn copy(
        path: &str,
        branch: Option<&str>,
        locked: bool,
    ) -> platitude_core::worktrees::WorktreeEntry {
        // One commit per copy, told apart by the folder.
        let head = match path.rsplit('/').next() {
            Some("home") => "1111",
            Some("plain") => "2222",
            _ => "3333",
        };
        platitude_core::worktrees::WorktreeEntry {
            path: path.to_string(),
            branch: branch.map(str::to_string),
            head_hex: Some(head.to_string()),
            bare: false,
            detached: branch.is_none(),
            locked,
            lock_reason: if locked {
                "kept".to_string()
            } else {
                String::new()
            },
            prunable: false,
            prune_reason: String::new(),
            main: path.ends_with("home"),
        }
    }

    fn copies() -> NavSectionModel {
        section(
            "worktrees",
            Source::Worktrees {
                list: vec![
                    copy("C:/work/home", Some("main"), false),
                    copy("C:/work/plain", Some("topic"), false),
                    copy("C:/work/held", None, true),
                ],
                current: "c:/work/home".to_string(),
            },
        )
    }

    fn fulls(model: &NavSectionModel) -> Vec<String> {
        model.copy_rows().into_iter().map(|row| row.full).collect()
    }

    #[test]
    fn the_card_ignores_what_the_section_is_filtering_to() {
        let mut model = copies();
        model.arrange();
        let whole = fulls(&model);
        assert_eq!(whole.len(), 3);

        model.filter = "plain".to_string();
        model.arrange();
        assert_eq!(model.shown_rows(), 1, "the filter left one row on the list");
        assert_eq!(fulls(&model), whole);
    }

    #[test]
    fn a_copy_shown_as_gone_is_off_the_card() {
        let mut model = copies();
        model.hidden = vec!["C:/work/plain".to_string()];
        model.arrange();
        assert_eq!(model.total, 2);
        assert_eq!(fulls(&model), vec!["C:/work/home", "C:/work/held"]);
    }

    #[test]
    fn a_copy_row_says_what_the_sections_row_draws() {
        let rows = copies().copy_rows();
        assert_eq!(
            rows[0],
            CopyRow {
                name: "home".to_string(),
                full: "C:/work/home".to_string(),
                branch: "main".to_string(),
                change: "MAIN".to_string(),
                reason: String::new(),
                head: "1111".to_string(),
            }
        );
        assert_eq!(
            (rows[1].branch.as_str(), rows[1].change.as_str()),
            ("topic", "")
        );
        assert_eq!(
            (rows[2].branch.as_str(), rows[2].change.as_str()),
            ("", "LOCKED")
        );
    }

    /// What a WORKTREE card stands on, by path from a row and by folder and
    /// commit from a chip; a copy shown as gone has no card.
    #[test]
    fn a_copy_is_found_by_its_path_or_by_its_folder_where_it_stands() {
        let mut model = copies();
        let held = model.copy_of("C:/work/held").expect("listed");
        assert_eq!(
            (held.change.as_str(), held.reason.as_str()),
            ("LOCKED", "kept")
        );
        assert_eq!(model.copy_standing("held", "3333"), "C:/work/held");
        assert_eq!(
            model.copy_standing("held", "1111"),
            "",
            "not standing there"
        );
        assert!(model.copy_of("C:/work/elsewhere").is_none());

        model.hidden = vec!["C:/work/held".to_string()];
        model.arrange();
        assert!(
            model.copy_of("C:/work/held").is_none(),
            "going: no card for it"
        );
    }

    /// A new copy goes beside the repository's own, under its name — the
    /// tab may stand in another copy, and the place does not move with it.
    #[test]
    fn a_new_copy_is_placed_off_the_repositorys_own_copy() {
        let place = copies().new_copy("feature/login").expect("a place");
        assert_eq!(place.path, "C:/work/home.worktrees/feature-login");
        assert_eq!(place.name, "feature-login");
        assert_eq!(place.taken, "");
    }

    #[test]
    fn a_place_git_still_lists_is_taken() {
        let model = section(
            "worktrees",
            Source::Worktrees {
                list: vec![
                    copy("C:/work/home", Some("main"), false),
                    copy("C:/work/home.worktrees/topic", Some("topic"), false),
                    copy("C:/work/home.worktrees/Fix-Case", Some("fix/case"), false),
                ],
                current: "c:/work/home".to_string(),
            },
        );
        assert_eq!(model.new_copy("topic").expect("a place").taken, "listed");
        // One folder to a disk that does not tell case apart (Windows,
        // macOS), where git turns the place down as well.
        assert_eq!(model.new_copy("fix/case").expect("a place").taken, "listed");
    }

    /// Looked at on disk: something in the folder takes it, an empty
    /// folder does not (git fills one).
    #[test]
    fn a_folder_holding_something_is_taken() {
        let dir = std::env::temp_dir().join(format!("pgg-new-copy-{}", std::process::id()));
        let home = dir.join("home").to_string_lossy().replace('\\', "/");
        let model = section(
            "worktrees",
            Source::Worktrees {
                list: vec![copy(&home, Some("main"), false)],
                current: home.clone(),
            },
        );
        let place = dir.join("home.worktrees").join("busy");
        std::fs::create_dir_all(&place).unwrap();
        let empty = model.new_copy("busy").expect("a place").taken;
        std::fs::write(place.join("x.txt"), "x").unwrap();
        let holding = model.new_copy("busy").expect("a place").taken;
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(empty, "");
        assert_eq!(holding, "folder");
    }

    #[test]
    fn no_place_before_the_listing_names_the_repositorys_own_copy() {
        let model = section(
            "worktrees",
            Source::Worktrees {
                list: vec![copy("C:/work/plain", Some("topic"), false)],
                current: "c:/work/plain".to_string(),
            },
        );
        assert!(model.new_copy("x").is_none());
        assert!(!model.has_place_for_copies());
        assert!(copies().new_copy("").is_none());
        assert!(copies().has_place_for_copies());
    }

    #[test]
    fn only_the_working_copies_have_a_copy_card() {
        let model = section(
            "branches",
            Source::Locals(locals(vec![local("main", true)])),
        );
        assert!(model.copy_rows().is_empty());
    }
}
