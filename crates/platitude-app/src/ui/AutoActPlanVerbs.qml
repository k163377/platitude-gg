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
    readonly property var commitMenu: driver.commitMenu
    readonly property var renderedBarrier: driver.barrierRendered

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — no verb is named by two of them (`AutoActDriver`).
    function run(act, arg) {
        if (act === "rebase-plan" || act === "rebase-plan-run" || act === "rebase-edit-stop"
            || act === "rebase-edit-stop-out") {
            // Exercise the menu entry and its handler, then dismiss the menu as the actual click does.
            // Each verb selects the smallest plan its result needs.
            const back = act === "rebase-plan" ? 3 : act === "rebase-plan-run" ? 2 : 1
            const fromOid = arg !== "" ? driver.autoActOid(arg)
                          : graphModel.oidAt(graphModel.rowOf(workTree.headOid) + back)
            page.openRowMenu(fromOid)
            page.startRebasePlan(fromOid)
            commitMenu.close()
            planOpenTimer.begin(act)
        } else {
            return false
        }
        return true
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
                // One of each look: a fold leaning into its parent, a drop struck through, and the selection on the
                // newest row so the right pane is that commit's.
                plan.setAction(1, "squash")
                plan.setAction(2, "drop")
                plan.selectRow(0)
                page.activateRow(workTree.headOid)
                AppBackend.report("rebase_plan rows=" + plan.stepCount + " dirty=" + plan.dirty
                                  + " drops=" + plan.dropCount + " onto=" + (plan.ontoRef !== "")
                                  + " pushed=" + page.planPushed)
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
        /// The run's own answer, latched off the notify the way `tipLandedTimer.answeredOp` is — the fetch the
        /// freeze leaves running rewrites `lastWriteOp` with every answer of its own, so a beat that reads the
        /// sampled name can find a fetch's where the rebase's stood (same measured failure). One-way and by name:
        /// a fetch answering *first* must not take the arm, so only the rebase's answer sets it, and what it said
        /// about stopping and failing is taken in the same breath.
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
    Connections {
        target: driver.repoTab
        function onWriteSeqChanged() {
            if (!planRanTimer.running || planRanTimer.answeredOp !== ""
                    || driver.repoTab.writeSeq <= driver.writeSeqBefore
                    || driver.repoTab.lastWriteOp !== "rebase")
                return
            planRanTimer.answeredOp = driver.repoTab.lastWriteOp
            planRanTimer.answeredStopped = driver.repoTab.lastWriteStopped
            planRanTimer.answeredError = driver.repoTab.lastWriteError !== ""
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
