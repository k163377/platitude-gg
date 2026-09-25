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
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            name: field(map, "name")?,
            full: field(map, "full")?,
            branch: field(map, "branch")?,
            change: field(map, "change")?,
        })
    }
}

impl NavSectionModel {
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
                }
            })
            .collect()
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
        platitude_core::worktrees::WorktreeEntry {
            path: path.to_string(),
            branch: branch.map(str::to_string),
            head_hex: None,
            bare: false,
            detached: branch.is_none(),
            locked,
            lock_reason: String::new(),
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
                    copy("C:\\work\\home", Some("main"), false),
                    copy("C:\\work\\plain", Some("topic"), false),
                    copy("C:\\work\\held", None, true),
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
        model.hidden = vec!["C:\\work\\plain".to_string()];
        model.arrange();
        assert_eq!(model.total, 2);
        assert_eq!(fulls(&model), vec!["C:\\work\\home", "C:\\work\\held"]);
    }

    #[test]
    fn a_copy_row_says_what_the_sections_row_draws() {
        let rows = copies().copy_rows();
        assert_eq!(
            rows[0],
            CopyRow {
                name: "home".to_string(),
                full: "C:\\work\\home".to_string(),
                branch: "main".to_string(),
                change: "MAIN".to_string(),
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

    #[test]
    fn only_the_working_copies_have_a_copy_card() {
        let model = section(
            "branches",
            Source::Locals(locals(vec![local("main", true)])),
        );
        assert!(model.copy_rows().is_empty());
    }
}
