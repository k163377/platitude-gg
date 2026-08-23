//! The working tree taken out of the way and put back, and the button
//! that reads whether it can be.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    Verb {
        name: "stash-tip",
        when: &[],
        plain: "stash_tip blocked=true tip=true",
    },
    // The two whose subject is the mark the leading row wears once the
    // rebuild a write asks for has landed — an archive box where a
    // stash was just made, the working tree's dashed ring where one
    // was just popped back. `gone=` is the row the write took away,
    // and it is what the run waited on; without the line the shot was
    // taken at the write barrier, which is the graph as it was.
    //
    // `stash named` leaves a summary in the commit box first, and
    // `named=` is the entry's own reflog subject held against it — read
    // off the sidebar's model, so a name that never left the box says
    // `false`. **Read as a pair with the bare verb**: one alone passes
    // a build that names every entry the same way. The picture settles
    // neither — the two rows are the same row with different text in
    // it, at a width that elides most of it.
    Verb {
        name: "stash",
        when: &[(
            Arg::Is("named"),
            "graph_settled gone=true top=stash named=true",
        )],
        plain: "graph_settled gone=true top=stash named=false",
    },
    // The same press, made from the working tree's own row with the
    // pane that describes it open — so the row the press takes away is
    // the one the reader is standing on. Where they are put instead is
    // the claim, and the picture holds none of it: the highlight left
    // on a vacated index lights the entry the press just made, which
    // at this width is a row like any other, and a viewport that never
    // moved frames the same way in a history this short.
    Verb {
        name: "stash-lands",
        when: &[],
        plain: "stash_landed wip=false follows=true onscreen=true",
    },
    // `back=` is the other half of the same press: the entry's name is
    // in the commit box, where the entry is not there to hold it any
    // more. Read off the box, and invisible in the picture at any
    // width the summary field is drawn at.
    Verb {
        name: "stash-pop-row",
        when: &[],
        plain: "graph_settled gone=true top=wip named=false back=true",
    },
    // The band's Stash button, on each of the four things a working
    // tree can be. A button held down frames the same whichever
    // refusal put it there, so the reading is said out loud beside
    // what the band did with it — and the four are read as a set:
    // one alone passes a band that answers the same thing to
    // everything. `tip=` is the string the button would open, for
    // the reason `fetch-tip` gives.
    Verb {
        name: "stash-state",
        when: &[
            (
                Arg::Is("ready"),
                "stash_state mode=ready enabled=true tip=true",
            ),
            (
                Arg::Is("clean"),
                "stash_state mode=clean enabled=false tip=true",
            ),
            (
                Arg::Is("conflicts"),
                "stash_state mode=conflicts enabled=false tip=true",
            ),
        ],
        plain: "stash_state mode=unborn enabled=false tip=true",
    },
];
