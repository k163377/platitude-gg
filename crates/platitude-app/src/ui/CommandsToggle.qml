import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The block that opens the git commands this tab ran, kept at the foot of the left menu — the mirror of the fold
// control at the head of it (デザイン規約 §git が言ったことを読む場所). A block filling one end of a band rather than a mark
// floating in it, the whole of it taking the press, and no corner of its own (`FoldBlock`).
//
// The one control on that pane that is not about a ref: what it opens is a panel rather than an action, and `>_` is the
// whole of its wording. Pressed, the pane's foot goes down to this block alone and the panel takes the width beside it,
// which is why the block is the rail's width wherever it stands — it is the same seat before and after.
Rectangle {
    id: toggle

    /// The RepoPage this seat belongs to (null while no tab is open).
    property var curPage: null

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
    readonly property alias markColor: commandsMark.color

    visible: toggle.curPage !== null
    implicitWidth: Theme.railWidth
    implicitHeight: Theme.headerHeight
    color: toggle.open ? Theme.bgSelected : commandsMouse.containsMouse ? Theme.bgHover : Theme.bgElevated
    border.width: toggle.wrong ? Theme.borderWidth : 0
    border.color: Theme.danger
    Label {
        id: commandsMark
        anchors.centerIn: parent
        text: ">_"
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontMd
        color: toggle.wrong ? Theme.danger
               : toggle.log !== null && toggle.log.running ? Theme.accent
               : toggle.open ? Theme.textPrimary
               : Theme.textMuted
    }
    // What divides the block from whatever stands beside it — the pane's own ground while the panel is down, the panel
    // itself while it is up, and both are the same value as the pane (`bgBase` = `bgSurface`), so nothing else draws
    // this edge. The fold control draws the same hairline at the other end of the other band (`FoldBlock`).
    Rectangle {
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: Theme.borderWidth
        color: Theme.borderSubtle
    }
    MouseArea {
        id: commandsMouse
        anchors.fill: parent
        hoverEnabled: true
        onClicked: toggle.curPage.toggleCommands()
        ToolTip.visible: containsMouse
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.text: toggle.open
                      ? qsTr("Hide the git commands this window ran")
                      : toggle.wrong
                        ? qsTr("The last command failed — read it here")
                        : qsTr("Show the git commands this window ran")
    }
}
