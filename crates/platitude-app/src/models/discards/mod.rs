//! What destructive operations took away from the repository on screen
//! (破棄記録仕様.md), read off git's reflogs and Platitude GG's own record
//! when the discard log asks (`platitude_core::discards`), and brought back
//! through the tab's session.
//!
//! Off the session, as `RepoConfigModel`'s reads are: the log reads when it
//! is opened and when it is asked again, not on the session's refresh. An
//! answer is matched to its ask by number, and the next ask cancels the
//! read before it. A restore is a write, queued on the tab's session like
//! every other (`RepoSession::restore_discard`).

use std::path::PathBuf;
use std::sync::Arc;

use platitude_core::Oid;
use platitude_core::discards::Discard;
use qtbridge::{QmlObject, qobject};
use tokio_util::sync::CancellationToken;

use crate::encode::DiscardRows;
use crate::hub::{Feed, Hub};

use super::qml_register;

mod qobject;

/// What a spawned read sends back.
enum DiscardMsg {
    Read {
        generation: u64,
        found: Vec<Discard>,
    },
    Failed {
        generation: u64,
        message: String,
    },
}

pub struct DiscardModel {
    /// The entries, newest first, as QML words them (`RecoverEntries`).
    rows: DiscardRows,
    /// The entries as read, by row — what the graph draws for the one
    /// picked, and what a restore brings back.
    found: Vec<Discard>,
    /// "idle" | "reading" | "ready" | "error".
    state: String,
    /// git's words when the read failed.
    error: String,
    feed: Arc<Feed<DiscardMsg>>,
    attached: bool,
    /// The tab whose session a restore is queued on.
    tab_id: i32,
    /// The number of the read standing; an answer under any other is dropped.
    generation: u64,
    cancel: Option<CancellationToken>,
}

impl Default for DiscardModel {
    fn default() -> Self {
        Self {
            rows: DiscardRows::default(),
            found: Vec::new(),
            state: "idle".into(),
            error: String::new(),
            feed: Arc::new(Feed::default()),
            attached: false,
            tab_id: 0,
            generation: 0,
            cancel: None,
        }
    }
}

impl DiscardModel {
    fn listen(&mut self) {
        if self.attached {
            return;
        }
        self.feed.attach(self.get_qml_method_invoker());
        self.attached = true;
    }

    /// Reads `path`'s reflogs, each to `limit` lines (the graph's window), or whole.
    fn read(&mut self, path: String, limit: Option<usize>) {
        if path.is_empty() {
            return;
        }
        self.generation += 1;
        if let Some(previous) = self.cancel.take() {
            previous.cancel();
        }
        let cancel = CancellationToken::new();
        self.cancel = Some(cancel.clone());
        self.state = "reading".into();
        self.listen();
        let feed = Arc::clone(&self.feed);
        let generation = self.generation;
        let spawned = Hub::with(|hub| {
            let Some(handle) = hub.runtime_handle() else {
                return false;
            };
            let executor = hub.executor();
            handle.spawn(async move {
                let workdir = PathBuf::from(&path);
                let msg =
                    match platitude_core::discards::read_repo(&executor, &workdir, limit, &cancel)
                        .await
                    {
                        Ok(found) => DiscardMsg::Read { generation, found },
                        Err(e) if e.is_cancelled() => return,
                        Err(e) => DiscardMsg::Failed {
                            generation,
                            message: e.to_string(),
                        },
                    };
                feed.push(msg);
            });
            true
        })
        .unwrap_or(false);
        if !spawned {
            self.state = "error".into();
            self.error = "internal: runtime unavailable".into();
        }
        self.changed();
    }

    fn take_feed(&mut self) {
        for msg in self.feed.drain() {
            match msg {
                DiscardMsg::Read { generation, .. } | DiscardMsg::Failed { generation, .. }
                    if generation != self.generation =>
                {
                    tracing::debug!(
                        generation,
                        "a reflog read answered an ask the log has moved past"
                    );
                }
                DiscardMsg::Read { found, .. } => {
                    self.rows = crate::encode::discard_rows(&found);
                    self.found = found;
                    self.state = "ready".into();
                    self.error = String::new();
                }
                DiscardMsg::Failed { message, .. } => {
                    // An earlier read's rows would stay up to be picked and
                    // restored under a failure that may mean they moved.
                    self.rows = crate::encode::discard_rows(&[]);
                    self.found = Vec::new();
                    self.state = "error".into();
                    self.error = message;
                }
            }
        }
        self.changed();
    }

    fn entry(&self, index: i32) -> Option<&Discard> {
        usize::try_from(index).ok().and_then(|i| self.found.get(i))
    }

    fn index_of(&self, key: &str) -> i32 {
        if key.is_empty() {
            return -1;
        }
        self.found
            .iter()
            .position(|entry| entry_key(entry) == key)
            .and_then(|at| i32::try_from(at).ok())
            .unwrap_or(-1)
    }

    /// The entry's tips on the graph, each once, full hex — the parts'
    /// where they stand.
    fn tips_of(&self, index: i32) -> Vec<(Oid, &'static str)> {
        let mut seen = std::collections::HashSet::new();
        self.entry(index)
            .map(|entry| {
                entry
                    .parts
                    .iter()
                    .filter(|part| seen.insert(part.tip))
                    .map(|part| (part.tip, crate::encode::look_word(part.look)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Each copy part's base made for it, and the commit it stands on.
    fn stands_of(&self, index: i32) -> Vec<String> {
        self.entry(index)
            .map(|entry| {
                entry
                    .parts
                    .iter()
                    .filter_map(|part| {
                        let stands = part.stands?;
                        Some(format!(
                            "{} {} {}",
                            part.tip.to_hex(),
                            stands.made.to_hex(),
                            stands.on.to_hex()
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The commits every part of the entry would bring back.
    fn lost_of(&self, index: i32) -> Vec<String> {
        self.entry(index)
            .map(|entry| entry.lost().iter().map(Oid::to_hex).collect())
            .unwrap_or_default()
    }

    /// Queues the restore of the entry's part `part`, or of every part
    /// (`part` < 0); whether the session took it.
    fn restore(&self, index: i32, part: i32) -> bool {
        let Some(entry) = self.entry(index) else {
            return false;
        };
        let parts = match usize::try_from(part) {
            Ok(at) => entry.parts.get(at).cloned().into_iter().collect(),
            Err(_) => entry.parts.clone(),
        };
        if parts.is_empty() {
            return false;
        }
        crate::hub::from_session(self.tab_id, |session| session.restore_discard(parts))
            .flatten()
            .is_some()
    }
}

/// An entry as the same entry across reads: what was done, when, and on
/// which tips — the index moves as entries come and go.
fn entry_key(entry: &Discard) -> String {
    let tips: Vec<String> = entry.parts.iter().map(|part| part.tip.to_hex()).collect();
    format!("{:?} {} {}", entry.kind, entry.at, tips.join(","))
}

qml_register!(DiscardModel, "DiscardModel", singleton = false);
