//! How a read's report of HEAD reaches the one record, and the two
//! questions asked of that record: whether the branch HEAD is on is the
//! only thing holding its tip, and whether a remote already has the
//! commit it is on.

use super::*;

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
    /// another ref sitting exactly on the tip.
    ///
    /// That half is how tags count without being walked (their cost scales
    /// with the tag count — ci/baseline/head-reach-windows-x64.md). A tag
    /// strictly ahead of the tip is missed, which costs a hold mark on a
    /// row that could have been a click.
    pub(super) fn record_head_from_refs(&self, looked: u64, refs: &[RefEntry], head: &HeadState) {
        // Offered first: a listing a later read has moved past says
        // nothing about the tip, and a hold from it would walk the reach
        // from where HEAD no longer is.
        let offer = self.standing.offer_head(looked, head);
        if offer == HeadOffer::Stale {
            return;
        }
        let hold = head.oid.map(|tip| HeadHold {
            tip,
            branch: head.branch.clone().unwrap_or_default(),
            on_a_ref: reachable::a_ref_sits_on_head(refs, head),
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
        if hold.on_a_ref {
            self.publish_head_reach(true);
            return;
        }
        let Ok(permit) = Arc::clone(&self.head_reach_slot).try_acquire_owned() else {
            tracing::trace!("head reach skipped: the previous walk has not finished");
            return;
        };
        let s = Arc::clone(self);
        self.runtime.spawn(async move {
            let _permit = permit;
            let cancel = s.root_cancel.clone();
            match reachable::reached_without_branch(
                &s.exec_background,
                &workdir,
                &hold.tip.to_hex(),
                &hold.branch,
                &cancel,
            )
            .await
            {
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
