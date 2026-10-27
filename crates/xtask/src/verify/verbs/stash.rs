//! The working tree taken out of the way and put back, and the button
//! that reads whether it can be.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    Verb {
        name: "stash-tip",
        when: &[],
        plain: "stash_tip blocked=true tip=true",
    },
    // The mark the leading row wears once the write's rebuild has landed —
    // an archive box for a stash just made, the dashed ring for one just
    // popped. `gone=` is the row the write took away, and the run waits on
    // it; without it the shot is the graph at the write barrier, as it was.
    //
    // `named=` is the new entry's reflog subject, read off the sidebar's
    // model, held against the summary `stash named` typed first. Only the
    // `named` run reads it: the bare run types nothing, so it says `false`
    // whatever the entry is called.
    Verb {
        name: "stash",
        when: &[(
            Arg::Is("named"),
            "graph_settled gone=true top=stash named=true",
        )],
        plain: "graph_settled gone=true top=stash named=false",
    },
    // The same press from the working tree's own row with its pane open,
    // so the row taken away is the one the reader stands on; where they
    // are put instead is the claim. The highlight left on the vacated
    // index lights the new entry, a row like any other at this width.
    Verb {
        name: "stash-lands",
        when: &[],
        plain: "stash_landed wip=false follows=true onscreen=true",
    },
    // A stash rename git turned down, as a branch's is: before the new
    // entry went in (another git's lock — the box stays open, the gesture
    // going on), and after it, with the old one left beside it (half
    // done). Both `warning` (デザイン規約 §答えの要らない報せ). Bare, the
    // write's own boundary.
    Verb {
        name: "rename-stash",
        when: &[
            (
                Arg::WithPreset("stash-lock"),
                "write_notice open=true clears=true why=true tone=warning log=false wrong=false \
                 said=The stash was not renamed place=GIT COMMANDS \
                 says=Another git process seems to be running — see GIT COMMANDS",
            ),
            // The drop git turned down after the new entry was made — a
            // reftable store's lock (git 2.48+, the host's line alone:
            // verb-tiers.txt), which names no lock.
            (
                Arg::WithPreset("reftable-droplock"),
                "write_notice open=true clears=true why=true tone=warning log=false wrong=false \
                 said=The rename did not finish place=GIT COMMANDS \
                 says=The new name was made, but the old one is still there — see GIT COMMANDS",
            ),
            // …and a files store's, named.
            (
                Arg::WithPreset("stash-droplock"),
                "write_notice open=true clears=true why=true tone=warning log=false wrong=false \
                 said=The rename did not finish place=GIT COMMANDS \
                 says=The new name was made, but the old one is still there — see GIT COMMANDS",
            ),
        ],
        plain: "write_barrier mine=true",
    },
    // `back=`: the popped entry's name is back in the commit box, read off
    // the box.
    Verb {
        name: "stash-pop-row",
        when: &[],
        plain: "graph_settled gone=true top=wip named=false back=true",
    },
    // The band's Stash button on each of the four states a working tree
    // can be in, read as a set: one alone passes a band that answers the
    // same to everything. `tip=` for the reason `fetch-tip` gives.
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
