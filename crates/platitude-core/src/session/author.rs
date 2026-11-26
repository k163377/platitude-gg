//! Who a commit is attributed to: the identity git will record, the
//! signing configuration behind it, and HEAD's own message and author —
//! what an amend starts from.

use super::*;

impl RepoSession {
    /// Re-reads the author identity and signing configuration — as the
    /// repository opens, and after the window writes an identity outside
    /// any session (`hub::Tabs::refresh_authors`). The task answers once
    /// the configuration has published with the read that did not land,
    /// empty where it did. `None` where no repository is open.
    ///
    /// A follow read whose askers wait (`session::asks`): one read at a
    /// time, so an older answer never lands after a newer one, and an ask
    /// is answered by a read that began after it.
    pub fn refresh_author(self: &Arc<Self>) -> Option<tokio::task::JoinHandle<Vec<FollowUp>>> {
        let workdir = self.workdir()?;
        // On the caller's thread: the place is the ask's moment.
        let stamp = self.author_read.stamp();
        let s = Arc::clone(self);
        Some(self.runtime.spawn(async move {
            s.author_read
                .run_from(stamp, || async {
                    let cancel = s.root_cancel.clone();
                    match identity::load(&s.executor, &workdir, &cancel).await {
                        Ok(config) => {
                            relock(&s.authorship).identity = Some(config.identity.clone());
                            s.sink.event(SessionEvent::AuthorLoaded { config });
                            s.read_identity_shown();
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
                })
                .await
        }))
    }

    /// Reads the identity as `.mailmap` spells it ([`identity::shown`]) —
    /// after each identity read, and wherever a status shows HEAD moved
    /// ([`Self::authorship_status`]): a pull or a switch may bring another
    /// `.mailmap`, one edited and not committed reaches it at the next move.
    /// Apart from the identity's read, so a commit waits on no face. A newer
    /// ask cancels the one still out; nothing to read before an identity
    /// is.
    fn read_identity_shown(self: &Arc<Self>) {
        let Some(identity) = relock(&self.authorship).identity.clone() else {
            return;
        };
        self.spawn_read_latest(
            "identity-shown",
            &self.latest.identity_shown,
            |s, workdir, cancel| async move {
                let shown = identity::shown(&s.executor, &workdir, &identity, &cancel).await?;
                Ok(SessionEvent::AuthorShown { shown })
            },
        );
    }

    /// Reads HEAD's message so an amend can start from it — on demand,
    /// since only the amend path wants it.
    pub fn load_head_message(self: &Arc<Self>) {
        self.spawn_read("head-message", |s, workdir, cancel| async move {
            // An unborn branch answers with the empty prefill. A failed read
            // goes to the error surface: an amend started from a blank it
            // trusts would commit the blank.
            let message = commit::head_message(&s.executor, &workdir, &cancel)
                .await?
                .unwrap_or_default();
            Ok(SessionEvent::HeadMessageLoaded { message })
        });
    }

    /// Reads HEAD's author — whom an amend keeps, which the commit button
    /// names — while an amend is in view ([`Self::watch_amend`]). A newer
    /// ask cancels the one still out. A failed read is forgotten, so the
    /// next status asks again.
    fn load_head_author(self: &Arc<Self>) {
        self.spawn_read_latest(
            "head-author",
            &self.latest.head_author,
            |s, workdir, cancel| async move {
                match commit::head_author(&s.executor, &workdir, &cancel).await {
                    Ok(author) => {
                        // An unborn branch has no author to keep.
                        let author = author.unwrap_or_default();
                        if !cancel.is_cancelled() {
                            relock(&s.authorship).author_at = Some(author.oid);
                        }
                        Ok(SessionEvent::HeadAuthorLoaded { author })
                    }
                    Err(e) => {
                        if !e.is_cancelled() {
                            relock(&s.authorship).amend.owe_author();
                        }
                        Err(e)
                    }
                }
            },
        );
    }

    /// Asks whether an amend pressed now would leave HEAD changing nothing
    /// ([`commit::amend_would_empty`]) — the commit button's to refuse
    /// before git does. A newer ask cancels the one still out.
    ///
    /// A failed read still answers, `false`, so the button is not left
    /// waiting — git refuses an empty amend at the press all the same —
    /// and the ask is forgotten, so the next status asks again
    /// (`session::asks`).
    pub fn check_amend(self: &Arc<Self>) {
        self.spawn_read_latest(
            "amend-empties",
            &self.latest.amend,
            |s, workdir, cancel| async move {
                match commit::amend_would_empty(&s.executor, &workdir, &cancel).await {
                    Ok(empties) => Ok(SessionEvent::AmendChecked { empties }),
                    Err(e) => {
                        if !e.is_cancelled() {
                            relock(&s.authorship).amend.asked = None;
                        }
                        s.fail("amend-empties", e);
                        Ok(SessionEvent::AmendChecked { empties: false })
                    }
                }
            },
        );
    }

    /// Whether an amend is in view — the amend box ticked, or HEAD's own
    /// commit open where its message is edited. While it is, what an amend
    /// would make of the index is asked now and again whenever a status
    /// shows the index or HEAD moved ([`Self::authorship_status`]): another
    /// git can change what is staged without changing how many files are (a
    /// different hunk of the same file), so the index is keyed by
    /// `staged_print` — each staged entry's mode and blob hash — not by a
    /// count. HEAD's author is read the same way ([`Self::load_head_author`]):
    /// as the amend comes into view and on each status that shows HEAD moved
    /// under it. An author last read off a HEAD a status has since shown gone
    /// is let go of as the amend comes into view — an empty answer, ahead of
    /// the read — so the commit button names nobody until that read answers;
    /// one still HEAD's stands meanwhile. Held by
    /// `session_integration::amend_watch`.
    pub fn watch_amend(self: &Arc<Self>, on: bool) {
        let gone = {
            let mut kept = relock(&self.authorship);
            kept.amend = AmendWatch {
                on,
                ..AmendWatch::default()
            };
            on && kept.author_gone()
        };
        if gone {
            self.sink.event(SessionEvent::HeadAuthorLoaded {
                author: commit::HeadAuthor::default(),
            });
        }
        if on {
            self.check_amend();
            self.load_head_author();
        }
    }

    /// A status read published: where it shows HEAD moved, the identity's
    /// `.mailmap` spelling is read again, and where an amend is in view, its
    /// asks go again for what moved under it — HEAD's author, and the index or
    /// HEAD the last ask was about. Nothing staged needs no git — an amend of
    /// the message alone never empties a commit ([`commit::commit`] lets an
    /// empty HEAD's through).
    pub(super) fn authorship_status(self: &Arc<Self>, status: &crate::status::WorktreeStatus) {
        let key = AmendKey {
            head: status.branch_oid,
            staged: status.staged().next().is_some(),
            print: status.staged_print,
        };
        let (moved, ask, author) = {
            let mut kept = relock(&self.authorship);
            let moved = kept.head_moved(key.head);
            (
                moved,
                kept.amend.ask_about(key),
                kept.amend.ask_author(moved),
            )
        };
        if moved {
            self.read_identity_shown();
        }
        if author {
            self.load_head_author();
        }
        match ask {
            AmendAsk::Nothing => {}
            AmendAsk::Git => self.check_amend(),
            // An ask still out was about an index that has gone: its answer
            // would land after this one.
            AmendAsk::NothingStaged => {
                drop(self.latest.amend.begin(&self.root_cancel));
                self.sink
                    .event(SessionEvent::AmendChecked { empties: false });
            }
        }
    }
}

/// What the reads of who a commit is by keep between them: the identity the
/// last configuration read found, which its `.mailmap` spelling is read for
/// ([`RepoSession::read_identity_shown`]); the HEAD the last published status
/// showed, so one that shows it moved asks again what a move may change; the
/// HEAD the last author read answered about; and the amend watch. Each is
/// `None` before its first.
#[derive(Debug, Default)]
pub(crate) struct Authorship {
    identity: Option<identity::Identity>,
    head_seen: Option<Option<crate::oid::Oid>>,
    author_at: Option<Option<crate::oid::Oid>>,
    amend: AmendWatch,
}

impl Authorship {
    /// Whether a status showing `head` shows HEAD moved since the last one,
    /// which it then holds. The first status does too: a read made before it
    /// may have found the HEAD before this one.
    fn head_moved(&mut self, head: Option<crate::oid::Oid>) -> bool {
        self.head_seen.replace(head) != Some(head)
    }

    /// Whether the last author read answered about a HEAD the last status
    /// shows gone. Not before both: nothing to let go of, or nothing to tell
    /// by.
    fn author_gone(&self) -> bool {
        matches!((self.author_at, self.head_seen), (Some(at), Some(seen)) if at != seen)
    }
}

/// What [`RepoSession::watch_amend`] keeps: whether an amend is in view,
/// what the index and HEAD were when it was last asked about, and whether
/// HEAD's author is owed a read that failed.
#[derive(Debug, Default)]
pub(crate) struct AmendWatch {
    on: bool,
    asked: Option<AmendKey>,
    author_owed: bool,
}

impl AmendWatch {
    /// Whether a status asks for HEAD's author again: in view, where it
    /// shows HEAD `moved`, or after a read that failed.
    fn ask_author(&mut self, moved: bool) -> bool {
        self.on && (moved | std::mem::take(&mut self.author_owed))
    }

    /// A read of the author failed: the next status asks again.
    fn owe_author(&mut self) {
        self.author_owed = true;
    }

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

    /// HEAD moves on the first status, and wherever one shows a HEAD the
    /// last did not — an unborn one included.
    #[test]
    fn head_moves_on_the_first_status_and_where_it_changed() {
        let one = Some(crate::oid::Oid::from_hex_str(&"1".repeat(40)).expect("an id"));
        let two = Some(crate::oid::Oid::from_hex_str(&"2".repeat(40)).expect("an id"));
        let mut kept = Authorship::default();
        assert!(kept.head_moved(one), "the first status");
        assert!(!kept.head_moved(one));
        assert!(kept.head_moved(two));
        assert!(kept.head_moved(None), "HEAD gone unborn");
        assert!(!kept.head_moved(None));
    }

    /// The last author answered is let go of only where a status shows its
    /// HEAD gone — not before an answer or a status, nor while it is HEAD's.
    #[test]
    fn an_author_goes_only_where_a_status_shows_its_head_gone() {
        let one = Some(crate::oid::Oid::from_hex_str(&"1".repeat(40)).expect("an id"));
        let two = Some(crate::oid::Oid::from_hex_str(&"2".repeat(40)).expect("an id"));
        let mut kept = Authorship::default();
        assert!(!kept.author_gone(), "nothing read");
        kept.author_at = Some(one);
        assert!(!kept.author_gone(), "no status yet");
        kept.head_moved(one);
        assert!(!kept.author_gone(), "still HEAD's");
        kept.head_moved(two);
        assert!(kept.author_gone());
        kept.author_at = Some(None);
        kept.head_moved(None);
        assert!(!kept.author_gone(), "unborn, as read");
    }

    /// HEAD's author is asked for again where HEAD moved, and after a read
    /// that failed; out of view, never — and a failure out of view is owed to
    /// nobody.
    #[test]
    fn heads_author_is_asked_again_only_where_head_moved_in_view() {
        let mut watch = AmendWatch::default();
        assert!(!watch.ask_author(true), "out of view");
        watch.on = true;
        assert!(!watch.ask_author(false));
        assert!(watch.ask_author(true));
        watch.owe_author();
        assert!(watch.ask_author(false), "the read failed");
        assert!(!watch.ask_author(false), "asked once");
    }

    /// In view: asked again only where the index or HEAD moved, and without
    /// git where nothing is staged.
    #[test]
    fn in_view_only_a_moved_index_is_asked_about() {
        let mut watch = AmendWatch {
            on: true,
            ..AmendWatch::default()
        };
        assert_eq!(watch.ask_about(key(true, 1)), AmendAsk::Git);
        assert_eq!(watch.ask_about(key(true, 1)), AmendAsk::Nothing);
        assert_eq!(watch.ask_about(key(true, 2)), AmendAsk::Git);
        assert_eq!(watch.ask_about(key(false, 0)), AmendAsk::NothingStaged);
        assert_eq!(watch.ask_about(key(false, 0)), AmendAsk::Nothing);
    }
}
