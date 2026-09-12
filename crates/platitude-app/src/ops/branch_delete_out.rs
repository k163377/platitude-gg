//! The plain branch delete, and the card that stayed up for git's
//! answer.

/// The one press whose menu does not go with it: `git branch --delete`
/// may be turned down, and the refusal has to land on the row that asked
/// for it (`AppMenuItem.staysOpen`).
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
/// arrives. Where in this notify it answered does not stand — that is
/// [`Self::answer`], put down at the top of every drain, and it is how
/// the page tells the answer in hand from one standing over from before.
///
/// Only the plain form writes here. `-D` and `Delete both` were already
/// the answer to a refusal and their menus go at the press, so they
/// stand for nothing.
#[derive(Debug, Default)]
pub struct BranchDeleteOut {
    /// The branch the standing card asked about, and the id the queue
    /// took its write under — zero while none is out.
    name: String,
    waiting: u64,
    /// git's answer by that name: the branch it took, and the one it
    /// turned down. At most one of them is ever set.
    landed: String,
    refused: String,
    /// Where that answer stands in the answers this notify carried, or
    /// `None` where this notify carried none of it.
    at: Option<usize>,
}

impl BranchDeleteOut {
    /// The press: a plain delete of `name` went to the queue and came
    /// back with this id.
    ///
    /// The last answer goes with it, so a delete of a re-made branch of
    /// the same name reads its own answer as a change rather than as the
    /// old one still standing. Where the queue accepted nothing (the
    /// session is closed) the last answer still goes — it is over either
    /// way — but nothing is waited for: no answer is coming, and one
    /// arriving under some other press's id is not this card's.
    pub fn asked(&mut self, name: &str, accepted: Option<u64>) {
        self.landed = String::new();
        self.refused = String::new();
        self.name = name.to_string();
        self.waiting = accepted.filter(|id| *id != 0).unwrap_or_default();
        if self.waiting == 0 {
            tracing::debug!("branch delete the queue took nothing for has no answer coming");
        }
    }

    /// Puts down where the last notify's answer stood: this drain
    /// answers for itself. What git said is left standing — the card
    /// reads that as an edge, and may not have been drawn yet.
    pub fn new_notify(&mut self) {
        self.at = None;
    }

    /// git answered a write, standing at `at` in this notify's answers.
    /// Says whether it was this card's.
    ///
    /// `reported` is whether the refusal came with something to report —
    /// the far side keeping the branch, rather than git declining on its
    /// own. **That one turns no row**: nothing about the name was
    /// refused, so the card has nothing to morph into and the words go
    /// to the page's notice bar instead (デザイン規約 §答えの要らない報せ).
    pub fn answered(&mut self, id: u64, at: usize, failed: bool, reported: bool) -> bool {
        if self.waiting == 0 || self.waiting != id {
            return false;
        }
        self.waiting = 0;
        let asked = std::mem::take(&mut self.name);
        let (took, turned_down) = match (failed, reported) {
            (false, _) => (asked, String::new()),
            (true, false) => (String::new(), asked),
            (true, true) => (String::new(), String::new()),
        };
        self.landed = took;
        self.refused = turned_down;
        self.at = Some(at);
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
        self.at
    }
}
