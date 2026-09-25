//! The one write a run is waiting for, kept by the id its own ask was
//! given (rules-refs/app-ui.md「自動化の write barrier」).
//!
//! No event says which write is *yours*; only the id `RepoSession::write`
//! hands back does, so the ask keeps it here inside that call
//! (`RepoTab::ask_session`) and every question afterwards is equality
//! against it. Ids are not a sequence (`OperationId`): `>=` would let
//! another write's answer stand in for this one.
//!
//! The tab keeps it because the harness presses through the product's
//! input path, which answers nothing; the tab lives on one thread, so the
//! arm, the ask and the read are one uninterrupted stretch.
//!
//! [`WatchedWrite`] is the id and how far along it is; [`WriteWatch`] adds
//! what the run said about it, and waiting is read from that stage,
//! whatever sampler does the waiting. A breach is final — a run that
//! could start over would photograph a page its broken write never
//! reached.

/// What the watch is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum WatchedWrite {
    /// Nobody is watching: the ordinary application, and a run before its
    /// first arm.
    #[default]
    Asleep,
    /// The next write this tab is asked to make is the one to keep.
    /// `turned_down`: the queue took nothing (the session is closed) —
    /// unlike a press not yet in, which the watch cannot see.
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
    /// The next ask is the one to keep. Always allowed: only the run knows
    /// whether the write it gives up was one it was waiting for
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
            // Nothing armed: keep what is held — the serial queue serves a
            // second ask after it.
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

    /// The id being watched, 0 while there is none (for a stalled run's
    /// diagnosis).
    pub(super) fn id(&self) -> u64 {
        match self {
            Self::Held { id, .. } => *id,
            _ => 0,
        }
    }

    /// Where the watch stands, for that diagnosis; each word names a
    /// different reason a barrier has not opened.
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
    /// The run has announced nothing: the dispatch armed for whatever the
    /// verb writes on its way through.
    #[default]
    Dispatched,
    /// Armed, and the run's input has not gone in yet (a row still
    /// sought, a hold running, a question standing).
    Input,
    /// The input went in: the run is waiting.
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
    /// The next ask this tab makes is the one to wait for. Answers with
    /// the breach where arming is one (the run is then broken); `None`
    /// when the run may go on. The breach is arming over a write the run
    /// is waiting out — over nothing, a write through, or one never
    /// pressed for is ordinary.
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

    /// The run's input has gone in (a press returned, a hold ran out, a
    /// question was answered); from here the run is waiting.
    pub(super) fn input_went(&mut self) {
        if self.stage == Stage::Input {
            self.stage = Stage::Pressed;
        }
    }

    /// The run moves on from the write it pressed for without waiting it
    /// out — a composite's door (`publish-new-go` adds the remote, then
    /// pushes), telling it apart from forgetting to wait. The id stays, so
    /// the diagnosis still names the write.
    pub(super) fn let_go(&mut self) {
        if self.stage == Stage::Pressed {
            self.stage = Stage::Dispatched;
        }
    }

    /// The contract broke somewhere the run could see and this one could
    /// not — the verb says so and the run ends.
    pub(super) fn broke(&mut self, breach: &str) {
        if self.stage != Stage::Broken {
            self.stage = Stage::Broken;
            breach.clone_into(&mut self.wanted);
        }
    }

    /// Whether the contract is broken.
    pub(super) fn broken(&self) -> bool {
        self.stage == Stage::Broken
    }

    /// Where the run stands, for the line a ceiling leaves.
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
    /// Whether the write waited for is through both boundaries and the
    /// contract holds.
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

    /// The run's own id is the lower one, so a barrier reading `>=` would
    /// open on the other.
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

    /// The same with the run's own id the higher one: neither direction
    /// of the comparison is the test on its own.
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

    /// A press that goes in ticks after the arm (a hold, a dialog) waits
    /// in `armed`, which tells "not landed" from "the queue took nothing".
    #[test]
    fn an_ask_that_takes_its_time_is_still_the_one_kept() {
        let mut watch = WatchedWrite::default();
        watch.arm();
        assert_eq!(watch.stage(), "armed");
        // A write nobody here pressed answers during the hold; nothing is
        // held, so nothing moves.
        watch.answered(THEIRS);
        watch.settled(THEIRS);
        assert_eq!(watch.stage(), "armed");

        watch.asked(Some(MINE));
        watch.answered(MINE);
        watch.settled(MINE);
        assert!(watch.through());
    }

    /// Its own word, because a run reading "armed" is still waiting for
    /// its press to land and one reading this is not.
    #[test]
    fn an_ask_the_queue_turned_down_says_so() {
        let mut watch = WatchedWrite::default();
        watch.arm();
        watch.asked(None);
        assert_eq!(watch.stage(), "turned-down", "an answer, not a wait");
        assert!(!watch.through());
    }

    /// Every run arms twice (at the dispatch and at the press), often over
    /// a write nobody pressed for; whether giving it up is a mistake is the
    /// run's question. Refusing here fails verbs whose own menu read is
    /// still out when they press.
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

    /// A run that armed, pressed, and is waiting on its write.
    fn waiting() -> WriteWatch {
        let mut watch = WriteWatch::default();
        assert_eq!(watch.arm("press"), None);
        watch.asked(Some(MINE));
        watch.input_went();
        assert_eq!(watch.run_stage(), "pressed");
        watch
    }

    /// Waiting is read from ids and stages, not from the shared barrier's
    /// timer — `line-back` waits from a sampler of its own.
    #[test]
    fn arming_over_the_write_this_run_is_waiting_out_is_the_breach() {
        let mut watch = waiting();
        let breach = watch.arm("second").expect("the first was still out");
        assert!(breach.contains("press"), "it names the press: {breach}");
        assert!(breach.contains("id 7"), "and the write: {breach}");
        assert!(breach.contains("second"), "and what was armed: {breach}");
        assert!(watch.broken());
    }

    #[test]
    fn a_write_answered_but_not_settled_is_still_out() {
        let mut watch = waiting();
        watch.answered(MINE);
        assert!(watch.arm("second").is_some());
        assert!(watch.broken());
    }

    /// How `line-back` stages three writes.
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

    #[test]
    fn arming_over_a_write_this_run_never_pressed_for_is_ordinary() {
        let mut watch = WriteWatch::default();
        watch.arm("open the menu");
        watch.asked(Some(THEIRS));
        assert_eq!(watch.run_stage(), "input", "nothing was announced");

        assert_eq!(watch.arm("press"), None);
        assert!(!watch.broken());
    }

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

    #[test]
    fn a_broken_run_cannot_be_armed_again() {
        let mut watch = waiting();
        let first = watch.arm("second").expect("the breach");

        let again = watch.arm("third").expect("still refused");
        assert_eq!(again, first, "and the words are the first ones, not new");
        assert!(watch.broken());
        assert_eq!(watch.run_stage(), "broken", "and it did not go back");
    }

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

    /// A breach only the run saw: an input path that answered with
    /// nothing accepted for it.
    #[test]
    fn a_breach_the_run_saw_ends_it_too() {
        let mut watch = waiting();
        watch.broke("the input went in and the queue took nothing");
        assert!(watch.broken());
        assert!(watch.wanted().contains("took nothing"));
        assert!(watch.arm("second").is_some(), "and nothing arms past it");
    }
}
