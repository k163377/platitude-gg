pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The interactive-rebase plan: opening one over a row, dressing its rows with verbs, running it, and the
/// stop a run asks for.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given. What these verbs act on
/// hangs off that driver; the names it owns are read back once below so the verbs can name them bare.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing
// and is never given a size.
Item {
    id: acts

    /// The driver these verbs belong to. `var` because naming its type here would be a circle: it is
    /// the file that builds this one.
    required property var driver

    // The driver's own names, read once so the verbs can name them bare.
    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var workTree: driver.workTree
    readonly property var graphModel: driver.graphModel
    readonly property var wipPane: driver.wipPane
    readonly property var planPane: driver.planPane
    readonly property var detailsPane: driver.detailsPane
    readonly property var detailsModel: driver.detailsModel
    readonly property var commitMenu: driver.commitMenu
    readonly property var renderedBarrier: driver.barrierRendered

    /// The walks over the right pane, which this file's dispatch starts by name. Theirs is the one subject here that
    /// is not the plan face itself, and one of them begins before a plan exists at all.
    AutoActPlanBoxVerbs { id: boxes; driver: acts.driver }

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — no verb is named by two of them (`AutoActDriver`).
    function run(act, arg) {
        if (act === "rebase-plan" || act === "rebase-plan-run" || act === "rebase-edit-stop"
            || act === "rebase-edit-stop-out" || act === "plan-reword-verb"
            || act === "plan-reword-out" || act === "plan-reword-ask" || act === "plan-loading"
            || act === "plan-details-held" || act === "plan-fold-carry") {
            // Exercise the menu entry and its handler, then dismiss the menu as the actual click does.
            // Each verb selects the smallest plan its result needs — the three about the right pane's boxes need
            // only that the newest commit be a row of it, since that is the row the plan opens the selection on,
            // and the carry needs three: one to hold the fold, the oldest to carry it over, and one above.
            const back = act === "rebase-plan" ? 3
                       : act === "rebase-plan-run" || act === "plan-fold-carry" ? 2
                       : 1
            const fromOid = arg !== "" ? driver.autoActOid(arg)
                          : graphModel.oidAt(graphModel.rowOf(workTree.headOid) + back)
            page.openRowMenu(fromOid)
            page.startRebasePlan(fromOid)
            commitMenu.close()
            if (act === "plan-loading")
                planLoadingTimer.start()
            else
                planOpenTimer.begin(act)
        } else if (act === "plan-across-merge" || act === "plan-off-branch"
                   || act === "plan-unfetched-base") {
            // The same press, on the three histories a preview turns down before the plan opens (`PlanRefusal`).
            // Nothing is written and no plan comes back, so the answer is the bar in the middle and the notice
            // barrier is the only one that can finish the run.
            //
            // Each is its own verb because each is caught saying a different heading — the picture cannot tell
            // a bar that came down from a log that came up, and the three bars differ only in their words.
            //
            // The row is what tells the preset's refusal apart: one below the tip is the merge in `rewrite-merge`
            // and the oldest row a `--depth 2` clone holds in `shallow`, while the commit no branch of
            // `one-commit` can see is row 0 and has to be named.
            const refusedOid = arg !== "" ? driver.autoActOid(arg)
                             : graphModel.oidAt(graphModel.rowOf(workTree.headOid) + 1)
            page.openRowMenu(refusedOid)
            page.startRebasePlan(refusedOid)
            commitMenu.close()
            driver.barrierNotice.start()
        } else if (act === "plan-amend-kept") {
            // The one order in which a plan closing over the boxes meets text that was never the plan's: type the
            // amend first, and open the plan from HEAD's own row so nothing moves the selection. Every other way in
            // right-clicks a row further down, and **that** is what takes the draft — the arrows and the clicks
            // walk off an unsaved message the same way (`graph-step-dirty`), which is not what this asks about.
            const tip = workTree.headOid !== "" ? workTree.headOid : driver.branchesModel.headOid
            page.jumpToRef(tip)
            boxes.boxAmend.begin(tip)
        } else {
            return false
        }
        return true
    }
    // ---- the face the press puts up before any of that -----------------
    // The pane in the graph's seat with nothing in it but the mode's one word, `Discard` and the turning mark. The face
    // is **held** from the edge that raised it (`RepoPage.planLoadHeld`): the rows arrive through the feed and can
    // land while the asynchronous grab is still out, and the picture would then be of the plan rather than of the
    // wait for it (verify-ui スキル §中間状態は実 edge を latch する).
    //
    // **The edge is caught on the model's own signal, not sampled.** `open()` raises `loading` inside the dispatch
    // and the answer to a five-commit range can be back before the next tick, so a sampler looking for it would find
    // a state that had already gone and wait out the watchdog in silence.
    Connections {
        target: AppBackend.autoAct === "plan-loading" ? page.rebasePlan : null
        function onChanged() {
            const plan = page.rebasePlan
            if (page.planLoadHeld || !plan.loading || plan.active)
                return
            planLoadingTimer.sawRows = plan.stepCount
            planLoadingTimer.sawOnto = plan.ontoRef !== "" || plan.ontoOid !== ""
            page.planLoadHeld = true
        }
    }
    // Completed a turn later and off the page's own answer rather than in the callback that pressed
    // (app-ui.md §UI 自動化の因果性). Nothing in the line can be read off the picture: an empty pane photographs the same
    // whether a read is out, was refused, or was never asked for.
    SampleTimer {
        id: planLoadingTimer
        /// What the edge saw, kept for the report: read back afterwards these would be the *plan's* — the model goes
        /// on filling itself while only the face is held.
        property int sawRows: -1
        property bool sawOnto: false
        onTriggered: {
            if (!page.planLoadHeld || !page.planShown || page.planActive)
                return
            planLoadingTimer.stop()
            AppBackend.report("plan_loading held=true shown=true standing=" + page.planActive
                              + " rows=" + planLoadingTimer.sawRows
                              + " onto=" + planLoadingTimer.sawOnto)
            renderedBarrier.begin()
        }
    }
    // The plan's own three landings. First the open: rows arrive through the feed, so the verbs wait for the model
    // to say the plan stands before touching it — and the overview also waits for the publish answer, since the
    // pushed count is half of what its picture claims.
    SampleTimer {
        id: planOpenTimer
        property string act: ""
        function begin(which) {
            planOpenTimer.act = which
            planOpenTimer.start()
        }
        onTriggered: {
            const plan = page.rebasePlan
            if (!plan.active || plan.loading || plan.stepCount === 0)
                return
            if (planOpenTimer.act === "rebase-plan" && page.planPushed === 0)
                return
            planOpenTimer.stop()
            if (planOpenTimer.act === "rebase-plan") {
                // One of each look: a fold leaning into its parent and a drop struck through. The selection is
                // already on the newest row — `RebasePlanModel::take` opens it there — so the right pane is that
                // commit's. **`selected=` is in the report because the picture cannot say it**: the row's own
                // highlight reads `page.selectedOid`, which the `activateRow` below sets, so a model that opened
                // with nothing selected would photograph exactly the same.
                plan.setAction(1, "squash")
                plan.setAction(2, "drop")
                page.activateRow(workTree.headOid)
                AppBackend.report("rebase_plan rows=" + plan.stepCount + " dirty=" + plan.dirty
                                  + " drops=" + plan.dropCount + " onto=" + (plan.ontoRef !== "")
                                  + " pushed=" + page.planPushed + " selected=" + plan.selectedRow)
                renderedBarrier.begin()
            } else if (planOpenTimer.act === "rebase-plan-run") {
                // Dropping a commit origin still reaches, so the *dropped row stays drawn* — the old line is
                // origin/main's to keep (§履歴を合流させる). What this write moves is the branch's own tip: the old
                // head's row goes to the reflog and the rewritten one takes its place, and that pair is the barrier
                // (app-ui.md — その書き込みが動かす当のモデルの行を待つ).
                plan.setAction(plan.stepCount - 1, "drop")
                driver.writeSeqBefore = repoTab.writeSeq
                planRanTimer.begin(workTree.headOid)
                plan.runPlan()
            } else if (planOpenTimer.act === "plan-details-held") {
                // The row the plan opened on is already the selected one, and the right pane is already its
                // commit's (`RepoPage.onPlanActiveChanged`) — the walk starts from exactly the screen a reader
                // gets, with no verb dressed on anything.
                boxes.boxHeld.begin()
            } else if (planOpenTimer.act === "plan-fold-carry") {
                // The reorder as a hand makes it: a fold taken up, carried down over the oldest place — where no
                // fold can stand — and set back down where it started. Driven through the list's own functions,
                // which are the ones the row's `MouseArea` calls (`RebasePlanRow`), so the two brackets the fold
                // rule waits on are under test rather than assumed.
                //
                // **Nothing here is timed.** A move is answered inside the call, so the whole trip is one turn —
                // and that is the point: the demotion used to land on the way through, at a moment no sampler
                // could have caught either.
                //
                // The selection is put on the carried row first, so `selectedAction` *is* that row's verb for the
                // whole trip (the reorder remaps the selection with it) — the model has no other way to be asked
                // for one row's verb, and the picture cannot answer it: a chip redrawn as `pick` and a row that
                // was never a fold photograph the same.
                const carried = plan.stepCount - 2
                plan.setAction(carried, "squash")
                plan.selectRow(carried)
                planPane.view.moveBegan()
                planPane.view.moveRequested(carried, carried + 1)
                const underHand = plan.selectedAction
                planPane.view.moveRequested(carried + 1, carried)
                planPane.view.moveEnded()
                AppBackend.report("plan_fold_carry carried=" + underHand
                                  + " landed=" + plan.selectedAction
                                  + " row=" + plan.selectedRow + " rows=" + plan.stepCount
                                  + " dirty=" + plan.dirty)
                renderedBarrier.begin()
            } else if (planOpenTimer.act.startsWith("plan-reword")) {
                // The newest row is already the selected one (`RebasePlanModel::take`), and the plan already put the
                // right pane on its commit (`RepoPage.onPlanActiveChanged`) — all that is missing is the verb that
                // makes the boxes the row's.
                plan.setAction(0, "reword")
                boxes.boxReword.begin(planOpenTimer.act)
            } else {
                plan.setAction(plan.stepCount - 1, "edit")
                planEditStopTimer.start()
                plan.runPlan()
            }
        }
    }
    // The run's landing: the plan is away, the write answered by name, and the branch's tip rewritten — the old
    // head's row gone to the reflog, the new one drawn. The *dropped* row is no barrier at all here: origin still
    // reaches it, so its old line stays drawn on purpose (§履歴を合流させる).
    SampleTimer {
        id: planRanTimer
        property string headBefore: ""
        /// The run's own answer, picked out of the answers that notify carried the way `tipLandedTimer.answeredOp`
        /// is — the fetch the freeze leaves running rewrites the answer group with every answer of its own, so a
        /// beat that reads the sampled name can find a fetch's where the rebase's stood (same measured failure).
        /// One-way and by name: a fetch answering *first* must not take the arm, so only the rebase's answer sets
        /// it, and what it said about stopping and failing is taken in the same breath.
        property string answeredOp: ""
        property bool answeredStopped: false
        property bool answeredError: false
        function begin(oid) {
            planRanTimer.headBefore = oid
            planRanTimer.answeredOp = ""
            planRanTimer.answeredStopped = false
            planRanTimer.answeredError = false
            planRanTimer.start()
        }
        onTriggered: {
            if (planRanTimer.answeredOp === "" || repoTab.busyCount !== 0 || page.rebasePlan.active
                    || workTree.headOid === planRanTimer.headBefore
                    || workTree.headOid === ""
                    || graphModel.rowOf(workTree.headOid) < 0
                    || graphModel.rowOf(planRanTimer.headBefore) >= 0)
                return
            planRanTimer.stop()
            AppBackend.report("rebase_plan_ran op=" + planRanTimer.answeredOp
                              + " moved=" + (workTree.headOid !== planRanTimer.headBefore)
                              + " gone=" + (graphModel.rowOf(planRanTimer.headBefore) < 0)
                              + " stopped=" + planRanTimer.answeredStopped
                              + " plan=" + page.rebasePlan.active
                              + " error=" + planRanTimer.answeredError)
            renderedBarrier.begin()
        }
    }
    /// **Read out of the answers the notify carried, not off the group they leave behind.** One drain empties the
    /// whole queue and notifies once (`RepoTab::write_answers`), so the fetch coming back behind the run arrives in
    /// the same beat and the group is left describing *it* — the name this is waiting for was never on screen for a
    /// moment, and the run walks into its watchdog instead. Each answer keeps its own stop and refusal, so the two
    /// read here are the rebase's.
    Connections {
        target: driver.repoTab
        function onWriteSeqChanged() {
            if (!planRanTimer.running || planRanTimer.answeredOp !== "")
                return
            const tab = driver.repoTab
            for (let i = 0; i < tab.writeAnswerCount(); i++) {
                if (tab.writeAnswerSeq(i) <= driver.writeSeqBefore || tab.writeAnswerOp(i) !== "rebase")
                    continue
                planRanTimer.answeredOp = tab.writeAnswerOp(i)
                planRanTimer.answeredStopped = tab.writeAnswerStopped(i)
                planRanTimer.answeredError = tab.writeAnswerFailed(i)
                return
            }
        }
    }
    // The stop that was asked for. Its tree is clean — nothing conflicted — so what the picture cannot hold is the
    // reason: git's own edit marker, the skip whose cost came back with it, and the exit card's rows still standing
    // (P3-確認事項 §A / デザイン規約 §進行中の操作から出る).
    //
    // The `-out` verb carries the same stop through its own `--continue`: a clean stop put down moves no row of the
    // file list, so leaving it is the way out that nothing on that side ever announces (`RepoPage.leaveWipWhenDone`),
    // and the reader is left on an empty face that photographs like any other.
    SampleTimer {
        id: planEditStopTimer
        onTriggered: {
            if (repoTab.busyCount !== 0 || !page.wipShown || !workTree.opEditing)
                return
            planEditStopTimer.stop()
            AppBackend.report("edit_stop editing=" + workTree.opEditing
                              + " skipfree=" + workTree.opSkipFree
                              + " oid=" + (workTree.opEditOid !== "")
                              + " cont=" + wipPane.offersOpExit("--continue")
                              + " skip=" + wipPane.offersOpExit("--skip")
                              + " op=" + workTree.opText)
            if (planOpenTimer.act !== "rebase-edit-stop-out") {
                renderedBarrier.begin()
                return
            }
            AppBackend.report("op_exit_held " + wipPane.completeOpExit("--continue"))
            driver.awaitOpExitLanding()
        }
    }
}
