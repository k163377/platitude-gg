pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The lane column's own presses: a left drag sends the lanes sideways when
// they overflow their column, and a press that did not travel still means
// what it would have meant on the row itself.
//
// Only up while there is somewhere sideways to go — with the whole graph
// inside its column this strip would take the rows' presses for nothing.
MouseArea {
    id: pan

    /// The three columns' arithmetic (`GraphColumnMetrics`): where the
    /// lanes start, how wide they are drawn, and how far they have been
    /// sent — this writes that last one.
    required property var columns
    /// The list underneath: which row a press landed on is its answer.
    required property var view
    required property var graphModel

    x: pan.columns.labelW
    width: pan.columns.graphColW
    height: parent.height
    z: 1
    visible: pan.columns.graphXMax > 0
    acceptedButtons: Qt.LeftButton
    property real pressX: 0
    property real startGX: 0
    property bool panning: false
    onPressed: mouse => {
        pan.pressX = mouse.x
        pan.startGX = pan.columns.graphX
        pan.panning = false
    }
    onPositionChanged: mouse => {
        if (!pan.pressed)
            return
        if (!pan.panning && Math.abs(mouse.x - pan.pressX) > Theme.spaceXs)
            pan.panning = true
        if (pan.panning)
            pan.columns.graphX = Math.max(0, Math.min(
                pan.startGX - (mouse.x - pan.pressX), pan.columns.graphXMax))
    }
    // The lanes are part of the row, so a double-click on them means
    // what it means anywhere else on it. Without this the gesture
    // would die in exactly the repositories wide enough to need
    // panning, and nothing on screen would say why.
    onDoubleClicked: mouse => {
        // mouse.y is in this MouseArea's frame, which starts at the
        // pane's top; the list starts below the ask bar. Map, or a
        // standing question makes every lane click land rows lower.
        const p = pan.mapToItem(pan.view, mouse.x, mouse.y)
        const idx = pan.view.indexAt(pan.columns.labelW + 1,
                                     pan.view.contentY + p.y)
        if (idx < 0)
            return
        // Asked of the row itself, so which chip a row leads to is
        // worked out in exactly one place.
        const row = pan.view.itemAtIndex(idx)
        if (row && row.movable)
            pan.view.rowSwitchRequested(pan.graphModel.oidAt(idx),
                                        row.primaryRecord)
    }
    onReleased: mouse => {
        if (pan.panning)
            return
        // Same frame correction as the double-click above.
        const p = pan.mapToItem(pan.view, mouse.x, mouse.y)
        const idx = pan.view.indexAt(pan.columns.labelW + 1,
                                     pan.view.contentY + p.y)
        if (idx >= 0) {
            // Same as a click on the row itself: the lanes are part
            // of the row, so a press that lands on them leaves the
            // keyboard here too (規約 §矢印で履歴を辿る).
            pan.view.takeKeyboard()
            pan.view.currentIndex = idx
            pan.view.rowSelected(pan.graphModel.oidAt(idx))
        }
    }
}
