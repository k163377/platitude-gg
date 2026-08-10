pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Commit-graph row: [branch/tag chips][lanes + identicon node][subject]
// — fixed-width label and graph columns keep subjects aligned. Column
// geometry arrives through properties on the owning ListView
// (labelWidth / graphColWidth / graphFullWidth / graphXOffset, and the
// uncommitted row's tallies), and clicks go back up through its
// rowSelected / rowMenuRequested signals.
Item {
    id: rowItem
    required property int index
    required property string oid_hex
    required property string author
    required property double atime
    required property string subject
    /// Everything after the subject, minus the co-author trailers. Only
    /// the hover card reads it — but the role has to be declared here or
    /// it arrives as `undefined` and whoever touches it stops mid-way.
    required property string body
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
    /// The find bar's line is somewhere in this row (platitude-core::find
    /// decides; the model marks it). Only ever true while a search is on.
    required property bool matched
    // Dimmed because the search passed this row over — not because it is
    // in any way unavailable (デザイン規約 §暗く落とした段: a row falls
    // this way only when many fall together, which is what makes it read
    // as "not the ones" rather than "not allowed"). A query nothing
    // answers takes every row down, which is the same sentence with
    // nothing left over.
    readonly property bool dimmed:
        rowItem.ListView.view ? rowItem.ListView.view.findOn && !rowItem.matched
                              : false

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
    // One kind of change and how many rows of it the file list holds. The
    // mark is the same ChangeIcon those rows carry, so the tally reads as
    // "these, that many" rather than as a legend of its own — and a kind
    // with nothing in it takes no seat (デザイン規約 §無効: what is not
    // there does not stand).
    component Tally: RowLayout {
        id: tally
        required property string code
        required property int count
        visible: tally.count > 0
        // Nothing between the mark and its number: the seat below is the
        // ink's width, so what the eye measures is already the mark's own
        // air (デザイン規約 §余白). A step here would put it back, and the
        // pair has to read as one thing from across the room — the gap to
        // the next kind is the only one that should be visible.
        spacing: 0
        // The seat is the ink, not the box. Drawn to the box, `!` would
        // stand five pixels from its own number while `+` stood two, and
        // neither would belong to it.
        // The seat is the ink, not the box. Drawn to the box, `!` would
        // stand five pixels from its own number while `+` stood two, and
        // neither would belong to it.
        Item {
            Layout.preferredWidth: tallyMark.inkWidth
            Layout.preferredHeight: Theme.iconXs
            ChangeIcon {
                id: tallyMark
                anchors.centerIn: parent
                change: tally.code
                // A step under `iconSm`: this mark stands beside a digit
                // of its own rather than beside a word, and at `iconSm` it
                // measured 8px against the digit's 6 (規約 §寸法).
                width: Theme.iconXs
                height: Theme.iconXs
                // The grid shrinks and the line has to shrink with it, or
                // the mark carries more weight than the digit beside it
                // (app-ui.md §語の隣に立つ印).
                stroke: Metrics.iconStroke * Theme.iconXs / Theme.iconMd
            }
        }
        Label {
            leftPadding: Theme.spaceXs / 2
            text: tally.count
            color: tallyMark.tint
            font.pixelSize: Theme.fontSm
        }
    }
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
    // The node is drawn on the same canvas as the lanes and only the node
    // dims, so the whole thing has to be asked for again.
    onDimmedChanged: laneCanvas.requestPaint()
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
            id: labelColumn
            Layout.preferredWidth: rowItem.labelsW
            Layout.fillHeight: true
            opacity: rowItem.dimmed ? Metrics.dimFade : 1
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
            // Nothing in this column can be hovered on its own: rowMouse
            // fills the row and is declared after it, so it takes every
            // hover the chips would have seen (デザイン規約 §hover の
            // ツールチップ). What the stacked chips hold is read from the
            // RefListPopup that rowMouse opens under the pointer.
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
                    // Everything from here down is the node, and the node
                    // is the row's own — so it dims with the row while
                    // the lanes above stay lit. A lane is one line drawn
                    // across many rows: dimming it per row would break
                    // each line into a bright-and-dark ladder that says
                    // nothing about what was searched for.
                    ctx.globalAlpha = rowItem.dimmed ? Metrics.dimFade : 1
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
                    const ax = shared ? nodeX - Theme.borderWidth : nodeX
                    const ay = shared ? midY - Theme.borderWidth : midY
                    laneCanvas.face(ctx, ax, ay, r, rowItem.avatar,
                                    rowItem.avatar_url)
                    if (shared) {
                        const br = Theme.iconSm / 2
                        const bx = ax + r - Theme.borderWidth
                        const by = ay + r + Theme.borderWidth
                        // Punched out of what is already drawn, so the
                        // smaller face reads as being in front of the
                        // node rather than blended into it. At full
                        // strength whatever the row's is: this takes
                        // pixels away, and a dimmed eraser would leave
                        // the author's face showing through the badge.
                        ctx.save()
                        ctx.globalAlpha = 1
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
            // The tick goes with the message, not with the lanes: it
            // stands in the subject column and belongs to this row alone.
            // Left bright it would be the loudest thing on a row the
            // search passed over.
            opacity: rowItem.dimmed ? Metrics.dimFade : 1
            // Short colored tick before the message: separates rows
            // visually (deliberately not a continuous line) and echoes
            // the commit's chain color.
            //
            // These three steps — this margin, this width, the layout's
            // spacing — are what `GraphPane.subjectTextX` adds up, since
            // that is where the find bar's cap is measured from.
            Rectangle {
                Layout.leftMargin: Theme.spaceSm
                implicitWidth: 2 * Theme.borderWidth
                implicitHeight: Theme.iconMd
                radius: Theme.borderWidth
                color: Theme.graphLane[rowItem.node_color % Theme.graphLane.length]
            }
            Label {
                // The uncommitted row's words are a fixed length, and the
                // counts read as part of the same sentence: it keeps its
                // own width so they sit right after it rather than out at
                // the pane's far edge.
                Layout.fillWidth: !rowItem.isWip
                // No total: the tallies beside it add up to exactly that
                // number, and the pane's own heading says it as well
                // (規約 §未コミット行が名乗るもの).
                text: rowItem.isWip ? qsTr("Uncommitted changes")
                                    : rowItem.subject
                elide: Text.ElideRight
                font.pixelSize: Theme.fontMd
                color: rowItem.isWip ? Theme.textSecondary : Theme.textPrimary
                rightPadding: rowItem.isWip ? 0 : Theme.spaceSm
            }
            // What the working tree does to HEAD, in lines. One reading
            // rather than the two the panes take: the same line touched in
            // the index and again in the tree is counted by both of those,
            // and their sum is not what this row stands for.
            // What is in the working tree, by kind. Conflicts lead: what is
            // stopped is the thing to see first, and the rest is the order
            // the file list would put them in.
            RowLayout {
                Layout.leftMargin: Theme.spaceSm
                Layout.rightMargin: Theme.spaceSm
                spacing: Theme.spaceSm
                Tally {
                    code: "UU"
                    count: rowItem.isWip && rowItem.ListView.view
                           ? rowItem.ListView.view.wipConflicted : 0
                }
                Tally {
                    code: "A"
                    count: rowItem.isWip && rowItem.ListView.view
                           ? rowItem.ListView.view.wipAdded : 0
                }
                Tally {
                    code: "M"
                    count: rowItem.isWip && rowItem.ListView.view
                           ? rowItem.ListView.view.wipModified : 0
                }
                Tally {
                    code: "D"
                    count: rowItem.isWip && rowItem.ListView.view
                           ? rowItem.ListView.view.wipDeleted : 0
                }
                Tally {
                    code: "R"
                    count: rowItem.isWip && rowItem.ListView.view
                           ? rowItem.ListView.view.wipRenamed : 0
                }
                Tally {
                    code: "C"
                    count: rowItem.isWip && rowItem.ListView.view
                           ? rowItem.ListView.view.wipCopied : 0
                }
            }
            // The rest of the row, on the one row that does not fill it.
            Item {
                visible: rowItem.isWip
                Layout.fillWidth: true
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
    /// This row's chip is the one with the list open under it.
    property bool chipHeld: false
    // A chip is `fontSmLine` tall — sixteen pixels in a row of
    // twenty-eight — and a hand that has just arrived is still settling. Landing takes
    // the chip itself, but once the list is out the whole chip column
    // holds it: drifting a dozen pixels inside the column the chips live
    // in is not leaving them (2026-08-09 trace — the hand landed at row
    // y 17 and was at y 2 eight milliseconds later, and the list went
    // with it).
    function chipUnder(px, py) {
        if (!rowChip.visible || rowChip.records.length < 2)
            return null
        const p = rowItem.mapToItem(rowChip, px, py)
        if (rowChip.contains(Qt.point(p.x, p.y)))
            return rowChip
        if (!rowItem.chipHeld)
            return null
        const c = rowItem.mapToItem(labelColumn, px, py)
        return labelColumn.contains(Qt.point(c.x, c.y)) ? rowChip : null
    }
    // Opens on a rest, the way the row's own card does, and closes with
    // the pointer. Not on landing: a pointer crossing the chip column on
    // its way somewhere passes over every stacked chip on the way, and
    // each one it touched used to put its list out and take it back a
    // breath later — the flashing reported on 2026-08-09. Nothing else
    // in the column answers a hover, so the wait costs the hand that
    // means it nothing but the wait.
    //
    // The row's own card gives way to it: the two open off the same
    // pointer and land in the same place, and the chip is the more
    // particular thing to be standing on (デザイン規約 §hover のツール
    // チップ). Stepping off the chip onto the rest of the row offers the
    // card again from the beginning — the same as walking in from
    // outside, because that is what the hand just did.
    function noteChipHover(px, py) {
        const chip = rowItem.naming ? null : rowItem.chipUnder(px, py)
        if (chip === rowItem.hoveredChip || !rowItem.ListView.view)
            return
        rowItem.hoveredChip = chip
        if (chip) {
            hoverDelay.stop()
            rowItem.ListView.view.rowHoverRequested(rowItem, false)
            if (rowItem.ListView.view.chipListAnchor === chip) {
                // The list this chip opened is still out: the hand walked
                // down into it and came back up. The rest is a question
                // about opening, and nothing is being opened — re-hold
                // now, or the settle closes the list at `hoverKeepMs` and
                // the rest reopens it at `tipDelayMs`, which reads as a
                // blink (規約 §hover のツールチップ「戻る手は待たせない」).
                chipDelay.stop()
                rowItem.chipHeld = true
                rowItem.ListView.view.chipExpandRequested(
                    chip.records, chip)
            } else {
                chipDelay.restart()
            }
        } else {
            chipDelay.stop()
            rowItem.chipHeld = false
            rowItem.ListView.view.chipCollapseRequested()
            if (rowMouse.containsMouse && !rowItem.isWip)
                hoverDelay.restart()
        }
    }
    Timer {
        id: chipDelay
        interval: Metrics.tipDelayMs
        onTriggered: {
            if (!rowItem.hoveredChip || !rowItem.ListView.view)
                return
            rowItem.chipHeld = true
            rowItem.ListView.view.chipExpandRequested(
                rowItem.hoveredChip.records, rowItem.hoveredChip)
        }
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
    /// Where the pointer is along the row, so the card can open under it
    /// rather than at the row's left edge — a row is the width of the
    /// pane, and its left edge is nowhere near the pointer.
    readonly property real pointerX: rowMouse.mouseX

    // Hover details: what the row no longer shows as columns — who wrote
    // it, when, and whoever they credited. The row reports; the page
    // decides, because the card outlives this delegate (it is recycled
    // the moment the row scrolls off).
    //
    // The WIP row opens nothing. It has no commit behind it and so none
    // of those facts, and its count is already in its own label — the
    // same answer its double-click and its right-click give
    // (規約 §hover のツールチップ).
    Timer {
        id: hoverDelay
        interval: Metrics.tipDelayMs
        onTriggered: {
            if (rowMouse.containsMouse && rowItem.ListView.view)
                rowItem.ListView.view.rowHoverRequested(rowItem, true)
        }
    }
    onIsWipChanged: hoverDelay.stop()
    // A pooled row is under no pointer, and the row it comes back as has
    // its own chips: anything this one was holding goes with it.
    ListView.onPooled: {
        hoverDelay.stop()
        chipDelay.stop()
        rowItem.hoveredChip = null
        rowItem.chipHeld = false
    }
    Connections {
        target: rowMouse
        function onContainsMouseChanged() {
            if (rowItem.isWip)
                return
            if (rowMouse.containsMouse) {
                hoverDelay.restart()
            } else {
                hoverDelay.stop()
                if (rowItem.ListView.view)
                    rowItem.ListView.view.rowHoverRequested(rowItem, false)
            }
        }
    }
}
