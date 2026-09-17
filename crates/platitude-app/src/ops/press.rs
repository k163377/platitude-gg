//! One press, and the answer that is its own.

/// A write this window asked for: the id the queue accepted it under,
/// and **where that write's own answer stands** in the notify that
/// carried it.
///
/// **By id alone.** One drain empties the whole queue and notifies
/// once (`RepoTab::write_answers`), so a notify can carry several
/// answers — the fetch running behind a press comes back in the
/// middle of it. A property every answer rewrites is left
/// describing whichever finished last, and read that way a commit
/// that landed never empties the editor, a delete git took never
/// closes the card that stayed up for it, and a stash that refused
/// is never reported. All of them are silence, which is why none of
/// them shows up as a failure anywhere.
///
/// **The answer's place is the notify's own.** It is put down
/// at the top of every drain ([`Self::new_notify`]), so what a
/// page reads here is this drain's own — which is the whole of
/// what "the press is answered once" rests on.
///
/// **What git said is on the answer itself**, where every
/// reader waiting for its own write reads it
/// (`RepoTab::write_answer_at`); this holds the one thing
/// the answer cannot say, which is whose press it was.
/// What a press needs to remember beyond that is the
/// owner's, and the owners hold one of these (`StashOut`,
/// `BranchDeleteOut`) — the editor's commit is a bare one.
#[derive(Debug, Default)]
pub struct Press {
    /// The write this press is waiting on, as the bridge carries an
    /// `OperationId` — zero while none is out.
    waiting: u64,
    /// Where its answer stands in the answers this notify carried, or
    /// `None` where this notify carried none of it.
    at: Option<usize>,
}

impl Press {
    /// The press: what it asked for went to the queue and came back with
    /// this id.
    ///
    /// Where the queue accepted nothing (the session is closed) nothing
    /// is waited for — an answer is what the press is held open by, and
    /// one nobody will ever answer would hold it for good.
    pub fn asked(&mut self, accepted: Option<u64>) {
        self.waiting = accepted.filter(|id| *id != 0).unwrap_or_default();
        if self.waiting == 0 {
            tracing::debug!("press the queue took nothing for has no answer coming");
        }
    }

    /// Puts down where the last notify's answer stood: this drain
    /// answers for itself, and an empty place is a drain that brought
    /// this press nothing.
    pub fn new_notify(&mut self) {
        self.at = None;
    }

    /// git answered a write, standing at `at` in this notify's answers.
    /// Says whether it was this press's — **an answer nobody is waiting
    /// for is somebody else's to make sense of**, and the tab folds
    /// those into the group it keeps for readers that hold no id.
    pub fn answered(&mut self, id: u64, at: usize) -> bool {
        if self.waiting == 0 || self.waiting != id {
            return false;
        }
        self.waiting = 0;
        self.at = Some(at);
        true
    }

    /// Where this press's own answer stands in this notify, or `None`
    /// where this notify carried none.
    #[must_use]
    pub fn answer(&self) -> Option<usize> {
        self.at
    }
}
