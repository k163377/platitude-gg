import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The seat that opens the git commands this tab ran, at the foot of the left menu (デザイン規約 §git が言ったことを読む場所).
//
// It wears whatever the pane it stands in wears: a band with the mark and the name in the sections' own columns while
// the list is open, a cell of the rail's own reach while the list is folded, and — once the panel is up — the left end
// of the panel's header band, where the mark stands beside the name that band was already carrying. Three shapes, one
// control, so the mark says the same thing in the same words wherever the pane put it.
//
// It answers the pointer with the wash the rows, headers and rail cells beside it answer with, and nothing else: no
// ground of its own, no corner, no frame. What is left to say the state is the mark itself (デザイン規約 §git が言った
// ことを読む場所 — 通常 / 実行中 / 直近が失敗 / 開いている間).
Rectangle {
    id: toggle

    /// The RepoPage this seat belongs to (null while no tab is open).
    property var curPage: null
    /// Whether the seat carries the log's name beside the mark. False in the two seats where a name would be said
    /// twice or not fit: the folded rail (where no section carries one either) and the panel's own band.
    property bool captioned: false
    /// Whether the seat closes itself off from what stands above it — the hairline the rail's own head block draws
    /// under itself. False in the panel's band, where the seat is part of that band rather than under it.
    property bool ruled: true
    /// The step the mark is drawn at, which is the one whatever it stands among is drawn at: a band's marks beside the
    /// sections' (`iconMd`), a block filling a band on its own one up from them (`iconLg`), and the folded rail's cell
    /// the cell's own content (`iconXl` — デザイン規約 §寸法, and §左メニューを畳む for why those two differ).
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

    /// Where a section's mark stands and where its caption begins (`NavHeader`'s row: a step, the fold arrow, a step,
    /// the mark, a step). Written as that row's own sum rather than as numbers, so this band stays in the sections'
    /// columns when theirs move — the fold arrow's column is left empty here, since this row has no section to open
    /// and a name that skipped the column would not line up with any of them.
    readonly property real markX: Theme.spaceXs + Theme.iconSm + Theme.spaceXs
    readonly property real captionX: toggle.markX + Theme.iconMd + Theme.spaceXs

    visible: toggle.curPage !== null
    implicitWidth: Theme.railWidth
    implicitHeight: toggle.captioned ? Theme.rowHeight : Theme.toolbarHeight
    color: commandsMouse.containsMouse ? Theme.bgHover : "transparent"
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
        // alone, which is where every cell and block up that column puts its own.
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
