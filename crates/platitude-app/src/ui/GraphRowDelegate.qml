pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Commit-graph row: [branch/tag chips][lanes + identicon node][subject]
// — fixed-width label and graph columns keep subjects aligned. Column
// geometry arrives through properties on the owning ListView
// (labelWidth / graphColWidth / graphFullWidth / graphXOffset /
// wipCount), and clicks go back up through its rowSelected /
// rowMenuRequested signals.
Item {
    id: rowItem
    required property int index
    required property string oid_hex
    required property string author
    required property double atime
    required property string subject
    required property int node_lane
    required property int node_color
    required property int avatar
    required property string geometry
    required property string labels
    required property string stash_ref

    width: ListView.view.width
    height: Theme.graphRowHeight

    readonly property bool selected: ListView.isCurrentItem
    // The top row's highlight and hit area bleed over the list's top
    // margin: hovering or selecting the first commit shows one
    // unbroken band level with the neighbouring header bands instead
    // of leaving a dark sliver above the row.
    readonly property real topBleed: index === 0 && ListView.view
                                     ? ListView.view.topMargin : 0
    // The all-zero id marks the synthetic uncommitted-changes row.
    readonly property bool isWip: oid_hex !== "" && !/[^0]/.test(oid_hex)
    // Chip records are separated by U+001F (see encode.rs). Branch-like
    // records (HEAD / local / remote) and tags get separate chips.
    readonly property var labelRecords: labels === "" ? [] : labels.split(String.fromCharCode(31))
    readonly property var branchRecords: labelRecords.filter(r => r[0] !== "T")
    readonly property var tagRecords: labelRecords.filter(r => r[0] === "T")

    Rectangle {
        anchors.fill: parent
        anchors.topMargin: -rowItem.topBleed
        color: Theme.bgSelected
        visible: rowItem.selected
    }
    Rectangle {
        anchors.fill: parent
        anchors.topMargin: -rowItem.topBleed
        color: Theme.bgHover
        visible: rowMouse.containsMouse && !rowItem.selected
    }

    onGeometryChanged: laneCanvas.requestPaint()
    onNode_laneChanged: laneCanvas.requestPaint()
    onAvatarChanged: laneCanvas.requestPaint()

    // Column widths come from the ListView (owner scope).
    readonly property real labelsW: ListView.view ? ListView.view.labelWidth : Metrics.labelColW

    RowLayout {
        anchors.fill: parent
        spacing: 0

        // Branch / tag chips, right-aligned against the graph.
        Item {
            Layout.preferredWidth: rowItem.labelsW
            Layout.fillHeight: true
            Row {
                anchors.right: parent.right
                anchors.rightMargin: Theme.spaceXs
                anchors.verticalCenter: parent.verticalCenter
                spacing: Theme.spaceXs
                RefChip {
                    records: rowItem.branchRecords
                    tagStyle: false
                    maxWidth: rowItem.tagRecords.length > 0
                              ? (rowItem.labelsW - Theme.spaceSm) / 2
                              : rowItem.labelsW - Theme.spaceSm
                }
                RefChip {
                    records: rowItem.tagRecords
                    tagStyle: true
                    maxWidth: rowItem.branchRecords.length > 0
                              ? (rowItem.labelsW - Theme.spaceSm) / 2
                              : rowItem.labelsW - Theme.spaceSm
                }
            }
            MouseArea {
                id: labelHover
                anchors.fill: parent
                hoverEnabled: true
                acceptedButtons: Qt.NoButton
            }
            ToolTip.visible: labelHover.containsMouse && rowItem.labelRecords.length > 0
            ToolTip.delay: 300
            ToolTip.text: {
                let lines = []
                for (let i = 0; i < rowItem.labelRecords.length; i++) {
                    const r = rowItem.labelRecords[i]
                    const icon = r[0] === "T" ? "⚑" : r[0] === "R" ? "☁"
                               : r[0] === "H" ? "HEAD" : "⎇"
                    // Aggregated records keep their PR mark visible here.
                    const pr = r.length > 3 && r[3] === "1" ? qsTr(" · PR") : ""
                    lines.push(icon + " " + r.substring(4) + pr)
                }
                return lines.join("\n")
            }
        }

        // Lanes viewport: the full-width canvas slides behind a clip
        // when the graph column scrolls horizontally.
        Item {
            Layout.preferredWidth: rowItem.ListView.view
                                   ? rowItem.ListView.view.graphColWidth : 120
            Layout.fillHeight: true
            clip: true
            Canvas {
                id: laneCanvas
                x: rowItem.ListView.view ? -rowItem.ListView.view.graphXOffset : 0
                width: rowItem.ListView.view
                       ? rowItem.ListView.view.graphFullWidth : 120
                height: parent.height
                onPaint: {
                    const ctx = getContext("2d")
                    ctx.clearRect(0, 0, width, height)
                    ctx.lineWidth = Metrics.laneStroke
                    const laneCount = Theme.graphLane.length
                    const cx = function (l) { return Metrics.laneInset + l * Metrics.laneW + Metrics.laneW / 2 }
                    const midY = height / 2
                    const nodeX = cx(rowItem.node_lane)
                    // decode precomputed draw tokens: t/i/o + lane + color
                    // (uppercase = dashed WIP edge)
                    if (rowItem.geometry !== "") {
                        const toks = rowItem.geometry.split(";")
                        for (let n = 0; n < toks.length; n++) {
                            const t = toks[n]
                            const k = t[0].toLowerCase()
                            const dot = t.indexOf(".")
                            const lane = parseInt(t.substring(1, dot))
                            const x = cx(lane)
                            ctx.strokeStyle = Theme.graphLane[parseInt(t.substring(dot + 1)) % laneCount]
                            ctx.setLineDash(t[0] === k ? [] : Metrics.laneDash)
                            ctx.beginPath()
                            if (k === "t") {
                                ctx.moveTo(x, 0)
                                ctx.lineTo(x, height)
                            } else if (k === "i") {
                                ctx.moveTo(x, 0)
                                ctx.bezierCurveTo(x, midY * 0.66, nodeX, midY * 0.34, nodeX, midY)
                            } else {
                                ctx.moveTo(nodeX, midY)
                                ctx.bezierCurveTo(nodeX, height - midY * 0.34,
                                                  x, height - midY * 0.66, x, height)
                            }
                            ctx.stroke()
                        }
                        ctx.setLineDash([])
                    }
                    // The WIP row has no commit and no author: a dashed,
                    // empty node instead of the identicon.
                    const r = Metrics.nodeIcon / 2
                    if (rowItem.isWip) {
                        ctx.strokeStyle = Theme.textSecondary
                        ctx.lineWidth = Metrics.laneStroke
                        ctx.setLineDash(Metrics.laneDash)
                        ctx.beginPath()
                        ctx.arc(nodeX, midY, r - 1, 0, 2 * Math.PI)
                        ctx.stroke()
                        ctx.setLineDash([])
                        return
                    }
                    // Stash rows draw the archive-box glyph instead of the
                    // author identicon.
                    if (rowItem.stash_ref !== "") {
                        ctx.fillStyle = Theme.bgElevated
                        ctx.beginPath()
                        ctx.arc(nodeX, midY, r - 1, 0, 2 * Math.PI)
                        ctx.fill()
                        ctx.strokeStyle = Theme.textSecondary
                        ctx.lineWidth = Metrics.iconStroke
                        const bw = r * 1.2
                        ctx.strokeRect(nodeX - bw / 2, midY - bw / 2, bw, bw * 0.36)
                        ctx.strokeRect(nodeX - bw * 0.4, midY - bw * 0.1, bw * 0.8, bw * 0.58)
                        // Dashed ring like the WIP node: not part of the
                        // committed history proper.
                        ctx.strokeStyle = Theme.textSecondary
                        ctx.lineWidth = Metrics.laneStroke
                        ctx.setLineDash(Metrics.laneDash)
                        ctx.beginPath()
                        ctx.arc(nodeX, midY, r - 1, 0, 2 * Math.PI)
                        ctx.stroke()
                        ctx.setLineDash([])
                        return
                    }
                    // The commit node is the author's identicon (5x5,
                    // mirrored; local substitute for network avatars). The
                    // pattern uses only the inner part of the circle so the
                    // clip cuts less of it.
                    ctx.save()
                    ctx.beginPath()
                    ctx.arc(nodeX, midY, r, 0, 2 * Math.PI)
                    ctx.clip()
                    ctx.fillStyle = Theme.bgElevated
                    ctx.fillRect(nodeX - r, midY - r, 2 * r, 2 * r)
                    ctx.fillStyle = Theme.graphLane[(rowItem.avatar >> 15) & 0x7]
                    const inner = 2 * r * Metrics.identiconFill
                    const cell = inner / 5
                    const ox = nodeX - inner / 2
                    const oy = midY - inner / 2
                    for (let row = 0; row < 5; row++) {
                        for (let col = 0; col < 3; col++) {
                            if ((rowItem.avatar >> (row * 3 + col)) & 1) {
                                ctx.fillRect(ox + col * cell, oy + row * cell,
                                             cell + 0.5, cell + 0.5)
                                if (col < 2)
                                    ctx.fillRect(ox + (4 - col) * cell, oy + row * cell,
                                                 cell + 0.5, cell + 0.5)
                            }
                        }
                    }
                    ctx.restore()
                    ctx.strokeStyle = Theme.borderStrong
                    ctx.lineWidth = Theme.borderWidth
                    ctx.beginPath()
                    ctx.arc(nodeX, midY, r, 0, 2 * Math.PI)
                    ctx.stroke()
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Theme.spaceXs
            // Short colored tick before the message: separates rows
            // visually (deliberately not a continuous line) and echoes
            // the commit's chain color.
            Rectangle {
                Layout.leftMargin: Theme.spaceSm
                implicitWidth: 2 * Theme.borderWidth
                implicitHeight: Theme.iconMd
                radius: Theme.borderWidth
                color: Theme.graphLane[rowItem.node_color % Theme.graphLane.length]
            }
            Label {
                Layout.fillWidth: true
                text: rowItem.isWip
                      ? qsTr("Uncommitted changes (%1)")
                        .arg(rowItem.ListView.view ? rowItem.ListView.view.wipCount : 0)
                      : rowItem.subject
                elide: Text.ElideRight
                font.pixelSize: Theme.fontMd
                color: rowItem.isWip ? Theme.textSecondary : Theme.textPrimary
                rightPadding: Theme.spaceSm
            }
        }
    }

    MouseArea {
        id: rowMouse
        anchors.fill: parent
        anchors.topMargin: -rowItem.topBleed
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: mouse => {
            rowItem.ListView.view.currentIndex = rowItem.index
            rowItem.ListView.view.rowSelected(rowItem.oid_hex)
            // The synthetic WIP row is not a commit, so nothing in the
            // commit menu applies to it.
            if (mouse.button === Qt.RightButton && !rowItem.isWip)
                rowItem.ListView.view.rowMenuRequested(rowItem.oid_hex)
        }
    }
    // Hover details: what the row no longer shows as columns.
    ToolTip.visible: rowMouse.containsMouse
    ToolTip.delay: 700
    ToolTip.text: rowItem.isWip
                  ? qsTr("Working-tree changes — not committed yet")
                  : rowItem.author + "\n"
                    + Qt.formatDateTime(new Date(rowItem.atime * 1000), "yyyy-MM-dd HH:mm") + "\n"
                    + rowItem.oid_hex.substring(0, 8)
}
