pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// Where the graph's selection is, and where the view stands to show it: the arrow keys walking the history (規約
// §矢印で履歴を辿る), the jumps the page asks for, and putting the reader's place back after a background rebuild wrote new
// rows over the old ones.
//
// Nothing here draws — every one of these is a write to the list's `currentIndex` or its `contentY`, and this is the
// one place either is written from.
Item {
    id: walk

    /// The list being walked.
    required property var view
    required property var graphModel
    /// A question is standing over a row, waiting to be answered.
    required property bool asking
    /// The page is holding the selection where it is: a half-written message has put a question up and the highlight
    /// back, and it stands until it is answered. The keys must not tug against it — each press would be pushed back and
    /// the pair would fight.
    property bool selectionHeld: false

    /// A step came to rest on a row and it is time to read it.
    signal activated(string oidHex)

    /// Moves the selection one row (`delta` = ∓1) and brings it into view; answers whether it moved. The keys and the
    /// automation hook both come through here — a headless run cannot inject a keystroke, so the step has to be
    /// callable as well as pressable (verify-ui).
    ///
    /// Refused while something stands over a row: a question waiting to be answered, a name box being typed into, or
    /// the page holding the selection back. The name box is the one that has to be named here rather than left to the
    /// focus: it lives inside a row, and a single-line box does not consume Up and Down — they come up through the
    /// delegate to the list.
    ///
    /// Refused as well while the pane is off screen: this item is inside it, so its own visibility is the pane's.
    function stepRow(delta) {
        if (!walk.visible || walk.selectionHeld || walk.asking
                || walk.view.namingOid !== "" || walk.view.count === 0)
            return false
        const from = walk.view.currentIndex
        const row = from < 0 ? 0 : Math.max(0, Math.min(from + delta, walk.view.count - 1))
        if (row === from)
            return false
        // Asked before the move: whether there was a reading position to keep. A step off the edge is one row short of
        // being on screen, but a selection that was nowhere in sight is nobody's place — that one is centered instead.
        const near = walk.view.rowOnScreen(from)
        walk.view.currentIndex = row
        walk.revealStep(row, near)
        walk.noteStep()
        return true
    }
    /// Brings a stepped-onto row into view. `near` moves as little as will do, which is the row itself; anything else
    /// centers.
    ///
    /// Row positions are worked out rather than read off the items: `itemAtIndex` answers null for rows the view has
    /// not built, and a walk that trusts it is cut to the viewport (P3-確認事項: the WIP list's range selection is the
    /// standing example).
    function revealStep(row, near) {
        if (!near) {
            walk.view.positionViewAtIndex(row, ListView.Center)
            return
        }
        // The first row's box carries the list's top margin with it — its highlight is painted over that sliver — so
        // stepping onto it goes the whole way to the top rather than leaving a dark band.
        const top = walk.view.originY + row * Theme.graphRowHeight
                    - (row === 0 ? walk.view.topMargin : 0)
        const bottom = walk.view.originY + (row + 1) * Theme.graphRowHeight
        if (top < walk.view.contentY)
            walk.view.contentY = walk.view.clampY(top)
        else if (bottom > walk.view.contentY + walk.view.height)
            walk.view.contentY = walk.view.clampY(bottom - walk.view.height)
    }
    /// How the last step left `row` sitting in the viewport, for the headless run: `in` when the view never moved (the
    /// row was already there), `edge` when it came in flush against the top or the bottom (the least a step can move
    /// the view), `center` when it was put in the middle instead. Which of the three is right is the whole of 規約
    /// §矢印で履歴を辿る's second paragraph, and a screenshot cannot tell an edge that was reached by one row from one that was
    /// jumped to. `wasY` is where the view stood before that step.
    function stepLanding(row, wasY) {
        if (Math.abs(walk.view.contentY - wasY) < 1)
            return "in"
        const top = walk.view.originY + row * Theme.graphRowHeight
        const bottom = top + Theme.graphRowHeight
        const viewBottom = walk.view.contentY + walk.view.height
        if (Math.abs(top - walk.view.contentY) < 1 || Math.abs(bottom - viewBottom) < 1)
            return "edge"
        if (Math.abs((top + bottom) / 2 - (walk.view.contentY + viewBottom) / 2) < Theme.graphRowHeight)
            return "center"
        return "adrift"
    }

    /// Books the reading of the row stepped onto. The first step of a run is read at once — a single press has to
    /// answer inside the interaction budget — and the ones behind it only push the settle back. So a held arrow is read
    /// exactly twice: where it set off, and where it stopped. A selection carries three git processes with it (`git
    /// show`, the history question, the signature question), which is not a thing to run at the keyboard's repeat rate.
    function noteStep() {
        if (stepTimer.running) {
            walk.stepPending = true
            stepTimer.restart()
            return
        }
        walk.stepPending = false
        walk.landStep()
        stepTimer.restart()
    }
    /// Whether a step went by unread while the settle was running. Without it the settle behind a single press would
    /// read the same row twice.
    property bool stepPending: false
    function landStep() {
        // The row under the highlight as it stands, not the one the key asked for: a background rebuild during the
        // settle writes the rows in place, and what was stepped onto is whatever is there to be seen now.
        const oidHex = walk.graphModel.oidAt(walk.view.currentIndex)
        if (oidHex !== "")
            walk.activated(oidHex)
    }
    Timer {
        id: stepTimer
        interval: Metrics.keyStepSettleMs
        onTriggered: {
            if (!walk.stepPending)
                return
            walk.stepPending = false
            walk.landStep()
        }
    }
    function jumpToRow(row) {
        walk.view.currentIndex = row
        walk.view.positionViewAtIndex(row, ListView.Center)
    }
    /// Re-centers on the current row a beat from now. Centering must outlive the ListView's own relayout: a model reset
    /// (tag swap / reload) zeroes contentY during the polish that runs after our handlers, so the anchor is applied a
    /// beat later.
    function anchorSoon() {
        anchorTimer.restart()
    }
    /// Puts the viewport back over the rows it was reading after `rows` commits arrived above them. A background
    /// rebuild replaces the graph by writing over the rows in place, so newcomers at the top slide everything below
    /// them down while the view stays where it is and quietly shows different commits — the further down someone is
    /// reading, the more that costs them.
    ///
    /// At the top of the list there is nothing to preserve: the newest commits are exactly what belongs there, so it
    /// stays pinned. Applied a beat later, for the same reason as the anchor above: the list has not laid the new rows
    /// out yet, so its content is still the old height and clamping against it would swallow the correction.
    function shiftRows(rows) {
        if (rows === 0)
            return
        if (walk.view.contentY <= walk.view.originY - walk.view.topMargin)
            return
        shiftTimer.pending += rows
        shiftTimer.restart()
    }
    /// Brings `row` into view once the pass that put it there has settled, and only if it is not already on screen — a
    /// row in sight is not worth taking the reader's place for (the same rule `goToMatch` answers to).
    ///
    /// Carried by the shift timer rather than one of its own: the two write the same contentY for opposite reasons, and
    /// whether the row is on screen is only true of the position the shift leaves behind. One timer settles the order.
    function showRowSoon(row) {
        shiftTimer.showRow = row
        shiftTimer.restart()
    }
    Timer {
        id: shiftTimer
        property int pending: 0
        property int showRow: -1
        interval: Metrics.anchorDelayMs
        onTriggered: {
            const rows = shiftTimer.pending
            shiftTimer.pending = 0
            if (rows !== 0)
                walk.view.contentY = walk.view.clampY(walk.view.contentY + rows * Theme.graphRowHeight)
            const row = shiftTimer.showRow
            shiftTimer.showRow = -1
            if (row >= 0 && !walk.view.rowOnScreen(row))
                walk.view.positionViewAtIndex(row, ListView.Center)
        }
    }
    Timer {
        id: anchorTimer
        interval: Metrics.anchorDelayMs
        onTriggered: {
            if (walk.view.currentIndex >= 0)
                walk.view.positionViewAtIndex(walk.view.currentIndex, ListView.Center)
        }
    }
}
