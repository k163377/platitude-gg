//! The discard log (破棄記録仕様.md): its seat, the list, an entry on the
//! graph under the band, the card a rest opens, and what the band's
//! `Restore` leaves.

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
            // Brought back whole: the entry put down and gone from the
            // list — the band's press, answered.
            (Arg::Is("11,restore"), "restores=1 how=whole refused=false"),
            // Work that would have conflicted, waiting as a stash.
            (Arg::Is("16,part:1"), "restores=1 how=stash refused=false"),
            // A filter that leaves nothing: `No results found` stands
            // where the rows would.
            (Arg::Has("filter:zzz"), "entries=19 listed=0"),
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
];
