//! Where a write that walks the history comes to rest, and what is left
//! standing when git stops in the middle of one.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // In a history that fits, a viewport that followed frames like one
    // that never moved. `op=` names the write: `follows=` / `onscreen=`
    // both hold of a selection already at the tip.
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
    // The pull off the menu row. `op=pull` says git was given the pull
    // itself — a fetch and a merge would answer twice, under two names.
    Verb {
        name: "pull-go",
        when: &[],
        plain: "tip_landed follows=true onscreen=true op=pull",
    },
    // The same press on a branch that is ahead: the pull does nothing and
    // the screen is the one from before it, so the command log is opened.
    // `moved=false`: a graph redrawn identically frames like one untouched.
    Verb {
        name: "pull-ahead",
        when: &[],
        plain: "pull_ahead refused=false log=true moved=false",
    },
    // A divergence is refused before the press (`pull-blocked`).
    // A merge stopped on a conflict: `error=false log=false` are no red
    // line and no command log over an ordinary conflict.
    Verb {
        name: "merge-stops",
        when: &[],
        plain: "merge_stopped wip=true conflicts=true error=false log=false msg=true cont=false",
    },
    // The same stop for the three that step. `cont=` flips from
    // `merge-stops`: their `--continue` steps onward where a merge's
    // writes the commit, so the row stays on their card.
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
    // The `rebase` row's note about the range. The picture cannot date it:
    // a note a frame late moves the card's edge from under the hand
    // (規約 §行が読む答えはどこから来るか). One run per answer — nothing
    // above `v0.2` is pushed, and `feature/topic-a` forks below
    // `origin/main` so its range reaches back over it.
    Verb {
        name: "integrate-menu",
        when: &[(
            Arg::Is("v0.2:tag"),
            "integrate_menu ref=v0.2 offered=true pushed=false",
        )],
        plain: "integrate_menu ref=feature/topic-a offered=true pushed=true",
    },
    // The same stop reached through a carry: the stash the rewrite took
    // out of its way still stands, and `stashes=1` is the claim — a screen
    // that lost the work frames nearly like one holding it in the stash
    // (規約 §未コミット変更がある状態で履歴を書き換える).
    Verb {
        name: "drop-stops",
        when: &[],
        plain: "write_stopped wip=true conflicts=true error=false log=false cont=true stashes=1",
    },
    // Rewrites turned down before git is asked, through the row menu's
    // door. A window judges only the connection — the press reaching
    // core, core withholding the write, a bar coming down with the report
    // (デザイン規約 §答えの要らない報せ). Which refusal it was is not a
    // window's to say: the conditions are read by `integrate_integration`,
    // the kind by `repo_tab::drain_report_tests`, the sentence by
    // `tst_reportdress.qml` — so the cheapest history stands for them, a
    // branch of one commit.
    Verb {
        name: "fold-first-commit",
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
    // The other door: the plan's preview turns the history down before
    // the plan opens. One run for the three shapes it refuses — they
    // differ only in the heading, which `tst_reportdress.qml` reads.
    // `plan=false`: a plan opened and put away frames like one never
    // opened. `tone=warning`, not the row menu's `danger` — the plan is a
    // gesture still going; no picture reads a 2px line.
    Verb {
        name: "plan-off-branch",
        when: &[],
        plain: "write_notice open=true clears=true why=true tone=warning log=false wrong=false \
                plan=false said=Not on this branch",
    },
    // The button under the card finishes the merge, and an empty box
    // commits the message git wrote when it stopped — a merge commit and
    // an ordinary one draw the same row.
    Verb {
        name: "merge-commit",
        when: &[],
        plain: "merge_committed merging=false kept=true typed=false",
    },
    // The plan standing open with a verb of each cost. `dirty=` opens the
    // run button, `pushed=` is the amber the range earned, `selected=` the
    // row the model opened on — the highlight in the picture is the
    // page's, so nothing else notices a plan opened with no selection.
    Verb {
        name: "rebase-plan",
        when: &[],
        plain: "rebase_plan rows=4 dirty=true drops=1 onto=true pushed=3 selected=0",
    },
    // A fold carried down over the oldest place and set back where it
    // started. `carried=` is the pass through, one turn long — afterwards
    // a chip redrawn as `pick` draws like one never a fold. The rest is
    // the trip's arithmetic: the row came home, the plan is its opening
    // length, and the fold is still what the run would ask for.
    Verb {
        name: "plan-fold-carry",
        when: &[],
        plain: "plan_fold_carry carried=squash landed=squash row=1 rows=3 dirty=true",
    },
    // Escape over the plan, on both sides of what prices its exit. With
    // nothing composed `Discard` is a press, so Escape takes it; with a
    // verb set on a row it is a hold, and a key down once is not one —
    // `took=false held=true` is the plan standing its ground.
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
    // The face the plan opens with, before its rows exist: an empty pane
    // photographs the same whether a read is out, refused, or never
    // asked. `standing=false` says the picture is of the wait itself.
    Verb {
        name: "plan-loading",
        when: &[],
        plain: "plan_loading held=true shown=true standing=false rows=0 onto=false",
    },
    // The screen while git is still replaying. `counted=true` is git's own
    // step count read off `rebase-merge/msgnum`, so the badge and ring were
    // caught from a real edge; the numbers after it are wherever the
    // replay had reached.
    Verb {
        name: "replay-running",
        when: &[],
        plain: "replay_running op=REBASING counted=true ring=true held=true",
    },
    // The run's landing; `gone=` is the dropped row, read off the graph's
    // own model.
    Verb {
        name: "rebase-plan-run",
        when: &[],
        plain: "rebase_plan_ran op=rebase moved=true gone=true stopped=false plan=false error=false",
    },
    // The stop that was asked for: git's edit marker up, the skip's cost
    // back, the exit card's rows standing — over a clean tree it frames
    // like the emptied-commit stop (デザイン規約 §フル interactive rebase).
    // `box=` / `button=`: the one stepping stop that keeps the message
    // boxes and the commit button, since amending is what it stops for.
    Verb {
        name: "rebase-edit-stop",
        when: &[],
        plain: "edit_stop editing=true skipfree=false oid=true cont=true skip=true box=true button=true",
    },
    // The row's verb walked out of `reword` and back. The dropped draft and
    // the commit's own message draw the same two boxes, and a dropped one
    // resting there reads as unchanged — so the next save is refused with
    // nothing on screen to say why.
    Verb {
        name: "plan-reword-verb",
        when: &[],
        plain: "plan_reword_verb verb=reword restored=true dirty=false editable=true draft=true",
    },
    // The plan walked away from with a reword half-written: the boxes are
    // the plain amend's again, and the claim is that they hold this
    // commit's own message.
    Verb {
        name: "plan-reword-out",
        when: &[],
        plain: "plan_reword_out plan=false restored=true dirty=false editable=true",
    },
    // The closing plan finding an amend the reader half wrote before the
    // plan existed, on a row the plan never moved off. `restored=`: the
    // boxes rest on the commit's own message, so the text is unsaved;
    // `kept=`: it is still the reader's sentence. `guard=false`: Cancel
    // stays a plain press — a hold would say it costs something
    // (`plan-fold-carry` photographs the side that does).
    Verb {
        name: "plan-amend-kept",
        when: &[],
        plain: "plan_amend_kept kept=true restored=true dirty=true editable=true guard=false",
    },
    // The right pane's other two doors under a standing plan. The row
    // highlight is the page's own selection, so a model left behind draws
    // the same; a diff read under the plan is drawn nowhere; `centre=` is
    // where the middle landed once the plan was put away.
    Verb {
        name: "plan-details-held",
        when: &[],
        plain: "plan_details_held row=1 oid=true outside=true diff=false centre=graph",
    },
    // The warning the standing plan kept off the boxes: whether a remote
    // has the commit rides beside HEAD, so the plan cannot have spent it.
    // Typing after it closes has to find the boxes dirty on HEAD's row.
    Verb {
        name: "plan-reword-ask",
        when: &[],
        plain: "plan_reword_ask dirty=true onhead=true",
    },
    // Where putting the operation down leaves the reader, from this stop
    // and every other exit-card row. Over a clean tree the face it stood
    // on is empty once the operation is gone, so a reader left on it
    // frames like one taken to the commit — `wip=` and `op=` are the claim.
    Verb {
        name: "rebase-edit-stop-out",
        when: &[],
        plain: "op_exit_landed wip=false op= follows=true onscreen=true",
    },
    // What stands beside the exit card. A stopped rebase, pick or revert
    // hides the message boxes and the commit button — the card's rows are
    // the way on — and a stopped merge keeps both, its `--continue` being
    // that button. A picture of a hidden box and of an empty one differ by
    // a frame, so the drawn items say it.
    Verb {
        name: "op-exit",
        when: &[(
            Arg::WithPreset("conflict"),
            "op_exit card=true box=true button=true",
        )],
        plain: "op_exit card=true box=false button=false",
    },
    // Which exit-card row the run actually ran. The row is named by a
    // positional word, so a misspelt or missing one matches nothing — and
    // the card sizes itself to what it holds, so it crops the same.
    Verb {
        name: "op-exit-go",
        when: &[],
        plain: "op_exit_held true",
    },
    // The same landing the ordinary way: a conflict resolved, staged and
    // continued. It lands only if the rows emptying and the operation
    // going are read out of one status (`RepoPage.leaveWipWhenDone`).
    Verb {
        name: "op-exit-lands",
        when: &[],
        plain: "op_exit_landed wip=false op= follows=true onscreen=true",
    },
    // Taking the branch back. The line itself is the claim: it is written
    // once the branch reaches the commit asked for. The write answers
    // before the refs it invalidated are published, so the graph a barrier
    // photographs is the pre-reset one — which a no-op build draws too.
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
    // `--hard` answers for the tree too: it matches the commit the branch
    // landed on whatever it held, so `files=0` holds on every fixture.
    Verb {
        name: "reset-hard",
        when: &[],
        plain: "reset_landed mode=hard files=0",
    },
    // The row before the hold: the tag saying what else `--hard` takes is
    // read against the count it is drawn from, so this holds on a clean
    // fixture (no tag) as well as a dirty one.
    Verb {
        name: "reset-hard-confirm",
        when: &[],
        plain: "reset_row tagged=true",
    },
];
