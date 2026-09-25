//! The hub's own life: install, access, shutdown — plus the runtime and
//! git handles everything else asks for.

use super::*;

impl Hub {
    /// Installs the hub into the main thread. Call once before `QApp::run`.
    pub fn install(runtime: tokio::runtime::Runtime, store: Store, held_elsewhere: String) {
        let settings = store.load_settings();
        let state = store.load_state();
        tracing::info!(
            settings = ?store.settings_path(),
            state = ?store.state_path(),
            "settings store"
        );
        let chosen = resolve_git(&runtime, &settings.defaults.git_path);
        // Installed before anything clones the executor: every spawn goes
        // through a clone, so the cap is application-wide
        // (`platitude_core::process::Slots`).
        let slots = Arc::new(platitude_core::process::Slots::new(
            platitude_core::process::Limits::of(settings.defaults.git_concurrency),
        ));
        let executor = chosen.executor.scheduled(slots);
        HUB.with(|h| {
            *h.borrow_mut() = Some(Hub {
                runtime: Some(runtime),
                executor,
                git_program: chosen.program,
                restart_wanted: false,
                tabs: HashMap::new(),
                next_tab_id: 0,
                parked_writes: Vec::new(),
                saves: saves::Saves::default(),
                store,
                settings,
                saved_state: state.clone(),
                state,
                held_elsewhere,
            });
        });
        Hub::with(Hub::forget_missing_avatars);
    }

    /// Runs `f` with the hub; `None` (logged) before `install`.
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
            // Past the event loop the run's watchdog cannot see: the
            // stations name the step a hung shutdown was on
            // (`harness::deadline`).
            crate::harness::station(crate::harness::Station::SettingsFlush);
            hub.flush_state();
            crate::harness::station(crate::harness::Station::TabsClosing);
            let mut writes = std::mem::take(&mut hub.parked_writes);
            // Joined with the sessions' writes (`hub::saves`).
            let saves = hub.saves.take_all();
            let saves_joined = saves.len();
            writes.extend(saves);
            for (_, tab) in hub.tabs.drain() {
                if let Some(session) = tab.session {
                    session.close();
                    if let Some(write) = session.take_write_join() {
                        writes.push(write);
                    }
                }
            }
            if let Some(rt) = hub.runtime.take() {
                // Waited out: `kill_on_drop` would end git mid-write.
                // Normally already over — the window does not close while
                // one is pending (`Hub::writes_settled`).
                crate::harness::station(crate::harness::Station::WritesJoining);
                rt.block_on(async {
                    for write in writes {
                        if let Err(error) = write.await {
                            tracing::warn!(%error, "a write loop did not end cleanly");
                        }
                    }
                });
                // Read by the `quit-save-held` run.
                crate::harness::said(&format!("writes_joined saves={saves_joined}"));
                crate::harness::station(crate::harness::Station::RuntimeStopping);
                rt.shutdown_timeout(std::time::Duration::from_secs(2));
            }
            // Sessions took their own picture files; this is the run's
            // directory and whatever a cut-short read left in it.
            crate::harness::station(crate::harness::Station::RunDirClearing);
            platitude_core::preview::remove_run_dir();
        }
    }

    pub fn runtime_handle(&self) -> Option<tokio::runtime::Handle> {
        self.runtime.as_ref().map(|r| r.handle().clone())
    }

    /// Runs `save` — a configuration write outside any session — and holds
    /// it until it ends (`hub::saves`). `false` with no runtime.
    pub fn spawn_save(
        &mut self,
        save: impl std::future::Future<Output = ()> + Send + 'static,
    ) -> bool {
        let Some(handle) = self.runtime_handle() else {
            return false;
        };
        // Holds only where a run asked for it (`harness::held_save`).
        self.saves.hold(handle.spawn(async move {
            crate::harness::held_save().await;
            save.await;
        }));
        true
    }

    pub fn executor(&self) -> GitExecutor {
        self.executor.clone()
    }

    /// The handle a configuration save runs on: the application's git with
    /// no stock time budget, as the local write lane (`hub::saves`).
    pub fn save_executor(&self) -> GitExecutor {
        saves::executor_for(&self.executor)
    }

    /// Where the git this run spawns is — what the settings screen shows
    /// behind an empty box.
    pub fn git_program(&self) -> &str {
        &self.git_program
    }

    /// The git an empty box means: what the next start would spawn with
    /// no path set.
    pub fn path_program(&self) -> String {
        path_program()
    }

    /// Marks this run for replacement by one on the newly chosen git.
    /// Never unset: the settings file is already written, and a window
    /// left on the old binary would disagree with its own settings screen.
    pub fn want_restart(&mut self) {
        self.restart_wanted = true;
    }

    pub fn restart_wanted(&self) -> bool {
        self.restart_wanted
    }
}

/// What the settings' `git_path` came to: the handle, and its binary.
struct GitChoice {
    executor: GitExecutor,
    program: String,
}

impl GitChoice {
    fn on_path() -> Self {
        Self {
            executor: GitExecutor::new(),
            program: path_program(),
        }
    }
}

/// The git spawned with no path named — `PATH`'s, or the one behind the
/// launcher there (`process::default_program_path`). Asked even when the
/// settings name a binary: the settings screen compares an emptied box
/// against it.
fn path_program() -> String {
    platitude_core::process::default_program_path()
        .to_string_lossy()
        .into_owned()
}

/// Which git this run spawns, settled once before the hub exists, so
/// nothing can hold a handle to a different one (`Hub::executor` is
/// cloned by every session).
///
/// A named git that does not answer falls back to `PATH`'s (デザイン規約
/// §設定の画面): obeying it would put the window behind the missing-git
/// gate, which has no way to the settings box that could fix the path.
fn resolve_git(runtime: &tokio::runtime::Runtime, git_path: &str) -> GitChoice {
    if git_path.is_empty() {
        return GitChoice::on_path();
    }
    // Startup time (CLAUDE.md §性能予算): one `git --version`, only when
    // the settings name a git.
    let cancel = tokio_util::sync::CancellationToken::new();
    let probe = runtime.block_on(platitude_core::version::probe(git_path, &cancel));
    if probe.answered() {
        // Recorded as what is actually spawned: a Git for Windows
        // launcher resolves to the git behind it (`process::spawnable`),
        // and that is what the screen shows.
        let program = platitude_core::process::spawnable(std::path::Path::new(git_path));
        let program = program.to_string_lossy().into_owned();
        tracing::info!(
            path = git_path,
            program = %program,
            version = probe.version(),
            "git from settings"
        );
        return GitChoice {
            executor: GitExecutor::with_program(program.as_str()),
            program,
        };
    }
    tracing::warn!(
        path = git_path,
        outcome = ?probe,
        "the git named in the settings did not answer; running the one on PATH"
    );
    GitChoice::on_path()
}
