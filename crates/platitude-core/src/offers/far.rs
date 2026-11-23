//! What a ref's menu reads of the far side.

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
