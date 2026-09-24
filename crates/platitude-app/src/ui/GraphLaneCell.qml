pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// Lanes viewport of one commit-graph row: the full-width canvases slide behind clips when the graph column scrolls
// horizontally. Two canvases with two clips: the lanes stop at the column's edge, while the faces' clipper leans
// `spaceSm` further — up to where the message tick stands — so a column pulled toward its floor slides its edge over
// the badge's gap, and the floor itself is where the tick meets the badge
// (GraphColumnMetrics.graphColWMin).
Item {
    id: laneCell

    /// How far the lanes have been sent sideways, and how wide they are at their widest — the list's own two numbers,
    /// read in, so this cell has no view to be missing.
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
    /// The node's lane centre, and where the badge sits when somebody shares the commit — one set of numbers for the
    /// two canvases that draw the badge and punch its ring, so the cut and the ink cannot drift apart.
    /// **GraphColumnMetrics.graphColWMin mirrors this geometry**: the column's floor is where the message tick meets
    /// the badge's ink (author up-left by a border, badge centre a border inside the node's edge, outline half in half
    /// out).
    readonly property real nodeMidX: Metrics.laneInset + laneCell.nodeLane * Metrics.laneW + Metrics.laneW / 2
    readonly property real badgeCX: laneCell.nodeMidX + Metrics.nodeIcon / 2 - 2 * Theme.borderWidth
    readonly property real badgeCY: Theme.graphRowHeight / 2 + Metrics.nodeIcon / 2

    onGeometryChanged: ink.requestPaint()
    onNodeLaneChanged: laneCell.repaintNode()
    onCoAuthorsChanged: laneCell.repaintNode()
    onDimmedChanged: laneCell.repaintNode()
    onAvatarChanged: ink.requestPaint()
    // Assigning a picture changes no history, so this role moves on rows that are otherwise untouched — and a canvas
    // repaints only when it is asked to.
    onAvatarUrlChanged: laneCell.loadFace()
    // What the row *is* decides which mark stands on its node — the dashed ring of the working tree, the stash's
    // archive box, or the author's face — and a rebuild writes a different row over the same delegate
    // (`GraphModel::splice_notified`). Without these two the mark of the row that was there stays:
    // a stash popped off the top left its box on the working-tree row, and a stash just made wore the dashed ring.
    onIsWipChanged: laneCell.repaintNode()
    onStashRefChanged: laneCell.repaintNode()
    // The two names remain, both repainting the one canvas: what a caller asks for is that this ink be brought in
    // line, and which parts of it live on which surface stopped being a real distinction when the surfaces merged.
    function repaintLanes() {
        ink.requestPaint()
    }
    function repaintNode() {
        ink.requestPaint()
    }
    /// How wide the canvas below is drawn.
    ///
    /// **The lanes reach `fullWidth`, but only the column is ever on screen while the graph is not sent sideways**,
    /// and a canvas is an image the size of the item it is: at the reference repository the lanes are several times
    /// the width the column shows, on every row that is built, and the same ratio again on every repaint — two
    /// full-width canvases a row would put tens of megabytes on the working set once the graph is scrolled through
    /// (ci/baseline/code-costs-windows-x64.md §メモリの形).
    ///
    /// **Full width while it is sent sideways**, because that is what needs the rest of it: the picture is slid by
    /// moving the canvas (`x`), so during a pan it has to already hold what the slide will
    /// bring in. Widening and narrowing again each cost one repaint of the rows on screen — the same repaint the
    /// column's own divider drag costs, and taken once at each end of a pan. **Qt asks for that
    /// repaint itself for a canvas that is visible** (the note on `onWidthChanged` below), so the first frame of a pan
    /// is the lanes and not the old picture stretched — verified at that frame (`PGG_AUTO_ACT=graph-bar`).
    ///
    /// It reads only properties, so a binding on it takes the dependencies it names (`xOffset`, `fullWidth`, and the
    /// box the caller hands in); the rule against binding to a *method* is about the ones that measure and never
    /// notify (app-ui.md).
    function inkWidth(box) {
        return laneCell.xOffset > 0 ? laneCell.fullWidth : Math.min(laneCell.fullWidth, box)
    }

    function loadFace() {
        if (laneCell.avatarUrl !== "")
            ink.loadImage(laneCell.avatarUrl)
        ink.requestPaint()
    }
    Component.onCompleted: laneCell.loadFace()

    // One clipper for the row's whole ink, and it is the faces': `spaceSm` past the column, up to where the message
    // tick stands, so a narrowing column or a sideways pan slides the clip edge over the badge's gap
    // (規約 §グラフ列は最も広い所のレーンまで). The lanes stop at the column's own edge, which
    // is a clip *inside* the paint — one canvas holds both inks, and only the faces lean past.
    //
    // One canvas, the lanes under and the faces over: a canvas is an image plus the texture it
    // uploads, and two would double both on every row that exists and again on every repaint, for two inks that are
    // drawn in one order into one picture anyway.
    Item {
        width: parent.width + Theme.spaceSm
        height: parent.height
        clip: true
        InkCanvas {
            id: ink
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
                // ---- the lanes, and the marks that ride them, held to the column's edge ----
                // In canvas coordinates that edge is `xOffset + laneCell.width`: the canvas is slid left by
                // `xOffset`, and the cell is the column. This rect is the lanes' clip; they have no item of their own.
                ctx.save()
                ctx.beginPath()
                ctx.rect(0, 0, laneCell.xOffset + laneCell.width, height)
                ctx.clip()
                ctx.lineWidth = Metrics.laneStroke
                // The row's lanes and the marks that ride them dim with it (規約 §コミットを探す): a lane crossing a
                // match stays lit for that row alone. Set on every paint — the list hands this cell from row to row,
                // and the alpha is the one piece of the context the next row's lanes read.
                ctx.globalAlpha = laneCell.dimmed ? Metrics.dimFade : 1
                const laneCount = Theme.graphLane.length
                const cx = function (l) { return Metrics.laneInset + l * Metrics.laneW + Metrics.laneW / 2 }
                const midY = height / 2
                const nodeX = laneCell.nodeMidX
                // One stroke per segment: `through` the whole row, `into` the node from the top edge, `out` of it to
                // the bottom edge; dashed is the WIP leash.
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
                // The WIP row has no commit and no author: a dashed, empty node. A lane mark, drawn
                // inside the lanes' clip — nothing of it leans past the column.
                const r = Metrics.nodeIcon / 2
                if (laneCell.isWip) {
                    ctx.strokeStyle = Theme.textSecondary
                    ctx.lineWidth = Metrics.laneStroke
                    ctx.setLineDash(Metrics.laneDash)
                    ctx.beginPath()
                    ctx.arc(nodeX, midY, r - 1, 0, 2 * Math.PI)
                    ctx.stroke()
                    ctx.setLineDash([])
                    // The context outlives this paint: a return that kept the save would hand the lanes' clip and
                    // this row's alpha to the next paint, whose clearRect would then miss the faces' strip.
                    ctx.restore()
                    return
                }
                // Stash rows draw the bare archive box of the STASHES section (NavIcon "stash", same
                // 16-unit grid) — no ring around it, so the node reads as the icon and nothing
                // else. The glyph is centered on the node point (its own middle, 8.5 of the
                // grid), and its footprint is cleared first so the dashed leash comes out
                // from under the box.
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
                    // Same discipline as the ring above: the save ends with the paint.
                    ctx.restore()
                    return
                }
                // ---- the faces, leaning as far as the clipper goes ----
                ctx.restore()
                // Saved like the lanes' half: the context outlives this paint, and the list hands this cell from row
                // to row as it scrolls, so an alpha or a composite left on it is the next row's to start from.
                ctx.save()
                // The commit node is the author's picture where they were given one, and their identicon otherwise
                // (5x5, mirrored; what stands in for the avatar services this application cannot use). The pattern uses
                // only the inner part of the circle so the clip cuts less of it. A commit somebody shares steps its
                // author up and left by a hair, and badges the first co-author at the lower right (規約 §co-author の表示):
                // the pair straddles the lane and the author stays the bigger face. Only the first — the rest are named
                // in the row's hover.
                const shared = laneCell.mateFace !== 0
                const ax = shared ? laneCell.nodeMidX - Theme.borderWidth : laneCell.nodeMidX
                const ay = shared ? midY - Theme.borderWidth : midY
                // The node dims with its row. **Darker, not see-through**: at `dimFade` the face is a wash, and the
                // lanes into and out of it would run on across it, so the disc it covers — outline and all — is taken
                // out of the lanes first and the face goes down over the row's own ground.
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
                    // Punched out of what is already drawn — the author's face and the lane ink under it alike — so
                    // the smaller face reads as being in front of the node, and no lane
                    // line shows through the gap the badge keeps around itself. At full strength whatever the row's
                    // is: this takes pixels away, and a dimmed eraser would leave the author's face showing through
                    // the badge.
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
