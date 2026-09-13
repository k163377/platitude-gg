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
        let chosen = resolve_git(&runtime, &settings.defaults.git_path);
        // The one set of slots this process runs git in, installed on
        // the handle before anything clones it: every session, screen
        // and dialog spawns through a clone of this executor, so the cap
        // the settings name is the whole application's
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
            // The ending is past the event loop, so the run's own watchdog
            // cannot reach any of it: these are what says which step a
            // process that stopped answering here was on
            // (`harness::deadline`).
            crate::harness::station(crate::harness::Station::SettingsFlush);
            hub.flush_state();
            crate::harness::station(crate::harness::Station::TabsClosing);
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
                crate::harness::station(crate::harness::Station::WritesJoining);
                rt.block_on(async {
                    for write in writes {
                        if let Err(error) = write.await {
                            tracing::warn!(%error, "a write loop did not end cleanly");
                        }
                    }
                });
                crate::harness::station(crate::harness::Station::RuntimeStopping);
                rt.shutdown_timeout(std::time::Duration::from_secs(2));
            }
            // Every session is gone with the runtime, and each took its
            // own picture files with it; this is the run's directory
            // itself, and whatever a read cut short left in it.
            crate::harness::station(crate::harness::Station::RunDirClearing);
            platitude_core::preview::remove_run_dir();
        }
    }

    pub fn runtime_handle(&self) -> Option<tokio::runtime::Handle> {
        self.runtime.as_ref().map(|r| r.handle().clone())
    }

    pub fn executor(&self) -> GitExecutor {
        self.executor.clone()
    }

    /// Where the git this run spawns actually is. The settings screen
    /// shows it behind an empty box, which is the one place "the git on
    /// `PATH`" can be said as something a reader could go and look at.
    pub fn git_program(&self) -> &str {
        &self.git_program
    }

    /// Where an empty box points: the git `PATH` resolves, which is what
    /// the next start would spawn if the settings named nothing.
    pub fn path_program(&self) -> String {
        path_program()
    }

    /// Says that this run is to be replaced by one on the git that has
    /// just been chosen. Only ever set: a restart asked for is a restart
    /// that happens, because what asked for it is a settings file already
    /// written — a window that came back on the old binary would disagree
    /// with its own settings screen.
    pub fn want_restart(&mut self) {
        self.restart_wanted = true;
    }

    pub fn restart_wanted(&self) -> bool {
        self.restart_wanted
    }
}

/// What the settings' `git_path` came to: the handle everything spawns
/// through, and where that binary is.
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

/// Where the git this app would spawn with no path named is — the one
/// `PATH` resolves, or the one behind the launcher standing on it
/// (`process::default_program_path`).
///
/// **Asked even when the settings name a binary**, because the settings
/// screen has to be able to tell an emptied box from a chosen one: empty
/// means this, and a reader who picks the very git already running must
/// not be told they picked another.
fn path_program() -> String {
    platitude_core::process::default_program_path()
        .to_string_lossy()
        .into_owned()
}

/// Which git this run spawns, settled once — before the hub exists, so
/// nothing can ever hold a handle to a different one (`Hub::executor` is
/// cloned by every session, and the first tab is opened from the same
/// handler that starts the version check).
///
/// **A named git that does not answer is fallen back on, not obeyed.**
/// Obeying it would put the window behind the missing-git gate, which has
/// no way into the settings screen — the reader would be locked out of
/// the one box that could fix the path, by the value in that box. The
/// fallback is the same reading the rules already take of an old git:
/// only a git that is nowhere at all is a gate (規約 §git が無い時・古い時).
/// Nothing on screen says it happened: what does is the settings screen
/// asking that same path again as it opens, and answering in red under
/// the box that holds it.
fn resolve_git(runtime: &tokio::runtime::Runtime, git_path: &str) -> GitChoice {
    if git_path.is_empty() {
        return GitChoice::on_path();
    }
    // One `git --version`, and only for a run whose settings name a git:
    // the window is not up yet, so this is startup time being spent
    // (CLAUDE.md §性能予算).
    let cancel = tokio_util::sync::CancellationToken::new();
    let probe = runtime.block_on(platitude_core::version::probe(git_path, &cancel));
    if probe.answered() {
        // Recorded as what is spawned for the path, not as the path: a
        // chooser opened on a Git for Windows install lands on the
        // launcher, and the executor spawns the git behind it
        // (`process::spawnable`), so the screen shows that one as the git
        // this run is on.
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
