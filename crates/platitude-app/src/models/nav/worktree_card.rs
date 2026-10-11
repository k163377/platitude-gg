//! The worktrees as the operation panel's WORKTREE card offers
//! them: every one this section holds, **whatever the section is
//! filtering to** — as on the branches' card (`card`).

use super::*;

/// One row of the card: a worktree, said the way the section's own
/// row says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeRow {
    /// The worktree's folder — what the section's row says.
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
    /// The commit it stands on, as hex — what tells two worktrees of one
    /// folder name apart on the graph (`worktree_standing`).
    pub head: String,
}

/// The card, in the order the section draws it.
pub type WorktreeRows = Listed<WorktreeRow>;

impl Record for WorktreeRow {
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

/// A folder a worktree would stand in, and what is in its way — where a new
/// worktree for a branch would go (`new_worktree`, what the two rows that
/// make one stand on) or where one renamed would (`renamed_worktree`, what
/// its name box stands on).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewWorktree {
    /// git's spelling of the folder (`platitude_core::worktrees::new_worktree_path`
    /// / `renamed_worktree_path`).
    pub path: String,
    /// Its last segment — the worktree's name on screen once it stands there.
    pub name: String,
    /// What is in the way, empty for nothing. Both: `listed` (git still lists
    /// a worktree at that path — its folder may be gone) and `folder` —
    /// something in the folder for a new one (git makes a worktree in an empty
    /// folder), anything there at all for a rename (git would move into it). A
    /// rename's only: `case` (the same folder to a disk that ignores case) and
    /// what the system keeps out of a name — `character`, `device`, `trailing`.
    pub taken: String,
    /// What the system keeps out of the name, where `taken` says so: the
    /// character, the device's name, or the last letter. Empty otherwise.
    pub kept: String,
}

impl Record for NewWorktree {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        Fields::new()
            .put("path", &self.path)
            .put("name", &self.name)
            .put("taken", &self.taken)
            .put("kept", &self.kept)
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            path: field(map, "path")?,
            name: field(map, "name")?,
            taken: field(map, "taken")?,
            kept: field(map, "kept")?,
        })
    }
}

impl NavSectionModel {
    /// Where a new worktree for `branch` goes, beside the repository's own
    /// worktree (`MAIN` row). `None` before the listing names that worktree,
    /// and for an empty name.
    ///
    /// The folder is looked at on disk here, once per ask: the box asks per
    /// keystroke and the menu as it opens, which is when the answer is read.
    pub(super) fn new_worktree(&self, branch: &str) -> Option<NewWorktree> {
        let rows = self.worktree_rows();
        let main = rows.iter().find(|row| row.change == MAIN)?;
        let path = platitude_core::worktrees::new_worktree_path(&main.full, branch);
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
        Some(NewWorktree {
            name: crate::urlpath::path_leaf(&path).to_string(),
            taken: taken.to_string(),
            path,
            kept: String::new(),
        })
    }

    /// Where the worktree at `from` goes renamed `name` — beside where it is
    /// (`renamed_worktree_path`) — and what is in the way there, said in
    /// `taken`: what this system keeps out of the name (`character`, `device`,
    /// `trailing` — `Disk::refuses`, with `kept` the part it keeps), `case`
    /// (only the letters' case differs, and the disk calls that the same
    /// folder), `listed` (git lists another worktree there), `folder` (anything
    /// at all is there: git would move the worktree into it), or empty — the
    /// folder's own name too, which is no rename. What a worktree's name box
    /// stands on, asked per keystroke. `None` for a name that is not one
    /// folder's, and for a path with no parent.
    pub(super) fn renamed_worktree(&self, from: &str, name: &str) -> Option<NewWorktree> {
        use platitude_core::session::same_path_key;
        use platitude_core::worktrees::{
            Disk, DiskRefusal, is_folder_name, path_free, renamed_worktree_path,
        };
        if !is_folder_name(name) {
            return None;
        }
        let path = renamed_worktree_path(from, name);
        if path.is_empty() {
            return None;
        }
        let own = path == from.trim_end_matches('/');
        let refused = Disk::HERE.refuses(name).filter(|_| !own);
        if let Some(refused) = refused {
            let (taken, kept) = match refused {
                DiskRefusal::Character(c) => ("character", c.to_string()),
                DiskRefusal::Device => (
                    "device",
                    name.split('.').next().unwrap_or_default().to_string(),
                ),
                DiskRefusal::Trailing(c) => ("trailing", c.to_string()),
            };
            return Some(NewWorktree {
                name: name.to_string(),
                taken: taken.to_string(),
                path,
                kept,
            });
        }
        let key = same_path_key(&path);
        let free = path_free(std::path::Path::new(&path));
        let taken = if own {
            ""
        } else if key == same_path_key(from) {
            if free { "" } else { "case" }
        } else if self
            .worktree_rows()
            .iter()
            .any(|row| same_path_key(&row.full) == key)
        {
            "listed"
        } else if !free {
            "folder"
        } else {
            ""
        };
        Some(NewWorktree {
            name: name.to_string(),
            taken: taken.to_string(),
            path,
            kept: String::new(),
        })
    }

    /// Whether new worktrees have a place at all: the listing names the
    /// repository's own worktree, and that worktree has a parent to go beside
    /// it in (`worktrees_folder`). Nothing on disk is looked at.
    pub(super) fn has_place_for_worktrees(&self) -> bool {
        self.worktree_rows().iter().any(|row| {
            row.change == MAIN && platitude_core::worktrees::worktrees_folder(&row.full).is_some()
        })
    }

    /// The card's rows. Empty for every section but the worktrees.
    ///
    /// No filter, and no row the section is showing as gone — the worktrees
    /// `total` counts — each said by the section's own `field`.
    pub(super) fn worktree_rows(&self) -> Vec<WorktreeRow> {
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
                WorktreeRow {
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

    /// The worktree at `path`, as its row says it — what a WORKTREE card stands
    /// on, whichever entrance raised it. `None` for a path this listing
    /// does not hold, or holds as gone.
    pub(super) fn worktree_of(&self, path: &str) -> Option<WorktreeRow> {
        self.worktree_rows()
            .into_iter()
            .find(|row| row.full == path)
    }

    /// Whether the worktree at `path` (as git lists it) is the repository's
    /// own — whose marks are the house where the others' are the tree
    /// (デザイン規約 §ref の種別). False for a path not listed.
    pub(super) fn is_home_worktree(&self, path: &str) -> bool {
        self.worktree_of(path).is_some_and(|row| row.change == MAIN)
    }

    /// The name the worktree at `path` goes by where something else names
    /// it — its WORKTREES row's: the branch the repository's own has out,
    /// the folder otherwise and where that one is detached (デザイン規約
    /// §ref の種別「リポジトリ自身の worktree の呼び名」). The path's last
    /// segment for a worktree not listed, empty for an empty path.
    pub(super) fn name_of_worktree(&self, path: &str) -> String {
        match self.worktree_of(path) {
            Some(row) if row.change == MAIN && !row.branch.is_empty() => row.branch,
            Some(row) => row.name,
            None => crate::urlpath::path_leaf(path).to_string(),
        }
    }

    /// The path of the worktree called `name` standing on `head` — what a
    /// folder's chip on the graph names, which carries only the folder:
    /// two worktrees of one name under different parents sit on different
    /// commits, or are the one chip. Empty where none does.
    pub(super) fn worktree_standing(&self, name: &str, head: &str) -> String {
        self.worktree_rows()
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

    fn worktree_entry(
        path: &str,
        branch: Option<&str>,
        locked: bool,
    ) -> platitude_core::worktrees::WorktreeEntry {
        // One commit per worktree, told apart by the folder.
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
            held: None,
        }
    }

    fn worktrees() -> NavSectionModel {
        section(
            "worktrees",
            Source::Worktrees {
                list: vec![
                    worktree_entry("C:/work/home", Some("main"), false),
                    worktree_entry("C:/work/plain", Some("topic"), false),
                    worktree_entry("C:/work/held", None, true),
                ],
                current: "c:/work/home".to_string(),
            },
        )
    }

    fn fulls(model: &NavSectionModel) -> Vec<String> {
        model
            .worktree_rows()
            .into_iter()
            .map(|row| row.full)
            .collect()
    }

    #[test]
    fn the_card_ignores_what_the_section_is_filtering_to() {
        let mut model = worktrees();
        model.arrange();
        let whole = fulls(&model);
        assert_eq!(whole.len(), 3);

        model.filter = "plain".to_string();
        model.arrange();
        assert_eq!(model.shown_rows(), 1, "the filter left one row on the list");
        assert_eq!(fulls(&model), whole);
    }

    /// What names the repository's own worktree from elsewhere: the house,
    /// and its branch for a name — its folder while detached. Every other
    /// worktree by its folder (デザイン規約 §ref の種別).
    #[test]
    fn the_repositorys_own_worktree_is_home_and_goes_by_its_branch() {
        let model = worktrees();
        assert!(model.is_home_worktree("C:/work/home"));
        assert!(!model.is_home_worktree("C:/work/plain"));
        assert!(!model.is_home_worktree(""));
        assert_eq!(model.name_of_worktree("C:/work/home"), "main");
        assert_eq!(model.name_of_worktree("C:/work/plain"), "plain");
        assert_eq!(
            model.name_of_worktree("C:/work/gone"),
            "gone",
            "one not listed: its folder"
        );
        assert_eq!(model.name_of_worktree(""), "");

        let detached = section(
            "worktrees",
            Source::Worktrees {
                list: vec![worktree_entry("C:/work/home", None, false)],
                current: "c:/work/plain".to_string(),
            },
        );
        assert!(detached.is_home_worktree("C:/work/home"));
        assert_eq!(detached.name_of_worktree("C:/work/home"), "home");
    }

    /// The branch whose marks are the house: the one the repository's own
    /// worktree keeps — a rebase standing there too — and only while this
    /// window stands elsewhere.
    #[test]
    fn the_home_branch_is_the_one_the_repositorys_own_worktree_keeps_elsewhere() {
        let mut model = worktrees();
        let list = vec![
            worktree_entry("C:/work/home", Some("main"), false),
            worktree_entry("C:/work/plain", Some("topic"), false),
        ];
        assert!(model.settle_home_branch(&list, "c:/work/plain"));
        assert_eq!(model.home_branch, "main");
        assert!(
            !model.settle_home_branch(&list, "c:/work/plain"),
            "the same listing moves nothing"
        );
        assert!(model.settle_home_branch(&list, "c:/work/home"));
        assert_eq!(
            model.home_branch, "",
            "none where this window is that worktree"
        );

        let mut rebasing = worktree_entry("C:/work/home", None, false);
        rebasing.held = Some(platitude_core::worktrees::Held {
            branch: "main".to_string(),
            by: platitude_core::worktrees::HeldBy::Rebase,
        });
        model.settle_home_branch(&[rebasing], "c:/work/plain");
        assert_eq!(
            model.home_branch, "main",
            "a rebase standing there keeps it"
        );
    }

    #[test]
    fn a_worktree_shown_as_gone_is_off_the_card() {
        let mut model = worktrees();
        model.hidden = vec!["C:/work/plain".to_string()];
        model.arrange();
        assert_eq!(model.total, 2);
        assert_eq!(fulls(&model), vec!["C:/work/home", "C:/work/held"]);
    }

    #[test]
    fn a_worktree_row_says_what_the_sections_row_draws() {
        let rows = worktrees().worktree_rows();
        assert_eq!(
            rows[0],
            WorktreeRow {
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
    /// commit from a chip; a worktree shown as gone has no card.
    #[test]
    fn a_worktree_is_found_by_its_path_or_by_its_folder_where_it_stands() {
        let mut model = worktrees();
        let held = model.worktree_of("C:/work/held").expect("listed");
        assert_eq!(
            (held.change.as_str(), held.reason.as_str()),
            ("LOCKED", "kept")
        );
        assert_eq!(model.worktree_standing("held", "3333"), "C:/work/held");
        assert_eq!(
            model.worktree_standing("held", "1111"),
            "",
            "not standing there"
        );
        assert!(model.worktree_of("C:/work/elsewhere").is_none());

        model.hidden = vec!["C:/work/held".to_string()];
        model.arrange();
        assert!(
            model.worktree_of("C:/work/held").is_none(),
            "going: no card for it"
        );
    }

    /// A new worktree goes beside the repository's own, under its name — the
    /// tab may stand in another worktree, and the place does not move with it.
    #[test]
    fn a_new_worktree_is_placed_off_the_repositorys_own_worktree() {
        let place = worktrees().new_worktree("feature/login").expect("a place");
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
                    worktree_entry("C:/work/home", Some("main"), false),
                    worktree_entry("C:/work/home.worktrees/topic", Some("topic"), false),
                    worktree_entry("C:/work/home.worktrees/Fix-Case", Some("fix/case"), false),
                ],
                current: "c:/work/home".to_string(),
            },
        );
        assert_eq!(
            model.new_worktree("topic").expect("a place").taken,
            "listed"
        );
        // One folder to a disk that does not tell case apart (Windows,
        // macOS), where git turns the place down as well.
        assert_eq!(
            model.new_worktree("fix/case").expect("a place").taken,
            "listed"
        );
    }

    /// Looked at on disk: something in the folder takes it, an empty
    /// folder does not (git fills one).
    #[test]
    fn a_folder_holding_something_is_taken() {
        let dir = crate::temp::root().join(format!("pgg-new-worktree-{}", std::process::id()));
        let home = dir.join("home").to_string_lossy().replace('\\', "/");
        let model = section(
            "worktrees",
            Source::Worktrees {
                list: vec![worktree_entry(&home, Some("main"), false)],
                current: home.clone(),
            },
        );
        let place = dir.join("home.worktrees").join("busy");
        std::fs::create_dir_all(&place).unwrap();
        let empty = model.new_worktree("busy").expect("a place").taken;
        std::fs::write(place.join("x.txt"), "x").unwrap();
        let holding = model.new_worktree("busy").expect("a place").taken;
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(empty, "");
        assert_eq!(holding, "folder");
    }

    #[test]
    fn no_place_before_the_listing_names_the_repositorys_own_worktree() {
        let model = section(
            "worktrees",
            Source::Worktrees {
                list: vec![worktree_entry("C:/work/plain", Some("topic"), false)],
                current: "c:/work/plain".to_string(),
            },
        );
        assert!(model.new_worktree("x").is_none());
        assert!(!model.has_place_for_worktrees());
        assert!(worktrees().new_worktree("").is_none());
        assert!(worktrees().has_place_for_worktrees());
    }

    /// A renamed worktree stays in its parent, and a name that is not one
    /// folder's has no place; its own name is no rename, and a path git still
    /// lists for another worktree is taken.
    #[test]
    fn a_renamed_worktree_stays_beside_where_it_was() {
        let model = worktrees();
        // Looked at on disk too: a name no machine has there.
        let leaf = format!("pgg-renamed-beside-{}", std::process::id());
        let place = model
            .renamed_worktree("C:/work/held", &leaf)
            .expect("a place");
        assert_eq!(place.path, format!("C:/work/{leaf}"));
        assert_eq!(place.taken, "");
        assert_eq!(
            model
                .renamed_worktree("C:/work/held", "held")
                .expect("a place")
                .taken,
            "",
            "its own name is the way out"
        );
        assert_eq!(
            model
                .renamed_worktree("C:/work/held", "plain")
                .expect("a place")
                .taken,
            "listed"
        );
        for not in ["", "..", "a/b", "a\\b"] {
            assert!(
                model.renamed_worktree("C:/work/held", not).is_none(),
                "{not:?}"
            );
        }
    }

    /// What this system keeps out of a name is said before the disk is looked
    /// at, with the part it keeps — on Windows; elsewhere the same names are
    /// free. The folder's own name stays the way out.
    #[test]
    fn a_name_the_system_keeps_has_its_reason_and_the_part_it_keeps() {
        let model = worktrees();
        let answer = |name: &str| {
            let place = model
                .renamed_worktree("C:/work/held", name)
                .expect("one folder's name");
            (place.taken, place.kept)
        };
        let owed = [
            ("why?", ("character", "?")),
            ("con.txt", ("device", "con")),
            ("dot.", ("trailing", ".")),
        ];
        for (name, (taken, kept)) in owed {
            let want = if cfg!(windows) {
                (taken.to_string(), kept.to_string())
            } else {
                (String::new(), String::new())
            };
            assert_eq!(answer(name), want, "{name:?}");
        }
        assert_eq!(
            model
                .renamed_worktree("C:/work/con", "con")
                .expect("its own")
                .taken,
            "",
            "its own name is no rename"
        );
    }

    /// Looked at on disk: anything at all takes the place, an empty folder too
    /// (git would move the worktree into it); a case-only rename is taken where
    /// the disk calls it the same folder.
    #[test]
    fn anything_at_a_renamed_worktrees_place_takes_it() {
        let dir = crate::temp::root().join(format!("pgg-renamed-worktree-{}", std::process::id()));
        let at = |leaf: &str| dir.join(leaf).to_string_lossy().replace('\\', "/");
        let model = section(
            "worktrees",
            Source::Worktrees {
                list: vec![worktree_entry(&at("loose"), None, false)],
                current: at("home"),
            },
        );
        std::fs::create_dir_all(dir.join("loose")).unwrap();
        std::fs::create_dir_all(dir.join("empty")).unwrap();
        let empty = model
            .renamed_worktree(&at("loose"), "empty")
            .expect("a place")
            .taken;
        let case = model
            .renamed_worktree(&at("loose"), "LOOSE")
            .expect("a place")
            .taken;
        let case_free = !dir.join("LOOSE").exists();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(empty, "folder");
        assert_eq!(case, if case_free { "" } else { "case" });
    }

    #[test]
    fn only_the_worktrees_section_has_a_worktree_card() {
        let model = section(
            "branches",
            Source::Locals(locals(vec![local("main", true)])),
        );
        assert!(model.worktree_rows().is_empty());
    }
}
