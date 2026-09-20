//! Who a commit is by, and the boxes that say so — the identity dialog
//! and its badge, the signature, the caret in a message box, and the
//! settings card the avatar list comes down inside.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // The caret is the whole of these two, and a hook that never
    // reached the box leaves a picture of the resting colour --
    // which is a real state, and the other half of each pair.
    Verb {
        name: "edit-message-focus",
        when: &[],
        plain: "message_focus pane=details focused=true",
    },
    Verb {
        name: "wip-message-focus",
        when: &[],
        plain: "message_focus pane=wip focused=true",
    },
    // A dialog that stayed open because the write did not take looks
    // exactly like one nobody has answered yet, and "which half
    // landed" is not something a picture holds at all.
    Verb {
        name: "identity",
        when: &[],
        plain: "identity state=missing dialog=true nameSaved=false emailSaved=false unsaved=false",
    },
    Verb {
        name: "identity-half",
        when: &[],
        plain: "state=ready dialog=true nameSaved=true emailSaved=false unsaved=true",
    },
    // The three forced tooltips: a hook that never reached its
    // target photographs the resting state, which is a real state.
    // `tip=` is the attached ToolTip's own visible — the output
    // side, as everywhere.
    Verb {
        name: "identity-tip",
        when: &[],
        plain: "identity_tip unsaved=true badge=true tip=true",
    },
    Verb {
        name: "signature-tip",
        when: &[],
        plain: "signature_tip code=E tip=true",
    },
    // The same popup has a real loading and a causally settled form;
    // neither is selected by a millisecond window. Both are opened by
    // the field's own press, and `typing=` is the half of that press a
    // photograph cannot answer: the list must come down with the caret
    // left in the box it came from.
    Verb {
        name: "settings-tools-loading",
        when: &[],
        plain: "settled=false loading=true open=true typing=true",
    },
    Verb {
        name: "settings-tools",
        when: &[],
        plain: "settled=true loading=false open=true typing=true",
    },
    // Which git the app runs, and what that binary answered. **Nothing
    // here is a picture**: a path is drawn the same whether or not there
    // is anything at the end of it, and the sentence under the box takes
    // a subprocess to arrive — so a run photographed at the moment it
    // typed frames "Asking for the version…" and reads as green.
    //
    // The three rows are the three states the chapter has. With no
    // argument the box is empty and the store is empty — the resting
    // state a fresh settings directory comes up in — and the git on
    // `PATH` answered it. **Which** of the two answers is not claimed:
    // the container runs the supported minimum, a desk runs the newest,
    // and the run that photographs an old one is handed its git by
    // `--old-git`.
    //
    // `in-use` types the path this run is already spawning: **a box holding
    // the git already running offers nothing**, whatever it is spelled
    // like, and that is the row that says so. `other` types the second git
    // the run was staged with (`--other-git`) and gets the button. **The
    // flags are one claim each and a picture answers none**: `offers=` is
    // the rule the press is guarded by, and `held=` is the frozen copy the
    // button was drawn from — a button whose word says `Restart to apply`
    // over a `holdMs` of zero photographs exactly like one that holds, and
    // would fall through under a hand.
    //
    // With a path that has nothing at the end of it, the refusal has to
    // be the *missing* one: a run that fell back to "it ran and said
    // something else" would be photographing the wrong sentence, and
    // `offers=false` is what says the loud button stayed away.
    // **Which version is not claimed anywhere here.** The container runs
    // the supported minimum, a desk runs the newest, and the run that
    // photographs an old one is handed its git by `--old-git` — including
    // the `in-use` run, which is how the offered shape is seen over a git
    // below the minimum. What every row claims instead is the run of
    // flags, which is why they stand together in the line.
    Verb {
        name: "settings-git-path",
        when: &[
            (Arg::Is(""), "git_path shown= stored= answered=true"),
            (
                Arg::Is("in-use"),
                "answered=true offers=false held=false restart=false",
            ),
            (
                Arg::Is("other"),
                "answered=true offers=true held=true restart=false",
            ),
        ],
        plain: "answered=false offers=false held=false restart=false state=missing",
    },
    // The process chapter's two boxes, typed and read back out of the
    // store. **Nothing here is a picture**: a box holding a number and a
    // box whose edit never landed frame the same, and the interval the
    // page's tick reads is in a unit no box shows — so the store's answer
    // is the line. With no argument both boxes are emptied: the default
    // count — which is the machine's own, a third of its threads, so the
    // line says `default=true`, a word every machine answers the
    // same — and the copies never read (`copies_ms=0` is what stops
    // the tick).
    Verb {
        name: "settings-processes",
        // Three, which no machine defaults to: the floor is
        // two and the ceiling eight, and the line has to be able to say
        // `default=false` on every machine the run is taken on.
        when: &[(
            Arg::Is("3:10"),
            "git_processes concurrency=3 default=false copies_secs=10 copies_ms=10000",
        )],
        plain: "default=true copies_secs=0 copies_ms=0",
    },
    // The way out taken over a git waiting to be applied. **Nothing here
    // is a picture**: a screen that stayed is drawn exactly like one
    // nobody asked to close, and the four halves of the claim are the way
    // out having been taken, the screen still standing, the mark turned,
    // and *what* is holding it — an unsaved identity would hold the same
    // screen the same way, and this must be the offer.
    Verb {
        name: "settings-git-leave",
        when: &[],
        plain: "settings_git_leave offers=true armed=true unsaved=0 open=true",
    },
    // The rail is the one way between the two categories that no other
    // verb travels: the rest name a category before the screen is up. A
    // picture of the git chapters cannot say which of the two put them
    // there, so both halves are said in the line.
    Verb {
        name: "settings-switch",
        when: &[],
        plain: "settings_switch was_app=true app=false git=true",
    },
    // The way out the screen owns, which the `✕` and Escape are both one
    // line onto. **Nothing here is a picture**: a window with no settings
    // screen over it is drawn exactly like one where the screen never
    // opened, so both halves have to be said — that it was up, and that
    // it went.
    //
    // `unsaved=0` is the third: the way out stops for an identity nobody
    // has saved, so a run where it did not stop has to say that nothing
    // was holding it. Boxes drift from git's answer on their own — git
    // answers late, and answers again — and a screen that called any of
    // that an edit would warn about work nobody did. The argument names
    // the category, because the git one is where both such chapters are.
    // `save=false` is the fourth: boxes nobody typed in say what git
    // holds, and a Save lit over them offers to hand git its own answer.
    Verb {
        name: "settings-escape",
        when: &[],
        plain: "settings_escape unsaved=0 global=false repo=false save=false was_open=true now_open=false",
    },
    // The same way out, taken over an identity that has not been saved.
    // **Neither half is a picture**: a screen that stayed is drawn like
    // one nobody asked to close, and the question in its foot is drawn
    // like any other foot until it is read. `save=true` is the pair of
    // the line above: the one typing that stops the way out is the one
    // that lights the button the reader is sent to.
    Verb {
        name: "settings-leave",
        when: &[],
        plain: "settings_leave unsaved=1 save=true asked=true open=true",
    },
    // The git category's two groups, which is the tallest this screen
    // gets. **The pair splits the claim**, because a row is one
    // substring and there are two things a picture cannot say.
    //
    // This one takes the reach: a column cut off at the window's edge is
    // drawn exactly like one that ends there, and the way it silently
    // breaks is a content height read off implicit sizes that a wrapping
    // label under-reports — which leaves the chapters clipped *and* the
    // bar down.
    // With an argument it is a *switch*: the screen has already landed on
    // the repository the reader is in, and the run picks another. What
    // must hold then is that the boxes followed — a screen still showing
    // the repository it was on, or showing nothing, is one offering to
    // write the wrong thing into the one now named above it, and both
    // photograph as an ordinary form.
    Verb {
        name: "settings-repo",
        when: &[(Arg::Is(""), "settings_fit reach=true")],
        plain: "repo_config rows=2 state=ready matches=true save=false",
    },
    // And this one takes the read and the list. `state=ready` is what
    // empty boxes cannot say for themselves — "this repository sets
    // nothing of its own" and "the read never landed" look alike —
    // while `rows=2` says both open repositories reached the chooser,
    // since a chooser offering only the one it landed on is a chooser
    // nobody can use.
    //
    // `save=false` is this chapter's own button, and it is a second
    // claim rather than a spelling of `matches=`: this Save asks for no
    // filled boxes — an emptied one is how an override is taken out —
    // so what keeps it dark over a screen nobody has typed in is the
    // touch alone.
    Verb {
        name: "settings-repo-pick",
        when: &[],
        plain: "repo_config rows=2 state=ready matches=true save=false open=true",
    },
    // The same group's line-ending chapter, picked. **The picture is a
    // chooser holding one of four sentences of the same shape**, so it
    // cannot say whether the pick moved anything: `were=` is what the
    // repository held before it, and it is `false` because that is what
    // every demo repository is built with (`demo::repo`).
    //
    // The repository is the only level there is: the screen writes
    // `core.autocrlf` into the one somebody picked and nowhere else
    // (規約 §設定の画面), so there is no second scope for a run to reach
    // — and the one it would have reached is the machine's own file,
    // which no run owns.
    Verb {
        name: "settings-eol",
        when: &[
            (
                Arg::Is("input"),
                "line_endings scope=local state=ready were=false held=input",
            ),
            // The row that writes nothing. What it falls back to is the
            // machine's, so the line stops before `effective=` — the
            // trailing space is what keeps `held=` from matching a held
            // value that merely starts where this one ends.
            (
                Arg::Is("inherited"),
                "line_endings scope=local state=ready were=false held= ",
            ),
        ],
        plain: "line_endings scope=local state=ready were=false",
    },
    // The same card's avatar half. Only this one of its four wants a
    // line: the other three cannot reach the camera with the wrong
    // state, because each waits on the state itself — a filed picture
    // in the list, a lit row, a row gone from the store — and a run
    // where any of that never came photographs nothing at all. What
    // no predicate covers is the second grab: the list is a popup, it
    // lives in overlay.png, and a mirror refreshed over an empty
    // overlay passes (the blind spot `commit-menu` is here for). Three
    // popups because the card is modal — its dimmer, itself, and the
    // list that came down inside it.
    Verb {
        name: "avatar-combo",
        when: &[],
        plain: "overlay saved=true popups=3",
    },
    // The badge's sentence names the address it is about, and the
    // address is handed to it late (`%1`). A picture cannot answer
    // that: an empty name leaves a box the same size with the same
    // words up to the gap, so `named=` is the run's own reading of the
    // tip against the author it was opened over.
    Verb {
        name: "avatar-tip",
        when: &[],
        plain: "avatar_tip tip=true named=true",
    },
];
