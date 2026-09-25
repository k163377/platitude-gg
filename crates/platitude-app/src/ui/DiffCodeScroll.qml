pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// How far sideways the diff's code has been sent, and the hands that send it: the bar under the rows and the
// middle-click autoscroll, which also carries the rows up and down (デザイン規約 §diff を横へ送る).
//
// Declared beside the pane's body, not over it: a `HoverHandler` on an overlay kills the rows' hover (so "pointer in
// the pane" is asked of the pane's own handler), and a bar inside the list loses its drag to the list's filtering.
Item {
    id: codeScroll

    /// The list: its frame seats the hand and the bar, and the hand carries its `contentY`.
    required property var view
    /// How wide the code is and how much of it fits — the pane's numbers.
    required property real codeWidth
    required property real roomWidth
    /// Whether the pointer is anywhere in the pane (the pane's own handler): the bar is out only then (デザイン規約
    /// §diff を横へ送る), and dims by it too, having no view to ask (`AutoScrollBar.inArea`).
    required property bool paneHovered
    /// The file on screen: a different one starts at its left edge, the same one read again keeps its place, as the
    /// vertical place is kept (`DiffScrollPlace`).
    required property string file

    /// How far the code has been sent. The reach is handed in (`codeWidth`, from `DiffReach`); the clamp is this
    /// component's.
    property real offset: 0
    readonly property real maxOffset: Math.max(0, codeScroll.codeWidth - codeScroll.roomWidth)
    readonly property bool canPan: codeScroll.maxOffset > 0

    /// The strip below the rows the bar stands on, given back when there is nowhere to send (デザイン規約 §diff を横へ送る).
    /// Read off the bar: no token says how tall it is.
    readonly property real barRoom: codeScroll.canPan ? bar.height : 0

    /// Sends the code `dx` further, clamped — the wheel and the hand come through here.
    function shift(dx) {
        codeScroll.offset = Math.max(0, Math.min(codeScroll.offset + dx, codeScroll.maxOffset))
    }
    onFileChanged: codeScroll.offset = 0
    // A window that grew or a shorter reading can leave the place past the end.
    onMaxOffsetChanged: codeScroll.shift(0)

    /// Automation: the hand, driven without a pointer (a middle button cannot be injected), and what it and the bar
    /// are doing.
    function startHand(x, y) { hand.start(x, y) }
    function driftHand(x, y) { hand.drift(x, y) }
    readonly property alias handScrolling: hand.scrolling
    readonly property alias barShown: bar.visible

    // The rows' frame, off the list — whose x/y are already in these coordinates (the pane's body fills this parent).
    Item {
        id: frame
        x: codeScroll.view.x
        y: codeScroll.view.y
        width: codeScroll.view.width
        height: codeScroll.view.height

        // Middle-click autoscroll, both ways, sideways anywhere in the pane (デザイン規約 §diff を横へ送る).
        MiddleAutoScroll {
            id: hand
            anchors.fill: parent
            panFrom: 0
            panTo: frame.width
            canPan: codeScroll.canPan
            onDrifted: (dy, dx) => {
                codeScroll.view.contentY = codeScroll.view.clampY(codeScroll.view.contentY + dy)
                if (dx !== 0)
                    codeScroll.shift(dx)
            }
        }
        // `AlwaysOn`: Fusion shows an unattached bar's handle only under the pointer, and here the pane decides when it
        // is out. It stands in the `barRoom` strip below the rows (rules-refs/app-ui.md「diff は横へ送る・溝は動かない」).
        AutoScrollBar {
            id: bar
            inArea: codeScroll.paneHovered
            visible: codeScroll.canPan && (codeScroll.paneHovered || pressed || hand.scrolling)
            policy: ScrollBar.AlwaysOn
            orientation: Qt.Horizontal
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.bottom
            size: codeScroll.codeWidth > 0 ? codeScroll.roomWidth / codeScroll.codeWidth : 1
            position: codeScroll.codeWidth > 0 ? codeScroll.offset / codeScroll.codeWidth : 0
            onPositionChanged: {
                // Every send — thumb, wheel or hand — moves `position`, which is what lights the bar.
                bar.moved()
                if (pressed)
                    codeScroll.offset = Math.max(0, Math.min(position * codeScroll.codeWidth, codeScroll.maxOffset))
            }
        }
    }
}
