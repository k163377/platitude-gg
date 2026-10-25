pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The lines the two column boundaries raise, as tall as this is. Drawn under the list for the pane, so the ticks and
// lanes stay in front of them, and again over each highlight ground a row or the stand-in wears and under its ink:
// the selection's ground and the stand-in's band would cut the pane's lines, and the pointer's and the question's
// washes would tint them. Each of those surfaces starts at the pane's left edge, so the boundaries' x is theirs as it
// stands.
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
