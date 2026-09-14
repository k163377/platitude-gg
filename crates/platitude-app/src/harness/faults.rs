//! The fault the harness raises inside a graph pass — the one state of
//! the band a demo repository cannot be walked into — and the seam it
//! reaches the session through ([`PassHooks`]).
//!
//! A build without the harness hands the session nothing: the seam is
//! answered `None`, the session holds no door, and the type below is not
//! compiled at all.

use std::sync::Arc;

use platitude_core::session::PassHooks;
#[cfg(feature = "automation")]
use platitude_core::session::PassStep;

/// The fault standing over every graph pass this process runs.
///
/// **One for the process rather than one per tab.** What is driven into
/// this is being photographed, not used (`PGG_AUTO_ACT=graph-stale` /
/// `graph-stopped`), and a run photographs one window on one repository;
/// a fault raised for a tab is a fault raised for the run. It stands once
/// raised — a fault one pass could lift would be a race, the pass already
/// walking taking it and the pass asked for next succeeding, so the mark
/// would go up and straight back down (`PassHooks::fault`).
#[cfg(feature = "automation")]
#[derive(Default)]
struct GraphFaults {
    at: std::sync::Mutex<Option<PassStep>>,
    /// Whether every pass must walk as one that began before the first
    /// status did ([`PassHooks::holds_back_the_working_tree_row`]).
    /// Raised before the repository is opened, so the opening's own walk
    /// is one of them, and lowered by the run when it has read what that
    /// pass left the page holding.
    holds_the_row: std::sync::atomic::AtomicBool,
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
}

#[cfg(feature = "automation")]
impl PassHooks for GraphFaults {
    /// Nothing: a screen is driven by a fault that stands, never by
    /// something run once inside a pass — that door is the tests'.
    fn before(&self, _at: PassStep) {}

    fn fault(&self, at: PassStep) -> Option<platitude_core::GitError> {
        let standing = *self.lock();
        (standing == Some(at)).then(|| platitude_core::GitError::UnexpectedOutput {
            command: "git log".to_string(),
            message: "the graph walk was made to fail".to_string(),
        })
    }

    fn holds_back_the_working_tree_row(&self) -> bool {
        self.holds_the_row.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Raises the hold before anything is opened, off the run's own word
/// (`knobs::holds_the_working_tree_row`).
#[cfg(feature = "automation")]
pub(crate) fn hold_the_working_tree_row() {
    GraphFaults::standing()
        .holds_the_row
        .store(true, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(not(feature = "automation"))]
pub(crate) fn hold_the_working_tree_row() {}

/// Lowers it again and asks the tab's session for the pass that carries
/// the row, answering whether the hold had been up — the run reads the
/// page on either side of this call, and the difference between the two
/// is what it is about (`GraphModel.letTheWorkingTreeRowThrough`).
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
/// answering whether it was up (`GraphModel.walkAgainWhileHeld`).
///
/// **What a write leaves behind does not bring one on its own.** A
/// rebuild follows a read that moved (`session::refresh`), and a replay
/// that stops moves no branch — what moves is this window's own row
/// appearing, which is the very thing the hold takes away. So the pass a
/// landing owed by a stopped operation has to turn down is asked for
/// here.
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

/// A build without the harness hands the session no door at all.
#[cfg(not(feature = "automation"))]
pub(crate) fn pass_hooks() -> Option<Arc<dyn PassHooks>> {
    None
}

/// Every graph pass that reaches the step `step` names fails there from
/// here on, in place of the walk it would have made, and one is asked
/// for of the tab's session in the same call (`GraphModel.failGraphPass`
/// — `swapping` for the off-screen rebuild, anything else for the
/// stream; the words are the harness QML's, `WindowBadgeActs`).
#[cfg(feature = "automation")]
pub(crate) fn fail_graph_pass(tab_id: i32, step: &str) {
    let at = if step == "swapping" {
        PassStep::Swapping
    } else {
        PassStep::Streaming
    };
    *GraphFaults::standing().lock() = Some(at);
    crate::hub::with_session(tab_id, |s| match at {
        // Off screen, so the whole graph is left standing and goes out
        // of date where it is.
        PassStep::Swapping => s.refresh_log(),
        // The column is emptied first, so the walk stops with rows
        // missing.
        PassStep::Streaming => s.restart_log(),
    });
}

/// A build without the harness has no fault to raise, and raises none.
#[cfg(not(feature = "automation"))]
pub(crate) fn fail_graph_pass(_tab_id: i32, _step: &str) {}

/// The configuration saves a run asked to be held (`PGG_FAULT_HOLD_SAVE`):
/// every save the hub spawns waits here until the station the knob names
/// is reached, and the count of the ones waiting is what QML reads to
/// know the save a close is about to land on is provably out
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

/// A build without the harness holds no save, and has none to let go.
#[cfg(not(feature = "automation"))]
pub(crate) fn release_saves_at(_station: super::Station) {}

/// How many saves are standing at the hold.
#[cfg(feature = "automation")]
pub(crate) fn held_saves() -> usize {
    held_saves::waiting()
}
