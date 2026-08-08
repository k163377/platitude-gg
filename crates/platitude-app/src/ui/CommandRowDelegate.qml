pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One git invocation in the command log: the clock, the command, and how
// it went. A failure keeps git's own parting words under it — the whole
// point of the panel is that nothing is rephrased.
Rectangle {
    id: row

    required property int index
    required property double at_ms
    required property string args
    required property string full
    required property string state
    required property string result
    required property string duration
    required property string output

    readonly property bool failed: row.state === "failed"
    readonly property bool showsOutput: row.failed && row.output !== ""

    signal copyRequested(string text)

    // Width comes from the view; the height grows with the output block.
    height: Theme.rowHeight + (row.showsOutput
                               ? outputText.implicitHeight + 2 * Theme.spaceXs
                               : 0)
    color: row.failed || rowHover.containsMouse ? Theme.bgElevated : "transparent"

    // What went wrong is carried by the edge as well as by the words, so
    // a failure is findable while scrolling past at speed.
    Rectangle {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: 2 * Theme.borderWidth
        color: Theme.danger
        visible: row.failed
    }

    MouseArea {
        id: rowHover
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.RightButton
        onClicked: {
            // Read as the menu opens and left alone while it stands: a
            // command still running has nothing to copy yet, and its
            // output arriving must not push a second row in under the
            // pointer (デザイン規約 §メニュー).
            rowMenu.hasOutput = row.output !== ""
            rowMenu.offer()
        }
        ToolTip.visible: containsMouse && row.state !== "running"
        ToolTip.delay: Metrics.tipDelayMs
        // The reproducible form is long; it is here rather than in the
        // row so the log stays one line per command.
        ToolTip.text: row.full
    }

    AppMenu {
        id: rowMenu
        /// Whether git had said anything by the time the menu opened.
        property bool hasOutput: false
        AppMenuItem {
            text: qsTr("Copy command")
            onTriggered: row.copyRequested(row.full)
        }
        AppMenuItem {
            text: qsTr("Copy output")
            offered: rowMenu.hasOutput
            onTriggered: row.copyRequested(row.output)
        }
    }

    Item {
        id: line
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: Theme.rowHeight

        Label {
            id: clock
            anchors.left: parent.left
            anchors.leftMargin: Theme.spaceMd
            anchors.verticalCenter: parent.verticalCenter
            text: Qt.formatDateTime(new Date(row.at_ms), "HH:mm:ss")
            color: Theme.textMuted
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontSm
        }
        // The program never varies, so it is drawn rather than read: what
        // changes from row to row is the part after it.
        Label {
            id: program
            anchors.left: clock.right
            anchors.leftMargin: Theme.spaceLg
            anchors.verticalCenter: parent.verticalCenter
            text: "git"
            color: Theme.textMuted
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontSm
        }
        Label {
            anchors.left: program.right
            anchors.leftMargin: Theme.spaceXs
            anchors.right: outcome.left
            anchors.rightMargin: Theme.spaceMd
            anchors.verticalCenter: parent.verticalCenter
            text: row.args
            elide: Text.ElideRight
            color: Theme.textPrimary
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontSm
        }
        Row {
            id: outcome
            anchors.right: parent.right
            anchors.rightMargin: Theme.spaceMd
            anchors.verticalCenter: parent.verticalCenter
            spacing: Theme.spaceMd
            Label {
                text: row.result
                visible: text !== ""
                color: Theme.danger
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontSm
            }
            Label {
                // Exit 0 says nothing that the absence of a complaint has
                // not already said, so only the time it took is kept.
                text: row.state === "running" ? qsTr("running…") : row.duration
                color: row.state === "running" ? Theme.accent : Theme.textMuted
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontSm
            }
        }
    }

    // Indented to the command it belongs to. Positioned rather than
    // anchored: the column it lines up with lives inside `line`, which
    // makes it neither parent nor sibling of this.
    Label {
        id: outputText
        visible: row.showsOutput
        x: program.x
        y: line.height + Theme.spaceXs
        width: row.width - program.x - Theme.spaceMd
        text: row.output
        wrapMode: Text.Wrap
        color: Theme.textSecondary
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontSm
        lineHeight: Theme.fontSmLine
        lineHeightMode: Text.FixedHeight
    }
}
