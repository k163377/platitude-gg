//! The reads and the write, spawned against a path, and what comes back.
//!
//! Two halves, kept apart so the matching can be tested without Qt or a
//! repository (`work_tests`): the transitions — an ask numbered, a save
//! numbered, an answer taken or dropped — are plain methods on the
//! model, and the spawns and the `changed()` signals are the thin wrap
//! the slots call.

use super::*;

/// One read's place: the number its answer has to carry to be shown,
/// the path it is about, and the token its commands run under.
pub(super) struct ReadTicket {
    pub(super) generation: u64,
    pub(super) path: String,
    pub(super) cancel: CancellationToken,
}

impl RepoConfigModel {
    /// Attaches the feed on the first question: a window whose reader
    /// never opens this category never has one to hear (the same shape
    /// `TabsModel` uses for the picker's answers).
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
        let Some(ticket) = self.look_at(path) else {
            return;
        };
        self.changed();
        self.spawn_read(ticket);
    }

    /// Turns the screen to `path` and numbers the read that will fill it.
    /// `None` for no path at all.
    ///
    /// The values are emptied: these are another repository's, and
    /// boxes still holding them would be offering to write one
    /// repository's identity into another. The marks belong to the
    /// save that put them there, and that save was about the
    /// repository being left — as is any save still out, whose
    /// answer is dropped by number
    /// ([`Self::absorb`]).
    pub(super) fn look_at(&mut self, path: String) -> Option<ReadTicket> {
        if path.is_empty() {
            return None;
        }
        self.repo_path = path.clone();
        self.state = "reading".into();
        self.error = String::new();
        self.local_name = String::new();
        self.local_email = String::new();
        self.effective_name = String::new();
        self.effective_email = String::new();
        self.write_unsaved = false;
        self.write_name_saved = false;
        self.write_email_saved = false;
        self.save_pending = None;
        self.write_busy = false;
        Some(self.next_read(path))
    }

    /// Numbers a read of the repository on screen again, without saying
    /// the screen is starting over: what a save just landed is still
    /// worth showing, and the boxes have values to keep standing while
    /// the answer is out. `None` while no repository is named.
    pub(super) fn ask_again(&mut self) -> Option<ReadTicket> {
        if self.repo_path.is_empty() {
            return None;
        }
        Some(self.next_read(self.repo_path.clone()))
    }

    /// The next read's place, and the end of the one before it: the ask
    /// after a read cancels that read, so one still waiting for a slot
    /// never spawns and one already running is
    /// stopped.
    fn next_read(&mut self, path: String) -> ReadTicket {
        self.retire_read();
        let cancel = CancellationToken::new();
        self.read_cancel = Some(cancel.clone());
        ReadTicket {
            generation: self.read_generation,
            path,
            cancel,
        }
    }

    /// Ends the read that is out without numbering another: its number
    /// is spent, so an answer still coming under it is refused, and one
    /// still running is stopped.
    fn retire_read(&mut self) {
        self.read_generation += 1;
        if let Some(previous) = self.read_cancel.take() {
            previous.cancel();
        }
    }

    /// Asks git again for the repository on screen (the read after a
    /// save).
    fn refresh(&mut self) {
        if let Some(ticket) = self.ask_again() {
            self.spawn_read(ticket);
        }
    }

    /// Asks git what `path` sets for itself, and what it would use there,
    /// under the ticket's number and token.
    ///
    /// Two reads: the effective level cannot say which of its records
    /// came out of this repository's own file
    /// (`config::get_regexp_local`), and both halves of the screen need
    /// an answer — the boxes hold the override, the line under them
    /// names what a commit would carry.
    fn spawn_read(&mut self, ticket: ReadTicket) {
        self.listen();
        let feed = Arc::clone(&self.feed);
        let ReadTicket {
            generation,
            path,
            cancel,
        } = ticket;
        let spawned = Hub::with(|hub| {
            let Some(handle) = hub.runtime_handle() else {
                return false;
            };
            let executor = hub.executor();
            handle.spawn(async move {
                let workdir = PathBuf::from(&path);
                let held = platitude_core::identity::load_local(&executor, &workdir, &cancel).await;
                let effective = platitude_core::identity::load(&executor, &workdir, &cancel).await;
                let msg = match (held, effective) {
                    (Ok(held), Ok(effective)) => ConfigMsg::Read {
                        generation,
                        path,
                        local_name: held.name.unwrap_or_default(),
                        local_email: held.email.unwrap_or_default(),
                        effective_name: effective.identity.name.unwrap_or_default(),
                        effective_email: effective.identity.email.unwrap_or_default(),
                    },
                    // A read the ask after this one stopped answers
                    // nothing: that ask's own answer is coming, and a
                    // failure shown for this one would put "cancelled"
                    // under the boxes of a screen that never failed.
                    (Err(e), _) | (_, Err(e)) if e.is_cancelled() => return,
                    (Err(e), _) | (_, Err(e)) => ConfigMsg::ReadFailed {
                        generation,
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
        let Some(save) = self.begin_save() else {
            return;
        };
        self.listen();
        self.changed();
        let feed = Arc::clone(&self.feed);
        let path = self.repo_path.clone();
        // Held by the hub: the screen and the tab it is about can both
        // go, and the window cannot close over a `git config` half way
        // through its pair (`hub::saves`).
        let spawned = Hub::with(|hub| {
            // On the save's handle: a `git config` is a local
            // write, waited out
            // (`Hub::save_executor`).
            let executor = hub.save_executor();
            hub.spawn_save(async move {
                let cancel = CancellationToken::new();
                let workdir = PathBuf::from(&path);
                let written = platitude_core::identity::set_local_identity(
                    &executor, &workdir, &name, &email, &cancel,
                )
                .await;
                let msg = match written {
                    // What git answers: the write reads itself back, so
                    // a half that did not land shows
                    // here.
                    Ok(written) => ConfigMsg::Written {
                        save,
                        path,
                        error: written.message,
                        name_saved: written.name_saved,
                        email_saved: written.email_saved,
                    },
                    Err(e) => ConfigMsg::Written {
                        save,
                        path,
                        error: e.to_string(),
                        name_saved: false,
                        email_saved: false,
                    },
                };
                feed.push(msg);
            })
        })
        .unwrap_or(false);
        if !spawned {
            self.save_pending = None;
            self.write_busy = false;
            self.error = "internal: runtime unavailable".into();
            self.changed();
        }
    }

    /// Accepts a save and numbers it, or refuses one: a save is already
    /// out, or no repository is named. The marks describe the save that
    /// is starting.
    pub(super) fn begin_save(&mut self) -> Option<u64> {
        if self.save_pending.is_some() || self.repo_path.is_empty() {
            return None;
        }
        self.saves_asked += 1;
        self.save_pending = Some(self.saves_asked);
        self.write_busy = true;
        self.error = String::new();
        self.write_unsaved = false;
        self.write_name_saved = false;
        self.write_email_saved = false;
        Some(self.saves_asked)
    }

    pub(super) fn take_feed(&mut self) {
        let reread = self.absorb(self.feed.drain());
        self.changed();
        if reread {
            self.refresh();
        }
    }

    /// Takes the answers that are this screen's and drops the rest.
    /// Answers whether the repository is to be read again — a save has
    /// landed, whole or half, and what the boxes should say is what git
    /// holds now.
    ///
    /// **By number.** A read answers the ask standing (`read_generation`)
    /// and nothing older: the reader who went A → B → A is waiting on the
    /// third ask, and the first one's answer about the same path is a
    /// picture of A from before B. A save answers the save out and
    /// nothing else: one asked for before the screen turned away, or
    /// before a second save, is nobody's now. And the read that follows a
    /// landed save is a new ask, so a read from before the save cannot
    /// land on the boxes the save just changed.
    pub(super) fn absorb(&mut self, batch: Vec<ConfigMsg>) -> bool {
        let mut reread = false;
        for msg in batch {
            match msg {
                ConfigMsg::Read {
                    generation, path, ..
                }
                | ConfigMsg::ReadFailed {
                    generation, path, ..
                } if generation != self.read_generation => {
                    tracing::debug!(
                        generation,
                        standing = self.read_generation,
                        path,
                        "a repository read answered an ask the screen has moved past"
                    );
                }
                ConfigMsg::Written { save, path, .. } if Some(save) != self.save_pending => {
                    tracing::debug!(
                        save,
                        pending = ?self.save_pending,
                        path,
                        "a repository save answered a save the screen has moved past"
                    );
                }
                ConfigMsg::Read {
                    local_name,
                    local_email,
                    effective_name,
                    effective_email,
                    ..
                } => {
                    // Only a read the screen asked for takes the last words
                    // down with it. **A read after a save keeps them**:
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
                ConfigMsg::ReadFailed { message, .. } => {
                    self.state = "error".into();
                    self.error = message;
                }
                ConfigMsg::Written {
                    error,
                    name_saved,
                    email_saved,
                    ..
                } => {
                    // The read out when the save landed is from before
                    // it, whatever it answers — and it can answer in this
                    // very drain, behind this message, where the re-read
                    // below has numbered nothing yet. Ended here, so what
                    // it brings is refused above like any read the screen
                    // has moved past: a failure of its own would replace
                    // git's words about the save, and a reading would put
                    // the boxes back.
                    self.retire_read();
                    self.save_pending = None;
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
        reread
    }
}
