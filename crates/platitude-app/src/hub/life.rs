//! The hub's own life: installing it into the main thread, reaching it,
//! and taking it down — plus the two handles everything else asks for a
//! runtime and a git with.

use super::*;

impl Hub {
    /// Installs the hub into the main thread. Call once before `QApp::run`.
    ///
    /// A run that was turned away from the settings files is handed an
    /// empty store and `held_elsewhere` names the directory it did not get.
    pub fn install(runtime: tokio::runtime::Runtime, store: Store, held_elsewhere: String) {
        let settings = store.load_settings();
        let state = store.load_state();
        tracing::info!(
            settings = ?store.settings_path(),
            state = ?store.state_path(),
            "settings store"
        );
        HUB.with(|h| {
            *h.borrow_mut() = Some(Hub {
                runtime: Some(runtime),
                executor: GitExecutor::new(),
                tabs: HashMap::new(),
                next_tab_id: 0,
                parked_writes: Vec::new(),
                store,
                settings,
                saved_state: state.clone(),
                state,
                held_elsewhere,
            });
        });
        Hub::with(Hub::forget_missing_avatars);
    }

    /// Runs `f` with the hub; logs and returns `None` when uninstalled
    /// (only possible before `install`, i.e. never during normal slots).
    pub fn with<R>(f: impl FnOnce(&mut Hub) -> R) -> Option<R> {
        HUB.with(|h| {
            let mut guard = h.borrow_mut();
            match guard.as_mut() {
                Some(hub) => Some(f(hub)),
                None => {
                    tracing::error!("hub used before install");
                    None
                }
            }
        })
    }

    /// Shuts down all sessions and the runtime. Call after `QApp::run`.
    pub fn shutdown() {
        let hub = HUB.with(|h| h.borrow_mut().take());
        if let Some(mut hub) = hub {
            hub.flush_state();
            let mut writes = std::mem::take(&mut hub.parked_writes);
            for (_, tab) in hub.tabs.drain() {
                if let Some(session) = tab.session {
                    session.close();
                    if let Some(write) = session.take_write_join() {
                        writes.push(write);
                    }
                }
            }
            if let Some(rt) = hub.runtime.take() {
                // A local write in flight is waited out, never dropped
                // with the runtime — `kill_on_drop` would end git itself,
                // mid-write. Normally instant: the window does not close
                // while one is pending (`Hub::writes_settled`), so what
                // is joined here has already ended.
                rt.block_on(async {
                    for write in writes {
                        if let Err(error) = write.await {
                            tracing::warn!(%error, "a write loop did not end cleanly");
                        }
                    }
                });
                rt.shutdown_timeout(std::time::Duration::from_secs(2));
            }
        }
    }

    pub fn runtime_handle(&self) -> Option<tokio::runtime::Handle> {
        self.runtime.as_ref().map(|r| r.handle().clone())
    }

    pub fn executor(&self) -> GitExecutor {
        self.executor.clone()
    }
}
