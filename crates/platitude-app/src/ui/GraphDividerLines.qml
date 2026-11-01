pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The lines the two column boundaries raise, as tall as this is. Drawn under the list for the pane, so the ticks and
// lanes stay in front of them, and again over each highlight ground a row or the stand-in wears and under its ink:
// the selection's ground and the stand-in's band would cut the pane's lines, and the pointer's and the question's
// washes would tint them. Each of those surfaces starts at the pane's left edge, so the boundaries' x is theirs as it
// stands.
//
// Every highlight ground a graph row or the stand-in wears carries one of these, over the ground and under the ink, so
// the line runs through in one colour: a new highlight ground gets one too. A row builds it in a `Loader` only while it
// wears a ground and a boundary has the hand — the reader's acts bring it and take it, so it is not kept built
// (automation reads `GraphRowDelegate.groundLines`). On the stand-in, over its two grounds only (`GraphHeadPin.lines`).
// **Not over the footer's hover ground**: it never lights while the lines are out — a hand on a boundary has the
// boundary's `MouseArea` over the footer taking hover, and during a drag Qt delivers no hover while a grabber holds
// (`QQuickDeliveryAgentPrivate::handleMouseEvent`, the `!exclusiveGrabber` branch). Headless `graph-divider` (the hand
// one row below the selection, `grounds=2 ruled=2`), `graph-divider pin --preset deep` (HEAD selected at start: the
// stand-in in the selection's ground), `graph-divider pin-lit --preset deep-detached` (the stand-in in the hover
// ground).
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
