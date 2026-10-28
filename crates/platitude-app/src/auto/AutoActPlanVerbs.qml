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
    readonly property var worktree: driver.worktree
    readonly property var graphModel: driver.graphModel
    readonly property var wipPane: driver.wipPane
    readonly property var planPane: driver.planPane
    readonly property var detailsPane: driver.detailsPane
    readonly property var detailsModel: driver.detailsModel
    readonly property var commitMenu: driver.commitMenu

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
                          : graphModel.oidAt(graphModel.rowOf(worktree.headOid) + back)
            page.openRowMenu(fromOid)
            page.startRebasePlan(fromOid)
            commitMenu.close()
            if (act === "plan-loading") {
                planLoadingTimer.start()
            } else {
                // What the open does once the plan stands: the overview, the reword walks, the carry and Escape dress
                // it; the runs run it; the details walk hands on with nothing done.
                planOpenTimer.act = act
                planOpenTimer.order = act === "rebase-plan" || act.startsWith("plan-reword") ? ["dressed"]
                                    : act === "plan-fold-carry" ? ["carried"]
                                    : act === "plan-escape" || act === "plan-escape-held" ? ["escaped"]
                                    : act === "plan-details-held" ? []
                                    : ["ran"]
                planOpenTimer.start()
            }
        } else if (act === "plan-off-branch") {
            // The same press as `rebase-plan`, on a history the preview turns down (`PlanRefusal`): no plan comes
            // back, so the notice barrier finishes the run.
            //
            // One run covers the three refusal shapes: they share this road and differ only in the heading, which
            // `tst_reportdress.qml` reads; the shapes are pinned by `integrate_integration/plan.rs` and
            // `rebase_plan::drain_tests`. The row has to be named (`row:0`): the commit no branch of `one-commit`
            // sees is not under the tip.
            const refusedOid = arg !== "" ? driver.autoActOid(arg)
                             : graphModel.oidAt(graphModel.rowOf(worktree.headOid) + 1)
            page.openRowMenu(refusedOid)
            page.startRebasePlan(refusedOid)
            commitMenu.close()
            // `plan=`: a window that opened a plan and put it away frames like one that never opened it.
            driver.noticeSaysPlan()
            driver.barrierNotice.start()
        } else if (act === "plan-amend-kept") {
            // A half-written amend, then a plan opened from HEAD's own row (`planAmendTimer`).
            const tip = worktree.headOid
            page.jumpToRef(tip)
            planAmendTimer.tip = tip
            planAmendTimer.start()
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
    // Completed off the page's own answer. The line carries what an empty pane cannot: whether a read is out, was
    // refused, or was never asked.
    PageStage {
        id: planLoadingTimer
        name: "plan_loading"
        /// What the edge saw: read later they would be the plan's, as the model keeps filling while the face is held.
        property int sawRows: -1
        property bool sawOnto: false
        onTriggered: planLoadingTimer.complete({
            "name": "plan_loading_ready",
            "when": () => ({ "held": page.planLoadHeld, "shown": page.planShown, "inactive": !page.planActive }),
            "report": () => "plan_loading held=true shown=true standing=" + page.planActive
                            + " rows=" + planLoadingTimer.sawRows
                            + " onto=" + planLoadingTimer.sawOnto })
    }
    // The open: rows arrive through the feed, so the verbs wait for the plan to stand; the overview also waits for the
    // publish answer, half of what its picture claims. The carried row's verb half way through the trip
    // (`plan-fold-carry`) and whether Escape was taken (`plan-escape`) are the press's witnesses (`underHand`, `took`).
    PageStage {
        id: planOpenTimer
        name: "plan_open"
        property string act: ""
        onTriggered: {
            const op = planOpenTimer
            const plan = page.rebasePlan
            if (!Awaited.all("plan_open_ready", { "active": plan.active, "loaded": !plan.loading,
                                                  "steps": plan.stepCount > 0,
                                                  "pushed": op.act !== "rebase-plan" || page.planPushed !== 0 }))
                return
            if (op.act === "rebase-plan") {
                // One of each look: a fold and a drop. `selected=` is the model's own (`RebasePlanModel::take`): the
                // row highlight reads `page.selectedOid`, which `activateRow` below sets, so a model that opened with
                // nothing selected photographs the same.
                op.input("dressed", () => {
                    plan.setAction(1, "squash")
                    plan.setAction(2, "drop")
                    page.activateRow(worktree.headOid)
                })
                op.complete({ "when": () => ({}), "report": () => {
                    const dressed = page.rebasePlan
                    return "rebase_plan rows=" + dressed.stepCount + " dirty=" + dressed.dirty
                           + " drops=" + dressed.dropCount + " onto=" + (dressed.ontoRef !== "")
                           + " pushed=" + page.planPushed + " selected=" + dressed.selectedRow
                } })
            } else if (op.act === "rebase-plan-run") {
                // The barrier is the old head's row going and the new one standing, not the dropped row — origin
                // still reaches it (rules-refs/app-ui.md「プラン実行の write barrier」).
                op.input("ran", () => {
                    plan.setAction(plan.stepCount - 1, "drop")
                    planRanTimer.begin(worktree.headOid)
                    // Not through `pressWrite`, as for the edit stop below: the plan's door hands the tab no id.
                    plan.runPlan()
                })
                op.stop()
            } else if (op.act === "plan-details-held") {
                // The walk starts from the screen a reader gets: the opened row selected and the right pane on it
                // (`RepoPage.onPlanActiveChanged`), no verb dressed.
                op.stop()
                planHeldTimer.start()
            } else if (op.act === "plan-escape" || op.act === "plan-escape-held") {
                // Escape against `Discard` (デザイン規約 §フル interactive rebase「`Discard` は 2 状態」): with nothing
                // composed it is the press; with a verb set it would need a hold, so `held=true` (the plan still
                // standing) is the claim. Entered at `escapePressed`, the one line `Keys.onEscapePressed` calls.
                op.input("escaped", () => {
                    if (op.act === "plan-escape-held")
                        plan.setAction(1, "squash")
                    op.witness("took", page.escapePressed())
                })
                op.complete({ "when": () => ({}),
                              "report": () => "plan_escape discards=" + page.planDiscards
                                              + " took=" + op.witnessed("took")
                                              + " held=" + page.planShown
                                              + " dirty=" + page.rebasePlan.dirty })
            } else if (op.act === "plan-fold-carry") {
                // A fold carried down over the oldest place (where no fold can stand) and back, through the functions
                // the row's `MouseArea` calls (`RebasePlanRow`), so the fold rule's two brackets are under test. Moves
                // answer inside the call, so the trip is one turn — a demotion on the way would escape a sampler too.
                // The carried row is selected first so `selectedAction` is its verb throughout (the reorder remaps
                // the selection); a chip redrawn as `pick` photographs like a row that was never a fold.
                op.input("carried", () => {
                    const carried = plan.stepCount - 2
                    plan.setAction(carried, "squash")
                    plan.selectRow(carried)
                    planPane.view.moveBegan()
                    planPane.view.moveRequested(carried, carried + 1)
                    op.witness("underHand", plan.selectedAction)
                    planPane.view.moveRequested(carried + 1, carried)
                    planPane.view.moveEnded()
                })
                op.complete({ "when": () => ({}), "report": () => {
                    const carried = page.rebasePlan
                    return "plan_fold_carry carried=" + op.witnessed("underHand")
                           + " landed=" + carried.selectedAction
                           + " row=" + carried.selectedRow + " rows=" + carried.stepCount
                           + " dirty=" + carried.dirty
                } })
            } else if (op.act.startsWith("plan-reword")) {
                // The newest row is already selected with the right pane on it; the verb makes the boxes the row's.
                op.input("dressed", () => {
                    plan.setAction(0, "reword")
                })
                op.stop()
                planRewordTimer.start()
            } else {
                // Not through `pressWrite`: the plan's door hands the tab no id to hold, so the watch would stand
                // pressed for good and `-out`'s own press would arm over it. The stop is waited out by name instead.
                op.input("ran", () => {
                    plan.setAction(plan.stepCount - 1, "edit")
                    planEditStopTimer.start()
                    plan.runPlan()
                })
                op.stop()
            }
        }
    }
    // The run's landing: the plan away, the write answered by name, the old head's row gone and the new one drawn,
    // and the status the write asked for in (`statusFrom`, the report number its answer named — a status read while
    // the rebase was out stands the worktree row up for the operation, and a picture taken on the answer alone is of
    // that row, which the status after the write takes away). `wip=` is the row at the picture, so the claim is
    // read, not inferred from what stood around it.
    PageStage {
        id: planRanTimer
        name: "rebase_plan_ran"
        property string headBefore: ""
        /// The rebase's own answer, set once by name (the `Connections` below) with its stop and failure alongside.
        property string answeredOp: ""
        property bool answeredStopped: false
        property bool answeredError: false
        /// The report of HEAD that can carry the status behind the answer (`RepoTab.writeAnswerHeadSeq`), 0 until
        /// the answer names one.
        property int statusFrom: 0
        function begin(oid) {
            planRanTimer.headBefore = oid
            planRanTimer.start()
        }
        function landed() {
            return {
                "answered": planRanTimer.answeredOp !== "",
                "idle": repoTab.busyCount === 0,
                "closed": !page.rebasePlan.active,
                "moved": worktree.headOid !== planRanTimer.headBefore && worktree.headOid !== "",
                "drawn": graphModel.rowOf(worktree.headOid) >= 0,
                "gone": graphModel.rowOf(planRanTimer.headBefore) < 0,
                "status": planRanTimer.statusFrom !== 0 && worktree.statusSeq >= planRanTimer.statusFrom,
                "settled": PageSettled.settled(page)
            }
        }
        onTriggered: {
            Awaited.claim("rebase_plan_ran", () => !graphModel.wipRow && graphModel.rowOf(worktree.headOid) >= 0)
            planRanTimer.complete({
                "name": "rebase_plan_ran",
                "when": () => planRanTimer.landed(),
                "report": () => "rebase_plan_ran op=" + planRanTimer.answeredOp
                                + " moved=" + (worktree.headOid !== planRanTimer.headBefore)
                                + " gone=" + (graphModel.rowOf(planRanTimer.headBefore) < 0)
                                + " stopped=" + planRanTimer.answeredStopped
                                + " plan=" + page.rebasePlan.active
                                + " error=" + planRanTimer.answeredError
                                + " wip=" + graphModel.wipRow })
        }
    }
    /// Read out of the answers the notify carried (as `tipLandedTimer.answeredOp` is): one drain empties the queue and
    /// notifies once (`RepoTab::write_answers`), and the fetch the freeze leaves running can leave the answer group
    /// describing itself. By name, not by id: the plan's write leaves by the plan's own door, which hands the tab no id
    /// (`RebasePlanModel::run_plan`), so `wroteAndSettled` never opens on it.
    Connections {
        target: driver.repoTab
        function onWriteSeqChanged() {
            const tab = driver.repoTab
            for (let i = 0; i < tab.writeAnswerCount(); i++) {
                if (tab.writeAnswerSeq(i) <= driver.writeSeqBefore || tab.writeAnswerOp(i) !== "rebase")
                    continue
                if (planRanTimer.running && planRanTimer.answeredOp === "") {
                    planRanTimer.answeredOp = tab.writeAnswerOp(i)
                    planRanTimer.answeredStopped = tab.writeAnswerStopped(i)
                    planRanTimer.answeredError = tab.writeAnswerFailed(i)
                    planRanTimer.statusFrom = tab.writeAnswerHeadSeq(i)
                }
                if (planEditStopTimer.running && planEditStopTimer.listingFrom < 0)
                    planEditStopTimer.listingFrom = tab.writeAnswerReadsFrom(i)
                return
            }
        }
    }
    // The asked-for stop, on a clean tree: the report carries what the picture cannot — git's edit marker, the skip's
    // cost and the exit card's rows (デザイン規約 §進行中の操作から出る / P3-確認事項「クリーン停止の WIP 行は語が場面に合っていない」).
    // `-out` continues it: a clean stop moves no file row, so nothing on that side announces the way out
    // (`RepoPage.leaveWipWhenDone`).
    //
    // Also waited out: the WORKTREES listing read after the write, and the walk behind it. The listing is a read of
    // its own, landing apart from the status that raises the edit marker (`session::write::settle_after`), so a shot
    // on the status alone can name the main worktree by the branch the stop left; the walk comes after both, and draws
    // the worktree row the standing operation holds open (`PageSettled`).
    PageStage {
        id: planEditStopTimer
        name: "edit_stop"
        order: Harness.autoAct === "rebase-edit-stop-out" ? ["stopped", "continued"] : []
        /// The rebase's `reads_from` (`RepoTab.writeAnswerReadsFrom`), -1 until its answer is in: a listing that
        /// looked at or above it saw the stop.
        property int listingFrom: -1
        /// The stop standing: the answer in, nothing running, the edit marker up on the WIP face, the listing behind
        /// the write in, and the page settled.
        function stood() {
            const answered = planEditStopTimer.listingFrom >= 0
            return {
                "answer": answered,
                "idle": repoTab.busyCount === 0,
                "wip": page.wipShown,
                "editing": worktree.opEditing,
                "worktrees": answered && driver.worktreesModel.listingLooked() >= planEditStopTimer.listingFrom,
                "page": PageSettled.settled(page)
            }
        }
        function line() {
            return "edit_stop editing=" + worktree.opEditing
                   + " skipfree=" + worktree.opSkipFree
                   + " oid=" + (worktree.opEditOid !== "")
                   + " cont=" + wipPane.offersOpExit("--continue")
                   + " skip=" + wipPane.offersOpExit("--skip")
                   // The one stepping stop that keeps the message boxes and the commit button: amending is what it
                   // stops for (デザイン規約 §進行中の操作から出る). Read off the drawn items.
                   + " box=" + wipPane.commitBlock.messageSeat.visible
                   + " button=" + wipPane.commitBlock.commitSeat.visible
                   // The stop leaves HEAD on no branch, so the main worktree's WORKTREES row is named by its folder
                   // again (デザイン規約 §左メニューの所作) — once the listing behind the write is in.
                   + " home=" + driver.navProbe.homeWorktreeName()
                   + " op=" + worktree.opText
        }
        onTriggered: {
            const op = planEditStopTimer
            if (planOpenTimer.act !== "rebase-edit-stop-out") {
                op.complete({ "name": "edit_stop", "when": () => op.stood(), "report": () => op.line() })
                return
            }
            if (!Awaited.all("edit_stop", op.stood()))
                return
            // The stop as it stood, said before the continuation takes it down; where that leaves the reader is the
            // landing's (`AutoActDriver.awaitOpExitLanding`). Whether the continuation's press went in is its own
            // answer, kept as it went.
            op.advance("stopped", () => op.line())
            op.input("continued", () => {
                op.witness("exitHeld", driver.pressWrite("op-exit", () => wipPane.completeOpExit("--continue")))
            }, () => "op_exit_held " + op.witnessed("exitHeld"))
            op.stop()
            driver.awaitOpExitLanding()
        }
    }

    // The right pane's boxes under a plan: a `reword` row types into the amend's two boxes, so every way the plan lets
    // go has to hand them back — and none moves the commit on screen, the pane's only own refill cue
    // (`DetailsPane.syncMessage`). `-verb` walks the row's verb out of `reword` and back; `-out` / `-ask` cancel the
    // plan with the typing unsaved, and `-ask` types again for the question the closed plan was refusing to ask.
    PageStage {
        id: planRewordTimer
        name: "plan_reword"
        readonly property string act: Harness.autoAct
        order: planRewordTimer.act === "plan-reword-verb" ? ["typed", "saved", "picked", "back"]
             : planRewordTimer.act === "plan-reword-ask" ? ["typed", "cancelled", "gone", "retyped"]
             : ["typed", "cancelled"]
        /// Typed into the reword row, then into the boxes after the plan is gone — neither the commit's message nor
        /// each other.
        readonly property string typed: "planned"
        readonly property string retyped: "retyped"
        /// Whether the boxes rest on the commit's own message again (`dirty=` says whether the boxes moved off it).
        readonly property bool restored: detailsPane.baseSubject === detailsModel.messageSubject
                                         && detailsPane.baseBody === detailsModel.messageBody
        function outLine() {
            return "plan_reword_out plan=" + page.rebasePlan.active
                   + " restored=" + planRewordTimer.restored
                   + " dirty=" + detailsPane.messageDirty
                   + " editable=" + detailsPane.editable
        }
        onTriggered: {
            const op = planRewordTimer
            const plan = page.rebasePlan
            if (op.at("start")) {
                // The boxes take typing once routed at the verb's row (`RepoPage.planReword`), after its details.
                if (!Awaited.all("plan_reword_editable", { "editable": detailsPane.editable }))
                    return
                op.input("typed", () => {
                    detailsPane.setMessageText(op.typed, "")
                })
                // Only the round trip saves the draft; the other two leave it unsaved on purpose — that is what a
                // closing plan must not hand to the amend.
                if (op.act === "plan-reword-verb") {
                    op.input("saved", () => {
                        detailsPane.submitMessage()
                    })
                    return
                }
                op.input("cancelled", () => {
                    plan.cancelPlan()
                })
                return
            }
            if (op.at("saved")) {
                if (!Awaited.all("plan_reword_held", { "held": plan.selectedMsgSubject === op.typed }))
                    return
                op.input("picked", () => {
                    plan.setAction(0, "pick")
                })
                return
            }
            if (op.at("picked")) {
                // `pick` took the draft with it (`RebasePlanModel::set_action`); `reword` again is where a draft
                // left resting in the boxes would read as unchanged and refuse the next save.
                if (!Awaited.all("plan_reword_picked", { "pick": plan.selectedAction === "pick" }))
                    return
                op.input("back", () => {
                    plan.setAction(0, "reword")
                })
                return
            }
            if (op.at("back")) {
                op.complete({
                    "name": "plan_reword_back",
                    "when": () => ({ "editable": detailsPane.editable }),
                    "report": () => {
                        const back = page.rebasePlan
                        return "plan_reword_verb verb=" + back.selectedAction
                               + " restored=" + op.restored
                               + " dirty=" + detailsPane.messageDirty
                               + " editable=" + detailsPane.editable
                               + " draft=" + (back.selectedMsgSubject === "")
                    } })
                return
            }
            if (op.at("cancelled")) {
                if (op.act !== "plan-reword-ask") {
                    op.complete({ "name": "plan_reword_gone", "when": () => ({ "gone": !page.rebasePlan.active }),
                                  "report": () => op.outLine() })
                    return
                }
                if (!Awaited.all("plan_reword_gone", { "gone": !plan.active }))
                    return
                // Read before the boxes are typed into again. Published-ness is HEAD's own answer
                // (`WorktreeModel.headPublished`), so the plan cannot have spent it; type again and read the warning
                // with the boxes dirty.
                op.advance("gone", () => op.outLine())
                op.input("retyped", () => {
                    detailsPane.setMessageText(op.retyped, "")
                })
                return
            }
            // The picture waits out the rev-list only the retyping asks for.
            op.complete({ "name": "plan_reword_retyped", "when": () => ({ "page": PageSettled.settled(page) }),
                          "report": () => "plan_reword_ask dirty=" + detailsPane.messageDirty
                                          + " onhead=" + (page.selectedOid === worktree.headOid)
                                          + " published=" + page.selectedPublished })
        }
    }
    // A plain amend half written before any plan: typed, then a plan opened from HEAD's own row and cancelled — the one
    // order in which only the plan could have taken the text (a right-click further down moves the selection, and
    // walking off an unsaved message drops it either way, `graph-step-dirty`). Only the report can tell: kept text and
    // the commit's own differ by wording alone.
    PageStage {
        id: planAmendTimer
        name: "plan_amend"
        order: ["typed", "opened", "cancelled"]
        /// The commit the boxes are about; the plan opens on it, so `DetailsPane.syncMessage` never runs.
        property string tip: ""
        /// What is typed into the boxes: neither the commit's own message nor anything a plan would put there.
        readonly property string typed: "typed by hand"
        /// Whether the boxes rest on the commit's own message, as the closed plan must leave them (the text unsaved).
        readonly property bool restored: detailsPane.baseSubject === detailsModel.messageSubject
                                         && detailsPane.baseBody === detailsModel.messageBody
        // What `Discard` wore at the press (`RepoPage.planDiscards`) is the press's witness (`guard`). False is the
        // claim: this text is not the plan's, so the press costs nothing.
        onTriggered: {
            const op = planAmendTimer
            if (op.at("start")) {
                // The plain amend, which only HEAD's own row offers (`offers::message_edit`) and only once that
                // commit's own details are the ones on screen.
                if (!Awaited.all("plan_amend_editable", { "editable": detailsPane.editable }))
                    return
                op.input("typed", () => {
                    detailsPane.setMessageText(op.typed, "")
                })
                return
            }
            if (op.at("typed")) {
                if (!Awaited.all("plan_amend_typed", { "typed": detailsPane.boxSubject === op.typed }))
                    return
                op.input("opened", () => {
                    page.openRowMenu(op.tip)
                    page.startRebasePlan(op.tip)
                    commitMenu.close()
                })
                return
            }
            if (op.at("opened")) {
                if (!Awaited.all("plan_amend_stood", { "standing": page.planActive }))
                    return
                op.input("cancelled", () => {
                    op.witness("guard", page.planDiscards)
                    page.rebasePlan.cancelPlan()
                })
                return
            }
            op.complete({ "name": "plan_amend_gone", "when": () => ({ "gone": !page.planActive }),
                          "report": () => "plan_amend_kept kept="
                                          + (detailsPane.boxSubject === op.typed)
                                          + " restored=" + op.restored
                                          + " dirty=" + detailsPane.messageDirty
                                          + " editable=" + detailsPane.editable
                                          + " guard=" + op.witnessed("guard") })
        }
    }
    // The right pane's other doors while a plan stands. Neither writes, but both reach past the pane to where the
    // reader cannot see (デザイン規約 §フル interactive rebase「右の詳細ペインがそのまま生きる」): the parent hash (a walk
    // down the plan, which must take the model's row along), the base's hash (a walk out, with no row to land on) and
    // a file row (a diff under the plan); then `Discard`. All four are reported — the picture shows none of them.
    PageStage {
        id: planHeldTimer
        name: "plan_details_held"
        order: ["walked", "outside", "pressed", "cancelled"]
        // The witnesses: the commit the in-range hash pointed at (`wantOid`), kept so the stages after it can say the
        // selection reached it, and what each press did (`sawRow`, `sawOid`, `sawOutside`, `sawDiff`), read where it
        // happened — at the end they would be the cancelled screen's.
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
            const op = planHeldTimer
            const plan = page.rebasePlan
            if (op.at("start")) {
                // The parent hash is the plan's next row down.
                if (!Awaited.all("plan_details_parent", { "pane": op.paneSettled,
                                                          "parent": detailsModel.parentHex !== "" }))
                    return
                op.input("walked", () => {
                    page.jumpToRef(op.witness("wantOid", detailsModel.parentHex))
                })
                return
            }
            if (op.at("walked")) {
                // Both halves of the walk down; the page's goes the long way, through `activateRow` and its details.
                if (!Awaited.all("plan_details_walked", { "selected": page.selectedOid === op.witnessed("wantOid"),
                                                          "pane": op.paneSettled }))
                    return
                op.input("outside", () => {
                    op.witness("sawRow", plan.selectedRow)
                    op.witness("sawOid", plan.selectedOid === page.selectedOid)
                    // The walk out: the base is no row of the plan, refused on the spot, so it reads in the same beat.
                    page.jumpToRef(plan.ontoOid)
                    op.witness("sawOutside", page.selectedOid === op.witnessed("wantOid"))
                })
                return
            }
            if (op.at("outside")) {
                const row = op.fileRow()
                if (!Awaited.all("plan_details_file", { "row": !!row, "pane": op.paneSettled }))
                    return
                // The row's own press, arriving at the held door (`RepoPage.openDiff`) the whole way round; `diffShown`
                // is written inside the call, so an unheld door is open by the next line.
                op.input("pressed", () => {
                    row.press()
                    op.witness("sawDiff", page.diffShown)
                })
                op.input("cancelled", () => {
                    plan.cancelPlan()
                })
                return
            }
            op.complete({
                "name": "plan_details_gone",
                "when": () => ({ "gone": !(page.rebasePlan.active || page.planShown) }),
                "report": () => "plan_details_held row=" + op.witnessed("sawRow")
                                + " oid=" + op.witnessed("sawOid")
                                + " outside=" + op.witnessed("sawOutside")
                                + " diff=" + op.witnessed("sawDiff")
                                + " centre=" + (page.planShown ? "plan" : page.diffShown ? "diff" : "graph") })
        }
    }
}
