//! The discard log (`platitude_core::discards`): its seat, the list, an entry on the
//! graph under the band, the card a rest opens, what the band's `Restore`
//! leaves, and what the rows answer a hand with while the entry is on the graph.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // The seat lit as after a discard, the list shut.
    Verb {
        name: "recover",
        when: &[
            // Blinked as a discard blinks it: twice, and out.
            (Arg::Has("blink"), "lit=false blinks=2"),
            (Arg::Has("lit"), "lit=true"),
        ],
        plain: "open=false",
    },
    Verb {
        name: "recover-open",
        when: &[
            // A right-click and a double-click on a row the entry leaves dim,
            // from the row and from its chip's list, and on a tip drawn as a
            // stash or as uncommitted work: nothing opens, nothing moves.
            (
                Arg::Has("menu:passed"),
                "named=true menu=false rows=0 offered= dbl=false box=false stayed=true moved=false",
            ),
            (
                Arg::Is("0,menu:tip"),
                "menu=false rows=0 offered= dbl=false box=false stayed=true moved=false",
            ),
            (
                Arg::Is("3,menu:tip"),
                "menu=false rows=0 offered= dbl=false box=false stayed=true moved=false",
            ),
            // On a commit it draws, a name on the row all the same: the menu,
            // with only what adds and none of the name's rows.
            (
                Arg::Has("menu:tip"),
                "named=true menu=true rows=4 offered=Create branch here|cherry-pick|WORKTREE|TAG",
            ),
            // Its double-click makes a name; it moves to none.
            (Arg::Has("dbl:tip"), "named=true dbl=true box=true"),
            // …and the same from its chip's list.
            (Arg::Has("dbl:list"), "named=true dbl=- box=true"),
            // Two spaced clicks wait for no rename, and a quick click on
            // another row is still read.
            (
                Arg::Has("again:tip"),
                "named=true armed=false quick=true box=false",
            ),
            // Brought back whole: the entry put down and gone from the
            // list — the band's press, answered.
            (Arg::Is("11,restore"), "restores=1 how=whole refused=false"),
            // Work that would have conflicted, waiting as a stash.
            (Arg::Is("16,part:1"), "restores=1 how=stash refused=false"),
            // A filter that leaves nothing: `No results found` stands
            // where the rows would.
            (Arg::Has("filter:zzz"), "entries=19 listed=0"),
            // The picked entry stays in the list however it stands: its stand-in takes
            // a seat of its own under a filter that leaves its row out, and rides the
            // edge its row went out of (`RecoverPane.pickPin`).
            (Arg::Has("filter:Deleted"), "pin=seat card=false pick=3"),
            (Arg::Has("scroll:"), "pin=top card=false pick=0"),
            (Arg::Has("short"), "pin=bottom card=false pick=18"),
            (Arg::Has("card"), "card=true pick=-1"),
            // Past the graph's window: the press waits for the graph to
            // walk there.
            (Arg::WithPreset("discards-deep"), "reach=false restores=0"),
            // An entry of two parts, both on the graph, each of which comes back
            // alone too.
            (Arg::Is("11"), "pick=11 shown=true byPart=true"),
            // The branches one rebase moved: they come back together.
            (Arg::Is("12"), "pick=12 shown=true byPart=false"),
            // Nothing picked, whatever else the run looks at.
            (Arg::Starts("none"), "pick=-1 shown=false"),
        ],
        plain: "shown=true",
    },
    // Esc on the entry on the graph: down, under a read too, the keyboard back
    // on the list, and no word that its commits are not loaded while the graph
    // walked again under it.
    Verb {
        name: "recover-escape",
        when: &[],
        plain: "recover_escape hand=true took=true list=true reading=true tookReading=true \
                again=false pick=-1 shown=false open=true listKeys=true graphKeys=false notLoaded=0",
    },
];
