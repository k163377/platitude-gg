pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// Middle-click autoscroll: the pointer's distance from the anchor sets the speed, and any click exits. What the drift
// means — which list moves, and how far — is the surface's (`drifted`). Every surface that scrolls has one
// (デザイン規約 §中クリックの自動スクロール).
//
// It stands over what the surface draws and steps aside for two things (`claimedAt`). Waiting under the surface it
// would never hear the press: Qt's text controls take every middle button (on Linux a read-only `TextEdit` too).
//
// The gesture starts at the press and ends two ways, as a browser's does (Chromium `autoscroll_controller.cc`): let
// go without travelling and the drift stays latched; held while travelling, it ends when the hand lets go. Answered
// at the release instead, nothing happens while the button is down.
Item {
    id: autoScroll

    /// Where the column that goes sideways begins and ends in this frame; a gesture started between them pans too.
    /// None unless a surface names one (the graph, the diff).
    property real panFrom: 0
    property real panTo: 0
    /// Whether there is anywhere sideways to go at all.
    property bool canPan: false
    /// Whether a middle click in text being written pastes on this platform (the primary selection, Linux alone).
    /// A browser gives that click to the text as well (Firefox, bugzilla 1716068).
    readonly property bool middlePastes: Qt.platform.os === "linux"

    property bool scrolling: false
    property real anchorX: 0
    property real anchorY: 0
    property real currentX: 0
    property real currentY: 0
    /// Whether this gesture pans too — decided where the press landed and kept for the whole gesture
    /// (デザイン規約 §グラフを横へ送る).
    property bool panning: false
    /// Whether the hand left the dead zone with the button still down — what tells the two exits apart (`letGo`).
    property bool travelled: false
    /// Drift ticks since this gesture began. Automation only: a run proving the dead zone moves nothing has to know
    /// the ticker ran.
    property int ticks: 0

    /// How far the drift has travelled since the last tick. `dx` is 0 unless this gesture pans and there is room to pan
    /// in.
    signal drifted(real dy, real dx)

    /// A middle press at a point in this frame: starts the gesture, or steps aside for what claimed it (`claimedAt`).
    /// Answers whether the hand took it. The press and a run both come in here.
    function press(x, y) {
        if (autoScroll.claimedAt(x, y))
            return false
        autoScroll.start(x, y)
        return true
    }
    /// Whether a middle press at this point belongs to something under the hand: text being written where the platform
    /// pastes (`middlePastes`), or a surface inside this one with a hand of its own. Walks the items the press would
    /// land on, top down.
    function claimedAt(x, y) {
        let item = autoScroll.parent
        let at = autoScroll.mapToItem(item, x, y)
        while (item !== null) {
            if (item !== autoScroll.parent && item.middlePastes !== undefined)
                return true
            if (autoScroll.middlePastes && item.cursorPosition !== undefined && item.readOnly === false)
                return true
            const next = autoScroll.topChildAt(item, at.x, at.y)
            if (next !== null)
                at = item.mapToItem(next, at.x, at.y)
            item = next
        }
        return false
    }
    /// The child a press at this point of `item` lands on: visible, holding the point, highest `z`, last declared
    /// among equals. Not `childAt`, which ignores `z` (`DiffPane.doorOnTop`). This hand is passed over — it is asking.
    function topChildAt(item, x, y) {
        const kids = item.children
        let top = null
        for (let i = 0; i < kids.length; i++) {
            const kid = kids[i]
            if (kid === autoScroll || !kid.visible || !kid.contains(item.mapToItem(kid, x, y)))
                continue
            if (top === null || kid.z >= top.z)
                top = kid
        }
        return top
    }
    /// Starts autoscroll from a point in this frame; the press and the automation hook both come through here.
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
    /// Where the pointer has drifted to (the ticker reads its distance from the anchor as speed), and whether it has
    /// left the dead zone (`travelled`). The pointer and a run both come in here.
    function drift(x, y) {
        autoScroll.currentX = x
        autoScroll.currentY = y
        if (Math.abs(x - autoScroll.anchorX) > Metrics.middleScrollDeadZone
                || Math.abs(y - autoScroll.anchorY) > Metrics.middleScrollDeadZone)
            autoScroll.travelled = true
    }
    /// The gesture is over: a click, the wheel, Escape (`MiddleHand`), another surface's gesture, or this surface
    /// leaving the screen. What it did is left standing until `start` — cleared here, it would be gone before anything
    /// read it.
    function stop() {
        autoScroll.scrolling = false
    }
    /// The middle button was let go. Which exit this is was decided while it was down (`travelled`).
    function letGo() {
        if (autoScroll.travelled)
            autoScroll.stop()
    }

    // Over the rows and the view's bar (`AutoScrollBar` is at 1), so a running gesture takes the click that ends it
    // wherever it lands. Waiting, it takes the middle button alone and no hover, so everything underneath answers.
    z: 2
    // A surface taken off the screen takes its gesture with it, or the drift runs on for when the reader comes back.
    onVisibleChanged: {
        if (!autoScroll.visible)
            autoScroll.stop()
    }
    // The window's one gesture (`MiddleHand`): a hand that starts takes the place of whichever was running, and one
    // that stops gives the place up — here, so every way a gesture ends is counted.
    onScrollingChanged: {
        if (autoScroll.scrolling) {
            if (MiddleHand.running !== null && MiddleHand.running !== autoScroll)
                MiddleHand.running.stop()
            MiddleHand.running = autoScroll
        } else if (MiddleHand.running === autoScroll) {
            MiddleHand.running = null
        }
    }
    // A surface can be taken down in the middle of its gesture — a tab closed from the keyboard — and the window's one
    // gesture must not go on naming it.
    Component.onDestruction: {
        if (MiddleHand.running === autoScroll)
            MiddleHand.running = null
    }

    // The middle button while nothing is latched. It holds the grab until the button comes up, so the overlay below
    // never sees a hand dragging with the button down.
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.MiddleButton
        // Left unaccepted, the press goes on to what lies under the hand (`press`).
        onPressed: mouse => mouse.accepted = autoScroll.press(mouse.x, mouse.y)
        onPositionChanged: mouse => autoScroll.drift(mouse.x, mouse.y)
        onReleased: autoScroll.letGo()
    }
    MouseArea {
        visible: autoScroll.scrolling
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.AllButtons
        // The cursor says which ways this gesture goes, so lanes staying put under a sideways drift do not look broken.
        cursorShape: autoScroll.panning ? Qt.SizeAllCursor : Qt.SizeVerCursor
        onPositionChanged: mouse => autoScroll.drift(mouse.x, mouse.y)
        onPressed: mouse => {
            autoScroll.stop()
            mouse.accepted = true
        }
        // A hand on the wheel has stopped pointing: the gesture ends, and the notch, left unaccepted, still reaches the
        // surface underneath.
        onWheel: wheel => {
            autoScroll.stop()
            wheel.accepted = false
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
