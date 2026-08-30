//! Where a write that walks the history comes to rest, and what is left
//! standing when git stops in the middle of one.

use super::Verb;

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
    // from `merge-stops`: their `--continue` is a step onward
    // rather than the commit somebody is writing, so the row stays —
    // and a card with a row missing frames exactly like one that has
    // it.
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
    // run button, and pushed is the amber the range earned.
    Verb {
        name: "rebase-plan",
        when: &[],
        plain: "rebase_plan rows=4 dirty=true drops=1 onto=true pushed=3",
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
];
