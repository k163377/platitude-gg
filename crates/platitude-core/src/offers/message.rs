//! What the details pane's message boxes do with the commit on screen.
//!
//! Apart from the menus above: those weigh what a row may be *sent* to,
//! while this weighs the one place in the app where typing is itself the
//! start of a rewrite. A caret lands in a text box from a click nobody
//! aimed, so the question here is not "could this commit be rewritten"
//! but "is this the commit a stray keystroke may reach".

/// What the details pane does with the message boxes: take typing, or
/// refuse it and say why.
///
/// One answer rather than a pair, because "may this be edited" and "why
/// not" are the same question asked twice — a pane that asks them apart
/// can draw a live box with a refusal in its tooltip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageEdit {
    /// No commit on screen, or the tab is not open: the boxes hold no
    /// message, so there is nothing to refuse and nothing to explain.
    Nothing,
    /// Typing lands, and saving it is `commit --amend`.
    Amend,
    /// A stash. git keeps one off to the side of every branch, so there
    /// is no amend to run on it — the list on the left is where its
    /// label is rewritten (`stash::rename`).
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
/// `oid_hex` is the commit on screen, `head_oid` the one HEAD is on
/// (**not** the current branch's tip — detached, there is no branch to
/// ask), `stash_ref` the reflog selector when that row is a stash (`""`
/// otherwise), and `op_text` the standing operation's own word — `""`
/// exactly when nothing stands, which is the test the rest of the UI
/// asks (`WorkTreeModel.op_text`).
///
/// **Only HEAD's own commit.** Any older one is replayed rather than
/// amended, and a replay hands every commit after it a new id; the boxes
/// are not a place to put that behind an accident, because they take a
/// caret from any click that lands in them and the first keystroke is
/// already the rewrite's first keystroke. GitKraken draws the same line
/// — the most recent commit is the one its panel lets a click edit
/// (help.gitkraken.com/gitkraken-desktop/commits, observed) — and the
/// rewrites that are asked for by name keep their rows in the commit
/// menu ([`super::commit_menu`]'s `edit_history`).
///
/// **Nothing while an operation stands** — with one exception. git
/// refuses the amend outright there: `You are in the middle of a merge --
/// cannot amend`, the same sentence for a cherry-pick, and `Committing is
/// not possible because you have unmerged files` mid-rebase (measured, 2.55).
/// The exception is the stop that exists *for* amending: an interactive
/// rebase stopped at an `edit` step sits with HEAD on the commit, a clean
/// tree, and an amend git takes — `editing` is git's own word on it and
/// `edit_oid` names the commit it stopped on
/// ([`crate::integrate::RebaseStop`], abbreviated as git wrote it).
///
/// **The exemption is the stopped commit's, not HEAD's.** git leaves the
/// stop standing while a terminal makes further commits on top, and HEAD
/// then names a commit the exit card's "amend it above" was never about —
/// so the boxes open only while HEAD still is the stopped commit. An
/// empty `edit_oid` (git wrote none) falls back to trusting HEAD.
///
/// **Detached is not a reason.** `git commit --amend` on a detached HEAD
/// amends as usual (measured, 2.55) — unlike the menu's rewrite rows, which
/// need a branch to carry the result.
pub fn message_edit(
    open: bool,
    oid_hex: &str,
    head_oid: &str,
    stash_ref: &str,
    op_text: &str,
    editing: bool,
    edit_oid: &str,
) -> MessageEdit {
    if !open || oid_hex.is_empty() {
        return MessageEdit::Nothing;
    }
    // A stash is a commit and can be HEAD's own by hash, so it answers
    // first: what it is beats where it sits.
    if !stash_ref.is_empty() {
        return MessageEdit::Stash;
    }
    let head_is_the_stop = edit_oid.is_empty() || head_oid.starts_with(edit_oid);
    if !op_text.is_empty() && !(editing && head_is_the_stop && oid_hex == head_oid) {
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
        message_edit(true, oid, HEAD, stash_ref, op_text, false, "")
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

    /// The stop that exists for amending is the exception: HEAD's commit
    /// opens, and every other row stays refused as before.
    #[test]
    fn an_edit_stop_opens_heads_commit_and_nothing_else() {
        assert_eq!(
            message_edit(true, HEAD, HEAD, "", "REBASING", true, "abc1"),
            MessageEdit::Amend
        );
        assert_eq!(
            message_edit(true, OLDER, HEAD, "", "REBASING", true, "abc1"),
            MessageEdit::Standing
        );
        assert_eq!(
            message_edit(true, HEAD, HEAD, "stash@{0}", "REBASING", true, "abc1"),
            MessageEdit::Stash
        );
    }

    /// A terminal commit made during the stop moves HEAD off the stopped
    /// commit; the boxes shut rather than offering an amend of a commit
    /// the exit card never named. An unnamed stop trusts HEAD as before.
    #[test]
    fn a_head_that_left_the_stopped_commit_takes_the_amend_away() {
        assert_eq!(
            message_edit(true, HEAD, HEAD, "", "REBASING", true, "def9"),
            MessageEdit::Standing
        );
        assert_eq!(
            message_edit(true, HEAD, HEAD, "", "REBASING", true, ""),
            MessageEdit::Amend
        );
    }

    // Which of the two refusals a stash under a stopped operation gets:
    // the row is a stash whatever the repository is in the middle of, and
    // that is the one the reader can act on.
    #[test]
    fn a_stash_says_it_is_a_stash_before_anything_else() {
        assert_eq!(edit(HEAD, "stash@{0}", "MERGING"), MessageEdit::Stash);
        assert_eq!(edit(OLDER, "stash@{0}", ""), MessageEdit::Stash);
    }

    #[test]
    fn an_empty_pane_refuses_without_a_reason() {
        assert_eq!(edit("", "", ""), MessageEdit::Nothing);
        assert_eq!(
            message_edit(false, HEAD, HEAD, "", "", false, ""),
            MessageEdit::Nothing
        );
        assert_eq!(edit("", "", "").as_str(), "");
    }
}
