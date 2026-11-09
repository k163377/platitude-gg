import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A tab whose repository would not open, said where the panes would be. The picker's failures go to
// `OpenFailedDialog`; everything else — a restored tab, a worktree row, PGG_AUTO_OPEN — comes here. Two components, not
// one with two faces: the seats differ (デザイン規約 §可否・警告の出し場所), and so do the type, the alignment and the
// ways out; they share only the words (`Words.openFailure` / `openFailureNext`) and the width (規約 §レイアウト初期値).
//
// The panes' seat, inside `RepoPage`'s vertical `SplitView`: covering the whole page would leave the command log inside
// the cover, out of `>_`'s reach. The seat takes the left menu with it, so the `>_` seat is this screen's own
// (`commandsPage`). The log opens by hand only — opened on its own, a background read alone would stand an empty panel
// up. Headless: `open-fail-tab` / `open-fail-tab-bare` / `open-fail-tab-log`.
Item {
    id: screen

    /// core's answer (`errorKind`): `plain` / `bare` / `untrusted` / `config` / `other` (`message` is git's own words,
    /// which only the last three have — `Words.openFailureQuotesGit`).
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
    // button keeps its presses. Headless: `open-fail-sweep`.
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
            weight: Theme.fontWeightStrong
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
        // The reader's own step, in this application's words and colour (`Words.openFailureNext`).
        CardText {
            visible: text !== ""
            text: Words.openFailureNext(screen.kind)
            horizontalAlignment: Text.AlignHCenter
            width: parent.width
        }
        // git's words, and the only red on this screen: the gate that reports a git it cannot use draws the line the
        // same way (`StartupGate`).
        NoticeLine {
            visible: Words.openFailureQuotesGit(screen.kind)
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
