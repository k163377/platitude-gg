//! Moving the working copy: where the move landed, what it carried, and
//! the bars that come down to ask before it happens.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // Where a move comes to rest, and whether the carry left an entry
    // behind; the run that stashes nothing is half of the pair.
    //
    // `log=false` is on every move line here: a move the screen knows git
    // would refuse is never sent, so no refusal may raise the command log.
    Verb {
        name: "switch-lands",
        when: &[(
            Arg::Starts("feature/topic-a"),
            "switch_landed branch=feature/topic-a stashes=2 wanted=2 conflicts=1 log=false",
        )],
        plain: "switch_landed branch=feature/clash stashes=0 wanted=0 conflicts=0 log=false",
    },
    // Two presses, and the claim is that the second was turned away: a
    // build that sent both lands the same and adds only a refused
    // `switch --create` in a panel nobody opened. The line stops at
    // `held=`: the branch is the argument's local name, and `writes=`
    // counts every answer the queue gave, which an opening fetch can join.
    Verb {
        name: "switch-remote-twice",
        when: &[],
        plain: "switch_twice held=true",
    },
    // Judged on the held pill and the absent chip: a bar with no words in
    // it frames like one with words.
    Verb {
        name: "move-ask",
        when: &[],
        plain: "move_ask hold=true code= branch=main",
    },
    // The question with a report already standing under it. Two enabled
    // `StandardKey.Cancel` shortcuts in one window fire neither
    // (`tests/qml/tst_escape.qml`), so `holds=` judges the bug: `both` /
    // `none` are it, `notice` is the order upside down. `went=` / `left=`
    // cannot catch it — the run presses the body Escape runs, so a dead
    // shortcut's bar still goes down — they say one press moves one bar.
    // `stood=both` is the fixture half: a report the question lowered
    // would make the rest of the line true and mean nothing.
    Verb {
        name: "ask-over-notice",
        when: &[],
        plain: "ask_over_notice stood=both holds=question went=question left=notice",
    },
    // The same bar, swept. `words=true` is the fixture half: `all=true` off
    // a bar with nothing in it (air, no fields) is green on a screen the
    // reader never sees.
    Verb {
        name: "ask-sweep",
        when: &[],
        plain: "ask_sweep all=true caret=true hand=true words=true",
    },
    Verb {
        name: "replace-remote",
        when: &[],
        plain: "replace_ask hold=true code=",
    },
    // A hand on the pill: the tip is the only place the bar says what the
    // hold is for, and `tip=true` is it having opened.
    Verb {
        name: "replace-remote-tip",
        when: &[],
        plain: "replace_ask hold=true code= tip=true",
    },
    // The branch half of the same question
    // (デザイン規約 §手元の改名の後のリモート). The fields are the tag
    // half's minus the two that are a tag's alone (`nav::TABLE` says what
    // each claims).
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
        // `pick=none` picks nothing: the chooser opens on the answer that
        // takes nothing away, so `shown=true` and the pill is live from the
        // first frame.
        plain: "rename_carry kind=branch rows=3 pick=none shown=true answerable=true \
                hold=false neutral=true pill=Create",
    },
    // …and a hand on the held answer's pill, the only one with a tip: it
    // says the running order.
    Verb {
        name: "rename-local-upstream-tip",
        when: &[],
        plain: "rename_carry kind=branch rows=3 pick=replace shown=true answerable=true \
                hold=true neutral=false pill=Replace tip=true",
    },
    // …and the answer given. Not `replace`: the renamed branch is the
    // working tree's, whose upstream is the remote's own HEAD, which git
    // refuses to delete — that write is `replace-remote-go`'s. An argument
    // naming no answer takes `add`.
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
    // Where the move ended is a claim: the run that answered the question
    // has to have landed elsewhere, the one that only raised it nowhere.
    //
    // The shape of the question is the other. A stopped cherry-pick is put
    // down with `--quit` and its tree stashed, so nothing is destroyed and
    // the pill is a click; a stopped rebase has to be aborted (its `--quit`
    // detaches HEAD and orphans the copies it made), so it keeps the hold.
    // `branch=` is empty on purpose: a rebase runs on a detached HEAD.
    Verb {
        name: "switch-stopped",
        when: &[(
            Arg::Starts("main"),
            "switch_stopped code=rebase --abort accept= hold=true bang=false op=rebase branch= log=false",
        )],
        plain: "switch_stopped code=stash accept= hold=false bang=false op=cherry-pick branch=main log=false",
    },
    // One clause fewer: no operation to put down, only the unmerged index
    // the reader's own `--quit` left. The empty `op=` says so — the bar
    // frames like `switch-stopped`'s.
    Verb {
        name: "switch-conflicted",
        when: &[],
        plain: "switch_stopped code=stash accept= hold=false bang=false op= branch=main log=false",
    },
    // The one nothing here can clear, so not a question: the press stands
    // the tab in the copy that has the branch. `bar=false` catches a
    // question bar raised instead — a run that only waited for the copy
    // would sit out its ceiling saying nothing about why.
    Verb {
        name: "switch-held",
        when: &[],
        plain: "switch_held stood=true bar=false",
    },
    // The mark ahead of the row saying the press raises a question — read
    // from the report, since it is 16px in a full window. The argument
    // names which half of the pair the run is.
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
