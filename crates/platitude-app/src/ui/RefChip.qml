import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One aggregated chip: primary name + "+N". The kind reads through
// color alone, matching the sidebar header tints (local = accent,
// remote = secondary grey, detached HEAD = red, tag = amber); tags are
// additionally filled while branches stay outlined. The kind is the
// first record's own, not something the caller sets: a row hands over
// everything on it in one list, branches ahead of tags, and the chip
// shows the head of that list. The one icon is the remote/PR badge,
// same mark and same single slot as the sidebar rows — for tags too,
// which is how "this one is only here" reads.
Rectangle {
    id: chip
    property var records: []
    property real maxWidth: 140
    // Nowhere to go from here (the branch already under the working
    // tree): §無効 — the words drop to the muted colour, frame included,
    // since the frame is how a branch chip is read at all.
    property bool muted: false

    visible: records.length > 0
    height: Theme.fontSmLine
    width: Math.min(chipContent.implicitWidth + 2 * Theme.spaceXs, maxWidth)
    radius: Theme.radiusSm
    clip: true

    readonly property string rec: records.length > 0 ? records[0] : "L000"
    readonly property string recKind: rec[0]
    readonly property bool recHead: rec[1] === "1"
    readonly property bool recRemote: rec.length > 2 && rec[2] === "1"
    readonly property bool recPr: rec.length > 3 && rec[3] === "1"
    readonly property bool tagStyle: recKind === "T"
    // One slot, one mark: on the remote, or on the remote with a PR
    // open (§ブランチ状態バッジ — the two never stack).
    readonly property bool hasBadge: recRemote || recPr
    readonly property color chipColor: muted ? Theme.textMuted
                                      : tagStyle ? Theme.warning
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
                            - (chip.hasBadge ? Theme.iconSm + Theme.spaceXs : 0))
        }
        Label {
            visible: chip.records.length > 1
            text: "+" + (chip.records.length - 1)
            color: chip.chipColor
            font.pixelSize: Theme.fontSm
        }
        // Remote / PR badge: reserved width above, so it survives any
        // elision.
        NavIcon {
            visible: chip.hasBadge
            anchors.verticalCenter: parent.verticalCenter
            kind: chip.recPr ? "pr" : "remote"
            tint: chip.muted ? Theme.textMuted
                  : chip.recPr ? Theme.success : Theme.textSecondary
            width: Theme.iconSm
            height: Theme.iconSm
        }
    }
}
