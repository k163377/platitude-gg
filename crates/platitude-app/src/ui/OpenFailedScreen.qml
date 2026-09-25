import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A tab whose repository would not open, said where the panes would be. The picker's failures go to
// `OpenFailedDialog`; everything else — a restored tab, a worktree row, PGG_AUTO_OPEN — comes here. Why two components:
// rules-refs/app-ui.md「失敗の画面(`OpenFailedScreen`)と `OpenFailedDialog` は別部品」.
Item {
    id: screen

    /// core's answer (`errorKind`): `plain` / `bare` / `other` (`message` is git's own words, and only `other` has
    /// any).
    required property string kind
    required property string path
    required property string message
    /// How wide the page is. Measured against the page: a `SplitView` gives no width to an item that is not on
    /// screen, and this one is off screen until the moment it is needed.
    required property real pageWidth
    /// The page whose panes' seat this took: the command log's toggle has to be here too, since the log is where the
    /// failed command's words are (デザイン規約 §git が言ったことを読む場所).
    property var commandsPage: null

    /// Automation: the sweep hand, as a card's `<card>.background.pad`.
    property alias pad: failedHand

    /// The "Close tab" button.
    signal closeRequested()

    // Drags the words from the air around them (規約 §右のペインの字は掴める). Under the column, so the `Close tab`
    // button keeps its presses.
    SweepPad {
        id: failedHand
        anchors.fill: failedColumn
        content: failedColumn
    }
    Column {
        id: failedColumn
        anchors.centerIn: parent
        spacing: Theme.spaceMd
        width: Math.min(Theme.textWidth, screen.pageWidth - 2 * Theme.spaceXxl)

        CardText {
            text: Words.openFailure(screen.kind)
            pixelSize: Theme.fontLg
            weight: Font.DemiBold
            horizontalAlignment: Text.AlignHCenter
            width: parent.width
        }
        // Wrapped: a cut path would hand a dragging reader a `…` (規約 §右のペインの字は掴める).
        CardText {
            text: screen.path
            color: Theme.textSecondary
            horizontalAlignment: Text.AlignHCenter
            width: parent.width
        }
        // git's words, and the only red on this screen: the gate that reports a git it cannot use draws the line the
        // same way (Main.qml).
        NoticeLine {
            visible: screen.kind === "other"
            text: screen.message
            color: Theme.danger
            width: parent.width
        }
        NoticeButton {
            text: qsTr("Close tab")
            onActivated: screen.closeRequested()
        }
    }

    // Captioned: there is no list here for a bare mark to be read against.
    CommandsToggle {
        id: commandsSeat
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: Theme.rowHeight
        captioned: true
        visible: screen.commandsPage !== null && !commandsSeat.open
        curPage: screen.commandsPage
    }
}
