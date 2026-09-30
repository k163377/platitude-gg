pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The interactive-rebase plan: opening one over a row, dressing its rows with verbs, running it, the stop a run
/// asks for, and what the right pane's boxes do while a plan owns them or just after one lets go.
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

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

    /// Runs `act` if it is this family's and says whether it was; `AutoActDriver` asks each family in turn.
    function run(act, arg) {
        if (act === "rebase-plan" || act === "rebase-plan-run" || act === "rebase-edit-stop"
            || act === "rebase-edit-stop-out" || act === "plan-reword-verb"
            || act === "plan-reword-out" || act === "plan-reword-ask" || act === "plan-loading"
            || act === "plan-details-held" || act === "plan-fold-carry"
            || act === "plan-escape" || act === "plan-escape-held") {
            // The menu entry and its handler, then the menu dismissed as the click does. Each verb opens the smallest
            // plan it needs: the box verbs only the newest commit (the plan selects it), the carry three rows — the
            // fold, the oldest to carry it over, and one above.
            const back = act === "rebase-plan" || act === "plan-escape"
                         || act === "plan-escape-held" ? 3
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
        } else if (act === "plan-off-branch") {
            // The same press as `rebase-plan`, on a history the preview turns down (`PlanRefusal`): no plan comes
            // back, so the notice barrier finishes the run.
            //
            // One run covers the three refusal shapes: they share this road and differ only in the heading, which
            // `tst_reportdress.qml` reads; the shapes are pinned by `integrate_integration/plan.rs` and
            // `rebase_plan::drain_tests`. The row has to be named (`row:0`): the commit no branch of `one-commit`
            // sees is not under the tip.
            const refusedOid = arg !== "" ? driver.autoActOid(arg)
                             : graphModel.oidAt(graphModel.rowOf(workTree.headOid) + 1)
            page.openRowMenu(refusedOid)
            page.startRebasePlan(refusedOid)
            commitMenu.close()
            // `plan=`: a window that opened a plan and put it away frames like one that never opened it.
            driver.barrierNotice.saysPlan = true
            driver.barrierNotice.start()
        } else if (act === "plan-amend-kept") {
            // A half-written amend, then a plan opened from HEAD's own row (`planAmendTimer`).
            const tip = workTree.headOid
            page.jumpToRef(tip)
            planAmendTimer.begin(tip)
        } else {
            return false
        }
        return true
    }
    // ---- the face the press puts up before any of that -----------------
    // The waiting face, held from the edge that raised it (`RepoPage.planLoadHeld`): rows can land while the grab is
    // out (verify-ui implement.md「中間状態は実 edge を latch する」). The edge is caught on the model's own signal — a
    // small range answers before the next tick, so a sampler would miss it.
    Connections {
        target: Harness.autoAct === "plan-loading" ? page.rebasePlan : null
        function onChanged() {
            const plan = page.rebasePlan
            if (page.planLoadHeld || !plan.loading || plan.active)
                return
            planLoadingTimer.sawRows = plan.stepCount
            planLoadingTimer.sawOnto = plan.ontoRef !== "" || plan.ontoOid !== ""
            page.planLoadHeld = true
        }
    }
    // Completed a turn later, off the page's own answer. The line carries what an empty pane cannot: whether a read is
    // out, was refused, or was never asked.
    SampleTimer {
        id: planLoadingTimer
        /// What the edge saw: read later they would be the plan's, as the model keeps filling while the face is held.
        property int sawRows: -1
        property bool sawOnto: false
        onTriggered: {
            if (!page.planLoadHeld || !page.planShown || page.planActive)
                return
            planLoadingTimer.stop()
            Harness.report("plan_loading held=true shown=true standing=" + page.planActive
                              + " rows=" + planLoadingTimer.sawRows
                              + " onto=" + planLoadingTimer.sawOnto)
            renderedBarrier.begin()
        }
    }
    // The open: rows arrive through the feed, so the verbs wait for the plan to stand; the overview also waits for the
    // publish answer, half of what its picture claims.
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
                // One of each look: a fold and a drop. `selected=` is the model's own (`RebasePlanModel::take`): the
                // row highlight reads `page.selectedOid`, which `activateRow` below sets, so a model that opened with
                // nothing selected photographs the same.
                plan.setAction(1, "squash")
                plan.setAction(2, "drop")
                page.activateRow(workTree.headOid)
                Harness.report("rebase_plan rows=" + plan.stepCount + " dirty=" + plan.dirty
                                  + " drops=" + plan.dropCount + " onto=" + (plan.ontoRef !== "")
                                  + " pushed=" + page.planPushed + " selected=" + plan.selectedRow)
                renderedBarrier.begin()
            } else if (planOpenTimer.act === "rebase-plan-run") {
                // The barrier is the old head's row going and the new one standing, not the dropped row — origin
                // still reaches it (rules-refs/app-ui.md「プラン実行の write barrier」).
                plan.setAction(plan.stepCount - 1, "drop")
                planRanTimer.begin(workTree.headOid)
                driver.pressWrite("run-plan", () => {
                    plan.runPlan()
                    return true
                })
            } else if (planOpenTimer.act === "plan-details-held") {
                // The walk starts from the screen a reader gets: the opened row selected and the right pane on it
                // (`RepoPage.onPlanActiveChanged`), no verb dressed.
                planHeldTimer.begin()
            } else if (planOpenTimer.act === "plan-escape"
                       || planOpenTimer.act === "plan-escape-held") {
                // Escape against `Discard` (デザイン規約 §フル interactive rebase「`Discard` は 2 状態」): with nothing
                // composed it is the press; with a verb set it would need a hold, so `held=true` (the plan still
                // standing) is the claim. Entered at `escapePressed`, the one line `Keys.onEscapePressed` calls.
                if (planOpenTimer.act === "plan-escape-held")
                    plan.setAction(1, "squash")
                const took = page.escapePressed()
                Harness.report("plan_escape discards=" + page.planDiscards
                                  + " took=" + took
                                  + " held=" + page.planShown
                                  + " dirty=" + plan.dirty)
                renderedBarrier.begin()
            } else if (planOpenTimer.act === "plan-fold-carry") {
                // A fold carried down over the oldest place (where no fold can stand) and back, through the functions
                // the row's `MouseArea` calls (`RebasePlanRow`), so the fold rule's two brackets are under test. Moves
                // answer inside the call, so the trip is one turn — a demotion on the way would escape a sampler too.
                // The carried row is selected first so `selectedAction` is its verb throughout (the reorder remaps
                // the selection); a chip redrawn as `pick` photographs like a row that was never a fold.
                const carried = plan.stepCount - 2
                plan.setAction(carried, "squash")
                plan.selectRow(carried)
                planPane.view.moveBegan()
                planPane.view.moveRequested(carried, carried + 1)
                const underHand = plan.selectedAction
                planPane.view.moveRequested(carried + 1, carried)
                planPane.view.moveEnded()
                Harness.report("plan_fold_carry carried=" + underHand
                                  + " landed=" + plan.selectedAction
                                  + " row=" + plan.selectedRow + " rows=" + plan.stepCount
                                  + " dirty=" + plan.dirty)
                renderedBarrier.begin()
            } else if (planOpenTimer.act.startsWith("plan-reword")) {
                // The newest row is already selected with the right pane on it; the verb makes the boxes the row's.
                plan.setAction(0, "reword")
                planRewordTimer.begin(planOpenTimer.act)
            } else {
                plan.setAction(plan.stepCount - 1, "edit")
                planEditStopTimer.start()
                plan.runPlan()
            }
        }
    }
    // The run's landing: the plan away, the write answered by name, the old head's row gone and the new one drawn.
    SampleTimer {
        id: planRanTimer
        property string headBefore: ""
        /// The rebase's own answer, set once by name (the `Connections` below) with its stop and failure alongside.
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
            Harness.report("rebase_plan_ran op=" + planRanTimer.answeredOp
                              + " moved=" + (workTree.headOid !== planRanTimer.headBefore)
                              + " gone=" + (graphModel.rowOf(planRanTimer.headBefore) < 0)
                              + " stopped=" + planRanTimer.answeredStopped
                              + " plan=" + page.rebasePlan.active
                              + " error=" + planRanTimer.answeredError)
            renderedBarrier.begin()
        }
    }
    /// Read out of the answers the notify carried (as `tipLandedTimer.answeredOp` is): one drain empties the queue and
    /// notifies once (`RepoTab::write_answers`), and the fetch the freeze leaves running can leave the answer group
    /// describing itself.
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
    // The asked-for stop, on a clean tree: the report carries what the picture cannot — git's edit marker, the skip's
    // cost and the exit card's rows (デザイン規約 §進行中の操作から出る / P3-確認事項「クリーン停止の WIP 行は語が場面に合っていない」).
    // `-out` continues it: a clean stop moves no file row, so nothing on that side announces the way out
    // (`RepoPage.leaveWipWhenDone`).
    SampleTimer {
        id: planEditStopTimer
        onTriggered: {
            if (repoTab.busyCount !== 0 || !page.wipShown || !workTree.opEditing)
                return
            planEditStopTimer.stop()
            Harness.report("edit_stop editing=" + workTree.opEditing
                              + " skipfree=" + workTree.opSkipFree
                              + " oid=" + (workTree.opEditOid !== "")
                              + " cont=" + wipPane.offersOpExit("--continue")
                              + " skip=" + wipPane.offersOpExit("--skip")
                              // The one stepping stop that keeps the message boxes and the commit button: amending
                              // is what it stops for (デザイン規約 §進行中の操作から出る). Read off the drawn items.
                              + " box=" + wipPane.commitBlock.messageSeat.visible
                              + " button=" + wipPane.commitBlock.commitSeat.visible
                              + " op=" + workTree.opText)
            if (planOpenTimer.act !== "rebase-edit-stop-out") {
                renderedBarrier.begin()
                return
            }
            Harness.report("op_exit_held " + wipPane.completeOpExit("--continue"))
            driver.awaitOpExitLanding()
        }
    }

    // The right pane's boxes under a plan: a `reword` row types into the amend's two boxes, so every way the plan lets
    // go has to hand them back — and none moves the commit on screen, the pane's only own refill cue
    // (`DetailsPane.syncMessage`). `-verb` walks the row's verb out of `reword` and back; `-out` / `-ask` cancel the
    // plan with the typing unsaved, and `-ask` types again for the question the closed plan was refusing to ask.
    SampleTimer {
        id: planRewordTimer
        property string act: ""
        property string stage: ""
        /// Typed into the reword row, then into the boxes after the plan is gone — neither the commit's message nor
        /// each other.
        readonly property string typed: "planned"
        readonly property string retyped: "retyped"
        /// Whether the boxes rest on the commit's own message again (`dirty=` says whether the boxes moved off it).
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
                // The boxes take typing once routed at the verb's row (`RepoPage.planReword`), after its details.
                if (!detailsPane.editable)
                    return
                detailsPane.setMessageText(planRewordTimer.typed, "")
                // Only the round trip saves the draft; the other two leave it unsaved on purpose — that is what a
                // closing plan must not hand to the amend.
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
                Harness.report("plan_reword_verb verb=" + plan.selectedAction
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
                Harness.report("plan_reword_out plan=" + plan.active
                                  + " restored=" + planRewordTimer.restored
                                  + " dirty=" + detailsPane.messageDirty
                                  + " editable=" + detailsPane.editable)
                if (planRewordTimer.act !== "plan-reword-ask") {
                    planRewordTimer.stop()
                    renderedBarrier.begin()
                    return
                }
                // Published-ness is HEAD's own answer (`WorkTreeModel.headPublished`), so the plan cannot have spent
                // it; type again and read the warning with the boxes dirty.
                detailsPane.setMessageText(planRewordTimer.retyped, "")
                planRewordTimer.stage = "retyped"
                return
            }
            if (planRewordTimer.stage === "retyped") {
                planRewordTimer.stop()
                Harness.report("plan_reword_ask dirty=" + detailsPane.messageDirty
                                  + " onhead=" + (page.selectedOid === workTree.headOid)
                                  + " published=" + page.selectedPublished)
                renderedBarrier.begin()
            }
        }
    }
    // A plain amend half written before any plan: typed, then a plan opened from HEAD's own row and cancelled — the one
    // order in which only the plan could have taken the text (a right-click further down moves the selection, and
    // walking off an unsaved message drops it either way, `graph-step-dirty`). Only the report can tell: kept text and
    // the commit's own differ by wording alone.
    SampleTimer {
        id: planAmendTimer
        /// The commit the boxes are about; the plan opens on it, so `DetailsPane.syncMessage` never runs.
        property string tip: ""
        property string stage: ""
        /// What is typed into the boxes: neither the commit's own message nor anything a plan would put there.
        readonly property string typed: "typed by hand"
        /// Whether the boxes rest on the commit's own message, as the closed plan must leave them (the text unsaved).
        readonly property bool restored: detailsPane.baseSubject === detailsModel.messageSubject
                                         && detailsPane.baseBody === detailsModel.messageBody
        /// What `Discard` wore at the press (`RepoPage.planDiscards`). False is the claim: this text is not the plan's,
        /// so the press costs nothing.
        property bool guard: false
        function begin(oidHex) {
            planAmendTimer.tip = oidHex
            planAmendTimer.stage = "type"
            planAmendTimer.start()
        }
        onTriggered: {
            if (planAmendTimer.stage === "type") {
                // The plain amend, which only HEAD's own row offers (`offers::message_edit`) and only once that
                // commit's own details are the ones on screen.
                if (!detailsPane.editable)
                    return
                detailsPane.setMessageText(planAmendTimer.typed, "")
                planAmendTimer.stage = "typed"
                return
            }
            if (planAmendTimer.stage === "typed") {
                if (detailsPane.boxSubject !== planAmendTimer.typed)
                    return
                page.openRowMenu(planAmendTimer.tip)
                page.startRebasePlan(planAmendTimer.tip)
                commitMenu.close()
                planAmendTimer.stage = "stood"
                return
            }
            if (planAmendTimer.stage === "stood") {
                if (!page.planActive)
                    return
                planAmendTimer.guard = page.planDiscards
                page.rebasePlan.cancelPlan()
                planAmendTimer.stage = "gone"
                return
            }
            if (planAmendTimer.stage === "gone") {
                if (page.planActive)
                    return
                planAmendTimer.stop()
                Harness.report("plan_amend_kept kept="
                                  + (detailsPane.boxSubject === planAmendTimer.typed)
                                  + " restored=" + planAmendTimer.restored
                                  + " dirty=" + detailsPane.messageDirty
                                  + " editable=" + detailsPane.editable
                                  + " guard=" + planAmendTimer.guard)
                renderedBarrier.begin()
            }
        }
    }
    // The right pane's other doors while a plan stands. Neither writes, but both reach past the pane to where the
    // reader cannot see (デザイン規約 §フル interactive rebase「右の詳細ペインがそのまま生きる」): the parent hash (a walk
    // down the plan, which must take the model's row along), the base's hash (a walk out, with no row to land on) and
    // a file row (a diff under the plan); then `Discard`. All four are reported — the picture shows none of them.
    SampleTimer {
        id: planHeldTimer
        property string stage: ""
        /// The commit the in-range hash pointed at, kept so the steps after it can say the selection reached it.
        property string wantOid: ""
        /// What each press did, read where it happened (at the end they would be the cancelled screen's). The resting
        /// values fail, so a walk that stops early cannot pass.
        property int sawRow: -1
        property bool sawOid: false
        property bool sawOutside: false
        property bool sawDiff: true
        function begin() {
            planHeldTimer.stage = "parent"
            planHeldTimer.start()
        }
        /// The pane has settled on the page's selection, so its parent hash and file rows are this commit's.
        readonly property bool paneSettled: !detailsModel.loading && page.selectedOid !== ""
                                            && detailsModel.shaHex === page.selectedOid
        /// The first built file row, or null — a press aimed at nothing latches a wait that never ends. Folder rows
        /// have no walk name, and each `plan` commit puts its one file under one.
        function fileRow() {
            const view = detailsPane.filesWalk.view
            for (let i = 0; i < view.count; i++) {
                const row = view.itemAtIndex(i)
                if (row && row.walkKey !== "")
                    return row
            }
            return null
        }
        onTriggered: {
            const plan = page.rebasePlan
            if (planHeldTimer.stage === "parent") {
                // The parent hash is the plan's next row down.
                if (!planHeldTimer.paneSettled || detailsModel.parentHex === "")
                    return
                planHeldTimer.wantOid = detailsModel.parentHex
                page.jumpToRef(detailsModel.parentHex)
                planHeldTimer.stage = "walked"
                return
            }
            if (planHeldTimer.stage === "walked") {
                // Both halves of the walk down; the page's goes the long way, through `activateRow` and its details.
                if (page.selectedOid !== planHeldTimer.wantOid || !planHeldTimer.paneSettled)
                    return
                planHeldTimer.sawRow = plan.selectedRow
                planHeldTimer.sawOid = plan.selectedOid === page.selectedOid
                // The walk out: the base is no row of the plan, refused on the spot, so it reads in the same beat.
                page.jumpToRef(plan.ontoOid)
                planHeldTimer.sawOutside = page.selectedOid === planHeldTimer.wantOid
                planHeldTimer.stage = "file"
                return
            }
            if (planHeldTimer.stage === "file") {
                const row = planHeldTimer.fileRow()
                if (!row || !planHeldTimer.paneSettled)
                    return
                // The row's own press, arriving at the held door (`RepoPage.openDiff`) the whole way round; `diffShown`
                // is written inside the call, so an unheld door is open by the next line.
                row.press()
                planHeldTimer.sawDiff = page.diffShown
                plan.cancelPlan()
                planHeldTimer.stage = "gone"
                return
            }
            if (plan.active || page.planShown)
                return
            planHeldTimer.stop()
            Harness.report("plan_details_held row=" + planHeldTimer.sawRow
                              + " oid=" + planHeldTimer.sawOid
                              + " outside=" + planHeldTimer.sawOutside
                              + " diff=" + planHeldTimer.sawDiff
                              + " centre=" + (page.planShown ? "plan" : page.diffShown ? "diff" : "graph"))
            renderedBarrier.begin()
        }
    }
}
