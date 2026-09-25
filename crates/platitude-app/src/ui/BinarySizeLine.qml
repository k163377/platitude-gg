pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The word, then what the file weighs. The dot and the arrow are drawn, not glyphs (規約 §寸法「印は描いて出す」);
// the one wording without a size keeps its dash, since that is a sentence.
RowLayout {
    id: binaryLine

    required property string oldSize
    required property string newSize

    readonly property bool sized: binaryLine.oldSize !== "" || binaryLine.newSize !== ""
    readonly property bool bothSides: binaryLine.oldSize !== "" && binaryLine.newSize !== ""
    readonly property bool removed: binaryLine.newSize === "" && binaryLine.oldSize !== ""

    spacing: Theme.spaceXs
    Label {
        text: !binaryLine.sized ? qsTr("Binary file — no text diff")
              : binaryLine.removed ? qsTr("Binary file removed") : qsTr("Binary file")
        elide: Text.ElideRight
        color: Theme.textMuted
    }
    DotMark {
        visible: binaryLine.sized
        tint: Theme.textMuted
    }
    Label {
        visible: binaryLine.sized
        text: binaryLine.removed ? qsTr("was %1").arg(binaryLine.oldSize)
              : binaryLine.bothSides ? binaryLine.oldSize : binaryLine.newSize
        color: Theme.textMuted
    }
    Item {
        visible: binaryLine.bothSides
        Layout.preferredWidth: sizeMark.inkWidth
        Layout.preferredHeight: Theme.iconSm
        Layout.alignment: Qt.AlignVCenter
        NavIcon {
            id: sizeMark
            anchors.centerIn: parent
            kind: "arrow"
            width: Theme.iconSm
            height: Theme.iconSm
            stroke: Metrics.iconStroke * Theme.iconSm / Theme.iconMd
            tint: Theme.textMuted
        }
    }
    Label {
        visible: binaryLine.bothSides
        text: binaryLine.newSize
        color: Theme.textMuted
    }
    // Takes the slack; without it the layout centres what it cannot fill.
    Item { Layout.fillWidth: true }
}
