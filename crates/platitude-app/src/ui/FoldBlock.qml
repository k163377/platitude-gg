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
    property string mark: "«"
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
        text: block.mark
        // The mark is the whole of the button, so it is written at a
        // heading's size rather than a caption's: at fontSm a guillemet
        // is six pixels of ink.
        font.pixelSize: Theme.fontLg
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
