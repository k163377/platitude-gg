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
        } else {
            return false
        }
        return true
    }
    // ---- the face the press puts up before any of that -----------------
    // The pane in the graph's seat with nothing in it but the mode's one word, Cancel and the turning mark. The face
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
                planHeldTimer.begin()
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
    // The right pane's other two doors, while a plan stands. Neither is a write, so neither is what the mode's freeze
    // is for — but both reach past the pane, and this is the one screen where reaching past it lands somewhere the
    // reader cannot see (規約 §フル interactive rebase「右の詳細ペインがそのまま生きる」, and how far that goes).
    //
    // Three presses and a Cancel: the parent hash of a commit the plan holds — a walk *down* the plan, which has to
    // take the model's own row with it; the base's hash — a walk *out* of it, which has no row on this screen to land
    // on; and a file row, which used to read a diff into the pane the plan is standing on.
    //
    // **Not one of the four is in the picture.** The row highlight reads `page.selectedOid`, so a model left behind
    // photographs exactly like one that followed; a diff opened under the plan is drawn nowhere at all; and where the
    // middle lands after Cancel is the whole of the last claim.
    SampleTimer {
        id: planHeldTimer
        /// Where in the walk this run is — each step entered off the state the step before it asked for
        /// (app-ui.md §UI 自動化の因果性).
        property string stage: ""
        /// The commit the in-range hash pointed at, kept so the steps after it can say the selection reached it.
        property string wantOid: ""
        /// What each press was caught doing, read where it happened. Read back at the end they would every one of
        /// them be the *cancelled* screen's. The resting values are the failing ones, so a walk that stops early
        /// cannot report a pass it never reached.
        property int sawRow: -1
        property bool sawOid: false
        property bool sawOutside: false
        property bool sawDiff: true
        function begin() {
            planHeldTimer.stage = "parent"
            planHeldTimer.start()
        }
        /// The pane is showing the page's own selection and has finished being told so: the parent hash it draws and
        /// the file rows under it are both this commit's.
        readonly property bool paneSettled: !detailsModel.loading && page.selectedOid !== ""
                                            && detailsModel.shaHex === page.selectedOid
        /// The first file row the list has actually built, or null — `itemAtIndex` answers null until it has, and a
        /// press aimed at nothing latches a wait that never ends. Folder rows answer to no walk name, and every
        /// commit of the `plan` preset puts its one file under one.
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
                // The plan opened on its newest row and the page is on that commit; the hash beneath its own is the
                // parent, which the plan holds as the row below it.
                if (!planHeldTimer.paneSettled || detailsModel.parentHex === "")
                    return
                planHeldTimer.wantOid = detailsModel.parentHex
                page.jumpToRef(detailsModel.parentHex)
                planHeldTimer.stage = "walked"
                return
            }
            if (planHeldTimer.stage === "walked") {
                // Both halves of the walk down, waited for rather than read straight back: the page's half goes the
                // long way round through `activateRow` and the details it asks for.
                if (page.selectedOid !== planHeldTimer.wantOid || !planHeldTimer.paneSettled)
                    return
                planHeldTimer.sawRow = plan.selectedRow
                planHeldTimer.sawOid = plan.selectedOid === page.selectedOid
                // And the walk out. The base is the row after the oldest, which is to say no row of the plan at all —
                // refused where it stands, so what it did is readable in the same beat.
                page.jumpToRef(plan.ontoOid)
                planHeldTimer.sawOutside = page.selectedOid === planHeldTimer.wantOid
                planHeldTimer.stage = "file"
                return
            }
            if (planHeldTimer.stage === "file") {
                const row = planHeldTimer.fileRow()
                if (!row || !planHeldTimer.paneSettled)
                    return
                // The row's own press rather than the pane's signal: the door being held is at the far end of it
                // (`RepoPage.openDiff`), and the run has to arrive through everything in between. `diffShown` is
                // written inside that call, so an unheld door is already open by the next line.
                row.activated("", row.pathText, row.origPathText)
                planHeldTimer.sawDiff = page.diffShown
                plan.cancelPlan()
                planHeldTimer.stage = "gone"
                return
            }
            if (plan.active || page.planShown)
                return
            planHeldTimer.stop()
            AppBackend.report("plan_details_held row=" + planHeldTimer.sawRow
                              + " oid=" + planHeldTimer.sawOid
                              + " outside=" + planHeldTimer.sawOutside
                              + " diff=" + planHeldTimer.sawDiff
                              + " centre=" + (page.planShown ? "plan" : page.diffShown ? "diff" : "graph"))
            renderedBarrier.begin()
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
