//! The toolbar's push, from the press to the answer that is its own.

use super::Press;

/// One push the toolbar's button sent — the plain push, the leased
/// overwrite, the first push the question just took — and the branch it
/// was about, until git's answer to that press and no other.
///
/// **By id alone.** A remote branch's rename and delete answer under
/// the same label, and the fetch running behind a press comes back in
/// the same drain: a property every answer rewrote said "a push
/// answered" about somebody else's, and the button beside it put the
/// refusal on whichever branch was checked out by then
/// (`PublishFlow.pushSentBranch`, before this). The press writes the
/// id down here with the branch it was sent for, and what is handed
/// to the page is the answer at that id and the branch beside it.
///
/// **The branch outlives the answer**: a refusal is remembered per
/// branch (デザイン規約 §リモートへ送る), and the page reads which branch
/// off the same notify the answer arrived in — after which only a
/// second press overwrites it.
#[derive(Debug, Default)]
pub struct PushOut {
    press: Press,
    /// What the last press was sent for.
    branch: String,
}

impl PushOut {
    /// The press: the push went to the queue and came back with this id,
    /// and was about `branch`.
    ///
    /// Where the queue accepted nothing nothing is waited for — an
    /// answer that is never coming would hold it open for good — and
    /// the branch is still written down, so a later reader is handed
    /// this press's own.
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
