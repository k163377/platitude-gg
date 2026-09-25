//! The toolbar's push, from the press to the answer that is its own.

use super::Press;

/// One push the toolbar's button sent — the plain push, the leased
/// overwrite, the first push the question just took — and the branch it
/// was about, until git's answer to that press and no other.
///
/// By id alone: a remote branch's replace and delete answer under the same
/// label, and the fetch running behind a press comes back in the same
/// drain, so a property every answer rewrites reads somebody else's answer
/// as this push's.
///
/// The branch outlives the answer: a refusal is remembered per branch
/// (デザイン規約 §リモートへ送る), the page reads it off the notify the
/// answer arrived in, and only a second press overwrites it.
#[derive(Debug, Default)]
pub struct PushOut {
    press: Press,
    /// What the last press was sent for.
    branch: String,
}

impl PushOut {
    /// The push for `branch` went to the queue under `accepted`.
    ///
    /// The branch is written down even where the queue accepted nothing,
    /// so a later reader is handed this press's own.
    pub fn asked(&mut self, accepted: Option<u64>, branch: String) {
        self.press.asked(accepted);
        self.branch = branch;
    }

    /// Puts down where the last notify's answer stood.
    pub fn new_notify(&mut self) {
        self.press.new_notify();
    }

    /// git answered a write, standing at `at` in this notify's answers.
    /// Says whether it was this press's.
    pub fn answered(&mut self, id: u64, at: usize) -> bool {
        self.press.answered(id, at)
    }

    /// Where this press's own answer stands in this notify, or `None`
    /// where this notify carried none.
    #[must_use]
    pub fn answer(&self) -> Option<usize> {
        self.press.answer()
    }

    /// The branch the last press was sent for — the one an answer found
    /// here is about.
    #[must_use]
    pub fn branch(&self) -> &str {
        &self.branch
    }
}
