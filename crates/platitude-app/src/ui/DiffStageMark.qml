import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The mark a changed line puts out for the hand: `+` stages, `−` unstages — the direction of the write, not the kind
// of line; unframed (デザイン規約 §diff の中のステージ). Hidden rather than unbuilt off the pointer: the pointer moves
// faster than a canvas can be built and painted.
Rectangle {
    id: mark

    /// Whether the pointer is on this mark's line (`DiffPane.settlePointedRow`).
    required property bool shown
    /// Which way the write goes, and whether one is running.
    required property bool staged
    required property bool busy

    /// The press: this line goes over to the other side now.
    signal pressed()

    visible: mark.shown
    width: Theme.iconMd
    height: Theme.iconMd
    radius: Theme.radiusSm
    color: markHover.containsMouse ? Theme.bgHover : "transparent"
    /// On its own line, never over the lines either side; leftward first, off the code it stages
    /// (`SharedToolTip.tipRowSide`).
    readonly property string tipRowSide: "left"
    ToolTip.visible: markHover.containsMouse
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: mark.staged ? qsTr("Unstage this line") : qsTr("Stage this line")
    NavIcon {
        anchors.centerIn: parent
        width: Theme.iconSm
        height: Theme.iconSm
        kind: mark.staged ? "minus" : "plus"
        tint: mark.staged ? Theme.diffRemovedFg : Theme.diffAddedFg
    }
    MouseArea {
        id: markHover
        anchors.fill: parent
        hoverEnabled: true
        enabled: !mark.busy
        onClicked: mark.pressed()
    }
}
