//! Talking to a remote and coming back: the wait, the run of failures
//! that suspends it, the recovery, and the panel a refusal raises.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // A window whose panel the press took down and a window that
    // never raised one frame the same, and the mark the press also
    // quiets is 12 pixels of it in a corner. `was=` is judged with
    // both — a refusal that never landed leaves a mark that was
    // never red and a panel that was never up, and clearing nothing
    // would pass.
    Verb {
        name: "commands-clear",
        when: &[],
        plain: "commands_clear was=true wrong=false open=false",
    },
    // Recovery is the show, and the picture can only hold its quiet
    // half: a band that failed and healed ends the run looking like
    // one that never failed at all. `was=`/`hadline=` are the red
    // half — without them, a fetch that never failed raised no line,
    // and taking down nothing would pass as recovery.
    Verb {
        name: "fetch-recover",
        when: &[],
        plain: "fetch_recover was=true hadline=true wrong=false line=false failures=0",
    },
    // A run of failures cut short photographs the warning shape,
    // which is a real state and the one the short arguments are for
    // — so which shape the run came to rest in is said out loud.
    // `stopped=` is the tab's own suspension, the thing that puts
    // the word `Resume` on the button. The counts ride after it
    // unjudged: the fetch an opening fires lands its own failure
    // inside the run, and where it lands moves with the machine.
    Verb {
        name: "fetch-fail",
        when: &[(Arg::OneOf(&["", "1", "2"]), "fetch_fail stopped=false")],
        plain: "fetch_fail stopped=true",
    },
    // The resume on the end of such a run, whose whole show is over
    // before the picture is taken: what it ends on is a button back
    // at work, and that frames exactly like `fetch-fail 1`.
    Verb {
        name: "fetch-resume",
        when: &[],
        plain: "fetch_resume stopped=true fetched=true",
    },
    // Nothing is pressed in this one, so the picture on its own is a
    // graph — and a graph that fetched and one that did not frame the
    // same way. `fails=0` is the other half: a run whose opening fetch
    // came back with something to say reached its rows some other way.
    Verb {
        name: "open-fetches",
        when: &[],
        plain: "open_fetch fails=0",
    },
    // Intermediate communication is valid only after the real busy
    // edge was observed and latched for the asynchronous image grab.
    Verb {
        name: "force-push-hold",
        when: &[],
        plain: "push_hold mode=diverged busy=true",
    },
    // The bare button's wait. `framed=false` is judged rather than
    // looked at: a frame that grew while git was out would be a line
    // in the picture, but a frame that did not grow is nothing at
    // all, and nothing is what a correct run looks like too.
    Verb {
        name: "fetch-busy",
        when: &[],
        plain: "fetch_busy busy=true fails=0 framed=false",
    },
    // Both halves are absences on the picture: a dim button says
    // nothing about why it is dim, and a tooltip that stays away
    // frames exactly like one that was never asked for. The argument
    // names which side of the pair the run is.
    Verb {
        name: "fetch-tip",
        when: &[(
            Arg::Is("off"),
            "fetch_tip enabled=false tip=false remotes=0",
        )],
        plain: "fetch_tip enabled=true tip=true",
    },
];
