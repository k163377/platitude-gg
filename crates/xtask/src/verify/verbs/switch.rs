//! Moving the working copy: where the move landed, what it carried, and
//! the bars that come down to ask before it happens.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // Where a move comes to rest, and whether the carry left an entry
    // behind. The two landings frame almost alike — a branch is a
    // branch — so the counts beside it are the whole claim, and the
    // one that says nothing was stashed is half of the pair.
    //
    // **`log=false` is on all three of the move claims.** A move the
    // screen knows git would refuse is never sent, so no refusal can
    // raise the command log — a red panel under a press that has a
    // way out on screen the whole time (2026-08-22 ユーザー判断).
    // A shut panel and a panel that was never raised are the same
    // picture, so this is the half only the report can carry.
    Verb {
        name: "switch-lands",
        when: &[(
            Arg::Starts("feature/topic-a"),
            "switch_landed branch=feature/topic-a stashes=2 wanted=2 conflicts=1 log=false",
        )],
        plain: "switch_landed branch=feature/clash stashes=0 wanted=0 conflicts=0 log=false",
    },
    // Bars with no words in them frame like bars with words: both are
    // judged on the held pill and the absent chip, neither of which a
    // run that photographed the opening could say.
    Verb {
        name: "move-ask",
        when: &[],
        plain: "move_ask hold=true code= branch=main",
    },
    // The same bar, swept rather than photographed. `words=true` is the
    // fixture half: a bar with nothing in it has air and no fields, and
    // `all=true` off one would be a green run on a screen the reader
    // never sees.
    Verb {
        name: "ask-sweep",
        when: &[],
        plain: "ask_sweep all=true caret=true hand=true words=true",
    },
    Verb {
        name: "rename-remote",
        when: &[],
        plain: "rename_ask hold=true code=",
    },
    // A bar that never came down and a bar that came down empty are
    // the same picture, and where the move ended is the half the
    // picture cannot answer at all: the run that answered the
    // question has to have landed somewhere else, and the run that
    // only raised it has to have landed nowhere.
    //
    // **The shape of the question is the claim.** A stopped
    // cherry-pick is put down with `--quit` and its tree goes into a
    // stash, so nothing is destroyed and the pill is an ordinary
    // click; a stopped rebase has to be aborted (its `--quit`
    // detaches HEAD and orphans the copies it made), so that one
    // keeps the hold. The two frame alike — a bar is a bar — and only
    // the chip and `hold=` tell them apart.
    // `branch=` is empty on purpose: a rebase runs on a detached
    // HEAD, so the working tree has no branch to name while it is
    // standing.
    Verb {
        name: "switch-stopped",
        when: &[(
            Arg::Starts("main"),
            "switch_stopped code=rebase --abort accept= hold=true bang=false op=rebase branch= log=false",
        )],
        plain: "switch_stopped code=stash accept= hold=false bang=false op=cherry-pick branch=main log=false",
    },
    // The same question with one clause fewer: no operation to put
    // down, the unmerged index the reader's own `--quit` left is the
    // whole of what is in the way. `op=` empty is the half that says
    // so — the bar itself frames exactly like `switch-stopped`'s.
    Verb {
        name: "switch-conflicted",
        when: &[],
        plain: "switch_stopped code=stash accept= hold=false bang=false op= branch=main log=false",
    },
    // And the one nothing here can clear: the pill goes to the copy
    // that has the branch instead of moving onto it, which is what
    // the `!` after the word says. No chip — git has no one command
    // for opening a working copy.
    Verb {
        name: "switch-held",
        when: &[],
        plain: "switch_stopped code= accept=Open hold=false bang=true op= branch=main log=false",
    },
    // The mark ahead of the row, which says the press raises a
    // question rather than moving. **Read from the report, not the
    // picture**: it is 16px in a full window, and the row it stands
    // on is the same row with it and without it. The argument names
    // which half of the pair the run is.
    Verb {
        name: "switch-mark",
        when: &[(
            Arg::Ends(":asks"),
            "switch_mark offered=true asks=true want=asks indent=true",
        )],
        plain: "switch_mark offered=true asks=false want=plain indent=",
    },
    Verb {
        name: "switch-stopped-go",
        when: &[],
        plain: "switch_stopped_landed branch=feature/clash op= conflicts=0 stashes=1 wanted=1 log=false",
    },
    Verb {
        name: "switch-conflicted-go",
        when: &[],
        plain: "switch_stopped_landed branch=feature/clash op= conflicts=0 stashes=1 wanted=1 log=false",
    },
];
