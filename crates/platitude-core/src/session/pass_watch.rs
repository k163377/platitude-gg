//! What a graph pass says when it ends without saying anything: the drop
//! guard every pass carries, and where its report goes. The passes
//! themselves are [`super::log`].

use super::*;

/// A graph pass that has begun and has not answered for itself yet.
///
/// **A panic is the one way a pass ends without a word.** Every other
/// ending goes through a `match` that reports it — a git that failed, a
/// timeout, a cancellation — but a task that panics dies where it stands,
/// so neither arm runs and nothing is sent. The runtime catches it at the
/// task boundary and the process carries on, which is what makes it quiet:
/// the graph is left turning on an empty column, or left standing on a
/// picture no later ref will ever change (measured — a window in the
/// second state is indistinguishable from a healthy one).
///
/// So a pass carries this, and the unwind that kills it drops it. What it
/// says is what the pass could not.
pub(super) struct PassWatch<'a> {
    session: &'a RepoSession,
    /// Which of the two ways this pass would have reported itself.
    told: Told,
    answered: bool,
}

/// Where a pass that fell over has to say so, which is wherever it would
/// have said anything at all.
enum Told {
    /// The pass announced itself and reset the graph for its own stream,
    /// so the column is empty and turning: the answer belongs to that
    /// stream ([`SessionEvent::LogFailed`]).
    Stream(u64),
    /// The pass built off screen and would have replaced the graph at the
    /// end. Nothing on screen is waiting, and what is standing there is a
    /// real picture — only older than it should be — so this reads as the
    /// operation it was ([`RepoSession::fail`]).
    Operation,
}

impl<'a> PassWatch<'a> {
    pub(super) fn operation(session: &'a RepoSession) -> Self {
        Self {
            session,
            told: Told::Operation,
            answered: false,
        }
    }

    /// The pass has announced its stream: from here an empty column is
    /// waiting on this generation, and nothing else can answer for it.
    pub(super) fn announced(&mut self, generation: u64) {
        self.told = Told::Stream(generation);
    }

    /// The pass reported itself, whichever way it went.
    pub(super) fn answered(&mut self) {
        self.answered = true;
    }
}

impl Drop for PassWatch<'_> {
    fn drop(&mut self) {
        if self.answered {
            return;
        }
        // The runtime is going away and taking its tasks with it. Nothing
        // is left to read a report, and a window that is closing must not
        // be told its history could not be read.
        if self.session.root_cancel.is_cancelled() {
            return;
        }
        tracing::error!("the graph walk ended without an answer");
        // **Caught, because this runs inside the unwind it is reporting.**
        // A second panic crossing an unwinding frame is an abort, and the
        // way out of here reaches a feed and a QML invoker — nothing this
        // side owns. Failing to report is what the state was before this
        // guard existed; killing the window is not.
        let told = &self.told;
        let session = self.session;
        let reported = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match told {
            Told::Stream(generation) => session.sink.event(SessionEvent::LogFailed {
                generation: *generation,
                // **No words**: nobody said anything, so what the screen
                // shows is its own — the sentence behind the band's
                // `PARTIAL HISTORY` badge (`BandStateCard`,
                // app-ui.md「Rust に文言を置かない」).
                error: String::new(),
            }),
            Told::Operation => session.fail(
                "log",
                GitError::UnexpectedOutput {
                    command: "git log".to_string(),
                    message: "the graph walk ended without an answer".to_string(),
                },
            ),
        }));
        if reported.is_err() {
            tracing::error!("and the report of it fell over too");
        }
    }
}
