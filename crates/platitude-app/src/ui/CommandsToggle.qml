import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The git commands this tab ran, reached from the band by a mark that also carries the state of the last one
// (デザイン規約 §git が言ったことを読む場所). The only control on the band that is not a button: what it opens is a panel
// rather than an action, and `>_` is the whole of its wording.
Rectangle {
    id: toggle

    /// The RepoPage of the active tab (null while no tab is open).
    property var curPage: null
    /// The padding and height of the action buttons beside it, so the mark is the same size of target they are.
    ///
    /// Measured off the pair rather than written to tokens of its own: what sets how big a target is here is the
    /// padding a Fusion `ToolButton` keeps around its content — a number the theme does not have (sized from the table
    /// it came out 24x20 against the neighbours' 92x32). The button's `padding` rather than the two sides it settles
    /// to: those carry the shared box's slack as well (`ActionButton.slack`), so reading them would move this mark
    /// every time the fetch button changed its wording.
    property real controlPadding: 0
    property real controlHeight: 0

    /// The log this tab is filling. Asked of the log rather than of the page: closing a tab takes the page's models
    /// down while the page itself is still standing, so `curPage !== null` is true for a beat after there is nothing
    /// left to read off it.
    readonly property var log: toggle.curPage !== null ? toggle.curPage.pageCommands : null
    /// The last command failed, or the tab is carrying an error line — one mark for both.
    readonly property bool wrong: toggle.log !== null
        && (toggle.log.failed || toggle.curPage.pageTab.lastError !== "")
    /// Whether the panel is up.
    readonly property bool open: toggle.curPage !== null && toggle.curPage.commandsOpen
    /// Automation: the colour the mark actually painted, which is what says the page's news reached it
    /// (`PG_AUTO_ACT=commands-clear`).
    readonly property alias markColor: commandsMark.color

    visible: toggle.curPage !== null
    implicitWidth: commandsMark.implicitWidth + 2 * toggle.controlPadding
    implicitHeight: toggle.controlHeight
    radius: Theme.radiusSm
    color: toggle.open ? Theme.bgSelected : commandsMouse.containsMouse ? Theme.bgHover : "transparent"
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
