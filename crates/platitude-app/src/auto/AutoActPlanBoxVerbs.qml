pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The runs the plan's dispatcher hands off to: the ones that act on the **right pane** while a plan stands over it,
/// or just after one has let go. Each is a walk of its own rather than a branch, and one of them starts before there
/// is a plan at all — which is why none of them is a step inside `AutoActPlanVerbs.planOpenTimer`.
///
/// Built by `AutoActPlanVerbs`, which starts these by the names below; what they act on hangs off the same driver,
/// read back once here so the verbs can name it bare.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing and is never
// given a size.
Item {
    id: acts

    /// The driver these verbs belong to. `var` because naming its type here would be a circle: it is the file that
    /// builds the family that builds this one.
    required property var driver

    // The driver's own names, read once so the verbs can name them bare.
    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var detailsPane: driver.detailsPane
    readonly property var detailsModel: driver.detailsModel
    readonly property var commitMenu: driver.commitMenu
    readonly property var renderedBarrier: driver.barrierRendered

    // What the dispatcher starts, under names of its own: an alias cannot carry an id's own name.
    readonly property alias boxReword: planRewordTimer
    readonly property alias boxAmend: planAmendTimer
    readonly property alias boxHeld: planHeldTimer

    // What the right pane's boxes do while a plan owns them, and after one lets go of them. A `reword` row types into
    // the same two boxes a plain amend does — there is one place in this app to type a message — so every way the
    // plan lets go has to hand them back, and none of those ways moves the commit on screen, which is the only cue
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
    // The other text a closing plan finds in the boxes: a plain amend the reader had half written before any plan
    // existed. The walk types it, opens a plan **from HEAD's own row** so the selection never moves, and cancels —
    // the one order in which the plan is the only thing that could have taken the text (a right-click further down
    // the graph moves the selection, and walking off an unsaved message drops it either way, `graph-step-dirty`).
    //
    // **The picture cannot answer this.** The boxes hold one message whichever way the run went, and the message
    // they come back to is the one the reader was amending — so kept text and the commit's own text differ by their
    // wording alone, at the size the shot is taken.
    SampleTimer {
        id: planAmendTimer
        /// The commit the boxes are about, taken once: the plan opens on the same row, so the selection is never
        /// asked to move and `DetailsPane.syncMessage` never runs.
        property string tip: ""
        /// Where in the walk this run is. Each step is entered off the state the step before it asked for — the
        /// sampler carries only the looking (app-ui.md §UI 自動化の因果性).
        property string stage: ""
        /// What is typed into the boxes: neither the commit's own message nor anything a plan would put there.
        readonly property string typed: "typed by hand"
        /// Whether the boxes are resting on the commit's own message, which is where the closed plan has to leave
        /// them — that is what makes what is standing in them unsaved rather than already written.
        readonly property bool restored: detailsPane.baseSubject === detailsModel.messageSubject
                                         && detailsPane.baseBody === detailsModel.messageBody
        /// What Cancel was wearing when it was pressed, read while the plan still stands. False is the claim: this
        /// text is not the plan's to take, so the press costs nothing and the button must not say it does
        /// (`RepoPage.planDiscards`). The dressed side is a picture — `plan-fold-carry` stands on a plan with a verb
        /// set and photographs it.
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
                // The keystroke has to have landed before the plan is asked for, or what the run photographs is
                // whether the typing arrived at all.
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
                AppBackend.report("plan_amend_kept kept="
                                  + (detailsPane.boxSubject === planAmendTimer.typed)
                                  + " restored=" + planAmendTimer.restored
                                  + " dirty=" + detailsPane.messageDirty
                                  + " editable=" + detailsPane.editable
                                  + " guard=" + planAmendTimer.guard)
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
}
