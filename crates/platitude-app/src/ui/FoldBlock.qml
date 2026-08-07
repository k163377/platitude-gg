import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The control that folds the left menu to its icons and puts it back.
// A block at the end of the pane's header band rather than a mark
// floating in it: it has nothing to do with the filter it stands next
// to, and on the same bare ground it read as part of the input. The
// whole block takes the click, so the reach is the band's full height.
Rectangle {
    id: block
    /// Which way the list goes when this is pressed — away to the left,
    /// or back out to the right.
    property bool folds: true
    property string tip: ""
    /// Whether something stands to its left to be divided from. False
    /// when the block is the whole band (folded), where the hairline
    /// would land on the window's own edge.
    property bool divided: true
    signal activated()

    color: Theme.bgElevated

    HoverToolButton {
        anchors.fill: parent
        padding: 0
        // Carried in a box of its own rather than sized by the button:
        // the mark stands beside the sections' icons on the folded rail
        // and has to be the same weight and the same 20 as they are.
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
        ToolTip.visible: hovered
        ToolTip.delay: 600
        ToolTip.text: block.tip
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
