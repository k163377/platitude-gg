//! How a read's report of HEAD reaches the one record, and the two
//! questions asked of that record: whether the branch HEAD is on is the
//! only thing holding its tip, and whether a remote already has the
//! commit it is on.

use super::*;

/// What the reachability question keeps between asks.
pub(super) struct HeadReach {
    /// One permit: the walk is the only part of a refresh that scales
    /// with the history, and a tick arriving mid-walk is dropped.
    slot: Arc<tokio::sync::Semaphore>,
    /// Whether a rewrite takes the branches on the tip along (the person's
    /// `rebase.updateRefs`) — read by the reach that first needs it
    /// ([`RepoSession::reached_elsewhere`]), dropped by
    /// [`RepoSession::reread_update_refs`].
    update_refs: Derived<bool>,
}

impl Default for HeadReach {
    fn default() -> Self {
        Self {
            slot: Arc::new(tokio::sync::Semaphore::new(1)),
            update_refs: Derived::default(),
        }
    }
}

impl RepoSession {
    /// Takes what a read saw of HEAD into the record (`Standing`) and
    /// reports where it moved — or, after a write, that it stayed.
    /// `looked` is the read's pre-spawn stamp: a read that looked before a
    /// write ended is refused, whichever order the two landed in.
    pub(super) fn observe_head(&self, looked: u64, head: &HeadState) {
        let offer = self.standing.offer_head(looked, head);
        self.report_head(offer, head);
    }

    fn report_head(&self, offer: HeadOffer, head: &HeadState) {
        if let HeadOffer::Settled { seq } | HeadOffer::Moved { seq } = offer {
            self.sink.event(SessionEvent::HeadObserved {
                head: head.clone(),
                seq,
            });
        }
    }

    /// Records what the refs read saw of the branch tip, including the
    /// half of the reachability question the listing answers alone:
    /// what else sits exactly on the tip.
    ///
    /// That half is how tags count without being walked (their cost scales
    /// with the tag count — ci/baseline/head-reach-windows-x64.md). A tag
    /// strictly ahead of the tip is missed, which costs a hold mark on a
    /// row that could have been a click. So is a branch another worktree
    /// has out, until a worktree listing has named it
    /// (`worktree_holders`) — the opening's can land after its refs read —
    /// and a ref move asks again.
    pub(super) fn record_head_from_refs(&self, looked: u64, refs: &[RefEntry], head: &HeadState) {
        // Offered first: a listing a later read has moved past says
        // nothing about the tip, and a hold from it would walk the reach
        // from where HEAD no longer is. One the later read only overtook
        // (`HeadOffer::Overtaken` — the status read's HEAD often lands
        // first) still does: left without a hold, the reach answers as for
        // an unborn branch.
        let offer = self.standing.offer_head(looked, head);
        if offer == HeadOffer::Stale {
            return;
        }
        let holders = self.worktree_holders();
        let hold = head.oid.map(|tip| HeadHold {
            tip,
            branch: head.branch.clone().unwrap_or_default(),
            on_tip: reachable::OnTip::of(refs, head, |name| holders.branches.contains_key(name)),
        });
        self.standing.set_hold(hold);
        // After the hold: both are in place before anything this read
        // wakes can look.
        self.report_head(offer, head);
    }

    /// Where the last read left HEAD, saving a caller two processes.
    /// `None`: no read has landed yet.
    pub(super) fn known_head_tip(&self) -> Option<Option<Oid>> {
        self.standing.head_tip()
    }

    /// Answers whether the branch HEAD is on is the only thing holding its
    /// tip, and sends the answer if it moved.
    ///
    /// Off the write queue and the poll's budget, started where the graph
    /// is rebuilt: it describes the same picture (whether a rewrite leaves
    /// the old commits drawn).
    pub(super) fn settle_head_reach(self: &Arc<Self>) {
        let Some(workdir) = self.workdir() else {
            return;
        };
        let Some(hold) = self.standing.hold() else {
            // No tip (an unborn branch): nothing to lose, nothing to ask.
            self.publish_head_reach(false);
            return;
        };
        // A branch on the tip that the rewrite leaves there holds it as a
        // tag does — once the setting is known (`reached_elsewhere`).
        let left_on_tip = !hold.on_tip.branches.is_empty()
            && self.head_reach.update_refs.peek(|along| !along) == Some(true);
        if hold.on_tip.kept || left_on_tip {
            self.publish_head_reach(true);
            return;
        }
        let Ok(permit) = Arc::clone(&self.head_reach.slot).try_acquire_owned() else {
            tracing::trace!("head reach skipped: the previous walk has not finished");
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let _permit = permit;
            let cancel = s.root_cancel.clone();
            match s.reached_elsewhere(&workdir, &hold, &cancel).await {
                Ok(reached) => s.publish_head_reach(reached),
                // A failed walk answers with the mark: it is the safe
                // answer, and the next refresh asks again.
                Err(e) => {
                    tracing::warn!(error = %e, "could not tell whether the branch tip is held");
                    s.publish_head_reach(false);
                }
            }
        });
    }

    /// Reads `rebase.updateRefs` again — where the person may have changed
    /// it: the settings screen closing — and asks the reach again where the
    /// setting decides it (a branch on the tip, nothing kept beside it).
    pub fn reread_update_refs(self: &Arc<Self>) {
        self.head_reach.update_refs.forget();
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            // After a walk in flight, which may have read the setting
            // before it was dropped: asked while it runs, the reach would
            // be dropped as a refresh's tick is, and nothing else asks
            // again before a ref moves.
            let Ok(walked) = Arc::clone(&s.head_reach.slot).acquire_owned().await else {
                return;
            };
            drop(walked);
            if s.standing
                .hold()
                .is_some_and(|hold| !hold.on_tip.kept && !hold.on_tip.branches.is_empty())
            {
                s.settle_head_reach();
            }
        });
    }

    /// The rest of the question, past the listing: whether anything the
    /// rewrite leaves reaches the tip.
    ///
    /// A branch on the tip asks for the person's `rebase.updateRefs` first
    /// — without it the branch stays and holds the tip, with it the branch
    /// moves and is left out of the walk with HEAD's own. Read the first
    /// time a branch on the tip asks, and not watched: the setting rarely
    /// moves (user decision — ask before changing). It is read again where
    /// the person may have changed it ([`Self::reread_update_refs`]),
    /// and afresh by the next session — only the tab in front has one, so
    /// a tab brought back to the front or reopened does.
    async fn reached_elsewhere(
        &self,
        workdir: &Path,
        hold: &HeadHold,
        cancel: &CancellationToken,
    ) -> Result<bool, GitError> {
        let mut moving = vec![hold.branch.clone()];
        if !hold.on_tip.branches.is_empty() {
            let along = self
                .head_reach
                .update_refs
                .get_or_try_init(|| {
                    reachable::rewrites_take_branches_along(&self.exec_background, workdir, cancel)
                })
                .await?;
            if !along {
                return Ok(true);
            }
            moving.extend(hold.on_tip.branches.iter().cloned());
        }
        reachable::reached_without(
            &self.exec_background,
            workdir,
            &hold.tip.to_hex(),
            &moving,
            cancel,
        )
        .await
    }

    fn publish_head_reach(&self, reached_elsewhere: bool) {
        if self.standing.offer_reach(reached_elsewhere) {
            self.sink
                .event(SessionEvent::HeadReachChecked { reached_elsewhere });
        }
    }

    /// Takes what a graph pass read off HEAD's row — whether a remote
    /// already has that commit — into the record, and sends it if it
    /// moved. `head` is the commit the pass walked from (`None` when
    /// unborn); `walked` is the mark on its row (exact —
    /// `session::published`), or `None` where the window stopped short of
    /// HEAD: a detached HEAD more than a window's worth of commits back.
    /// Only then is git asked, one read at a time (`head_published_read`).
    pub(super) fn settle_head_published(self: &Arc<Self>, head: Option<Oid>, walked: Option<bool>) {
        // Once the record has moved past the HEAD this pass walked from,
        // the walk the move started answers instead.
        if let Some(standing) = self.standing.head_tip()
            && standing != head
        {
            return;
        }
        let Some(oid) = head else {
            self.publish_head_published(HeadPublished {
                oid: None,
                published: false,
            });
            return;
        };
        if let Some(published) = walked {
            self.publish_head_published(HeadPublished {
                oid: Some(oid),
                published,
            });
            return;
        }
        // Once per commit per refs listing: only a push or fetch, which
        // moves the listing, can change the answer.
        let refs = (*relock(&self.refs_key)).unwrap_or_default();
        if !self.standing.claim_published_ask(oid, refs) {
            return;
        }
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let Some(workdir) = s.workdir() else {
                return;
            };
            let cancel = s.head_published_read.begin(&s.root_cancel);
            let range = publish::only(&oid.to_hex());
            match publish::state_of(&s.exec_background, &workdir, &range, &cancel).await {
                Ok(state) => s.publish_head_published(HeadPublished {
                    oid: Some(oid),
                    published: state.rewrites_published(),
                }),
                Err(e) => {
                    if !e.is_cancelled() {
                        tracing::warn!(error = %e, "could not tell whether a remote has HEAD");
                    }
                }
            }
        });
    }

    fn publish_head_published(&self, answer: HeadPublished) {
        if self.standing.offer_published(answer) {
            self.sink.event(SessionEvent::HeadPublished {
                oid: answer.oid,
                published: answer.published,
            });
        }
    }
}
