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

    /// The pill was clicked: the owner runs what the question guarded.
    signal confirmed()
    /// Walked away from — Escape, the ✕, or a click elsewhere.
    signal cancelled()

    readonly property bool open: bar.label !== ""
    readonly property color tone: bar.danger ? Theme.danger : Theme.warning

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
        // Clicking this is the answer. It is the only thing on the bar
        // that acts, so nothing else here can be hit by accident.
        Rectangle {
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: acceptLabel.implicitWidth + 2 * Theme.spaceMd
            implicitHeight: Theme.controlHeight
            radius: Theme.radiusSm
            color: acceptMouse.containsMouse ? Theme.bgHover : "transparent"
            border.color: bar.tone
            border.width: Theme.borderWidth
            Label {
                id: acceptLabel
                anchors.centerIn: parent
                text: bar.accept
                color: bar.tone
                font.pixelSize: Theme.fontMd
            }
            MouseArea {
                id: acceptMouse
                anchors.fill: parent
                hoverEnabled: true
                onClicked: bar.confirmed()
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
