//! Reading a session's graph back out of its events, the way the UI
//! model does.

use std::collections::BTreeMap;

use platitude_core::session::{LogRow, RefLabel, SessionEvent};

/// One row as a consumer of the event stream ends up showing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeenRow {
    pub oid_hex: String,
    pub labels: Vec<RefLabel>,
}

/// The same replay as [`replay_graph`], keeping the whole row — the lane
/// geometry a chip-level view drops. Chips are not corrected here: a test
/// reading lanes is asking what the walk drew.
pub fn replay_rows(events: &[SessionEvent]) -> BTreeMap<u32, LogRow> {
    let mut generation = 0;
    let mut rows: BTreeMap<u32, LogRow> = BTreeMap::new();
    for event in events {
        match event {
            SessionEvent::LogStarted { generation: g } if *g > generation => {
                generation = *g;
                rows.clear();
            }
            SessionEvent::LogChunk {
                generation: g,
                rows: chunk,
            } if *g == generation => {
                rows.extend(chunk.iter().map(|r| (r.row, r.clone())));
            }
            SessionEvent::LogReplaced {
                generation: g,
                rows: fresh,
                ..
            } if *g > generation => {
                generation = *g;
                rows = fresh.iter().map(|r| (r.row, r.clone())).collect();
            }
            _ => {}
        }
    }
    rows
}

/// Replays the graph events the way the UI model does (`GraphModel::drain`
/// in platitude-app), keyed by row number, generation rules included: a
/// message about a replaced graph is dropped.
///
/// Chips are corrected after the walk (`LabelsChanged`, e.g. from a fetch
/// that moves no ref this repository holds), so reading only the walk
/// would miss them.
pub fn replay_graph(events: &[SessionEvent]) -> BTreeMap<u32, SeenRow> {
    fn take(batch: &[LogRow], into: &mut BTreeMap<u32, SeenRow>) {
        for r in batch {
            into.insert(
                r.row,
                SeenRow {
                    oid_hex: r.oid_hex.clone(),
                    labels: r.labels.clone(),
                },
            );
        }
    }
    let mut generation = 0;
    let mut rows: BTreeMap<u32, SeenRow> = BTreeMap::new();
    for event in events {
        match event {
            SessionEvent::LogStarted { generation: g } if *g > generation => {
                generation = *g;
                rows.clear();
            }
            SessionEvent::LogChunk {
                generation: g,
                rows: chunk,
            } if *g == generation => take(chunk, &mut rows),
            SessionEvent::LogReplaced {
                generation: g,
                rows: fresh,
                ..
            } if *g > generation => {
                generation = *g;
                rows.clear();
                take(fresh, &mut rows);
            }
            SessionEvent::LabelsChanged {
                generation: g,
                rows: changed,
            } if *g == generation => {
                for (row, labels) in changed {
                    if let Some(seen) = rows.get_mut(row) {
                        seen.labels = labels.clone();
                    }
                }
            }
            _ => {}
        }
    }
    rows
}
