import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The control that folds the left menu to its icons and puts it back. A block at the end of the pane's header
// band: it has nothing to do with the filter it stands next to, and on the same bare
// ground it read as part of the input. The whole block takes the click, so the reach is the band's full height.
//
// It says nothing on hover. Every other icon-only control in the window carries a tooltip, but this one names itself
// twice over: the chevrons are the standing idiom for a panel folding away, and the answer is on screen the moment it
// is pressed and one press from being taken back (デザイン規約 §左メニューを畳む). The name lives on `Accessible.name`, where the
// people who cannot read the mark still get it.
Rectangle {
    id: block
    /// Which way the list goes when this is pressed — away to the left, or back out to the right.
    property bool folds: true
    /// Whether something stands to its left to be divided from. False when the block is the whole band (folded), where
    /// the hairline would land on the window's own edge.
    property bool divided: true
    signal activated()

    color: Theme.bgElevated

    HoverToolButton {
        anchors.fill: parent
        padding: 0
        // Square, edge to edge: this is a band's worth of ground, and the headers, rows and rail cells it stands among
        // all wash without a corner of their own.
        washRadius: 0
        Accessible.name: block.folds ? qsTr("Fold the list to its icons") : qsTr("Unfold the list")
        // Carried in a box of its own: the mark stands beside the sections' icons on
        // the folded rail and has to be the same weight and the same 20 as they are.
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
    // What divides it from the band it sits at the end of.
    Rectangle {
        visible: block.divided
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: Theme.borderWidth
        color: Theme.borderSubtle
    }
}
