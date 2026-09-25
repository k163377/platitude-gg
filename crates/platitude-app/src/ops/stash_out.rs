//! The stash that took the working tree away, and the tree turning out
//! to be empty behind it.

use super::Press;

/// One stash press that empties the working tree, from the press to the
/// reading that shows the tree gone.
///
/// git answering says the entry was made; what the page waits for is the
/// status behind it showing the tree empty, and only then does whose doing
/// that was matter — somebody committing the same changes in a terminal
/// empties the tree the same way, over the message the reader is still
/// writing.
///
/// Either half may arrive first (the two feeds are drained apart): the
/// answer is written down as it comes and the tree is handed in as the
/// page last read it ([`Self::taken`]), so whichever is second completes
/// the pair. Settled from one side only, a press loses the tree that
/// emptied in the other order.
///
/// Which status can answer is a stamp: the counts say which report of HEAD
/// they were read beside (`WorkTreeModel.statusSeq`), and the answer names
/// the smallest one that speaks for what it left. A status from before
/// that fence would settle the press on the dirty tree before the empty
/// one arrived; nor is the report of HEAD in hand a measure, since one
/// arrives alone, with no counts.
///
/// Only the presses that can empty a tree write here: armed by a pop,
/// apply, drop or rename, it would claim a tree somebody else emptied.
#[derive(Debug, Default)]
pub struct StashOut {
    /// The write that took the tree away, and where its answer stood in
    /// this notify — the page's cue to re-read the open file or to say
    /// what git refused.
    press: Press,
    /// That write landed and nothing has read the tree it left yet.
    ours: bool,
    /// The smallest report the counts can stand beside and still speak
    /// for that write; zero while no answer is standing.
    after: u64,
}

impl StashOut {
    /// The stash went to the queue under `accepted`. Whatever an earlier
    /// press left standing goes: the tree emptying next is the work of the
    /// stash asked for last.
    pub fn asked(&mut self, accepted: Option<u64>) {
        self.ours = false;
        self.after = 0;
        self.press.asked(accepted);
    }

    /// Puts down where the last notify's answer stood; the wait for the
    /// tree outlives any number of notifies.
    pub fn new_notify(&mut self) {
        self.press.new_notify();
    }

    /// git answered a write, standing at `at` in this notify's answers
    /// and naming `after` as the smallest report the counts that can
    /// speak for it will stand beside. Says whether it was this press's —
    /// a refusal too, whose words are nobody else's to say.
    ///
    /// Only a landed answer claims the tree: a refusal took nothing away.
    pub fn answered(&mut self, id: u64, at: usize, failed: bool, after: u64) -> bool {
        if !self.press.answered(id, at) {
            return false;
        }
        self.ours = !failed;
        self.after = after;
        true
    }

    /// The page is acting on the working tree as it last read it: the
    /// counts stood beside the report numbered `seen`, and `emptied` is
    /// whether they left nothing in it.
    ///
    /// Says whether this window's own stash is what emptied it. The page
    /// asks from both sides of the pair — where a status lands and where a
    /// write answers. The first status that can speak for the press settles
    /// it either way: a tree still dirty was not emptied by the stash, and a
    /// later emptying is somebody else's.
    pub fn taken(&mut self, seen: u64, emptied: bool) -> bool {
        if !self.ours || self.after == 0 || seen < self.after {
            return false;
        }
        self.ours = false;
        self.after = 0;
        emptied
    }

    /// Where this press's own answer stands in this notify, or `None`
    /// where this notify carried none.
    #[must_use]
    pub fn answer(&self) -> Option<usize> {
        self.press.answer()
    }

    /// Whether a landing is still standing.
    #[cfg(test)]
    pub fn ours(&self) -> bool {
        self.ours
    }
}
