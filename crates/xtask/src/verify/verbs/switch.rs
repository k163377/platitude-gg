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
    // way out on screen the whole time.
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
    // The same landing reached by two presses, where the claim is that
    // the second one was turned away. **Only the report can say it**: a
    // build that sent both frames exactly like this one — the branch is
    // the branch either way — and what it adds is a refused
    // `switch --create` in a panel nobody opened. The line stops at
    // `held=` so the rest of it stays readable: the branch is the
    // argument's local name, and `writes=` counts every answer the queue
    // gave, which an opening fetch can join.
    Verb {
        name: "switch-remote-twice",
        when: &[],
        plain: "switch_twice held=true",
    },
    // Bars with no words in them frame like bars with words: both are
    // judged on the held pill and the absent chip, neither of which a
    // run that photographed the opening could say.
    Verb {
        name: "move-ask",
        when: &[],
        plain: "move_ask hold=true code= branch=main",
    },
    // The same question, with a report already standing under it. Two
    // enabled `StandardKey.Cancel` shortcuts in one window fire neither
    // (`tests/qml/tst_escape.qml`), so `holds=` is the half that judges
    // the bug: `both` is it (measured — the build before the order was
    // written says exactly that), `none` is the same thing from the
    // other side, and `notice` is the order upside down. **`went=` and
    // `left=` cannot catch it on their own** — the run presses the body
    // Escape runs, so a bar whose shortcut is dead still goes down when
    // it is pressed; what they add is that the
    // press moves one bar and only one. And `stood=both` is the fixture
    // half: a report the question quietly lowered would make the rest
    // of the line true and mean nothing.
    Verb {
        name: "ask-over-notice",
        when: &[],
        plain: "ask_over_notice stood=both holds=question went=question left=notice",
    },
    // The same bar, swept. `words=true` is the
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
    // The branch half of the same question: a name changed here, and what
    // the remote it was measured against does about it
    // (デザイン規約 §手元の改名をリモートへ運ぶ). The fields are the tag
    // half's, minus the two that are a tag's alone
    // (`nav::TABLE`, where what each one claims is written down).
    Verb {
        name: "rename-local-upstream",
        when: &[
            (
                Arg::Ends(":replace"),
                "rename_carry kind=branch rows=3 pick=replace shown=true answerable=true \
                 hold=true neutral=false pill=Replace",
            ),
            (
                Arg::Ends(":add"),
                "rename_carry kind=branch rows=3 pick=add shown=true answerable=true hold=false \
                 neutral=true pill=Create",
            ),
            (
                Arg::Ends(":leave"),
                "rename_carry kind=branch rows=3 pick=leave shown=true answerable=true \
                 hold=false neutral=true pill=Leave",
            ),
        ],
        // `pick=none` is the run that picks nothing, and what stands in
        // the chooser then is the answer the question opens on — the one
        // that takes nothing away, which is why `shown=true` and the pill
        // is live from the first frame.
        plain: "rename_carry kind=branch rows=3 pick=none shown=true answerable=true \
                hold=false neutral=true pill=Create",
    },
    // …and the same road with the answer given. **`replace` is not on
    // this one**: the branch it renames is the one the working tree is
    // on, whose upstream is the remote's own HEAD, and git refuses to
    // delete that — the write behind that answer is `rename-remote-go`'s.
    // An argument naming no answer takes `add`.
    Verb {
        name: "rename-local-upstream-go",
        when: &[(
            Arg::Ends(":leave"),
            "rename_carry kind=branch rows=3 pick=leave shown=true answerable=true hold=false \
             neutral=true pill=Leave",
        )],
        plain: "rename_carry kind=branch rows=3 pick=add shown=true answerable=true hold=false \
                neutral=true pill=Create",
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
    // that has the branch, which is what the `!` after the word
    // says. No chip — git has no one command
    // for opening a working copy.
    Verb {
        name: "switch-held",
        when: &[],
        plain: "switch_stopped code= accept=Open hold=false bang=true op= branch=main log=false",
    },
    // The mark ahead of the row, which says the press raises a
    // question. **Read from the report**: it is 16px in a full
    // window, and the row it stands on is the same row with it and
    // without it. The argument names which half of the pair the run
    // is.
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
