//! One press, and the answer that is its own.

/// A write this window asked for: the id the queue accepted it under,
/// and where that write's own answer stands in the notify that carried it.
///
/// By id alone: one drain empties the whole queue and notifies once
/// (`RepoTab::write_answers`), so the fetch running behind a press can
/// answer in the same notify. A property every answer rewrites describes
/// whichever finished last, and the misses — an editor never emptied, a
/// card never closed, a refusal never reported — are all silent.
///
/// The answer's place is put down at the top of every drain
/// ([`Self::new_notify`]), so a press is answered once.
///
/// What git said is on the answer itself (`RepoTab::write_answer_at`); this
/// holds only whose press it was. Owners that remember more hold one of
/// these (`StashOut`, `BranchDeleteOut`); the editor's commit is a bare one.
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
    /// What the press asked for went to the queue under `accepted`.
    ///
    /// `None` (the session is closed) waits for nothing: an answer that
    /// never comes would hold the press open for good.
    pub fn asked(&mut self, accepted: Option<u64>) {
        self.waiting = accepted.filter(|id| *id != 0).unwrap_or_default();
        if self.waiting == 0 {
            tracing::debug!("press the queue took nothing for has no answer coming");
        }
    }

    /// Puts down where the last notify's answer stood.
    pub fn new_notify(&mut self) {
        self.at = None;
    }

    /// git answered a write, standing at `at` in this notify's answers.
    /// Says whether it was this press's; the tab folds the others into the
    /// group it keeps for readers that hold no id.
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
