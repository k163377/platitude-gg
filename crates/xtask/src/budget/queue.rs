//! Who is handed the machine next, as arithmetic over what the ledger
//! holds: the order the waiting units stand in, and whether the one
//! asking may start now.
//!
//! Nothing here touches a file or a clock: the ledger is read under its
//! lock (`ledger::Pool::look`) and answered here, so the rule is a
//! function of a list and is tested as one.
//!
//! **The rule is one sentence** ([`admits`]): a waiting unit may start
//! when it stands in the prefix of the ordered queue that fits in what is
//! free, and the prefix stops at the first unit that does not fit. So a
//! waiting landing or launch keeps lower ranks out of the room it is
//! short of, with no separate reservation; and there is no backfill,
//! which would let a seat's endless one-weight verbs walk over a
//! four-weight `cargo test` forever. With landings first and nothing
//! preempted, ordinary work behind an unbroken run of landings waits as
//! long as they take (反映前テストの機械化.md §機械の予算と優先キュー).

use std::collections::BTreeMap;

/// Where a unit stands in the one queue every seat on the machine shares.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Rank {
    /// A landing's unit: `land` moves main, and the seats' gates rebase
    /// over what lands anyway.
    Landing,
    /// A window the user asked for (`cargo xtask launch`): somebody is
    /// waiting at the screen, which no test is. Only the build and the
    /// start are counted; the window holds none of the machine.
    Launch,
    /// Everything else — a seat's own gate, whatever its flags.
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

    /// The word back; an unreadable one counts as ordinary work.
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
    /// What the unit takes out of the budget while it runs; zero for a
    /// turn.
    pub weight: u32,
    /// The whole its owner asks the machine for. The pool is the largest
    /// any live ticket names ([`budget_of`]), so a gate told a wider
    /// `--jobs` widens it for all while it runs.
    pub budget: u32,
    pub rank: Rank,
    /// A landing's turn: outside the budget, and held for the whole of
    /// one landing ([`turn_is`]).
    pub turn: bool,
    /// Which tree the unit belongs to, which is what the fairness is
    /// between.
    pub seat: String,
    pub what: String,
    /// Whether it has been handed the machine.
    pub running: bool,
    /// When it was handed the machine; zero while it is still waiting.
    /// Dated from the registration instead, a unit admitted after a long
    /// queue would be past `ledger::LEFTOVER_CEILING` on its first second.
    pub ran_since: u64,
    /// The process this unit started; zero before that and for a unit
    /// that starts none. Read only when the owner is killed: its lock
    /// frees, but what it started is still on the machine
    /// (`ledger::Pool::leftover`).
    pub child: u32,
    /// The program started at [`Ticket::child`]: a pid is handed out
    /// again, and the name tells the work from a stranger that inherited
    /// it (`subprocess::image_still_at`). Empty for a ticket that started
    /// nothing.
    pub child_name: String,
    /// The second a reader last asked after [`Ticket::child`] — paced to
    /// once a second (`ledger::Pool::leftover`).
    pub probed: u64,
    /// Whether this late leftover has been reported, so the line is said
    /// once machine-wide.
    pub told: bool,
    /// Read off the ledger: the owner is gone and [`Ticket::child`] is
    /// not.
    pub orphaned: bool,
    /// Read off the ledger too: a leftover past the longest a step may
    /// run. Its room stays held (`ledger::Pool::leftover`); the standing
    /// flags it as the one thing a person may have to end.
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

/// The pool: the largest budget any live ticket names, never less than
/// `mine` — a smaller ask does not shrink the machine under the others.
pub(crate) fn budget_of(tickets: &[Ticket], mine: u32) -> u32 {
    tickets.iter().map(|t| t.budget).fold(mine, u32::max)
}

/// What each seat has been handed, by rank: the running total the
/// fairness is read off ([`order`]), kept in the ledger (`ledger::SERVED`).
pub(crate) type Served = BTreeMap<(Rank, String), u64>;

/// The waiting units in the order they are handed the machine: the
/// higher rank first, then the seat served the least weight, then
/// arrival.
///
/// **A seat's place is what it has been served**, a count that survives
/// the unit that earned it: ordering by place in the seat's own queue, or
/// by what it is running now, hands a seat with two waiting units the
/// machine twice in a row.
///
/// A seat that arrives late starts at the least any seat here has
/// ([`floor_of`]): at zero it would take the machine until it caught up,
/// at its old count it would pay a debt it never asked for. Counted
/// inside a rank, so a seat's landing costs its ordinary work no place.
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

/// The least any seat with a live ticket has been served — where a seat
/// with no count of its own starts.
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

/// Whether it is `seq`'s turn: the oldest live turn is the one landing
/// that runs. Two at once would each gate against a main the other is
/// about to move.
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
    // Turns hold none of the machine and are in neither list above, so
    // they are listed on their own.
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

    #[test]
    fn a_window_the_user_asked_for_goes_behind_a_landing_and_ahead_of_the_tests() {
        let tickets = vec![
            waiting(1, "b", 1, Rank::Normal),
            waiting(2, "c", 1, Rank::Launch),
            waiting(3, "a", 1, Rank::Landing),
        ];
        assert_eq!(seqs(&order(&tickets, &fresh())), vec![3, 2, 1]);
        // A launch short of room keeps the tests behind it out, and lets
        // the landing past.
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

    /// The count must survive the served unit finishing: once a's first
    /// ticket is gone, an order read off the tickets alone hands a the
    /// next room too.
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
        // (`ledger::Pool::register`); a's has moved by one.
        let after_one = served_of(&[("a", 1), ("b", 0)]);
        let running_now = vec![running(1, "a", 1), queued[1].clone(), queued[2].clone()];
        assert_eq!(seqs(&order(&running_now, &after_one)), vec![3, 2]);
        // a's first has finished: both seats hold nothing, and a's
        // remaining unit is the older.
        let done = vec![queued[1].clone(), queued[2].clone()];
        assert_eq!(
            seqs(&order(&done, &after_one)),
            vec![3, 2],
            "the seat that was already served took the next room too"
        );
    }

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

    /// The number a seat's row is made with when it joins the queue
    /// (`ledger::Pool::register`).
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
        // The holder finishes.
        let freed = vec![tickets[1].clone(), tickets[2].clone()];
        assert!(
            admits(&freed, 24, 2, &fresh()),
            "the landing takes the room"
        );
    }

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
        // The running one is gone; the next in arrival order takes it.
        let rest = vec![tickets[0].clone(), tickets[2].clone()];
        assert!(turn_is(&rest, 7));
        assert!(!turn_is(&[], 7), "a turn nobody holds is nobody's");
    }
}
