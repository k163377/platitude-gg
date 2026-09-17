//! Where a write that walks the history comes to rest, and what is left
//! standing when git stops in the middle of one.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // Half of these the picture holds — which row is lit, and whose
    // commit fills the pane; in a history that fits, a viewport that
    // followed frames like one that never moved. `op=` is the half the
    // pair cannot say: both hold of a selection already at the tip.
    Verb {
        name: "revert-commit",
        when: &[],
        plain: "tip_landed follows=true onscreen=true op=revert",
    },
    Verb {
        name: "cherry-pick",
        when: &[],
        plain: "tip_landed follows=true onscreen=true op=cherry-pick",
    },
    Verb {
        name: "merge-branch",
        when: &[],
        plain: "tip_landed follows=true onscreen=true op=merge",
    },
    // The other landing the same press has. Two halves the picture
    // cannot hold either: that no red line was written over an
    // ordinary conflict, and that the command log stayed down.
    Verb {
        name: "merge-stops",
        when: &[],
        plain: "merge_stopped wip=true conflicts=true error=false log=false msg=true cont=false",
    },
    // The same landing for the three that step. `cont=` flips over
    // from `merge-stops`: their `--continue` is a step onward, where
    // a merge's is a commit being written, so the row stays
    // — and a card with a row missing frames exactly like one that
    // has it.
    Verb {
        name: "cherry-pick-stops",
        when: &[],
        plain: "write_stopped wip=true conflicts=true error=false log=false cont=true",
    },
    Verb {
        name: "revert-stops",
        when: &[],
        plain: "write_stopped wip=true conflicts=true error=false log=false cont=true",
    },
    Verb {
        name: "rebase-stops",
        when: &[],
        plain: "write_stopped wip=true conflicts=true error=false log=false cont=true",
    },
    // The menu before any of that runs, and what its `rebase` row says
    // about the range. **The picture cannot date the note**: one read
    // off the rows as the card opens and one arriving a frame later
    // photograph the same, and only the second moves the card's edge out
    // from under the hand (規約 §行が読む答えはどこから来るか). Two runs
    // because one answer proves nothing — `v0.2` sits on `origin/main`
    // so nothing above it is pushed, and `feature/topic-a` forks below
    // that tip so the range reaches back over it.
    Verb {
        name: "integrate-menu",
        when: &[(
            Arg::Is("v0.2:tag"),
            "integrate_menu ref=v0.2 offered=true pushed=false",
        )],
        plain: "integrate_menu ref=feature/topic-a offered=true pushed=true",
    },
    // The same landing reached through a carry, where the stash the
    // rewrite took out of its own way is still standing. Nothing
    // raises git's words over it any more, so the count is the claim:
    // a screen that lost the work and one that is holding it in the
    // stash frame nearly alike (規約 §未コミット変更がある状態で
    // 履歴を書き換える).
    Verb {
        name: "drop-stops",
        when: &[],
        plain: "write_stopped wip=true conflicts=true error=false log=false cont=true stashes=1",
    },
    // The rewrites turned down before git is asked. **The picture is
    // half the claim** — a bar came down over the graph — and the line
    // is the other half: which of the five it was, and that the report
    // reached the page at all (デザイン規約 §答えの要らない報せ).
    // `ref=/` because a rewrite is about no ref: the row it was
    // pressed on is what it is about, and the row is
    // in the picture.
    Verb {
        name: "fold-across-merge",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=The history was not rewritten",
    },
    Verb {
        name: "fold-off-branch",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=The history was not rewritten",
    },
    Verb {
        name: "fold-first-commit",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=The history was not rewritten",
    },
    Verb {
        name: "fold-unfetched-base",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=The history was not rewritten",
    },
    Verb {
        name: "drop-last-commit",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=danger log=false wrong=false \
                said=The history was not rewritten",
    },
    // The same three histories through the other door: the plan's preview
    // turns them down before it opens, and the bar it raises is the whole
    // answer. **The heading is where the two doors part** — this one is
    // about the range that would be replayed, the row menu's about the one
    // commit that was pressed — so `said=` is what each row here is for,
    // and the line under it is the same sentence either way
    // (`Words.rewriteRefusedWhy`). `tone=warning` is the other half: the
    // plan is a gesture still going, and no picture can read a 2px line.
    Verb {
        name: "plan-across-merge",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=warning log=false wrong=false \
                said=A merge is in the way",
    },
    Verb {
        name: "plan-off-branch",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=warning log=false wrong=false \
                said=Not on this branch",
    },
    Verb {
        name: "plan-unfetched-base",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=warning log=false wrong=false \
                said=The history stops here",
    },
    // The other end of the same merge: the button under the card is
    // what finishes it, and an empty box commits the message git
    // wrote when it stopped. Neither half is in the picture — a
    // merge commit and an ordinary one draw the same row.
    Verb {
        name: "merge-commit",
        when: &[],
        plain: "merge_committed merging=false kept=true typed=false",
    },
    // The plan stood open with a verb of each cost on it. The counts
    // are the halves a picture underclaims: dirty is what opens the
    // run button, pushed is the amber the range earned, and selected is
    // the row the model opened on — the highlight in the picture is the
    // page's, so nothing else here would notice a plan that opened with
    // no selection at all.
    Verb {
        name: "rebase-plan",
        when: &[],
        plain: "rebase_plan rows=4 dirty=true drops=1 onto=true pushed=3 selected=0",
    },
    // A fold carried down over the oldest place and set back down where
    // it started. `carried=` is the half no picture holds: the pass
    // through is one turn long, and afterwards a chip redrawn as `pick`
    // and a row that was never a fold draw exactly the same. The three
    // after it are the trip's own arithmetic — the row came home, the
    // plan is the length it opened at, and the fold is still what the run
    // would ask for.
    Verb {
        name: "plan-fold-carry",
        when: &[],
        plain: "plan_fold_carry carried=squash landed=squash row=1 rows=3 dirty=true",
    },
    // Escape over the plan, on the two sides of the one thing that
    // prices its exit. With nothing composed `Discard` is a press, so
    // Escape is that press and the face goes; with a verb set on a row
    // it is a hold, and a key that is down once is not one — `took=false
    // held=true` is the plan standing its ground, which is the whole of
    // the second run. Neither is in the picture: a plan that was never
    // opened and a plan Escape put away photograph the same graph.
    Verb {
        name: "plan-escape",
        when: &[],
        plain: "plan_escape discards=false took=true held=false",
    },
    Verb {
        name: "plan-escape-held",
        when: &[],
        plain: "plan_escape discards=true took=false held=true dirty=true",
    },
    // The face the plan opens with, before its rows exist. Every half
    // of it is invisible: an empty pane photographs the same whether a
    // read is out, was refused, or was never asked for — and
    // `standing=false` is what says the picture is of the wait
    // itself.
    Verb {
        name: "plan-loading",
        when: &[],
        plain: "plan_loading held=true shown=true standing=false rows=0 onto=false",
    },
    // The screen while git is still replaying. The picture holds the
    // badge and the ring; what it cannot say is that either of them
    // was caught from a real edge — `counted=true` is git's own step
    // count read off `rebase-merge/msgnum`, and the
    // numbers after it are whatever the replay had reached.
    Verb {
        name: "replay-running",
        when: &[],
        plain: "replay_running op=REBASING counted=true ring=true held=true",
    },
    // The run's landing: answered by name, the dropped row gone from
    // the graph's own model, the draft away, and no red line raised.
    Verb {
        name: "rebase-plan-run",
        when: &[],
        plain: "rebase_plan_ran op=rebase moved=true gone=true stopped=false plan=false error=false",
    },
    // The stop that was asked for: git's edit marker up, the skip's
    // cost back with it, and the exit card's rows still standing — a
    // clean tree frames exactly like the emptied-commit stop without
    // these (P3-確認事項 §A).
    Verb {
        name: "rebase-edit-stop",
        when: &[],
        plain: "edit_stop editing=true skipfree=false oid=true cont=true skip=true",
    },
    // The row's verb walked out of `reword` and back into it. What the
    // picture cannot say is whose message the boxes are holding: the
    // dropped draft and the commit's own message draw the same two boxes,
    // and the dropped one rests there reading as unchanged — so the next
    // save is refused with nothing on screen to say why.
    Verb {
        name: "plan-reword-verb",
        when: &[],
        plain: "plan_reword_verb verb=reword restored=true dirty=false editable=true draft=true",
    },
    // The plan walked away with a reword half-written. The boxes are the
    // plain amend's again the moment it does, and the same two boxes are
    // drawn either way — so the whole claim is that what rests in them is
    // this commit's own message.
    Verb {
        name: "plan-reword-out",
        when: &[],
        plain: "plan_reword_out plan=false restored=true dirty=false editable=true",
    },
    // The other text the same closing plan finds in the boxes: an amend the
    // reader had half written before any plan existed, on a row the plan
    // never asked to move off. `restored=` says the boxes rest on the
    // commit's own message, so what stands in them is unsaved — and
    // `kept=` says it is still the reader's sentence. Nobody can retype
    // it from the screen, so losing it is not a redraw. `guard=false` is
    // the other side of that: this text is the reader's, so Cancel stays
    // a plain press, where the hold would say the press costs
    // something (`plan-fold-carry` photographs the side that does).
    Verb {
        name: "plan-amend-kept",
        when: &[],
        plain: "plan_amend_kept kept=true restored=true dirty=true editable=true guard=false",
    },
    // The right pane's other two doors under a standing plan. Every half
    // of this one is invisible: the row highlight is the page's own
    // selection, so a model left behind draws the same picture; a diff
    // read under the plan is drawn nowhere; and `centre=` is where the
    // middle landed once the plan was put away, which the graph in the
    // shot cannot tell from a graph that was never left.
    Verb {
        name: "plan-details-held",
        when: &[],
        plain: "plan_details_held row=1 oid=true outside=true diff=false centre=graph",
    },
    // And the warning the standing plan was keeping off the boxes:
    // whether a remote already has the commit rides beside HEAD, so a
    // plan that stood over the boxes cannot have spent it. Typing again
    // after it closes has to find the boxes dirty on HEAD's own row —
    // nothing in the picture says which commit the warning is
    // about.
    Verb {
        name: "plan-reword-ask",
        when: &[],
        plain: "plan_reword_ask dirty=true onhead=true",
    },
    // The far end of the same stop, and of every other exit-card row:
    // where putting the operation down leaves the reader. The face it
    // was standing on is empty over a clean tree once the operation is
    // gone, so a reader left on it frames exactly like one who was taken
    // to the commit — `wip=` and `op=` are the whole claim, and neither
    // is in the picture.
    Verb {
        name: "rebase-edit-stop-out",
        when: &[],
        plain: "op_exit_landed wip=false op= follows=true onscreen=true",
    },
    // Which of the exit card's rows the run actually ran. The card sizes
    // itself to what it holds, so a row that was never reached crops to
    // the same picture as one that ran and took its operation with it —
    // and the row is named by a positional word, where a misspelling
    // (or a forgotten argument) is simply a row nothing matches.
    Verb {
        name: "op-exit-go",
        when: &[],
        plain: "op_exit_held true",
    },
    // The same landing reached the ordinary way: a conflict resolved,
    // staged, and continued. Its file rows do empty, so this is the half
    // that was reachable before an operation could hold the face open —
    // and it lands only if the emptying and the operation's going are
    // read out of one status (`RepoPage.leaveWipWhenDone`).
    Verb {
        name: "op-exit-lands",
        when: &[],
        plain: "op_exit_landed wip=false op= follows=true onscreen=true",
    },
    // Taking the branch back. **The line's own existence is the claim**:
    // it is written on the far side of the branch arriving at the commit
    // the run asked for, and a reset that never reached git leaves the
    // run in its watchdog with nothing said. The picture cannot stand in
    // for it — the write answers before the refs it invalidated are
    // published, so the graph a barrier photographs is the one from
    // before the reset, which is also what a build that reset nothing
    // draws.
    Verb {
        name: "reset-soft",
        when: &[],
        plain: "reset_landed mode=soft",
    },
    Verb {
        name: "reset-mixed",
        when: &[],
        plain: "reset_landed mode=mixed",
    },
    // And the one mode that answers for the working tree as well: after
    // `--hard` the tree matches the commit the branch landed on, whatever
    // it held before, so `files=0` holds on every fixture — a build that
    // moved the branch and left the tree alone says a number here.
    Verb {
        name: "reset-hard",
        when: &[],
        plain: "reset_landed mode=hard files=0",
    },
    // The row before the hold runs: the tag saying what else `--hard`
    // takes is read back against the count it is drawn from, so this
    // holds over a clean fixture (no tag, nothing to lose) as well as a
    // dirty one.
    Verb {
        name: "reset-hard-confirm",
        when: &[],
        plain: "reset_row tagged=true",
    },
];
