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
        if (bar.hold)
            holdAnim.restart()
    }

    /// The pill was clicked: the owner runs what the question guarded.
    signal confirmed()
    /// Walked away from — Escape, the ✕, or a click elsewhere.
    signal cancelled()

    readonly property bool open: bar.label !== ""
    readonly property color tone: bar.danger ? Theme.danger : Theme.warning
    // A question walked away from mid-press takes the press with it: a
    // fill left standing would carry on into whatever is asked next.
    onOpenChanged: if (!bar.open) holdAnim.stop()

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
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: acceptLabel.implicitWidth + 2 * Theme.spaceMd
            implicitHeight: Theme.controlHeight
            radius: Theme.radiusSm
            color: acceptMouse.containsMouse && bar.holdProgress === 0
                   ? Theme.bgHover : "transparent"
            border.color: bar.tone
            border.width: Theme.borderWidth
            // The hold filling the frame from the left, inset by the
            // border so the frame stays a frame while it fills: that the
            // fill reaches the end is the whole progress report.
            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.margins: Theme.borderWidth
                width: (parent.width - 2 * Theme.borderWidth) * bar.holdProgress
                color: bar.tone
                visible: bar.holdProgress > 0
            }
            Label {
                id: acceptLabel
                anchors.centerIn: parent
                text: bar.accept
                // Lifted while the fill runs under it: the words cross
                // both the filled side and the bare one.
                color: bar.holdProgress > 0 ? Theme.textOnAccent : bar.tone
                font.pixelSize: Theme.fontMd
            }
            ToolTip.visible: bar.tip !== "" && acceptMouse.containsMouse
            ToolTip.delay: 600
            ToolTip.text: bar.tip
            MouseArea {
                id: acceptMouse
                anchors.fill: parent
                hoverEnabled: true
                onClicked: if (!bar.hold) bar.confirmed()
                onPressedChanged: {
                    if (!bar.hold)
                        return
                    if (pressed)
                        holdAnim.restart()
                    else
                        holdAnim.stop()
                }
            }
            NumberAnimation {
                id: holdAnim
                target: bar
                property: "holdProgress"
                from: 0
                to: 1
                duration: Metrics.holdMs
                // Letting go part way leaves nothing behind, so the next
                // press starts the whole way from the beginning again.
                onStopped: bar.holdProgress = 0
                onFinished: bar.confirmed()
            }
        }
        // Escape and a click anywhere else walk away too; this is the
        // way out that can be seen, for a bar that stands until it is
        // answered.
        Label {
            Layout.alignment: Qt.AlignVCenter
            text: "✕"
            color: dismissMouse.containsMouse ? Theme.textPrimary
                                              : Theme.textSecondary
            font.pixelSize: Theme.fontMd
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
