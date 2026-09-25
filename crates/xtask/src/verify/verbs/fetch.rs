//! Talking to a remote and coming back: the wait, the run of failures
//! that suspends it, the recovery, and the panel a refusal raises.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // `was=true` is judged beside the other two: a refusal that never
    // landed leaves no red mark and no panel, and clearing nothing
    // would pass.
    Verb {
        name: "commands-clear",
        when: &[],
        plain: "commands_clear was=true wrong=false open=false",
    },
    // The drag, left standing for the picture. `holds=` is the selection
    // the model kept, `worn=` the wash every row drew of it: a wash placed
    // off the row's own layout comes out empty with `holds=` still true.
    Verb {
        name: "commands-select",
        when: &[],
        plain: "commands_pick holds=true worn=true",
    },
    // Ctrl+C on it. `perRow=` is one line on the pad the copy goes
    // through for every row in the panel. Tab-led lines are not counted:
    // a failed row brings git's words indented under it, and no run owns
    // which of its commands fail.
    Verb {
        name: "commands-copy",
        when: &[],
        plain: "commands_copy perRow=true",
    },
    // The same drag started on the ground under the last row
    // (規約 §git が言ったことを読む場所). `reach=9/9`: from every corner —
    // pressing one point passes a hand that answers only there (as
    // `details-sweep`). `ground=true`: a log that fills its panel
    // leaves no ground, and `reach=0/9` off it says nothing.
    Verb {
        name: "commands-sweep",
        when: &[],
        plain: "commands_sweep reach=9/9 ground=true",
    },
    // The panel taken down without being emptied — the only way the
    // mark's red is on screen with nothing over it. `wrong=true` fails a
    // refusal that never landed, `open=false` a press that never came.
    Verb {
        name: "commands-fail-shut",
        when: &[],
        plain: "commands_shut wrong=true open=false",
    },
    // The same panel taken down by Escape (デザイン規約
    // §git が言ったことを読む場所). `took=true` is the page claiming the
    // key; `open=false` alone passes an Escape that fell through.
    // `wrong=true`: the panel goes, the record does not.
    Verb {
        name: "commands-escape",
        when: &[],
        plain: "commands_escape took=true open=false wrong=true",
    },
    // Recovery. `was=`/`hadline=`/`wasopen=` are the red half — a fetch
    // that never failed raised nothing, and taking down nothing would
    // pass. `open=false` is the panel the failure raised going down with
    // it (a laptop that sleeps wakes with its news taken down).
    Verb {
        name: "fetch-recover",
        when: &[],
        plain: "fetch_recover was=true hadline=true wasopen=true wrong=false line=false failures=0 open=false",
    },
    // The pair: the reader's own panel, up before anything failed, which
    // the same recovery leaves standing. `open=true` alone passes a run
    // that never recovered, so it is read beside the cleared
    // `wrong=`/`line=`/`failures=`. The orders the two panels come up in
    // are walked in tests/qml/tst_commandsowner.qml.
    Verb {
        name: "fetch-recover-held",
        when: &[],
        plain: "fetch_recover was=true hadline=true wasopen=true wrong=false line=false failures=0 open=true",
    },
    // A run of failures cut short rests in the warning shape, which the
    // short arguments are for. `stopped=` is the tab's suspension (the
    // `Resume` word). The counts ride unjudged: the opening fetch lands
    // its own failure inside the run, and where moves with the machine.
    Verb {
        name: "fetch-fail",
        when: &[(Arg::OneOf(&["", "1", "2"]), "fetch_fail stopped=false")],
        plain: "fetch_fail stopped=true",
    },
    // A hand on each of the three live shapes. `tip=` is the words on
    // screen (`fetch-tip` judges the string handed over). `word=` beside
    // `rest=`: the stopped shape steps its word back, so only there do
    // the two differ. The counts ride unjudged, as for `fetch-fail`.
    Verb {
        name: "fetch-hover",
        when: &[(
            Arg::OneOf(&["3"]),
            "fetch_hover tip=true word=true rest=false stopped=true",
        )],
        plain: "fetch_hover tip=true word=true rest=true stopped=false",
    },
    // The tip's link word, pressed. The panel the failures raised is put
    // away first, so `open=` is the press's own doing; `lit=` is the
    // panel naming which reader it stands for.
    Verb {
        name: "fetch-tip-link",
        when: &[],
        plain: "fetch_link open=true lit=true",
    },
    // The resume at the end of such a run, over before the shot: the
    // button back at work frames like `fetch-fail 1`.
    Verb {
        name: "fetch-resume",
        when: &[],
        plain: "fetch_resume stopped=true fetched=true",
    },
    // The fetch an opening fires, nothing pressed. `fails=0`: it came
    // back clean. Its refs land after the rows, in two places: `behind=1`
    // is the sidebar's word for them and `top=origin/main` the graph's, a
    // pass later still.
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
    // The resting button's wait: the frame is drawn either way here, so
    // `turned=false` says the wait did not change its colour.
    Verb {
        name: "fetch-busy",
        when: &[],
        plain: "fetch_busy busy=true fails=0 turned=false",
    },
    // Both halves are absences: a dim button says nothing about why, and
    // a tooltip that stays away frames like one never asked for. The
    // argument names the side.
    Verb {
        name: "fetch-tip",
        when: &[(
            Arg::Is("off"),
            "fetch_tip enabled=false tip=false remotes=0",
        )],
        plain: "fetch_tip enabled=true tip=true",
    },
];
