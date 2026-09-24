pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The lane column's own presses: a left drag sends the lanes sideways when they overflow their column, and a press that
// did not travel still means what it would have meant on the row itself.
//
// Only up while there is somewhere sideways to go — with the whole graph inside its column this strip would take the
// rows' presses for nothing.
MouseArea {
    id: pan

    /// The three columns' arithmetic (`GraphColumnMetrics`): where the lanes start, how wide they are drawn, and how
    /// far they have been sent — this writes that last one.
    required property var columns
    /// The list underneath: which row a press landed on is its answer, and the row itself is what answers it.
    required property var view

    x: pan.columns.labelW
    width: pan.columns.graphColW
    height: parent.height
    z: 1
    visible: pan.columns.graphXMax > 0
    acceptedButtons: Qt.LeftButton
    property real pressX: 0
    property real startGX: 0
    property bool panning: false
    /// Whether the press that is down landed on a row. The footer below the last row is the other thing under this
    /// strip, and it is a button: it waits for the release the way every button does.
    property bool pressOnRow: false
    // **The row is answered at the press**, as it is everywhere else along it (`GraphRowDelegate.leftClick`). Reading
    // it off the release instead picked the row the hand had drifted onto, not the one it had pressed — the strip
    // takes the grab, so a drag down it never cancelled the way a press on the rows themselves does.
    onPressed: mouse => {
        pan.pressX = mouse.x
        pan.startGX = pan.columns.graphX
        pan.panning = false
        pan.pressOnRow = pan.pressLanded(mouse, mouse.modifiers)
    }
    onPositionChanged: mouse => {
        if (!pan.pressed)
            return
        if (!pan.panning && Math.abs(mouse.x - pan.pressX) > Theme.spaceXs)
            pan.panning = true
        if (pan.panning)
            pan.columns.graphX = Math.max(0, Math.min(pan.startGX - (mouse.x - pan.pressX), pan.columns.graphXMax))
    }
    /// A press on this strip, as a run with no pointer to press with puts one in — at a point in this
    /// strip's own frame, so **the mapping from the point to a row is the strip's own**, which is the half worth
    /// proving (PGG_AUTO_ACT=graph-reclick-lanes). Answers false where the point is on no row.
    ///
    /// **It goes in at the handler's own body**: a hook that did what the handler would have done
    /// stays green while the handler does something else, which is exactly how the gap this proves survived a run.
    function clickAt(x, y, modifiers) {
        return pan.pressLanded({ "x": x, "y": y },
                               modifiers === undefined ? Qt.NoModifier : modifiers)
    }
    /// The row a press on this strip landed on, or null.
    ///
    /// `mouse.y` is in this MouseArea's frame, which starts at the pane's top; the list starts below the ask bar. Map,
    /// or a standing question makes every lane press land rows lower.
    function rowAt(mouse) {
        const p = pan.mapToItem(pan.view, mouse.x, mouse.y)
        const idx = pan.view.indexAt(pan.columns.labelW + 1, pan.view.contentY + p.y)
        return idx < 0 ? null : pan.view.itemAtIndex(idx)
    }
    // **The lanes are part of the row, so both gestures are the row's own.** Handed straight to the row:
    // where a row leads and what a second click on it means are decided in one place
    // (`GraphRowDelegate`), and this strip covers the whole of the column between the two dividers — the half of the
    // row a reader is most likely to aim at when the graph is wide. Answering it with a copy of half of what a click
    // does is how the name gesture came to do nothing there, in exactly the repositories wide enough to raise this
    // strip.
    onDoubleClicked: mouse => {
        const row = pan.rowAt(mouse)
        if (row)
            row.doubleClick(mouse.modifiers)
    }
    // What is left for the release is the footer, which is a button and acts on one. A press that travelled sideways
    // was a pan, and the footer is not what it was aimed at.
    onReleased: mouse => {
        if (pan.panning || pan.pressOnRow)
            return
        pan.tailPressed(mouse)
    }
    /// What a press on this strip means. One line in the handler above, so the run and the hand go in at the
    /// same place (`clickAt`). Answers whether it landed on a row.
    ///
    /// **The modifiers travel with it.** Dropped here, Ctrl and Shift came out as a plain press — and a plain press
    /// puts the whole choice back down to the one commit under it (`RepoPage.pickRow`), so building a choice died on
    /// the half of the row a wide repository makes this strip (デザイン規約 §複数のコミットを選ぶ).
    function pressLanded(at, modifiers) {
        const row = pan.rowAt(at)
        if (!row)
            return false
        row.leftClick(modifiers)
        return true
    }
    /// A press under the last row. The only thing down there is the window's own footer, and it takes presses
    /// (`GraphTailFooter.loadMore`) — handed over, for the reason the rows' gestures are:
    /// what a press on the cut means is decided in one place.
    function tailPressed(at) {
        const tail = pan.view.footerItem
        if (tail === null)
            return false
        const p = pan.mapToItem(tail, at.x, at.y)
        if (p.x < 0 || p.y < 0 || p.x > tail.width || p.y > tail.height)
            return false
        return tail.loadMore()
    }
}
