//! What the drain tests hand a tab: feed messages as the bridge carries
//! them, and a write's answer settled straight in.

use super::*;
use platitude_core::OperationKind as K;

/// The id the queue gave the press under test, and one it gave somebody else.
pub(super) const OURS: u64 = 7;
pub(super) const SOMEBODY_ELSE: u64 = 8;

/// One answer as the bridge carries it.
pub(super) fn reported(
    id: u64,
    kind: K,
    error: &str,
    report: Option<platitude_core::WriteReport>,
) -> TabMsg {
    TabMsg::WriteState {
        id,
        kind,
        running: false,
        error: error.into(),
        report,
        head_seq: 0,
        reads_from: 0,
    }
}

pub(super) fn answered(id: u64, kind: K, error: &str) -> TabMsg {
    reported(id, kind, error, None)
}

/// The write starting — none of its three boundaries (`session::write`).
pub(super) fn started(id: u64, kind: K) -> TabMsg {
    TabMsg::WriteState {
        id,
        kind,
        running: true,
        error: String::new(),
        report: None,
        head_seq: 0,
        reads_from: 0,
    }
}

/// A message that is no write's answer, for a drain that carries none.
pub(super) fn no_answer() -> TabMsg {
    TabMsg::MergeTools {
        names: Vec::new(),
        settled: true,
    }
}

/// Write 1's answer, settled into `tab` without the feed, with no HEAD
/// report or listing stamp behind it.
pub(super) fn settle(
    tab: &mut RepoTab,
    kind: K,
    error: impl Into<String>,
    report: Option<platitude_core::WriteReport>,
) {
    tab.settle_write(1, kind, error.into(), report, 0, 0);
}
