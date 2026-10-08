pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The gesture that names a graph row: the two clicks at a row, the same pair put in through the strip over the lane
/// column, and the pair (with the double-click that moves beside it) put in at a row of the card a chip unfolds into.
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    // The driver's own names, read once so the verbs can name them bare.
    readonly property var page: driver.page
    readonly property var worktree: driver.worktree
    readonly property var graphModel: driver.graphModel
    readonly property var graphPane: driver.graphPane
    readonly property var refList: driver.refList
    readonly property var rowCard: driver.rowCard

    /// Runs `act` if it is one of this family's, and says whether it was.
    function run(act, arg) {
        if (act === "graph-reclick-lanes") {
            // The argument is the row the presses land on.
            laneClickTimer.row = Number(arg === "" ? "0" : arg)
            laneClickTimer.start()
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
            // The submit is ticks away: armed now, the watch catches the ask whenever it comes
            // (`AutoActDriver.beginWrite`).
            if (act === "graph-rename")
                driver.beginWrite("graph-rename")
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
        onTriggered: {
            const item = graphPane.view.itemAtIndex(reclickGraphTimer.row)
            // Not laid out yet: latching a click here would wait for an answer to a question nobody put.
            if (!item)
                return
            if (acts.reclickGraphStep === 0) {
                // Written where a real pointer writes it, so the row makes every decision after that itself
                // (`GraphRowDelegate.settlePointed`).
                if (reclickGraphTimer.points)
                    item.pointerRowX = item.labelsW - 2
                item.leftClick(Qt.NoModifier)
                acts.reclickGraphStep = 1
            } else if (acts.reclickGraphStep === 1) {
                // Inside the double-click window a second click would be half a double-click.
                if (item.clickGuarded)
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
                if (acts.reclickGraphArmed && graphPane.namingOid === "")
                    return
                // And the box's row sent back into view, a beat after it opens (`RepoPage.startRename`).
                if (reclickGraphTimer.scrolls && !graphPane.rowOnScreen(reclickGraphTimer.row))
                    return
                reclickGraphTimer.stop()
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
            if (!item)
                return
            if (acts.laneClickStep === 0) {
                // The strip is only up while the lanes overflow, so the column is squeezed to its floor first.
                page.setGraphColumns(graphPane.labelW, 0)
                acts.laneClickStep = 1
            } else if (acts.laneClickStep === 1) {
                if (!graphPane.lanePan.visible)
                    return
                // A point in the strip's own frame, over this row — the strip works out which row that is.
                if (!graphPane.lanePan.clickAt(1, item.mapToItem(graphPane, 0, item.height / 2).y))
                    return
                acts.laneClickStep = 2
            } else if (acts.laneClickStep === 2) {
                if (graphPane.view.clickGuarded)
                    return
                if (!graphPane.lanePan.clickAt(1, item.mapToItem(graphPane, 0, item.height / 2).y))
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
                if (!item)
                    return
                if (reclickListTimer.across)
                    item.leftClick(Qt.NoModifier)
                // Hover cannot be injected, so this enters where the row's own rest timer would (`ref-list`).
                graphPane.view.chipExpandRequested(item.oid_hex, reclickListTimer.row,
                                                   item.chipItem.records, item.chipItem)
                acts.reclickListStep = 1
            } else if (acts.reclickListStep === 1) {
                // The card lays its rows out as it is shown; until it is up there is no row to click.
                if (!refList.opened)
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
                if (reclickListTimer.across && refList.rowGuarded(reclickListTimer.card))
                    return
                if (!refList.clickRow(reclickListTimer.card))
                    return
                if (reclickListTimer.across) {
                    // The row's click was the first: one click on the card is already the second.
                    acts.reclickListArmed = refList.rowArmed(reclickListTimer.card)
                    acts.reclickListStep = 3
                    return
                }
                acts.reclickListStep = 2
            } else if (acts.reclickListStep === 2) {
                if (refList.rowGuarded(reclickListTimer.card))
                    return
                if (!refList.clickRow(reclickListTimer.card))
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
}
