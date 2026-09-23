pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// Middle-click autoscroll: the pointer's distance from the anchor sets the speed, and any click exits. What the drift
// means — which list moves, and how far it may go — is the surface's (`drifted`).
//
// **Every surface that scrolls has one** (デザイン規約 §中クリックの自動スクロール): the lists (`AppListView` carries
// it), the blocks and the settings chapters that scroll as one piece, the message boxes, the combo's card. They differ
// only in where sideways is offered — the graph asks for the lanes column, the diff is one column of text throughout,
// and everything else goes up and down alone.
//
// **Over what the surface draws, and it steps aside for two things** (`claimedAt`): text being written, where the
// platform pastes with the middle button, and a surface inside this one with a hand of its own. Everything else a
// middle press lands on is the gesture's — which is a browser's rule, and the one Qt's own delivery cannot give: its
// text controls take every button, so a hand waiting under them never hears the press (measured, qmltestrunner — a
// middle press on a `TextField` in a row focuses it; on Linux a read-only `TextEdit` takes it as well, for a paste it
// then does not make, qtdeclarative v6.10.3 `qquicktextedit.cpp`).
//
// **The gesture starts at the press and ends two ways, which is what every browser's does**
// (Chromium `autoscroll_controller.cc`, three states): a middle button put down and let go without travelling leaves
// the drift latched, and one held while the hand travels ends when the hand lets go. Answering at the release instead
// had neither — nothing at all happened while the button was down, and a hand that pressed, pulled and let go was
// left holding a drift it had just finished making.
Item {
    id: autoScroll

    /// Where the column that goes sideways begins and ends in this frame. A gesture that starts between the two carries
    /// it sideways as well. **None unless a surface names one** — only the graph and the diff go sideways.
    property real panFrom: 0
    property real panTo: 0
    /// Whether there is anywhere sideways to go at all.
    property bool canPan: false
    /// Whether a middle click in text being written pastes on this platform: X11's and Wayland's primary selection,
    /// which Qt's text controls answer the middle button with there (`QQuickTextControl`: editable and
    /// `supportsSelection()`). **A browser gives that click to the text as well** — Firefox starts no autoscroll on
    /// editable content while middle-click paste is on, which it is by default on Linux alone (bugzilla 1716068).
    readonly property bool middlePastes: Qt.platform.os === "linux"

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

    /// A middle press at a point in this frame: the gesture starts there, or the hand steps aside and the press goes on
    /// to what claimed it (`claimedAt`). Answers whether the hand took it. **The press and a run both come in here**,
    /// so a run cannot start a gesture the hand would have handed on (verify-ui スキル §注入はハンドラ本体そのものへ入れる).
    function press(x, y) {
        if (autoScroll.claimedAt(x, y))
            return false
        autoScroll.start(x, y)
        return true
    }
    /// Whether a middle press at this point belongs to something drawn under the hand: **text being written, where the
    /// platform pastes with that button** (`middlePastes`), or **a surface inside this one with a hand of its own**
    /// (a message box on the block it stands in) — whose hand is the next one under the press, once this one lets it
    /// go. Read off the items the press would land on, top down from the frame this hand covers.
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
    /// The child a press at this point of `item` lands on: visible, holding the point, the highest `z`, and of equal
    /// ones the last declared. **Walked here rather than asked of `childAt`**, which takes siblings in the order they
    /// were declared and never looks at `z` (measured — `DiffPane.doorOnTop`). This hand and its own two areas are
    /// passed over: they are what is asking.
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
    /// The gesture is over: the click that took it down, the wheel, the key (`MiddleHand`), a gesture starting on
    /// another surface, or this surface going off the screen.
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

    // Over the rows and over the view's own bar (`AutoScrollBar` stands at 1), so a gesture under way takes the click
    // that ends it wherever it lands. Waiting, it takes the middle button alone and holds no hover, so the rows, the
    // bar and their pointers go on answering underneath.
    z: 2
    // A surface taken off the screen takes its gesture with it. The drift is a pointer mode with a tick behind it, and
    // both would otherwise still be running when the reader came back to a pane they left minutes ago.
    onVisibleChanged: {
        if (!autoScroll.visible)
            autoScroll.stop()
    }
    // The window's one gesture (`MiddleHand`): a hand that starts takes the place of whichever was running, and one that
    // stops gives the place up — here, so every way a gesture ends is counted, whoever ended it.
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

    // The middle button, while nothing is latched. **The press is the start**, and the drag that may follow it belongs
    // to this area too: it holds the grab until the button comes up, so the overlay below never sees that hand.
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
        // The cursor says which ways this gesture goes, so a press that did not land on the lanes does not look broken
        // when the lanes stay put under a sideways drift.
        cursorShape: autoScroll.panning ? Qt.SizeAllCursor : Qt.SizeVerCursor
        onPositionChanged: mouse => autoScroll.drift(mouse.x, mouse.y)
        onPressed: mouse => {
            autoScroll.stop()
            mouse.accepted = true
        }
        // A hand on the wheel has stopped pointing: the gesture ends, and the notch goes on to the surface underneath,
        // which sends it the way it sends every notch (measured, qmltestrunner: left unaccepted here it still moves the
        // list below).
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
