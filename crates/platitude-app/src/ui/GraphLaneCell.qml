pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// Lanes viewport of one commit-graph row: the full-width canvases slide behind clips when the graph column scrolls
// horizontally. Two canvases with two clips: the lanes stop at the column's edge, while the faces' clipper leans
// `spaceSm` further — up to where the message tick stands — so a column pulled toward its floor slides its edge over
// the badge's gap instead of through the ink, and the floor itself is where the tick meets the badge
// (GraphColumnMetrics.graphColWMin).
Item {
    id: laneCell

    /// How far the lanes have been sent sideways, and how wide they are at their widest — the list's own two numbers,
    /// read in rather than off `ListView.view`, so this cell has no view to be missing.
    required property real xOffset
    required property real fullWidth
    /// Precomputed draw tokens for this row's lanes (`encode.rs`).
    required property string geometry
    required property int nodeLane
    /// Packed `Co-authored-by` records; the first one badges the node.
    required property string coAuthors
    required property int avatar
    /// A picture this author was given, or empty for the generated pattern.
    required property string avatarUrl
    required property bool isWip
    required property string stashRef
    /// The search passed this row over. Only the row's own marks dim — the lanes stay lit.
    required property bool dimmed

    readonly property var mateRecords: laneCell.coAuthors === ""
        ? [] : laneCell.coAuthors.split(String.fromCharCode(31))
    /// Identicon code of the first co-author, or 0 when nobody is credited.
    readonly property int mateFace: laneCell.mateRecords.length === 0
        ? 0 : parseInt(laneCell.mateRecords[0].split(String.fromCharCode(30))[2])
    /// The node's lane centre, and where the badge sits when somebody shares the commit — one set of numbers for the
    /// two canvases that draw the badge and punch its ring, so the cut and the ink cannot drift apart.
    /// **GraphColumnMetrics.graphColWMin mirrors this geometry**: the column's floor is where the message tick meets
    /// the badge's ink (author up-left by a border, badge centre a border inside the node's edge, outline half in half
    /// out).
    readonly property real nodeMidX: Metrics.laneInset + laneCell.nodeLane * Metrics.laneW + Metrics.laneW / 2
    readonly property real badgeCX: laneCell.nodeMidX + Metrics.nodeIcon / 2 - 2 * Theme.borderWidth
    readonly property real badgeCY: Theme.graphRowHeight / 2 + Metrics.nodeIcon / 2

    onGeometryChanged: laneCanvas.requestPaint()
    // The badge's punched ring rides the lane canvas while the faces ride their own — a lane change, a credit change
    // and a dim each move ink on both.
    onNodeLaneChanged: laneCell.repaintNode()
    onCoAuthorsChanged: laneCell.repaintNode()
    onDimmedChanged: laneCell.repaintNode()
    onAvatarChanged: nodeCanvas.requestPaint()
    // Assigning a picture changes no history, so this role moves on rows that are otherwise untouched — and a canvas
    // repaints only when it is asked to.
    onAvatarUrlChanged: laneCell.loadFace()
    // What the row *is* decides which mark stands on its node — the dashed ring of the working tree, the stash's
    // archive box, or the author's face — and a rebuild writes a different row over the same delegate rather than
    // building a new one (`GraphModel::splice_notified`). Without these two the mark of the row that was there stays:
    // a stash popped off the top left its box on the working-tree row, and a stash just made wore the dashed ring.
    onIsWipChanged: laneCell.repaintNode()
    onStashRefChanged: laneCell.repaintNode()
    function repaintLanes() {
        laneCanvas.requestPaint()
    }
    function repaintNode() {
        laneCanvas.requestPaint()
        nodeCanvas.requestPaint()
    }
    /// How wide the two canvases below are drawn.
    ///
    /// **The lanes reach `fullWidth`, but only the column is ever on screen while the graph is not sent sideways**,
    /// and a canvas is an image the size of the item it is: at the reference repository that is 852 pixels held for
    /// 252 shown, on every row that is built, and the same ratio again on every repaint (measured — the two
    /// canvases at full width are 42.6MB of the working set once the graph has been scrolled through).
    ///
    /// **Full width while it is sent sideways**, because that is what needs the rest of it: the picture is slid by
    /// moving the canvas rather than repainting it (`x`), so during a pan it has to already hold what the slide will
    /// bring in. Widening and narrowing again each cost one repaint of the rows on screen — the same repaint the
    /// column's own divider drag costs, and taken once at each end of a pan rather than per frame. **Qt asks for that
    /// repaint itself for a canvas that is visible** (the note on `onWidthChanged` below), so the first frame of a pan
    /// is the lanes and not the old picture stretched — verified at that frame (`PG_AUTO_ACT=graph-bar`).
    ///
    /// A function rather than one property because the two canvases stand in boxes of different widths — the faces'
    /// clipper leans `spaceSm` further than the lanes' — and the rule about how wide to draw is one rule. It reads
    /// only properties, so a binding on it takes the dependencies it names (`xOffset`, `fullWidth`, and whatever the
    /// caller hands in); the rule against binding to a *method* is about the ones that measure and never notify
    /// (app-ui.md).
    function inkWidth(box) {
        return laneCell.xOffset > 0 ? laneCell.fullWidth : Math.min(laneCell.fullWidth, box)
    }

    function loadFace() {
        if (laneCell.avatarUrl !== "")
            nodeCanvas.loadImage(laneCell.avatarUrl)
        nodeCanvas.requestPaint()
    }
    Component.onCompleted: laneCell.loadFace()

    Item {
        anchors.fill: parent
        clip: true
        InkCanvas {
            id: laneCanvas
            x: -laneCell.xOffset
            width: laneCell.inkWidth(parent.width)
            height: parent.height
            // Resizing a canvas scales what it already holds; only a repaint redraws it. Qt asks for that repaint
            // itself, but only for a canvas that is visible — and a row waiting in the ListView's reuse pool is not. So
            // a graph that grows a lane while a row is pooled brings that row back with its lanes and node stretched
            // sideways (seen after switching windows: the pool is where rows sit while another window is in
            // front).
            onWidthChanged: requestPaint()
            onHeightChanged: requestPaint()
            onPaint: {
                const ctx = getContext("2d")
                ctx.clearRect(0, 0, width, height)
                ctx.lineWidth = Metrics.laneStroke
                const laneCount = Theme.graphLane.length
                const cx = function (l) { return Metrics.laneInset + l * Metrics.laneW + Metrics.laneW / 2 }
                const midY = height / 2
                const nodeX = laneCell.nodeMidX
                // decode precomputed draw tokens: t/i/o + lane + color (uppercase = dashed WIP edge)
                if (laneCell.geometry !== "") {
                    const toks = laneCell.geometry.split(";")
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
                            ctx.bezierCurveTo(nodeX, height - midY * 0.34, x, height - midY * 0.66, x, height)
                        }
                        ctx.stroke()
                    }
                    ctx.setLineDash([])
                }
                // The badge punches its ring out of the lanes as well: the faces sit on the canvas below (whose clip
                // leans past the column), and a ring cut only there would leave the lane line showing through the gap
                // the badge keeps around itself.
                if (!laneCell.isWip && laneCell.stashRef === "" && laneCell.mateFace !== 0) {
                    ctx.save()
                    ctx.globalCompositeOperation = "destination-out"
                    ctx.beginPath()
                    ctx.arc(laneCell.badgeCX, laneCell.badgeCY, Theme.iconSm / 2 + Theme.borderWidth, 0, 2 * Math.PI)
                    ctx.fill()
                    ctx.restore()
                }
                // The marks below are the row's own — they dim with it while the lanes above stay lit. A lane is one
                // line drawn across many rows: dimming it per row would break each line into a bright-and-dark ladder.
                ctx.globalAlpha = laneCell.dimmed ? Metrics.dimFade : 1
                // The WIP row has no commit and no author: a dashed, empty node instead of a face. A lane mark, drawn
                // here — nothing of it leans past the column.
                const r = Metrics.nodeIcon / 2
                if (laneCell.isWip) {
                    ctx.strokeStyle = Theme.textSecondary
                    ctx.lineWidth = Metrics.laneStroke
                    ctx.setLineDash(Metrics.laneDash)
                    ctx.beginPath()
                    ctx.arc(nodeX, midY, r - 1, 0, 2 * Math.PI)
                    ctx.stroke()
                    ctx.setLineDash([])
                    return
                }
                // Stash rows draw the bare archive box of the STASHES section (NavIcon "stash", same 16-unit grid)
                // instead of the author identicon — no ring around it, so the node reads as the icon and nothing else.
                // The glyph is centered on the node point (its own middle, 8.5 of the grid, not the grid's), and its
                // footprint is cleared first so the dashed leash comes out from under the box instead of running
                // through it.
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
                }
            }
        }
    }
    // The faces' own clipper: `spaceSm` past the column, up to where the message tick stands, so a narrowing column or
    // a sideways pan slides the clip edge over the badge's gap instead of cutting a face mid-ink (規約 §グラフ列は最も
    // 広い所のレーンまで).
    Item {
        width: parent.width + Theme.spaceSm
        height: parent.height
        clip: true
        InkCanvas {
            id: nodeCanvas
            x: -laneCell.xOffset
            width: laneCell.inkWidth(parent.width)
            height: parent.height
            // Same resize rule as the lane canvas above.
            onWidthChanged: requestPaint()
            onHeightChanged: requestPaint()
            // A row coming back from the reuse pool carries a new author, so the picture it holds is loaded again
            // before the paint that would draw it.
            onImageLoaded: requestPaint()
            /// One round face: the assigned picture when there is a url for it, the generated pattern otherwise. Shared
            /// by the node and the co-author badge so the two cannot drift apart in shape or outline.
            function face(ctx, cx, cy, radius, code, url) {
                ctx.save()
                ctx.beginPath()
                ctx.arc(cx, cy, radius, 0, 2 * Math.PI)
                ctx.clip()
                if (url !== "") {
                    if (nodeCanvas.isImageLoaded(url)) {
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
                // The WIP ring and the stash box are lane marks, drawn on the canvas above; this one is the faces
                // alone.
                if (laneCell.isWip || laneCell.stashRef !== "")
                    return
                // The node is the row's own — it dims with the row while the lanes stay lit.
                ctx.globalAlpha = laneCell.dimmed ? Metrics.dimFade : 1
                const midY = height / 2
                const r = Metrics.nodeIcon / 2
                // The commit node is the author's picture where they were given one, and their identicon otherwise
                // (5x5, mirrored; what stands in for the avatar services this application cannot use). The pattern uses
                // only the inner part of the circle so the clip cuts less of it. A commit somebody shares steps its
                // author up and left by a hair, and badges the first co-author at the lower right (規約 §co-author の表示):
                // the pair straddles the lane and the author stays the bigger face. Only the first — the rest are named
                // in the row's hover.
                const shared = laneCell.mateFace !== 0
                const ax = shared ? laneCell.nodeMidX - Theme.borderWidth : laneCell.nodeMidX
                const ay = shared ? midY - Theme.borderWidth : midY
                nodeCanvas.face(ctx, ax, ay, r, laneCell.avatar, laneCell.avatarUrl)
                if (shared) {
                    const br = Theme.iconSm / 2
                    // Punched out of what is already drawn, so the smaller face reads as being in front of the node
                    // rather than blended into it. At full strength whatever the row's is: this takes pixels away, and
                    // a dimmed eraser would leave the author's face showing through the badge.
                    ctx.save()
                    ctx.globalAlpha = 1
                    ctx.globalCompositeOperation = "destination-out"
                    ctx.beginPath()
                    ctx.arc(laneCell.badgeCX, laneCell.badgeCY, br + Theme.borderWidth, 0, 2 * Math.PI)
                    ctx.fill()
                    ctx.restore()
                    nodeCanvas.face(ctx, laneCell.badgeCX, laneCell.badgeCY, br, laneCell.mateFace, "")
                }
            }
        }
    }
}
