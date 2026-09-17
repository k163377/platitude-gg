//! Walking the graph without the mouse: the stand-in for a HEAD scrolled
//! off, and what the arrows do to the selection, the viewport and the
//! diff standing over it.

use super::Verb;

pub(super) const TABLE: &[Verb] = &[
    // The stand-in for a HEAD scrolled off. The five read as one set:
    // a band that is there and a band that is gone frame the same
    // way, and which edge it took is a difference of a few hundred
    // pixels in a picture of two thousand rows. `onScreen=` is the
    // other half of every one of them — the stand-in is wanted
    // exactly while its row is not in sight, so a run where both are
    // true (or neither) has the rule backwards.
    // The top edge carries two more that no picture holds: the
    // selection stays on the stand-in when the row it stands for is
    // scrolled off, and the stand-in keeps the sliver the list holds
    // above its own first row. `room=4` is `headerHeight` less
    // `graphRowHeight` — a token moving under it is a thing to look
    // at.
    Verb {
        name: "graph-head",
        when: &[],
        plain: "shown=true above=true onScreen=false lit=false landed=true picked=true room=4",
    },
    Verb {
        name: "graph-head-below",
        when: &[],
        plain: "shown=true above=false onScreen=false lit=false landed=false picked=false room=0",
    },
    // The pointer's ground, which only a stand-in that is not already
    // carrying the selection has anywhere to show — a selected row does
    // not brighten under the pointer, and neither does its stand-in. So
    // this one belongs on the preset whose HEAD is not the row the page
    // opened on, and its `above=true` is what says the run scrolled up
    // for the top edge there, past the bottom one it started
    // with.
    Verb {
        name: "graph-head-lit",
        when: &[],
        plain: "shown=true above=true onScreen=false lit=true",
    },
    // And the two that take it away again: the press has to leave the
    // selection on the row it led to, which is the whole of what it
    // is for; the scroll back only has to bring the row into sight.
    Verb {
        name: "graph-head-go",
        when: &[],
        plain: "shown=false above=false onScreen=true lit=false landed=true",
    },
    Verb {
        name: "graph-head-back",
        when: &[],
        plain: "shown=false above=false onScreen=true lit=false",
    },
    // Walking the graph with the arrows. The picture holds which row
    // is lit and, once `card=true`, whose commit fills the right-hand
    // pane — the two catch up one after the other, and a shot between
    // them holds a different commit in each. What it cannot hold is
    // the walk working at all: the keyboard on the list, the settle
    // behind a held key landing the selection, the viewport carrying
    // the row stepped onto. A walk that moved nothing sits still.
    Verb {
        name: "graph-step",
        when: &[],
        plain: "landing=in back=false refused=0 focused=true diff=false onscreen=true selected=true card=true",
    },
    // The other two landings, which no picture holds: a row brought in
    // flush against the bottom one step at a time, and a row centered
    // because the one it stepped off was nowhere on screen. Both frame
    // as a graph with a lit row somewhere in it.
    Verb {
        name: "graph-step-edge",
        when: &[],
        plain: "landing=edge back=false refused=0 focused=true",
    },
    Verb {
        name: "graph-step-far",
        when: &[],
        plain: "landing=center back=false refused=0 focused=true",
    },
    // The same walk with the key held down, which is the one thing
    // the picture cannot hold at all: a run that read every row it
    // passed through frames exactly like one that read two. `reads=2`
    // is the rule — the row the hand set off from, and the row it
    // stopped on — and a third read is the first repeat being taken
    // for a press of its own, which is what every OS's delay before
    // that repeat invites (`GraphRowWalk.noteStep`).
    Verb {
        name: "graph-step-hold",
        when: &[],
        plain: "reads=2 refused=0 selected=true card=true",
    },
    // The refusing half. `back=true` is the whole of it — nothing
    // moved — and it is worth nothing without `refused=`, since a walk
    // that was never attempted leaves the same row lit. Read it beside
    // a plain `graph-step`: a step that always refuses passes it on
    // its own.
    Verb {
        name: "graph-step-named",
        when: &[],
        plain: "back=true refused=1",
    },
    // And the one that steps anyway: a half-written message is a
    // draft, and the arrows walk off it the way a click does — so
    // what it has to say is a plain step's answer, word for word.
    Verb {
        name: "graph-step-dirty",
        when: &[],
        plain: "landing=in back=false refused=0 focused=true diff=false onscreen=true selected=true card=true",
    },
    // A diff opened over the graph. `focused=false` is the mechanism —
    // Qt leaves active focus on a pane it has just swapped away, and
    // the keys go on arriving there — and `diff=true` is what the
    // report was about: a step behind the diff moves the selection,
    // and moving the selection closes the diff, so the screen jumps
    // back to the graph. A picture of the diff still standing is also
    // a picture of a run where the arrow was never pressed.
    Verb {
        name: "graph-step-diff",
        when: &[],
        plain: "back=true refused=1 focused=false diff=true",
    },
];
