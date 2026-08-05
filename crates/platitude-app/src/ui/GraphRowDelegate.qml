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
    // Chip records are separated by U+001F (see encode.rs), and arrive
    // in the order the chip reads them out: HEAD → local → remote → tag.
    // One chip for the row, so a commit that is both a branch tip and a
    // release shows the branch — the tag is behind the "+N", where the
    // hover card has it.
    readonly property var labelRecords: labels === "" ? [] : labels.split(String.fromCharCode(31))
    readonly property var branchRecords: labelRecords.filter(r => r[0] !== "T")
    // Whether this row is somewhere HEAD could stand: the working-tree
    // row is not a commit, and a stash sits on no branch's history.
    readonly property bool movable: !rowItem.isWip && rowItem.stash_ref === ""
    // Where a double-click on this row goes: the branch chip's own first
    // record, so what is on screen is what is moved to. Empty means the
    // row shows no branch, which is the offer to put one there.
    readonly property string primaryRecord:
        rowItem.movable && rowItem.branchRecords.length > 0 ? rowItem.branchRecords[0] : ""
    // The chip itself — what a stacked one is unstacked under.
    readonly property alias chipItem: rowChip
    // This row's chip column is a branch-name box right now.
    readonly property bool naming:
        rowItem.ListView.view ? rowItem.ListView.view.namingOid === rowItem.oid_hex : false
    // ... or a yes-strip.
    readonly property bool asking:
        rowItem.ListView.view ? rowItem.ListView.view.askOid === rowItem.oid_hex : false

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

        // The row's chip, right-aligned against the graph — or, on a
        // row with no branch to move to, the box that names one here.
        Item {
            Layout.preferredWidth: rowItem.labelsW
            Layout.fillHeight: true
            RefChip {
                id: rowChip
                // Assigning `visible` here replaces the chip's own rule,
                // so the "has anything to show" half has to be repeated:
                // without it a row with no refs draws an empty frame.
                visible: rowItem.labelRecords.length > 0
                         && !rowItem.naming && !rowItem.asking
                anchors.right: parent.right
                anchors.rightMargin: Theme.spaceXs
                anchors.verticalCenter: parent.verticalCenter
                records: rowItem.labelRecords
                // One chip now has the column to itself; the names it
                // cannot fit are read in the card, not squeezed here.
                maxWidth: rowItem.labelsW - Theme.spaceSm
            }
            // A row with nothing to move to answers the double-click with
            // the one thing that would give it something: a name. The
            // question is asked where the chips would be, not over the
            // window (デザイン規約: 表示の切り替えで足りるならダイアログを出さない).
            SlimField {
                id: nameField
                visible: rowItem.naming
                anchors.right: parent.right
                anchors.rightMargin: Theme.spaceXs
                anchors.verticalCenter: parent.verticalCenter
                width: rowItem.labelsW - 2 * Theme.spaceXs
                font.pixelSize: Theme.fontSm
                placeholderText: qsTr("Create branch here?")
                onAccepted: rowItem.ListView.view.namingSubmitted(
                    rowItem.oid_hex, nameField.text.trim())
                // Held on the view, not here: this delegate is recycled
                // the moment the row scrolls off, and half a name is
                // still worth not losing.
                onTextEdited: rowItem.ListView.view.namingText = nameField.text
                Keys.onEscapePressed: rowItem.ListView.view.namingCancelled()
            }
            // A question about this row stands where its chips were:
            // clicking the strip is the answer, Escape or any other
            // click walks away, and nothing happens until one of the
            // two (デザイン規約 §可否・警告の出し場所). What the answer
            // costs is on the strip's own tooltip.
            Rectangle {
                id: askStrip
                visible: rowItem.asking
                anchors.right: parent.right
                anchors.rightMargin: Theme.spaceXs
                anchors.verticalCenter: parent.verticalCenter
                width: rowItem.labelsW - 2 * Theme.spaceXs
                height: Theme.fontSmLine
                radius: Theme.radiusSm
                color: askMouse.containsMouse ? Theme.bgHover : "transparent"
                readonly property color tone:
                    rowItem.ListView.view && rowItem.ListView.view.askDanger
                    ? Theme.danger : Theme.warning
                border.color: tone
                border.width: Theme.borderWidth
                Label {
                    anchors.fill: parent
                    leftPadding: Theme.spaceXs
                    rightPadding: Theme.spaceXs
                    text: rowItem.ListView.view ? rowItem.ListView.view.askLabel : ""
                    color: askStrip.tone
                    font.pixelSize: Theme.fontSm
                    elide: Text.ElideRight
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
                MouseArea {
                    id: askMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: rowItem.ListView.view.askConfirmed(rowItem.oid_hex)
                }
                ToolTip.visible: askMouse.containsMouse
                ToolTip.delay: 300
                ToolTip.text: rowItem.ListView.view ? rowItem.ListView.view.askDetail : ""
                Keys.onEscapePressed: rowItem.ListView.view.askCancelled()
            }
            MouseArea {
                id: labelHover
                anchors.fill: parent
                hoverEnabled: true
                acceptedButtons: Qt.NoButton
            }
            ToolTip.visible: labelHover.containsMouse && !rowItem.asking
                             && rowItem.labelRecords.length > 0
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

    // The box carries on from wherever the last delegate to hold it left
    // off — including a fresh one, when the row is scrolled back into
    // view mid-name.
    onNamingChanged: rowItem.takeNamingFocus()
    Component.onCompleted: {
        rowItem.takeNamingFocus()
        rowItem.takeAskFocus()
    }
    function takeNamingFocus() {
        if (!rowItem.naming || !rowItem.ListView.view)
            return
        nameField.text = rowItem.ListView.view.namingText
        nameField.forceActiveFocus()
    }
    // Same for the strip: Escape has to land somewhere.
    onAskingChanged: rowItem.takeAskFocus()
    function takeAskFocus() {
        if (rowItem.asking)
            askStrip.forceActiveFocus()
    }

    // Which stacked chip the pointer is over, if it is over one that has
    // something to unstack. Worked out from the row's own coordinates
    // rather than a hover area inside the chip: this one is on top, so it
    // is the one that hears about the pointer at all.
    property Item hoveredChip: null
    function chipUnder(px, py) {
        if (!rowChip.visible || rowChip.records.length < 2)
            return null
        const p = rowItem.mapToItem(rowChip, px, py)
        return rowChip.contains(Qt.point(p.x, p.y)) ? rowChip : null
    }
    // Opens and closes with the pointer, with no wait either way: only a
    // chip with something stacked behind it answers at all, so there is
    // nothing to open by accident on the way past.
    function noteChipHover(px, py) {
        const chip = rowItem.naming ? null : rowItem.chipUnder(px, py)
        if (chip === rowItem.hoveredChip || !rowItem.ListView.view)
            return
        rowItem.hoveredChip = chip
        if (chip)
            rowItem.ListView.view.chipExpandRequested(chip.records, chip)
        else
            rowItem.ListView.view.chipCollapseRequested()
    }

    MouseArea {
        id: rowMouse
        anchors.fill: parent
        anchors.topMargin: -rowItem.topBleed
        // While the box is open the chip column belongs to it: this area
        // is painted over everything in the row, so anything under it
        // would never see a click of its own.
        anchors.leftMargin: rowItem.naming ? rowItem.labelsW : 0
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
        // Where the row leads: the chip it shows, or — with no branch on
        // it — the offer to put one there. The page decides which.
        onDoubleClicked: mouse => {
            if (mouse.button !== Qt.LeftButton || !rowItem.movable)
                return
            rowItem.ListView.view.rowSwitchRequested(
                rowItem.oid_hex, rowItem.primaryRecord)
        }
        onPositionChanged: mouse => rowItem.noteChipHover(mouse.x, mouse.y)
        onContainsMouseChanged: {
            if (!rowMouse.containsMouse)
                rowItem.noteChipHover(-1, -1)
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
