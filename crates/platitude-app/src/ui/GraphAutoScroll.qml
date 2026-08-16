pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// Middle-click toggles autoscroll mode: the pointer distance from
// the anchor sets the speed; any click exits. What the drift means —
// which list moves, and how far it may go — is the pane's (`drifted`).
Item {
    id: autoScroll

    /// Where the lanes column begins and ends in this frame. A gesture
    /// that starts between the two carries the lanes sideways as well.
    required property real laneFrom
    required property real laneTo
    /// Whether there is anywhere sideways to go at all.
    required property bool canPan

    property bool scrolling: false
    property real anchorX: 0
    property real anchorY: 0
    property real currentX: 0
    property real currentY: 0
    /// Whether this autoscroll carries the lanes sideways as well. Decided
    /// by where the middle click landed, and kept for the whole gesture
    /// (デザイン規約 §グラフを横へ送る).
    property bool panning: false

    /// How far the drift has travelled since the last tick. `dx` is 0
    /// unless this gesture pans and there is room to pan in.
    signal drifted(real dy, real dx)

    /// Starts autoscroll from a point in the pane's frame. The press and
    /// the automation hook both come through here, so which column offers
    /// the sideways drift is answered in exactly one place.
    function start(x, y) {
        autoScroll.anchorX = x
        autoScroll.anchorY = y
        autoScroll.currentX = x
        autoScroll.currentY = y
        autoScroll.panning = x >= autoScroll.laneFrom && x < autoScroll.laneTo
        autoScroll.scrolling = true
    }
    /// Where the pointer has drifted to since. Its distance from the
    /// anchor is what the ticker below reads as speed — the moving
    /// pointer and the automation hook write the same two values.
    function drift(x, y) {
        autoScroll.currentX = x
        autoScroll.currentY = y
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.MiddleButton
        onClicked: mouse => autoScroll.start(mouse.x, mouse.y)
    }
    MouseArea {
        visible: autoScroll.scrolling
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.AllButtons
        // The cursor says which ways this gesture goes, so a press that
        // did not land on the lanes does not look broken when the lanes
        // stay put under a sideways drift.
        cursorShape: autoScroll.panning ? Qt.SizeAllCursor : Qt.SizeVerCursor
        onPositionChanged: mouse => autoScroll.drift(mouse.x, mouse.y)
        onPressed: mouse => {
            autoScroll.scrolling = false
            mouse.accepted = true
        }
        Timer {
            running: autoScroll.scrolling
            interval: 16
            repeat: true
            onTriggered: {
                const dy = (autoScroll.currentY - autoScroll.anchorY)
                         * Metrics.middleScrollGain
                const dx = autoScroll.panning && autoScroll.canPan
                         ? (autoScroll.currentX - autoScroll.anchorX)
                           * Metrics.middleScrollGain
                         : 0
                autoScroll.drifted(dy, dx)
            }
        }
        Rectangle {
            x: autoScroll.anchorX - Theme.iconMd / 2
            y: autoScroll.anchorY - Theme.iconMd / 2
            width: Theme.iconMd
            height: Theme.iconMd
            radius: Theme.iconMd / 2
            color: "transparent"
            border.color: Theme.borderStrong
            border.width: Theme.borderWidth
            Rectangle {
                anchors.centerIn: parent
                width: Theme.spaceXs
                height: Theme.spaceXs
                radius: Theme.spaceXs / 2
                color: Theme.borderStrong
            }
        }
    }
}
