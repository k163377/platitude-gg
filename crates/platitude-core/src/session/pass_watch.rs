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

/// A place inside a graph pass that the outside is let into
/// ([`PassHooks`]).
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

/// What the outside is let into a graph pass with.
///
/// **The one ending in the session nothing outside can ask for.**
/// `PassWatch` speaks for a pass that stopped without a word, and the
/// only thing that stops one that way is a panic — which the runtime
/// swallows at the task boundary, so a report that stopped working would
/// look exactly like the silence it exists to break. Every other ending
/// is asked for from outside and can be driven from there; this one has
/// to be caused from inside the pass, which is what this is for: every
/// pass asks it at each [`PassStep`], on its own task.
///
/// **Handed in when the session is opened** ([`RepoSession::open`]), in
/// the shape the sink and the command observer take, and for the same
/// reason: what implements it lives with whoever drives the session —
/// the tests, the application's harness — and a session opened with
/// `None` neither asks nor holds anything for it. Nothing in this crate
/// implements one, so the shipped binary has no door here.
pub trait PassHooks: Send + Sync + 'static {
    /// Called by every pass that reaches `at`, on that pass's own task.
    /// Whether anything runs there, and how many times, is the
    /// implementation's to decide.
    ///
    /// **The door a test ends a pass through**, since nothing it can ask
    /// for ends one the way `PassWatch` answers for: a failed git, a
    /// timeout and a cancellation all leave by a `match` that reports
    /// itself, and only an unwind leaves by no arm at all. So the fault
    /// has to be raised inside the pass, and this is what raises it —
    /// the panic is the implementation's, which is why the pass has none.
    ///
    /// Called outside every lock the pass holds, because what runs here
    /// may unwind, and a guard held across that would poison a lock the
    /// pass behind this one takes next. An implementation that hands
    /// something out to be run once owes the same to its own lock: take
    /// it out under the lock, run it outside.
    fn before(&self, at: PassStep);

    /// The fault standing at `at`, as the error the walk there would have
    /// come back with. The pass fails there in place of the walk, by the
    /// same reporting arm a git that failed would have left through —
    /// the report, the words and the mark are all the ordinary ones.
    ///
    /// **The door a *screen* is driven through**, where [`Self::before`]
    /// is the one a test ends a pass through. What the band says about a
    /// graph that is not the repository's cannot be photographed
    /// otherwise: the state needs a git that fails, and a demo repository
    /// built to be walked has no such git in it.
    ///
    /// **Asked of every pass, and meant to stand.** A fault one pass
    /// could lift would be a race — the pass already walking takes it,
    /// the pass the caller then asks for succeeds, and the mark goes up
    /// and straight back down before anything can be read off it.
    fn fault(&self, at: PassStep) -> Option<GitError>;
}

impl RepoSession {
    /// Lets what was handed in at opening into the pass at `at`.
    pub(super) fn run_pass_step(&self, at: PassStep) {
        if let Some(hooks) = &self.pass_hooks {
            hooks.before(at);
        }
    }

    /// The fault standing at `at`, if anything was handed in that has one.
    pub(super) fn pass_fault(&self, at: PassStep) -> Option<GitError> {
        self.pass_hooks.as_ref().and_then(|hooks| hooks.fault(at))
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
                // `STALE GRAPH` badge (`BandStateCard`,
                // app-ui.md「Rust に文言を置かない」).
                error: String::new(),
            }),
            Told::Operation => {
                // The graph left standing is whole and no longer this
                // repository's, which is the half of the badge's state
                // this arm is (`SessionEvent::LogStale`).
                session.tell_graph_stale(true);
                session.fail(
                    "log",
                    GitError::UnexpectedOutput {
                        command: "git log".to_string(),
                        message: "the graph walk ended without an answer".to_string(),
                    },
                );
            }
        }));
        if reported.is_err() {
            tracing::error!("and the report of it fell over too");
        }
    }
}
