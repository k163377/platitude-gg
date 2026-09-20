import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The mark a changed line puts out for the hand: `+` where a press takes the line into the staging area and `−`
// where it takes it back out, in the pair of colours that gesture wears everywhere else (デザイン規約 §diff の中のステージ).
// It names the *direction* of the write — an added and a deleted line are both staged by the same `+`. Unframed: a
// box around a mark this size reads as a control that came loose from the toolbar, and the ground it needs is the
// one the pointer brings with it.
//
// **It writes, there and then**, and it is the only thing in a row that takes a press at all. The row builds it
// only on a line that can be staged on its own (`DiffRowDelegate`), and whether the pointer is on that line is what
// shows it: the pointer moves faster than a canvas can be built and painted, so the mark waits built and hidden on
// the lines the pointer can reach.
Rectangle {
    id: mark

    /// Whether the pointer is on this mark's line (the pane's answer, `DiffPane.settlePointedRow`).
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
