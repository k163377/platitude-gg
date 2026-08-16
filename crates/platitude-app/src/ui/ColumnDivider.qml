pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// One draggable boundary between two of the graph's columns: a strip a
// splitter wide that says where the pointer is on it and how far a drag
// has carried it. The pane owns the widths — this only reports.
MouseArea {
    id: divider

    /// The item a drag's landing place is reported in: the pane whose
    /// columns this boundary sits between.
    required property Item frame

    /// The pointer arrived on the boundary, or left it.
    signal pointedInto(bool inside)
    /// Where the pointer is on it, in **scene coordinates** — the badge a
    /// refused drag wears is drawn a frame up from the pane, by the page.
    signal pointedAt(point at)
    /// A drag asked for this much, in `frame`'s coordinates. Unclamped:
    /// what the hand wanted is the only thing that can tell a refusal
    /// from a rest.
    signal dragged(real x)

    width: Theme.splitterWidth
    height: parent.height
    z: 2
    hoverEnabled: true
    // The cursor never changes: it is the platform's splitter shape
    // in both states, and a column that will not move says so with
    // the badge instead (規約 §グラフ列は最も広い所のレーンまで).
    // Swapping in a drawn arrow made the refusal read as a
    // different tool from the divider one column over.
    cursorShape: Qt.SplitHCursor
    preventStealing: true
    onContainsMouseChanged: {
        divider.pointedInto(divider.containsMouse)
        // Entering does not always bring a move with it, and the
        // mark is drawn where this says the hand is.
        if (divider.containsMouse)
            divider.pointedAt(divider.mapToItem(null, divider.mouseX,
                                                divider.mouseY))
    }
    onPositionChanged: mouse => {
        divider.pointedAt(divider.mapToItem(null, mouse.x, mouse.y))
        if (divider.pressed)
            divider.dragged(divider.mapToItem(divider.frame, mouse.x, 0).x)
    }
}
