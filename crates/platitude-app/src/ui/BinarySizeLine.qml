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
    // Signs between the words (デザイン規約 §アイコンの席 の修飾): each box the words' line, so they stand where the
    // face's own `+` would.
    DotMark {
        visible: binaryLine.sized
        tint: Theme.textMuted
        Layout.preferredHeight: sizeWord.implicitHeight
        middle: sizeWord.baselineOffset - WordFace.signMiddle(sizeWord.font.pixelSize)
    }
    Label {
        id: sizeWord
        visible: binaryLine.sized
        text: binaryLine.removed ? qsTr("was %1").arg(binaryLine.oldSize)
              : binaryLine.bothSides ? binaryLine.oldSize : binaryLine.newSize
        color: Theme.textMuted
    }
    NavIcon {
        id: sizeMark
        visible: binaryLine.bothSides
        Layout.preferredWidth: sizeMark.wordCell
        Layout.preferredHeight: sizeWord.implicitHeight
        width: sizeMark.wordCell
        height: sizeWord.implicitHeight
        kind: "arrow"
        set: "sign"
        size: sizeWord.font.pixelSize
        wordBase: sizeWord.baselineOffset
        tint: Theme.textMuted
    }
    Label {
        visible: binaryLine.bothSides
        text: binaryLine.newSize
        color: Theme.textMuted
    }
    // Takes the slack; without it the layout centres what it cannot fill.
    Item { Layout.fillWidth: true }
}
