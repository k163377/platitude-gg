//! The one write a run is waiting for, kept by the id its own ask was
//! given.
//!
//! **Why a watch.** Every event about a write carries an id, and none of
//! them says which is *yours*; the id that does is the one
//! [`RepoSession::write`] hands back to the caller that asked.
//! So the ask keeps it here, inside the call that returns it
//! (`RepoTab::ask_session`), and every question afterwards is equality
//! against that one id. **Ids are not a sequence**: a caller can be
//! numbered and then lose its turn to another (`OperationId` — compared by
//! equality and nothing else), so `>=` would let somebody else's answer
//! stand in for this one.
//!
//! **Why the tab keeps it.** The caller is a press on the product's own
//! input path, which answers nothing — the harness presses a button.
//! The tab is where that press lands and where the answers come back,
//! and it lives on one thread, so the arm, the ask and the read are a
//! single uninterrupted stretch.
//!
//! **Two halves, and the contract is the join.** [`WatchedWrite`] is the
//! id: which write, and how far along. [`WriteWatch`] adds what the run
//! said about it — whether it announced a press, and whether it is still
//! waiting — because the same write means different things depending on
//! that. What the watch carries is often one nobody pressed for (the read
//! a menu asks for on its way open, the fetch a refused push asks for
//! itself), and giving *that* up for the press about to be made is
//! ordinary; giving up one the run is waiting out is the breach.
//!
//! **Waiting is waiting, whatever does it.** A verb that waits from a
//! sampler of its own is waiting exactly as much as one waiting on the
//! shared barrier, so what this side reads is ids and stages.
//!
//! **A breach is the end of the run.** Once said, the contract refuses to
//! be armed again and refuses to let the run complete: a run that could
//! start over would photograph a page its broken write never reached, and
//! one that could finish would be green.

/// What the watch is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum WatchedWrite {
    /// Nobody is watching: the ordinary application, and a run before its
    /// first arm.
    #[default]
    Asleep,
    /// The next write this tab is asked to make is the one to keep.
    ///
    /// `turned_down` is an ask the queue took nothing for — the session is
    /// closed. A press that has not gone in yet is a different
    /// state, and one the watch cannot see.
    Armed { turned_down: bool },
    /// That ask was taken under this id, and these are the two boundaries
    /// behind it (`session::write` の三境界).
    Held {
        id: u64,
        answered: bool,
        settled: bool,
    },
}

impl WatchedWrite {
    /// **The next ask is the one to keep.**
    ///
    /// Always allowed, and **the contract is the run's to hold**:
    /// what the watch is carrying may be a write nobody pressed for — the
    /// read a menu asks for on its way open, the fetch a refused push
    /// asks for itself — and throwing that over for the press about to be
    /// made is the ordinary thing to do. Only the run knows whether the
    /// write it is giving up is one it was waiting for
    /// (`AutoActDriver.beginWrite`).
    pub(super) fn arm(&mut self) {
        *self = Self::Armed { turned_down: false };
    }

    /// The tab asked the queue for a write and this is what came back.
    pub(super) fn asked(&mut self, id: Option<u64>) {
        *self = match (*self, id) {
            (Self::Armed { .. }, Some(id)) => Self::Held {
                id,
                answered: false,
                settled: false,
            },
            (Self::Armed { .. }, None) => Self::Armed { turned_down: true },
            // Nothing is armed for it: the ordinary application, and the
            // writes a run causes without pressing for them. The watch
            // keeps what it holds — a second ask goes in behind the one
            // being waited on, and the queue serves them in that order.
            (held, _) => held,
        };
    }

    /// git answered the write under `id`.
    pub(super) fn answered(&mut self, id: u64) {
        if let Self::Held {
            id: mine, answered, ..
        } = self
            && *mine == id
        {
            *answered = true;
        }
    }

    /// Everything that write invalidated has been read again and
    /// published.
    pub(super) fn settled(&mut self, id: u64) {
        if let Self::Held {
            id: mine, settled, ..
        } = self
            && *mine == id
        {
            *settled = true;
        }
    }

    /// Whether the write being watched is through both boundaries.
    pub(super) fn through(&self) -> bool {
        matches!(
            self,
            Self::Held {
                answered: true,
                settled: true,
                ..
            }
        )
    }

    /// The id being watched, 0 while there is none — for the diagnosis a
    /// run that stopped answering leaves behind.
    pub(super) fn id(&self) -> u64 {
        match self {
            Self::Held { id, .. } => *id,
            _ => 0,
        }
    }

    /// Where the watch stands, for that same diagnosis. **Five words, and
    /// each says a different thing about why a barrier has not opened**:
    /// nothing is being watched / a press is expected and has not been
    /// accepted / the queue took nothing / git has it / git answered it /
    /// it is through.
    pub(super) fn stage(&self) -> &'static str {
        match self {
            Self::Asleep => "asleep",
            Self::Armed { turned_down: false } => "armed",
            Self::Armed { turned_down: true } => "turned-down",
            Self::Held {
                answered: false, ..
            } => "held",
            Self::Held {
                answered: true,
                settled: false,
                ..
            } => "answered",
            Self::Held { .. } => "settled",
        }
    }
}

/// What the run has said about the write the watch is carrying.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Stage {
    /// The run is under way and has announced nothing: the dispatch armed
    /// for whatever the verb writes on its way through, and a verb that
    /// writes there has nothing left to say.
    #[default]
    Dispatched,
    /// Armed, and the input this run means to put in has not gone in yet:
    /// a sampler still looking for its row, a hold running out, a
    /// question still standing. **Nothing has been asked for yet.**
    Input,
    /// The input went in. **From here the run is waiting**, however it
    /// waits — the shared barrier or a sampler of its own.
    Pressed,
    /// The contract broke. Final: nothing arms, nothing completes.
    Broken,
}

/// A verb broke the contract. Words, which go into a failure line
/// with the verb's name beside them.
pub(super) type Breach = String;

/// The one write a run is waiting for, and what the run has said about it.
#[derive(Debug, Clone, Default)]
pub(super) struct WriteWatch {
    held: WatchedWrite,
    stage: Stage,
    /// What the run called the press it is waiting on, for the breach's
    /// words and the line a ceiling leaves.
    wanted: String,
}

impl WriteWatch {
    /// **The next ask this tab makes is the one to wait for.** Answers
    /// with the breach where arming is one, having taken the run down
    /// with it; `None` when the run may go on.
    ///
    /// **Arming over a write the run is waiting out is the breach** — its
    /// answer would be thrown away, and the run would go on to photograph
    /// a page that write had not reached. Arming over anything else is
    /// ordinary: over nothing, over a write that is through, and over one
    /// the run never said it pressed for.
    pub(super) fn arm(&mut self, what: &str) -> Option<Breach> {
        if self.stage == Stage::Broken {
            return Some(self.wanted.clone());
        }
        if self.stage == Stage::Pressed && !self.held.through() {
            let breach = format!(
                "the write pressed for ({}) was still out: {} id {} — and {what} was armed over it",
                self.wanted,
                self.held.stage(),
                self.held.id()
            );
            self.stage = Stage::Broken;
            self.wanted = breach.clone();
            return Some(breach);
        }
        self.held.arm();
        self.stage = Stage::Input;
        what.clone_into(&mut self.wanted);
        None
    }

    /// The input this run put in has gone: a press returned, a hold ran
    /// out, a question was answered. **From here the run is waiting.**
    pub(super) fn input_went(&mut self) {
        if self.stage == Stage::Input {
            self.stage = Stage::Pressed;
        }
    }

    /// **This run is done with the write it pressed for without waiting
    /// it out** — the door a composite operation goes through when it
    /// means to move on, which is what tells that apart from forgetting
    /// to wait (`publish-new-go` adds the remote, then pushes).
    ///
    /// The watch keeps the id: the write is still out there and the
    /// diagnosis should still name it. What is given up is the waiting.
    pub(super) fn let_go(&mut self) {
        if self.stage == Stage::Pressed {
            self.stage = Stage::Dispatched;
        }
    }

    /// The contract broke somewhere the run could see and this one could
    /// not — the verb says so and the run ends. Final, like a breach
    /// raised here.
    pub(super) fn broke(&mut self, breach: &str) {
        if self.stage != Stage::Broken {
            self.stage = Stage::Broken;
            breach.clone_into(&mut self.wanted);
        }
    }

    /// Whether the contract is broken. **Nothing arms and nothing
    /// completes past this**, so a run that broke cannot start over and
    /// cannot go green by another road.
    pub(super) fn broken(&self) -> bool {
        self.stage == Stage::Broken
    }

    /// Where the run stands, for the line a ceiling leaves: `dispatched` /
    /// `input` / `pressed` / `broken`. Each says a different thing about
    /// why a barrier has not opened, and "no id" says none of them.
    pub(super) fn run_stage(&self) -> &'static str {
        match self.stage {
            Stage::Dispatched => "dispatched",
            Stage::Input => "input",
            Stage::Pressed => "pressed",
            Stage::Broken => "broken",
        }
    }

    /// What the run called the press it is waiting on — or, once broken,
    /// the words the breach was said in.
    pub(super) fn wanted(&self) -> &str {
        &self.wanted
    }

    // --- the id side, passed through to the watch ----------------------

    pub(super) fn asked(&mut self, id: Option<u64>) {
        self.held.asked(id);
    }
    pub(super) fn answered(&mut self, id: u64) {
        self.held.answered(id);
    }
    pub(super) fn settled(&mut self, id: u64) {
        self.held.settled(id);
    }
    /// Whether the write being waited for is through both boundaries —
    /// **and the contract holds**: a breach is past mending.
    pub(super) fn through(&self) -> bool {
        self.stage != Stage::Broken && self.held.through()
    }
    pub(super) fn id(&self) -> u64 {
        self.held.id()
    }
    pub(super) fn stage(&self) -> &'static str {
        self.held.stage()
    }
}

#[cfg(test)]
mod watched_write_tests {
    use super::WatchedWrite;

    /// Two ids, and **the run's own is the lower one** — which is the
    /// whole point of the identity comparison. A request takes its number
    /// and then queues, so one that stalls in between lets a later ask in
    /// first: the ids and the order writes finish in are not the same
    /// list, and a barrier that read `>=` would open on the wrong one.
    const MINE: u64 = 7;
    const THEIRS: u64 = 8;

    #[test]
    fn a_watch_nobody_armed_keeps_nothing() {
        let mut watch = WatchedWrite::default();
        assert_eq!(watch.stage(), "asleep");
        watch.asked(Some(MINE));
        assert_eq!(watch.stage(), "asleep", "the ordinary application");
        assert!(!watch.through());
    }

    #[test]
    fn the_ask_that_follows_the_arm_is_the_one_kept() {
        let mut watch = WatchedWrite::default();
        watch.arm();
        assert_eq!(watch.stage(), "armed");
        assert_eq!(watch.id(), 0, "nothing has been accepted yet");

        watch.asked(Some(MINE));
        assert_eq!(watch.stage(), "held");
        assert_eq!(watch.id(), MINE);
        assert!(!watch.through());
    }

    /// **Somebody else's write, finishing first, is not this one** — and
    /// it carries the *higher* id, so an ordering comparison would have
    /// opened the barrier on it.
    #[test]
    fn another_writes_answer_does_not_pass_for_this_one() {
        let mut watch = WatchedWrite::default();
        watch.arm();
        watch.asked(Some(MINE));

        watch.answered(THEIRS);
        watch.settled(THEIRS);
        assert_eq!(watch.stage(), "held", "neither boundary is this write's");
        assert!(!watch.through());

        watch.answered(MINE);
        assert_eq!(watch.stage(), "answered");
        assert!(!watch.through(), "the reads behind it are still out");

        watch.settled(MINE);
        assert_eq!(watch.stage(), "settled");
        assert!(watch.through());
    }

    /// The same the other way round, for the arrangement where the run's
    /// own ask was numbered *after* the one that finishes first: neither
    /// direction of the comparison is the test on its own.
    #[test]
    fn nor_does_an_earlier_numbered_ones() {
        let mut watch = WatchedWrite::default();
        watch.arm();
        watch.asked(Some(THEIRS));

        watch.answered(MINE);
        watch.settled(MINE);
        assert!(!watch.through());

        watch.answered(THEIRS);
        watch.settled(THEIRS);
        assert!(watch.through());
    }

    /// A press that goes in ticks after the arm — a hold running out, a
    /// dialog being answered. The watch waits in `armed` for as long as
    /// that takes, which is what tells "the input has not landed" from
    /// "the queue took nothing".
    #[test]
    fn an_ask_that_takes_its_time_is_still_the_one_kept() {
        let mut watch = WatchedWrite::default();
        watch.arm();
        assert_eq!(watch.stage(), "armed");
        // …the hold runs, and a write nobody here pressed answers inside
        // it. The watch holds nothing, so there is nothing for it to move.
        watch.answered(THEIRS);
        watch.settled(THEIRS);
        assert_eq!(watch.stage(), "armed");

        watch.asked(Some(MINE));
        watch.answered(MINE);
        watch.settled(MINE);
        assert!(watch.through());
    }

    /// The queue took nothing — the session is closed. Said in its own
    /// word, because a run that reads "armed" is still waiting for its
    /// press to land and one that reads this is not.
    #[test]
    fn an_ask_the_queue_turned_down_says_so() {
        let mut watch = WatchedWrite::default();
        watch.arm();
        watch.asked(None);
        assert_eq!(watch.stage(), "turned-down", "an answer, not a wait");
        assert!(!watch.through());
    }

    /// **Arming again takes the watch off whatever it held, always.**
    /// Every run does it twice over — once at the dispatch, once at the
    /// press the verb actually makes — and what it gives up is as often
    /// as not a write nobody pressed for: the read a menu asks for on its
    /// way open, the fetch a refused push asks for itself. Whether giving
    /// it up is a mistake is the run's question
    /// (measured: refusing it here failed six tag verbs in the container,
    /// on a read of their own menu that the host had already settled).
    #[test]
    fn arming_again_takes_the_watch_off_whatever_it_held() {
        let mut watch = WatchedWrite::default();
        watch.arm();
        watch.arm();
        assert_eq!(watch.stage(), "armed");

        // …over a write still out, and over one that is through.
        let mut watch = WatchedWrite::default();
        watch.arm();
        watch.asked(Some(THEIRS));
        watch.arm();
        assert_eq!(watch.stage(), "armed");
        assert_eq!(watch.id(), 0, "and it is holding nothing again");

        watch.asked(Some(MINE));
        watch.answered(MINE);
        watch.settled(MINE);
        watch.arm();
        assert_eq!(watch.stage(), "armed");
    }

    /// An ask with nothing armed for it goes by: the ordinary
    /// application's writes, and the ones a run causes without pressing
    /// for them. The watch keeps what it holds — the queue is serial, so a
    /// second ask is served after the one being waited on.
    #[test]
    fn an_ask_nothing_is_armed_for_goes_by() {
        let mut watch = WatchedWrite::default();
        watch.arm();
        watch.asked(Some(MINE));
        watch.asked(Some(THEIRS));
        assert_eq!(watch.id(), MINE, "the watch keeps its own");

        watch.answered(MINE);
        watch.settled(MINE);
        watch.asked(Some(THEIRS));
        assert_eq!(watch.id(), MINE, "and goes on holding it once through");
        assert!(watch.through());
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;

    const MINE: u64 = 7;
    const THEIRS: u64 = 8;

    /// A run that armed, pressed, and is waiting on the write it pressed
    /// for — however it waits.
    fn waiting() -> WriteWatch {
        let mut watch = WriteWatch::default();
        assert_eq!(watch.arm("press"), None);
        watch.asked(Some(MINE));
        watch.input_went();
        assert_eq!(watch.run_stage(), "pressed");
        watch
    }

    /// **Waiting is waiting, whatever is doing it.** The shared barrier is
    /// one sampler among several — `line-back` waits from its own, and
    /// stages three writes through it — so what this side reads is ids
    /// and stages. Before this, the guard read the timer, and a
    /// verb waiting from its own timer could have its unfinished id thrown
    /// away with no word said (reported, reproduced).
    #[test]
    fn arming_over_the_write_this_run_is_waiting_out_is_the_breach() {
        let mut watch = waiting();
        let breach = watch.arm("second").expect("the first was still out");
        assert!(breach.contains("press"), "it names the press: {breach}");
        assert!(breach.contains("id 7"), "and the write: {breach}");
        assert!(breach.contains("second"), "and what was armed: {breach}");
        assert!(watch.broken());
    }

    /// The same once git has answered but the reads behind it are still
    /// out — the second boundary is part of what is being waited for.
    #[test]
    fn a_write_answered_but_not_settled_is_still_out() {
        let mut watch = waiting();
        watch.answered(MINE);
        assert!(watch.arm("second").is_some());
        assert!(watch.broken());
    }

    /// Through both boundaries, so the run has what it waited for and the
    /// next press is ordinary — which is how `line-back` stages three.
    #[test]
    fn arming_after_the_write_is_through_is_ordinary() {
        let mut watch = waiting();
        watch.answered(MINE);
        watch.settled(MINE);
        assert!(watch.through());
        assert_eq!(watch.arm("second"), None);
        assert!(!watch.broken());
        assert_eq!(watch.run_stage(), "input");
    }

    /// **A write is waited out only where the run announced a press.**
    /// What the watch carries is as often as not the read a menu makes on
    /// its way open; giving that up for the press about to be made is the
    /// ordinary thing (measured: refusing it failed six tag verbs in the
    /// container, on their own menu's read).
    #[test]
    fn arming_over_a_write_this_run_never_pressed_for_is_ordinary() {
        let mut watch = WriteWatch::default();
        watch.arm("open the menu");
        watch.asked(Some(THEIRS));
        assert_eq!(watch.run_stage(), "input", "nothing was announced");

        assert_eq!(watch.arm("press"), None);
        assert!(!watch.broken());
    }

    /// **A composite that means to move on says so.** `publish-new-go`
    /// adds the remote and then pushes, and never waits the first out —
    /// which is a different thing from forgetting to wait, and has a door
    /// of its own to go through.
    #[test]
    fn a_run_that_says_it_is_moving_on_may_arm_again() {
        let mut watch = waiting();
        watch.let_go();
        assert_eq!(watch.run_stage(), "dispatched");
        assert_eq!(
            watch.id(),
            MINE,
            "the write is still out there, and the diagnosis still names it"
        );

        assert_eq!(watch.arm("second"), None);
        assert!(!watch.broken());
    }

    /// **A broken run stays broken.** Arming again is
    /// refused: a run that could re-arm would go on to
    /// photograph a page its broken write never reached
    /// (reported, reproduced).
    #[test]
    fn a_broken_run_cannot_be_armed_again() {
        let mut watch = waiting();
        let first = watch.arm("second").expect("the breach");

        let again = watch.arm("third").expect("still refused");
        assert_eq!(again, first, "and the words are the first ones, not new");
        assert!(watch.broken());
        assert_eq!(watch.run_stage(), "broken", "and it did not go back");
    }

    /// …nor may it be let go, pressed, or answered into passing.
    #[test]
    fn a_broken_run_cannot_be_talked_out_of_it() {
        let mut watch = waiting();
        let id = watch.id();
        watch.arm("second");

        watch.let_go();
        watch.input_went();
        watch.answered(id);
        watch.settled(id);
        assert!(watch.broken());
        assert_eq!(watch.run_stage(), "broken");
        assert!(
            !watch.through(),
            "and the barrier it was waiting on never opens"
        );
    }

    /// A breach the run saw and this side could not — an input path that
    /// answered, with nothing accepted for it — ends the run the same way.
    #[test]
    fn a_breach_the_run_saw_ends_it_too() {
        let mut watch = waiting();
        watch.broke("the input went in and the queue took nothing");
        assert!(watch.broken());
        assert!(watch.wanted().contains("took nothing"));
        assert!(watch.arm("second").is_some(), "and nothing arms past it");
    }
}
