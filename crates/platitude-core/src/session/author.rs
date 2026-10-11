//! Who a commit is attributed to: the identity git will record, the
//! signing configuration behind it, and HEAD's own message and author —
//! what an amend starts from.

use super::*;

impl RepoSession {
    /// Re-reads the author identity and signing configuration. The task
    /// answers once the configuration has published (the write queue waits
    /// on it after an identity write) with the read that did not land,
    /// empty where it did. `None` where no repository is open.
    pub fn refresh_author(self: &Arc<Self>) -> Option<tokio::task::JoinHandle<Vec<FollowUp>>> {
        let workdir = self.workdir()?;
        let s = Arc::clone(self);
        Some(self.runtime.spawn(async move {
            let cancel = s.root_cancel.clone();
            match identity::load(&s.executor, &workdir, &cancel).await {
                Ok(config) => {
                    s.sink.event(SessionEvent::AuthorLoaded { config });
                    Vec::new()
                }
                Err(e) => {
                    if !e.is_cancelled() {
                        s.sink.event(SessionEvent::AuthorUnread);
                    }
                    s.fail(FollowUp::Author.label(), e);
                    vec![FollowUp::Author]
                }
            }
        }))
    }

    /// Records `user.name` / `user.email`.
    pub fn set_identity(
        self: &Arc<Self>,
        name: String,
        email: String,
        scope: identity::ConfigScope,
    ) -> Option<OperationId> {
        self.write(
            OperationKind::Identity,
            AfterWrite::Author,
            move |exec, repo, cancel| async move {
                let written =
                    identity::set_identity(&exec, &repo.workdir, &name, &email, scope, &cancel)
                        .await?;
                if written.is_saved() {
                    return Ok(());
                }
                // Half an identity reads as a whole one everywhere, so a
                // write that did not take fails even when git raised nothing.
                Err(GitError::Rejected {
                    message: if written.message.is_empty() {
                        "git still reports a different identity".to_string()
                    } else {
                        written.message
                    },
                })
            },
        )
    }

    /// Reads HEAD's message and author so an amend can start from them —
    /// on demand, since only the amend path wants them.
    pub fn load_head_commit(self: &Arc<Self>) {
        self.spawn_read("head-commit", |s, workdir, cancel| async move {
            // An unborn branch answers with the empty prefill. A failed read
            // goes to the error surface: an amend started from a blank it
            // trusts would commit the blank.
            let head = commit::head_commit(&s.executor, &workdir, &cancel)
                .await?
                .unwrap_or_default();
            Ok(SessionEvent::HeadCommitLoaded { head })
        });
    }

    /// Asks whether an amend pressed now would leave HEAD changing nothing
    /// ([`commit::amend_would_empty`]) — the commit button's to refuse
    /// before git does. A newer ask cancels the one still out.
    pub fn check_amend(self: &Arc<Self>) {
        self.spawn_read_latest(
            "amend-empties",
            &self.amend_read,
            |s, workdir, cancel| async move {
                let empties = commit::amend_would_empty(&s.executor, &workdir, &cancel).await?;
                Ok(SessionEvent::AmendChecked { empties })
            },
        );
    }

    /// Whether an amend is in view — the amend box ticked, or HEAD's own
    /// commit open where its message is edited. While it is, what an amend
    /// would make of the index is asked now and again whenever a status
    /// shows the index or HEAD moved ([`Self::amend_status`]): another git
    /// can change what is staged without changing how many files are.
    pub fn watch_amend(self: &Arc<Self>, on: bool) {
        *relock(&self.amend_watch) = AmendWatch { on, asked: None };
        if on {
            self.check_amend();
        }
    }

    /// A status read published: asks again where an amend is in view and
    /// the index or HEAD it was asked about moved. Nothing staged needs no
    /// git — an amend of the message alone never empties a commit
    /// ([`commit::commit`] lets an empty HEAD's through).
    pub(super) fn amend_status(self: &Arc<Self>, status: &crate::status::WorktreeStatus) {
        let key = AmendKey {
            head: status.branch_oid,
            staged: status.staged().next().is_some(),
            print: status.staged_print,
        };
        let ask = relock(&self.amend_watch).ask_about(key);
        match ask {
            AmendAsk::Nothing => {}
            AmendAsk::Git => self.check_amend(),
            // An ask still out was about an index that has gone: its answer
            // would land after this one.
            AmendAsk::NothingStaged => {
                drop(self.amend_read.begin(&self.root_cancel));
                self.sink
                    .event(SessionEvent::AmendChecked { empties: false });
            }
        }
    }
}

/// What [`RepoSession::watch_amend`] keeps: whether an amend is in view, and
/// what the index and HEAD were when it was last asked about.
#[derive(Debug, Default)]
pub(crate) struct AmendWatch {
    on: bool,
    asked: Option<AmendKey>,
}

impl AmendWatch {
    /// What a status showing `key` asks for: nothing out of view, or where
    /// the index and HEAD are as last asked about; no git where nothing is
    /// staged.
    fn ask_about(&mut self, key: AmendKey) -> AmendAsk {
        if !self.on || self.asked.replace(key) == Some(key) {
            AmendAsk::Nothing
        } else if key.staged {
            AmendAsk::Git
        } else {
            AmendAsk::NothingStaged
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum AmendAsk {
    Nothing,
    Git,
    NothingStaged,
}

/// What an amend's answer depends on: HEAD, and what the index holds where
/// it differs from HEAD (`WorktreeStatus::staged_print`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AmendKey {
    head: Option<crate::oid::Oid>,
    staged: bool,
    print: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(staged: bool, print: u64) -> AmendKey {
        AmendKey {
            head: None,
            staged,
            print,
        }
    }

    /// Out of view nothing is asked, however the index moves, and nothing is
    /// kept to compare against once it comes into view.
    #[test]
    fn nothing_is_asked_while_no_amend_is_in_view() {
        let mut watch = AmendWatch::default();
        assert_eq!(watch.ask_about(key(true, 1)), AmendAsk::Nothing);
        assert_eq!(watch.ask_about(key(true, 2)), AmendAsk::Nothing);
        watch.on = true;
        assert_eq!(watch.ask_about(key(true, 2)), AmendAsk::Git);
    }

    /// In view: asked again only where the index or HEAD moved, and without
    /// git where nothing is staged.
    #[test]
    fn in_view_only_a_moved_index_is_asked_about() {
        let mut watch = AmendWatch {
            on: true,
            asked: None,
        };
        assert_eq!(watch.ask_about(key(true, 1)), AmendAsk::Git);
        assert_eq!(watch.ask_about(key(true, 1)), AmendAsk::Nothing);
        assert_eq!(watch.ask_about(key(true, 2)), AmendAsk::Git);
        assert_eq!(watch.ask_about(key(false, 0)), AmendAsk::NothingStaged);
        assert_eq!(watch.ask_about(key(false, 0)), AmendAsk::Nothing);
    }
}
