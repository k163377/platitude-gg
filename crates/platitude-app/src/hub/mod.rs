//! Main-thread hub: tab registry, per-tab event feeds and the session sink.
//!
//! Background tasks push into `Feed` queues and wake the owning QML object
//! with a queued `drain`, which pulls on the main thread. `drain` is the
//! only method invoked through a `QmlMethodInvoker`: one lowercase word is
//! immune to camel-case conversion.

use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use platitude_core::OperationKind;
use platitude_core::details::DiffTarget;
use platitude_core::opstate::OpState;
use platitude_core::parse::diff::FilePatch;
use platitude_core::preview::FilePreview;
use platitude_core::process::{CommandEnd, GitExecutor};
use platitude_core::session::{
    DrawnGraph, FirstPass, LogRow, RefLabel, RefsSnapshot, RepoSession, SessionEvent, SessionSink,
};
use platitude_core::settings::{Settings, State, Store};
use platitude_core::stash::StashEntry;
use platitude_core::status::WorkTreeStatus;
use qtbridge::QmlMethodInvoker;

#[cfg(test)]
mod details_tests;
mod feed;
mod life;
mod msg;
#[cfg(test)]
mod plan_tests;
mod prefs;
mod saves;
mod sink;
mod tabs;

pub use feed::{Feed, Feeds, attach_feed, attached};
pub use msg::{
    CarriedStatusMsg, CloneMsg, CommandMsg, DetailsMsg, DiffMsg, GraphMsg, HeadMsg, OpProgressMsg,
    OpenMsg, PlanMsg, RefsMsg, StashList, StateMsg, StatusMsg, TabMsg,
};
pub use prefs::AvatarUrls;
pub use tabs::{from_session, listing_applied, stand_in, with_session};

/// The commit message typed into a tab and not committed yet — the one
/// thing that survives `Hub::release_tab`, since no repository can be
/// asked for it again. `amending` travels with it: an amend message put
/// back into a plain commit editor would make a second commit.
#[derive(Debug, Clone, Default)]
pub struct Draft {
    pub subject: String,
    pub body: String,
    pub amending: bool,
}

struct Tab {
    /// `None` until the tab is first in front, and again after it leaves
    /// the front (`Hub::release_tab`): only a tab being shown pays for a
    /// session and its git.
    session: Option<Arc<RepoSession>>,
    /// Sessions this tab has opened, counting from one. Tells their
    /// command-log rows apart: each session numbers its invocations from
    /// one, and the log outlives it (`CommandMsg`).
    runs: u64,
    /// The graph on screen, carried from a session let go of to the one
    /// taking its place (`platitude_core::session::DrawnGraph`); `None` at
    /// every other moment.
    drawn: Option<DrawnGraph>,
    /// The working copy the tab stands in: what a session opens, and the
    /// key of `drafts`.
    path: PathBuf,
    /// The repository's own working copy, where a tab whose copy will not
    /// open is stood back (`Hub::home_copy`). Equal to `path` when there
    /// is nowhere further back to go.
    home: PathBuf,
    feeds: Arc<Feeds>,
    /// The session's door into `feeds`, kept so a release or close can
    /// shut it (`BridgeSink::retired`): a write the close lets run on
    /// answers late, into feeds that outlive the session.
    sink: Option<Arc<sink::BridgeSink>>,
    /// Held across a release (see [`Draft`]), one per working copy this
    /// tab has stood in: a message is about what one copy has staged.
    /// `Hub::restand_tab` leaves the map as it is — the reader comes back.
    drafts: HashMap<String, Draft>,
    /// The rows a delete has taken off the screen and the write they wait
    /// on (`ops::StandIn`). Held here, not on the page: a write still out
    /// outlives the page drawing it. Reset with the session
    /// (`Hub::release_tab`).
    stand_in: crate::ops::StandIn,
}

/// Application-wide state living on the Qt main thread.
pub struct Hub {
    runtime: Option<tokio::runtime::Runtime>,
    executor: GitExecutor,
    /// Where the binary [`Hub::executor`] spawns is: the path the settings
    /// named, or what `PATH` resolves `git` to (`resolve_git`).
    git_program: String,
    /// A different git was chosen, so this run is to be replaced by one on
    /// it. Read by `main` after the window is gone — the only moment a
    /// successor can start without the two fighting over the settings
    /// files.
    restart_wanted: bool,
    tabs: HashMap<i32, Tab>,
    next_tab_id: i32,
    /// Write loops of sessions released or closed mid-write, which run on
    /// (`RepoSession::close`) — how the quit gate and the shutdown still
    /// see them. Pruned where read (`Hub::writes_settled`).
    parked_writes: Vec<tokio::task::JoinHandle<()>>,
    /// Configuration writes asked for outside any session, held until git
    /// answers and seen by the quit gate and the shutdown like
    /// `parked_writes` (`hub::saves`).
    saves: saves::Saves,
    store: Store,
    settings: Settings,
    /// What the window looks like now, and what is on disk. The flush
    /// compares the two rather than keeping a dirty flag: the UI reports
    /// the whole layout on a timer, mostly unchanged.
    state: State,
    saved_state: State,
    /// Empty unless another process holds the settings files this one
    /// would have used; then their directory (and `store` is an empty one).
    held_elsewhere: String,
}

thread_local! {
    static HUB: RefCell<Option<Hub>> = const { RefCell::new(None) };
}
