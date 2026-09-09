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
