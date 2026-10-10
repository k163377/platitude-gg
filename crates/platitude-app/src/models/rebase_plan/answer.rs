//! The answer a confirmed button's last press takes from the plan, given
//! once the press's floor has passed (デザイン規約 §回数押し): what runs is
//! fixed at the press, the plan takes no edit until it is given, and a plan
//! put away meanwhile takes the answer with it for good.

use platitude_core::sequencer::RebaseStep;

use super::RebasePlanModel;

/// One accepted answer, under the ticket its press was handed.
pub(super) struct Accepted {
    pub(super) ticket: i32,
    pub(super) answer: Answer,
}

/// What an accepted press gives.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Answer {
    /// The todo as it stood at the press, and the base and tip that pin it.
    Run {
        upstream: String,
        steps: Vec<RebaseStep>,
        expect: String,
        root: bool,
    },
    /// The draft put away.
    Discard,
}

impl RebasePlanModel {
    /// Takes the answer a press accepted and locks the plan behind it: no
    /// edit, no other answer and no other plan until it is given or the
    /// plan is put away. The ticket to give it with, or 0 where there is
    /// nothing to take — no plan, one being replaced, an answer already
    /// standing, or a run of a plan that asks nothing.
    pub(super) fn accept(&mut self, run: bool) -> i32 {
        if self.locked || !self.active || self.loading || (run && !self.dirty) {
            return 0;
        }
        let answer = if run {
            Answer::Run {
                upstream: self.onto_oid.clone(),
                steps: self.todo_steps(),
                expect: self.expect_head.clone(),
                root: self.root,
            }
        } else {
            Answer::Discard
        };
        self.last_ticket += 1;
        self.accepted = Some(Accepted {
            ticket: self.last_ticket,
            answer,
        });
        self.locked = true;
        self.last_ticket
    }

    /// The answer accepted under `ticket`, handed out once and the lock with
    /// it; `None` for another press's ticket, or where the plan was put away
    /// since ([`Self::drop_accepted`]).
    pub(super) fn take_accepted(&mut self, ticket: i32) -> Option<Answer> {
        if !self
            .accepted
            .as_ref()
            .is_some_and(|accepted| accepted.ticket == ticket)
        {
            return None;
        }
        self.locked = false;
        self.accepted.take().map(|accepted| accepted.answer)
    }

    /// The plan is put away: an answer still waiting goes with it, and no
    /// ticket brings it back.
    pub(super) fn drop_accepted(&mut self) {
        self.accepted = None;
        self.locked = false;
    }

    /// Hands the todo to the session as one queued write pinned to the tip
    /// it was composed against (`RepoSession::rebase_interactive`).
    pub(super) fn queue_run(
        &self,
        upstream: String,
        steps: Vec<RebaseStep>,
        expect: String,
        root: bool,
    ) {
        let options = platitude_core::integrate::RebaseOptions {
            onto: None,
            branch: None,
            // Branches inside the range move along with it, always
            // (デザイン規約 §履歴を合流させる — 既定のフラグ).
            update_refs: true,
            root,
        };
        crate::hub::with_session(self.tab_id, move |s| {
            s.rebase_interactive(upstream, steps, options, expect);
        });
    }
}
