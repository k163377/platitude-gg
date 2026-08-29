pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The hand that picks the diff's text out of its rows: a drag selects, a right-click asks for the menu, and a plain
// click puts the selection down (デザイン規約 §diff の中身をコピーする).
//
// **It covers the code column and not the gutter, and it stops short of the list's own bar.** The two numbers and the
// seat between them are the row's own — the `+` a line puts out there has to keep taking presses — so this starts
// where they end (`gutterW`); and the bar down the right edge is drawn over the rows, so a hand that ran to the frame
// took every press on the trough and the bar could not be grabbed at all (`barRoom`, 2026-08-29 ユーザー報告).
//
// **Inside that column every place nobody else takes is a start**, the ground a file shorter than the frame leaves
// under its last row included (`rowAt`, 規約 §diff の中身をコピーする). The hunk headings are the one exception, and
// they are one because something else is standing there.
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
    /// How much of the right edge belongs to the list's own scroll bar, which is where the code column ends
    /// (`AppListView.barRoom` — the same strip the graph's stand-in gives back, `GraphHeadPin.barRoom`).
    required property real barRoom
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
    width: Math.max(0, pick.view.width - pick.gutterW - pick.barRoom)
    height: pick.view.height

    /// Whether a drag is running. The press that started it is still held, so this and `hand.pressed` say the same
    /// thing — except during a right-click, which never starts one.
    property bool dragging: false
    /// Where the hand was last seen, in this item's own coordinates. Kept because the edge below re-reads it after
    /// every step it sends: the rows move under a pointer that is standing still.
    property real handX: 0
    property real handY: 0

    // ---- what the hand does, named so a headless run enters where it enters -------------------------------------
    /// A press at a point of this item, in its own coordinates: true where the text took it, false where it goes down
    /// to whatever is under this — the hunk's own two words, or the list itself. **The `MouseArea` below and a run
    /// enter here**, so a hand that was never wired up reports nothing (verify-ui §壊れない動詞の実装).
    function takeAt(x, y, button) {
        const row = pick.rowAt(y)
        if (row < 0 || !pick.takesPress(row))
            return false
        pick.handX = x
        pick.handY = y
        if (button === Qt.LeftButton)
            pick.pressText(row, pick.byteAt(row, x))
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
            pick.dragText(row, pick.byteAt(row, x))
    }
    /// A right-click that has been let go of, at a point of this item.
    function answerAt(x, y) {
        const row = pick.rowAt(y)
        if (row >= 0)
            pick.askMenu(row, pick.byteAt(row, x))
    }
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
    /// Which row a point of this item is over, or -1 where the list has no rows at all. **The point is clamped
    /// twice.** To the frame, so that a drag past either edge keeps naming the row it can still see while the edge
    /// below carries the rows to it — and to the rows themselves (`onRows`), so that the ground a file shorter than
    /// the frame leaves under its last row belongs to that row.
    function rowAt(y) {
        const inside = Math.max(0, Math.min(y, pick.height - 1))
        // One pixel in from the left: rows are as wide as the list, so any x inside it finds the same row.
        return pick.view.indexAt(1, pick.onRows(pick.view.contentY + inside))
    }
    /// A content y brought onto the band the rows themselves stand on — the whole of what makes the ground under the
    /// last row a place a selection can start (規約 §diff の中身をコピーする「掴めるのは、誰も取らない所すべて」).
    /// Without it a press there reached nothing at all, which is the same dead corner the right pane's values had
    /// (2026-08-28 ユーザー報告).
    ///
    /// `originY` rather than zero: a list of rows with differing heights moves its own origin once it has been sent
    /// to its end (app-ui.md, `CommandsPane.clampY`). An empty list has no band at all — the clamp then answers above
    /// its own last row, `indexAt` finds nothing, and the press goes down to the list as it always did.
    function onRows(y) {
        const first = pick.view.originY
        const last = first + pick.view.contentHeight - 1
        return Math.max(first, Math.min(y, last))
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

    // ---- the ground under the last row ---------------------------------------------------------------------------
    /// Where the ground a file shorter than the frame leaves under its last row begins, in this item's own
    /// coordinates, and whether there is any of it at all. A sweep run against a diff that fills its frame would
    /// prove nothing, so it says so rather than passing (verify-ui `diff-sweep`).
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
        if (!pick.takeAt(x, y, Qt.LeftButton))
            return false
        // Up into the text at the x it started from — a drag does not jump sideways, and reading a row out at a place
        // the reader never pressed is how a run passes while the gesture does not work.
        const top = pick.view.itemAtIndex(0)
        pick.followAt(x, top ? top.y - pick.view.contentY + top.height / 2 : 1)
        pick.releaseText()
        return true
    }
    /// Automation: the first row of the view that is a hunk's heading, or -1.
    function firstHunkRow() {
        for (let i = 0; i < pick.view.count; i++) {
            const item = pick.view.itemAtIndex(i)
            if (item && item.kind === "hunk")
                return i
        }
        return -1
    }
    /// Automation: whether the list's own bar is standing at all — the run's own honesty, since a diff that fits its
    /// frame has no bar and proves nothing about the strip under one.
    readonly property bool barOut: pick.barRoom > 0
    /// Automation: that strip, and whether this hand gives it back (`PG_AUTO_ACT=diff-bar`). A press cannot be
    /// injected, so what is read is the frame the press would land in — where this hand ends against where the bar
    /// begins — and the edge answering right beside it. The three that are judged stand together at the head, since
    /// `must_say` only takes words that are neighbours (`graph-head`, the same shape); the strip's own width follows
    /// as the diagnostic it is.
    function barTally() {
        const handRight = Math.round(pick.x + pick.width)
        const barLeft = Math.round(pick.view.x + pick.view.width - pick.barRoom)
        return "out=" + pick.barOut
             + " clear=" + (pick.barOut && handRight <= barLeft)
             + " reach=" + pick.pressAtRightEdge()
             + " strip=" + Math.round(pick.barRoom)
    }
    /// Automation: a press on the very last pixel this hand covers, on the first row that takes one. What lies beyond
    /// it is the bar's strip (`barRoom`), and this is the half that says the strip was given back to the bar and not
    /// eaten out of the code: an edge that answers is an edge that stops in the right place
    /// (`PG_AUTO_ACT=diff-bar`). Nothing is left standing — a press that never moved selected nothing.
    function pressAtRightEdge() {
        for (let i = 0; i < pick.view.count; i++) {
            const item = pick.view.itemAtIndex(i)
            if (!item || item.kind === "hunk")
                continue
            const took = pick.takeAt(pick.width - 1, item.y - pick.view.contentY + item.height / 2, Qt.LeftButton)
            pick.releaseText()
            return took
        }
        return false
    }
    /// Automation: the other half — a press on a hunk's heading is still the hunk's, ground or no ground. The two
    /// words there act on the hunk (規約 §diff の中のステージ), and this is what says their face kept every press.
    function pressOnHunk(row) {
        const item = pick.view.itemAtIndex(row)
        if (!item)
            return true
        const took = pick.takeAt(1, item.y - pick.view.contentY + item.height / 2, Qt.LeftButton)
        pick.releaseText()
        return took
    }

    // ---- the hand ----------------------------------------------------------------------------------------------
    MouseArea {
        id: hand
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        // Not hoverEnabled: the rows below keep their own (see the note at the top).
        cursorShape: Qt.IBeamCursor
        // The three above, and nothing beside them: what the hand does is written once, where a run enters it too.
        onPressed: mouse => { mouse.accepted = pick.takeAt(mouse.x, mouse.y, mouse.button) }
        onPositionChanged: mouse => pick.followAt(mouse.x, mouse.y)
        onReleased: pick.releaseText()
        onCanceled: pick.releaseText()
        onClicked: mouse => { if (mouse.button === Qt.RightButton) pick.answerAt(mouse.x, mouse.y) }
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
