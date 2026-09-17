//! The plain branch delete, and the card that stayed up for git's
//! answer.

use super::Press;

/// The one press whose menu stays up: `git branch --delete` may be
/// turned down, and the refusal has to land on the row that asked for
/// it (`AppMenuItem.staysOpen`).
///
/// **What the press writes down is the name and the id together.** The
/// card goes, or turns its row into the held `-D`, off the name it asked
/// with — and which answer that name belongs to is a thing only the
/// press still knows: a drain empties the whole queue and notifies once,
/// so the fetch running behind the press answers in the same batch and
/// looks exactly like an answer to this.
///
/// **The answer stands until the next plain delete is asked**, because
/// the card reads it as an edge and may not have been drawn yet when it
/// arrives. Where in this notify it answered is the drain's — that is
/// [`Self::answer`], put down at the top of every drain, and it is how
/// the page tells the answer in hand from one standing over from before.
///
/// Only the plain form writes here. `-D` and `Delete both` were already
/// the answer to a refusal and their menus go at the press, so they
/// stand for nothing.
#[derive(Debug, Default)]
pub struct BranchDeleteOut {
    /// The write the card is waiting on, by the id the queue accepted it
    /// under — and where its answer stood in the notify that carried it.
    press: Press,
    /// The branch the standing card asked about, held until that answer
    /// turns it into one of the two below.
    name: String,
    /// git's answer by that name: the branch it took, and the one it
    /// turned down. At most one of them is ever set.
    landed: String,
    refused: String,
}

impl BranchDeleteOut {
    /// The press: a plain delete of `name` went to the queue and came
    /// back with this id.
    ///
    /// The last answer goes with it, so a delete of a re-made branch
    /// of the same name reads its own answer as a change. Where the
    /// queue accepted nothing (the session is closed) the last answer
    /// still goes — it is over either way — but nothing is waited for:
    /// no answer is coming, and one arriving under some other press's
    /// id is not this card's.
    pub fn asked(&mut self, name: &str, accepted: Option<u64>) {
        self.landed = String::new();
        self.refused = String::new();
        self.name = name.to_string();
        self.press.asked(accepted);
    }

    /// Puts down where the last notify's answer stood: this drain
    /// answers for itself. What git said is left standing — the card
    /// reads that as an edge, and may not have been drawn yet.
    pub fn new_notify(&mut self) {
        self.press.new_notify();
    }

    /// git answered a write, standing at `at` in this notify's answers.
    /// Says whether it was this card's.
    ///
    /// `reported` is whether the refusal came with something to
    /// report — the far side keeping the branch. **That one goes to
    /// the page's notice bar**: the refusal is the far side's, so
    /// the card has nothing to morph into and the row stays as it
    /// is (デザイン規約 §答えの要らない報せ).
    pub fn answered(&mut self, id: u64, at: usize, failed: bool, reported: bool) -> bool {
        if !self.press.answered(id, at) {
            return false;
        }
        let asked = std::mem::take(&mut self.name);
        let (took, turned_down) = match (failed, reported) {
            (false, _) => (asked, String::new()),
            (true, false) => (String::new(), asked),
            (true, true) => (String::new(), String::new()),
        };
        self.landed = took;
        self.refused = turned_down;
        true
    }

    /// The branch git took, for the card to go on.
    #[must_use]
    pub fn landed(&self) -> &str {
        &self.landed
    }

    /// The branch git would not take on its own, for the row to turn on.
    #[must_use]
    pub fn refused(&self) -> &str {
        &self.refused
    }

    /// Where this card's own answer stands in this notify, or `None`
    /// where this notify carried none.
    #[must_use]
    pub fn answer(&self) -> Option<usize> {
        self.press.answer()
    }
}
