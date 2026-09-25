//! What a graph pass says when it ends without saying anything: the drop
//! guard every pass carries, and where its report goes. The passes
//! themselves are [`super::log`].

use super::*;

/// A graph pass that has begun and has not answered for itself yet.
///
/// **A panic is the one way a pass ends without a word**: every other
/// ending goes through a reporting `match`, but the runtime swallows a
/// panic at the task boundary, leaving the graph turning on an empty
/// column or standing on a picture no later ref will change. So a pass
/// carries this, and the unwind drops it: it says what the pass could not.
pub(super) struct PassWatch<'a> {
    session: &'a RepoSession,
    /// Which of the two ways this pass would have reported itself.
    told: Told,
    answered: bool,
}

/// Where a pass that fell over has to say so: wherever it would have said
/// anything at all.
enum Told {
    /// The pass reset the graph for its own stream, so the empty, turning
    /// column waits on that stream's answer ([`SessionEvent::LogFailed`]).
    Stream(u64),
    /// The pass built off screen to swap in at the end. What stands on
    /// screen is a real, older picture, so this reads as the operation it
    /// was ([`RepoSession::fail`]).
    Operation,
}

/// A place inside a graph pass that the outside is let into
/// ([`PassHooks`]). The two steps are the two `Told`s: past the first the
/// pass owns an empty column, past the second nothing on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassStep {
    /// A streaming pass, once it has emptied the graph and announced the
    /// generation the column is now turning on.
    Streaming,
    /// An off-screen pass, at the point it begins the walk it would swap
    /// in at the end.
    Swapping,
}

/// What the outside is let into a graph pass with: every pass asks it at
/// each [`PassStep`], on its own task. The panic `PassWatch` reports is
/// swallowed at the task boundary and nothing outside can cause one, so
/// this is where a test causes it — else a report that stopped working
/// would look like the silence it breaks.
///
/// Handed in at [`RepoSession::open`], like the sink and the command
/// observer. Nothing in this crate implements one, so the shipped binary
/// has no door here.
pub trait PassHooks: Send + Sync + 'static {
    /// Called by every pass that reaches `at`, on that pass's own task;
    /// what runs there, and how often, is the implementation's. **The door
    /// a test ends a pass through** — the panic is the implementation's.
    ///
    /// Called outside every lock the pass holds: what runs here may
    /// unwind, and a guard held across that poisons a lock the next pass
    /// takes. An implementation handing out something to run once owes
    /// the same to its own lock: take it out under the lock, run it
    /// outside.
    fn before(&self, at: PassStep);

    /// The fault standing at `at`, as the error the walk there would have
    /// returned; the pass fails through the ordinary reporting arm.
    ///
    /// **The door a *screen* is driven through**: the band's state for a
    /// graph that is not the repository's needs a failing git, which a
    /// demo repository does not have.
    ///
    /// **Asked of every pass, and meant to stand**: a fault one pass could
    /// lift is a race — the pass already walking takes it, the next one
    /// succeeds, and the mark goes up and down before anything reads it.
    fn fault(&self, at: PassStep) -> Option<GitError>;

    /// Whether a pass must walk as one that began before this window's
    /// first status did: no row of its own uncommitted work at the top,
    /// and no leash from it.
    ///
    /// **The one arrangement a repository cannot be walked into**: which
    /// of walk and status arrives first is the scheduler's, and the readers
    /// landing on the working tree answer for the pass that lost — every
    /// other working copy has a row, this window none, all wearing the
    /// all-zero id. `false` for everything but a harness.
    ///
    /// **While it is up, every pass is published** (`RepoSession::run_swap_pass`
    /// otherwise drops a pass whose picture is already on screen): after a
    /// write nothing else may differ — a stopped replay moves no branch —
    /// so the arrangement would be walked and then dropped.
    fn holds_back_the_working_tree_row(&self) -> bool {
        false
    }
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

    /// Whether a pass must walk as one that began before the first status
    /// did ([`PassHooks::holds_back_the_working_tree_row`]).
    pub(super) fn holds_back_the_working_tree_row(&self) -> bool {
        self.pass_hooks
            .as_ref()
            .is_some_and(|hooks| hooks.holds_back_the_working_tree_row())
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
        // Closing: nothing is left to read a report.
        if self.session.root_cancel.is_cancelled() {
            return;
        }
        tracing::error!("the graph walk ended without an answer");
        // **Caught**: this runs inside the unwind it reports, and a second
        // panic there is an abort — the way out reaches a feed and a QML
        // invoker this side does not own.
        let told = &self.told;
        let session = self.session;
        let reported = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match told {
            Told::Stream(generation) => session.sink.event(SessionEvent::LogFailed {
                generation: *generation,
                // Empty: the words are the band's (`BandStateCard`,
                // rules-refs/app-ui.md「Rust に文言を置かない」).
                error: String::new(),
            }),
            Told::Operation => {
                // The graph left standing is whole but stale
                // (`SessionEvent::LogStale`).
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
