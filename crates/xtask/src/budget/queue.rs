//! Who is handed the machine next, as arithmetic over what the ledger
//! holds: the order the waiting units stand in, and whether the one
//! asking may start now.
//!
//! Nothing here touches a file or a clock. The ledger is read once under
//! its own lock (`super::look`) and answered here, so the rule that
//! decides a gate's pace is a function of a list — which is how the
//! priority, the fairness and the admission stop are tested at all
//! (`super::tests`), rather than by racing gates on a machine.
//!
//! **The rule is one sentence**: a waiting unit may start when it stands
//! in the prefix of the ordered queue that fits in what is free, and the
//! prefix stops at the first unit that does not fit. Everything the
//! design owes falls out of it:
//!
//! - **A landing goes first, then a window the user is waiting for,
//!   then every test**, because that is the order the ranks sort in
//!   ([`order`]). One rule covers all three: what is short of room stops
//!   the walk, so a waiting landing keeps ordinary work out, and a
//!   waiting launch keeps it out too — but neither keeps out the rank
//!   above it.
//! - **Work that would delay a landing does not start**: the walk stops
//!   at the landing's unit when the room is not there yet, so the weight
//!   a finishing unit gives back is not taken by the next small one — it
//!   accumulates until the landing fits. That is the whole of the
//!   admission stop, and it needs no separate reservation.
//! - **A heavy unit is not starved by light ones.** Skipping past the
//!   unit that does not fit (a backfill) would let a seat's endless
//!   one-weight verbs walk over the four-weight `cargo test` behind them
//!   forever.
//! - **Room a landing will not use is used**: what is left after the
//!   landing's units fit goes on down the queue.
//!
//! What the rule cannot promise is the other direction, and it is worth
//! saying where the rule is: with landings first and nothing preempted,
//! ordinary work behind an unbroken run of landings waits as long as the
//! landings take.

use std::collections::BTreeMap;

/// Where a unit stands in the one queue every seat on the machine shares.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Rank {
    /// A landing's unit. `land` is the one thing that moves main, and in
    /// one line with the seats' own gates it stands for minutes behind
    /// work whose branches rebase over what lands anyway.
    Landing,
    /// A window the user asked for (`cargo xtask launch`, the fast path
    /// of CLAUDE.md ビルド・テスト). Somebody is waiting at the screen
    /// for it, which no test is; it goes ahead of every test and behind
    /// a landing. It is the build and the start that are counted — the
    /// window itself is the user's and holds none of the machine.
    Launch,
    /// Everything else — a seat's own gate, whatever it was asked with.
    /// `--all` and `--fresh` are a bigger run, not a lesser one, and
    /// nothing here reads them.
    Normal,
}

impl Rank {
    pub(crate) fn word(self) -> &'static str {
        match self {
            Rank::Landing => "landing",
            Rank::Launch => "launch",
            Rank::Normal => "normal",
        }
    }

    /// The word back, an unreadable one counting as ordinary work: a
    /// ticket nobody can parse must not be handed the machine first.
    pub(crate) fn parse(word: &str) -> Rank {
        match word {
            "landing" => Rank::Landing,
            "launch" => Rank::Launch,
            _ => Rank::Normal,
        }
    }
}

/// One unit's claim on the machine, as the ledger holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Ticket {
    /// When it arrived, counted machine-wide: the tie-break under the
    /// fairness, and the whole of the landings' own order.
    pub seq: u64,
    pub pid: u32,
    /// What the unit takes out of the budget while it runs. Zero for a
    /// turn, which queues landings against each other and asks for none
    /// of the machine.
    pub weight: u32,
    /// The whole its owner asks the machine for. Every gate names the
    /// same number unless one was told `--jobs` above the machine's
    /// count, and the pool is the largest any live ticket names
    /// ([`budget_of`]) — so widening it stays what it was: an explicit
    /// ask, made by one gate and seen by all.
    pub budget: u32,
    pub rank: Rank,
    /// A landing's turn rather than a unit of work: outside the budget,
    /// and held for the whole of one landing ([`turn_is`]).
    pub turn: bool,
    /// Which tree the unit belongs to, which is what the fairness is
    /// between.
    pub seat: String,
    pub what: String,
    /// Whether it has been handed the machine.
    pub running: bool,
    /// When it was handed the machine, and zero for as long as it is
    /// still waiting.
    ///
    /// **Not when the ticket was registered**, which is what the ledger
    /// wrote before: the only thing this dates is how long the unit has
    /// been *running* (`ledger::LEFTOVER_CEILING`), and a unit that
    /// queued for an hour and then ran for a minute has been running for
    /// a minute. Dated from the registration, a step admitted after a
    /// long queue was already past the ceiling on its first second.
    pub ran_since: u64,
    /// The process this unit started, once it has started one; zero
    /// before that and for a unit that starts none.
    ///
    /// **A ticket comes down when its owner is done, and its owner waits
    /// for this** — so on the ordinary road this is only ever a number
    /// nobody has to look at. It is there for the road where the owner
    /// is killed: its lock frees at once, but the cargo, the container
    /// or the app it started is still on the machine, and handing that
    /// room out is over-subscription with nobody left to notice
    /// (`super::Pool::read`).
    pub child: u32,
    /// The program that was started at [`Ticket::child`], as the unit
    /// spelled it. A pid is a name the machine hands out again the
    /// moment its process is gone, so a leftover asked only whether
    /// *something* is at that number would hold the room for whatever
    /// inherited it; the name is what tells the work apart from a
    /// stranger (`subprocess::image_still_at`). Empty for a ticket that
    /// started nothing.
    pub child_name: String,
    /// The second a reader last asked after [`Ticket::child`]. The ask
    /// costs a process on Windows, and every waiter would otherwise pay
    /// it ten times a second for as long as the leftover ran.
    pub probed: u64,
    /// Whether somebody has already been told this leftover is late.
    /// The ledger remembers it so that the line is said once for the
    /// machine rather than by every waiter that looks at it
    /// (`ledger::Pool::leftover`).
    pub told: bool,
    /// Read off the ledger rather than written in it: the owner of this
    /// ticket is gone and [`Ticket::child`] is not, so the room is being
    /// held for work nobody is waiting on any more.
    pub orphaned: bool,
    /// Read off the ledger too: a leftover that has been running longer
    /// than a step is allowed to. **Not a reason to hand its room out**
    /// — the work is still on the machine, and the room is held while it
    /// is (`ledger::Pool::leftover`) — but the one thing here a person
    /// may have to end by hand, so it says so wherever the queue is
    /// shown.
    pub overdue: bool,
}

/// What the running units are holding.
pub(crate) fn used(tickets: &[Ticket]) -> u32 {
    tickets
        .iter()
        .filter(|t| t.running && !t.turn)
        .map(|t| t.weight)
        .sum()
}

/// The pool every live ticket is measured against: the largest budget
/// any of them names, and never less than `mine` — a process that asked
/// for more than the others is the explicit ask, and one that asked for
/// less does not shrink the machine under the others' feet.
pub(crate) fn budget_of(tickets: &[Ticket], mine: u32) -> u32 {
    tickets.iter().map(|t| t.budget).fold(mine, u32::max)
}

/// What each seat has been handed, by rank: the running total the
/// fairness is read off ([`order`]). Kept in the ledger beside the
/// tickets, because it has to outlive the units it counts — a seat just
/// served has nothing left in the ledger to say so.
pub(crate) type Served = BTreeMap<(Rank, String), u64>;

/// The waiting units in the order they are handed the machine: the
/// higher rank first, then the seat that has been handed least of the
/// machine, then arrival.
///
/// **A seat's place is what it has been served, not what it is holding
/// and not where it stands in its own queue.** Both of the smaller
/// answers read right at one instant and wrong at the next. With a
/// waiting twice and b once, ordering by the queue hands it to a, and
/// then — a's second unit and b's only one both first in *their* queues
/// — hands it to a again. Ordering by what is still running fixes that
/// only while a's first unit is still running: the moment it finishes,
/// both seats are holding nothing and the older unit is a's again. So
/// the count is kept in the ledger and survives the unit that earned it.
///
/// Weight rather than a count of units, because weight is what is being
/// shared: a seat running one `cargo test` has taken as much as one
/// running four verbs, and the order says so.
///
/// **A seat that arrives late does not arrive with credit.** Its count
/// starts at the least any seat with work here has — not at zero, which
/// would let it take the machine until it caught up with an hour of
/// somebody else's gate, and not at whatever it had when it last ran,
/// which would be a debt it never asked for.
///
/// Counted inside a rank rather than across all of them, so a landing
/// does not cost that seat's ordinary work its place.
pub(crate) fn order<'a>(tickets: &'a [Ticket], served: &Served) -> Vec<&'a Ticket> {
    let floor = floor_of(tickets, served);
    let mut so_far: BTreeMap<(Rank, &str), u64> = BTreeMap::new();
    let mut waiting: Vec<&Ticket> = tickets.iter().filter(|t| !t.running && !t.turn).collect();
    waiting.sort_by_key(|t| t.seq);
    let mut ranked: Vec<(Rank, u64, u64, &Ticket)> = Vec::with_capacity(waiting.len());
    for ticket in waiting {
        let key = (ticket.rank, ticket.seat.as_str());
        let at = *so_far
            .entry(key)
            .or_insert_with(|| stood_at(served, ticket.rank, &ticket.seat, floor));
        ranked.push((ticket.rank, at, ticket.seq, ticket));
        so_far.insert(key, at + u64::from(ticket.weight));
    }
    ranked.sort_by_key(|(rank, at, seq, _)| (*rank, *at, *seq));
    ranked.into_iter().map(|(_, _, _, ticket)| ticket).collect()
}

/// Where a seat stands: what it has been served, never below the floor
/// the seats with work here are standing on.
pub(crate) fn stood_at(served: &Served, rank: Rank, seat: &str, floor: u64) -> u64 {
    served
        .get(&(rank, seat.to_string()))
        .copied()
        .unwrap_or(floor)
        .max(floor)
}

/// The least any seat with a live ticket has been served, which is what
/// a seat with no count of its own starts from. Zero when no seat here
/// has a count yet.
pub(crate) fn floor_of(tickets: &[Ticket], served: &Served) -> u64 {
    tickets
        .iter()
        .filter(|t| !t.turn)
        .filter_map(|t| served.get(&(t.rank, t.seat.clone())).copied())
        .min()
        .unwrap_or(0)
}

/// Whether the unit `seq` may start now: it stands in the prefix of
/// [`order`] that fits in what the running units left.
pub(crate) fn admits(tickets: &[Ticket], budget: u32, seq: u64, served: &Served) -> bool {
    let mut free = budget.saturating_sub(used(tickets));
    for ticket in order(tickets, served) {
        // The unit that does not fit ends the prefix, itself included:
        // what frees from here on is being kept for it.
        if ticket.weight > free {
            return false;
        }
        if ticket.seq == seq {
            return true;
        }
        free -= ticket.weight;
    }
    false
}

/// Whether it is `seq`'s turn among the landings: the oldest live turn
/// is the one landing that runs, and the rest stand in arrival order
/// behind it (CLAUDE.md Git 運用 — a landing is rebase, gate, census and
/// fast-forward, and two of them at once would gate against a main that
/// the other is about to move).
pub(crate) fn turn_is(tickets: &[Ticket], seq: u64) -> bool {
    tickets
        .iter()
        .filter(|t| t.turn)
        .map(|t| t.seq)
        .min()
        .is_some_and(|first| first == seq)
}

/// What the queue is waiting behind, for the line a wait that ran out
/// says and for `cargo xtask budget`.
pub(crate) fn standing(tickets: &[Ticket], budget: u32, served: &Served) -> String {
    let running: Vec<&Ticket> = tickets.iter().filter(|t| t.running && !t.turn).collect();
    let waiting = order(tickets, served);
    // The landings' own line, which the budget's queue does not carry: a
    // turn is an exclusion and holds none of the machine, so a landing
    // standing in line for one is invisible in everything above.
    let mut turns: Vec<&Ticket> = tickets.iter().filter(|t| t.turn).collect();
    turns.sort_by_key(|t| t.seq);
    let mut out = format!(
        "budget {}/{budget} held by {} unit(s), {} waiting, {} landing(s) in line\n",
        used(tickets),
        running.len(),
        waiting.len(),
        turns.len(),
    );
    for (at, ticket) in turns.iter().enumerate() {
        out.push_str(&format!(
            "  {}  {}\n",
            if at == 0 { "landing" } else { "  behind" },
            line(ticket)
        ));
    }
    for ticket in running {
        out.push_str(&format!("  running {}\n", line(ticket)));
    }
    for (at, ticket) in waiting.iter().enumerate() {
        out.push_str(&format!("  {:>2}.     {}\n", at + 1, line(ticket)));
    }
    out
}

fn line(ticket: &Ticket) -> String {
    format!(
        "{} weight {} — {} (seat {}, pid {}){}",
        ticket.rank.word(),
        ticket.weight,
        ticket.what,
        ticket.seat,
        ticket.pid,
        // What a person has to see to act on it: the unit is nobody's
        // any more, and the room is held for what it left behind — and
        // where that has gone on longer than a step may run, that this
        // is the one thing here nothing but a person will end.
        match (ticket.orphaned, ticket.overdue) {
            (true, false) => format!(
                " — LEFTOVER: this unit's process is gone and pid {} is not",
                ticket.child
            ),
            (true, true) => format!(
                " — LEFTOVER OVERDUE: this unit's process is gone, pid {} ({}) is still \
                 running past the longest a step may run, and its room is held until it is",
                ticket.child, ticket.child_name
            ),
            _ => String::new(),
        }
    )
}

#[cfg(test)]
mod tests {
    use super::{
        Rank, Served, Ticket, admits, budget_of, floor_of, order, stood_at, turn_is, used,
    };

    /// A waiting unit of `seat`, `weight` heavy, arrived at `seq`.
    fn waiting(seq: u64, seat: &str, weight: u32, rank: Rank) -> Ticket {
        Ticket {
            seq,
            pid: 100 + u32::try_from(seq).unwrap_or(0),
            weight,
            budget: 24,
            rank,
            turn: false,
            seat: seat.to_string(),
            what: format!("unit {seq}"),
            running: false,
            ran_since: 0,
            child: 0,
            child_name: String::new(),
            probed: 0,
            told: false,
            orphaned: false,
            overdue: false,
        }
    }

    fn running(seq: u64, seat: &str, weight: u32) -> Ticket {
        Ticket {
            running: true,
            ..waiting(seq, seat, weight, Rank::Normal)
        }
    }

    fn seqs(ordered: &[&Ticket]) -> Vec<u64> {
        ordered.iter().map(|t| t.seq).collect()
    }

    /// A machine no seat has been served on yet.
    fn fresh() -> Served {
        Served::new()
    }

    /// What the ledger holds after `seat` has been handed `weight`.
    fn served_of(rows: &[(&str, u64)]) -> Served {
        rows.iter()
            .map(|(seat, weight)| ((Rank::Normal, (*seat).to_string()), *weight))
            .collect()
    }

    #[test]
    fn a_landing_s_units_stand_ahead_of_every_other_seat_s() {
        let tickets = vec![
            waiting(1, "b", 1, Rank::Normal),
            waiting(2, "c", 1, Rank::Normal),
            waiting(3, "a", 1, Rank::Landing),
        ];
        assert_eq!(seqs(&order(&tickets, &fresh())), vec![3, 1, 2]);
    }

    /// The order the user asked for: what moves main, then the window
    /// somebody is waiting at, then every test.
    #[test]
    fn a_window_the_user_asked_for_goes_behind_a_landing_and_ahead_of_the_tests() {
        let tickets = vec![
            waiting(1, "b", 1, Rank::Normal),
            waiting(2, "c", 1, Rank::Launch),
            waiting(3, "a", 1, Rank::Landing),
        ];
        assert_eq!(seqs(&order(&tickets, &fresh())), vec![3, 2, 1]);
        // And the stop is the same rule: a launch short of room keeps
        // the tests behind it out, and lets the landing past.
        let short = vec![
            running(9, "d", 22),
            waiting(2, "c", 4, Rank::Launch),
            waiting(1, "b", 1, Rank::Normal),
            waiting(3, "a", 1, Rank::Landing),
        ];
        assert!(
            admits(&short, 24, 3, &fresh()),
            "the landing fits in the two free"
        );
        assert!(
            !admits(&short, 24, 2, &fresh()),
            "the launch does not fit yet"
        );
        assert!(
            !admits(&short, 24, 1, &fresh()),
            "a test took room the launch is waiting for"
        );
    }

    /// One round of a unit per seat before anybody's second: a seat that
    /// queued a hundred verbs does not stand in front of a seat with one.
    #[test]
    fn seats_take_a_round_each_before_anybody_takes_a_second() {
        let tickets = vec![
            waiting(1, "b", 1, Rank::Normal),
            waiting(2, "b", 1, Rank::Normal),
            waiting(3, "b", 1, Rank::Normal),
            waiting(4, "c", 1, Rank::Normal),
            waiting(5, "c", 1, Rank::Normal),
        ];
        assert_eq!(seqs(&order(&tickets, &fresh())), vec![1, 4, 2, 5, 3]);
    }

    /// The turn has to survive the unit that took it — **including that
    /// unit finishing**. With a waiting twice and b once the order is a,
    /// b, a; once a's first has been served and its ticket is gone,
    /// nothing among the tickets remembers it, and an order read off
    /// them alone hands a the next room as well.
    #[test]
    fn a_seat_already_served_stands_behind_one_that_was_not() {
        let queued = vec![
            waiting(1, "a", 1, Rank::Normal),
            waiting(2, "a", 1, Rank::Normal),
            waiting(3, "b", 1, Rank::Normal),
        ];
        assert_eq!(
            seqs(&order(&queued, &fresh())),
            vec![1, 3, 2],
            "a, then b, then a"
        );
        // Both seats have a row from the moment they queued
        // (`super::Pool::register`); a's has moved by the one unit it was
        // handed.
        let after_one = served_of(&[("a", 1), ("b", 0)]);
        // a's first is admitted and still running.
        let running_now = vec![running(1, "a", 1), queued[1].clone(), queued[2].clone()];
        assert_eq!(seqs(&order(&running_now, &after_one)), vec![3, 2]);
        // And now it has finished: its ticket is gone, both seats hold
        // nothing, and a's remaining unit is the older of the two.
        let done = vec![queued[1].clone(), queued[2].clone()];
        assert_eq!(
            seqs(&order(&done, &after_one)),
            vec![3, 2],
            "the seat that was already served took the next room too"
        );
    }

    /// Weight rather than a count of units, because weight is what the
    /// seats are sharing: one `cargo test` has taken as much of the
    /// machine as four verbs, and the seat handed it waits accordingly.
    #[test]
    fn a_seat_s_place_is_the_weight_it_was_served_not_the_units() {
        let tickets = vec![
            waiting(3, "a", 1, Rank::Normal),
            waiting(4, "b", 1, Rank::Normal),
        ];
        assert_eq!(
            seqs(&order(&tickets, &served_of(&[("a", 4), ("b", 1)]))),
            vec![4, 3],
            "the seat served four went behind the seat served one"
        );
    }

    /// A seat that arrives late arrives with neither credit nor debt: it
    /// starts where the seats already here are standing. At zero it
    /// would take the machine until it had caught up with an hour of
    /// somebody else's gate; at its own stale count it would be paying
    /// off a debt from a run nobody remembers. This is the number a
    /// seat's row is made with when it joins the queue
    /// (`super::Pool::register`).
    #[test]
    fn a_seat_that_arrives_late_starts_where_the_others_stand() {
        let here = vec![waiting(1, "a", 1, Rank::Normal)];
        let long_run = served_of(&[("a", 900)]);
        let floor = floor_of(&here, &long_run);
        assert_eq!(floor, 900, "the seat already here is what the floor is");
        assert_eq!(
            stood_at(&long_run, Rank::Normal, "newcomer", floor),
            900,
            "a seat with no row of its own arrived with credit"
        );
        assert_eq!(
            stood_at(
                &served_of(&[("newcomer", 0)]),
                Rank::Normal,
                "newcomer",
                floor
            ),
            900,
            "a stale row read as credit"
        );
        // Level with the seat already here, so arrival order decides.
        let both = vec![here[0].clone(), waiting(2, "newcomer", 1, Rank::Normal)];
        assert_eq!(
            seqs(&order(&both, &served_of(&[("a", 900), ("newcomer", 900)]))),
            vec![1, 2]
        );
    }

    /// A seat's landing does not push that seat's own ordinary work back
    /// a round: the rounds are counted inside a rank.
    #[test]
    fn a_seat_s_landing_does_not_cost_its_ordinary_work_a_round() {
        let tickets = vec![
            waiting(1, "a", 1, Rank::Landing),
            waiting(2, "a", 1, Rank::Normal),
            waiting(3, "b", 1, Rank::Normal),
        ];
        assert_eq!(seqs(&order(&tickets, &fresh())), vec![1, 2, 3]);
    }

    #[test]
    fn what_runs_is_what_is_held_and_a_turn_holds_nothing() {
        let tickets = vec![
            running(1, "a", 4),
            running(2, "b", 1),
            waiting(3, "c", 4, Rank::Normal),
            Ticket {
                turn: true,
                weight: 0,
                running: true,
                ..waiting(4, "a", 0, Rank::Landing)
            },
        ];
        assert_eq!(used(&tickets), 5);
    }

    /// The room that is free is handed down the queue in order, and the
    /// walk stops at the first unit that does not fit — what frees from
    /// there on is being kept for it.
    #[test]
    fn the_prefix_that_fits_starts_and_the_unit_that_does_not_stops_the_walk() {
        let tickets = vec![
            running(1, "a", 20),
            waiting(2, "b", 4, Rank::Normal),
            waiting(3, "c", 1, Rank::Normal),
        ];
        assert!(
            admits(&tickets, 24, 2, &fresh()),
            "the head fits in the four free"
        );
        assert!(
            !admits(&tickets, 24, 3, &fresh()),
            "a light unit walked over the heavy one ahead of it"
        );
    }

    /// The admission stop: while a landing's unit is waiting for room it
    /// has not got yet, no ordinary unit takes what frees.
    #[test]
    fn ordinary_work_stops_being_admitted_while_a_landing_waits_for_room() {
        let tickets = vec![
            running(1, "b", 22),
            waiting(2, "a", 4, Rank::Landing),
            waiting(3, "c", 1, Rank::Normal),
        ];
        assert!(
            !admits(&tickets, 24, 2, &fresh()),
            "the landing does not fit yet"
        );
        assert!(
            !admits(&tickets, 24, 3, &fresh()),
            "the two free went to ordinary work the landing is waiting for"
        );
        // The unit that was holding the machine finishes, and the room
        // the landing was waiting for is there.
        let freed = vec![tickets[1].clone(), tickets[2].clone()];
        assert!(
            admits(&freed, 24, 2, &fresh()),
            "the landing takes the room"
        );
    }

    /// Room a landing will not use is not left idle.
    #[test]
    fn what_the_landing_does_not_need_goes_on_down_the_queue() {
        let tickets = vec![
            waiting(1, "a", 4, Rank::Landing),
            waiting(2, "b", 1, Rank::Normal),
        ];
        assert!(admits(&tickets, 24, 1, &fresh()));
        assert!(
            admits(&tickets, 24, 2, &fresh()),
            "twenty free and nothing to wait for"
        );
    }

    #[test]
    fn the_pool_is_the_largest_budget_any_live_ticket_names() {
        let mut tickets = vec![waiting(1, "a", 1, Rank::Normal)];
        tickets[0].budget = 40;
        assert_eq!(budget_of(&tickets, 24), 40);
        tickets[0].budget = 12;
        assert_eq!(
            budget_of(&tickets, 24),
            24,
            "a gate that asked for less shrank the machine under the others"
        );
    }

    #[test]
    fn the_oldest_live_turn_is_the_landing_that_runs() {
        let turn = |seq: u64| Ticket {
            turn: true,
            weight: 0,
            ..waiting(seq, "a", 0, Rank::Landing)
        };
        let tickets = vec![turn(7), turn(3), turn(9)];
        assert!(turn_is(&tickets, 3));
        assert!(!turn_is(&tickets, 7));
        // The one that was running is gone; the next in arrival order
        // takes it, not the newest.
        let rest = vec![tickets[0].clone(), tickets[2].clone()];
        assert!(turn_is(&rest, 7));
        assert!(!turn_is(&[], 7), "a turn nobody holds is nobody's");
    }
}
