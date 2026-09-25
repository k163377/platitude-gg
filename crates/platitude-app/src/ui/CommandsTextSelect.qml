pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The hand that picks the log's text out of its rows: a drag selects, a plain click puts the selection down
// (デザイン規約 §git が言ったことを読む場所).
//
// Laid over the list, not inside the rows: a `Flickable` takes the grab from its children once a drag passes the
// threshold, and `reuseItems` rebuilds a scrolled-away row, so a drag held by a row loses its grip.
//
// A plain `MouseArea`, so the rows underneath keep their own hover (a `HoverHandler` here would take all of it —
// rules-refs/app-ui.md「行に重ねる面の `HoverHandler` は祖先が持つ」).
//
// Every place in the frame is a start, the ground under a short log's last row included (`rowAt`), except the
// list's own scroll bar at the right edge (`barRoom`).
Item {
    id: pick

    /// The rows, for the frame this stands over and for asking which row a point is on.
    required property var view
    /// Where the selection lives, and where the three columns come from.
    required property var commandsModel
    /// The list's scroll bar at the right edge, where this frame ends (`AppListView.barRoom`).
    required property real barRoom
    /// The column's layout, asked which place a press landed in — the same ruler that places the wash
    /// (`CommandRowDelegate`), so hit and wash cannot disagree.
    required property var ruler

    /// A drag has reached past the frame and wants the rows sent after it.
    signal scrollWanted(real dy)

    x: pick.view.x
    y: pick.view.y
    width: Math.max(0, pick.view.width - pick.barRoom)
    height: pick.view.height

    /// Column ids as `CommandsModel` numbers them: the three the row draws, the two tab gaps, and the block of words
    /// a failure carries below its line.
    readonly property int atClock: 0
    readonly property int atGapCmd: 1
    readonly property int atCmd: 2
    readonly property int atGapOut: 3
    readonly property int atOut: 4
    readonly property int atBelow: 9

    property bool dragging: false
    /// Where the hand was last seen, in this item's coordinates (the edge ticker below re-reads it).
    property real handX: 0
    property real handY: 0

    // ---- what the hand does ------------------------------------------------------------------------------------
    /// A press at a point (own coordinates): true where the text took it, false where it goes down to the list —
    /// only a log with no rows. The `MouseArea` below and a run both enter here, so an unwired hand reports nothing
    /// (verify-ui §壊れない動詞の実装と反復).
    function takeAt(x, y) {
        const row = pick.rowAt(y)
        if (row < 0)
            return false
        // The press takes focus so Ctrl+C reaches the text just picked; the panel only takes it when it opens.
        pick.view.forceActiveFocus()
        pick.handX = x
        pick.handY = y
        pick.pressText(row, pick.byteAt(row, x, y))
        return true
    }
    function followAt(x, y) {
        pick.handX = x
        pick.handY = y
        if (!pick.dragging)
            return
        const row = pick.rowAt(y)
        if (row >= 0)
            pick.dragText(row, pick.byteAt(row, x, y))
    }
    function pressText(row, at) {
        pick.commandsModel.beginSelect(row, at)
        pick.dragging = true
    }
    function dragText(row, at) {
        pick.commandsModel.extendSelect(row, at)
    }
    /// An empty selection is cleared, so a plain click takes down the previous drag's wash.
    function releaseText() {
        pick.dragging = false
        if (pick.commandsModel.selectionText() === "")
            pick.commandsModel.clearSelect()
    }

    // ---- pixels to rows and columns ----------------------------------------------------------------------------
    /// Which row a point is over, or -1 where the list has no rows. Clamped twice: to the frame, so a drag past an
    /// edge keeps naming the row it can see while the ticker scrolls, and onto the rows (`onRows`).
    function rowAt(y) {
        const inside = Math.max(0, Math.min(y, pick.height - 1))
        // Rows are as wide as the list, so any x inside it finds the same row.
        return pick.view.indexAt(1, pick.onRows(pick.view.contentY + inside))
    }
    /// A content y clamped onto the band the rows stand on — what makes the ground under the last row a start
    /// (規約 §git が言ったことを読む場所「始点はパネルの枠の中で誰も press を取らない所すべて」).
    /// From `originY`, not 0: rows of differing heights move the view's origin once it is sent to its end
    /// (`AppListView.clampY`). An empty log has no band, so `indexAt` finds nothing and the press goes to the list.
    function onRows(y) {
        const first = pick.view.originY
        const last = first + pick.view.contentHeight - 1
        return Math.max(first, Math.min(y, last))
    }

    /// Which byte of a row's line a point lands on; a gap holds one tab, whose only two places are its ends.
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
    /// One column: pixels to place by the column's layout (`LineRuler`), place to byte by the model. Both are
    /// lookups — treating a column as a width lands mid-line when the drag runs past its end.
    function hit(row, at, x) {
        const text = pick.commandsModel.columnText(row, at)
        return pick.commandsModel.hitAt(row, at, pick.ruler.placeAt(text, false, x))
    }
    function sideOf(x, from, to) {
        return x < (from + to) / 2 ? 0 : 1
    }

    // ---- the ground under the last row ---------------------------------------------------------------------------
    /// Where the ground under a short log's last row begins (own coordinates), and whether there is any: a sweep
    /// against a log that fills its panel proves nothing, so it says so (verify-ui `commands-sweep`).
    readonly property real groundTop: Math.max(0, pick.view.originY + pick.view.contentHeight - pick.view.contentY)
    readonly property bool hasGround: pick.groundTop < pick.height - 2
    /// Automation: a press on that ground and a drag up into the text, through the functions the `MouseArea` calls.
    /// It starts in the ground: a sweep started on a row would pass with the ground rule taken out.
    function sweepFromGround(fx, fy) {
        if (!pick.hasGround)
            return false
        const x = Math.max(1, Math.min(pick.width - 1, pick.width * fx))
        const y = pick.groundTop + 1 + (pick.height - pick.groundTop - 2) * fy
        if (!pick.takeAt(x, y))
            return false
        // Straight up at the starting x — a sideways jump would let a run pass while the real gesture fails.
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
        // The rows below keep their own hover (see the note at the top).
        cursorShape: Qt.IBeamCursor
        onPressed: mouse => { mouse.accepted = pick.takeAt(mouse.x, mouse.y) }
        onPositionChanged: mouse => pick.followAt(mouse.x, mouse.y)
        onReleased: pick.releaseText()
        onCanceled: pick.releaseText()
    }

    // ---- the edge ----------------------------------------------------------------------------------------------
    /// How far past the frame the hand is; zero inside, which stops the ticker.
    readonly property real pastY: pick.handY < 0 ? pick.handY
                                  : pick.handY > pick.height ? pick.handY - pick.height : 0
    // A drag past the frame scrolls the log at the middle-click hand's speed, and re-reads the row every step: the
    // rows move under a still pointer without any mouse event.
    Timer {
        running: pick.dragging && pick.pastY !== 0
        // One frame, the tick every drift in the app uses.
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
