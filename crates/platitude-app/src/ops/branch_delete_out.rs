//! The plain branch delete, and the card that stayed up for git's
//! answer.

use super::Press;

/// The one press whose menu stays up: `git branch --delete` may be
/// turned down, and the refusal has to land on the row that asked for
/// it (`AppMenuItem.staysOpen`).
///
/// The press writes down the name with the id: only the id tells this
/// answer from the fetch's in the same drain ([`Press`]).
///
/// The answer stands until the next plain delete is asked: the card reads
/// it as an edge and may not have been drawn yet. [`Self::answer`] is put
/// down at the top of every drain — how the page tells this notify's answer
/// from one standing over.
///
/// Only the plain form writes here: `-D` and `Delete both` close their
/// menus at the press.
#[derive(Debug, Default)]
pub struct BranchDeleteOut {
    /// The write the card waits on, and where its answer stood in this
    /// notify.
    press: Press,
    /// The branch asked about, until the answer moves it to one of the two
    /// below.
    name: String,
    /// git's answer: the branch it took, or the one it turned down. At most
    /// one is set.
    landed: String,
    refused: String,
}

impl BranchDeleteOut {
    /// A plain delete of `name` went to the queue under `accepted`.
    ///
    /// Clears the last answer, so a delete of a re-made branch of the same
    /// name reads its own answer as a change — also where the queue
    /// accepted nothing (`None`: the session is closed), and then nothing
    /// is waited for.
    pub fn asked(&mut self, name: &str, accepted: Option<u64>) {
        self.landed = String::new();
        self.refused = String::new();
        self.name = name.to_string();
        self.press.asked(accepted);
    }

    /// Puts down where the last notify's answer stood; what git said stays.
    pub fn new_notify(&mut self) {
        self.press.new_notify();
    }

    /// git answered a write, standing at `at` in this notify's answers.
    /// Says whether it was this card's.
    ///
    /// A `reported` refusal (the far side keeping the branch) sets neither
    /// name: it goes to the page's notice bar and the row stays as it is
    /// (デザイン規約 §答えの要らない報せ).
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
