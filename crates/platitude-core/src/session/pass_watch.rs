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

/// A place inside a graph pass that a test can be let into.
///
/// **The one ending in the session nothing outside can ask for.**
/// `PassWatch` speaks for a pass that stopped without a word, and the
/// only thing that stops one that way is a panic — which the runtime
/// swallows at the task boundary, so a report that stopped working would
/// look exactly like the silence it exists to break. Every other ending
/// is asked for from outside and can be driven from there; this one has
/// to be caused from inside the pass, which is what these are for.
///
/// The two steps are the two answers (`Told`): past the first the pass
/// owns an empty column, past the second it owns nothing on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassStep {
    /// A streaming pass, once it has emptied the graph and announced the
    /// generation the column is now turning on.
    Streaming,
    /// An off-screen pass, at the point it begins the walk it would swap
    /// in at the end.
    Swapping,
}

/// What the next pass to reach a given step runs there.
pub(super) type PassStepHook = (PassStep, Box<dyn FnOnce() + Send>);

impl RepoSession {
    /// Leaves `run` for the next graph pass to reach `at`, to be run
    /// there, on that pass's own task, once.
    ///
    /// **The door a test ends a pass through**, since nothing it can ask
    /// for ends one the way `PassWatch` answers for: a failed git, a
    /// timeout and a cancellation all leave by a `match` that reports
    /// itself, and only an unwind leaves by no arm at all. So the fault
    /// has to be raised inside the pass, and `run` is what raises it —
    /// the panic is the caller's, which is why there is none here.
    ///
    /// Taken by the pass that runs it, so exactly one falls over. A pass
    /// reaching a step nobody left anything at is a lock and a look.
    pub fn run_inside_next_pass(&self, at: PassStep, run: impl FnOnce() + Send + 'static) {
        *relock(&self.pass_step) = Some((at, Box::new(run)));
    }

    /// Runs what was left at `at`, if that is the step it was left at.
    ///
    /// Taken out under the lock and run outside it: what it is here to do
    /// is unwind, and a guard held across that would poison the lock —
    /// which the pass behind this one would then take, run, and unwind
    /// through in turn.
    pub(super) fn run_pass_step(&self, at: PassStep) {
        let run = {
            let mut left = relock(&self.pass_step);
            match left.take() {
                Some((step, run)) if step == at => Some(run),
                other => {
                    *left = other;
                    None
                }
            }
        };
        if let Some(run) = run {
            run();
        }
    }
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
