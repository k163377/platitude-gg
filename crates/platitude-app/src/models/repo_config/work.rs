//! The reads and the write, spawned against a path, and what comes back.

use super::*;

impl RepoConfigModel {
    /// Attaches the feed on the first question rather than at startup: a
    /// window whose reader never opens this category never has one to
    /// hear (the same shape `TabsModel` uses for the picker's answers).
    fn listen(&mut self) {
        if self.attached {
            return;
        }
        self.feed.attach(self.get_qml_method_invoker());
        self.attached = true;
    }

    /// Shows `path`, from the top: whatever the boxes were holding
    /// belonged to another repository, and so did the marks beside them.
    pub(super) fn read_repo(&mut self, path: String) {
        if path.is_empty() {
            return;
        }
        self.repo_path = path.clone();
        self.state = "reading".into();
        self.error = String::new();
        // Emptied rather than left standing: these are another
        // repository's values, and boxes still holding them would be
        // offering to write one repository's identity into another.
        self.local_name = String::new();
        self.local_email = String::new();
        self.effective_name = String::new();
        self.effective_email = String::new();
        // The marks belong to the save that put them there, and that save
        // was about the repository being left.
        self.write_unsaved = false;
        self.write_name_saved = false;
        self.write_email_saved = false;
        self.changed();
        self.ask(path);
    }

    /// Asks git what the repository on screen holds now, without saying
    /// the screen is starting over: what a save just landed is still
    /// worth showing, and the boxes have values to keep standing while
    /// the answer is out.
    fn refresh(&mut self) {
        let path = self.repo_path.clone();
        if !path.is_empty() {
            self.ask(path);
        }
    }

    /// Asks git what `path` sets for itself, and what it would use there.
    ///
    /// Two reads rather than one: the effective level cannot say which of
    /// its records came out of this repository's own file
    /// (`config::get_regexp_local`), and both halves of the screen need an
    /// answer — the boxes hold the override, the line under them names
    /// what a commit would carry.
    fn ask(&mut self, path: String) {
        self.listen();
        let feed = Arc::clone(&self.feed);
        let spawned = Hub::with(|hub| {
            let Some(handle) = hub.runtime_handle() else {
                return false;
            };
            let executor = hub.executor();
            handle.spawn(async move {
                let cancel = tokio_util::sync::CancellationToken::new();
                let workdir = PathBuf::from(&path);
                let held = platitude_core::identity::load_local(&executor, &workdir, &cancel).await;
                let effective = platitude_core::identity::load(&executor, &workdir, &cancel).await;
                let msg = match (held, effective) {
                    (Ok(held), Ok(effective)) => ConfigMsg::Read {
                        path,
                        local_name: held.name.unwrap_or_default(),
                        local_email: held.email.unwrap_or_default(),
                        effective_name: effective.identity.name.unwrap_or_default(),
                        effective_email: effective.identity.email.unwrap_or_default(),
                    },
                    (Err(e), _) | (_, Err(e)) => ConfigMsg::ReadFailed {
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

    /// Writes the pair into the repository the screen is showing, where an
    /// empty box asks for that key to be taken out
    /// (`identity::set_local_identity`).
    pub(super) fn write_repo(&mut self, name: String, email: String) {
        if self.write_busy || self.repo_path.is_empty() {
            return;
        }
        self.listen();
        self.write_busy = true;
        self.error = String::new();
        // The marks describe the save that is starting, not the last one.
        self.write_unsaved = false;
        self.write_name_saved = false;
        self.write_email_saved = false;
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
                let written = platitude_core::identity::set_local_identity(
                    &executor, &workdir, &name, &email, &cancel,
                )
                .await;
                let msg = match written {
                    // What git answers, not what was typed: the write
                    // reads itself back, so a half that did not land shows
                    // here rather than passing for a finished pair.
                    Ok(written) => ConfigMsg::Written {
                        path,
                        error: written.message,
                        name_saved: written.name_saved,
                        email_saved: written.email_saved,
                    },
                    Err(e) => ConfigMsg::Written {
                        path,
                        error: e.to_string(),
                        name_saved: false,
                        email_saved: false,
                    },
                };
                feed.push(msg);
            });
            true
        })
        .unwrap_or(false);
        if !spawned {
            self.write_busy = false;
            self.error = "internal: runtime unavailable".into();
            self.changed();
        }
    }

    pub(super) fn take_feed(&mut self) {
        let mut reread = false;
        for msg in self.feed.drain() {
            // An answer for a repository the reader has already moved off
            // is not this screen's any more: the boxes are showing another
            // repository's file, and filling them from this would be
            // offering to write one repository's values into another.
            match msg {
                ConfigMsg::Read {
                    path,
                    local_name,
                    local_email,
                    effective_name,
                    effective_email,
                } => {
                    if path != self.repo_path {
                        continue;
                    }
                    // Only a read the screen asked for takes the last words
                    // down with it. **The read that follows a save must not**:
                    // that one is this model's own doing (`refresh`), and the
                    // words standing are git's account of why the save did
                    // not land — cleared here, they would be on screen for as
                    // long as one `git config` takes to answer, and the pane
                    // would fall back to a sentence of its own that says far
                    // less than git just did.
                    let asked_for = self.state == "reading";
                    self.local_name = local_name;
                    self.local_email = local_email;
                    self.effective_name = effective_name;
                    self.effective_email = effective_email;
                    self.state = "ready".into();
                    if asked_for {
                        self.error = String::new();
                    }
                }
                ConfigMsg::ReadFailed { path, message } => {
                    if path != self.repo_path {
                        continue;
                    }
                    self.state = "error".into();
                    self.error = message;
                }
                ConfigMsg::Written {
                    path,
                    error,
                    name_saved,
                    email_saved,
                } => {
                    if path != self.repo_path {
                        continue;
                    }
                    self.write_busy = false;
                    self.write_unsaved = !(name_saved && email_saved);
                    self.write_name_saved = name_saved;
                    self.write_email_saved = email_saved;
                    self.error = error;
                    // What the boxes should now say is what git holds, and
                    // a write that only half landed is exactly the case
                    // where those two differ.
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
