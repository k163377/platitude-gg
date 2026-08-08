pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The shape every standing question takes (デザイン規約 §可否・警告の出し場所):
// a bar that comes down from the top of the list the question is about and
// pushes its rows down rather than covering them — what is being judged has
// to stay in sight — with the row it concerns marked instead of named, so
// the question is written exactly once.
//
// Two lists raise one: the graph, and the working tree's changed files.
// Only one question stands at a time (the page holds the run it guards),
// so the Escape below is never ambiguous.
Rectangle {
    id: bar

    /// The question. Empty is closed; nothing else opens or shuts it.
    property string label: ""
    /// What answering costs, in the one line §用語 allows for it.
    property string detail: ""
    /// The words on the pill that answers.
    property string accept: ""
    /// Throwing away work in hand (danger) rather than reaching past this
    /// machine (warning) — §状態.
    property bool danger: false
    /// What the pill says on hover: that it is held, and what the far side
    /// will make of what it does. The bar's own line has room for one
    /// thing only, and this is where §長押し puts the rest.
    property string tip: ""
    /// Whether answering takes a hold rather than a click (デザイン規約
    /// §進行中・長押しの定数): the frame fills from the left while the
    /// press lasts, and letting go part way leaves nothing behind. A hold
    /// pill reports no click at all — neither the release that completes
    /// the hold nor the one that gives up on it may fall through to the
    /// answer.
    property bool hold: false
    /// How far into the hold the press has got, 0 to 1.
    property real holdProgress: 0
    /// Automation: run the hold to its end without a press behind it.
    function completeHold() {
        if (bar.hold) {
            backAnim.stop()
            holdAnim.restart()
        }
    }
    /// Called off without the hand letting go — the question itself has
    /// gone. Blanks the fill rather than sliding it back: the slide is an
    /// answer to a press that stopped short, and there is no longer
    /// anything here for it to be an answer about.
    function blankHold() {
        holdAnim.stop()
        backAnim.stop()
        bar.holdProgress = 0
    }
    /// The keys that stand in for the press (デザイン規約 §長押し).
    function holdKey(key) {
        return key === Qt.Key_Space || key === Qt.Key_Return
                || key === Qt.Key_Enter
    }

    /// The pill was clicked: the owner runs what the question guarded.
    signal confirmed()
    /// Walked away from — Escape, the ✕, or a click elsewhere.
    signal cancelled()

    readonly property bool open: bar.label !== ""
    readonly property color tone: bar.danger ? Theme.danger : Theme.warning
    // A question walked away from mid-press takes the press with it: a
    // fill left standing would carry on into whatever is asked next.
    //
    // Opening hands the pill the focus so the keyboard's way in needs no
    // hunting for it. The bar only ever opens because the person just asked
    // for it from a list, so there is no typing here to interrupt.
    //
    // A tick later, not now: the gesture that raised the question is still
    // being delivered — a double-click on a graph row has a release and a
    // second click behind it — and the list it lands on takes the focus
    // back if the pill claims it first.
    onOpenChanged: {
        if (bar.open)
            Qt.callLater(acceptPill.forceActiveFocus)
        else
            bar.blankHold()
    }

    // Sized by its own words, opened and closed with the standard 200ms.
    clip: true
    color: Theme.bgElevated
    implicitHeight: bar.open ? askRow.implicitHeight + 2 * Theme.spaceMd : 0
    Behavior on implicitHeight {
        NumberAnimation { duration: 200 }
    }
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: Theme.borderWidth
        color: bar.tone
    }
    RowLayout {
        id: askRow
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: Theme.spaceMd
        spacing: Theme.spaceMd
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                text: bar.label
                color: bar.tone
                font.pixelSize: Theme.fontMd
                font.weight: Font.DemiBold
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
            Label {
                text: bar.detail
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
        }
        // This is the answer — by a click, or by a press held all the way
        // down where the question asks for one. It is the only thing on
        // the bar that acts, so nothing else here can be hit by accident.
        Rectangle {
            id: acceptPill
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: acceptRow.implicitWidth + 2 * Theme.spaceMd
            implicitHeight: Theme.controlHeight
            radius: Theme.radiusSm
            color: acceptMouse.containsMouse && bar.holdProgress === 0
                   ? Theme.bgHover : "transparent"
            border.color: bar.tone
            border.width: Theme.borderWidth
            // Reachable without a pointer, and given the focus as the bar
            // opens: the pill is the only thing here that acts, so there is
            // nothing else for a tab to land on first (デザイン規約 §長押し).
            //
            // Closed, it leaves the tab order by going disabled rather than
            // by dropping `activeFocusOnTab` — Qt refuses to clear that on
            // the item that currently holds the focus, and warns. Disabling
            // takes the focus away first, and the pill draws its own colours
            // rather than the palette's, so the collapse looks no different.
            activeFocusOnTab: true
            enabled: bar.open
            // The pill draws itself rather than being a control, so it
            // has to name itself. The gesture is said here because the
            // words on it no longer carry it.
            Accessible.role: Accessible.Button
            Accessible.name: bar.accept
            Accessible.description: bar.hold ? qsTr("Hold to activate") : ""
            // The hold filling the frame from the left, inset by the
            // border so the frame stays a frame while it fills: that the
            // fill reaches the end is the whole progress report.
            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.margins: Theme.borderWidth
                // Never thinner than `holdFillMin` while it runs — see
                // ActionButton for why the proportional start is no good.
                width: bar.holdProgress > 0
                       ? Math.max(Metrics.holdFillMin,
                                  (parent.width - 2 * Theme.borderWidth)
                                  * bar.holdProgress)
                       : 0
                color: bar.tone
                visible: bar.holdProgress > 0
            }
            // Outside the frame: the frame's colour says what answering
            // costs, and focus must not be able to take that over.
            Rectangle {
                anchors.fill: parent
                anchors.margins: -Theme.spaceXs / 2
                color: "transparent"
                border.color: Theme.borderFocus
                border.width: Theme.borderWidth
                radius: Theme.radiusMd
                visible: acceptPill.activeFocus
            }
            Row {
                id: acceptRow
                anchors.centerIn: parent
                spacing: Theme.spaceXs
                // Ahead of the word, where the eye starts: the pill says
                // how it is answered before it says what answering does
                // (デザイン規約 §長押し). A pill answered by a click wears
                // no mark and spends no width on one.
                HoldIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.verticalCenterOffset: Metrics.opticalDrop
                    visible: bar.hold
                    progress: bar.holdProgress
                    tint: acceptLabel.color
                }
                Label {
                    id: acceptLabel
                    anchors.verticalCenter: parent.verticalCenter
                    text: bar.accept
                    // Lifted while the fill runs under it: the words cross
                    // both the filled side and the bare one.
                    color: bar.holdProgress > 0 ? Theme.textOnAccent : bar.tone
                    font.pixelSize: Theme.fontMd
                }
            }
            ToolTip.visible: bar.tip !== "" && acceptMouse.containsMouse
            ToolTip.delay: 600
            ToolTip.text: bar.tip
            MouseArea {
                id: acceptMouse
                anchors.fill: parent
                hoverEnabled: true
                // `released` inside the pill, not `clicked`: Qt stops
                // emitting `clicked` once its own press-and-hold timer has
                // gone off (800ms), so a click pill held down the way the
                // hold pills ask for would answer nothing at all and say
                // nothing about it. Releasing away from the pill still
                // calls it off — `containsMouse` is what a click checked.
                onReleased: if (!bar.hold && containsMouse) bar.confirmed()
                // `containsPress`, not `pressed`: a press dragged off the
                // pill has to call the hold off, the way letting go does.
                // `pressed` stays true out there — it keeps the grab — and
                // would leave sliding away as no escape at all.
                onContainsPressChanged: {
                    if (!bar.hold)
                        return
                    if (containsPress) {
                        backAnim.stop()
                        holdAnim.restart()
                    } else {
                        holdAnim.stop()
                    }
                }
            }
            // The same answer without a pointer: Space or Enter, held where
            // the question asks for a hold and simply pressed where it does
            // not. Auto-repeat is dropped on both edges — see ActionButton.
            Keys.onPressed: event => {
                if (event.isAutoRepeat || !bar.holdKey(event.key))
                    return
                if (bar.hold) {
                    backAnim.stop()
                    holdAnim.restart()
                }
                event.accepted = true
            }
            Keys.onReleased: event => {
                if (event.isAutoRepeat || !bar.holdKey(event.key))
                    return
                if (bar.hold)
                    holdAnim.stop()
                else
                    bar.confirmed()
                event.accepted = true
            }
            NumberAnimation {
                id: holdAnim
                target: bar
                property: "holdProgress"
                from: 0
                to: 1
                duration: Metrics.holdMs
                // A press that stopped short slides back out. A hold pill
                // reports no click at all, so without this a plain click
                // on it is answered by nothing happening (デザイン規約
                // §長押し). Pressed all the way through, the question has
                // already been answered and the bar is on its way out.
                onStopped: {
                    if (bar.holdProgress >= 1)
                        bar.holdProgress = 0
                    else if (bar.holdProgress > 0)
                        backAnim.restart()
                }
                onFinished: bar.confirmed()
            }
            NumberAnimation {
                id: backAnim
                target: bar
                property: "holdProgress"
                to: 0
                duration: Metrics.holdBackMs
                easing.type: Easing.OutCubic
            }
        }
        // Escape and a click anywhere else walk away too; this is the
        // way out that can be seen, for a bar that stands until it is
        // answered.
        NavIcon {
            Layout.alignment: Qt.AlignVCenter
            width: Theme.iconMd
            height: Theme.iconMd
            kind: "close"
            tint: dismissMouse.containsMouse ? Theme.textPrimary
                                             : Theme.textSecondary
            MouseArea {
                id: dismissMouse
                anchors.fill: parent
                anchors.margins: -Theme.spaceXs
                hoverEnabled: true
                onClicked: bar.cancelled()
            }
        }
    }
    // The bar has no focus of its own — neither list takes any — so
    // Escape is heard as a shortcut while it stands.
    Shortcut {
        // `sequences` rather than `sequence`: Cancel is more than one key
        // on some platforms, and binding the single form takes only the
        // first of them (Qt warns about exactly this).
        sequences: [StandardKey.Cancel]
        enabled: bar.open
        onActivated: bar.cancelled()
    }
}
