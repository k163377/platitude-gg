pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// Lanes viewport of one commit-graph row: one canvas, slid sideways as the graph column scrolls.
Item {
    id: laneCell

    /// The list's sideways offset and the lanes' full width, handed in so the cell needs no view.
    required property real xOffset
    required property real fullWidth
    /// This row's lane segments (`{kind, lane, color, dashed}` — `encode::Lanes`).
    required property var geometry
    required property int nodeLane
    /// The `Co-authored-by` records (`{name, email, face}`); the first one badges the node.
    required property var coAuthors
    required property int avatar
    /// A picture this author was given, or empty for the generated pattern.
    required property string avatarUrl
    required property bool isWip
    required property string stashRef
    /// The search passed this row over: its lanes, its marks and its face all go down.
    required property bool dimmed

    /// Identicon code of the first co-author, or 0 when nobody is credited.
    readonly property int mateFace: laneCell.coAuthors.length === 0 ? 0 : laneCell.coAuthors[0].face
    /// The node's centre and the co-author badge's — one set for the punch and the ink. GraphColumnMetrics.graphColWMin
    /// mirrors this geometry: move one, move both.
    readonly property real nodeMidX: Metrics.laneInset + laneCell.nodeLane * Metrics.laneW + Metrics.laneW / 2
    readonly property real badgeCX: laneCell.nodeMidX + Metrics.nodeIcon / 2 - 2 * Theme.borderWidth
    readonly property real badgeCY: Theme.graphRowHeight / 2 + Metrics.nodeIcon / 2

    onGeometryChanged: ink.requestPaint()
    onNodeLaneChanged: laneCell.repaintNode()
    onCoAuthorsChanged: laneCell.repaintNode()
    onDimmedChanged: laneCell.repaintNode()
    onAvatarChanged: ink.requestPaint()
    // An assigned picture changes no history, so this role moves on rows that are otherwise untouched.
    onAvatarUrlChanged: laneCell.loadFace()
    // A rebuild writes a different row over the same delegate (`GraphModel::splice_notified`): without these two the
    // previous row's node mark stays.
    onIsWipChanged: laneCell.repaintNode()
    onStashRefChanged: laneCell.repaintNode()
    // Both repaint the one canvas; the two names say what the caller changed.
    function repaintLanes() {
        ink.requestPaint()
    }
    function repaintNode() {
        ink.requestPaint()
    }
    /// How wide the canvas is drawn: the visible column, or `fullWidth` while the lanes are sent sideways (the pan
    /// slides the canvas, so it must already hold what slides in). A canvas is an image its own size, so full width on
    /// every row costs the working set (ci/baseline/code-costs-windows-x64.md §メモリの形). Binding-safe: it reads only
    /// properties and its argument.
    function inkWidth(box) {
        return laneCell.xOffset > 0 ? laneCell.fullWidth : Math.min(laneCell.fullWidth, box)
    }

    function loadFace() {
        if (laneCell.avatarUrl !== "")
            ink.loadImage(laneCell.avatarUrl)
        ink.requestPaint()
    }
    Component.onCompleted: laneCell.loadFace()

    // The faces' clipper, `spaceSm` past the column to where the message tick stands (規約 §グラフ列は最も広い所のレーンまで);
    // the lanes stop at the column's edge by a clip inside the paint. One canvas, not two — each is an image plus a
    // texture on every row (rules-refs/app-ui.md「1 行 1 Canvas」).
    Item {
        width: parent.width + Theme.spaceSm
        height: parent.height
        clip: true
        InkCanvas {
            id: ink
            x: -laneCell.xOffset
            width: laneCell.inkWidth(parent.width)
            height: parent.height
            // Resizing a canvas scales what it holds; Qt repaints only a visible one, and a row in the reuse pool is
            // not — it would come back stretched.
            onWidthChanged: requestPaint()
            onHeightChanged: requestPaint()
            // `loadImage` is asynchronous: paint again once the face arrives.
            onImageLoaded: requestPaint()
            /// One round face — the assigned picture, or the generated identicon (5x5, mirrored). Shared by the node
            /// and the badge so the two cannot drift apart.
            function face(ctx, cx, cy, radius, code, url) {
                ctx.save()
                ctx.beginPath()
                ctx.arc(cx, cy, radius, 0, 2 * Math.PI)
                ctx.clip()
                if (url !== "") {
                    if (ink.isImageLoaded(url)) {
                        ctx.drawImage(url, cx - radius, cy - radius, 2 * radius, 2 * radius)
                    }
                } else {
                    ctx.fillStyle = Theme.bgElevated
                    ctx.fillRect(cx - radius, cy - radius, 2 * radius, 2 * radius)
                    ctx.fillStyle = Theme.graphLane[(code >> 15) & 0x7]
                    const inner = 2 * radius * Metrics.identiconFill
                    const cell = inner / 5
                    const ox = cx - inner / 2
                    const oy = cy - inner / 2
                    for (let row = 0; row < 5; row++) {
                        for (let col = 0; col < 3; col++) {
                            if ((code >> (row * 3 + col)) & 1) {
                                ctx.fillRect(ox + col * cell, oy + row * cell, cell + 0.5, cell + 0.5)
                                if (col < 2)
                                    ctx.fillRect(ox + (4 - col) * cell, oy + row * cell, cell + 0.5, cell + 0.5)
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
                // ---- the lanes and their marks, clipped to the column's edge (`xOffset + width` on the canvas) ----
                ctx.save()
                ctx.beginPath()
                ctx.rect(0, 0, laneCell.xOffset + laneCell.width, height)
                ctx.clip()
                ctx.lineWidth = Metrics.laneStroke
                // Lanes dim per row (規約 §コミットを探す). Set on every paint: a reused cell's context carries the
                // last row's alpha.
                ctx.globalAlpha = laneCell.dimmed ? Metrics.dimFade : 1
                const laneCount = Theme.graphLane.length
                const cx = function (l) { return Metrics.laneInset + l * Metrics.laneW + Metrics.laneW / 2 }
                const midY = height / 2
                const nodeX = laneCell.nodeMidX
                // Dashed is the WIP leash.
                for (const seg of laneCell.geometry) {
                    const x = cx(seg.lane)
                    ctx.strokeStyle = Theme.graphLane[seg.color % laneCount]
                    ctx.setLineDash(seg.dashed ? Metrics.laneDash : [])
                    ctx.beginPath()
                    if (seg.kind === "through") {
                        ctx.moveTo(x, 0)
                        ctx.lineTo(x, height)
                    } else if (seg.kind === "into") {
                        ctx.moveTo(x, 0)
                        ctx.bezierCurveTo(x, midY * 0.66, nodeX, midY * 0.34, nodeX, midY)
                    } else {
                        ctx.moveTo(nodeX, midY)
                        ctx.bezierCurveTo(nodeX, height - midY * 0.34, x, height - midY * 0.66, x, height)
                    }
                    ctx.stroke()
                }
                ctx.setLineDash([])
                // The WIP row: a dashed, empty node, inside the lanes' clip.
                const r = Metrics.nodeIcon / 2
                if (laneCell.isWip) {
                    ctx.strokeStyle = Theme.textSecondary
                    ctx.lineWidth = Metrics.laneStroke
                    ctx.setLineDash(Metrics.laneDash)
                    ctx.beginPath()
                    ctx.arc(nodeX, midY, r - 1, 0, 2 * Math.PI)
                    ctx.stroke()
                    ctx.setLineDash([])
                    // Restore before returning: the context outlives the paint, and a kept save hands this clip and
                    // alpha to the next one, whose clearRect then misses the faces' strip.
                    ctx.restore()
                    return
                }
                // Stash rows: NavIcon "stash" on its 16-grid, no ring, centred on the node (its middle is 8.5 of the
                // grid); the footprint is cleared first so the dashed leash comes out from under the box.
                if (laneCell.stashRef !== "") {
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
                    // Restore before returning, as above.
                    ctx.restore()
                    return
                }
                // ---- the faces, leaning as far as the clipper goes ----
                ctx.restore()
                // Saved like the lanes' half, so this row's alpha and composite do not carry over.
                ctx.save()
                // The author's face; on a shared commit it steps up-left by a border and the first co-author badges the
                // lower right (規約 §co-author の表示).
                const shared = laneCell.mateFace !== 0
                const ax = shared ? laneCell.nodeMidX - Theme.borderWidth : laneCell.nodeMidX
                const ay = shared ? midY - Theme.borderWidth : midY
                // Dimmed darker, not see-through: the disc is punched out of the lanes first, or they would run across
                // the face.
                if (laneCell.dimmed) {
                    ctx.globalCompositeOperation = "destination-out"
                    ctx.beginPath()
                    ctx.arc(ax, ay, r + Theme.borderWidth / 2, 0, 2 * Math.PI)
                    ctx.fill()
                    ctx.globalCompositeOperation = "source-over"
                    ctx.globalAlpha = Metrics.dimFade
                }
                ink.face(ctx, ax, ay, r, laneCell.avatar, laneCell.avatarUrl)
                if (shared) {
                    const br = Theme.iconSm / 2
                    // Punched out of the face and lanes beneath, at full alpha even on a dimmed row — a dimmed eraser
                    // leaves the author's face showing through the badge.
                    ctx.save()
                    ctx.globalAlpha = 1
                    ctx.globalCompositeOperation = "destination-out"
                    ctx.beginPath()
                    ctx.arc(laneCell.badgeCX, laneCell.badgeCY, br + Theme.borderWidth, 0, 2 * Math.PI)
                    ctx.fill()
                    ctx.restore()
                    ink.face(ctx, laneCell.badgeCX, laneCell.badgeCY, br, laneCell.mateFace, "")
                }
                ctx.restore()
            }
        }
    }
}
