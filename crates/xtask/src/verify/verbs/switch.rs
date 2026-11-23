//! Switching the worktree to another branch: where it landed, what it carried, and
//! the bars that come down to ask before it happens.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // Where a move comes to rest, and whether the carry left an entry
    // behind; the run that stashes nothing is half of the pair.
    //
    // `log=false` is on every move line here: a move the screen knows git
    // would refuse is never sent, so no refusal may raise the command log.
    // The carried half (`--preset switch-carry`): a tree dirty on main is
    // stashed around the move and popped after it; the pop conflicts on
    // `feature/topic-a`, so the entry stays beside the one already there
    // (`stashes=2`) and the tree holds the conflict (`conflicts=1`,
    // `session::build` on carrying).
    Verb {
        name: "switch-lands",
        when: &[(
            Arg::IsOn("feature/topic-a:2", "switch-carry"),
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
    // it frames like one with words. `says=` is the count core read, on
    // its way through to the bar — `--preset diverged`'s main holds one
    // commit origin/main does not.
    Verb {
        name: "move-ask",
        when: &[],
        plain: "move_ask hold=true code= branch=main says=main loses 1 commit.",
    },
    // The question asked with a report already standing: it waits behind
    // the report, off screen, and comes down once the report is back up.
    // `shown=notice` is the one-bar-at-a-time claim, and `then=question`
    // that the question was kept, not dropped. Two enabled
    // `StandardKey.Cancel` shortcuts in one window fire neither
    // (`tests/qml/tst_escape.qml`), so `holds=` judges that bug: `both` /
    // `none` are it, `question` is a key on a bar nobody sees. `went=`
    // cannot catch it — the run presses the body Escape runs, so a dead
    // shortcut's bar still goes up — it says one press moves one bar.
    // `waiting=question` is the fixture half: a question never raised
    // would make the rest of the line true and mean nothing.
    Verb {
        name: "ask-over-notice",
        when: &[],
        plain: "ask_over_notice shown=notice waiting=question holds=notice went=notice then=question in_place=true",
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
            // The old upstream's row taken away while the bar stands: the
            // two answers left, the upstream's setting worn, and the line
            // that says why.
            (
                Arg::Ends(":gone"),
                "rename_carry kind=branch rows=2 pick=gone shown=true answerable=true hold=false \
                 neutral=true pill=Set line=true",
            ),
            // …and one gone before the rename (`--preset gone-out`): the
            // bar comes down on the two answers and their line from its
            // first frame.
            (
                Arg::IsOn("release-1.3", "gone-out"),
                "rename_carry kind=branch rows=2 pick=none shown=true answerable=true hold=false \
                 neutral=true pill=Set line=true",
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
        when: &[
            (
                Arg::Ends(":leave"),
                "rename_carry kind=branch rows=3 pick=leave shown=true answerable=true hold=false \
                 neutral=true pill=Leave",
            ),
            // …and what `Set` wrote: the upstream on the new name, nothing
            // sent.
            (
                Arg::Ends(":gone"),
                "rename_carry kind=branch rows=2 pick=gone shown=true answerable=true hold=false \
                 neutral=true pill=Set line=true upstream=origin/main-renamed sent=false",
            ),
        ],
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
    //
    // `main` is the branch already out: the press moves nothing, so it is
    // turned away before the question (`pressed=false`) and no question was
    // raised (`code=` empty, `bar=false` — both set as a question is asked,
    // in the press's own turn) — a build that asks raises a bar whose answer
    // sends nothing at all.
    Verb {
        name: "switch-conflicted",
        when: &[(
            Arg::Starts("main"),
            "switch_stopped code= accept= hold=false bang=false op= branch=main log=false pressed=false bar=false",
        )],
        plain: "switch_stopped code=stash accept= hold=false bang=false op= branch=main log=false",
    },
    // The one nothing here can clear, so not a question: the press stands
    // the tab in the worktree that has the branch. `bar=false` catches a
    // question bar raised instead — a run that only waited for the worktree
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
    // The same landing, from an unmerged index instead of an operation:
    // not a character apart from `switch-stopped-go`'s, so the proof is
    // the pair's. `switch-conflicted` (no `-go`) claims the empty `op=` as
    // the bar stands, and a run on the wrong preset fails there. `--preset
    // cherry-pick-quit` is the one shape with no operation standing (the
    // reader's own `--quit` left the index); `conflict-staged` cannot
    // stand for it.
    Verb {
        name: "switch-conflicted-go",
        when: &[],
        plain: "switch_stopped_landed branch=feature/clash op= conflicts=0 stashes=1 wanted=1 log=false",
    },
    // The way cleared under the standing question from this window: the
    // bar stands and asks the plain switch, its line naming what went.
    // `:abort --preset rebase-conflict` empties the tree, so the page lands
    // on HEAD by itself first — a landing that took the question down
    // reads `bar=false`, and one that left the stash words reads
    // `code=stash`. `:resolve --preset cherry-pick-quit` keeps the
    // uncommitted row and has no operation to name.
    Verb {
        name: "switch-cleared",
        when: &[(
            Arg::IsOn("main:abort", "rebase-conflict"),
            "switch_cleared bar=true code=switch neutral=true hold=false op= detail=The rebase has ended.",
        )],
        plain: "switch_cleared bar=true code=switch neutral=true hold=false op= detail=No conflicts are left.",
    },
];
