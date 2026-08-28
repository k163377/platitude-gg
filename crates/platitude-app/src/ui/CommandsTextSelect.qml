pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The hand that picks the log's text out of its rows: a drag selects, a plain click puts the selection down
// (デザイン規約 §git が言ったことを読む場所). There is no menu behind it — the reader drags and presses Ctrl+C, the way a
// terminal is read (2026-08-28 ユーザー指示).
//
// **It is laid over the list rather than declared inside it**, for the two reasons that are the same reason twice: a
// `Flickable` takes the grab away from its own children once a drag passes the threshold, and `reuseItems` builds a
// scrolled-away row again from scratch. A hand that started on row 40 and is now dragging past the bottom of the
// window would lose its grip both ways.
//
// It is a plain `MouseArea` and it is not `hoverEnabled`, which is what lets the rows underneath keep their own hover —
// the lit ground and the one line a row has no room for (a `HoverHandler` here would take all of it, app-ui.md).
//
// Everything about *which byte* a press landed on is asked of the model: this knows which of the row's three columns
// the pointer was over and how far along it, and only the model holds the line those columns are drawn from.
Item {
    id: pick

    /// The rows, for the frame this stands over and for asking which row a point is on.
    required property var view
    /// Where the selection lives.
    required property var commandsModel
    /// One measured column of the mono font and what a wide glyph costs beyond its two — the same pair the wash is
    /// placed with, so the hit and the wash agree.
    required property real charW
    required property real wideDelta

    /// A drag has reached past the frame and wants the rows sent after it.
    signal scrollWanted(real dy)

    x: pick.view.x
    y: pick.view.y
    width: pick.view.width
    height: pick.view.height

    /// Which column a press landed in, as `CommandsModel` numbers them: the three the row draws, the two tabs between
    /// them, and one more for the block of words a failure carries below its line.
    readonly property int atClock: 0
    readonly property int atGapCmd: 1
    readonly property int atCmd: 2
    readonly property int atGapOut: 3
    readonly property int atOut: 4
    readonly property int atBelow: 9

    /// Whether a drag is running. The press that started it is still held, so this and the area's `pressed` say the
    /// same thing.
    property bool dragging: false
    /// Where the hand was last seen, in this item's own coordinates. Kept because the edge below re-reads it after
    /// every step it sends: the rows move under a pointer that is standing still.
    property real handX: 0
    property real handY: 0

    // ---- what the hand does, named so a headless run enters where it enters -------------------------------------
    /// A press on the text: the selection starts here and holds nothing until the hand moves.
    function pressText(row, at) {
        pick.commandsModel.beginSelect(row, at)
        pick.dragging = true
    }
    /// The hand has reached here.
    function dragText(row, at) {
        pick.commandsModel.extendSelect(row, at)
    }
    /// The button is up. A press that never moved selected nothing, and a selection of nothing is no selection — so
    /// the wash the previous drag left goes down with this press rather than standing under an unrelated click.
    function releaseText() {
        pick.dragging = false
        if (pick.commandsModel.selectionText() === "")
            pick.commandsModel.clearSelect()
    }

    // ---- pixels to rows and columns ----------------------------------------------------------------------------
    /// Which row a point of this item is over, or -1 where it is over none. The y is clamped to the frame so that a
    /// drag past either edge keeps naming the row it can still see, while the edge below carries the rows to it.
    function rowAt(y) {
        const inside = Math.max(0, Math.min(y, pick.height - 1))
        // One pixel in from the left: rows are as wide as the list, so any x inside it finds the same row.
        return pick.view.indexAt(1, pick.view.contentY + inside)
    }

    /// Which byte of a row's line a point lands on. The row itself says where its three columns are drawn; a gap is
    /// answered as how far across it the pointer was (0..1), which is all there is to say about the one tab in it.
    function byteAt(row, x, y) {
        const item = pick.view.itemAtIndex(row)
        if (!item)
            return 0
        if (y - (item.y - pick.view.contentY) >= item.lineHeight)
            return pick.commandsModel.hitAt(row, pick.atBelow, 0, pick.charW, pick.wideDelta)
        if (x < item.clockEnd)
            return pick.hit(row, pick.atClock, x - item.clockX)
        if (x < item.cmdX)
            return pick.hit(row, pick.atGapCmd, (x - item.clockEnd) / Math.max(1, item.cmdX - item.clockEnd))
        if (x < item.cmdEnd)
            return pick.hit(row, pick.atCmd, x - item.cmdX)
        if (x < item.outX)
            return pick.hit(row, pick.atGapOut, (x - item.cmdEnd) / Math.max(1, item.outX - item.cmdEnd))
        return pick.hit(row, pick.atOut, x - item.outX)
    }
    function hit(row, at, x) {
        return pick.commandsModel.hitAt(row, at, x, pick.charW, pick.wideDelta)
    }

    // ---- the hand ----------------------------------------------------------------------------------------------
    MouseArea {
        id: hand
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton
        // Not hoverEnabled: the rows below keep their own (see the note at the top).
        cursorShape: Qt.IBeamCursor
        onPressed: mouse => hand.take(mouse)
        onPositionChanged: mouse => hand.follow(mouse)
        onReleased: pick.releaseText()
        onCanceled: pick.releaseText()

        /// Decides whether this press is the text's at all, and starts the drag where it is.
        function take(mouse) {
            const row = pick.rowAt(mouse.y)
            if (row < 0) {
                // Down to the list itself: below the last row there is nothing to pick.
                mouse.accepted = false
                return
            }
            // The press is what puts the keys here as well, so Ctrl+C reaches the text a hand has just picked out —
            // the panel takes focus when it opens, but a reader who has been somewhere else since comes back through
            // this and nothing else.
            pick.view.forceActiveFocus()
            pick.handX = mouse.x
            pick.handY = mouse.y
            pick.pressText(row, pick.byteAt(row, mouse.x, mouse.y))
        }
        function follow(mouse) {
            pick.handX = mouse.x
            pick.handY = mouse.y
            if (!pick.dragging)
                return
            const row = pick.rowAt(mouse.y)
            if (row >= 0)
                pick.dragText(row, pick.byteAt(row, mouse.x, mouse.y))
        }
    }

    // ---- the edge ----------------------------------------------------------------------------------------------
    /// How far past the frame the hand has reached. Zero while it is inside, which is also what stops the ticker.
    readonly property real pastY: pick.handY < 0 ? pick.handY
                                  : pick.handY > pick.height ? pick.handY - pick.height : 0
    // A drag that has left the frame carries the log after it, at the speed the middle-click hand travels at. The rows
    // are re-read on every step: they move under a pointer that is standing still, so what is under it changes without
    // the mouse saying anything.
    Timer {
        running: pick.dragging && pick.pastY !== 0
        // A frame, the same tick every other drift in the app travels on.
        interval: 16
        repeat: true
        onTriggered: {
            pick.scrollWanted(pick.pastY * Metrics.middleScrollGain)
            const row = pick.rowAt(pick.handY)
            if (row >= 0)
                pick.dragText(row, pick.byteAt(row, pick.handX, pick.handY))
        }
    }
}
