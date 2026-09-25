//! The reads and the write, spawned against a path, and what comes back.
//!
//! The transitions (asks and saves numbered, answers taken or dropped)
//! are plain methods so `work_tests` can drive them without Qt or a
//! repository; the spawns and `changed()` are the thin wrap around them.

use super::*;

/// One read as asked: its answer has to carry `generation` to be shown.
pub(super) struct ReadTicket {
    pub(super) generation: u64,
    pub(super) path: String,
    pub(super) cancel: CancellationToken,
}

impl RepoConfigModel {
    /// Attaches the feed on the first ask, so a window that never opens
    /// this category never holds one.
    fn listen(&mut self) {
        if self.attached {
            return;
        }
        self.feed.attach(self.get_qml_method_invoker());
        self.attached = true;
    }

    /// Shows `path`, from the top ([`Self::look_at`]).
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
    /// Empties the values (boxes still holding them would offer to write
    /// one repository's identity into another) and the marks, and lets go
    /// of any save still out — its answer, about the repository being
    /// left, is then dropped by [`Self::absorb`].
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

    /// Numbers another read of the repository on screen without starting
    /// the screen over — the boxes and a landed save's marks stay while it
    /// is out. `None` while no repository is named.
    pub(super) fn ask_again(&mut self) -> Option<ReadTicket> {
        if self.repo_path.is_empty() {
            return None;
        }
        Some(self.next_read(self.repo_path.clone()))
    }

    /// The next read's ticket; the read before it is retired.
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

    /// Ends the read that is out without numbering another: an answer
    /// still coming under its number is refused, and one still waiting
    /// for a slot or running is stopped.
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

    /// Asks git what `path` sets for itself and what it would use there,
    /// under the ticket's number and token. Two reads: the effective level
    /// cannot say which records are the repository's own
    /// (`config::get_regexp_at`).
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
                    // A cancelled read answers nothing: its failure would
                    // put "cancelled" under the boxes.
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

    /// The `save` slot (`identity::set_local_identity`).
    pub(super) fn write_repo(&mut self, name: String, email: String) {
        let Some(save) = self.begin_save() else {
            return;
        };
        self.listen();
        self.changed();
        let feed = Arc::clone(&self.feed);
        let path = self.repo_path.clone();
        // Held by the hub: the screen and its tab can both go, and the
        // window cannot close over a `git config` half way through its
        // pair (`hub::saves`).
        let spawned = Hub::with(|hub| {
            let executor = hub.save_executor();
            hub.spawn_save(async move {
                let cancel = CancellationToken::new();
                let workdir = PathBuf::from(&path);
                let written = platitude_core::identity::set_local_identity(
                    &executor, &workdir, &name, &email, &cancel,
                )
                .await;
                let msg = match written {
                    // The write reads itself back, so a half that did
                    // not land shows here.
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

    /// Takes the answers to the asks standing (module docs) and drops the
    /// rest. Answers whether to read the repository again — a save landed,
    /// whole or half, and the boxes should say what git holds now.
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
                    // Only a read the screen asked for clears the words: the
                    // re-read after a save (`refresh`) keeps git's account of
                    // why the save did not land.
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
                    // The read out now is from before the save, and can
                    // answer later in this very drain, before the re-read
                    // is numbered — retired here so it is refused.
                    self.retire_read();
                    self.save_pending = None;
                    self.write_busy = false;
                    self.write_unsaved = !(name_saved && email_saved);
                    self.write_name_saved = name_saved;
                    self.write_email_saved = email_saved;
                    self.error = error;
                    reread = true;
                }
            }
        }
        reread
    }
}
