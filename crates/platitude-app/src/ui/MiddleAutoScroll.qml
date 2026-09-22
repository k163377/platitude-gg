pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// Middle-click autoscroll: the pointer's distance from the anchor sets the speed, and any click exits. What the drift
// means — which list moves, and how far it may go — is the pane's (`drifted`).
//
// Not the graph's own: the diff moves under the same hand, and the two panes differ only in where sideways is offered
// (the graph asks for the lanes column, the diff is one column of text throughout).
//
// **The gesture starts at the press and ends two ways, which is what every browser's does**
// (Chromium `autoscroll_controller.cc`, three states): a middle button put down and let go without travelling leaves
// the drift latched, and one held while the hand travels ends when the hand lets go. Answering at the release instead
// had neither — nothing at all happened while the button was down, and a hand that pressed, pulled and let go was
// left holding a drift it had just finished making.
Item {
    id: autoScroll

    /// Where the column that goes sideways begins and ends in this frame. A gesture that starts between the two carries
    /// it sideways as well.
    required property real panFrom
    required property real panTo
    /// Whether there is anywhere sideways to go at all.
    required property bool canPan

    property bool scrolling: false
    property real anchorX: 0
    property real anchorY: 0
    property real currentX: 0
    property real currentY: 0
    /// Whether this autoscroll carries the column sideways as well. Decided by where the middle click landed, and kept
    /// for the whole gesture (デザイン規約 §グラフを横へ送る).
    property bool panning: false
    /// Whether the hand has travelled out of the dead zone while still holding the button down. That is the whole of
    /// what tells the two exits apart: let go having travelled and the drift ends with the hand, let go without and it
    /// stays for the next click to take down.
    property bool travelled: false
    /// How many times the drift below has been asked for a distance since this gesture began. **Automation only** —
    /// a run proving that a hand inside the dead zone moves nothing has to know the ticker actually ran, and the
    /// absence of movement cannot say that for itself (規約 §UI 自動化の因果性).
    property int ticks: 0

    /// How far the drift has travelled since the last tick. `dx` is 0 unless this gesture pans and there is room to pan
    /// in.
    signal drifted(real dy, real dx)

    /// Starts autoscroll from a point in the pane's frame. The press and the automation hook both come through here, so
    /// which column offers the sideways drift is answered in exactly one place.
    function start(x, y) {
        autoScroll.anchorX = x
        autoScroll.anchorY = y
        autoScroll.currentX = x
        autoScroll.currentY = y
        autoScroll.panning = x >= autoScroll.panFrom && x < autoScroll.panTo
        autoScroll.travelled = false
        autoScroll.ticks = 0
        autoScroll.scrolling = true
    }
    /// Where the pointer has drifted to since. Its distance from the anchor is what the ticker below reads as speed —
    /// the moving pointer and the automation hook write the same two values.
    ///
    /// **And whether the hand has left the dead zone**, which is the whole of what tells the two exits apart. Decided
    /// here rather than in the handler, so a run with no pointer to move goes in where the pointer does
    /// (verify-ui スキル §注入はハンドラ本体そのものへ入れる).
    function drift(x, y) {
        autoScroll.currentX = x
        autoScroll.currentY = y
        if (Math.abs(x - autoScroll.anchorX) > Metrics.middleScrollDeadZone
                || Math.abs(y - autoScroll.anchorY) > Metrics.middleScrollDeadZone)
            autoScroll.travelled = true
    }
    /// The gesture is over: the click that took it down, the key, or this pane going off the screen.
    ///
    /// **What the last gesture did is left standing** — `start` is what clears it. Cleared here, the reason this one
    /// ended would be gone by the time anything could read it back.
    function stop() {
        autoScroll.scrolling = false
    }
    /// The middle button was let go. Which exit this is was decided while it was down (`travelled`).
    function letGo() {
        if (autoScroll.travelled)
            autoScroll.stop()
    }

    // A pane taken off the screen takes its gesture with it. The drift is a pointer mode with a tick behind it, and
    // both would otherwise still be running when the reader came back to a pane they left minutes ago.
    onVisibleChanged: {
        if (!autoScroll.visible)
            autoScroll.stop()
    }

    // The middle button, while nothing is latched. **The press is the start**, and the drag that may follow it belongs
    // to this area too: it holds the grab until the button comes up, so the overlay below never sees that hand.
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.MiddleButton
        onPressed: mouse => autoScroll.start(mouse.x, mouse.y)
        onPositionChanged: mouse => autoScroll.drift(mouse.x, mouse.y)
        onReleased: autoScroll.letGo()
    }
    MouseArea {
        visible: autoScroll.scrolling
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.AllButtons
        // The cursor says which ways this gesture goes, so a press that did not land on the lanes does not look broken
        // when the lanes stay put under a sideways drift.
        cursorShape: autoScroll.panning ? Qt.SizeAllCursor : Qt.SizeVerCursor
        onPositionChanged: mouse => autoScroll.drift(mouse.x, mouse.y)
        onPressed: mouse => {
            autoScroll.stop()
            mouse.accepted = true
        }
        Timer {
            id: drift
            running: autoScroll.scrolling
            interval: 16
            repeat: true
            onTriggered: {
                autoScroll.ticks++
                const dy = Metrics.handSent(autoScroll.currentY - autoScroll.anchorY, drift.interval)
                const dx = autoScroll.panning && autoScroll.canPan
                         ? Metrics.handSent(autoScroll.currentX - autoScroll.anchorX, drift.interval)
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
