pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// How far sideways the diff's code has been sent, and the hands that send
// it: the bar along the view's bottom edge and the middle-click autoscroll,
// which carries the rows up and down at the same time
// (デザイン規約 §diff を横へ送る).
//
// Declared with `parent:` pointing at the view itself: a child of a
// Flickable is otherwise adopted by the content and scrolls away with the
// rows (app-ui.md).
Item {
    id: codeScroll

    /// The list whose rows carry this offset, and which the hand also
    /// sends up and down.
    required property var view
    /// How wide the code is and how much of it fits — the pane owns both,
    /// since it owns the gutter that eats the difference.
    required property real codeWidth
    required property real roomWidth
    /// The file on screen. A different one starts at its own left edge;
    /// the same one read again — which is what every partial write ends
    /// with — keeps the place it was being read from, the way the vertical
    /// place is kept (`DiffScrollPlace`).
    required property string file

    /// How far the code has been sent, and how far it can go.
    property real offset: 0
    readonly property real maxOffset:
        Math.max(0, codeScroll.codeWidth - codeScroll.roomWidth)
    readonly property bool canPan: codeScroll.maxOffset > 0

    /// Sends the code `dx` further, as far as it goes. The bar, the wheel
    /// and the hand all arrive here, so one clamp holds for all of them.
    function shift(dx) {
        codeScroll.offset = Math.max(
            0, Math.min(codeScroll.offset + dx, codeScroll.maxOffset))
    }
    onFileChanged: codeScroll.offset = 0
    // A window that grew, or a shorter file: the place being read can end
    // up past the end of what there is to read.
    onMaxOffsetChanged: codeScroll.shift(0)

    /// Whether the pointer is anywhere in the view — the bar lies over the
    /// last row, so it comes out only while someone is here to use it, and
    /// stays out for as long as it is being dragged (デザイン規約
    /// §グラフを横へ送る, the same bar in the same seat).
    HoverHandler {
        id: pointer
    }

    // The hand: middle-click autoscroll, both ways at once. Sideways is
    // offered everywhere in this pane — unlike the graph, where only the
    // lanes column goes sideways, a diff is one column of text throughout.
    MiddleAutoScroll {
        id: hand
        anchors.fill: parent
        panFrom: 0
        panTo: codeScroll.width
        canPan: codeScroll.canPan
        onDrifted: (dy, dx) => {
            codeScroll.view.contentY = codeScroll.view.clampY(
                codeScroll.view.contentY + dy)
            if (dx !== 0)
                codeScroll.shift(dx)
        }
    }
    /// Automation: the hand, started and drifted without a pointer — and
    /// what it is doing, for the run to read back.
    function startHand(x, y) { hand.start(x, y) }
    function driftHand(x, y) { hand.drift(x, y) }
    readonly property alias handScrolling: hand.scrolling
    readonly property alias barShown: bar.visible

    // Fusion draws a bar's handle only in the style's "active" state,
    // which for one not attached to a Flickable means while the pointer is
    // on the bar itself — a strip on the bottom edge nobody would find.
    // When this bar is out it is because this pane put it there, so the
    // style stops deciding (GraphPane's lane bar, same reason).
    ScrollBar {
        id: bar
        visible: codeScroll.canPan
                 && (pointer.hovered || pressed || hand.scrolling)
        policy: ScrollBar.AlwaysOn
        orientation: Qt.Horizontal
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        z: 2
        size: codeScroll.codeWidth > 0
              ? codeScroll.roomWidth / codeScroll.codeWidth : 1
        position: codeScroll.codeWidth > 0
                  ? codeScroll.offset / codeScroll.codeWidth : 0
        onPositionChanged: {
            if (pressed)
                codeScroll.offset = Math.max(0, Math.min(
                    position * codeScroll.codeWidth, codeScroll.maxOffset))
        }
    }
}
