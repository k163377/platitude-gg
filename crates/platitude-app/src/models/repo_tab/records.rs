//! The records a tab hands QML and takes back: what a remote was asked
//! about, a write's report, the commit editor's unsent words, and the rows
//! a delete has taken off the screen.

use platitude_core::remote::RemoteBranch;

use crate::encode::{Fields, Record, field};

/// The name a remote was asked about (`checkRemoteBranch`), and the branch
/// here whose history the answer weighed it against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteBranchAsked {
    pub target: RemoteBranch,
    pub local: String,
}

impl Record for RemoteBranchAsked {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        Fields::new()
            .put("remote", &self.target.remote)
            .put("branch", &self.target.branch)
            .put("local", &self.local)
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Option<Self> {
        Some(Self {
            target: RemoteBranch {
                remote: field(map, "remote")?,
                branch: field(map, "branch")?,
            },
            local: field(map, "local")?,
        })
    }
}

/// What a write that did not happen says for itself, for the page's notice
/// (デザイン規約 §答えの要らない報せ): `kind` (`drain::report_word`) is what both
/// sentences turn on, `remote` and `name` what it is about, and `fact` the
/// one thing the second line names beside them (`WriteReport::fact`) —
/// never anybody's message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reported {
    pub kind: String,
    pub remote: String,
    pub name: String,
    pub fact: String,
}

impl Record for Reported {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        Fields::new()
            .put("kind", &self.kind)
            .put("remote", &self.remote)
            .put("name", &self.name)
            .put("fact", &self.fact)
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Option<Self> {
        Some(Self {
            kind: field(map, "kind")?,
            remote: field(map, "remote")?,
            name: field(map, "name")?,
            fact: field(map, "fact")?,
        })
    }
}

/// The commit editor's unsent words as the tab holds them while it has no
/// page (`hub::Draft`), handed back whole: the words and whether they
/// were an amend's travel together.
impl Record for crate::hub::Draft {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        Fields::new()
            .put("subject", &self.subject)
            .put("body", &self.body)
            .put("amending", &self.amending)
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Option<Self> {
        Some(Self {
            subject: field(map, "subject")?,
            body: field(map, "body")?,
            amending: field(map, "amending")?,
        })
    }
}

/// What `RepoTab.goneRows` hands the graph (`GraphModel.setGone`), under the
/// fields' own names.
impl Record for crate::ops::Rows {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        Fields::new()
            .put("branch", &self.branch)
            .put("remote", &self.remote)
            .put("tag", &self.tag)
            .put("stash", &self.stash)
            .put("worktree", &self.worktree)
            .put("worktreeChip", &self.worktree_chip)
            .put("worktreeBranch", &self.worktree_branch)
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Option<Self> {
        Some(Self {
            branch: field(map, "branch")?,
            remote: field(map, "remote")?,
            tag: field(map, "tag")?,
            stash: field(map, "stash")?,
            worktree: field(map, "worktree")?,
            worktree_chip: field(map, "worktreeChip")?,
            worktree_branch: field(map, "worktreeBranch")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every record a tab hands QML comes back as it went, under the
    /// fields' own names.
    #[test]
    fn a_tabs_records_round_trip() {
        let asked = RemoteBranchAsked {
            target: RemoteBranch {
                remote: "my/fork".into(),
                branch: "feature/x".into(),
            },
            local: "x".into(),
        };
        assert_eq!(RemoteBranchAsked::from_map(&asked.to_map()), Some(asked));
        let report = Reported {
            kind: "outdated".into(),
            remote: "origin".into(),
            name: "main".into(),
            fact: String::new(),
        };
        assert_eq!(Reported::from_map(&report.to_map()), Some(report));
        let draft = crate::hub::Draft {
            subject: "fix: the subject".into(),
            body: "The body.\n".into(),
            amending: true,
        };
        assert_eq!(crate::hub::Draft::from_map(&draft.to_map()), Some(draft));
        let rows = crate::ops::Rows {
            branch: "topic".into(),
            worktree_chip: "topic-tree".into(),
            worktree_branch: "topic".into(),
            ..crate::ops::Rows::default()
        };
        assert_eq!(crate::ops::Rows::from_map(&rows.to_map()), Some(rows));
    }
}
