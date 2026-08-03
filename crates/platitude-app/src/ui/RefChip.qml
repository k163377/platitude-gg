import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One aggregated chip: primary name + "+N". No icons — the kind reads
// through color alone, matching the sidebar header tints (local =
// accent, remote = light blue, detached HEAD = red, tag = amber);
// tags are additionally filled while branches stay outlined.
Rectangle {
    id: chip
    property var records: []
    property bool tagStyle: false
    property real maxWidth: 140

    visible: records.length > 0
    height: Theme.fontSmLine
    width: Math.min(chipContent.implicitWidth + 2 * Theme.spaceXs, maxWidth)
    radius: Theme.radiusSm
    clip: true

    readonly property string rec: records.length > 0 ? records[0] : "L000"
    readonly property string recKind: rec[0]
    readonly property bool recHead: rec[1] === "1"
    readonly property bool recPr: rec.length > 3 && rec[3] === "1"
    readonly property color chipColor: tagStyle ? Theme.warning
                                      : recKind === "R" ? Theme.textSecondary
                                      : recKind === "H" ? Theme.danger
                                      : Theme.accent

    color: tagStyle ? Theme.bgElevated : "transparent"
    border.color: chipColor
    border.width: Theme.borderWidth

    Row {
        id: chipContent
        anchors.verticalCenter: parent.verticalCenter
        anchors.left: parent.left
        anchors.leftMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        Label {
            text: chip.rec.substring(4)
            color: chip.chipColor
            font.pixelSize: Theme.fontSm
            font.weight: chip.recHead ? Font.DemiBold : Font.Normal
            elide: Text.ElideRight
            width: Math.min(implicitWidth,
                            chip.maxWidth - 2 * Theme.spaceXs
                            - (chip.records.length > 1 ? Theme.spaceLg : 0)
                            - (chip.recPr ? Theme.iconSm + Theme.spaceXs : 0))
        }
        Label {
            visible: chip.records.length > 1
            text: "+" + (chip.records.length - 1)
            color: chip.chipColor
            font.pixelSize: Theme.fontSm
        }
        // PR badge: reserved width above, so it survives any elision.
        NavIcon {
            visible: chip.recPr
            anchors.verticalCenter: parent.verticalCenter
            kind: "pr"
            tint: Theme.success
            width: Theme.iconSm
            height: Theme.iconSm
        }
    }
}
