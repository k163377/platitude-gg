pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The lane column's own presses: a left drag pans the lanes sideways, and a press that did not travel is handed to the
// row under it (rules-refs/app-ui.md「レーン列は行の一部で」). Only up while the lanes overflow their column.
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
    property real pressX: 0
    property real startGX: 0
    property bool panning: false
    /// Whether the press that is down landed on a row; if not, its release goes to the footer, a button.
    property bool pressOnRow: false
    // Answered at the press, as on the row (`GraphRowDelegate.leftClick`): the strip holds the grab, so a drag down it
    // never cancels, and the release would pick the row the hand drifted onto.
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
    /// Automation (PGG_AUTO_ACT=graph-reclick-lanes): a press at a point in this strip's own frame, entering the
    /// handler's own body (`pressLanded`) so the point-to-row mapping is proven too. False where no row is hit.
    function clickAt(x, y, modifiers) {
        return pan.pressLanded({ "x": x, "y": y },
                               modifiers === undefined ? Qt.NoModifier : modifiers)
    }
    /// The row a press on this strip landed on, or null. Mapped: this strip starts at the pane's top and the list below
    /// the ask bar, so unmapped a standing question makes every press land rows lower.
    function rowAt(mouse) {
        const p = pan.mapToItem(pan.view, mouse.x, mouse.y)
        const idx = pan.view.indexAt(pan.columns.labelW + 1, pan.view.contentY + p.y)
        return idx < 0 ? null : pan.view.itemAtIndex(idx)
    }
    // Handed straight to the row, which alone decides what a click and a second click mean (`GraphRowDelegate`) — a
    // partial copy here silently loses gestures on every repository wide enough to raise this strip.
    onDoubleClicked: mouse => {
        const row = pan.rowAt(mouse)
        if (row)
            row.doubleClick(mouse.modifiers)
    }
    // The release is left for the footer; a press that travelled was a pan, not aimed at it.
    onReleased: mouse => {
        if (pan.panning || pan.pressOnRow)
            return
        pan.tailPressed(mouse)
    }
    /// What a press on this strip means, entered by the handler and by `clickAt`. Answers whether it landed on a row.
    /// The modifiers must travel: dropped, Ctrl / Shift become a plain press, which collapses the choice to one commit
    /// (`RepoPage.pickRow`, デザイン規約 §複数のコミットを選ぶ).
    function pressLanded(at, modifiers) {
        const row = pan.rowAt(at)
        if (!row)
            return false
        row.leftClick(modifiers)
        return true
    }
    /// A press under the last row, handed to the footer, which decides what it means (`GraphTailFooter.loadMore`).
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
