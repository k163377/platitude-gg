//! One worktree, one write order — held apart from the sessions
//! writing to it.
//!
//! A session closed mid-write keeps that write running
//! ([`RepoSession::close`]), and a tab reopened over the same repository
//! is a new session with a queue of its own: two queues, one index. Every
//! local write takes its place in this order when it is accepted, so a
//! session opened later can never step in front of a write accepted
//! before it.
//!
//! Keyed by the git directory (`RepoInfo::git_dir`, what git answered for
//! the folder the tab was opened from): linked worktrees have separate
//! indexes, so separate orders. Two tabs opened through paths git spells
//! differently would get two orders and race; a tab reopened over itself
//! reuses its own path.
//!
//! What takes a place is [`OperationKind::writes_here`], not the lane.
//!
//! Nothing here holds a session: the members are weak, so a session the
//! tab strip released is not kept alive by being on an order.

use std::collections::VecDeque;

use super::*;

/// Every order this process has handed out, by git directory. Weak, so
/// the last session on a worktree takes its order with it.
static ORDERS: Mutex<BTreeMap<PathBuf, std::sync::Weak<WriteOrder>>> = Mutex::new(BTreeMap::new());

/// Held while places are taken, in any order: a write with places in two
/// orders takes both in one instant ([`take_places`]). Taken one at a time,
/// two such writes could each stand in front of the other in one of the
/// orders, and wait for each other for good.
static TAKING: Mutex<()> = Mutex::new(());

/// A place in `own` (where there is one) and in each of `others`, for one
/// write, taken in one instant (`TAKING`): every write ahead of it in any of
/// them was accepted before it, so none of those waits for it.
///
/// Taken for a write just accepted — inside the call that hands the
/// write's id back. Taken when the write's turn came round instead, a tab
/// opened over a running write would step in front of everything still
/// queued behind it.
pub(super) fn take_places(
    own: Option<&Arc<WriteOrder>>,
    others: &[Arc<WriteOrder>],
) -> (Option<Place>, Vec<Place>) {
    let _taking = relock(&TAKING);
    let own = own.map(|order| order.place_now());
    let others = others.iter().map(|order| order.place_now()).collect();
    (own, others)
}

/// The order for `git_dir`, made if no session is on that tree yet.
/// Dead entries are swept here, the only moment a new one is added.
pub(super) fn of(git_dir: &Path) -> Arc<WriteOrder> {
    let mut orders = relock(&ORDERS);
    orders.retain(|_, order| order.strong_count() > 0);
    if let Some(order) = orders.get(git_dir).and_then(std::sync::Weak::upgrade) {
        return order;
    }
    let order = Arc::new(WriteOrder::new());
    orders.insert(git_dir.to_path_buf(), Arc::downgrade(&order));
    order
}

/// The order the local writes of one worktree run in.
pub(super) struct WriteOrder {
    held: Mutex<Held>,
    /// Rung whenever the front of the order moved. The value says
    /// nothing, but a receiver not polled when one went out still sees
    /// the version move, so a turn cannot be slept through.
    moved: tokio::sync::watch::Sender<u64>,
}

#[derive(Default)]
struct Held {
    /// The places handed out and not yet given back, oldest first. The
    /// front is the write git may run.
    waiting: VecDeque<u64>,
    /// Places handed out so far; numbers are never reused.
    handed_out: u64,
    /// What the write holding the front said it is — `None` between
    /// writes. How a poll keeps out of a tree somebody else is writing
    /// ([`RepoSession::tree_write`]).
    running: Option<Operation>,
    /// The sessions to tell when a write on this tree lands. Left on
    /// [`RepoSession::close`]: a closed session has no page left to read
    /// into.
    members: Vec<std::sync::Weak<RepoSession>>,
}

impl WriteOrder {
    fn new() -> Self {
        Self {
            held: Mutex::new(Held::default()),
            moved: tokio::sync::watch::channel(0).0,
        }
    }

    /// The next place, for [`take_places`] alone.
    fn place_now(self: &Arc<Self>) -> Place {
        let mut held = relock(&self.held);
        held.handed_out += 1;
        let place = held.handed_out;
        held.waiting.push_back(place);
        Place {
            order: Arc::clone(self),
            place,
        }
    }

    /// The write git is running in this tree, whoever asked for it.
    pub(super) fn running(&self) -> Option<Operation> {
        relock(&self.held).running
    }

    /// Puts `session` on the list of readers of this tree.
    pub(super) fn join(&self, session: &Arc<RepoSession>) {
        let mut held = relock(&self.held);
        held.members.retain(|member| member.strong_count() > 0);
        held.members.push(Arc::downgrade(session));
    }

    /// Takes it off again. By pointer: a close runs inside the session,
    /// where there is no `Arc` to compare with.
    pub(super) fn leave(&self, session: &RepoSession) {
        let gone = std::ptr::from_ref(session);
        let mut held = relock(&self.held);
        held.members
            .retain(|member| member.as_ptr() != gone && member.strong_count() > 0);
    }

    /// Every session on this tree but `writer`. Answers with the sessions
    /// rather than calling them: holding this lock across a call back into
    /// a session would take the two locks in the order nothing else takes
    /// them in.
    pub(super) fn others(&self, writer: &RepoSession) -> Vec<Arc<RepoSession>> {
        let writer = std::ptr::from_ref(writer);
        relock(&self.held)
            .members
            .iter()
            .filter(|member| member.as_ptr() != writer)
            .filter_map(std::sync::Weak::upgrade)
            .collect()
    }

    fn is_front(&self, place: u64) -> bool {
        relock(&self.held).waiting.front() == Some(&place)
    }

    fn now_running(&self, place: u64, operation: Operation) {
        let mut held = relock(&self.held);
        if held.waiting.front() == Some(&place) {
            held.running = Some(operation);
        }
    }

    /// Gives `place` back, from wherever in the order it sits. Anywhere
    /// but the front is a write that never ran (refused by a closed queue,
    /// or dropped with its task).
    fn give_back(&self, place: u64) {
        {
            let mut held = relock(&self.held);
            let Some(at) = held.waiting.iter().position(|waiting| *waiting == place) else {
                return;
            };
            held.waiting.remove(at);
            if at == 0 {
                held.running = None;
            }
        }
        self.moved.send_modify(|rung| *rung += 1);
    }
}

/// A write's place in its worktree's order, held from acceptance
/// until the write and the reads behind it are done. Given back by
/// dropping it, whichever way the write ended, so a place can never be
/// left blocking the tree.
pub(super) struct Place {
    order: Arc<WriteOrder>,
    place: u64,
}

impl Place {
    /// The order this is a place in.
    pub(super) fn order(&self) -> Arc<WriteOrder> {
        Arc::clone(&self.order)
    }

    /// Waits until this is the write the tree is running, and says what
    /// it is for as long as it holds the front.
    pub(super) async fn granted(&self, operation: Operation) {
        let mut moved = self.order.moved.subscribe();
        while !self.order.is_front(self.place) {
            // The sender lives in the order this holds an `Arc` to, so it
            // outlives the wait.
            if moved.changed().await.is_err() {
                break;
            }
        }
        self.order.now_running(self.place, operation);
    }
}

impl Drop for Place {
    fn drop(&mut self) {
        self.order.give_back(self.place);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn an_operation() -> Operation {
        Operation::new(OperationKind::Commit, AfterWrite::Graph)
    }

    /// A place in `order` alone, as a write on its own tree takes one.
    fn place_in(order: &Arc<WriteOrder>) -> Place {
        take_places(Some(order), &[])
            .0
            .expect("a place in its own order")
    }

    /// A write that puts work into another tree too stands behind what was
    /// accepted there before it, and in front of what is accepted after.
    #[tokio::test]
    async fn a_write_in_two_orders_takes_its_turn_in_both() {
        let (here, there) = (Arc::new(WriteOrder::new()), Arc::new(WriteOrder::new()));
        let there_first = place_in(&there);
        let (own, into) = take_places(Some(&here), std::slice::from_ref(&there));
        let own = own.expect("a place here");
        let there_after = place_in(&there);

        own.granted(an_operation()).await;
        assert!(
            !there.is_front(into[0].place),
            "it waits there behind what was accepted first"
        );
        drop(there_first);
        into[0].granted(an_operation()).await;
        assert!(there.is_front(into[0].place));
        assert!(
            !there.is_front(there_after.place),
            "and what was accepted there after waits behind it"
        );
    }

    #[tokio::test]
    async fn places_are_served_in_the_order_they_were_taken() {
        let order = Arc::new(WriteOrder::new());
        let first = place_in(&order);
        let second = place_in(&order);

        first.granted(an_operation()).await;
        assert!(
            order.running().is_some(),
            "the front says what the tree is running"
        );
        assert!(
            !order.is_front(second.place),
            "the place taken second waits behind it"
        );

        drop(first);
        assert_eq!(order.running(), None, "and says nothing between writes");
        second.granted(an_operation()).await;
        assert!(order.is_front(second.place));
    }

    #[tokio::test]
    async fn a_place_given_back_from_behind_the_front_blocks_nobody() {
        let order = Arc::new(WriteOrder::new());
        let running = place_in(&order);
        let refused = place_in(&order);
        let behind = place_in(&order);
        running.granted(an_operation()).await;

        // A write the queue turned away after its place was taken.
        drop(refused);
        drop(running);
        behind.granted(an_operation()).await;
        assert!(order.is_front(behind.place));
    }

    /// Keys nothing else in the binary can name: the registry is
    /// process-wide.
    #[test]
    fn one_order_per_worktree_and_none_once_it_is_let_go() {
        let tree = std::path::Path::new("/write-order-test/one/.git");
        let other = std::path::Path::new("/write-order-test/other/.git");
        let held = of(tree);
        assert!(
            Arc::ptr_eq(&held, &of(tree)),
            "the second session on a tree joins the first one's order"
        );
        assert!(
            !Arc::ptr_eq(&held, &of(other)),
            "a different tree waits for nobody"
        );

        let watching = std::sync::Arc::downgrade(&held);
        drop(place_in(&held));
        drop(held);
        assert!(
            watching.upgrade().is_none(),
            "the registry holds nothing of its own: the last session out frees it"
        );
        assert_eq!(
            relock(&of(tree).held).handed_out,
            0,
            "and the tree opened again starts on an order with nobody in it"
        );
    }
}
