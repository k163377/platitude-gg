import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The details pane's band while the selected row is a stash, standing in place of `COMMIT` and dressed like
// `PaneHeader`: `STASH`, and the two things git can do with it from here, spelled as the stash row's menu spells them
// (デザイン規約 §変更を退避する — also why `stash@{n}` is not drawn nor searched for, and why `drop` stays on the
// row's menu).
Rectangle {
    id: band

    /// Reflog selector — what `apply` and `pop` are run on, and `""` hides the band. Not drawn.
    property string stashRef: ""

    signal applyRequested(string selector)
    signal popRequested(string selector)

    /// One box for both words, the two answers to one question: each sized to its own word, the shorter would be the
    /// visibly smaller target (デザイン規約 §余白). Measured off the chips, which are what is drawn.
    readonly property real actBox: Math.max(applyChip.implicitWidth, popChip.implicitWidth)

    visible: band.stashRef !== ""
    implicitHeight: Theme.headerHeight
    color: Theme.bgElevated
    BandRule { z: 1 }
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceXs
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        NavIcon {
            kind: "stash"
            tint: Theme.textSecondary
            width: Theme.iconMd
            height: Theme.iconMd
        }
        Label {
            text: qsTr("STASH")
            font.pixelSize: Theme.fontMd
            font.weight: Font.DemiBold
            color: Theme.textSecondary
            // Unlike `PaneHeader`'s, this caption shares its row with the buttons and would paint over them when
            // squeezed.
            elide: Text.ElideRight
            Layout.fillWidth: true
        }
        HoverToolButton {
            padding: Theme.spaceSm
            implicitWidth: band.actBox + 2 * padding
            tip: qsTr("Apply this stash, keeping it")
            onClicked: band.applyRequested(band.stashRef)
            contentItem: CodeChip {
                id: applyChip
                word: "apply"
                size: Theme.fontSm
                tint: Theme.textPrimary
            }
        }
        HoverToolButton {
            padding: Theme.spaceSm
            implicitWidth: band.actBox + 2 * padding
            tip: qsTr("Apply this stash and drop it")
            onClicked: band.popRequested(band.stashRef)
            contentItem: CodeChip {
                id: popChip
                word: "pop"
                size: Theme.fontSm
                tint: Theme.textPrimary
            }
        }
    }
}
