pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The hand that picks the log's text out of its rows: a drag selects, a plain click puts the selection down
// (デザイン規約 §git が言ったことを読む場所). There is no menu behind it — the reader drags and presses Ctrl+C, the way a
// terminal is read.
//
// **It is laid over the list rather than declared inside it**, for the two reasons that are the same reason twice: a
// `Flickable` takes the grab away from its own children once a drag passes the threshold, and `reuseItems` builds a
// scrolled-away row again from scratch. A hand that started on row 40 and is now dragging past the bottom of the
// window would lose its grip both ways.
//
// It is a plain `MouseArea` and it is not `hoverEnabled`, which is what lets the rows underneath keep their own hover —
// the lit ground and the one line a row has no room for (a `HoverHandler` here would take all of it, app-ui.md).
//
// **Every place in the frame that nobody else takes is a start**, the ground a log shorter than its panel leaves under
// the last row included (`rowAt`, 規約 §git が言ったことを読む場所). Nothing stands over these rows at all, so here that
// is the whole of the rule — except at the right edge, where the list's own bar is drawn over them and this stops
// short of it (`barRoom`, observed against the diff's hand, which had the same shape).
//
// **Which byte a press landed on takes two questions and neither of them is arithmetic.** This knows which of the
// row's three columns the pointer was over and how far into it in pixels; the column's own layout says which place of
// it that is (`LineRuler`), and the model says which byte of the line that place stands on — the line those columns
// are cut from is there, and where their characters are drawn is here.
Item {
    id: pick

    /// The rows, for the frame this stands over and for asking which row a point is on.
    required property var view
    /// Where the selection lives, and where the three columns come from.
    required property var commandsModel
    /// How much of the right edge belongs to the list's own scroll bar, which is where this frame ends
    /// (`AppListView.barRoom`).
    required property real barRoom
    /// The column's own layout, asked which place the press landed in. The same ruler the wash is placed with
    /// (`CommandRowDelegate`), so the hit and the wash cannot disagree.
    required property var ruler

    /// A drag has reached past the frame and wants the rows sent after it.
    signal scrollWanted(real dy)

    x: pick.view.x
    y: pick.view.y
    width: Math.max(0, pick.view.width - pick.barRoom)
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
    /// A press at a point of this item, in its own coordinates: true where the text took it, false where it goes down
    /// to the list itself — which is only ever a log with no rows in it. **The `MouseArea` below and a run enter
    /// here**, so a hand that was never wired up reports nothing (verify-ui §壊れない動詞の実装).
    function takeAt(x, y) {
        const row = pick.rowAt(y)
        if (row < 0)
            return false
        // The press is what puts the keys here as well, so Ctrl+C reaches the text a hand has just picked out — the
        // panel takes focus when it opens, but a reader who has been somewhere else since comes back through this and
        // nothing else.
        pick.view.forceActiveFocus()
        pick.handX = x
        pick.handY = y
        pick.pressText(row, pick.byteAt(row, x, y))
        return true
    }
    /// The hand has reached here. A drag that has left the rows keeps the one it can still see (`rowAt`).
    function followAt(x, y) {
        pick.handX = x
        pick.handY = y
        if (!pick.dragging)
            return
        const row = pick.rowAt(y)
        if (row >= 0)
            pick.dragText(row, pick.byteAt(row, x, y))
    }
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
    /// Which row a point of this item is over, or -1 where the list has no rows at all. **The point is clamped
    /// twice.** To the frame, so that a drag past either edge keeps naming the row it can still see while the edge
    /// below carries the rows to it — and to the rows themselves (`onRows`), so that the ground a short log leaves
    /// under its last row belongs to that row.
    function rowAt(y) {
        const inside = Math.max(0, Math.min(y, pick.height - 1))
        // One pixel in from the left: rows are as wide as the list, so any x inside it finds the same row.
        return pick.view.indexAt(1, pick.onRows(pick.view.contentY + inside))
    }
    /// A content y brought onto the band the rows themselves stand on — the whole of what makes the ground under the
    /// last row a place a selection can start (規約 §git が言ったことを読む場所「掴めるのは、誰も取らない所すべて」).
    /// Without it a press there reached nothing at all, which is the same dead corner the right pane's values had.
    ///
    /// `originY` rather than zero: this list is sent to its end over rows of differing heights — a failure brings
    /// git's words down with it — and a view that has been so moves its own origin (`CommandsPane.clampY`). An empty
    /// log has no band at all — the clamp then answers above its own last row, `indexAt` finds nothing, and the press
    /// goes down to the list as it always did.
    function onRows(y) {
        const first = pick.view.originY
        const last = first + pick.view.contentHeight - 1
        return Math.max(first, Math.min(y, last))
    }

    /// Which byte of a row's line a point lands on. The row itself says where its three columns are drawn, and each
    /// column answers for the pixels inside it; a gap holds one tab, whose only two places are its ends.
    function byteAt(row, x, y) {
        const item = pick.view.itemAtIndex(row)
        if (!item)
            return 0
        if (y - (item.y - pick.view.contentY) >= item.lineHeight)
            return pick.commandsModel.hitAt(row, pick.atBelow, 0)
        if (x < item.clockEnd)
            return pick.hit(row, pick.atClock, x - item.clockX)
        if (x < item.cmdX)
            return pick.commandsModel.hitAt(row, pick.atGapCmd, pick.sideOf(x, item.clockEnd, item.cmdX))
        if (x < item.cmdEnd)
            return pick.hit(row, pick.atCmd, x - item.cmdX)
        if (x < item.outX)
            return pick.commandsModel.hitAt(row, pick.atGapOut, pick.sideOf(x, item.cmdEnd, item.outX))
        return pick.hit(row, pick.atOut, x - item.outX)
    }
    /// One column: the pixels are the column's own to read (`LineRuler`), and the place that comes back is the
    /// model's to turn into a byte of the line. **Nothing in between is arithmetic** — a column is not a width, and
    /// the walk that treated it as one selected a byte in the middle of a line the reader had dragged past the end of.
    function hit(row, at, x) {
        const text = pick.commandsModel.columnText(row, at)
        return pick.commandsModel.hitAt(row, at, pick.ruler.placeAt(text, false, x))
    }
    /// Which side of a gap's one tab a point is on, as the two places that tab has.
    function sideOf(x, from, to) {
        return x < (from + to) / 2 ? 0 : 1
    }

    // ---- the ground under the last row ---------------------------------------------------------------------------
    /// Where the ground a log shorter than its panel leaves under the last row begins, in this item's own
    /// coordinates, and whether there is any of it at all. A sweep run against a log that fills its panel would prove
    /// nothing, so it says so rather than passing (verify-ui `commands-sweep`).
    readonly property real groundTop: Math.max(0, pick.view.originY + pick.view.contentHeight - pick.view.contentY)
    readonly property bool hasGround: pick.groundTop < pick.height - 2
    /// Automation: the gesture as a hand makes it — a press on that ground, and a drag up into the text. It enters the
    /// same two functions the `MouseArea` below calls, and it **starts in the ground**: an injection that began on a
    /// row would go green with the whole of this taken back out (`SweepRoom`, the same rule).
    function sweepFromGround(fx, fy) {
        if (!pick.hasGround)
            return false
        const x = Math.max(1, Math.min(pick.width - 1, pick.width * fx))
        const y = pick.groundTop + 1 + (pick.height - pick.groundTop - 2) * fy
        if (!pick.takeAt(x, y))
            return false
        // Up into the text at the x it started from — a drag does not jump sideways, and reading a row out at a place
        // the reader never pressed is how a run passes while the gesture does not work.
        const top = pick.view.itemAtIndex(0)
        pick.followAt(x, top ? top.y - pick.view.contentY + top.height / 2 : 1)
        pick.releaseText()
        return true
    }

    // ---- the hand ----------------------------------------------------------------------------------------------
    MouseArea {
        id: hand
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton
        // Not hoverEnabled: the rows below keep their own (see the note at the top).
        cursorShape: Qt.IBeamCursor
        // The two above, and nothing beside them: what the hand does is written once, where a run enters it too.
        onPressed: mouse => { mouse.accepted = pick.takeAt(mouse.x, mouse.y) }
        onPositionChanged: mouse => pick.followAt(mouse.x, mouse.y)
        onReleased: pick.releaseText()
        onCanceled: pick.releaseText()
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
