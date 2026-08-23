import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A tab whose repository would not open, said where the panes would be.
//
// The same three lines the picker's dialog says, out of the same place (`Words.openFailure`): which of the three it was
// is core's answer (`errorKind`), not something read off git's wording. Which of the two screens a failure lands on is
// decided by the road, not the kind: the picker's answers go to the dialog, everything else — a restored tab, a
// worktree row, PG_AUTO_OPEN — comes here.
//
// Not one component with that dialog: they are two seats (デザイン規約 §可否・警告の出し場所) and read as such — this one is centred in
// the seat it takes over and offers the way out of a tab, the dialog is a window's left-aligned form and offers the
// picker again. What they do share is the wording, and that already lives in `Words`.
Item {
    id: screen

    /// core's answer: `plain` / `bare` / `other` (`message` is git's own words, and only that third kind has any).
    required property string kind
    required property string path
    required property string message
    /// How wide the page is. Measured against the page rather than this item's own box: a `SplitView` gives no width to
    /// an item that is not on screen, and this one is off screen until the moment it is needed.
    required property real pageWidth
    /// The page this screen took the panes' seat on. This screen stands where the left menu would be, so the block at
    /// the foot of that pane has to be here too — a tab that would not open still ran the command that says why, and
    /// with nothing else on screen to say it the log is the only place it is written (デザイン規約 §git が言ったことを読む場所).
    property var commandsPage: null

    /// The "Close tab" button.
    signal closeRequested()

    Column {
        anchors.centerIn: parent
        spacing: Theme.spaceMd
        width: Math.min(700, screen.pageWidth - 2 * Theme.spaceXl)

        Label {
            text: Words.openFailure(screen.kind)
            font.pixelSize: Theme.fontLg
            font.weight: Font.DemiBold
            anchors.horizontalCenter: parent.horizontalCenter
        }
        Label {
            text: screen.path
            color: Theme.textSecondary
            elide: Text.ElideMiddle
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
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

    CommandsToggle {
        id: commandsSeat
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        visible: screen.commandsPage !== null && !commandsSeat.open
        curPage: screen.commandsPage
    }
}
