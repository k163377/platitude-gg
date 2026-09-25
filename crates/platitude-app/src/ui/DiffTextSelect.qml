pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The hand that picks the diff's text out of its rows: a drag selects, a right-click asks for the menu, a plain click
// puts the selection down (デザイン規約 §diff の中身をコピーする).
//
// It covers the code: it starts after the first gutter, whose `+` keeps taking presses (`gutterW`), and stops short of
// the list's bar, which is drawn over the rows (`barRoom`, rules-refs/app-ui.md「一覧の上に面を重ねたら」). A drag is
// of one column — the one the press landed in (デザイン規約 §diff を 2 列で読む).
//
// **Laid over the list**, not in the rows: a `Flickable` takes the grab from its children past the drag threshold,
// and `reuseItems` rebuilds a scrolled-away row. A plain `MouseArea` with hover left to the rows — a `HoverHandler`
// here would take their `+` and lit hunk.
//
// Where a press landed in the file is the model's to say (`DiffModel::selection`); this knows pixels.
Item {
    id: pick

    required property var view
    required property var diffModel
    /// How wide a column's gutter is, which is where its code starts (`DiffTextMetrics.gutterW`).
    required property real gutterW
    /// Whether the rows are read as two columns, and how wide the left one is — the right starts a hairline after
    /// it (`DiffPane.halfW`).
    required property bool split
    required property real halfW
    /// The list's own scroll bar strip, where the code ends (`AppListView.barRoom`).
    required property real barRoom
    /// How far the code has been sent sideways (`DiffCodeScroll.offset`).
    required property real codeX
    /// The line's own layout, asked where the press landed (`LineRuler`) — the same ruler the wash is placed with, so
    /// the hit and the wash cannot disagree.
    required property var ruler

    /// A drag has reached past the frame and wants the rows (and the text) sent after it.
    signal scrollWanted(real dy, real dx)
    /// A right-click landed on the text; the selection is already what it should act on.
    signal menuWanted()

    x: pick.view.x + pick.gutterW
    y: pick.view.y
    width: Math.max(0, pick.view.width - pick.gutterW - pick.barRoom)
    height: pick.view.height

    /// Whether a drag is running — `hand.pressed`, except during a right-click, which never starts one.
    property bool dragging: false
    /// Which column the drag is of — the one the press landed in (`sideAt`).
    property int side: 0
    /// Where the hand was last seen, in this item's coordinates, for the edge below.
    property real handX: 0
    property real handY: 0

    // ---- columns ---------------------------------------------------------------------------------------------------
    /// The hairline between the two columns, in this item's coordinates (the first gutter is left of this item).
    readonly property real divider: pick.halfW - pick.gutterW
    /// 0 for the rows' own lines — the only column as one — and 1 for the right of a split row.
    function sideAt(x) {
        return pick.split && x >= pick.divider ? 1 : 0
    }
    /// Where a column's code starts, in this item's coordinates: the second's is past the hairline and its gutter.
    function codeStart(side) {
        return side === 1 ? pick.divider + Theme.borderWidth + pick.gutterW : 0
    }

    // ---- what the hand does, named so a headless run enters where it enters -------------------------------------
    /// A press at a point of this item: true where the text took it, false where it goes down to what is under this —
    /// a gutter's mark, the hunk's two words, or the list. **The `MouseArea` below and a run enter here**
    /// (verify-ui §壊れない動詞の実装と反復).
    function takeAt(x, y, button) {
        const row = pick.rowAt(y)
        if (row < 0 || !pick.takesPress(row))
            return false
        const side = pick.sideAt(x)
        // The second column's gutter is the row's, as the first's is.
        if (x < pick.codeStart(side))
            return false
        pick.handX = x
        pick.handY = y
        if (button === Qt.LeftButton)
            pick.pressText(side, row, pick.byteAt(side, row, x - pick.codeStart(side)))
        return true
    }
    /// A drag that has left the rows keeps the one it can still see (`rowAt`), and one that has left its column
    /// keeps that too.
    function followAt(x, y) {
        pick.handX = x
        pick.handY = y
        if (!pick.dragging)
            return
        const row = pick.rowAt(y)
        if (row >= 0)
            pick.dragText(row, pick.byteAt(pick.side, row, x - pick.codeStart(pick.side)))
    }
    /// A right-click that has been let go of, at a point of this item.
    function answerAt(x, y) {
        const row = pick.rowAt(y)
        const side = pick.sideAt(x)
        if (row >= 0)
            pick.askMenu(side, row, pick.byteAt(side, row, x - pick.codeStart(side)))
    }
    /// The selection starts here and holds nothing until the hand moves.
    function pressText(side, row, at) {
        pick.side = side
        pick.diffModel.beginSelect(side, row, at)
        pick.dragging = true
    }
    function dragText(row, at) {
        pick.diffModel.extendSelect(pick.side, row, at)
    }
    /// The button is up. A press that never moved is cleared, which keeps `selActive` honest.
    function releaseText() {
        pick.dragging = false
        if (!pick.diffModel.selHasNew && pick.diffModel.selRemoved === 0)
            pick.diffModel.clearSelect()
    }
    /// Inside the selection it stays; outside it — the other column included — the row underneath becomes the
    /// selection (デザイン規約 §diff の中身をコピーする).
    function askMenu(side, row, at) {
        if (!pick.diffModel.selectionHolds(side, row, at)) {
            pick.side = side
            pick.diffModel.selectRow(side, row)
        }
        pick.menuWanted()
    }

    // ---- pixels to rows ----------------------------------------------------------------------------------------
    /// Which row a point of this item is over, or -1 where the list has no rows. **Clamped twice**: to the frame, so a
    /// drag past either edge keeps naming the row it can still see, and to the rows (`onRows`), so the ground under
    /// the last row belongs to that row (rules-refs/app-ui.md「本文の面の掴み代は」).
    function rowAt(y) {
        const inside = Math.max(0, Math.min(y, pick.height - 1))
        // One pixel in from the left: rows are as wide as the list, so any x inside it finds the same row.
        return pick.view.indexAt(1, pick.onRows(pick.view.contentY + inside))
    }
    /// A content y brought onto the band the rows stand on, from `originY`, which a list of rows of differing heights
    /// moves (`AppListView.clampY`). An empty list has no band: `indexAt` finds nothing and the press goes down to the
    /// list.
    function onRows(y) {
        const first = pick.view.originY
        const last = first + pick.view.contentHeight - 1
        return Math.max(first, Math.min(y, last))
    }
    /// Which byte of one side of that row's line an x of its column lands on: the row's layout names the place
    /// (`LineRuler`), the model the byte (`DiffModel.sourceByteAt`). Past the line's end the layout answers the end,
    /// and left of the column the head. A heading has no place to land on.
    function byteAt(side, row, x) {
        const item = pick.view.itemAtIndex(row)
        if (!item || item.banded)
            return 0
        return pick.diffModel.sourceByteAt(side, row,
                                           pick.ruler.placeAt(item.textOf(side), item.boldOf(side),
                                                              pick.codeX + Math.max(0, x)))
    }
    /// Not a hunk heading: the press there belongs to its two words (デザイン規約 §diff の中身をコピーする).
    function takesPress(row) {
        const item = pick.view.itemAtIndex(row)
        return !!item && item.kind !== "hunk"
    }

    // ---- the ground under the last row ---------------------------------------------------------------------------
    /// Where the ground a file shorter than the frame leaves under its last row begins, in this item's coordinates,
    /// and whether there is any — a sweep against a diff that fills its frame proves nothing (verify-ui `diff-sweep`).
    readonly property real groundTop: Math.max(0, pick.view.originY + pick.view.contentHeight - pick.view.contentY)
    readonly property bool hasGround: pick.groundTop < pick.height - 2
    /// Automation: a press on that ground and a drag up into the text, through the functions the `MouseArea` calls.
    /// It **starts in the ground**: begun on a row it would go green with the ground taken back out (`SweepRoom`).
    /// Across the first column's code.
    function sweepFromGround(fx, fy) {
        if (!pick.hasGround)
            return false
        const to = pick.split ? pick.divider : pick.width
        const x = Math.max(1, Math.min(to - 1, to * fx))
        const y = pick.groundTop + 1 + (pick.height - pick.groundTop - 2) * fy
        if (!pick.takeAt(x, y, Qt.LeftButton))
            return false
        // Up at the x it started from — reading a row out where the reader never pressed passes a broken gesture.
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
    /// Automation: whether the list's own bar is standing — a diff that fits its frame proves nothing about the strip.
    readonly property bool barOut: pick.barRoom > 0
    /// Automation: whether this hand gives the bar's strip back (`PGG_AUTO_ACT=diff-bar`) — where the hand ends
    /// against where the bar begins, and the edge answering beside it. The three judged words lead, since `must_say`
    /// only takes neighbours; the strip's width follows as a diagnostic.
    function barTally() {
        const handRight = Math.round(pick.x + pick.width)
        const barLeft = Math.round(pick.view.x + pick.view.width - pick.barRoom)
        return "out=" + pick.barOut
             + " clear=" + (pick.barOut && handRight <= barLeft)
             + " reach=" + pick.pressAtRightEdge()
             + " strip=" + Math.round(pick.barRoom)
    }
    /// Automation: a press on the last pixel this hand covers, on the first row that takes one — an edge that
    /// answers stops in the right place (`PGG_AUTO_ACT=diff-bar`). A press that never moved leaves nothing selected.
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
    /// Automation: whether a press on a hunk's heading is taken — it must not be (`takesPress`).
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
        cursorShape: Qt.IBeamCursor
        onPressed: mouse => { mouse.accepted = pick.takeAt(mouse.x, mouse.y, mouse.button) }
        onPositionChanged: mouse => pick.followAt(mouse.x, mouse.y)
        onReleased: pick.releaseText()
        onCanceled: pick.releaseText()
        onClicked: mouse => { if (mouse.button === Qt.RightButton) pick.answerAt(mouse.x, mouse.y) }
    }
    // The second column's gutter keeps the arrow: a press there is the row's (`takeAt`), and a beam over the `+` reads
    // as selectable text. No button and no hover, so presses and hover go through. Built only side by side.
    Loader {
        active: pick.split
        x: pick.divider + Theme.borderWidth
        width: pick.gutterW
        height: pick.height
        sourceComponent: MouseArea {
            acceptedButtons: Qt.NoButton
            cursorShape: Qt.ArrowCursor
        }
    }

    // ---- the edge ----------------------------------------------------------------------------------------------
    /// How far past the frame the hand has reached, on each axis; zero inside, which stops the ticker below.
    readonly property real pastY: pick.handY < 0 ? pick.handY
                                  : pick.handY > pick.height ? pick.handY - pick.height : 0
    readonly property real pastX: pick.handX < 0 ? pick.handX
                                  : pick.handX > pick.width ? pick.handX - pick.width : 0
    // A drag past the frame carries the diff after it at the middle-click hand's gain, re-reading the row on every
    // step: the rows move under a pointer standing still.
    Timer {
        running: pick.dragging && (pick.pastY !== 0 || pick.pastX !== 0)
        // A frame, the same tick `MiddleAutoScroll` drifts on.
        interval: 16
        repeat: true
        onTriggered: {
            pick.scrollWanted(pick.pastY * Metrics.middleScrollGain, pick.pastX * Metrics.middleScrollGain)
            const row = pick.rowAt(pick.handY)
            if (row >= 0)
                pick.dragText(row, pick.byteAt(pick.side, row, pick.handX - pick.codeStart(pick.side)))
        }
    }
}
