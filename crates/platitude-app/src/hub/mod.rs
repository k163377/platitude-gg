//! Main-thread hub: tab registry, per-tab event feeds and the session sink.
//!
//! Threading model: background tasks push messages into
//! `Feed` queues and wake the owning QML object with a queued `drain`
//! invocation; the object pulls its messages on the Qt main thread. The
//! only method ever invoked through a `QmlMethodInvoker` is `drain` — a
//! single lowercase word, immune to camel-case conversion accidents.

use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use platitude_core::details::DiffTarget;
use platitude_core::opstate::OpState;
use platitude_core::parse::diff::FilePatch;
use platitude_core::preview::FilePreview;
use platitude_core::process::{CommandEnd, GitExecutor};
use platitude_core::session::{
    AUTO_FETCH_OP, LogRow, RefLabel, RefsSnapshot, RepoSession, SessionEvent, SessionSink,
};
use platitude_core::settings::{Settings, State, Store};
use platitude_core::stash::StashEntry;
use platitude_core::status::WorkTreeStatus;
use qtbridge::QmlMethodInvoker;

mod feed;
mod msg;
mod prefs;
mod sink;
mod tabs;

pub use feed::{Feed, Feeds};
pub use msg::{CommandMsg, DiffMsg, GraphMsg, PickMsg, StatusMsg, TabMsg};
pub use prefs::AvatarUrls;

// The five siblings reach the imports above through `use super::*`,
// which sees what this module can see -- the block stays whole here
// rather than being dealt out five ways.

struct Tab {
    /// `None` until the tab is first looked at. Restoring a window full of
    /// tabs must not spend a `RepoSession` — and the git it spawns — on
    /// repositories nobody has asked to see yet.
    session: Option<Arc<RepoSession>>,
    /// Kept after opening too: it is the name a per-repository setting is
    /// filed under (`reapply_settings`).
    path: PathBuf,
    feeds: Arc<Feeds>,
}

/// Application-wide state living on the Qt main thread.
pub struct Hub {
    runtime: Option<tokio::runtime::Runtime>,
    executor: GitExecutor,
    tabs: HashMap<i32, Tab>,
    next_tab_id: i32,
    store: Store,
    settings: Settings,
    /// What the window looks like now, and what is already on disk. The
    /// flush compares the two rather than trusting a dirty flag: the UI
    /// reports the whole layout on a timer, so most reports say nothing
    /// new and a flag would be set by all of them.
    state: State,
    saved_state: State,
    /// Empty unless another process is already using the files this one
    /// would have used; then it names their directory (and the store this
    /// hub holds is an empty one).
    held_elsewhere: String,
}

thread_local! {
    static HUB: RefCell<Option<Hub>> = const { RefCell::new(None) };
}
