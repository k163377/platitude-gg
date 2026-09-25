//! The fault the harness raises inside a graph pass — the one state of
//! the band a demo repository cannot be walked into — and the seam it
//! reaches the session through ([`PassHooks`]); and the held
//! configuration saves. A build without the harness answers the seam
//! `None`.

use std::sync::Arc;

use platitude_core::session::PassHooks;
#[cfg(feature = "automation")]
use platitude_core::session::PassStep;

/// The fault standing over every graph pass this process runs — one for
/// the process, since a run (`graph-stale` / `graph-stopped`) photographs
/// one window on one repository. It stands once raised: one a pass could
/// lift would be taken by the pass already walking and the next would
/// succeed, so the mark would go straight back down (`PassHooks::fault`).
#[cfg(feature = "automation")]
#[derive(Default)]
struct GraphFaults {
    at: std::sync::Mutex<Option<PassStep>>,
    /// Whether every pass must walk as one that began before the first
    /// status did ([`PassHooks::holds_back_the_working_tree_row`]).
    /// Raised before the repository is opened, so the opening's own walk
    /// is held too; lowered by the run once it has read the page.
    holds_the_row: std::sync::atomic::AtomicBool,
    /// How many passes have reached a step, whether the last of them met
    /// the fault, and the wake each one sends — what [`fail_graph_pass`]
    /// keeps the fault's picture up by.
    reached: std::sync::atomic::AtomicU64,
    last_met: std::sync::atomic::AtomicBool,
    reaching: tokio::sync::Notify,
}

#[cfg(feature = "automation")]
impl GraphFaults {
    fn standing() -> &'static Arc<Self> {
        static STANDING: std::sync::OnceLock<Arc<GraphFaults>> = std::sync::OnceLock::new();
        STANDING.get_or_init(Arc::default)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<PassStep>> {
        match self.at.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn reached(&self) -> u64 {
        self.reached.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn last_met(&self) -> bool {
        self.last_met.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(feature = "automation")]
impl PassHooks for GraphFaults {
    /// A pass that gets this far can take the fault's picture down, until
    /// it too meets the fault ([`fail_graph_pass`]).
    fn before(&self, _at: PassStep) {
        self.last_met
            .store(false, std::sync::atomic::Ordering::SeqCst);
        self.reached
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.reaching.notify_one();
    }

    fn fault(&self, at: PassStep) -> Option<platitude_core::GitError> {
        let standing = *self.lock();
        (standing == Some(at)).then(|| {
            self.last_met
                .store(true, std::sync::atomic::Ordering::SeqCst);
            platitude_core::GitError::UnexpectedOutput {
                command: "git log".to_string(),
                message: "the graph walk was made to fail".to_string(),
            }
        })
    }

    fn holds_back_the_working_tree_row(&self) -> bool {
        self.holds_the_row.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Raises the hold before anything is opened (`fault_hold_wip_row`).
#[cfg(feature = "automation")]
pub(crate) fn hold_the_working_tree_row() {
    GraphFaults::standing()
        .holds_the_row
        .store(true, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(not(feature = "automation"))]
pub(crate) fn hold_the_working_tree_row() {}

/// Lowers it and asks the tab's session for the pass that carries the
/// row, answering whether the hold had been up
/// (`GraphModel.letTheWorkingTreeRowThrough`).
#[cfg(feature = "automation")]
pub(crate) fn let_the_working_tree_row_through(tab_id: i32) -> bool {
    let held = GraphFaults::standing()
        .holds_the_row
        .swap(false, std::sync::atomic::Ordering::SeqCst);
    if held {
        crate::hub::with_session(tab_id, |s| s.refresh_log());
    }
    held
}

#[cfg(not(feature = "automation"))]
pub(crate) fn let_the_working_tree_row_through(_tab_id: i32) -> bool {
    false
}

/// Asks the tab's session for a pass with the hold left standing,
/// answering whether it was up (`GraphModel.walkAgainWhileHeld`). Nothing
/// else brings one: a rebuild follows a read that moved
/// (`session::refresh`), a stopped replay moves no branch, and this
/// window's row appearing is what the hold takes away.
#[cfg(feature = "automation")]
pub(crate) fn walk_again_while_held(tab_id: i32) -> bool {
    let held = GraphFaults::standing()
        .holds_the_row
        .load(std::sync::atomic::Ordering::SeqCst);
    if held {
        crate::hub::with_session(tab_id, |s| s.refresh_log());
    }
    held
}

#[cfg(not(feature = "automation"))]
pub(crate) fn walk_again_while_held(_tab_id: i32) -> bool {
    false
}

/// What every session this application opens lets into its graph passes
/// (`RepoSession::open`).
#[cfg(feature = "automation")]
pub(crate) fn pass_hooks() -> Option<Arc<dyn PassHooks>> {
    let standing: Arc<dyn PassHooks> = Arc::clone(GraphFaults::standing()) as _;
    Some(standing)
}

#[cfg(not(feature = "automation"))]
pub(crate) fn pass_hooks() -> Option<Arc<dyn PassHooks>> {
    None
}

/// Every graph pass reaching `step` (`swapping` for the off-screen
/// rebuild, anything else for the stream) fails there from here on, and
/// one is asked of the tab's session — and asked again, for as long as
/// the run lasts, whenever the passes have all stopped and the last did
/// not meet the fault (`GraphModel.failGraphPass`).
///
/// An ask is not a pass: a read that moved asks for a rebuild, which
/// cancels the pass it displaces (`take_log_run`), and nothing on screen
/// tells the harness QML its ask was lost or a later pass took the
/// picture down. So the session's boundary
/// (`RepoSession::wait_for_graph_passes`) and [`GraphFaults`] decide. An
/// ask no pass reached ends it: the session was closed.
#[cfg(feature = "automation")]
pub(crate) fn fail_graph_pass(tab_id: i32, step: &str) {
    let at = if step == "swapping" {
        PassStep::Swapping
    } else {
        PassStep::Streaming
    };
    let faults = Arc::clone(GraphFaults::standing());
    *faults.lock() = Some(at);
    let Some(session) = crate::hub::from_session(tab_id, Arc::downgrade) else {
        return;
    };
    let Some(runtime) = crate::hub::Hub::with(|hub| hub.runtime_handle()).flatten() else {
        return;
    };
    runtime.spawn(async move {
        loop {
            let reached = faults.reached();
            let Some(asked) = session.upgrade() else {
                return;
            };
            match at {
                // Off screen, so the whole graph is left standing and goes
                // out of date where it is.
                PassStep::Swapping => asked.refresh_log(),
                // The column is emptied first, so the walk stops with rows
                // missing.
                PassStep::Streaming => asked.restart_log(),
            }
            asked.wait_for_graph_passes().await;
            drop(asked);
            if faults.reached() == reached {
                return;
            }
            // While it stands, only the next pass to reach a step can take
            // it down.
            while faults.last_met() {
                faults.reaching.notified().await;
                let Some(watched) = session.upgrade() else {
                    return;
                };
                watched.wait_for_graph_passes().await;
            }
            tracing::info!(step = ?at, "the last graph pass did not meet the fault; asking again");
        }
    });
}

#[cfg(not(feature = "automation"))]
pub(crate) fn fail_graph_pass(_tab_id: i32, _step: &str) {}

/// The configuration saves a run asked to be held (`PGG_FAULT_HOLD_SAVE`):
/// every save the hub spawns waits here until the named station, and QML
/// reads the waiting count to know the save a close lands on is out
/// (`PGG_AUTO_ACT=quit-save-held`).
#[cfg(feature = "automation")]
mod held_saves {
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    static WAITING: AtomicUsize = AtomicUsize::new(0);
    static RELEASED: AtomicBool = AtomicBool::new(false);
    static LET_GO: OnceLock<tokio::sync::Notify> = OnceLock::new();

    fn let_go() -> &'static tokio::sync::Notify {
        LET_GO.get_or_init(tokio::sync::Notify::new)
    }

    pub(super) async fn hold() {
        WAITING.fetch_add(1, Ordering::SeqCst);
        loop {
            // Armed before the flag is read, so a release that lands
            // between the read and the wait is not missed.
            let told = let_go().notified();
            if RELEASED.load(Ordering::SeqCst) {
                break;
            }
            told.await;
        }
        WAITING.fetch_sub(1, Ordering::SeqCst);
    }

    pub(super) fn release() {
        RELEASED.store(true, Ordering::SeqCst);
        let_go().notify_waiters();
    }

    pub(super) fn waiting() -> usize {
        WAITING.load(Ordering::SeqCst)
    }
}

/// Where a run asked for the saves to be held, the hold; everywhere else
/// nothing at all (`Hub::spawn_save`).
pub(crate) async fn held_save() {
    #[cfg(feature = "automation")]
    {
        if super::knobs().fault_hold_save.is_empty() {
            return;
        }
        tracing::info!(target: "bench", "fault: a configuration save is held");
        held_saves::hold().await;
    }
}

/// The station reached lets the held saves go, where it is the one the
/// run named (`harness::deadline::at`).
#[cfg(feature = "automation")]
pub(crate) fn release_saves_at(station: super::Station) {
    if super::knobs().fault_hold_save != station.slug() {
        return;
    }
    tracing::info!(target: "bench", "fault: the held saves let go at `{}`", station.slug());
    held_saves::release();
}

#[cfg(not(feature = "automation"))]
pub(crate) fn release_saves_at(_station: super::Station) {}

/// How many saves are standing at the hold.
#[cfg(feature = "automation")]
pub(crate) fn held_saves() -> usize {
    held_saves::waiting()
}
