pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The hand that picks the log's text out of its rows: a drag selects, a double-click takes a word and the drag behind
// it goes word by word (`wordAt`), a plain click puts the selection down — with Ctrl+C, the log's only way to copy
// (`CommandsPane`).
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

    /// A drag has reached the edge's belt and wants the rows sent after it, by this many pixels (the edge, below).
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
    property point handPosition: Qt.point(0, 0)
    /// Whether the drag has taken text yet — left the place it was pressed at, which the two below hold. Only then
    /// does the edge below send: a browser starts its selection's autoscroll the same way, so a click on the last row
    /// in sight, most of which is the edge's belt, leaves the rows where they are.
    property bool selecting: false
    property int fromRow: -1
    property int fromAt: 0

    // ---- what the hand does ------------------------------------------------------------------------------------
    /// A press at a point (own coordinates): true where the text took it, false where it goes down to the list —
    /// only a log with no rows. The `MouseArea` below and a run both enter here, so an unwired hand reports nothing
    /// (verify-ui §壊れない動詞の実装と反復).
    function takeAt(position) {
        const row = pick.rowAt(position.y)
        if (row < 0)
            return false
        // The press takes focus so Ctrl+C reaches the text just picked; the panel only takes it when it opens.
        pick.view.forceActiveFocus()
        pick.handPosition = position
        pick.pressText(row, pick.byteAt(row, position))
        return true
    }
    function followAt(position) {
        pick.handPosition = position
        if (!pick.dragging)
            return
        const row = pick.rowAt(position.y)
        if (row >= 0)
            pick.dragText(row, pick.byteAt(row, position))
    }
    function pressText(row, at) {
        pick.commandsModel.beginSelect(row, at)
        pick.selecting = false
        pick.fromRow = row
        pick.fromAt = at
        pick.dragging = true
    }
    function dragText(row, at) {
        pick.commandsModel.extendSelect(row, at)
        if (row !== pick.fromRow || at !== pick.fromAt)
            pick.selecting = true
    }
    /// A double-click at a point (own coordinates): the word under it becomes the selection, and the drag behind it
    /// grows word by word (`CommandsModel::take_word`). It lands after the second press, which `takeAt` has already
    /// taken (`tst_textdoubleclick`). A gap, the words below a failure's line and the blank past its end take nothing:
    /// the model names no character there.
    function wordAt(position) {
        const row = pick.rowAt(position.y)
        const item = pick.view.itemAtIndex(row)
        if (!item)
            return
        const at = pick.columnAt(item, position)
        const drawn = at === pick.atClock || at === pick.atCmd || at === pick.atOut
        const place = drawn ? pick.ruler.unitAt(pick.commandsModel.columnText(row, at), false,
                                                position.x - pick.columnX(item, at))
                            : 0
        pick.commandsModel.selectWord(row, at, place)
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
    /// A content y clamped onto the band the rows stand on — what makes the ground under the last row a start (see the
    /// header; 規約 §右のペインの字は掴める), a sweep from there running from the last row's end.
    /// From `originY`, not 0: rows of differing heights move the view's origin once it is sent to its end
    /// (`AppListView.clampY`). An empty log has no band, so `indexAt` finds nothing and the press goes to the list.
    function onRows(y) {
        const first = pick.view.originY
        const last = first + pick.view.contentHeight - 1
        return Math.max(first, Math.min(y, last))
    }

    /// Which byte of a row's line a point lands on; a gap holds one tab, whose only two places are its ends.
    function byteAt(row, position) {
        const item = pick.view.itemAtIndex(row)
        if (!item)
            return 0
        const x = position.x
        const at = pick.columnAt(item, position)
        if (at === pick.atBelow)
            return pick.commandsModel.hitAt(row, at, 0)
        if (at === pick.atGapCmd)
            return pick.commandsModel.hitAt(row, at, pick.sideOf(x, item.clockEnd, item.cmdX))
        if (at === pick.atGapOut)
            return pick.commandsModel.hitAt(row, at, pick.sideOf(x, item.cmdEnd, item.outX))
        return pick.hit(row, at, x - pick.columnX(item, at))
    }
    /// Which of a row's columns a point is over: the three it draws, the gaps between them, or the words a failure
    /// carries below its line — the one split a press and a double-click both read.
    function columnAt(item, position) {
        const x = position.x
        if (position.y - (item.y - pick.view.contentY) >= item.lineHeight)
            return pick.atBelow
        return x < item.clockEnd ? pick.atClock
             : x < item.cmdX ? pick.atGapCmd
             : x < item.cmdEnd ? pick.atCmd
             : x < item.outX ? pick.atGapOut
             : pick.atOut
    }
    /// Where one of the three drawn columns starts, in this item's coordinates.
    function columnX(item, at) {
        return at === pick.atClock ? item.clockX : at === pick.atCmd ? item.cmdX : item.outX
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
        if (!pick.takeAt(Qt.point(x, y)))
            return false
        // Straight up at the starting x — a sideways jump would let a run pass while the real gesture fails.
        const top = pick.view.itemAtIndex(0)
        pick.followAt(Qt.point(x, top ? top.y - pick.view.contentY + top.height / 2 : 1))
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
        onPressed: mouse => { mouse.accepted = pick.takeAt(Qt.point(mouse.x, mouse.y)) }
        onPositionChanged: mouse => pick.followAt(Qt.point(mouse.x, mouse.y))
        onReleased: pick.releaseText()
        onCanceled: pick.releaseText()
        onDoubleClicked: mouse => pick.wordAt(Qt.point(mouse.x, mouse.y))
    }

    // ---- the edge ----------------------------------------------------------------------------------------------
    /// How far the hand is past a belt inside the frame's top or bottom (`Metrics.selectPast`), which the browser's
    /// selection autoscroll sends the rows by — the log stands on the window's foot, and a window against the screen's
    /// foot leaves the hand no room past it. Zero between the belts, which stops the stepping below.
    readonly property real pastY: Metrics.selectPast(pick.handPosition.y, pick.height)
    // A drag that has taken text scrolls the log while the hand is in a belt, and re-reads the row every step: the
    // rows move under a still pointer without any mouse event. A step each animation frame
    // (rules-refs/app-ui.md「押さえた手の送りはフレームで進める」).
    FrameAnimation {
        id: edge
        running: pick.dragging && pick.selecting && pick.pastY !== 0
        onTriggered: {
            pick.scrollWanted(Metrics.selectSent(pick.pastY, edge.frameTime * 1000))
            const row = pick.rowAt(pick.handPosition.y)
            if (row >= 0)
                pick.dragText(row, pick.byteAt(row, pick.handPosition))
        }
    }
}
