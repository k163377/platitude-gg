import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The seat that opens the git commands this tab ran, at the foot of the left menu (デザイン規約 §git が言ったことを読む場所).
//
// It wears whatever the pane it stands in wears: one of the sections' own bands while the list is open, the fold
// control's own block at the other end of the rail while the list is folded, and the left end of the log's header band
// once the panel is up — where that band is this same row, run the width of the window. One control, so the mark says
// the same thing in the same words wherever the pane put it.
//
// It takes the ground those three seats are on (`bgElevated`) and answers the pointer with the wash they answer with,
// and nothing else: no corner, no frame, no fill of its own for being open. What says the state is the mark
// (デザイン規約 §git が言ったことを読む場所 — 通常 / 実行中 / 直近が失敗 / 開いている間).
Rectangle {
    id: toggle

    /// The RepoPage this seat belongs to (null while no tab is open).
    property var curPage: null
    /// Whether the seat carries the log's name beside the mark. False in the one seat with no room for it — the folded
    /// rail, where no section carries a name either.
    property bool captioned: false
    /// Whether the seat closes itself off from what stands above it — the hairline the rail's own head block draws
    /// under itself. False in the panel's band, where the seat is part of that band rather than under it.
    property bool ruled: true
    /// The step the mark is drawn at, which is the one whatever it stands among is drawn at: the sections' own marks
    /// where it stands beside a name (`iconMd`), and the fold control's where it stands alone at the end of the rail
    /// (`iconLg` — that block is what this one is read against down there, 2026-08-23 ユーザー指示. デザイン規約 §寸法).
    property real markSize: Theme.iconMd

    /// The log this tab is filling. Asked of the log rather than of the page: closing a tab takes the page's models
    /// down while the page itself is still standing, so `curPage !== null` is true for a beat after there is nothing
    /// left to read off it.
    readonly property var log: toggle.curPage !== null ? toggle.curPage.pageCommands : null
    /// The last command failed, or the tab is carrying an error line — one mark for both. The page keeps that rule,
    /// since the mark moves seats and the rule does not.
    readonly property bool wrong: toggle.curPage !== null && toggle.curPage.commandsWrong
    /// Whether the panel is up.
    readonly property bool open: toggle.curPage !== null && toggle.curPage.commandsOpen
    /// Automation: the colour the mark actually painted, which is what says the page's news reached it
    /// (`PG_AUTO_ACT=commands-clear`).
    readonly property alias markColor: commandsMark.tint

    /// Where the mark stands and where the name begins — a section's row without the fold arrow's column, which is
    /// what this row has no use for (2026-08-23 ユーザー指示: 「>モードは無いので左に揃えて」). Written as the sum of the
    /// steps rather than as numbers, so the row moves with the pane's own inset when that moves.
    readonly property real markX: Theme.spaceXs
    readonly property real captionX: toggle.markX + Theme.iconMd + Theme.spaceXs

    visible: toggle.curPage !== null
    // Its name's own run when it carries one — the log's band seats it in a row of controls, where a seat asking for
    // the pane's width would push the rest of them off the end. It stops at the name so that what follows sits the
    // band's own step away, the way a section's count sits from its caption.
    implicitWidth: toggle.captioned ? toggle.captionX + commandsName.implicitWidth : Theme.railWidth
    implicitHeight: toggle.captioned ? Theme.rowHeight : Theme.headerHeight
    // A band's ground, which is what all three of its seats stand on: the sections' headers, the fold control's block
    // and the log's own band are `bgElevated`, and this row is read against them (2026-08-23 ユーザー指示). In the
    // log's band it repaints the value that band already carries, so the row reads the same there with no case for it.
    color: Theme.bgElevated
    // The wash goes over that ground rather than instead of it — `bgHover` is a white at 8% and has nothing of its own
    // to sit on (`NavHeader` layers the same two).
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: commandsMouse.containsMouse
    }
    Rectangle {
        visible: toggle.ruled
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: Theme.borderWidth
        color: Theme.borderSubtle
    }
    NavIcon {
        id: commandsMark
        kind: "terminal"
        // In the sections' mark column when the name is beside it; in the middle of the rail's width when it stands
        // alone, which is where the fold control and every cell between them put theirs.
        width: toggle.markSize
        height: toggle.markSize
        x: toggle.captioned ? toggle.markX + (Theme.iconMd - toggle.markSize) / 2
                            : (Theme.railWidth - toggle.markSize) / 2
        anchors.verticalCenter: parent.verticalCenter
        tint: toggle.wrong ? Theme.danger
              : toggle.log !== null && toggle.log.running ? Theme.accent
              : toggle.open ? Theme.textPrimary
              : Theme.textMuted
    }
    // The same name the panel's own band carries, so the two doors into the log do not name it differently. It goes
    // red with the mark rather than staying grey beside it: a section's band moves its mark and its caption together,
    // and one red glyph in a column of words is not what a failure looks like here (規約 §無効 is the same shape).
    Label {
        id: commandsName
        visible: toggle.captioned
        x: toggle.captionX
        anchors.verticalCenter: parent.verticalCenter
        text: qsTr("GIT COMMANDS")
        font.pixelSize: Theme.fontMd
        font.weight: Font.DemiBold
        color: toggle.wrong ? Theme.danger : Theme.textSecondary
    }
    MouseArea {
        id: commandsMouse
        anchors.fill: parent
        hoverEnabled: true
        onClicked: toggle.curPage.toggleCommands()
        // Only where the mark stands alone: a seat that says its own name has nothing left for a tooltip to add
        // (規約 §hover のツールチップ).
        ToolTip.visible: containsMouse && !toggle.captioned
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.text: toggle.open
                      ? qsTr("Hide the git commands this window ran")
                      : toggle.wrong
                        ? qsTr("The last command failed — read it here")
                        : qsTr("Show the git commands this window ran")
    }
}
