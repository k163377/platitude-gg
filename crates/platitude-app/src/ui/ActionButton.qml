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
    /// Frame drawn around the button, and the colour the hold fills it
    /// with. Transparent leaves the button bare.
    property color frameColor: "transparent"
    /// Text the label's box is measured for. A button whose wording
    /// changes with its state would otherwise move everything beside it
    /// in the toolbar every time the state changed.
    property string widestText: ""
    /// Held all the way down.
    signal held()
    /// Pressed and let go, meaning the button's ordinary action.
    ///
    /// A hold button never emits this: neither the release that completes
    /// a hold nor the one that gives up on it part way may fall through
    /// to what this button does when it is not a hold button.
    signal activated()
    readonly property color fg: holdProgress > 0 ? Theme.textOnAccent
                                : enabled ? tone : Theme.textMuted

    /// Automation: run the hold to its end without a press behind it.
    function completeHold() {
        if (actionBtn.holdMs > 0)
            holdAnim.restart()
    }

    // Tab reaches the buttons that need a second way in. The rest of the
    // toolbar stays out of the tab order: a hold is the only gesture here
    // that a pointer alone can fail to make (デザイン規約 §長押し).
    activeFocusOnTab: actionBtn.holdMs > 0

    onClicked: if (actionBtn.holdMs <= 0) actionBtn.activated()
    onDownChanged: {
        if (actionBtn.holdMs <= 0)
            return
        if (actionBtn.down)
            holdAnim.restart()
        else
            holdAnim.stop()
    }
    // The hold's other hand: focus it, then hold Space or Enter. Accepting
    // the key keeps AbstractButton from also taking Space as a press, which
    // would drive the same fill from `down` a second time.
    //
    // Auto-repeat is dropped on both edges. A held key repeats its press on
    // every platform and its release on some, and either edge would restart
    // the fill from zero for as long as the key was held — the hold could
    // then never complete.
    Keys.onPressed: event => {
        if (actionBtn.holdMs <= 0 || event.isAutoRepeat || !holdKey(event.key))
            return
        holdAnim.restart()
        event.accepted = true
    }
    Keys.onReleased: event => {
        if (actionBtn.holdMs <= 0 || event.isAutoRepeat || !holdKey(event.key))
            return
        holdAnim.stop()
        event.accepted = true
    }
    function holdKey(key) {
        return key === Qt.Key_Space || key === Qt.Key_Return
                || key === Qt.Key_Enter
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
        border.color: actionBtn.frameColor
        border.width: Theme.borderWidth
        radius: Theme.radiusSm
        // The hold, filling the frame from the left. Inset by the border
        // so the frame stays a frame while it fills.
        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.margins: Theme.borderWidth
            // Never thinner than `holdFillMin` while it runs: proportional
            // from zero, the first tenth of the hold is a sub-pixel sliver,
            // so the press reads as not having taken and the whole gesture
            // feels longer than it is (デザイン規約 §進行中・長押しの定数).
            width: actionBtn.holdProgress > 0
                   ? Math.max(Metrics.holdFillMin,
                              (parent.width - 2 * Theme.borderWidth)
                              * actionBtn.holdProgress)
                   : 0
            color: actionBtn.frameColor
            visible: actionBtn.holdProgress > 0
        }
        // Drawn outside the frame rather than in it: the frame's colour is
        // already saying this button is the dangerous one, and focus must
        // not be able to take that over.
        Rectangle {
            anchors.fill: parent
            anchors.margins: -Theme.spaceXs / 2
            color: "transparent"
            border.color: Theme.borderFocus
            border.width: Theme.borderWidth
            radius: Theme.radiusMd
            visible: actionBtn.activeFocus
        }
    }
    // The toolbar's size unless an instance says otherwise: the hunk
    // header's buttons are the body size the rest of that row is.
    font.pixelSize: Theme.fontMd

    contentItem: RowLayout {
        spacing: Theme.spaceXs
        // A button with no icon to name it spends no width on one — the
        // word is the whole of it (the hunk header's buttons).
        Item {
            visible: actionBtn.kind !== "" || actionBtn.busy
            implicitWidth: visible ? Theme.iconMd : 0
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
        // Measured, never drawn: a hidden item is left out of the layout,
        // and a Label measures the way the visible one does — TextMetrics
        // reports a few pixels tighter, which is enough of a difference
        // to shift the toolbar it is here to hold still.
        Label {
            id: widest
            visible: false
            text: actionBtn.widestText
            font.pixelSize: actionBtn.font.pixelSize
        }
        Label {
            id: btnLabel
            // The box is the widest wording plus one gap, so the last
            // letter stands off the frame the way the first stands off
            // the icon. Without it the words sit hard against the border
            // (measured: 8px of air on the left, 5 on the right).
            readonly property real box: widest.implicitWidth > 0
                                        ? widest.implicitWidth + Theme.spaceXs : 0
            text: actionBtn.text
            color: actionBtn.fg
            font.pixelSize: actionBtn.font.pixelSize
            elide: Text.ElideRight
            Layout.maximumWidth: 240
            Layout.preferredWidth: Math.max(implicitWidth, box)
            Layout.alignment: Qt.AlignVCenter
        }
    }
}
