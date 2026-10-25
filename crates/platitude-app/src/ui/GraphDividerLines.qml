pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The lines the two column boundaries raise, as tall as this is.
Item {
    id: lines

    /// Where the two boundaries stand and whether each wants its line (`GraphColumnDividers`).
    required property GraphColumnDividers dividers

    /// The lines as drawn, for the hooks — not their conditions, which would pass an unwired line.
    readonly property alias labelShown: labelLine.visible
    readonly property alias graphShown: graphLine.visible

    Rectangle {
        id: labelLine
        x: lines.dividers.labelX + Theme.borderWidth
        width: Theme.splitterWidth - 2 * Theme.borderWidth
        height: lines.height
        color: Theme.borderStrong
        visible: lines.dividers.labelLineWanted
    }
    Rectangle {
        id: graphLine
        x: lines.dividers.graphDividerX + Theme.borderWidth
        width: Theme.splitterWidth - 2 * Theme.borderWidth
        height: lines.height
        color: Theme.borderStrong
        visible: lines.dividers.graphLineWanted
    }
}
