pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The gesture that names a graph row: the two clicks at a row, the same pair put in through the strip over the lane
/// column, and the pair (with the double-click that moves beside it) put in at a row of the card a chip unfolds into.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given. What these verbs act on
/// hangs off that driver; the names it owns are
/// read back once below so the verbs can name them bare.
// An `Item` only because `QtObject` has no default property to hold the timers below; it is a
// sizeless holder.
Item {
    id: acts

    /// The driver these verbs belong to. `var` because naming its type here would be a circle: it is
    /// the file that builds this one.
    required property var driver

    // The driver's own names, read once so the verbs can name them bare.
    readonly property var page: driver.page
    readonly property var workTree: driver.workTree
    readonly property var graphModel: driver.graphModel
    readonly property var graphPane: driver.graphPane
    readonly property var refList: driver.refList
    readonly property var rowCard: driver.rowCard

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — each verb is named by one (`AutoActDriver`).
    function run(act, arg) {
        if (act === "graph-reclick-lanes") {
            // The argument is the row the presses land on.
            laneClickTimer.row = Number(arg === "" ? "0" : arg)
            laneClickTimer.start()
        } else if (act === "graph-reclick" || act === "graph-rename"
                   || act === "graph-reclick-scrolled" || act === "graph-reclick-mark"
                   || act === "graph-reclick-still") {
            // The gesture at the row itself. The argument is `<行>[:<付ける名前>]` — the first row is the default; a
            // run that wants a chip says which (`--preset tags`). `graph-rename` carries the gesture through to git.
            const renameCut = arg.indexOf(":")
            reclickGraphTimer.row = Number(renameCut < 0 ? (arg === "" ? "0" : arg) : arg.substring(0, renameCut))
            reclickGraphTimer.name = renameCut < 0 ? "" : arg.substring(renameCut + 1)
            reclickGraphTimer.scrolls = act === "graph-reclick-scrolled"
            reclickGraphTimer.marks = act === "graph-reclick-mark"
            reclickGraphTimer.points = act === "graph-reclick-still"
            // The submit is ticks away; the watch is armed here and catches the ask whenever it comes
            // (`AutoActDriver.beginWrite`), so what it waits on in between is the input.
            if (act === "graph-rename")
                driver.beginWrite("graph-rename")
            reclickGraphTimer.start()
        } else if (act === "graph-reclick-list" || act === "graph-reclick-across" || act === "ref-list-pick") {
            // The same gesture, and the double-click beside it, put in at the card the chip unfolds into. The
            // argument `<行>[:<カードの行>]` defaults to the card's first row — the one on the chip's own seat.
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
    // PGG_AUTO_ACT=graph-reclick / graph-reclick-list / ref-list-pick: the two clicks of the rename gesture put in at a
    // graph row, and at a row of the card its chip unfolds into — and, on the same card, the double-click that is the
    // way to move. Every step waits for its own answer: the row has to exist before it can be clicked, the card has to
    // be up before one of its rows can be, and the double-click window the first click opened has to have passed
    // before a second one counts as a second (app-ui.md §UI 自動化).
    property bool reclickGraphArmed: false
    property int reclickGraphStep: 0
    /// When the second click went in, so the report can say how long the box made the reader wait. **A diagnosis**
    /// — the wait is the double-click window and the sampler reads through it (app-ui.md §UI 自動化);
    /// it is here because a lag is the one thing about this gesture a picture cannot show.
    property real reclickGraphAt: 0
    SampleTimer {
        id: reclickGraphTimer
        property int row: 0
        /// The name to put in the box once it is open, for the run that carries the gesture through to git
        /// (`graph-rename`); empty for the one that stops at the box.
        property string name: ""
        /// Whether the run scrolls the history away between the second click and the box. **The delegate is pooled by
        /// that scroll**, which the gesture survives (`ReclickGesture`) — and the box that
        /// opens has to be sent back into view.
        property bool scrolls: false
        /// Whether the run ends inside the wait, on the mark the chip wears while it runs.
        property bool marks: false
        /// Whether the pointer is rested on the chip first, so the card it opens has a rest running under the wait —
        /// what the gesture has to hold still.
        property bool points: false
        onTriggered: {
            const item = graphPane.view.itemAtIndex(reclickGraphTimer.row)
            // A row the view has not laid out yet is not a row that was clicked: latching here would wait for an
            // answer to a question nobody put.
            if (!item)
                return
            if (acts.reclickGraphStep === 0) {
                // The pointer comes to rest on the chip first, which starts the rest that opens the card
                // (`GraphRowDelegate.restDelay`). Written where a real pointer writes it, so the row makes every
                // decision after that for itself (`row-part`).
                if (reclickGraphTimer.points)
                    item.pointerRowX = item.labelsW - 2
                item.leftClick(Qt.NoModifier)
                acts.reclickGraphStep = 1
            } else if (acts.reclickGraphStep === 1) {
                // Inside the window the first click opened, a second click is the other half of a double-click and not
                // a second click at all.
                if (item.clickGuarded)
                    return
                // waits(measured): the origin of the `wait=` below, which the report prints and nothing here reads
                acts.reclickGraphAt = Date.now()
                item.leftClick(Qt.NoModifier)
                // Read where it is set: the wait is short and the box is what it turns into.
                acts.reclickGraphArmed = item.renameArmed
                // The history walks away under the wait: the row that was clicked is pooled, and what was armed on it
                // has to survive that. Far enough that the row is well outside the view's own buffer.
                if (reclickGraphTimer.scrolls)
                    graphPane.view.positionViewAtIndex(reclickGraphTimer.row + 200, ListView.Beginning)
                if (reclickGraphTimer.marks) {
                    // **The mark is what the reader has to see during the wait**, so this run ends inside
                    // it. Read off the chip itself (`RefChip.waiting`), because a picture taken a beat
                    // late frames the box instead and would say nothing either way.
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
                // The box is open: the row it is on has to have been sent back into view before the shot
                // (`RepoPage.startRename`), which the walk does a beat after the box opens.
                if (reclickGraphTimer.scrolls && !graphPane.rowOnScreen(reclickGraphTimer.row))
                    return
                reclickGraphTimer.stop()
                Harness.report(
                "graph_reclick row=" + reclickGraphTimer.row
                + " armed=" + acts.reclickGraphArmed
                + " box=" + (graphPane.namingOid !== "")
                // Nothing hover opened or closed under the wait — the rest that was running when the second click
                // landed is part of the same beat (`GraphList.renameWaiting`).
                + " list=" + refList.opened
                + " card=" + rowCard.opened
                + " mode=" + graphPane.namingMode
                + " kind=" + graphPane.namingKind
                + " typed=" + graphPane.namingText
                // The tree has not moved: the gesture the reader made was not the double-click, and this is the half
                // of the report a picture of an open box cannot make.
                + " branch=" + workTree.branch
                // Whether the row the box is on is in sight — the whole of the scrolled run's claim, and true of the
                // plain one for nothing having moved it.
                + " shown=" + graphPane.rowOnScreen(reclickGraphTimer.row)
                // How long the box took, against the window it is waiting out. The sampler's own beat is in the
                // difference, so this is read as "about the window".
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
    // PGG_AUTO_ACT=graph-reclick-lanes: the same gesture put in through **the strip over the lane column** — the half
    // of the row between the two dividers, which has a press-taking layer of its own wherever the lanes overflow their
    // column (`GraphLanePan`). A reader aiming at the middle of a wide graph is aiming at that strip, and a strip that
    // answered with a copy of half of what a click does left the gesture doing nothing there.
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
                // The strip is only up while the lanes have somewhere sideways to go, so the column is squeezed to its
                // floor first — the state a repository wide enough to need panning is in from the start.
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
                + " branch=" + workTree.branch)
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
        /// Whether the two clicks are put in at **different surfaces** — the first at the row, the second at the card
        /// its chip opens into. That is what a reader does without knowing it: the card comes up on the chip's own
        /// seat after a rest, so the second click at one spot lands somewhere else.
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
                    // The write barrier is what finishes this one: the move is the whole of it.
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
                // The card came down for the box: it was standing on the column the box opens in, and one left up
                // would be covering what the run is about.
                + " list=" + refList.opened
                + " mode=" + graphPane.namingMode
                + " kind=" + graphPane.namingKind
                + " typed=" + graphPane.namingText
                + " branch=" + workTree.branch)
                driver.complete()
            }
        }
    }
}
