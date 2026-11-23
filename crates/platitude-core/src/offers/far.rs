//! What a ref's menu reads of the far side: a local branch's upstream, or
//! the sides a tag's name stands on. One value per kind of row, built by
//! the model that lists the row and handed to [`super::ref_menu`] whole.

use super::RefKind;

/// What the menu on one row reads over there.
///
/// Only a local branch and a tag have a far side its rows read: a remote
/// branch is a reading itself, and a stash or a worktree has none. A far
/// side is read on a row of its own kind alone ([`Self::on`]); the rows of
/// a row's own menu read none, the cards' rows do (`RefBranchMenu` /
/// `RefTagMenu`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FarSide {
    /// Nothing read over there: the rows that reach it stay out.
    #[default]
    Unread,
    Branch(BranchFar),
    Tag(TagFar),
}

impl FarSide {
    /// The far side as a row of `kind` reads it: a branch's on a local
    /// branch's row, a tag's on a tag's, and nothing on any other pairing.
    ///
    /// The kind is the menu's word and the far side the model's record,
    /// handed to `GitFacts.refMenuOffers` apart, so no type keeps the two
    /// together; this is the one place they are matched.
    pub(super) fn on(self, kind: RefKind) -> Self {
        match (kind, self) {
            (RefKind::Branch, Self::Branch(_)) | (RefKind::Tag, Self::Tag(_)) => self,
            _ => Self::Unread,
        }
    }
}

/// A local branch's upstream, as its card reads it
/// (`RefsSnapshot::branch_far`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BranchFar {
    /// The remote reading the branch speaks for.
    pub reading: Reading,
    /// An upstream at all — the reading, or one gone or here, which leaves
    /// [`Self::reading`] at [`Reading::None`].
    pub tracked: bool,
    /// The repository has a remote to name an upstream on.
    pub has_remote: bool,
}

/// Where the remote reading a local branch speaks for stands.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Reading {
    /// No listed reading: the branch tracks nothing, a branch here, or one
    /// gone.
    #[default]
    None,
    /// On the branch's commit: one row on the graph.
    Together,
    /// On another commit, so the pair is two rows
    /// (`session::BranchItem::upstream_drifted`). The reading is deleted
    /// from its own row.
    Drifted,
}

/// The sides a tag's name stands on, and what its rows that reach over
/// there stand on (`TagMenuFacts::far`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TagFar {
    /// `None` where the section has not answered for the name, which reads
    /// as a tag here: the everyday delete keeps its seat, and the rows that
    /// need a remote reading stay out until there is one.
    pub sides: Option<TagSides>,
    /// A remote for the card's rows to go to — where its push goes
    /// (`TagMenuFacts::push_remote`). With none, no row reaches over there,
    /// whatever reading is left from before.
    pub has_remote: bool,
    /// Why the rows that delete over there stand greyed, if they do.
    pub held_back: TagDeleteHeld,
}

/// Which sides of a tag exist ([`TagFar::sides`]).
///
/// The rows differ by side (`tag --delete` needs a local tag,
/// `push --delete` a remote one, sending a local one), and the sidebar lists
/// a name held on both sides once, so the row cannot tell on its own
/// (`session::TagItem::sides`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagSides {
    /// Made here and no remote is known to carry the name.
    Here,
    /// A remote carries it and this repository does not.
    Remote,
    Both,
}

impl TagSides {
    /// Decoded from the word a tag's record carries (`encode::far`); `None`
    /// for a tag whose section has not been read yet.
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "here" => Some(Self::Here),
            "remote" => Some(Self::Remote),
            "both" => Some(Self::Both),
            _ => None,
        }
    }

    /// The word [`Self::from_word`] reads.
    pub fn word(self) -> &'static str {
        match self {
            Self::Here => "here",
            Self::Remote => "remote",
            Self::Both => "both",
        }
    }

    /// Whether this repository holds the tag.
    pub(super) fn here(self) -> bool {
        matches!(self, Self::Here | Self::Both)
    }

    /// Whether a remote was last heard to carry the name.
    pub(super) fn on_remote(self) -> bool {
        matches!(self, Self::Remote | Self::Both)
    }
}

/// Why a tag menu's rows that delete over there stand greyed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TagDeleteHeld {
    /// They do not.
    #[default]
    No,
    /// The remote reached has the name on another commit than here.
    Drifted,
    /// Several remotes carry the name, none of them the reference, and
    /// nobody named one.
    Unnamed,
}
