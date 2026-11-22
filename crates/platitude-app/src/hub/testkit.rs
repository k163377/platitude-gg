//! A hub for tests: built bare, or installed on the test's own thread for a
//! model that reaches it through `Hub::with` (`hub::stage_ahead` and the
//! other per-tab owners). Nothing here asks git anything: the tabs it
//! reserves have no session, or one whose runtime nobody drives
//! (`open_installed_tab`) — opened, so the hub's own way of letting go of
//! a session runs, with none of its reads or writes ever spawned.

use super::*;

/// A hub whose runtime nobody drives, numbering its tabs after
/// `next_tab_id`: the pictures are held process-wide, so each test numbers
/// its tabs apart from the others' (`preview_images`).
pub(super) fn bare(next_tab_id: i32) -> Hub {
    Hub {
        runtime: Some(
            tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("a runtime"),
        ),
        executor: GitExecutor::new(),
        git_program: String::new(),
        restart_wanted: false,
        tabs: HashMap::new(),
        next_tab_id,
        parked_writes: Vec::new(),
        saves: saves::Saves::default(),
        store: Store::ephemeral(),
        settings: Settings::default(),
        saved_state: State::default(),
        state: State::default(),
        held_elsewhere: String::new(),
    }
}

/// Installs a bare hub on this thread with one tab reserved, and answers
/// that tab's id. The hub is the thread's own (`HUB` is thread-local), and
/// each test runs on a thread of its own; the tab's id is the process's,
/// numbered apart from every other installed hub's and from the ones `bare`
/// hubs are given, as the pictures held by id are (`preview_images`).
pub(crate) fn installed_tab() -> i32 {
    static NEXT: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(10_000);
    let mut hub = bare(NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    let repo = PathBuf::from("repo");
    let id = hub.reserve_tab(repo.clone(), repo).expect("a tab");
    HUB.with(|h| *h.borrow_mut() = Some(hub));
    id
}

/// Opens the installed tab's session on the runtime nobody drives: the
/// session stands, and what it would read or write never starts.
pub(crate) fn open_installed_tab(id: i32) {
    Hub::with(|hub| hub.ensure_open(id));
}

/// What the installed hub has queued on tab `id`'s own feed — the
/// messages its tab drains (`RepoTab::drain`), taken off the feed.
pub(crate) fn tab_feed_drained(id: i32) -> Vec<TabMsg> {
    Hub::with(|hub| hub.tabs.get(&id).map(|tab| tab.feeds.tab.drain()))
        .flatten()
        .unwrap_or_default()
}
