//! What the details pane's message boxes do with the commit on screen.
//!
//! Apart from the menus: here typing is itself the start of a rewrite,
//! and a caret lands from a click nobody aimed, so the question is "is
//! this the commit a stray keystroke may reach".

/// What the details pane does with the message boxes: take typing, or
/// refuse it and say why.
///
/// One answer: asked apart, a pane can draw a live box with a refusal in
/// its tooltip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageEdit {
    /// No commit on screen, or the tab is not open: nothing to refuse or
    /// explain.
    Nothing,
    /// Typing lands, and saving it is `commit --amend`.
    Amend,
    /// A stash: there is no amend to run on it — its label is rewritten
    /// from the list on the left (`stash::rename`).
    Stash,
    /// A commit HEAD is not on ([`message_edit`]).
    NotHead,
    /// An operation is standing on the repository ([`message_edit`]).
    Standing,
}

impl MessageEdit {
    /// The word `GitFacts.messageEdit` answers with and the page
    /// branches on; `""` where there is nothing to say.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Nothing => "",
            Self::Amend => "amend",
            Self::Stash => "stash",
            Self::NotHead => "not-head",
            Self::Standing => "standing",
        }
    }
}

/// Whether the boxes take typing, and what stands in the way when they
/// do not.
///
/// `oid_hex` is the commit on screen, `head_oid` the one HEAD is on (an
/// id: detached has no branch to ask), `stash_ref` the reflog selector
/// when that row is a stash (`""` otherwise), and `op_text` the standing
/// operation's own word — `""` exactly when nothing stands
/// (`WorktreeModel.op_text`).
///
/// **Only HEAD's own commit.** Amending an older one replays every commit
/// after it, and the first stray keystroke would start that; rewrites
/// asked for by name stay in the commit menu ([`super::commit_menu`]'s
/// `edit_history`).
///
/// **Nothing while an operation stands**, since git refuses the amend
/// there (`You are in the middle of a merge -- cannot amend`; mid-rebase
/// `Committing is not possible because you have unmerged files`) — except
/// an interactive rebase stopped at an `edit` step (`editing`,
/// [`crate::integrate::RebaseStop`]), where git takes the amend.
///
/// **The exemption is HEAD's, and nothing narrower.** Holding it to the
/// commit the stop began on refuses the second amend: the first moves
/// HEAD off it and nothing under `rebase-merge/` follows
/// (rules-refs/core.md「`edit` 停止中の amend の門」, デザイン規約
/// §フル interactive rebase).
///
/// **Detached still takes typing**: `commit --amend` works on a detached
/// HEAD, unlike the menu's rewrite rows, which need a branch.
pub fn message_edit(
    open: bool,
    oid_hex: &str,
    head_oid: &str,
    stash_ref: &str,
    op_text: &str,
    editing: bool,
) -> MessageEdit {
    if !open || oid_hex.is_empty() {
        return MessageEdit::Nothing;
    }
    // A stash can be HEAD's own by hash, so it answers first.
    if !stash_ref.is_empty() {
        return MessageEdit::Stash;
    }
    if !op_text.is_empty() && !(editing && oid_hex == head_oid) {
        return MessageEdit::Standing;
    }
    if oid_hex != head_oid {
        return MessageEdit::NotHead;
    }
    MessageEdit::Amend
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "abc123";
    const OLDER: &str = "def456";

    /// The same open tab, `oid` on screen, no edit stop.
    fn edit(oid: &str, stash_ref: &str, op_text: &str) -> MessageEdit {
        message_edit(true, oid, HEAD, stash_ref, op_text, false)
    }

    #[test]
    fn heads_own_commit_is_the_one_the_boxes_take() {
        assert_eq!(edit(HEAD, "", ""), MessageEdit::Amend);
        assert_eq!(edit(HEAD, "", "").as_str(), "amend");
    }

    #[test]
    fn a_commit_further_back_is_read_only() {
        assert_eq!(edit(OLDER, "", ""), MessageEdit::NotHead);
        assert_eq!(edit(OLDER, "", "").as_str(), "not-head");
    }

    #[test]
    fn a_standing_operation_takes_the_amend_away() {
        assert_eq!(edit(HEAD, "", "MERGING"), MessageEdit::Standing);
    }

    #[test]
    fn an_edit_stop_opens_heads_commit_and_nothing_else() {
        assert_eq!(
            message_edit(true, HEAD, HEAD, "", "REBASING", true),
            MessageEdit::Amend
        );
        assert_eq!(
            message_edit(true, OLDER, HEAD, "", "REBASING", true),
            MessageEdit::Standing
        );
        assert_eq!(
            message_edit(true, HEAD, HEAD, "stash@{0}", "REBASING", true),
            MessageEdit::Stash
        );
    }

    /// Binds the second amend: the first moves HEAD off the id the stop
    /// began on. `editing` is the whole exemption, so a rebase stopped for
    /// any other reason keeps the refusal.
    #[test]
    fn an_edit_stop_opens_whatever_head_moved_onto() {
        assert_eq!(
            message_edit(true, OLDER, OLDER, "", "REBASING", true),
            MessageEdit::Amend
        );
        assert_eq!(
            message_edit(true, HEAD, HEAD, "", "REBASING", false),
            MessageEdit::Standing
        );
    }

    // Of the two refusals, the stash's is the one the reader can act on.
    #[test]
    fn a_stash_says_it_is_a_stash_before_anything_else() {
        assert_eq!(edit(HEAD, "stash@{0}", "MERGING"), MessageEdit::Stash);
        assert_eq!(edit(OLDER, "stash@{0}", ""), MessageEdit::Stash);
    }

    #[test]
    fn an_empty_pane_refuses_without_a_reason() {
        assert_eq!(edit("", "", ""), MessageEdit::Nothing);
        assert_eq!(
            message_edit(false, HEAD, HEAD, "", "", false),
            MessageEdit::Nothing
        );
        assert_eq!(edit("", "", "").as_str(), "");
    }
}
