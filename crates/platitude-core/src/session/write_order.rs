//! One working tree, one write order — held apart from the sessions
//! writing to it.
//!
//! A session closed mid-write keeps that write running
//! ([`RepoSession::close`]), and a tab reopened over the same repository
//! is a new session with a queue of its own. Two queues, one index: left
//! alone they race for `.git/index.lock`, and the second one's commit can
//! land in front of the first one's. This is what gives them a single
//! order. Every local write takes its **place** in that order at the
//! moment it is accepted, whoever accepted it, and runs when the place
//! comes up — so a session opened later can never step in front of a
//! write accepted before it.
//!
//! **Keyed by the git directory**, which is what holds the index and the
//! standing operation: two sessions on one working tree share an order,
//! two linked worktrees of one repository do not — separate indexes, and
//! nothing between them to order.
//!
//! The key is what git answered for the folder the tab was opened from
//! (`RepoInfo::git_dir`), so **two tabs opened through paths git spells
//! differently would be given two orders** and race as they would with
//! no order at all. A tab reopened over itself cannot: it opens from the
//! path it already held, which git answers the same way twice.
//!
//! **What takes a place is what writes here**
//! ([`OperationKind::writes_here`]), which is a different question from
//! the lane a write is supervised on. A push and the fetches take none —
//! their whole effect is at the other end, and ordering them would put a
//! commit behind somebody else's round trip. A composite delete takes
//! one even though the network paces its far half, because its near half
//! takes a ref away here.
//!
//! **Nothing here holds a session.** The members are weak and the running
//! write is a few numbers, so an order costs nothing once the last
//! session on its tree has gone — and a session the tab strip released is
//! not kept alive by being on one.

use std::collections::VecDeque;

use super::*;

/// Every order this process has handed out, by git directory.
///
/// Weak, so the last session on a working tree takes its order with it:
/// held strongly, this would grow by one entry per repository the reader
/// ever opened, each pinning the tree it was made for.
static ORDERS: Mutex<BTreeMap<PathBuf, std::sync::Weak<WriteOrder>>> = Mutex::new(BTreeMap::new());

/// The order for `git_dir`, made if no session is on that tree yet.
///
/// The dead entries are swept here: the map is walked by every session
/// that opens, which is the only moment a new one
/// is added.
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

/// The order the local writes of one working tree run in.
pub(super) struct WriteOrder {
    held: Mutex<Held>,
    /// Rung whenever the front of the order moved. The value says
    /// nothing — a waiter reads the order itself again on every ring —
    /// but a receiver that was not polled when one went out still sees
    /// the version move, so a turn cannot be slept through.
    moved: tokio::sync::watch::Sender<u64>,
}

#[derive(Default)]
struct Held {
    /// The places handed out and not yet given back, oldest first. The
    /// front is the write git may run; everything behind it waits.
    waiting: VecDeque<u64>,
    /// Places handed out so far, which is where the next number comes
    /// from. Never reused: a number is a position in one process's
    /// history of this tree.
    handed_out: u64,
    /// What git is running for this tree, as the write holding the front
    /// named itself — `None` between writes. Read by every session on
    /// the tree, which is how a poll keeps out of a tree somebody else is
    /// writing ([`RepoSession::tree_write`]).
    running: Option<Operation>,
    /// The sessions to tell when a write on this tree lands. Weak, and
    /// left on [`RepoSession::close`]: a closed session has no page left
    /// to read into, and its own writes are the ones being waited out.
    members: Vec<std::sync::Weak<RepoSession>>,
}

impl WriteOrder {
    fn new() -> Self {
        Self {
            held: Mutex::new(Held::default()),
            moved: tokio::sync::watch::channel(0).0,
        }
    }

    /// Takes the next place for a write just accepted.
    ///
    /// Called inside the same call that hands the write's id back, so the
    /// places are in the order the asks were made. A place taken when the
    /// write's turn came round instead would be no order at all across
    /// sessions: a tab opened over a running write would take its place
    /// the moment that write ended, in front of everything still queued
    /// behind it.
    pub(super) fn take_place(self: &Arc<Self>) -> Place {
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
    /// where there is no `Arc` to compare with, and a `Weak` answers for
    /// the address either way.
    pub(super) fn leave(&self, session: &RepoSession) {
        let gone = std::ptr::from_ref(session);
        let mut held = relock(&self.held);
        held.members
            .retain(|member| member.as_ptr() != gone && member.strong_count() > 0);
    }

    /// Every session on this tree but `writer` — who to tell that the
    /// tree has been written by somebody else.
    ///
    /// Answers with the sessions: the caller is inside a write loop,
    /// and holding this lock across a call back into a session would
    /// take the two locks in the order nothing else takes them
    /// in.
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

    /// Gives `place` back, from wherever in the order it sits.
    ///
    /// Anywhere but the front is a write that never ran — refused by a
    /// closed queue, or dropped with the task holding it — and taking it
    /// out is all there is to do. The front is the write that was
    /// running, so what it said it was goes with it.
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

/// A write's place in its working tree's order, held from acceptance
/// until the write and the reads behind it are done.
///
/// Given back by dropping it, whichever way the write ended — refused
/// before it ran, unwound, or taken down with the runtime — so a place
/// can never be left blocking the tree.
pub(super) struct Place {
    order: Arc<WriteOrder>,
    place: u64,
}

impl Place {
    /// Waits until this is the write the tree is running, and says what
    /// it is for as long as it holds the front.
    pub(super) async fn granted(&self, operation: Operation) {
        let mut moved = self.order.moved.subscribe();
        while !self.order.is_front(self.place) {
            // The bell belongs to the order this holds an `Arc` to, so
            // the sender outlives the wait; a closed channel would still
            // leave the loop.
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

    #[tokio::test]
    async fn places_are_served_in_the_order_they_were_taken() {
        let order = Arc::new(WriteOrder::new());
        let first = order.take_place();
        let second = order.take_place();

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
        let running = order.take_place();
        let refused = order.take_place();
        let behind = order.take_place();
        running.granted(an_operation()).await;

        // The shape of a write the queue turned away after its place was
        // taken: it never reaches the front, and the one behind it goes
        // on to its own turn.
        drop(refused);
        drop(running);
        behind.granted(an_operation()).await;
        assert!(order.is_front(behind.place));
    }

    /// Keys nothing else in the binary can name, because the registry is
    /// the one thing here every session in the process shares.
    #[test]
    fn one_order_per_working_tree_and_none_once_it_is_let_go() {
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
        drop(held.take_place());
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
