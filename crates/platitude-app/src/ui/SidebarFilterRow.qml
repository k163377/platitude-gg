pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The sidebar's header band: a frameless filter that takes all the
// width left over, and the fold control at the end of it. It is the band
// the other panes' headers line up with, so it takes the header height,
// and the hairline that closes it is the band's rather than the input's
// (it runs on under the button).
Item {
    id: filterRow

    /// What is typed into the band. The sections read it back, and the
    /// smoke hook writes it — into the field itself, so what they are
    /// asked is what a typist asks them.
    property alias text: field.text

    signal foldRequested()

    implicitHeight: Theme.headerHeight
    RowLayout {
        anchors.fill: parent
        spacing: 0
        TextField {
            id: field
            Layout.fillWidth: true
            Layout.fillHeight: true
            font.pixelSize: Theme.fontMd
            leftPadding: Theme.spaceSm
            rightPadding: Theme.spaceSm
            topPadding: 0
            bottomPadding: 0
            placeholderText: qsTr("Filter")
            background: null
        }
        // The whole block is the button: the reach is the band's
        // full height, not a mark's worth of it. The rail's band
        // is the same block with the mark pointing back.
        FoldBlock {
            Layout.fillHeight: true
            Layout.preferredWidth: Theme.headerHeight
            onActivated: filterRow.foldRequested()
        }
    }
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: Theme.borderWidth
        color: field.activeFocus ? Theme.borderFocus
                                 : Theme.borderSubtle
    }
}
