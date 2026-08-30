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
    readonly property var detailsPane: driver.detailsPane
    readonly property var detailsModel: driver.detailsModel
    readonly property var commitMenu: driver.commitMenu
    readonly property var renderedBarrier: driver.barrierRendered

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — no verb is named by two of them (`AutoActDriver`).
    function run(act, arg) {
        if (act === "rebase-plan" || act === "rebase-plan-run" || act === "rebase-edit-stop"
            || act === "rebase-edit-stop-out" || act === "plan-reword-verb"
            || act === "plan-reword-out" || act === "plan-reword-ask") {
            // Exercise the menu entry and its handler, then dismiss the menu as the actual click does.
            // Each verb selects the smallest plan its result needs — the three about the right pane's boxes need
            // only that the newest commit be a row of it, since that is the row the plan opens the selection on.
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
            } else if (planOpenTimer.act.startsWith("plan-reword")) {
                // The newest row is already the selected one, and the plan already put the right pane on its commit
                // (`RepoPage.onPlanActiveChanged`) — all that is missing is the verb that makes the boxes the row's.
                plan.setAction(0, "reword")
                planRewordTimer.begin(planOpenTimer.act)
            } else {
                plan.setAction(plan.stepCount - 1, "edit")
                planEditStopTimer.start()
                plan.runPlan()
            }
        }
    }
    // What the right pane's boxes do while a plan owns them, and after one lets go of them. A `reword` row types into
    // the same two boxes a plain amend does (提案 2026-08-30: メッセージを打つ場所はアプリに 1 つ), so every way the
    // plan lets go has to hand them back — and none of those ways moves the commit on screen, which is the only cue
    // the pane refills on by itself (`DetailsPane.syncMessage`).
    //
    // `plan-reword-verb` walks the row's verb out of `reword` and back into it, with the plan still standing. The
    // other two walk the whole plan away with the typing unsaved, and read what the boxes are left holding; the
    // second of them goes on to type into them again, for the question the closed plan was refusing to ask.
    SampleTimer {
        id: planRewordTimer
        property string act: ""
        /// Where in the walk this run is. Each step is entered off the state the step before it asked for — the
        /// sampler carries only the looking (app-ui.md §UI 自動化の因果性).
        property string stage: ""
        /// What the reword row is given, and what the boxes are asked for once more after the plan is gone: two
        /// texts that are neither the commit's own message nor each other.
        readonly property string typed: "planned"
        readonly property string retyped: "retyped"
        /// Whether the boxes are resting on the commit's own message again. Read off the resting text, since the
        /// `dirty` in the same report is what says whether the boxes themselves have moved off it.
        readonly property bool restored: detailsPane.baseSubject === detailsModel.messageSubject
                                         && detailsPane.baseBody === detailsModel.messageBody
        function begin(which) {
            planRewordTimer.act = which
            planRewordTimer.stage = "type"
            planRewordTimer.start()
        }
        onTriggered: {
            const plan = page.rebasePlan
            if (planRewordTimer.stage === "type") {
                // The boxes take typing once the page has routed them at the row carrying the verb
                // (`RepoPage.planReword`), which waits on that commit's own details arriving.
                if (!detailsPane.editable)
                    return
                detailsPane.setMessageText(planRewordTimer.typed, "")
                // Only the round trip needs the plan to be holding the draft. The two that walk away leave it
                // unsaved on purpose: an unsaved reword is the half-written message a closing plan would be
                // handing to the amend it hands the boxes back to.
                if (planRewordTimer.act === "plan-reword-verb") {
                    detailsPane.submitMessage()
                    planRewordTimer.stage = "held"
                    return
                }
                plan.cancelPlan()
                planRewordTimer.stage = "gone"
                return
            }
            if (planRewordTimer.stage === "held") {
                if (plan.selectedMsgSubject !== planRewordTimer.typed)
                    return
                plan.setAction(0, "pick")
                planRewordTimer.stage = "picked"
                return
            }
            if (planRewordTimer.stage === "picked") {
                // `pick` took the draft with it (`RebasePlanModel::set_action`); `reword` again is where a draft
                // left resting in the boxes would read as unchanged and refuse the next save.
                if (plan.selectedAction !== "pick")
                    return
                plan.setAction(0, "reword")
                planRewordTimer.stage = "back"
                return
            }
            if (planRewordTimer.stage === "back") {
                if (!detailsPane.editable)
                    return
                planRewordTimer.stop()
                AppBackend.report("plan_reword_verb verb=" + plan.selectedAction
                                  + " restored=" + planRewordTimer.restored
                                  + " dirty=" + detailsPane.messageDirty
                                  + " editable=" + detailsPane.editable
                                  + " draft=" + (plan.selectedMsgSubject === ""))
                renderedBarrier.begin()
                return
            }
            if (planRewordTimer.stage === "gone") {
                if (plan.active)
                    return
                AppBackend.report("plan_reword_out plan=" + plan.active
                                  + " restored=" + planRewordTimer.restored
                                  + " dirty=" + detailsPane.messageDirty
                                  + " editable=" + detailsPane.editable)
                if (planRewordTimer.act !== "plan-reword-ask") {
                    planRewordTimer.stop()
                    renderedBarrier.begin()
                    return
                }
                // The keystroke that would have asked whether a remote already has this commit was spent while the
                // plan was standing over the slot (`RepoPage.askSelectedPublished`), and the question is asked on
                // the edge rather than on every letter. So type again, and wait for the slot to be carrying this
                // very commit's own answer.
                detailsPane.setMessageText(planRewordTimer.retyped, "")
                planRewordTimer.stage = "asked"
                return
            }
            if (planRewordTimer.stage === "asked") {
                if (repoTab.publishRange !== page.selectedOid + "^!")
                    return
                planRewordTimer.stop()
                AppBackend.report("plan_reword_ask dirty=" + detailsPane.messageDirty
                                  + " asked=" + (repoTab.publishRange === page.selectedOid + "^!")
                                  + " published=" + page.selectedPublished)
                renderedBarrier.begin()
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
