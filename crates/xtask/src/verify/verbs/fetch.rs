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
    // The drag, left standing for the picture: the wash over the rows is
    // the deliverable, and a run whose drag reached nothing frames the
    // same way — an unwashed log and one nobody dragged over are one
    // photograph. **Two halves, each answering for itself**:
    // `holds=` is the selection the model kept, `worn=` is the rectangle
    // every row drew of it. A wash placed off the row's own layout can
    // come out empty with the runs perfectly right, and `holds=` alone
    // went green over exactly that (measured).
    Verb {
        name: "commands-select",
        when: &[],
        plain: "commands_pick holds=true worn=true",
    },
    // And the key that takes it, which leaves nothing in the picture at
    // all: the window frames the same whether Ctrl+C took the log, one
    // row of it, or nothing. `perRow=` is the whole judgement — one line
    // on the clipboard for every row in the panel, read back off the pad
    // the copy goes through. Lines that begin with a tab are not counted:
    // a row that failed brings git's own words down indented under it,
    // and no run owns which of its commands fail.
    Verb {
        name: "commands-copy",
        when: &[],
        plain: "commands_copy perRow=true",
    },
    // The same drag started on the ground under the last row — every
    // place in the panel that nobody else takes is a start
    // (規約 §git が言ったことを読む場所). `reach=9/9` is from every
    // corner of that ground: a verb that pressed the middle alone
    // goes green over a hand that only answers there, which is the
    // fault the right pane's values shipped with (`details-sweep`).
    // `ground=true` is the fixture's half — a log that fills its
    // panel leaves no ground, and `reach=0/9` off one says nothing
    // about the hand.
    Verb {
        name: "commands-sweep",
        when: &[],
        plain: "commands_sweep reach=9/9 ground=true",
    },
    // The panel taken down without being emptied, which is the only
    // way the mark's red is on screen with nothing standing over it.
    // The picture does hold the red — the row goes red with the mark —
    // but only a reader can see that, and this is the judgement: a run
    // whose refusal never landed ends on the same resting row, and one
    // that landed it and never pressed ends with the panel still up.
    // The two halves are named so each answers for itself.
    Verb {
        name: "commands-fail-shut",
        when: &[],
        plain: "commands_shut wrong=true open=false",
    },
    // The same panel taken down by the key, which is the door
    // cut for exactly this panel: nothing else on screen said what git
    // said, so it raised itself, and until now only a press could put
    // it away (デザイン規約 §git が言ったことを読む場所). `took=true`
    // is the page claiming the key — an Escape that fell through to
    // nobody leaves the same shut window as one that never went out,
    // and `open=false` alone cannot tell them apart. `wrong=true` is
    // the refusal still on the mark afterwards: the panel goes, the
    // record does not.
    Verb {
        name: "commands-escape",
        when: &[],
        plain: "commands_escape took=true open=false wrong=true",
    },
    // Recovery is the show, and the picture can only hold its quiet
    // half: a band that failed and healed ends the run looking like
    // one that never failed at all. `was=`/`hadline=`/`wasopen=` are
    // the red half — without them, a fetch that never failed raised no
    // line and no panel, and taking down nothing would pass as
    // recovery. `open=false` is the panel that failure raised going
    // down with it, which is the whole of what the reader asked for:
    // a laptop that sleeps wakes with its news taken down.
    Verb {
        name: "fetch-recover",
        when: &[],
        plain: "fetch_recover was=true hadline=true wasopen=true wrong=false line=false failures=0 open=false",
    },
    // The other side of that one word, and the reason it is a pair: the
    // reader's own panel, up before anything failed, which the same
    // recovery must leave standing. `open=true` alone would pass on a
    // run whose recovery never happened, so it is read beside the
    // `wrong=`/`line=`/`failures=` the landing fetch cleared. The order
    // the two panels came up in is the whole difference and no picture
    // holds it — the orders themselves are walked in
    // tests/qml/tst_commandsowner.qml.
    Verb {
        name: "fetch-recover-held",
        when: &[],
        plain: "fetch_recover was=true hadline=true wasopen=true wrong=false line=false failures=0 open=true",
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
    // A hand on each of the three live shapes. `tip=` is the words
    // on screen (`fetch-tip` judges the string the button hands
    // over), and `word=` beside `rest=` is the pair a picture
    // cannot hold: the stopped shape is the one that steps its word
    // back, so it is the one whose two halves have to differ. The
    // counts ride after them unjudged for the reason `fetch-fail`'s
    // do.
    Verb {
        name: "fetch-hover",
        when: &[(
            Arg::OneOf(&["3"]),
            "fetch_hover tip=true word=true rest=false stopped=true",
        )],
        plain: "fetch_hover tip=true word=true rest=true stopped=false",
    },
    // The word inside that tip which is a place to go,
    // pressed. The panel is put away first — the failures
    // raised it on the way here — so `open=` is the press's own
    // doing, and `lit=` is the panel saying which reader it is
    // standing for. The picture holds the second one alone: a
    // panel that was never down looks exactly like a panel that
    // came back up.
    Verb {
        name: "fetch-tip-link",
        when: &[],
        plain: "fetch_link open=true lit=true",
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
    // The refs that fetch brought back land after its rows and in two
    // places of their own: `behind=1` is the sidebar's word for them
    // and `top=origin/main` the graph's, a pass later still. A run
    // that says neither photographed a graph that had fetched beside a
    // sidebar that had not.
    Verb {
        name: "open-fetches",
        when: &[],
        plain: "open_fetch fails=0 behind=1 top=origin/main",
    },
    // Intermediate communication is valid only after the real busy
    // edge was observed and latched for the asynchronous image grab.
    Verb {
        name: "force-push-hold",
        when: &[],
        plain: "push_hold mode=diverged busy=true",
    },
    // The bare button's wait. `framed=false` is judged: a frame that
    // grew while git was out would be a line in the picture, but a
    // frame that did not grow is nothing at all, and nothing is what
    // a correct run looks like too.
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
