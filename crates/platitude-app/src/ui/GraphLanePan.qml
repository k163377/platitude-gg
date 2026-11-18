pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The lane column's own presses: each is handed to the row under it at the press — `pressLanded` →
// `row.pickAtPress(modifiers)`, `onDoubleClicked` → `row.doubleClick(modifiers)`, every decision the row's
// (`GraphRowDelegate`) — and a left drag pans the lanes sideways. The row's node comes on only at a release that did
// not travel (`releaseLanded`): at the press it would slide the lanes out from under the hand that grabbed them. Only
// up while the lanes overflow their column (`graphXMax > 0` — nearly always on a real repository); headless
// `graph-reclick-lanes` squeezes the column to its floor to raise it. The lanes are the row's: only the chip column
// has a target of its own, and the column between the two boundaries is where a reader aims most when the graph is
// wide — a dead strip there reads as "sometimes it does not work".
MouseArea {
    id: pan

    /// The three columns' arithmetic (`GraphColumnMetrics`); the pan writes its `graphX`.
    required property var columns
    /// The list underneath, which says which row a press landed on.
    required property var view

    x: pan.columns.labelW
    width: pan.columns.graphColW
    height: parent.height
    z: 1
    visible: pan.columns.graphXMax > 0
    acceptedButtons: Qt.LeftButton
    // A row's name box reaches into the lane column once it outgrows its own, and a press on it is the box's — handed
    // to the row, it would close the box over what is being typed. A mask is the whole of the hit test, so it says the
    // strip's own rectangle too. Headless: `graph-rename-fit`'s `:/0:` run reads it in `hits=`.
    containmentMask: QtObject {
        function contains(point: point): bool {
            if (point.x < 0 || point.y < 0 || point.x > pan.width || point.y > pan.height)
                return false
            return !pan.onNameBox(point)
        }
    }
    property real pressX: 0
    property real startGX: 0
    property bool panning: false
    /// The node lane of the row the press that is down landed on, read at the press — the row the release brings on
    /// is the one picked, whatever the hand drifted onto since. -1 off the rows: the release goes to the footer, a
    /// button.
    property int pressLane: -1
    // Picked at the press, as on the row (`GraphRowDelegate.leftClick`): the strip holds the grab, so a drag down it
    // never cancels, and the release would pick the row the hand drifted onto.
    onPressed: mouse => {
        pan.pressX = mouse.x
        // The hand takes the lanes where they stand: a node's glide in flight stops under it.
        pan.columns.panTo(pan.columns.graphX)
        pan.startGX = pan.columns.graphX
        pan.panning = false
        pan.pressLanded(Qt.point(mouse.x, mouse.y), mouse.modifiers)
    }
    onPositionChanged: mouse => {
        if (!pan.pressed)
            return
        if (!pan.panning && Math.abs(mouse.x - pan.pressX) > Theme.spaceXs)
            pan.panning = true
        if (pan.panning)
            pan.columns.panTo(pan.startGX - (mouse.x - pan.pressX))
    }
    /// Automation (_PGG_AUTO_ACT=graph-reclick-lanes / graph-node-reveal): a click that did not travel at a point in
    /// this strip's own frame, entering the handlers' own bodies (`pressLanded`, then `releaseLanded`) so the
    /// point-to-row mapping is proven too. False where no row is hit.
    function clickAt(position, modifiers) {
        pan.panning = false
        if (!pan.pressLanded(position, modifiers === undefined ? Qt.NoModifier : modifiers))
            return false
        pan.releaseLanded(position)
        return true
    }
    /// The row a point of this strip is on, or null. Mapped: this strip starts at the pane's top and the list below
    /// the ask bar, so unmapped a standing question makes every press land rows lower.
    function rowAt(position) {
        const p = pan.mapToItem(pan.view, position)
        const idx = pan.view.indexAt(pan.columns.labelW + 1, pan.view.contentY + p.y)
        return idx < 0 ? null : pan.view.itemAtIndex(idx)
    }
    /// Whether a point of this strip is on the name box a row holds open.
    function onNameBox(position) {
        const row = pan.rowAt(position)
        if (!row || !row.naming)
            return false
        const p = pan.mapToItem(row, position)
        return p.x >= row.nameBoxX && p.x < row.nameBoxX + row.nameBoxWidth
    }
    // Handed straight to the row, which alone decides what a click and a second click mean (`GraphRowDelegate`) — a
    // partial copy here silently loses gestures on every repository wide enough to raise this strip.
    onDoubleClicked: mouse => {
        const row = pan.rowAt(Qt.point(mouse.x, mouse.y))
        if (row)
            row.doubleClick(mouse.modifiers)
    }
    onReleased: mouse => pan.releaseLanded(Qt.point(mouse.x, mouse.y))
    /// What a press at a point of this strip means, entered by the handler and by `clickAt`. Answers whether it
    /// landed on a row. The modifiers must travel: dropped, Ctrl / Shift become a plain press, which collapses the
    /// choice to one commit (`RepoPage.pickRow`, デザイン規約 §複数のコミットを選ぶ). Dropped, both OSes still go
    /// green — no headless run presses this strip with a modifier held.
    function pressLanded(position, modifiers) {
        const row = pan.rowAt(position)
        pan.pressLane = row ? row.node_lane : -1
        if (!row)
            return false
        row.pickAtPress(modifiers)
        return true
    }
    /// What the release of a press that did not travel means: on a row, its node comes on (`revealLane`); under the
    /// last row, the footer's. A press that travelled was a pan, aimed at neither.
    function releaseLanded(position) {
        if (pan.panning)
            return
        if (pan.pressLane >= 0)
            pan.columns.revealLane(pan.pressLane)
        else
            pan.tailPressed(position)
    }
    /// A press at a point of this strip under the last row, handed to the footer, which decides what it means
    /// (`GraphTailFooter.loadMore`). While the lanes overflow this strip covers the middle column, so without the
    /// hand-off the cut has a column that does nothing when pressed.
    function tailPressed(position) {
        const tail = pan.view.footerItem
        if (tail === null)
            return false
        const p = pan.mapToItem(tail, position)
        if (p.x < 0 || p.y < 0 || p.x > tail.width || p.y > tail.height)
            return false
        return tail.loadMore()
    }
}
