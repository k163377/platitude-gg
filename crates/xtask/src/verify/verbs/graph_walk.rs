//! Walking the graph without the mouse: the stand-in for a HEAD scrolled
//! off, and what the arrows do to the selection, the viewport and the
//! diff standing over it.

use super::Verb;

pub(super) const TABLE: &[Verb] = &[
    // The stand-in for a HEAD scrolled off; the five read as one set.
    // `onScreen=` is the other half of `shown=`: the stand-in is wanted
    // exactly while its row is out of sight. The top edge adds two: the
    // selection stays on the stand-in when its row scrolls off, and the
    // stand-in keeps the sliver above the list's first row (`room=4` is
    // `headerHeight` less `graphRowHeight`, and moves with those tokens).
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
    // The pointer's ground, shown only by a stand-in not carrying the
    // selection — so this runs on a preset whose HEAD is not the row the
    // page opened on, and `above=true` says it scrolled up to the top edge.
    Verb {
        name: "graph-head-lit",
        when: &[],
        plain: "shown=true above=true onScreen=false lit=true",
    },
    // The two that take it away: the press leaves the selection on the
    // row it led to; the scroll back only brings the row into sight.
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
    // Walking the graph with the arrows. `card=true` waits for the right
    // pane to catch up with the lit row (a shot between them holds a
    // different commit in each); the rest is the walk working at all.
    Verb {
        name: "graph-step",
        when: &[],
        plain: "landing=in back=false refused=0 focused=true diff=false onscreen=true selected=true card=true",
    },
    // The other two landings: a row brought in flush against the bottom
    // one step at a time, and a row centred because the one it left was
    // off screen.
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
    // The key held down. `reads=2` is the row set off from and the row
    // stopped on; a third is the first auto-repeat taken for a press of
    // its own, which every OS's repeat delay invites
    // (`GraphRowWalk.noteStep`).
    Verb {
        name: "graph-step-hold",
        when: &[],
        plain: "reads=2 refused=0 selected=true card=true",
    },
    // The refusing half: `back=true` (nothing moved) is worth nothing
    // without `refused=`. Read beside `graph-step` — a step that always
    // refuses passes this alone.
    Verb {
        name: "graph-step-named",
        when: &[],
        plain: "back=true refused=1",
    },
    // A half-written message is a draft: the arrows walk off it as a
    // click does, so this says a plain step's line word for word.
    Verb {
        name: "graph-step-dirty",
        when: &[],
        plain: "landing=in back=false refused=0 focused=true diff=false onscreen=true selected=true card=true",
    },
    // A diff opened over the graph. `focused=false`: Qt leaves active
    // focus on a pane it has just swapped away, and the keys go on
    // arriving there. `diff=true`: a step behind the diff would move the
    // selection, which closes the diff.
    Verb {
        name: "graph-step-diff",
        when: &[],
        plain: "back=true refused=1 focused=false diff=true",
    },
];
