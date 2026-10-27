pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The gesture that names a graph row: the two clicks at a row, the same pair put in through the strip over the lane
/// column, the pair with a press elsewhere and a step through the history between them, and the pair (with the
/// double-clicks that move beside it — to a branch, or to the worktree holding one) put in at a row of the card a chip
/// unfolds into.
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    // The driver's own names, read once so the verbs can name them bare.
    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var sidebarPane: driver.sidebarPane
    readonly property var worktree: driver.worktree
    readonly property var graphModel: driver.graphModel
    readonly property var graphPane: driver.graphPane
    readonly property var detailsPane: driver.detailsPane
    readonly property var refList: driver.refList
    readonly property var rowCard: driver.rowCard

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act === "graph-reclick-lanes") {
            // The argument is the row the presses land on.
            laneClickTimer.row = Number(arg === "" ? "0" : arg)
            laneClickTimer.start()
        } else if (act === "graph-reclick-elsewhere") {
            // The argument is the row the presses land on.
            elsewhereClickTimer.row = Number(arg === "" ? "0" : arg)
            elsewhereClickTimer.start()
        } else if (act === "graph-reclick" || act === "graph-rename"
                   || act === "graph-reclick-scrolled" || act === "graph-reclick-mark"
                   || act === "graph-reclick-still") {
            // The argument is `<行>[:<付ける名前>]` (row 0 by default); `graph-rename` carries the gesture to git.
            const renameCut = arg.indexOf(":")
            reclickGraphTimer.row = Number(renameCut < 0 ? (arg === "" ? "0" : arg) : arg.substring(0, renameCut))
            reclickGraphTimer.name = renameCut < 0 ? "" : arg.substring(renameCut + 1)
            reclickGraphTimer.scrolls = act === "graph-reclick-scrolled"
            reclickGraphTimer.marks = act === "graph-reclick-mark"
            reclickGraphTimer.points = act === "graph-reclick-still"
            reclickGraphTimer.fits = false
            // The submit is ticks away: armed now, the watch catches the ask whenever it comes
            // (`AutoActDriver.beginWrite`).
            if (act === "graph-rename")
                driver.beginWrite("graph-rename")
            reclickGraphTimer.start()
        } else if (act === "graph-rename-fit") {
            // The same gesture, stopped at the box with words in it: `<行>[:<列幅>[/<レーン列幅>][:<打つ文>]]`, a
            // column left at its width where its part is empty — the lanes squeezed below what they need raise the
            // strip over them (`GraphLanePan`). Without the last part the box holds the name it opened with; with it
            // empty, the box is rubbed out.
            const fitParts = arg.split(":")
            reclickGraphTimer.row = fitParts[0] === "" ? 0 : Number(fitParts[0])
            if (fitParts.length > 1 && fitParts[1] !== "") {
                const widths = fitParts[1].split("/")
                page.setGraphColumns(widths[0] === "" ? graphPane.labelW : Number(widths[0]),
                                     widths.length > 1 && widths[1] !== "" ? Number(widths[1])
                                                                           : graphPane.graphColWManual)
            }
            reclickGraphTimer.puts = fitParts.length > 2
            reclickGraphTimer.typed = fitParts.slice(2).join(":")
            reclickGraphTimer.name = ""
            reclickGraphTimer.scrolls = false
            reclickGraphTimer.marks = false
            reclickGraphTimer.points = false
            reclickGraphTimer.fits = true
            reclickGraphTimer.start()
        } else if (act === "graph-reclick-list" || act === "graph-reclick-across" || act === "ref-list-pick") {
            // At the card the chip unfolds into; the argument `<行>[:<カードの行>]` defaults to the card's first row.
            const listParts = arg.split(":")
            reclickListTimer.row = listParts[0] === "" ? 0 : Number(listParts[0])
            reclickListTimer.card = listParts.length > 1 ? Number(listParts[1]) : 0
            reclickListTimer.picks = act === "ref-list-pick"
            reclickListTimer.across = act === "graph-reclick-across"
            if (reclickListTimer.picks)
                driver.beginWrite("ref-list-pick")
            reclickListTimer.start()
        } else if (act === "ref-list-held") {
            // The double-click on a card row naming a branch another worktree has out: the tab goes and stands in
            // that worktree, as the graph row's own double-click does. Argument `<行>:<カードの行>`.
            const heldParts = arg.split(":")
            listHeldTimer.row = Number(heldParts[0])
            listHeldTimer.card = heldParts.length > 1 ? Number(heldParts[1]) : 0
            listHeldTimer.start()
        } else {
            return false
        }
        return true
    }
    // Every step waits for its own answer: the row before its click, the card before its rows, and the double-click
    // window before a second click counts as a second.
    property bool reclickGraphArmed: false
    property int reclickGraphStep: 0
    /// When the second click went in, for the report's `wait=` — a diagnosis, judged by nothing.
    property real reclickGraphAt: 0
    SampleTimer {
        id: reclickGraphTimer
        property int row: 0
        /// The name to type once the box is open (`graph-rename`); empty for the runs that stop at the box.
        property string name: ""
        /// Whether the history scrolls away between the second click and the box: the delegate is pooled, the
        /// gesture survives it (`ReclickGesture`), and the box has to be sent back into view.
        property bool scrolls: false
        /// Whether the run ends inside the wait, on the mark the chip wears while it runs.
        property bool marks: false
        /// Whether the pointer rests on the chip first, so a card's rest is running under the wait — which the
        /// gesture has to hold still.
        property bool points: false
        /// Whether the run reports where the box stands and how wide it came out (`graph-rename-fit`), after putting
        /// `typed` in it when `puts` — otherwise the box holds the name it opened with.
        property bool fits: false
        property bool puts: false
        property string typed: ""
        /// What a press at `x` along the row lands on, asked of the areas themselves: the lane strip over the list and
        /// the row's own area both hand it to the row (`GraphLanePan.pressLanded`); past them it is the box's, or
        /// nothing's.
        function landsAt(item, x) {
            const y = item.height / 2
            const strip = graphPane.lanePan
            if (strip.visible && strip.contains(strip.mapFromItem(item, x, y)))
                return "row"
            if (item.pressArea.contains(item.pressArea.mapFromItem(item, x, y)))
                return "row"
            return x >= item.nameBoxX && x < item.nameBoxX + item.nameBoxWidth ? "box" : "none"
        }
        function reportFit(item) {
            // The column's frame; the column stands at the row's left.
            const boxX = item.nameBoxX
            const right = boxX + item.nameBoxWidth
            const chipRight = item.labelsW - Theme.spaceXs
            const reach = item.width - Theme.spaceXxl
            const near = (a, b) => Math.abs(a - b) < 0.5
            // Where the box stands (`GraphRowChips.nameSeat`): ending where the chip ended with its head inside the
            // column, the column's room exactly, its head at the column's and past the column, or stopped at the
            // reach.
            const stands = near(boxX, Theme.spaceXs) && near(right, reach) ? "capped"
                         : near(right, chipRight) && boxX > Theme.spaceXs + 0.5 ? "right"
                         : near(right, chipRight) && near(boxX, Theme.spaceXs) ? "full"
                         : near(boxX, Theme.spaceXs) && right > chipRight && right < reach ? "head" : "other"
            // Three presses along the row, asked of what takes them: left of the box, on it — on its part under the
            // lane strip where the strip stands over it — and past it.
            const strip = graphPane.lanePan
            const underL = strip.visible ? Math.max(boxX, strip.mapToItem(item, 0, 0).x) : 0
            const underR = strip.visible ? Math.min(right, strip.mapToItem(item, strip.width, 0).x) : 0
            const onBox = underR - underL > 2 ? (underL + underR) / 2 : boxX + item.nameBoxWidth / 2
            const probes = [boxX / 2, onBox, (right + item.width) / 2]
            Harness.report(
            "graph_rename_fit row=" + reclickGraphTimer.row
            + " armed=" + acts.reclickGraphArmed
            + " box=" + (graphPane.namingOid !== "")
            + " mode=" + graphPane.namingMode
            + " hits=" + probes.map(x => reclickGraphTimer.landsAt(item, x)).join("/")
            // What a rest beside the box opens: nothing, the column being the box's (`GraphRowDelegate.partAt`).
            + " rest_left=" + (item.partAt(probes[0]) || "none")
            + " stands=" + stands
            + " selected=" + item.nameBoxSelected
            + " strip=" + graphPane.lanePan.visible
            + " typed=" + graphPane.namingText
            + " opened=" + graphPane.namingOpenedWith
            + " label_w=" + Math.round(item.labelsW)
            + " box_x=" + Math.round(boxX)
            + " box_w=" + Math.round(item.nameBoxWidth)
            + " right=" + Math.round(right)
            + " chip_right=" + Math.round(chipRight)
            + " reach=" + Math.round(reach))
            driver.complete()
        }
        onTriggered: {
            const item = graphPane.view.itemAtIndex(reclickGraphTimer.row)
            // Each step asks for the row itself: not laid out yet, a click would wait for an answer to a question
            // nobody put, and the scroll pools the row until the box's return rebuilds it.
            if (acts.reclickGraphStep === 0) {
                if (!Awaited.all("reclick_graph_row", { "row": !!item }))
                    return
                // Written where a real pointer writes it, so the row makes every decision after that itself
                // (`GraphRowDelegate.settlePointed`).
                if (reclickGraphTimer.points)
                    item.pointerRowX = item.labelsW - 2
                item.leftClick(Qt.NoModifier)
                acts.reclickGraphStep = 1
            } else if (acts.reclickGraphStep === 1) {
                // Inside the double-click window a second click would be half a double-click; and the details the
                // first click asked for land first, or `-mark` completes over that read.
                if (!Awaited.all("reclick_graph_second", { "row": !!item,
                                                           "window": !!item && !item.clickGuarded,
                                                           "details": page.pageDetailsSettled }))
                    return
                // waits(measured): the origin of the `wait=` below, which the report prints and nothing here reads
                acts.reclickGraphAt = Date.now()
                item.leftClick(Qt.NoModifier)
                // Read where it is set: the wait is short and the box is what it turns into.
                acts.reclickGraphArmed = item.renameArmed
                // Far enough that the clicked row is pooled, well outside the view's buffer.
                if (reclickGraphTimer.scrolls)
                    graphPane.view.positionViewAtIndex(reclickGraphTimer.row + 200, ListView.Beginning)
                if (reclickGraphTimer.marks) {
                    // Ends inside the wait; `mark=` is read off the chip (`RefChip.waiting`), since a picture a
                    // beat late frames the box instead.
                    reclickGraphTimer.stop()
                    Harness.report(
                    "graph_reclick_mark row=" + reclickGraphTimer.row
                    + " armed=" + acts.reclickGraphArmed
                    + " mark=" + item.chipWaiting
                    + " box=" + (graphPane.namingOid !== "")
                    + " window=" + Application.styleHints.mouseDoubleClickInterval)
                    driver.complete()
                    return
                }
                acts.reclickGraphStep = 2
            } else if (acts.reclickGraphStep === 2) {
                // The box open where one was armed, and its row sent back into view, a beat after it opens
                // (`RepoPage.startRename`).
                if (!Awaited.all("reclick_graph_box", {
                    "row": !!item,
                    "box": !acts.reclickGraphArmed || graphPane.namingOid !== "",
                    "shown": !reclickGraphTimer.scrolls || graphPane.rowOnScreen(reclickGraphTimer.row) }))
                    return
                reclickGraphTimer.stop()
                if (reclickGraphTimer.fits) {
                    if (reclickGraphTimer.puts) {
                        // In the way a recycled delegate puts it back (`GraphRowDelegate.takeNamingFocus`), so the box
                        // on screen holds what the run says.
                        graphPane.view.namingText = reclickGraphTimer.typed
                        item.takeNamingFocus()
                    }
                    reclickGraphTimer.reportFit(item)
                    return
                }
                Harness.report(
                "graph_reclick row=" + reclickGraphTimer.row
                + " armed=" + acts.reclickGraphArmed
                + " box=" + (graphPane.namingOid !== "")
                // Nothing hover opened or closed under the wait (`GraphList.renameWaiting`).
                + " list=" + refList.opened
                + " card=" + rowCard.opened
                + " mode=" + graphPane.namingMode
                + " kind=" + graphPane.namingKind
                + " typed=" + graphPane.namingText
                // The tree did not move — the gesture was not a double-click, which a picture cannot say.
                + " branch=" + worktree.branch
                // The scrolled run's whole claim.
                + " shown=" + graphPane.rowOnScreen(reclickGraphTimer.row)
                // Includes the sampler's beat, so read as "about the window".
                // waits(measured): printed beside the window it is read against, and judged by nothing
                + " wait=" + (Date.now() - acts.reclickGraphAt)
                + " window=" + Application.styleHints.mouseDoubleClickInterval)
                // The hand leaves the chip it rested on: the wait over, the row's card would come up on its rest
                // after the report, and whether the picture and the census caught it would be the machine's pace.
                if (reclickGraphTimer.points)
                    item.pointerRowX = -1
                if (reclickGraphTimer.name === "") {
                    driver.complete()
                    return
                }
                // Through to git, by the path the field's own Enter takes. The write barrier finishes this one.
                graphPane.view.namingSubmitted(graphModel.oidAt(reclickGraphTimer.row),
                                               reclickGraphTimer.name, graphPane.namingMode)
                driver.inputWent(true)
            }
        }
    }
    // PGG_AUTO_ACT=graph-reclick-lanes: the same gesture through the strip over the lane column, which takes presses
    // of its own wherever the lanes overflow their column (`GraphLanePan`).
    property bool laneClickArmed: false
    property int laneClickStep: 0
    SampleTimer {
        id: laneClickTimer
        property int row: 0
        onTriggered: {
            const item = graphPane.view.itemAtIndex(laneClickTimer.row)
            // A point in the strip's own frame, over this row — the strip works out which row that is.
            const strikes = () => graphPane.lanePan.clickAt(1, item.mapToItem(graphPane, 0, item.height / 2).y)
            if (acts.laneClickStep === 0) {
                if (!Awaited.all("lane_click_row", { "row": !!item }))
                    return
                // The strip is only up while the lanes overflow, so the column is squeezed to its floor first.
                page.setGraphColumns(graphPane.labelW, 0)
                acts.laneClickStep = 1
            } else if (acts.laneClickStep === 1) {
                const strip = !!item && graphPane.lanePan.visible
                if (!Awaited.all("lane_click_first", { "strip": strip, "struck": strip && strikes() }))
                    return
                acts.laneClickStep = 2
            } else if (acts.laneClickStep === 2) {
                // Inside the double-click window a second click would be half a double-click.
                const open = !!item && !graphPane.view.clickGuarded
                if (!Awaited.all("lane_click_second", { "window": open, "struck": open && strikes() }))
                    return
                acts.laneClickArmed = item.renameArmed
                acts.laneClickStep = 3
            } else if (acts.laneClickStep === 3) {
                if (acts.laneClickArmed && graphPane.namingOid === "")
                    return
                laneClickTimer.stop()
                Harness.report(
                "graph_reclick_lanes row=" + laneClickTimer.row
                + " strip=" + graphPane.lanePan.visible
                + " armed=" + acts.laneClickArmed
                + " box=" + (graphPane.namingOid !== "")
                + " typed=" + graphPane.namingText
                + " branch=" + worktree.branch)
                driver.complete()
            }
        }
    }
    // PGG_AUTO_ACT=graph-reclick-elsewhere: the two clicks with something between them, twice — a press on the details
    // pane (`pane=`), then a step down the history and back up (`step=`) — each time the click back is a first one;
    // the next one names (`armed=` / `box=`; デザイン規約 §左メニューの所作「間を空けた 2 回目」). Every press goes in as
    // a hand's does, the window's watcher first (`FocusRelease.pressedAt`), and the steps where the arrows enter
    // (`GraphPane.stepRow`).
    property bool elsewherePane: false
    property bool elsewhereStepped: false
    property bool elsewhereArmed: false
    property int elsewhereStep: 0
    SampleTimer {
        id: elsewhereClickTimer
        property int row: 0
        function press(item) {
            page.Window.window.caretHand.pressedAt(item.mapToItem(null, 2, item.height / 2))
            item.leftClick(Qt.NoModifier)
        }
        onTriggered: {
            const item = graphPane.view.itemAtIndex(elsewhereClickTimer.row)
            if (!Awaited.all("elsewhere_click_ready", { "ready": item }))
                return
            const step = acts.elsewhereStep
            // Spaced from the click before: inside its window the next press would be half a double-click.
            if ((step === 1 || step === 3 || step === 6) && item.clickGuarded)
                return
            if (step === 0) {
                elsewhereClickTimer.press(item)
            } else if (step === 1) {
                page.Window.window.caretHand.pressedAt(detailsPane.mapToItem(null, detailsPane.width / 2,
                                                                             detailsPane.height / 2))
            } else if (step === 2) {
                // A beat after: the gesture settles an input once it is through (`ReclickGesture.settle`).
                elsewhereClickTimer.press(item)
                acts.elsewherePane = item.renameArmed
            } else if (step === 3) {
                if (!graphPane.stepRow(1))
                    return
            } else if (step === 4) {
                if (!graphPane.stepRow(-1))
                    return
            } else if (step === 5) {
                // Back on the row once the step is read (`GraphRowWalk.noteStep`): read later, it would land after
                // the click and come between this click and the next.
                if (graphPane.stepSettling)
                    return
                elsewhereClickTimer.press(item)
                acts.elsewhereStepped = item.renameArmed
            } else if (step === 6) {
                elsewhereClickTimer.press(item)
                acts.elsewhereArmed = item.renameArmed
            } else {
                if (acts.elsewhereArmed && graphPane.namingOid === "")
                    return
                elsewhereClickTimer.stop()
                Harness.report(
                "graph_reclick_elsewhere row=" + elsewhereClickTimer.row
                + " pane=" + acts.elsewherePane
                + " step=" + acts.elsewhereStepped
                + " armed=" + acts.elsewhereArmed
                + " box=" + (graphPane.namingOid !== "")
                + " typed=" + graphPane.namingText
                + " branch=" + worktree.branch)
                driver.complete()
                return
            }
            acts.elsewhereStep = step + 1
        }
    }
    property bool reclickListArmed: false
    property int reclickListStep: 0
    SampleTimer {
        id: reclickListTimer
        property int row: 0
        property int card: 0
        /// Whether this run is the double-click that moves or the two clicks that name.
        property bool picks: false
        /// Whether the first click goes to the row and the second to the card its chip opens — what a reader does
        /// unawares, since the card comes up on the chip's own seat after a rest.
        property bool across: false
        onTriggered: {
            if (acts.reclickListStep === 0) {
                const item = graphPane.view.itemAtIndex(reclickListTimer.row)
                if (!Awaited.all("reclick_list_row", { "row": item !== null }))
                    return
                if (reclickListTimer.across)
                    item.leftClick(Qt.NoModifier)
                // Hover cannot be injected, so this enters where the row's own rest timer would (`ref-list`).
                graphPane.view.chipExpandRequested(item.oid_hex, reclickListTimer.row,
                                                   item.chipItem.records, item.chipItem)
                acts.reclickListStep = 1
            } else if (acts.reclickListStep === 1) {
                // The card lays its rows out as it is shown; until it is up there is no row to click.
                if (!Awaited.all("reclick_list_open", { "list": refList.opened }))
                    return
                if (reclickListTimer.picks) {
                    if (!driver.inputWent(refList.doubleClickRow(reclickListTimer.card)))
                        return
                    reclickListTimer.stop()
                    Harness.report("ref_list_pick row=" + reclickListTimer.row
                                      + " card=" + reclickListTimer.card
                                      + " list=" + refList.opened)
                    // The write barrier finishes this one.
                    return
                }
                // Inside the window the row's click opened, a click here is the other half of a double-click.
                const open = !reclickListTimer.across || !refList.rowGuarded(reclickListTimer.card)
                if (!Awaited.all("reclick_list_first", { "window": open,
                                                         "clicked": open && refList.clickRow(reclickListTimer.card) }))
                    return
                if (reclickListTimer.across) {
                    // The row's click was the first: one click on the card is already the second.
                    acts.reclickListArmed = refList.rowArmed(reclickListTimer.card)
                    acts.reclickListStep = 3
                    return
                }
                acts.reclickListStep = 2
            } else if (acts.reclickListStep === 2) {
                const open = !refList.rowGuarded(reclickListTimer.card)
                if (!Awaited.all("reclick_list_second", { "window": open,
                                                          "clicked": open && refList.clickRow(reclickListTimer.card) }))
                    return
                acts.reclickListArmed = refList.rowArmed(reclickListTimer.card)
                acts.reclickListStep = 3
            } else if (acts.reclickListStep === 3) {
                if (acts.reclickListArmed && graphPane.namingOid === "")
                    return
                reclickListTimer.stop()
                Harness.report(
                "graph_reclick_list row=" + reclickListTimer.row
                + " card=" + reclickListTimer.card
                + " armed=" + acts.reclickListArmed
                + " box=" + (graphPane.namingOid !== "")
                // The card came down for the box — it stood on the column the box opens in.
                + " list=" + refList.opened
                + " mode=" + graphPane.namingMode
                + " kind=" + graphPane.namingKind
                + " typed=" + graphPane.namingText
                + " branch=" + worktree.branch)
                driver.complete()
            }
        }
    }
    // Where that double-click lands: this tab, standing in the worktree holding the branch. **The page stays** — that
    // is what the landing is (`Hub::restand_tab`) — so this very driver reads the answer.
    //
    // **Every end ends the run**, each named on the line: a row that is no held branch (`held=false`, said before any
    // press), a row that turned the double-click away (the card stays up: `list=true`), and a page that took the
    // press and asked for no worktree (`asked=` empty — the whole road from the press to that ask runs inside the
    // press). Waiting only for the worktree, any of them would sit out the ceiling saying nothing about why.
    SampleTimer {
        id: listHeldTimer
        property int row: 0
        property int card: 0
        property int step: 0
        /// Read before the press: after it this page reads that worktree, and would answer with itself.
        property string wanted: ""
        property bool held: false
        /// What the page asked to open on the press (`RepoPage.openRepositoryPathRequested`), latched.
        property string asked: ""
        function report(stood) {
            listHeldTimer.stop()
            Harness.report("ref_list_held stood=" + stood
                              + " list=" + refList.opened
                              + " held=" + listHeldTimer.held
                              + " row=" + listHeldTimer.row
                              + " card=" + listHeldTimer.card
                              + " asked=" + listHeldTimer.asked
                              + " wanted=" + listHeldTimer.wanted
                              + " where=" + repoTab.repoPath)
            driver.complete()
        }
        onTriggered: {
            if (listHeldTimer.step === 0) {
                const item = graphPane.view.itemAtIndex(listHeldTimer.row)
                if (!Awaited.all("ref_list_held_row", { "row": item !== null }))
                    return
                // Hover cannot be injected, so this enters where the row's own rest timer would (`ref-list`).
                graphPane.view.chipExpandRequested(item.oid_hex, listHeldTimer.row, item.chipItem.records,
                                                   item.chipItem)
                listHeldTimer.step = 1
            } else if (listHeldTimer.step === 1) {
                if (!Awaited.all("ref_list_held_open", { "list": refList.opened }))
                    return
                // The card is up, so its records are this run's answer: a row that is no held branch never will be.
                const rec = refList.records[listHeldTimer.card]
                listHeldTimer.held = rec !== undefined && rec.kind === "branch" && rec.held
                if (!listHeldTimer.held) {
                    listHeldTimer.report(false)
                    return
                }
                // The chip's mark rides the ref joins; the listing the page asks may land a tick apart.
                listHeldTimer.wanted = sidebarPane.worktreesModel.worktreeHolding(rec.name)
                const named = listHeldTimer.wanted !== ""
                if (!Awaited.all("ref_list_held_press", {
                    "wanted": named, "pressed": named && refList.doubleClickRow(listHeldTimer.card) }))
                    return
                listHeldTimer.step = 2
            } else {
                Awaited.at("ref_list_held", "landing")
                const stood = repoTab.repoPath.toLowerCase() === listHeldTimer.wanted.toLowerCase()
                if (!refList.opened && listHeldTimer.asked !== "" && !(stood && PageSettled.settled(page)))
                    return
                listHeldTimer.report(stood)
            }
        }
    }
    Connections {
        target: listHeldTimer.running ? page : null
        function onOpenRepositoryPathRequested(path) {
            listHeldTimer.asked = path
        }
    }
}
