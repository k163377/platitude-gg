import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Toolbar action named twice over: an icon to find it by shape, the
// word to be sure of it. Both halves dim together when disabled.
HoverToolButton {
    id: actionBtn
    property string kind: ""
    /// Colour of both halves while the button is live.
    property color tone: Theme.textPrimary
    /// git is on the network for this button: the icon turns in its place.
    property bool busy: false
    /// Hold the turn still (automation — a spinning icon photographs
    /// differently every time).
    property bool still: false
    /// How long this button has to be held to fire `held()`; zero for an
    /// ordinary button, where a click is the whole gesture (デザイン規約
    /// §進行中・長押しの定数).
    property int holdMs: 0
    /// How far into the hold the press has got, 0 to 1.
    property real holdProgress: 0
    /// Held all the way down. Clicks are the caller's own business: a
    /// button that means one thing held and another clicked is the shape
    /// this is meant to avoid.
    signal held()
    readonly property color fg: enabled ? tone : Theme.textMuted

    /// Automation: run the hold to its end without a press behind it.
    function completeHold() {
        if (actionBtn.holdMs > 0)
            holdAnim.restart()
    }

    onDownChanged: {
        if (actionBtn.holdMs <= 0)
            return
        if (actionBtn.down)
            holdAnim.restart()
        else
            holdAnim.stop()
    }
    NumberAnimation {
        id: holdAnim
        target: actionBtn
        property: "holdProgress"
        from: 0
        to: 1
        duration: Math.max(actionBtn.holdMs, 1)
        // Letting go part way leaves nothing behind, so the next press
        // starts the whole way from the beginning again.
        onStopped: actionBtn.holdProgress = 0
        onFinished: actionBtn.held()
    }
    background: Rectangle {
        color: "transparent"
        // The press overlay, filling in from the left as the hold runs.
        Rectangle {
            width: parent.width * actionBtn.holdProgress
            height: parent.height
            radius: Theme.radiusSm
            color: Theme.bgPressed
            visible: actionBtn.holdProgress > 0
        }
    }
    contentItem: RowLayout {
        spacing: Theme.spaceXs
        Item {
            implicitWidth: Theme.iconMd
            implicitHeight: Theme.iconMd
            Layout.alignment: Qt.AlignVCenter
            NavIcon {
                anchors.fill: parent
                kind: actionBtn.kind
                tint: actionBtn.fg
                visible: !actionBtn.busy
            }
            // Its own item rather than a rotation on the one above: an
            // animator leaves the angle where it stopped, and the icon
            // that returns must not come back tilted.
            NavIcon {
                anchors.fill: parent
                kind: "spinner"
                tint: actionBtn.fg
                visible: actionBtn.busy
                // On the render thread, so it keeps turning while the GUI
                // thread drains models.
                RotationAnimator on rotation {
                    running: actionBtn.busy && !actionBtn.still
                    loops: Animation.Infinite
                    from: 0
                    to: 360
                    duration: Metrics.spinMs
                }
            }
        }
        Label {
            text: actionBtn.text
            color: actionBtn.fg
            font.pixelSize: Theme.fontMd
            elide: Text.ElideRight
            Layout.maximumWidth: 240
            Layout.alignment: Qt.AlignVCenter
        }
    }
}
