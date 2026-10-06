//! The pace's rules played out over time by hand: a read takes as long as
//! it weighs, and the world wakes the pace exactly when it asks to be
//! woken unless a test says otherwise.

use std::collections::HashMap;
use std::time::Duration;

use super::pace::*;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

/// What a read of this tree weighs, by its number from 0.
type OwnWeights = Box<dyn FnMut(usize) -> Duration>;

struct World {
    pacer: Pacer,
    at: Duration,
    own_weights: OwnWeights,
    worktree_weights: HashMap<String, Duration>,
    /// The read under way and when it ends.
    own_end: Option<(Duration, Duration)>,
    worktree_end: Option<(String, Duration, Duration)>,
    own_starts: Vec<Duration>,
    worktree_starts: Vec<(String, Duration)>,
    /// When this tree's reads ended.
    own_ends: Vec<Duration>,
}

impl World {
    fn new(pace: WorktreesPace, own_weights: OwnWeights) -> Self {
        let mut pacer = Pacer::new(pace);
        pacer.set_active(true, Duration::ZERO);
        Self {
            pacer,
            at: Duration::ZERO,
            own_weights,
            worktree_weights: HashMap::new(),
            own_end: None,
            worktree_end: None,
            own_starts: Vec::new(),
            worktree_starts: Vec::new(),
            own_ends: Vec::new(),
        }
    }

    fn steady(weight: Duration) -> Self {
        Self::new(WorktreesPace::Auto, Box::new(move |_| weight))
    }

    fn with_worktrees(mut self, worktrees: &[(&str, Duration)]) -> Self {
        let keys: Vec<String> = worktrees.iter().map(|(k, _)| (*k).to_string()).collect();
        self.pacer.list(&keys, self.at);
        for (key, weight) in worktrees {
            self.worktree_weights.insert((*key).to_string(), *weight);
        }
        self
    }

    /// Starts what the pace says to start at the moment the world is at.
    fn start_due(&mut self) {
        for start in self.pacer.starts(self.at) {
            match start {
                Start::Own { .. } => {
                    let weight = (self.own_weights)(self.own_starts.len());
                    self.own_starts.push(self.at);
                    self.own_end = Some((self.at + weight, weight));
                }
                Start::Worktree(key) => {
                    let weight = self.worktree_weights[&key];
                    self.worktree_starts.push((key.clone(), self.at));
                    self.worktree_end = Some((key, self.at, self.at + weight));
                }
            }
        }
    }

    /// Plays the world forward until `until`, waking the pace when it asks.
    fn run_until(&mut self, until: Duration) {
        self.run_with(until, |wake| wake);
    }

    /// As [`Self::run_until`], with `late` saying when a wake asked for at
    /// a moment actually comes.
    fn run_with(&mut self, until: Duration, late: impl Fn(Duration) -> Duration) {
        loop {
            self.start_due();
            let own_end = self.own_end.map(|(end, _)| end);
            let worktree_end = self.worktree_end.as_ref().map(|(_, _, end)| *end);
            let wake = self.pacer.next_wake().map(&late);
            let next = [own_end, worktree_end, wake]
                .into_iter()
                .flatten()
                .min()
                .expect("something is always waiting while the page is on screen");
            if next > until {
                self.at = until;
                return;
            }
            self.at = next.max(self.at);
            if let Some((end, weight)) = self.own_end
                && end <= self.at
            {
                self.own_end = None;
                self.own_ends.push(end);
                self.pacer.own_ended(end, Some(weight));
            }
            if let Some((key, began, end)) = self.worktree_end.clone()
                && end <= self.at
            {
                self.worktree_end = None;
                let weight = self.worktree_weights[&key];
                self.pacer.worktree_ended(&key, began, Some(weight));
            }
        }
    }

    /// The gaps between this tree's read starts.
    fn own_gaps(&self) -> Vec<Duration> {
        self.own_starts.windows(2).map(|w| w[1] - w[0]).collect()
    }

    fn worktree_starts_of(&self, key: &str) -> Vec<Duration> {
        self.worktree_starts
            .iter()
            .filter(|(k, _)| k == key)
            .map(|(_, at)| *at)
            .collect()
    }

    fn worktree_gaps(&self, key: &str) -> Vec<Duration> {
        let starts = self.worktree_starts_of(key);
        starts.windows(2).map(|w| w[1] - w[0]).collect()
    }
}

/// Equal to the millisecond: the shares are floats.
fn close(a: Duration, b: Duration) -> bool {
    a.abs_diff(b) <= ms(1)
}

fn own_interval_for(weight: Duration) -> Duration {
    weight.div_f64(OWN_SHARE).clamp(OWN_FLOOR, OWN_CEILING)
}

#[test]
fn a_light_tree_is_read_at_the_floor() {
    let light = OWN_FLOOR.mul_f64(OWN_SHARE) / 5;
    let mut world = World::steady(light);
    world.run_until(secs(120));
    let gaps = world.own_gaps();
    assert!(gaps.len() > 10, "read again and again: {gaps:?}");
    assert!(gaps.iter().all(|gap| close(*gap, OWN_FLOOR)), "{gaps:?}");
}

#[test]
fn a_steadily_heavy_tree_is_read_at_the_ceiling() {
    let heavy = OWN_CEILING.mul_f64(OWN_SHARE) * 2;
    let mut world = World::steady(heavy);
    world.run_until(secs(300));
    let gaps = world.own_gaps();
    // The first read follows the opening's floor; every one after is paced.
    assert!(
        gaps.iter().skip(1).all(|gap| close(*gap, OWN_CEILING)),
        "{gaps:?}"
    );
}

#[test]
fn a_tree_between_the_ends_is_read_at_its_weight_over_the_share() {
    let weight = (OWN_FLOOR + OWN_CEILING).mul_f64(OWN_SHARE) / 2;
    let mut world = World::steady(weight);
    world.run_until(secs(300));
    let gaps = world.own_gaps();
    let expected = own_interval_for(weight);
    assert!(expected > OWN_FLOOR && expected < OWN_CEILING);
    assert!(
        gaps.iter().skip(1).all(|gap| close(*gap, expected)),
        "{gaps:?}"
    );
}

#[test]
fn a_read_longer_than_the_ceiling_is_followed_after_the_rest() {
    let longer = OWN_CEILING + secs(1);
    let mut world = World::steady(longer);
    world.run_until(secs(200));
    let gaps = world.own_gaps();
    assert!(gaps.len() > 3, "{gaps:?}");
    assert!(
        gaps.iter()
            .skip(1)
            .all(|gap| close(*gap, longer + OWN_REST)),
        "no read starts under another, and none waits more than the rest: {gaps:?}"
    );
}

#[test]
fn light_reads_among_heavy_ones_do_not_bring_the_pace_back() {
    let pattern = [ms(50), ms(900), ms(900)];
    let mut world = World::new(WorktreesPace::Auto, Box::new(move |n| pattern[n % 3]));
    world.run_until(secs(600));
    let gaps = world.own_gaps();
    // From the first heavy read on, the light one in each three takes the
    // weight held down by one release step only.
    let lowest = own_interval_for(ms(900).mul_f64(RELEASE));
    assert!(lowest > OWN_FLOOR * 2, "the pattern means something");
    assert!(
        gaps.iter().skip(2).all(|gap| *gap + ms(1) >= lowest),
        "a light read let the pace back in: {gaps:?}"
    );
}

#[test]
fn a_sudden_crowd_pushes_the_pace_out_at_once_and_it_comes_back_in_steps() {
    let crowded = OWN_CEILING.mul_f64(OWN_SHARE) * 4 / 5;
    let light = ms(20);
    let weights = move |n: usize| if (5..10).contains(&n) { crowded } else { light };
    let mut world = World::new(WorktreesPace::Auto, Box::new(weights));
    world.run_until(secs(400));
    let gaps = world.own_gaps();
    assert!(
        gaps[..5].iter().all(|gap| close(*gap, OWN_FLOOR)),
        "{gaps:?}"
    );
    assert!(
        close(gaps[5], own_interval_for(crowded)),
        "the first crowded read pushes the next out at once: {gaps:?}"
    );
    let after = &gaps[9..];
    assert!(
        after.windows(2).all(|w| w[1] <= w[0]),
        "and the pace comes back without jumping: {gaps:?}"
    );
    // The first of these is the last crowded read's own gap.
    let steps_back = after
        .iter()
        .take_while(|gap| !close(**gap, OWN_FLOOR))
        .count();
    assert!(
        (2..=4).contains(&steps_back),
        "a step at a time, not all at once and not forever: {gaps:?}"
    );
}

#[test]
fn a_late_wake_leaves_the_pace_where_the_reads_put_it() {
    let heavy = OWN_CEILING.mul_f64(OWN_SHARE) * 2;
    let mut world = World::steady(heavy);
    // Every wake comes half a minute late: a crowded or sleeping machine.
    world.run_with(secs(600), |wake| wake + secs(30));
    let gaps = world.own_gaps();
    assert!(
        gaps.iter()
            .skip(1)
            .all(|gap| close(*gap, OWN_CEILING + secs(30))),
        "lateness neither resets the weight nor adds reads: {gaps:?}"
    );
}

#[test]
fn asked_again_during_a_read_starts_the_next_as_it_ends() {
    let mut world = World::steady(secs(2));
    world.run_until(ms(500));
    assert_eq!(
        world.own_starts,
        vec![Duration::ZERO],
        "the first read is under way"
    );
    world.pacer.ask_now(world.at);
    world.run_until(secs(3));
    assert_eq!(
        world.own_starts,
        vec![Duration::ZERO, secs(2)],
        "the next follows the end with no rest and no interval"
    );
}

#[test]
fn the_opening_counts_as_a_read_begun() {
    let mut pacer = Pacer::new(WorktreesPace::Auto);
    pacer.opened(Duration::ZERO);
    pacer.set_active(true, secs(1));
    assert!(
        pacer.starts(secs(1)).is_empty(),
        "nothing is due a second after opening"
    );
    assert_eq!(pacer.next_wake(), Some(OWN_FLOOR));
}

#[test]
fn a_page_off_screen_reads_nothing_and_back_on_reads_what_fell_due() {
    let mut world = World::steady(ms(20));
    world.run_until(secs(12));
    let before = world.own_starts.len();
    world.pacer.set_active(false, world.at);
    assert_eq!(
        world.pacer.next_wake(),
        None,
        "nothing waits on time off screen"
    );
    assert!(world.pacer.starts(secs(600)).is_empty());
    world.at = secs(600);
    world.pacer.set_active(true, world.at);
    world.run_until(secs(600));
    assert_eq!(world.own_starts.len(), before + 1, "one read, at once");
    assert_eq!(world.own_starts.last(), Some(&secs(600)));
}

#[test]
fn a_slow_worktree_does_not_hold_back_the_fast_ones() {
    let slow = secs(3);
    let mut world =
        World::steady(ms(50)).with_worktrees(&[("a", ms(100)), ("b", ms(100)), ("c", slow)]);
    world.run_until(secs(600));
    for fast in ["a", "b"] {
        let gaps = world.worktree_gaps(fast);
        assert!(gaps.len() > 30, "{fast} is read again and again: {gaps:?}");
        assert!(
            gaps.iter()
                .all(|gap| *gap <= WORKTREE_FLOOR + OWN_FLOOR + slow),
            "{fast} waits for the slow worktree one read at most: {gaps:?}"
        );
    }
    let slow_gaps = world.worktree_gaps("c");
    assert!(
        slow_gaps.len() > 5,
        "the slow worktree is read too: {slow_gaps:?}"
    );
    assert!(
        slow_gaps
            .iter()
            .all(|gap| *gap <= WORKTREE_CEILING + OWN_FLOOR),
        "within its ceiling, give or take this tree's read: {slow_gaps:?}"
    );
    assert!(
        slow_gaps.iter().all(|gap| *gap >= WORKTREE_CEILING),
        "and no sooner: its weight sets it there: {slow_gaps:?}"
    );
}

#[test]
fn a_worktree_slower_than_the_gap_is_read_after_this_tree_s_read() {
    let mut world = World::steady(ms(50)).with_worktrees(&[("slow", secs(6))]);
    world.run_until(secs(600));
    let starts = world.worktree_starts_of("slow");
    assert!(starts.len() > 5, "read again and again: {starts:?}");
    for start in &starts {
        assert!(
            world.own_ends.iter().any(|end| close(*end, *start)),
            "each read goes as this tree's read ends: {start:?} not in {:?}",
            world.own_ends
        );
    }
}

/// A read asked for at once says so — it lists the stashes, which no paced
/// read does — whether it starts at the ask or as the read under way ends.
#[test]
fn a_read_asked_for_at_once_says_so_and_a_paced_one_does_not() {
    let mut pacer = Pacer::new(WorktreesPace::Off);
    pacer.opened(Duration::ZERO);
    pacer.set_active(true, Duration::ZERO);
    pacer.ask_now(secs(1));
    assert_eq!(
        pacer.starts(secs(1)),
        vec![Start::Own { with_stashes: true }]
    );
    pacer.ask_now(secs(2));
    pacer.own_ended(secs(3), Some(ms(10)));
    assert_eq!(
        pacer.starts(secs(3)),
        vec![Start::Own { with_stashes: true }],
        "asked while one ran, the next is the asked one"
    );
    pacer.own_ended(secs(4), Some(ms(10)));
    let next = pacer.next_wake().expect("a paced read is due");
    assert_eq!(
        pacer.starts(next),
        vec![Start::Own {
            with_stashes: false
        }]
    );
}

/// Turned away before it read anything (a write of this tree runs), the
/// read the window asked for leaves its stashes to the next read — not to
/// the next time the window comes back.
#[test]
fn stashes_a_turned_away_read_was_to_list_go_to_the_next() {
    let mut pacer = Pacer::new(WorktreesPace::Off);
    pacer.opened(Duration::ZERO);
    pacer.set_active(true, Duration::ZERO);
    pacer.ask_now(secs(1));
    assert_eq!(
        pacer.starts(secs(1)),
        vec![Start::Own { with_stashes: true }]
    );
    pacer.own_refused(secs(1), true);
    let next = pacer.next_wake().expect("a paced read is due");
    assert!(
        next > secs(1),
        "not asked again at once: it would be turned away again"
    );
    assert_eq!(pacer.starts(next), vec![Start::Own { with_stashes: true }]);
}

/// The opening's read does not push out a read the window asked for
/// before it.
#[test]
fn the_opening_keeps_a_read_already_asked_for() {
    let mut pacer = Pacer::new(WorktreesPace::Off);
    pacer.set_active(true, Duration::ZERO);
    pacer.ask_now(Duration::ZERO);
    pacer.opened(ms(10));
    assert_eq!(
        pacer.starts(ms(10)),
        vec![Start::Own { with_stashes: true }]
    );
}

/// A worktree asked for again while its own read runs — the listing moved
/// past that read — is due again as the read ends; one asked for before its
/// read began is answered by it.
#[test]
fn a_worktree_asked_for_during_its_read_is_due_again_as_it_ends() {
    let mut pacer = Pacer::new(WorktreesPace::Auto);
    pacer.opened(Duration::ZERO);
    pacer.set_active(true, Duration::ZERO);
    pacer.list(&["a".to_string()], Duration::ZERO);
    pacer.worktree_stale("a", secs(1));
    pacer.worktree_ended("a", secs(2), Some(ms(10)));
    assert!(
        pacer.next_wake().is_some_and(|wake| wake > secs(2)),
        "asked before the read began, the read answered it"
    );
    pacer.worktree_stale("a", secs(4));
    pacer.worktree_ended("a", secs(3), Some(ms(10)));
    assert_eq!(
        pacer.next_wake(),
        Some(secs(3)),
        "asked after the read began, it is owed at once"
    );
}

#[test]
fn a_worktree_never_read_waits_for_this_tree_s_read_once() {
    let mut pacer = Pacer::new(WorktreesPace::Auto);
    pacer.opened(Duration::ZERO);
    pacer.set_active(true, secs(1));
    pacer.list(&["new".to_string()], secs(1));
    assert!(
        pacer.starts(secs(1)).is_empty(),
        "nothing says it would fit"
    );
    assert_eq!(
        pacer.starts(OWN_FLOOR),
        vec![Start::Own {
            with_stashes: false
        }]
    );
    pacer.own_ended(OWN_FLOOR + ms(10), Some(ms(10)));
    assert_eq!(
        pacer.starts(OWN_FLOOR + ms(10)),
        vec![Start::Worktree("new".to_string())],
        "it goes as this tree's read ends"
    );
}

#[test]
fn a_worktree_that_would_pass_its_ceiling_waiting_goes_now() {
    let mut pacer = Pacer::new(WorktreesPace::Auto);
    pacer.set_active(true, Duration::ZERO);
    pacer.list(&["c".to_string()], Duration::ZERO);
    // Read once, heavily, at 0: it falls due at its ceiling.
    pacer.worktree_ended("c", Duration::ZERO, Some(secs(20)));
    // This tree's read is overdue by then and starts beside it: the worktree
    // would wait for that read's end, a second past its ceiling.
    pacer.own_ended(Duration::ZERO, Some(secs(1)));
    let due = WORKTREE_CEILING;
    let starts = pacer.starts(due);
    assert!(
        starts.contains(&Start::Worktree("c".to_string())),
        "waiting for this tree's next read would pass the ceiling: {starts:?}"
    );
}

#[test]
fn worktrees_spread_out_as_they_grow_in_number() {
    let weight = ms(300);
    let few: Vec<(String, Duration)> = (0..3).map(|i| (format!("c{i}"), weight)).collect();
    let many: Vec<(String, Duration)> = (0..12).map(|i| (format!("c{i}"), weight)).collect();
    let gaps_of = |worktrees: &[(String, Duration)]| {
        let named: Vec<(&str, Duration)> =
            worktrees.iter().map(|(k, w)| (k.as_str(), *w)).collect();
        let mut world = World::steady(ms(50)).with_worktrees(&named);
        world.run_until(secs(900));
        world.worktree_gaps("c0")
    };
    let few_gaps = gaps_of(&few);
    let many_gaps = gaps_of(&many);
    let few_target = (weight * 3)
        .div_f64(WORKTREE_SHARE)
        .clamp(WORKTREE_FLOOR, WORKTREE_CEILING);
    let many_target = (weight * 12)
        .div_f64(WORKTREE_SHARE)
        .clamp(WORKTREE_FLOOR, WORKTREE_CEILING);
    assert!(few_target < many_target, "the count means something here");
    assert!(
        few_gaps
            .iter()
            .skip(1)
            .all(|gap| *gap >= few_target && *gap <= few_target + OWN_FLOOR),
        "{few_gaps:?}"
    );
    assert!(
        many_gaps
            .iter()
            .skip(1)
            .all(|gap| *gap >= many_target && *gap <= WORKTREE_CEILING + OWN_FLOOR),
        "{many_gaps:?}"
    );
}

#[test]
fn off_reads_no_worktree_and_fixed_ignores_the_weight() {
    let mut off =
        World::new(WorktreesPace::Off, Box::new(|_| ms(50))).with_worktrees(&[("a", ms(10))]);
    off.run_until(secs(120));
    assert!(off.worktree_starts.is_empty());
    assert!(off.own_starts.len() > 10, "this tree is read all the same");

    let every = secs(20);
    let mut fixed = World::new(WorktreesPace::Fixed(every), Box::new(|_| ms(50)))
        .with_worktrees(&[("a", secs(2))]);
    fixed.run_until(secs(300));
    let gaps = fixed.worktree_gaps("a");
    assert!(
        gaps.iter()
            .all(|gap| *gap >= every && *gap <= every + OWN_FLOOR),
        "{gaps:?}"
    );
}

#[test]
fn a_pass_over_the_worktrees_starts_none_of_the_pace_s_own() {
    let mut pacer = Pacer::new(WorktreesPace::Auto);
    pacer.set_active(true, Duration::ZERO);
    pacer.list(&["a".to_string()], Duration::ZERO);
    pacer.worktree_ended("a", Duration::ZERO, Some(ms(10)));
    pacer.set_passing(true);
    assert!(
        !pacer
            .starts(WORKTREE_CEILING)
            .contains(&Start::Worktree("a".to_string())),
        "the pass is reading them"
    );
    // The pass read it at the ceiling: due again a floor later (this
    // tree's read began then too, and is still out).
    pacer.worktree_ended("a", WORKTREE_CEILING, Some(ms(10)));
    pacer.set_passing(false);
    assert_eq!(pacer.next_wake(), Some(WORKTREE_CEILING + WORKTREE_FLOOR));
}

/// Turned away before it began (a pass took the worktrees between the start
/// and the read), a worktree is not asked for again at once: that would be
/// turned away again for as long as the pass lasts.
#[test]
fn a_worktree_turned_away_before_its_read_waits_for_this_tree_s_read() {
    let mut pacer = Pacer::new(WorktreesPace::Auto);
    pacer.set_active(true, Duration::ZERO);
    pacer.list(&["a".to_string()], Duration::ZERO);
    pacer.worktree_ended("a", Duration::ZERO, Some(ms(10)));
    // This tree's next read falls due after the worktree's.
    let own_weight = (OWN_FLOOR * 2).mul_f64(OWN_SHARE);
    assert_eq!(
        pacer.starts(Duration::ZERO),
        vec![Start::Own {
            with_stashes: false
        }]
    );
    pacer.own_ended(Duration::ZERO, Some(own_weight));
    let due = WORKTREE_FLOOR;
    assert_eq!(pacer.starts(due), vec![Start::Worktree("a".to_string())]);
    pacer.worktree_skipped("a");
    let next_own = own_interval_for(own_weight);
    assert_eq!(
        pacer.next_wake(),
        Some(next_own),
        "nothing to start again before this tree's read"
    );
    assert!(pacer.starts(due).is_empty());
    assert_eq!(
        pacer.starts(next_own),
        vec![Start::Own {
            with_stashes: false
        }]
    );
    pacer.own_ended(next_own + ms(10), Some(own_weight));
    assert_eq!(
        pacer.starts(next_own + ms(10)),
        vec![Start::Worktree("a".to_string())],
        "and goes as this tree's next read ends"
    );
}

/// A floor set lower reads a light tree sooner; a ceiling set higher lets
/// a heavy one wait longer — and a worktree's interval keeps to its own
/// pair.
#[test]
fn the_floors_and_ceilings_a_person_sets_are_the_ones_kept() {
    let bounds = PaceBounds {
        own_floor: secs(2),
        own_ceiling: secs(60),
        worktree_floor: secs(3),
        worktree_ceiling: secs(90),
    };
    let read = ms(20);
    let mut light = World::steady(read).with_worktrees(&[("a", ms(10))]);
    light.pacer.set_bounds(bounds);
    light.run_until(secs(120));
    // The rest is never more than the floor, so a floor under it is kept,
    // counted from a quick read's end at the most.
    assert!(
        light
            .own_gaps()
            .iter()
            .skip(1)
            .all(|gap| *gap + ms(1) >= secs(2) && *gap <= secs(2) + read + ms(1)),
        "{:?}",
        light.own_gaps()
    );
    assert!(
        light
            .worktree_gaps("a")
            .iter()
            .skip(1)
            .all(|gap| *gap >= secs(3) && *gap <= secs(3) + secs(2)),
        "{:?}",
        light.worktree_gaps("a")
    );

    let mut heavy = World::steady(secs(2));
    heavy.pacer.set_bounds(bounds);
    heavy.run_until(secs(600));
    let expected = secs(2).div_f64(OWN_SHARE).min(secs(60));
    assert!(expected > OWN_CEILING, "past the default ceiling");
    assert!(
        heavy
            .own_gaps()
            .iter()
            .skip(1)
            .all(|gap| close(*gap, expected)),
        "{:?}",
        heavy.own_gaps()
    );
}

#[test]
fn a_pair_asked_for_is_held_in_range_with_the_ceiling_never_under_the_floor() {
    assert_eq!(pace_bounds_secs(0, 0), (PACE_MIN_SECS, PACE_MIN_SECS));
    assert_eq!(
        pace_bounds_secs(10, 4),
        (10, 10),
        "a ceiling under the floor is the floor"
    );
    assert_eq!(pace_bounds_secs(5, u32::MAX), (5, PACE_MAX_SECS));
    assert_eq!(
        pace_bounds_secs(u32::MAX, 0),
        (PACE_MAX_SECS, PACE_MAX_SECS)
    );
    assert_eq!(pace_bounds_secs(5, 15), (5, 15));
}

/// This tree waiting long on a slow pace, and a worktree on an hour's fixed
/// interval: the reads fall due where the pace would have put them.
fn waiting_long() -> Pacer {
    let mut pacer = Pacer::new(WorktreesPace::Fixed(secs(3600)));
    let slow = PaceBounds {
        own_floor: secs(60),
        own_ceiling: secs(60),
        ..PaceBounds::default()
    };
    pacer.set_bounds(slow);
    pacer.opened(Duration::ZERO);
    pacer.set_active(true, Duration::ZERO);
    pacer.list(&["a".to_string()], Duration::ZERO);
    pacer.worktree_ended("a", Duration::ZERO, Some(ms(10)));
    assert_eq!(pacer.next_wake(), Some(secs(60)), "the long wait is set");
    pacer
}

/// A worktree read an hour apart, turned to read automatically ten seconds
/// in: it is due where the automatic pace puts it from its last read, not
/// at the hour.
#[test]
fn a_wait_already_set_is_planned_again_when_the_worktrees_reading_changes() {
    let mut pacer = waiting_long();
    pacer.set_pace(WorktreesPace::Auto, secs(10));
    assert_eq!(pacer.next_wake(), Some(WORKTREE_FLOOR));
    assert_eq!(
        pacer.starts(secs(10)),
        vec![Start::Worktree("a".to_string())],
        "overdue under the new reading, it is read now"
    );
}

/// Shorter bounds set while this tree waits on long ones: its next read is
/// planned again from where its last began.
#[test]
fn a_wait_already_set_is_planned_again_when_the_bounds_change() {
    let mut pacer = waiting_long();
    pacer.set_bounds(PaceBounds::default());
    assert_eq!(pacer.next_wake(), Some(OWN_FLOOR));

    // This tree kept on its long wait, so only the worktree's bounds move.
    let own_slow = PaceBounds {
        own_floor: secs(60),
        own_ceiling: secs(60),
        ..PaceBounds::default()
    };
    let mut worktrees = Pacer::new(WorktreesPace::Auto);
    worktrees.set_bounds(PaceBounds {
        worktree_floor: secs(3600),
        worktree_ceiling: secs(3600),
        ..own_slow
    });
    worktrees.opened(Duration::ZERO);
    worktrees.set_active(true, Duration::ZERO);
    worktrees.list(&["a".to_string()], Duration::ZERO);
    worktrees.worktree_ended("a", Duration::ZERO, Some(ms(10)));
    assert_eq!(worktrees.next_wake(), Some(secs(60)));
    worktrees.set_bounds(own_slow);
    assert_eq!(
        worktrees.starts(secs(10)),
        vec![Start::Worktree("a".to_string())],
        "the worktree is overdue under the shorter bounds"
    );
}

/// Turned off and on again, the worktrees' rows came down with the off:
/// each worktree is read again at once (one at a time, as ever) to put
/// them back.
#[test]
fn worktrees_turned_back_on_are_due_at_once() {
    let mut pacer = waiting_long();
    pacer.set_pace(WorktreesPace::Off, secs(1));
    pacer.set_pace(WorktreesPace::Auto, secs(2));
    assert_eq!(pacer.next_wake(), Some(secs(2)));
}
