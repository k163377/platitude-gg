//! The reads and the write, spawned against a path, and what comes back.

use platitude_core::eol::setting::{self, AutoCrlf, ConfigScope};

use super::*;

impl LineEndingsModel {
    /// Attaches the feed on the first question, so a window that never
    /// opens this chapter never holds one.
    fn listen(&mut self) {
        if self.attached {
            return;
        }
        self.feed.attach(self.get_qml_method_invoker());
        self.attached = true;
    }

    /// Shows `path`, from the top.
    pub(super) fn read_at(&mut self, path: String) {
        self.repo_path = path;
        self.state = "reading".into();
        self.error = String::new();
        // Emptied: the old repository's value in the field would offer to
        // write it into this one.
        self.held = String::new();
        self.effective = String::new();
        // A write still out is the old repository's: the path check drops
        // its answer, so waiting for it would refuse every later save.
        self.busy = false;
        self.changed();
        self.ask();
    }

    /// Asks git again without starting the screen over — the field keeps
    /// its value while the answer is out. Nothing to ask while `"idle"`.
    pub(super) fn refresh(&mut self) {
        if self.state != "idle" {
            self.ask();
        }
    }

    /// Asks git what that repository's own file sets, and what git would
    /// use there — two reads, since the effective level cannot say which
    /// of its records came from that file.
    fn ask(&mut self) {
        self.listen();
        let feed = Arc::clone(&self.feed);
        let path = self.repo_path.clone();
        let spawned = Hub::with(|hub| {
            let Some(handle) = hub.runtime_handle() else {
                return false;
            };
            let executor = hub.executor();
            handle.spawn(async move {
                let cancel = tokio_util::sync::CancellationToken::new();
                let workdir = PathBuf::from(&path);
                let held = setting::held(&executor, &workdir, ConfigScope::Local, &cancel).await;
                let effective = setting::effective(&executor, &workdir, &cancel)
                    .await
                    .map(spelled);
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

    /// `save`'s body (`eol::setting::set`).
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
        let spawned = Hub::with(|hub| {
            let Some(handle) = hub.runtime_handle() else {
                return false;
            };
            let executor = hub.executor();
            handle.spawn(async move {
                let cancel = tokio_util::sync::CancellationToken::new();
                let workdir = PathBuf::from(&path);
                let wanted = AutoCrlf::spoken(&value);
                let written =
                    setting::set(&executor, &workdir, ConfigScope::Local, wanted, &cancel).await;
                let msg = match written {
                    // The write reads itself back, so a value that did
                    // not land shows here.
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
            // An answer for a repository the reader has moved off is
            // dropped, for the reason `read_at` empties the field.
            match msg {
                EolMsg::Read {
                    path,
                    held,
                    effective,
                } => {
                    if path != self.repo_path {
                        continue;
                    }
                    // Only a read the screen asked for clears `error`; a
                    // read after a pick (`refresh`) keeps git's account of
                    // why the pick did not land.
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
                    // Re-read: a write that did not take leaves the field
                    // saying other than git holds.
                    reread = true;
                }
            }
        }
        self.changed();
        if reread {
            self.refresh();
        }
    }
}

/// git's own spelling, or empty where nothing is set — the value the empty
/// row writes, so the field and the write share one vocabulary.
fn spelled(value: Option<AutoCrlf>) -> String {
    value.map(AutoCrlf::spelled).unwrap_or_default().to_string()
}
