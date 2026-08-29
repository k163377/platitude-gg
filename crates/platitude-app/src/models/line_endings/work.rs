//! The reads and the write, spawned against a path, and what comes back.

use platitude_core::eol::setting::{self, AutoCrlf, ConfigScope};

use super::*;

impl LineEndingsModel {
    /// Attaches the feed on the first question rather than at startup: a
    /// window whose reader never opens this chapter never has one to hear
    /// (the same shape `RepoConfigModel` uses).
    fn listen(&mut self) {
        if self.attached {
            return;
        }
        self.feed.attach(self.get_qml_method_invoker());
        self.attached = true;
    }

    /// Which of git's files this stands at. `"local"` is the only word
    /// that reaches the repository's own file — anything else is the
    /// user's own, which is the level the screen opens on.
    fn scope(&self) -> ConfigScope {
        if self.scope == "local" {
            ConfigScope::Local
        } else {
            ConfigScope::Global
        }
    }

    /// Shows `path`, from the top: whatever the field was holding belonged
    /// to another repository.
    ///
    /// An empty path is the application's own directory, which is where
    /// git resolves the user's own configuration — so the global level
    /// asks with one and never has to be told a repository.
    pub(super) fn read_at(&mut self, path: String) {
        self.repo_path = path;
        self.state = "reading".into();
        self.error = String::new();
        // Emptied rather than left standing: this is another repository's
        // value, and a field still holding it would be offering to write
        // one repository's setting into another.
        self.held = String::new();
        self.effective = String::new();
        // A write still out belongs to that repository too — its answer
        // will be dropped by the path check, so waiting for it here would
        // wait forever, with every later save refused.
        self.busy = false;
        self.changed();
        self.ask();
    }

    /// Asks git again without saying the screen is starting over: what a
    /// pick just landed is still worth showing, and the field has a value
    /// to keep standing while the answer is out.
    ///
    /// Nothing to ask before the screen has named a level: `"idle"` is a
    /// model no chapter has looked at yet.
    pub(super) fn refresh(&mut self) {
        if self.state != "idle" {
            self.ask();
        }
    }

    /// Asks git what this level sets, and — for a repository — what it
    /// would use there.
    ///
    /// Two reads rather than one at the repository level: the effective
    /// level cannot say which of its records came out of that repository's
    /// own file (`config::get_regexp_at`), and both halves of the chapter
    /// need an answer — the field holds the override, the line under it
    /// names what git is doing right now. The global level asks once: what
    /// it would resolve to outside any repository is what it holds, since
    /// the only file under it is the machine's, and a global value stands
    /// over that one anyway.
    fn ask(&mut self) {
        self.listen();
        let feed = Arc::clone(&self.feed);
        let path = self.repo_path.clone();
        let scope = self.scope();
        let spawned = Hub::with(|hub| {
            let Some(handle) = hub.runtime_handle() else {
                return false;
            };
            let executor = hub.executor();
            handle.spawn(async move {
                let cancel = tokio_util::sync::CancellationToken::new();
                let workdir = workdir_of(&path);
                let held = setting::held(&executor, &workdir, scope, &cancel).await;
                let effective = if scope == ConfigScope::Local {
                    setting::effective(&executor, &workdir, &cancel)
                        .await
                        .map(spelled)
                } else {
                    Ok(String::new())
                };
                let msg = match (held, effective) {
                    (Ok(held), Ok(effective)) => EolMsg::Read {
                        path,
                        held: spelled(held),
                        effective,
                    },
                    (Err(e), _) | (_, Err(e)) => EolMsg::ReadFailed {
                        path,
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
            self.changed();
        }
    }

    /// Writes `value` into this level, where an empty value asks for the
    /// key to be taken out (`eol::setting::set`).
    pub(super) fn write_value(&mut self, value: String) {
        if self.busy {
            return;
        }
        self.listen();
        self.busy = true;
        self.error = String::new();
        self.changed();
        let feed = Arc::clone(&self.feed);
        let path = self.repo_path.clone();
        let scope = self.scope();
        let spawned = Hub::with(|hub| {
            let Some(handle) = hub.runtime_handle() else {
                return false;
            };
            let executor = hub.executor();
            handle.spawn(async move {
                let cancel = tokio_util::sync::CancellationToken::new();
                let workdir = workdir_of(&path);
                let wanted = AutoCrlf::spoken(&value);
                let written = setting::set(&executor, &workdir, scope, wanted, &cancel).await;
                let msg = match written {
                    // What git answers, not what was picked: the write
                    // reads itself back, so a value that did not land
                    // shows here rather than passing for a finished one.
                    Ok(written) => EolMsg::Written {
                        path,
                        error: written.message,
                    },
                    Err(e) => EolMsg::Written {
                        path,
                        error: e.to_string(),
                    },
                };
                feed.push(msg);
            });
            true
        })
        .unwrap_or(false);
        if !spawned {
            self.busy = false;
            self.error = "internal: runtime unavailable".into();
            self.changed();
        }
    }

    pub(super) fn take_feed(&mut self) {
        let mut reread = false;
        for msg in self.feed.drain() {
            // An answer for a repository the reader has already moved off
            // is not this chapter's any more: the field is showing another
            // repository's file, and filling it from this would be
            // offering to write one repository's setting into another.
            match msg {
                EolMsg::Read {
                    path,
                    held,
                    effective,
                } => {
                    if path != self.repo_path {
                        continue;
                    }
                    // Only a read the screen asked for takes the last
                    // words down with it. **The read that follows a pick
                    // must not**: that one is this model's own doing
                    // (`refresh`), and the words standing are git's account
                    // of why the pick did not land (the shape
                    // `RepoConfigModel` keeps for the identity pair).
                    let asked_for = self.state == "reading";
                    self.held = held;
                    self.effective = effective;
                    self.state = "ready".into();
                    if asked_for {
                        self.error = String::new();
                    }
                }
                EolMsg::ReadFailed { path, message } => {
                    if path != self.repo_path {
                        continue;
                    }
                    self.state = "error".into();
                    self.error = message;
                }
                EolMsg::Written { path, error } => {
                    if path != self.repo_path {
                        continue;
                    }
                    self.busy = false;
                    self.error = error;
                    // What the field should now say is what git holds, and
                    // a write that did not take is exactly the case where
                    // those two differ.
                    reread = true;
                }
            }
        }
        self.changed();
        if reread {
            self.refresh();
            // After the re-read is asked for, so a chapter woken by this
            // is asking git at the same time rather than behind us.
            self.wrote();
        }
    }
}

/// Where the reads and the write run. An empty path is the application's
/// own directory, which outside a repository is exactly the user's own
/// configuration — what the global level is about (`AppBackend`'s identity
/// read reaches it the same way).
fn workdir_of(path: &str) -> PathBuf {
    if path.is_empty() {
        super::super::app_backend::app_workdir()
    } else {
        PathBuf::from(path)
    }
}

/// git's own spelling, or empty for a level that sets nothing — which is
/// the same thing the empty row asks for, so the field and the write agree
/// on one vocabulary.
fn spelled(value: Option<AutoCrlf>) -> String {
    value.map(AutoCrlf::spelled).unwrap_or_default().to_string()
}
