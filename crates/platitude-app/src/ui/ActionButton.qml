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
    /// git is on the network for this button: the words step aside for a
    /// turning ring in the middle of the button, and the whole of it goes
    /// as dim and as deaf as a disabled one (デザイン規約 §長押し).
    ///
    /// Dimmed rather than actually disabled: `enabled` would take the
    /// focus away, and the press that started the network call is the
    /// very one that may have come from the keyboard.
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
    /// Frame drawn around the button. Transparent leaves the button bare.
    property color frameColor: "transparent"
    /// The last go at what this button does did not work. Drawn as a mark
    /// standing clear of the word's last letter, in the word's own colour
    /// — the frame and the colour already say something is wrong, and this
    /// is what says it is about this button rather than about the state
    /// the toolbar is in.
    property bool alert: false
    /// The colour the hold fills the button with — the frame's, since a
    /// framed button fills the frame it drew. A bare one names its own
    /// (the hunk heading's `Discard hunk`, which fills edge to edge the
    /// way a held menu row does — デザイン規約 §長押し).
    property color holdTone: actionBtn.frameColor
    readonly property bool framed: actionBtn.frameColor.a > 0
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
    readonly property color fg: actionBtn.busy ? Theme.textMuted
                                : holdProgress > 0 ? Theme.textOnAccent
                                : enabled ? tone : Theme.textMuted
    /// Nothing here answers a press while git is on the network for it.
    readonly property bool live: actionBtn.enabled && !actionBtn.busy

    /// Automation: run the hold to its end without a press behind it.
    function completeHold() {
        if (actionBtn.holdMs > 0) {
            backAnim.stop()
            holdAnim.restart()
        }
    }

    // Tab reaches the buttons that need a second way in. The rest of the
    // toolbar stays out of the tab order: a hold is the only gesture here
    // that a pointer alone can fail to make (デザイン規約 §長押し).
    activeFocusOnTab: actionBtn.holdMs > 0
    // The mark says it to whoever can see it, and this says it to whoever
    // cannot: the words themselves no longer carry the gesture.
    Accessible.description: actionBtn.holdMs > 0 ? qsTr("Hold to activate") : ""

    onClicked: if (actionBtn.holdMs <= 0 && actionBtn.live) actionBtn.activated()
    onDownChanged: {
        if (actionBtn.holdMs <= 0 || !actionBtn.live)
            return
        if (actionBtn.down) {
            backAnim.stop()
            holdAnim.restart()
        } else {
            holdAnim.stop()
        }
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
        if (actionBtn.holdMs <= 0 || !actionBtn.live
                || event.isAutoRepeat || !holdKey(event.key))
            return
        backAnim.stop()
        holdAnim.restart()
        event.accepted = true
    }
    Keys.onReleased: event => {
        if (actionBtn.holdMs <= 0 || !actionBtn.live
                || event.isAutoRepeat || !holdKey(event.key))
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
        // A press that stopped short slides back out instead of blanking:
        // a hold button reports no click at all, so without this the only
        // answer to a plain click is nothing happening (デザイン規約
        // §長押し). Pressed all the way through, it has already fired and
        // there is nothing left to say — that one blanks.
        onStopped: {
            if (actionBtn.holdProgress >= 1)
                actionBtn.holdProgress = 0
            else if (actionBtn.holdProgress > 0)
                backAnim.restart()
        }
        onFinished: actionBtn.held()
    }
    NumberAnimation {
        id: backAnim
        target: actionBtn
        property: "holdProgress"
        to: 0
        duration: Metrics.holdBackMs
        easing.type: Easing.OutCubic
    }
    background: Rectangle {
        color: "transparent"
        // The frame goes inert with the rest of the button while git is
        // out on the network: its colour is a warning about a press, and
        // there is no press to be had until this comes back.
        border.color: actionBtn.busy ? Theme.borderDefault : actionBtn.frameColor
        border.width: Theme.borderWidth
        radius: Theme.radiusSm
        // The hold, filling from the left. Inset by the border where
        // there is one, so the frame stays a frame while it fills; a bare
        // button fills edge to edge, the way a held menu row does.
        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.margins: actionBtn.framed ? Theme.borderWidth : 0
            radius: Theme.radiusSm
            // Never thinner than `holdFillMin` while it runs: proportional
            // from zero, the first tenth of the hold is a sub-pixel sliver,
            // so the press reads as not having taken and the whole gesture
            // feels longer than it is (デザイン規約 §進行中・長押しの定数).
            width: actionBtn.holdProgress > 0
                   ? Math.max(Metrics.holdFillMin,
                              (parent.width - 2 * anchors.margins)
                              * actionBtn.holdProgress)
                   : 0
            color: actionBtn.holdTone
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

    // The row keeps its size while the network call runs — the toolbar
    // must not shuffle under a pointer that is still resting on the
    // button — so the words step aside by going transparent rather than
    // by leaving the layout, and the ring turns over the middle of what
    // they left.
    contentItem: Item {
        implicitWidth: btnRow.implicitWidth
        implicitHeight: btnRow.implicitHeight

        RowLayout {
            id: btnRow
            anchors.fill: parent
            spacing: Theme.spaceXs
            opacity: actionBtn.busy ? 0 : 1
            // A button with no icon to name it spends no width on one —
            // the word is the whole of it (the hunk header's buttons).
            // A button that swaps between a named icon and the mark keeps
            // the wider of the two seats whichever it is wearing, so its
            // width does not change with its state and the toolbar does
            // not slide under a pointer already resting on it. One that
            // only ever wears the mark fits it, and its words sit as
            // close to it as a menu row's do.
            Item {
                id: seat
                /// Both marks to wear at once: what the button does, and
                /// that it is held rather than clicked.
                readonly property bool paired: actionBtn.kind !== ""
                                               && actionBtn.holdMs > 0
                /// How far the two are set apart across the slash.
                readonly property int spread: Theme.iconMd - Theme.spaceXs

                visible: actionBtn.kind !== "" || actionBtn.holdMs > 0
                implicitWidth: !visible ? 0
                               : actionBtn.kind !== ""
                                 ? Math.max(Theme.iconMd, Theme.iconSm + seat.spread)
                                 : Theme.iconSm
                implicitHeight: Theme.iconMd + Theme.spaceXs
                Layout.alignment: Qt.AlignVCenter
                NavIcon {
                    width: Theme.iconMd
                    height: Theme.iconMd
                    anchors.centerIn: parent
                    kind: actionBtn.kind
                    tint: actionBtn.fg
                    visible: actionBtn.holdMs <= 0
                }
                // A held button with nothing to name it says only how it
                // is worked, where the eye starts the row (デザイン規約
                // §長押し).
                HoldIcon {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.verticalCenterOffset: Metrics.opticalDrop
                    progress: actionBtn.holdProgress
                    tint: actionBtn.fg
                    visible: actionBtn.holdMs > 0 && !seat.paired
                }
                // A held button that also has a name wears both, set as a
                // fraction: each a size down, the slash between them, each
                // pushed off the middle line. Letting the hold mark take
                // the seat on its own would cost the button the one thing
                // that says what it does — `Push -f` would stop being a
                // push at a glance (デザイン規約 §長押し).
                Item {
                    anchors.fill: parent
                    visible: seat.paired
                    NavIcon {
                        kind: actionBtn.kind
                        tint: actionBtn.fg
                        width: Theme.iconSm
                        height: Theme.iconSm
                        x: 0
                        y: 0
                    }
                    Label {
                        text: "/"
                        color: actionBtn.fg
                        font.pixelSize: Theme.fontMd
                        anchors.centerIn: parent
                    }
                    HoldIcon {
                        progress: actionBtn.holdProgress
                        tint: actionBtn.fg
                        width: Theme.iconSm
                        height: Theme.iconSm
                        x: seat.spread
                        y: parent.height - Theme.iconSm
                    }
                }
            }
            // Measured, never drawn: a hidden item is left out of the
            // layout, and a Label measures the way the visible one does —
            // TextMetrics reports a few pixels tighter, which is enough of
            // a difference to shift the toolbar it is here to hold still.
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
                // the icon. Without it the words sit hard against the
                // border (measured: 8px of air on the left, 5 on the
                // right).
                readonly property real box: widest.implicitWidth > 0
                                            ? widest.implicitWidth + Theme.spaceXs : 0
                text: actionBtn.text
                color: actionBtn.fg
                font.pixelSize: actionBtn.font.pixelSize
                elide: Text.ElideRight
                Layout.maximumWidth: 240
                Layout.preferredWidth: Math.max(implicitWidth, box)
                Layout.alignment: Qt.AlignVCenter
                // Past the word's end rather than over its shoulder: the
                // last letter has to stay readable, and the box is
                // measured for the longest wording so there is room after
                // the shorter ones.
                NavIcon {
                    visible: actionBtn.alert
                    kind: "bang"
                    tint: actionBtn.fg
                    width: Theme.iconSm
                    height: Theme.iconSm
                    x: btnLabel.implicitWidth - Theme.spaceXs
                    y: -Theme.spaceXs
                }
            }
        }
        // Its own item rather than a rotation on the icon above: an
        // animator leaves the angle where it stopped, and the icon that
        // returns must not come back tilted.
        NavIcon {
            anchors.centerIn: parent
            width: Theme.iconMd
            height: Theme.iconMd
            kind: "spinner"
            tint: Theme.textMuted
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
}
