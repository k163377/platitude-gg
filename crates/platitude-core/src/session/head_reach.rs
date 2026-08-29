//! Whether the branch HEAD is on is the only thing holding its tip --
//! the hold record the refs read leaves, and the walk that answers the
//! other half.

use super::*;

impl RepoSession {
    /// Records what the refs read just saw of the branch tip, including
    /// the half of the reachability question the listing answers by
    /// itself: some other ref sitting exactly on the tip.
    ///
    /// That half is what makes tags count without paying for them. Tags
    /// are left out of the walk (`JetBrains/kotlin`: 45,846 of 53,672
    /// refs, 478ms of 501ms — ci/baseline/head-reach-windows-x64.md), and
    /// a tag on the tip is the shape that actually turns up; one strictly
    /// ahead of it is missed, which costs a hold mark on a row that could
    /// have been a click.
    pub(super) fn remember_head_hold(&self, refs: &[RefEntry], head: &HeadState) {
        let hold = head.oid.map(|tip| HeadHold {
            tip,
            branch: head.branch.clone().unwrap_or_default(),
            on_a_ref: reachable::a_ref_sits_on_head(refs, head),
        });
        *relock(&self.head_hold) = hold;
        // Beside the hold, and both are written before anything this
        // read wakes can look.
        *relock(&self.head_tip) = Some(head.oid);
    }

    /// Where the last refs read left HEAD, for a caller that would
    /// otherwise spawn two processes to ask again.
    ///
    /// `None` means no read has landed and there is nothing to go on.
    pub(super) fn known_head_tip(&self) -> Option<Option<Oid>> {
        *relock(&self.head_tip)
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
        let Some(hold) = relock(&self.head_hold).clone() else {
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
                &s.executor,
                &workdir,
                &hold.tip.to_hex(),
                &hold.branch,
                &cancel,
            )
            .await
            {
                Ok(reached) => s.publish_head_reach(reached),
                // A failed walk must not claim the tip is held: the mark
                // is the safe answer, and the next refresh asks again.
                Err(e) => {
                    tracing::warn!(error = %e, "could not tell whether the branch tip is held");
                    s.publish_head_reach(false);
                }
            }
        });
    }

    fn publish_head_reach(&self, reached_elsewhere: bool) {
        let moved =
            relock(&self.head_reach_seen).replace(reached_elsewhere) != Some(reached_elsewhere);
        if moved {
            self.sink
                .event(SessionEvent::HeadReachChecked { reached_elsewhere });
        }
    }
}
