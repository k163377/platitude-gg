pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// One draggable boundary between two of the graph's columns. The pane owns the widths — this only reports.
MouseArea {
    id: divider

    /// The pane whose columns this boundary sits between; drags are reported in its coordinates.
    required property Item frame

    /// The pointer arrived on the boundary, or left it.
    signal pointedInto(bool inside)
    /// Where the pointer is, in scene coordinates — the page draws the refusal badge a frame up from the pane.
    signal pointedAt(point at)
    /// A drag asked for this much, in `frame`'s coordinates. Unclamped: only what the hand wanted tells a refusal from
    /// a rest.
    signal dragged(real x)

    width: Theme.splitterWidth
    height: parent.height
    z: 2
    hoverEnabled: true
    // The splitter cursor in both states; a column that will not move says so with the badge
    // (規約 §グラフ列は最も広い所のレーンまで).
    cursorShape: Qt.SplitHCursor
    preventStealing: true
    onContainsMouseChanged: {
        divider.pointedInto(divider.containsMouse)
        // Entering does not always bring a move with it.
        if (divider.containsMouse)
            divider.pointedAt(divider.mapToItem(null, divider.mouseX, divider.mouseY))
    }
    onPositionChanged: mouse => {
        divider.pointedAt(divider.mapToItem(null, mouse.x, mouse.y))
        if (divider.pressed)
            divider.dragged(divider.mapToItem(divider.frame, mouse.x, 0).x)
    }
}
