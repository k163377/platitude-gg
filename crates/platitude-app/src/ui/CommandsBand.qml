import QtQuick
import platitude.ui

// The command log's row: the strip the left menu is left with down here, and the panel itself.
//
// Opening the log takes the foot of that pane down to its rail, and the block that opened it becomes the head of the
// strip — level with the panel's own band, the way the pane's header block is level with the panes either side of it.
// The panel takes the width the list gave up, so it starts at the rail rather than at the window's edge
// (デザイン規約 §git が言ったことを読む場所).
Item {
    id: band

    required property var commandsModel
    /// The page this row belongs to: the seat reads the log and the error line off it, the same as the seat at the foot
    /// of the pane does while this row is not here.
    required property var curPage
    /// A failure that never became a command row (a background read that gave up), shown in the panel's header.
    property string errorText: ""

    signal closeRequested()
    signal errorCleared()
    signal copyRequested(string text)

    /// Automation: the colour the `>_` came out to while the panel is up (`PG_AUTO_ACT=commands-clear`).
    readonly property alias markColor: seat.markColor

    /// The header's `Clear`, pressed from outside the panel (automation only — `PG_AUTO_ACT=commands-clear`).
    function clearPanel() {
        pane.clearPanel()
    }

    // The panel opens on its newest row, wherever it was raised from — the row that has just been added is the one
    // somebody came here to read. The panel itself never goes away, so it is this row that says when it was raised.
    onVisibleChanged: if (band.visible) pane.showLatest()

    Rectangle {
        id: strip
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: Theme.railWidth
        color: Theme.bgSurface
        CommandsToggle {
            id: seat
            curPage: band.curPage
        }
        // The hairline the block starts, carried on down the strip so the panel beside it has an edge for its whole
        // height (the two grounds are one value — `CommandsToggle`).
        Rectangle {
            anchors.right: parent.right
            anchors.top: seat.bottom
            anchors.bottom: parent.bottom
            width: Theme.borderWidth
            color: Theme.borderSubtle
        }
    }

    CommandsPane {
        id: pane
        anchors.left: strip.right
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        commandsModel: band.commandsModel
        errorText: band.errorText
        onCloseRequested: band.closeRequested()
        onErrorCleared: band.errorCleared()
        onCopyRequested: text => band.copyRequested(text)
    }
}
