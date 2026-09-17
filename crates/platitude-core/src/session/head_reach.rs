//! How a read's report of HEAD reaches the one record, and the two
//! questions asked of that record: whether the branch HEAD is on is the
//! only thing holding its tip, and whether a remote already has the
//! commit it is on.

use super::*;

impl RepoSession {
    /// Takes what a read saw of HEAD into the record (`Standing`) and
    /// tells the consumer where it moved to — or, after a write, that it
    /// stayed. `looked` is the stamp the read took before it spawned git:
    /// a read that looked before a write ended is refused here, whichever
    /// order the two landed in.
    pub(super) fn observe_head(&self, looked: u64, head: &HeadState) {
        let offer = self.standing.offer_head(looked, head);
        self.report_head(offer, head);
    }

    /// Sends the report the record decided a read's HEAD was owed, if any.
    fn report_head(&self, offer: HeadOffer, head: &HeadState) {
        if let HeadOffer::Settled { seq } | HeadOffer::Moved { seq } = offer {
            self.sink.event(SessionEvent::HeadObserved {
                head: head.clone(),
                seq,
            });
        }
    }

    /// Records what the refs read just saw of the branch tip, including
    /// the half of the reachability question the listing answers by
    /// itself: some other ref sitting exactly on the tip.
    ///
    /// That half is what makes tags count without paying for them. Tags
    /// are left out of the walk — on a tag-heavy repository they are the
    /// great majority of both the refs and the walk's own time
    /// (ci/baseline/head-reach-windows-x64.md) — and
    /// a tag on the tip is the shape that actually turns up; one strictly
    /// ahead of it is missed, which costs a hold mark on a row that could
    /// have been a click.
    pub(super) fn record_head_from_refs(&self, looked: u64, refs: &[RefEntry], head: &HeadState) {
        // Offered first: a listing whose HEAD a later read has already
        // moved past says nothing about the tip either, and a hold taken
        // from it would have the reach walked from where HEAD no longer is.
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
        // Beside the hold, and both are written before anything this
        // read wakes can look.
        self.report_head(offer, head);
    }

    /// Where the last read left HEAD, for a caller that would otherwise
    /// spawn two processes to ask again.
    ///
    /// `None` means no read has landed and there is nothing to go on.
    pub(super) fn known_head_tip(&self) -> Option<Option<Oid>> {
        self.standing.head_tip()
    }

    /// Answers whether the branch HEAD is on is the only thing holding its
    /// tip, and sends the answer if it moved.
    ///
    /// Runs off the write queue and off the poll's two-process budget: it
    /// is started where the graph is rebuilt, because it describes the
    /// same picture — whether a rewrite here leaves the old commits drawn.
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
    /// moved. `head` is the commit the pass walked from (`None` on a
    /// branch with no commits); `walked` is the mark on its row, or
    /// `None` where the walk's window stopped short of HEAD.
    ///
    /// **Off the rows the pass already drew, and git only where it
    /// cannot be.** Every emitted row is marked exactly
    /// (`session::published`), and HEAD is a starting point of the walk,
    /// so its row is in the window unless more than a window's worth of
    /// commits is newer than it — a detached HEAD parked on an old commit.
    /// Only that pass spends a read, and one at a time
    /// (`head_published_read`): the answer is a state of the
    /// repository.
    pub(super) fn settle_head_published(self: &Arc<Self>, head: Option<Oid>, walked: Option<bool>) {
        // A pass answers for the HEAD it walked from. Once the record has
        // moved past it, that answer is about a commit nobody is asking
        // about, and the walk the move started answers for where HEAD is.
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
        // Asked of git once per commit per refs listing: the listing is
        // what a push or a fetch moves, and it moving is what walks again
        // — the same commit under the same listing has the answer already.
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
