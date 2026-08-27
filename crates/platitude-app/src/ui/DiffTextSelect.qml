pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The hand that picks the diff's text out of its rows: a drag selects, a right-click asks for the menu, and a plain
// click puts the selection down (デザイン規約 §diff の中身をコピーする).
//
// **It covers the code column and not the gutter.** The two numbers and the seat between them are the row's own —
// the `+` a line puts out there has to keep taking presses — so this starts where they end (`gutterW`).
//
// **It is laid over the list rather than declared inside it**, for two reasons that are the same reason twice: a
// `Flickable` takes the grab away from its own children once a drag passes the threshold, and `reuseItems` builds a
// scrolled-away row again from scratch. A hand that started on row 40 and is now dragging past the bottom of the
// window would lose its grip both ways.
//
// It is a plain `MouseArea` and it is not `hoverEnabled`, which is what lets the rows underneath keep their own hover
// — the `+` under the pointer, the lit hunk (デザイン規約 §diff の中のステージ; a `HoverHandler` here would take all of it,
// 2026-08-17 実測).
//
// Everything about *where* the press landed in the file is asked of the model: this knows pixels, and only the model
// holds the lines those pixels are drawn from (`DiffModel::selection`).
Item {
    id: pick

    /// The rows, for the frame this stands over and for asking which row a point is on.
    required property var view
    /// Where the selection lives.
    required property var diffModel
    /// How wide the gutter is, which is where the code column starts.
    required property real gutterW
    /// How far the code has been sent sideways (`DiffCodeScroll.offset`) — a press lands on the character under it,
    /// not on the one that would be there at rest.
    required property real codeX
    /// One measured column of the mono font and what a wide glyph costs beyond its two (`DiffPane.charW` /
    /// `wideDelta`). The same pair the emphasis wash is placed with, so the hit and the wash agree.
    required property real charW
    required property real wideDelta

    /// A drag has reached past the frame and wants the rows (and the text) sent after it.
    signal scrollWanted(real dy, real dx)
    /// A right-click landed on the text; the selection is already what it should act on.
    signal menuWanted()

    x: pick.view.x + pick.gutterW
    y: pick.view.y
    width: Math.max(0, pick.view.width - pick.gutterW)
    height: pick.view.height

    /// Whether a drag is running. The press that started it is still held, so this and `hand.pressed` say the same
    /// thing — except during a right-click, which never starts one.
    property bool dragging: false
    /// Where the hand was last seen, in this item's own coordinates. Kept because the edge below re-reads it after
    /// every step it sends: the rows move under a pointer that is standing still.
    property real handX: 0
    property real handY: 0

    // ---- what the hand does, named so a headless run enters where it enters -------------------------------------
    /// A press on the text: the selection starts here and holds nothing until the hand moves.
    function pressText(row, at) {
        pick.diffModel.beginSelect(row, at)
        pick.dragging = true
    }
    /// The hand has reached here.
    function dragText(row, at) {
        pick.diffModel.extendSelect(row, at)
    }
    /// The button is up. A press that never moved selected nothing, and a selection of nothing is no selection —
    /// saying so keeps `selActive` honest for whoever asks next.
    function releaseText() {
        pick.dragging = false
        if (!pick.diffModel.selHasNew && pick.diffModel.selRemoved === 0)
            pick.diffModel.clearSelect()
    }
    /// A right-click. Inside the selection it leaves it alone; outside it, the row underneath becomes the selection —
    /// the rule the file list already reads by (デザイン規約 §バケツごとの一覧: メニューは常に光っている行に効く).
    function askMenu(row, at) {
        if (!pick.diffModel.selectionHolds(row, at))
            pick.diffModel.selectRow(row)
        pick.menuWanted()
    }

    // ---- pixels to rows ----------------------------------------------------------------------------------------
    /// Which row a point of this item is over, or -1 where it is over none. The y is clamped to the frame so that a
    /// drag past either edge keeps naming the row it can still see, while the edge below carries the rows to it.
    function rowAt(y) {
        const inside = Math.max(0, Math.min(y, pick.height - 1))
        // One pixel in from the left: rows are as wide as the list, so any x inside it finds the same row.
        return pick.view.indexAt(1, pick.view.contentY + inside)
    }
    /// Which byte of that row's line an x of this item lands on.
    function byteAt(row, x) {
        return pick.diffModel.hitByteAt(row, pick.codeX + x, pick.charW, pick.wideDelta)
    }
    /// Whether this row is one the hand may take. A hunk heading is not: it is the pane's own words, and the press
    /// there belongs to the two that act on the hunk (デザイン規約 §diff の中のステージ).
    function takesPress(row) {
        const item = pick.view.itemAtIndex(row)
        return !!item && item.kind !== "hunk"
    }

    // ---- the hand ----------------------------------------------------------------------------------------------
    MouseArea {
        id: hand
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        // Not hoverEnabled: the rows below keep their own (see the note at the top).
        cursorShape: Qt.IBeamCursor
        onPressed: mouse => hand.take(mouse)
        onPositionChanged: mouse => hand.follow(mouse)
        onReleased: pick.releaseText()
        onCanceled: pick.releaseText()
        onClicked: mouse => hand.answer(mouse)

        /// Decides whether this press is the text's at all, and starts the drag where it is.
        function take(mouse) {
            const row = pick.rowAt(mouse.y)
            if (row < 0 || !pick.takesPress(row)) {
                // Down to whatever is under this: the hunk's own two words, or the list itself.
                mouse.accepted = false
                return
            }
            pick.handX = mouse.x
            pick.handY = mouse.y
            if (mouse.button === Qt.LeftButton)
                pick.pressText(row, pick.byteAt(row, mouse.x))
        }
        function follow(mouse) {
            pick.handX = mouse.x
            pick.handY = mouse.y
            if (!pick.dragging)
                return
            const row = pick.rowAt(mouse.y)
            if (row >= 0)
                pick.dragText(row, pick.byteAt(row, mouse.x))
        }
        function answer(mouse) {
            if (mouse.button !== Qt.RightButton)
                return
            const row = pick.rowAt(mouse.y)
            if (row >= 0)
                pick.askMenu(row, pick.byteAt(row, mouse.x))
        }
    }

    // ---- the edge ----------------------------------------------------------------------------------------------
    /// How far past the frame the hand has reached, on each axis. Zero while it is inside, which is also what stops
    /// the ticker below.
    readonly property real pastY: pick.handY < 0 ? pick.handY
                                  : pick.handY > pick.height ? pick.handY - pick.height : 0
    readonly property real pastX: pick.handX < 0 ? pick.handX
                                  : pick.handX > pick.width ? pick.handX - pick.width : 0
    // A drag that has left the window carries the diff after it, at the speed the middle-click hand travels at — one
    // gain for every hand that sends this pane. The rows are re-read on every step: they move under a pointer that is
    // standing still, so what is under it changes without the mouse saying anything.
    Timer {
        running: pick.dragging && (pick.pastY !== 0 || pick.pastX !== 0)
        // A frame, the same tick `MiddleAutoScroll` drifts on — the two are one gesture apart and should travel at
        // the same rate.
        interval: 16
        repeat: true
        onTriggered: {
            pick.scrollWanted(pick.pastY * Metrics.middleScrollGain, pick.pastX * Metrics.middleScrollGain)
            const row = pick.rowAt(pick.handY)
            if (row >= 0)
                pick.dragText(row, pick.byteAt(row, pick.handX))
        }
    }
}
