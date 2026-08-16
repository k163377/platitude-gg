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

/// Replays the graph events the way the UI model does (`GraphModel::drain`
/// in platitude-app), keyed by row number.
///
/// Rows arrive with the walk and their chips are corrected afterwards: a
/// fetch that moves no ref this repository holds rebuilds nothing, so what
/// the remote turned out to have reaches the graph as `LabelsChanged`
/// against rows already on screen. Reading only the walk would miss it.
///
/// The generation rules are part of the model and are kept here: a message
/// about a graph that has been replaced is dropped rather than applied to
/// whatever now sits at those row numbers.
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
