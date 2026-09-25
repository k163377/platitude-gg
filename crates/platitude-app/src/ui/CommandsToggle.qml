import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The seat that opens the git commands this tab ran, at the foot of the left menu. One control in three seats — a
// section band, the folded rail's end block, the head of the log panel's band — dressed by `captioned` / `ruled` /
// `markSize` (デザイン規約 §git が言ったことを読む場所).
Rectangle {
    id: toggle

    /// The RepoPage this seat belongs to (null while no tab is open).
    property var curPage: null
    /// Whether the log's name stands beside the mark. False in the folded rail, which has no room for it.
    property bool captioned: false
    /// Whether a hairline closes the seat off from what is above it. False in the panel's band, which it belongs to.
    property bool ruled: true
    /// The mark's size, matching what it stands among: `iconMd` beside a name like the sections' marks, `iconLg`
    /// alone at the rail's end like the fold control.
    property real markSize: Theme.iconMd

    /// The log this tab is filling. Test this, not `curPage`: closing a tab takes the page's models down a beat
    /// before the page itself.
    readonly property var log: toggle.curPage !== null ? toggle.curPage.pageCommands : null
    /// The last command failed, or the tab carries an error line — one mark for both. The rule is the page's, since
    /// this mark moves between seats.
    readonly property bool wrong: toggle.curPage !== null && toggle.curPage.commandsWrong
    readonly property bool open: toggle.curPage !== null && toggle.curPage.commandsOpen
    /// Automation: the colour the mark actually painted — proof the page's state reached it
    /// (`PGG_AUTO_ACT=commands-clear`).
    readonly property alias markColor: commandsMark.tint

    /// Where the mark stands and the name begins: a section's row without the fold arrow's column.
    readonly property real markX: Theme.spaceXs
    readonly property real captionX: toggle.markX + Theme.iconMd + Theme.spaceXs

    visible: toggle.curPage !== null
    // As wide as its name when captioned: the log's band seats it in a row of controls, and a pane-wide seat would
    // push them off the end.
    implicitWidth: toggle.captioned ? toggle.captionX + commandsName.implicitWidth : Theme.railWidth
    implicitHeight: toggle.captioned ? Theme.rowHeight : Theme.headerHeight
    // The ground of all three seats, so none of them needs a case.
    color: Theme.bgElevated
    // `bgHover` is translucent, so it is layered over that ground rather than replacing it.
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
    // Goes red with the mark (デザイン規約 §git が言ったことを読む場所).
    Label {
        id: commandsName
        visible: toggle.captioned
        x: toggle.captionX
        anchors.verticalCenter: parent.verticalCenter
        text: Words.commandsTitle
        font.pixelSize: Theme.fontMd
        font.weight: Font.DemiBold
        color: toggle.wrong ? Theme.danger : Theme.textSecondary
    }
    MouseArea {
        id: commandsMouse
        anchors.fill: parent
        hoverEnabled: true
        onClicked: toggle.curPage.toggleCommands()
        // Only where the mark stands alone — a captioned seat already says it (規約 §hover のツールチップ).
        ToolTip.visible: containsMouse && !toggle.captioned
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.text: toggle.open
                      ? qsTr("Hide the git commands this window ran")
                      : toggle.wrong
                        ? qsTr("The last command failed — read it here")
                        : qsTr("Show the git commands this window ran")
    }
}
