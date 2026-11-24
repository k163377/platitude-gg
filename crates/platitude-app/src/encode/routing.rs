//! The bucket a worktree file row is routed by — the word the lists hand
//! QML (`models::nav`'s `Role::Bucket`) and QML hands back to the diff and
//! the stage presses, read here once.

use platitude_core::details::DiffTarget;
use platitude_core::stage::DiscardSide;

/// Which side of the worktree a file row is about, as its diff and its
/// stage presses address it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Routing {
    /// Unmerged: its diff is the working tree's, and nothing restores it
    /// until it is resolved.
    Conflicts,
    Unstaged,
    Untracked,
    Staged,
}

impl Routing {
    /// The word as the lists spell it; `None` for any other (a committed
    /// diff's `commit`, a caller's slip).
    pub fn of_word(word: &str) -> Option<Self> {
        Some(match word {
            "conflicts" => Self::Conflicts,
            "unstaged" => Self::Unstaged,
            "untracked" => Self::Untracked,
            "staged" => Self::Staged,
            _ => return None,
        })
    }

    pub fn word(self) -> &'static str {
        match self {
            Self::Conflicts => "conflicts",
            Self::Unstaged => "unstaged",
            Self::Untracked => "untracked",
            Self::Staged => "staged",
        }
    }

    /// The diff the row shows. A conflicted file shows its working-tree
    /// state; `orig_path` (empty for none) is a staged rename's source.
    pub fn diff_target(self, path: &str, orig_path: &str) -> DiffTarget {
        let path = path.to_string();
        match self {
            Self::Staged => DiffTarget::Staged {
                path,
                orig_path: (!orig_path.is_empty()).then(|| orig_path.to_string()),
            },
            Self::Untracked => DiffTarget::Untracked { path },
            Self::Conflicts | Self::Unstaged => DiffTarget::Unstaged { path },
        }
    }

    /// What a discard of the row throws away; none for a conflicted one,
    /// which git refuses to restore until told how it was resolved.
    pub fn discard_side(self) -> Option<DiscardSide> {
        match self {
            Self::Conflicts => None,
            Self::Unstaged => Some(DiscardSide::Unstaged),
            Self::Untracked => Some(DiscardSide::Untracked),
            Self::Staged => Some(DiscardSide::Staged),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bucket_reads_back_from_its_word_and_no_other_does() {
        for routing in [
            Routing::Conflicts,
            Routing::Unstaged,
            Routing::Untracked,
            Routing::Staged,
        ] {
            assert_eq!(Routing::of_word(routing.word()), Some(routing));
        }
        assert_eq!(Routing::of_word("commit"), None);
        assert_eq!(Routing::of_word("whole"), None);
        assert_eq!(Routing::of_word(""), None);
    }

    #[test]
    fn a_conflicted_row_shows_the_working_tree_and_restores_nothing() {
        assert_eq!(
            Routing::Conflicts.diff_target("f.txt", ""),
            DiffTarget::Unstaged {
                path: "f.txt".into()
            }
        );
        assert_eq!(Routing::Conflicts.discard_side(), None);
        assert_eq!(
            Routing::Staged.diff_target("new.txt", "old.txt"),
            DiffTarget::Staged {
                path: "new.txt".into(),
                orig_path: Some("old.txt".into())
            }
        );
    }
}
