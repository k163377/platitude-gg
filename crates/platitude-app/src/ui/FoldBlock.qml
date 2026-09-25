import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The block at the end of the left pane's header band that folds the menu to its icons and back; the whole block is the
// button. Its own ground: on the bare ground of the filter beside it, it would read as part of the input. No tooltip
// (デザイン規約 §左メニューを畳む) — the name is on `Accessible.name`.
Rectangle {
    id: block
    /// Pressing folds the list (true) or unfolds it.
    property bool folds: true
    /// Draws the left hairline. False when folded: the block is then the whole band, at the window's edge.
    property bool divided: true
    signal activated()

    color: Theme.bgElevated

    HoverToolButton {
        anchors.fill: parent
        padding: 0
        // Square, like every wash it stands among.
        washRadius: 0
        Accessible.name: block.folds ? qsTr("Fold the list to its icons") : qsTr("Unfold the list")
        // A box of its own so the mark keeps `iconLg`, the size of the section icons beside it on the folded rail.
        contentItem: Item {
            NavIcon {
                anchors.centerIn: parent
                kind: "chevrons"
                tint: Theme.textSecondary
                width: Theme.iconLg
                height: Theme.iconLg
                rotation: block.folds ? 180 : 0
            }
        }
        onClicked: block.activated()
    }
    Rectangle {
        visible: block.divided
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: Theme.borderWidth
        color: Theme.borderSubtle
    }
}
