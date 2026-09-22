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

/// The commit message typed into a tab and not committed yet.
///
/// The one thing a tab holds that no repository can be asked for again,
/// which is why it survives the release everything else goes through
/// (`Hub::release_tab`). The amend flag travels with the words because it
/// says what pressing the button under them would do: a message written
/// for an amend, put back into a plain commit editor, would make a
/// second commit of its own.
#[derive(Debug, Clone, Default)]
pub struct Draft {
    pub subject: String,
    pub body: String,
    pub amending: bool,
}

struct Tab {
    /// `None` until the tab is first looked at, and again after it leaves
    /// the front. Restoring a window full of tabs spends a `RepoSession`
    /// — and the git it spawns — on the repository it is asked to show,
    /// and a tab that has been switched away from goes back to costing
    /// the same nothing (`Hub::release_tab`).
    session: Option<Arc<RepoSession>>,
    /// How many sessions this tab has opened, counting from one. The
    /// command log outlives them — a tab standing in another working
    /// copy keeps its rows — and every session numbers its invocations
    /// from one, so this is what tells two of them apart
    /// (`CommandMsg`).
    runs: u64,
    /// The graph the page on this tab is showing, between the session
    /// that drew it being let go of and the one taking its place
    /// (`platitude_core::session::DrawnGraph`). `None` at every other
    /// moment — a page that goes down takes its graph with it, and one
    /// that stays hands this straight on.
    drawn: Option<DrawnGraph>,
    /// Kept after opening too: it is the name a per-repository setting is
    /// filed under (`reapply_settings`).
    path: PathBuf,
    /// The repository's own working copy — where a tab whose copy will
    /// not open is stood back (`Hub::home_copy`). Equal to `path` for a
    /// tab standing in that copy already, which is what says there is
    /// nowhere further back to go.
    home: PathBuf,
    feeds: Arc<Feeds>,
    /// The session's own door into those feeds, kept so a release or
    /// close can shut it (`BridgeSink::retired`): the writes a close lets
    /// run on answer late, and the feeds outlive the session they were
    /// speaking for. `None` for a tab with no session.
    sink: Option<Arc<sink::BridgeSink>>,
    /// Held across a release, because it is the only thing here that
    /// cannot be read again. **One per working copy this tab has stood
    /// in** (`Tab::path` is the key): a message written for what is
    /// staged in one copy says nothing about another, and standing the
    /// tab in a sibling copy is a gesture the reader comes back from
    /// (`Hub::restand_tab`, which leaves this map where it is — the tab
    /// is the same one).
    drafts: HashMap<String, Draft>,
    /// The rows a delete has taken off the screen and the write they are
    /// waiting on (`ops::StandIn`). Held here for the reason the draft is:
    /// a page is built for the tab in front and taken down behind it, and
    /// an operation that is still out is not over because the component
    /// drawing it went away. Let go with the session, which is the one
    /// thing that does end it (`Hub::release_tab`).
    stand_in: crate::ops::StandIn,
}

/// Application-wide state living on the Qt main thread.
pub struct Hub {
    runtime: Option<tokio::runtime::Runtime>,
    executor: GitExecutor,
    /// Where the binary [`Hub::executor`] spawns actually is: the path the
    /// settings named, or what `PATH` resolves `git` to (`resolve_git`).
    /// Settled at install with the executor, because it describes that
    /// same handle.
    git_program: String,
    /// A different git was chosen and answered, so this run is to be
    /// replaced by one on it. Read by `main` after the window is gone —
    /// which is the only moment a process can start its own successor
    /// without the two fighting over the settings files.
    restart_wanted: bool,
    tabs: HashMap<i32, Tab>,
    next_tab_id: i32,
    /// Write loops of closed sessions that still had local writes to
    /// finish — a tab released or closed mid-write lets the write run on
    /// (`RepoSession::close`), and these are how the quit gate and the
    /// shutdown still see it. Finished handles are pruned where they are
    /// read (`Hub::writes_settled`).
    parked_writes: Vec<tokio::task::JoinHandle<()>>,
    /// The configuration writes the application asked for outside any
    /// session — the identity screens' — held until git has answered,
    /// and seen by the quit gate and the shutdown the way the writes
    /// above are (`hub::saves`).
    saves: saves::Saves,
    store: Store,
    settings: Settings,
    /// What the window looks like now, and what is already on disk. The
    /// flush compares the two: the UI reports the whole layout on a
    /// timer, so most reports say nothing new and a flag would be set by
    /// all of them.
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
