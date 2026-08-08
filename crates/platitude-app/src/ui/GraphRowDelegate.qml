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
    /// A picture this author was given, or empty for the generated
    /// pattern. Resolved in Rust onto the row (models::graph).
    required property string avatar_url
    /// Packed `Co-authored-by` records; the first one badges the node.
    required property string co_authors
    readonly property var mateRecords:
        rowItem.co_authors === ""
        ? [] : rowItem.co_authors.split(String.fromCharCode(31))
    /// Identicon code of the first co-author, or 0 when nobody is credited.
    readonly property int mateFace:
        rowItem.mateRecords.length === 0
        ? 0 : parseInt(rowItem.mateRecords[0].split(String.fromCharCode(30))[2])
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
    // The standing question is about this row. Its words are on the bar
    // above the graph; the row answers "which one" and nothing else.
    readonly property bool marked:
        rowItem.ListView.view ? rowItem.ListView.view.askOid === rowItem.oid_hex
                              : false

    /// The record for the branch the working tree is on. The head flag
    /// rides on the detached-HEAD marker as well, and that one is a state
    /// rather than a branch, so it keeps its own colour.
    function isCurrentRecord(record) {
        return record[1] === "1" && record[0] !== "H"
    }
    /// Rich-text escaping, for names that share a tooltip with a coloured
    /// line.
    function escapeMarkup(text) {
        return text.replace(/&/g, "&amp;").replace(/</g, "&lt;")
                   .replace(/>/g, "&gt;")
    }

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
    // The row the standing question is about: the tone of the bar, run
    // down the edge the rows begin at.
    Rectangle {
        anchors.fill: parent
        anchors.topMargin: -rowItem.topBleed
        color: Theme.bgHover
        visible: rowItem.marked
        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: Metrics.laneStroke
            color: rowItem.ListView.view && rowItem.ListView.view.askDanger
                   ? Theme.danger : Theme.warning
        }
    }

    onGeometryChanged: laneCanvas.requestPaint()
    onNode_laneChanged: laneCanvas.requestPaint()
    onAvatarChanged: laneCanvas.requestPaint()
    // Assigning a picture changes no history, so this role moves on rows
    // that are otherwise untouched — and a canvas repaints only when it
    // is asked to.
    onAvatar_urlChanged: rowItem.loadFace()
    function loadFace() {
        if (rowItem.avatar_url !== "")
            laneCanvas.loadImage(rowItem.avatar_url)
        laneCanvas.requestPaint()
    }
    // A row coming back out of the reuse pool has to redraw even when
    // none of the three roles above differs from the row it was last
    // used for: what it holds is the picture it was left with, and the
    // graph may have grown a lane in the meantime — see the canvas.
    ListView.onReused: rowItem.loadFace()

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
                visible: rowItem.labelRecords.length > 0 && !rowItem.naming
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
            MouseArea {
                id: labelHover
                anchors.fill: parent
                hoverEnabled: true
                acceptedButtons: Qt.NoButton
            }
            ToolTip.visible: labelHover.containsMouse
                             && rowItem.labelRecords.length > 0
            ToolTip.delay: Metrics.tipDelayMs
            // The branch the working tree stands on is written here the
            // way the sidebar and the chips write it (textLink): the same
            // name must not read as "here" in one place and as any other
            // branch in the next. Colouring one line puts the whole
            // tooltip through rich text, so in that case every name is
            // escaped first — a refname may hold & and <.
            ToolTip.text: {
                let lines = []
                const colour = rowItem.labelRecords.some(rowItem.isCurrentRecord)
                for (let i = 0; i < rowItem.labelRecords.length; i++) {
                    const r = rowItem.labelRecords[i]
                    // Kind words, not glyphs: a tooltip is a string, so
                    // there is nothing to draw on, and ⚑ / ☁ / ⎇ are
                    // exactly what the named families do not all carry
                    // (デザイン規約 §QML実装ルール 印は描く).
                    const kind = r[0] === "T" ? qsTr("tag")
                               : r[0] === "R" ? qsTr("remote")
                               : r[0] === "H" ? "HEAD" : qsTr("branch")
                    // Aggregated records keep their PR mark visible here.
                    const pr = r.length > 3 && r[3] === "1" ? qsTr(" · PR") : ""
                    const bare = r.substring(5).split("\u001E")[0]
                    const name = colour ? rowItem.escapeMarkup(bare) : bare
                    const line = kind + " " + name + pr
                    lines.push(rowItem.isCurrentRecord(r)
                               ? "<font color=\"" + Theme.textLink + "\">"
                                 + line + "</font>"
                               : line)
                }
                return lines.join(colour ? "<br>" : "\n")
            }
        }

        // Lanes viewport: the full-width canvas slides behind a clip
        // when the graph column scrolls horizontally.
        Item {
            Layout.preferredWidth: rowItem.ListView.view
                                   ? rowItem.ListView.view.graphColWidth : 0
            Layout.fillHeight: true
            clip: true
            Canvas {
                id: laneCanvas
                x: rowItem.ListView.view ? -rowItem.ListView.view.graphXOffset : 0
                width: rowItem.ListView.view
                       ? rowItem.ListView.view.graphFullWidth : 0
                height: parent.height
                // Resizing a canvas scales what it already holds; only a
                // repaint redraws it. Qt asks for that repaint itself,
                // but only for a canvas that is visible — and a row
                // waiting in the ListView's reuse pool is not. So a graph
                // that grows a lane while a row is pooled brings that row
                // back with its lanes and node stretched sideways
                // (2026-08-08, seen after switching windows: the pool is
                // where rows sit while another window is in front).
                onWidthChanged: requestPaint()
                onHeightChanged: requestPaint()
                // A row coming back from the reuse pool carries a new
                // author, so the picture it holds is loaded again before
                // the paint that would draw it.
                onImageLoaded: requestPaint()
                /// One round face: the assigned picture when there is a
                /// url for it, the generated pattern otherwise. Shared by
                /// the node and the co-author badge so the two cannot
                /// drift apart in shape or outline.
                function face(ctx, cx, cy, radius, code, url) {
                    ctx.save()
                    ctx.beginPath()
                    ctx.arc(cx, cy, radius, 0, 2 * Math.PI)
                    ctx.clip()
                    if (url !== "") {
                        if (laneCanvas.isImageLoaded(url)) {
                            ctx.drawImage(url, cx - radius, cy - radius,
                                          2 * radius, 2 * radius)
                        }
                    } else {
                        ctx.fillStyle = Theme.bgElevated
                        ctx.fillRect(cx - radius, cy - radius,
                                     2 * radius, 2 * radius)
                        ctx.fillStyle = Theme.graphLane[(code >> 15) & 0x7]
                        const inner = 2 * radius * Metrics.identiconFill
                        const cell = inner / 5
                        const ox = cx - inner / 2
                        const oy = cy - inner / 2
                        for (let row = 0; row < 5; row++) {
                            for (let col = 0; col < 3; col++) {
                                if ((code >> (row * 3 + col)) & 1) {
                                    ctx.fillRect(ox + col * cell, oy + row * cell,
                                                 cell + 0.5, cell + 0.5)
                                    if (col < 2)
                                        ctx.fillRect(ox + (4 - col) * cell,
                                                     oy + row * cell,
                                                     cell + 0.5, cell + 0.5)
                                }
                            }
                        }
                    }
                    ctx.restore()
                    ctx.strokeStyle = Theme.borderStrong
                    ctx.lineWidth = Theme.borderWidth
                    ctx.beginPath()
                    ctx.arc(cx, cy, radius, 0, 2 * Math.PI)
                    ctx.stroke()
                }
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
                    // Stash rows draw the bare archive box of the STASHES
                    // section (NavIcon "stash", same 16-unit grid) instead
                    // of the author identicon — no ring around it, so the
                    // node reads as the icon and nothing else. The glyph is
                    // centered on the node point (its own middle, 8.5 of the
                    // grid, not the grid's), and its footprint is cleared
                    // first so the dashed leash comes out from under the box
                    // instead of running through it.
                    if (rowItem.stash_ref !== "") {
                        const s = Metrics.nodeIcon / 16
                        const gx = nodeX - 8 * s
                        const gy = midY - 8.5 * s
                        ctx.clearRect(gx + 3 * s, gy + 4 * s, 10 * s, 9 * s)
                        ctx.strokeStyle = Theme.textSecondary
                        ctx.lineWidth = Metrics.iconStroke
                        ctx.strokeRect(gx + 3 * s, gy + 4 * s, 10 * s, 3 * s)
                        ctx.strokeRect(gx + 4 * s, gy + 7 * s, 8 * s, 6 * s)
                        ctx.beginPath()
                        ctx.moveTo(gx + 6.5 * s, gy + 9.5 * s)
                        ctx.lineTo(gx + 9.5 * s, gy + 9.5 * s)
                        ctx.stroke()
                        return
                    }
                    // The commit node is the author's picture where they
                    // were given one, and their identicon otherwise (5x5,
                    // mirrored; what stands in for the avatar services
                    // this application cannot use). The pattern uses only
                    // the inner part of the circle so the clip cuts less
                    // of it.
                    // A commit somebody shares steps its author up and
                    // left by a hair, and badges the first co-author at
                    // the lower right (規約 §co-author の表示): the pair
                    // straddles the lane and the author stays the bigger
                    // face. Only the first — the rest are named in the
                    // row's hover.
                    const shared = rowItem.mateFace !== 0
                    const ax = shared ? nodeX - 2 * Theme.borderWidth : nodeX
                    const ay = shared ? midY - Theme.borderWidth : midY
                    laneCanvas.face(ctx, ax, ay, r, rowItem.avatar,
                                    rowItem.avatar_url)
                    if (shared) {
                        const br = Theme.iconSm / 2
                        const bx = ax + r - Theme.borderWidth
                        const by = ay + r + Theme.borderWidth
                        // Punched out of what is already drawn, so the
                        // smaller face reads as being in front of the
                        // node rather than blended into it.
                        ctx.save()
                        ctx.globalCompositeOperation = "destination-out"
                        ctx.beginPath()
                        ctx.arc(bx, by, br + Theme.borderWidth, 0, 2 * Math.PI)
                        ctx.fill()
                        ctx.restore()
                        laneCanvas.face(ctx, bx, by, br, rowItem.mateFace, "")
                    }
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
        rowItem.loadFace()
    }
    function takeNamingFocus() {
        if (!rowItem.naming || !rowItem.ListView.view)
            return
        nameField.text = rowItem.ListView.view.namingText
        nameField.forceActiveFocus()
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
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: rowItem.isWip
                  ? qsTr("Working-tree changes — not committed yet")
                  : rowItem.author + "\n"
                    + Qt.formatDateTime(new Date(rowItem.atime * 1000), "yyyy-MM-dd HH:mm") + "\n"
                    + rowItem.oid_hex.substring(0, 8)
}
