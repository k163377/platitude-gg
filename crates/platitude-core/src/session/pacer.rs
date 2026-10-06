//! Drives the page's pace (`session::pace`): starts this tree's reads and
//! the other worktrees' when the rules say, meters each one
//! (`process::Meter`), and hands the rules its end.
//!
//! One task per session, from the first time the page is paced to the
//! close. It sleeps until the rules' next moment or until something
//! changes them; the reads it starts run in tasks of their own, so a slow
//! read never holds the clock.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::graph_refresh::PollTicket;
use super::pace::{At, PaceBounds, Pacer, Start, WorktreesPace};
use super::{RepoSession, SessionEvent, StashRead, relock};
use crate::process::Meter;

/// The rules, the origin their moments count from, and the word that
/// they changed.
pub(super) struct Pacing {
    rules: Mutex<Pacer>,
    origin: std::time::Instant,
    stirred: tokio::sync::Notify,
    driving: AtomicBool,
}

impl Default for Pacing {
    fn default() -> Self {
        Self {
            rules: Mutex::new(Pacer::new(WorktreesPace::Auto)),
            origin: std::time::Instant::now(),
            stirred: tokio::sync::Notify::new(),
            driving: AtomicBool::new(false),
        }
    }
}

impl Pacing {
    /// Now, as the rules count.
    pub(super) fn at(&self) -> At {
        self.origin.elapsed()
    }

    /// Changes the rules and wakes the driver to look again.
    pub(super) fn change<R>(&self, edit: impl FnOnce(&mut Pacer, At) -> R) -> R {
        let at = self.at();
        let answer = edit(&mut relock(&self.rules), at);
        // A permit is kept when nobody waits, so a change made while the
        // driver is between looking and sleeping still wakes it.
        self.stirred.notify_one();
        answer
    }

    fn deadline(&self, at: At) -> tokio::time::Instant {
        tokio::time::Instant::from_std(self.origin + at)
    }
}

impl RepoSession {
    /// Whether the page is on screen to be read for: this tree on its pace,
    /// and the other worktrees on theirs (`session::pace`). Off stops starting
    /// reads and forgets nothing; on again reads at once whatever fell due.
    pub fn set_paced(self: &Arc<Self>, paced: bool) {
        self.pacing.change(|rules, at| rules.set_active(paced, at));
        if paced && !self.pacing.driving.swap(true, Ordering::SeqCst) {
            let s = Arc::clone(self);
            self.runtime.spawn(async move { s.drive_pace().await });
        }
    }

    /// Something likely moved outside — the window came back: this tree is
    /// read now (or as the read under way ends), with its stashes, which no
    /// other paced read covers (`Start::Own`), and so is the worktree the
    /// pane stands on, its row and its list from one `status`. The other
    /// worktrees keep their pace.
    pub fn poll_now(self: &Arc<Self>) {
        let pane = self.carried_pane.standing();
        self.pacing.change(|rules, at| {
            rules.ask_now(at);
            if let Some(key) = &pane {
                rules.worktree_stale(key, at);
            }
        });
    }

    /// The floors and ceilings the pace keeps to — the settings'
    /// (`settings::Defaults::pace_bounds`).
    pub fn set_pace_bounds(&self, bounds: PaceBounds) {
        self.pacing.change(|rules, _| rules.set_bounds(bounds));
    }

    /// The opening has read everything just now.
    pub(super) fn paced_from_the_opening(&self) {
        self.pacing.change(|rules, at| rules.opened(at));
    }

    async fn drive_pace(self: Arc<Self>) {
        loop {
            let (starts, wake, at) = {
                let at = self.pacing.at();
                let mut rules = relock(&self.pacing.rules);
                let starts = rules.starts(at);
                (starts, rules.next_wake(), at)
            };
            for start in starts {
                let s = Arc::clone(&self);
                match start {
                    // The poll's flight is taken here, before the spawn, so
                    // the graph passes count the read from its start.
                    Start::Own { with_stashes } => match self.begin_poll() {
                        Ok(ticket) => {
                            self.runtime.spawn(async move {
                                s.paced_read(at, with_stashes, ticket).await;
                            });
                        }
                        Err(_) => self
                            .pacing
                            .change(|rules, at| rules.own_refused(at, with_stashes)),
                    },
                    Start::Worktree(key) => {
                        self.runtime
                            .spawn(async move { s.paced_worktree(key, at).await });
                    }
                }
            }
            let until = async {
                match wake {
                    Some(wake) => tokio::time::sleep_until(self.pacing.deadline(wake)).await,
                    None => std::future::pending().await,
                }
            };
            tokio::select! {
                biased;
                () = self.root_cancel.cancelled() => return,
                () = self.pacing.stirred.notified() => {}
                () = until => {}
            }
        }
    }

    /// This tree's read: refs, status and the worktree listing (the stashes
    /// too when `with_stashes`), and the one rebuild they ask for — metered
    /// as one. A read that failed weighs what it spawned all the same: a
    /// read that keeps timing out is the heaviest kind.
    async fn paced_read(self: Arc<Self>, began: At, with_stashes: bool, ticket: PollTicket) {
        let mut ended = OwnEnded {
            session: Arc::clone(&self),
            weight: None,
        };
        let meter = Arc::new(Meter::default());
        meter.over(self.read_own_tree(with_stashes, ticket)).await;
        ended.weight = meter.weighed();
        tracing::debug!(
            weight_ms = ended.weight.map(|w| w.as_millis() as u64),
            children = meter.children(),
            took_ms = self.pacing.at().saturating_sub(began).as_millis() as u64,
            "paced read of this tree"
        );
        drop(ended);
        if self.keeps_what_it_reads() {
            self.sink.event(SessionEvent::PacedRead { worktree: None });
        }
    }

    async fn read_own_tree(self: &Arc<Self>, with_stashes: bool, ticket: PollTicket) {
        // The listing rides this read: the WORKTREES rows, the mark saying
        // a branch is another worktree's, and which worktrees there are to
        // read. The walk waits for it, so it draws the worktrees against
        // where the listing says they stand now — a worktree that has
        // committed since its reading is asked for again by that walk
        // (`relay::carried_current`).
        let listings = async {
            if with_stashes {
                self.read_listings().await
            } else {
                (StashRead::default(), self.read_worktrees().await)
            }
        };
        let poll = self.poll_reads(ticket, listings).await;
        // After the slot is back, as the clock's own poll does; the graph
        // pass the read holds outlasts it (`PollRead`).
        self.take_the_read_owed();
        drop(poll);
    }

    async fn paced_worktree(self: Arc<Self>, key: String, began: At) {
        let mut ended = WorktreeEnded {
            session: Arc::clone(&self),
            key,
            began,
            read: None,
        };
        let Some((path, weight)) = self.read_paced_worktree(&ended.key).await else {
            return;
        };
        tracing::debug!(
            worktree = %path,
            weight_ms = weight.map(|w| w.as_millis() as u64),
            "paced read of another worktree"
        );
        ended.read = Some(weight);
        drop(ended);
        if self.keeps_what_it_reads() {
            self.sink.event(SessionEvent::PacedRead {
                worktree: Some(path),
            });
        }
    }
}

/// Tells the rules this tree's read ended, however its task ends: one that
/// unwound, left running, would start no read of this tree again.
struct OwnEnded {
    session: Arc<RepoSession>,
    weight: Option<Duration>,
}

impl Drop for OwnEnded {
    fn drop(&mut self) {
        let weight = self.weight;
        self.session
            .pacing
            .change(|rules, at| rules.own_ended(at, weight));
    }
}

/// Tells the rules a worktree's read ended — or never began, when `read` is
/// still `None` — however its task ends, as [`OwnEnded`] does.
struct WorktreeEnded {
    session: Arc<RepoSession>,
    key: String,
    began: At,
    read: Option<Option<Duration>>,
}

impl Drop for WorktreeEnded {
    fn drop(&mut self) {
        let (key, began, read) = (&self.key, self.began, self.read);
        self.session.pacing.change(|rules, _| match read {
            Some(weight) => rules.worktree_ended(key, began, weight),
            None => rules.worktree_skipped(key),
        });
    }
}
