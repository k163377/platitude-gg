import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The seat that opens the git commands this tab ran, at the foot of the left menu. One control in three seats — a
// section band (`SidebarPane` open, `OpenFailedScreen`), the folded rail's end block (`SidebarPane` folded), the head
// of the log panel's band (captioned with `ruled: false`, first in `CommandsPane`'s `RowLayout`, its `(N)` inside the
// seat) — dressed by `captioned` / `ruled` / `markSize` (デザイン規約 §git が言ったことを読む場所).
//
// The foot of the page's left menu, not the window's bottom edge: the log is a tab's, so its way in sits on the tab's
// side. The seat wears whatever the pane is wearing (user decision — ask before changing). Open, a section's band —
// `rowHeight`, the heading's mark (`iconMd` box) and `GIT COMMANDS` in the heading's type — except that the mark's box
// keeps to the left edge (`spaceXs`): the fold arrow's column belongs to rows with a section to open (user decision).
// Folded, the fold block's own box and step (`railWidth` × `headerHeight`, the mark `iconLg` in the middle, no count),
// so the rail's two ends match (user decision); a cell's `iconXl` is something else — the cell's content, not a mark
// on a band. One ground for every seat, the bands' `bgElevated` (it is read beside the section bands and the fold
// block), the hover one wash over it; no frame and no corners — a `borderSubtle` hairline above, as under the fold
// block, is the edge with the list.
//
// Hand-built `Rectangle` + `MouseArea`, not `HoverToolButton`: that base has no seat for a mark in four colours
// (wrong / running / open / plain), and a replaced `contentItem` still keeps `ActionButton`'s press-and-release dress.
// Moving it onto the base means opening a four-value port there first.
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
    /// Whether `(count)` follows the name — inside the seat, so the hover and the press that closes the log take it
    /// too. Only the panel's band has a list to count.
    property bool counted: false
    property int count: 0

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

    /// Where the mark stands and the name begins: a section's row without the fold arrow's column. Written as the sum
    /// of the steps, never a copied number — that would drift silently when the pane's inner size moves.
    readonly property real markX: Theme.spaceXs
    readonly property real captionX: toggle.markX + Theme.iconMd + Theme.spaceXs

    visible: toggle.curPage !== null
    // As wide as its name when captioned: the log's band seats it in a row of controls, and a pane-wide seat would
    // push them off the end. The hover's ground ends as far past the last letter as it starts before the mark.
    implicitWidth: !toggle.captioned ? Theme.railWidth
                   : (toggle.counted ? commandsCount.x + commandsCount.implicitWidth
                                     : toggle.captionX + commandsName.implicitWidth) + toggle.markX
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
    // Captioned, the heading's mark: a step over its capitals, its box the caption's line so the two share a baseline
    // (デザイン規約 §アイコンの席 の文頭記号の見出し). On the folded rail it stands alone in its cell.
    NavIcon {
        id: commandsMark
        kind: "terminal"
        set: toggle.captioned ? "head" : ""
        size: commandsName.font.pixelSize
        wordBase: commandsName.baselineOffset + commandsName.anchors.verticalCenterOffset
        width: toggle.markSize
        height: toggle.captioned ? commandsName.height : toggle.markSize
        x: toggle.captioned ? toggle.markX + (Theme.iconMd - toggle.markSize) / 2
                            : (Theme.railWidth - toggle.markSize) / 2
        anchors.verticalCenter: parent.verticalCenter
        tint: toggle.wrong ? Theme.danger
              : toggle.log !== null && toggle.log.running ? Theme.accent
              : toggle.open ? Theme.textPrimary
              : Theme.textMuted
    }
    // Goes red with the mark: the mark's colour is all the seat says of the last command, and one red mark in a row of
    // plain words does not read as a failure.
    Label {
        id: commandsName
        visible: toggle.captioned
        x: toggle.captionX
        anchors.verticalCenter: parent.verticalCenter
        // Centred on its capitals, not on its line box (`NavHeader.capsLift`).
        anchors.verticalCenterOffset: -WordFace.capitalDrop(commandsName)
        text: Words.commandsTitle
        font.pixelSize: Theme.fontMd
        font.weight: Theme.fontWeightStrong
        color: toggle.wrong ? Theme.danger : Theme.textSecondary
    }
    // The sections' step, so the count sits as `(2)` does after `BRANCHES`.
    Label {
        id: commandsCount
        visible: toggle.captioned && toggle.counted
        x: toggle.captionX + commandsName.implicitWidth + Theme.spaceXs
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: commandsName.anchors.verticalCenterOffset
        text: "(" + toggle.count + ")"
        font.pixelSize: Theme.fontMd
        color: Theme.textMuted
    }
    MouseArea {
        id: commandsMouse
        anchors.fill: parent
        hoverEnabled: true
        onClicked: toggle.curPage.toggleCommands()
        readonly property TipAsk toggleTip: TipAsk {
            target: commandsMouse
            // Only where the mark stands alone — a captioned seat already says it (規約 §hover のツールチップ).
            asked: commandsMouse.containsMouse && !toggle.captioned
            words: toggle.open
                   ? qsTr("Hide the git commands this window ran")
                   : toggle.wrong
                     ? qsTr("The last command failed — read it here")
                     : qsTr("Show the git commands this window ran")
        }
    }
}
