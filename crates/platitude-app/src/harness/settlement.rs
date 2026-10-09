//! The page's settling query: reads owned by the session and the graph generation the UI holds.

/// Automation: samples the session's read boundary for `PageSettled`.
/// Stands in every build, as fail_graph_pass does; the non-automation stub below returns true without a session.
/// A missing session has no reads to await: a released tab or one being re-stood is covered by the page's lifecycle
/// terms, rather than held here waiting for a session that may never return.
#[cfg(feature = "automation")]
pub(crate) fn readings_settled(tab_id: i32, generation: u64) -> bool {
    crate::hub::from_session(tab_id, |session| session.graph_settled_at(generation)).unwrap_or(true)
}

/// Automation: the shipping half of the same query. Stands in every build, as fail_graph_pass does.
/// Without automation no sampler drives this query, so the stub
/// returns true without reaching the hub or core: no harness reading is owed.
#[cfg(not(feature = "automation"))]
pub(crate) fn readings_settled(_tab_id: i32, _generation: u64) -> bool {
    true
}
