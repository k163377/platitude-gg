import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The seat that opens the list of what destructive operations took away (破棄記録仕様.md), stacked on the git
// commands' seat at the foot of the left menu and dressed as that seat, colours included: a section band while the
// menu is open, the rail's end block folded, and the head of the list's own band while the list is up
// (`CommandsToggle`).
Rectangle {
    id: toggle

    /// The RepoPage this seat belongs to (null while no tab is open).
    property var curPage: null
    /// Whether the list's name stands beside the mark. False in the folded rail, which has no room for it.
    property bool captioned: false
    /// Whether a hairline closes the seat off from what is above it. False in the list's band, which it belongs to.
    property bool ruled: true
    /// The mark's size, matching what it stands among (`CommandsToggle.markSize`).
    property real markSize: Theme.iconMd
    /// Lit as if something had just been thrown away, to say where it went: the wash a left menu row wears under the
    /// hand or open, with the mark and the name in the colours they always wear.
    property bool lit: false

    readonly property bool open: toggle.curPage !== null && toggle.curPage.recoverOpen

    /// Where the mark stands and the name begins: a section's row without the fold arrow's column.
    readonly property real markX: Theme.spaceXs
    readonly property real captionX: toggle.markX + Theme.iconMd + Theme.spaceXs

    visible: toggle.curPage !== null
    // No count beside the name: it would grow with the record and take the band's room from the name.
    implicitWidth: toggle.captioned ? toggle.captionX + recoverName.implicitWidth + Theme.spaceXs : Theme.railWidth
    implicitHeight: toggle.captioned ? Theme.rowHeight : Theme.headerHeight
    color: Theme.bgElevated
    // One wash for both, layered over the ground as `bgHover` is everywhere: lit and under the hand reads the same.
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: toggle.lit || recoverMouse.containsMouse
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
        kind: "history"
        width: toggle.markSize
        height: toggle.markSize
        x: toggle.captioned ? toggle.markX + (Theme.iconMd - toggle.markSize) / 2
                            : (Theme.railWidth - toggle.markSize) / 2
        anchors.verticalCenter: parent.verticalCenter
        tint: toggle.open ? Theme.textPrimary : Theme.textMuted
    }
    Label {
        id: recoverName
        visible: toggle.captioned
        x: toggle.captionX
        anchors.verticalCenter: parent.verticalCenter
        // Plural, as the sections' names are: the things thrown away, one entry each.
        text: qsTr("DISCARDS")
        font.pixelSize: Theme.fontMd
        font.weight: Theme.fontWeightStrong
        color: Theme.textSecondary
    }
    MouseArea {
        id: recoverMouse
        anchors.fill: parent
        hoverEnabled: true
        onClicked: toggle.curPage.toggleRecover()
        // Only where the mark stands alone — a captioned seat already says it (規約 §hover のツールチップ).
        ToolTip.visible: containsMouse && !toggle.captioned
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.text: toggle.open ? qsTr("Hide what was thrown away")
                                  : qsTr("Show what was thrown away, to bring it back")
    }
}
