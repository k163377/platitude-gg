pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

/// The branch the working tree stands on never leaves the graph: while its own row is scrolled off, this stand-in rides
/// the edge the row went out of, and it steps aside the moment the row itself is on screen — so the graph never shows
/// the branch twice. A press on it goes to that row. The sidebar says the same thing about the same
/// branch in its own list (`HeadPinRow`); this is that answer in the pane where the branch is a chip.
///
/// **It is that commit's row, drawn as a row** — the chip, the lanes and node of its own `GraphLaneCell`, the subject
/// after the message tick. **Its own, and nothing else's**: a stand-in that borrowed the lanes of whatever happens to be
/// underneath would claim this commit is on lines it is not.
///
/// **And then it dissolves.** From the node's own edge the graph goes out into the ground; half a row past the row it
/// is gone and nothing has come back yet; and over the row after that the ground itself goes, so the real rows return
/// a little at a time. That is what the space between two places in a history looks like, and it is what keeps a hard
/// edge from cutting the row underneath in two.
///
/// **The pane adopts this, and not the list** (`GraphPane`), which is the other way round from the sidebar's. The lane
/// column has a press-taking strip of its own over the list (`GraphLanePan`), so a stand-in inside the list would hand
/// its middle third to a strip that answers with the row underneath.
Rectangle {
    id: pin

    required property var graphModel
    /// The list this rides: where it stands, how tall it is, and how far its rows have been pushed down — which edge
    /// this takes, and whether it is wanted at all, is read off those.
    required property var view
    /// The three columns as the rows have them, how wide the lanes are at their widest, and how far they have been sent
    /// sideways.
    required property real labelWidth
    required property real graphColWidth
    required property real graphFullWidth
    required property real graphXOffset
    /// How much of the right edge belongs to the list's own scroll bar. **What this stand-in takes presses on stops
    /// short of it** — one that took them there answered the trough with a jump to HEAD (observed + measured).
    /// Its ink is a separate question and does not read this: the words stop where a row's words stop, four pixels off
    /// the pane, and the ground is laid under the bar rather than over it (`band`).
    required property real barRoom

    /// The stand-in was pressed: go to the row it stands for.
    signal activated(int row)

    /// The pointer resting on this, for the headless run — hover cannot be injected, so what the pointer would light is
    /// written in the same one place the pointer's own arrival writes (verify-ui).
    property bool pointed: false

    readonly property int headRow: pin.graphModel.headRow
    /// Where that row sits in the list's own content coordinates. Worked out rather than read off the item: the row is
    /// two thousand rows away and the view has not built it (`GraphRowWalk.revealStep` does the same arithmetic).
    readonly property real rowTop: pin.view.originY + pin.headRow * Theme.graphRowHeight
    readonly property bool rowAbove: pin.rowTop < pin.view.contentY
    readonly property bool rowBelow: pin.rowTop + Theme.graphRowHeight > pin.view.contentY + pin.view.height
    /// A HEAD outside the loaded window has no row to lead to, and one whose chips have not arrived yet has no name to
    /// show — neither is a stand-in worth drawing (`models::graph::head`).
    readonly property bool wanted: pin.headRow >= 0 && pin.graphModel.headLabels !== ""
    /// The chips the stand-in draws, less the ones the window has already said are gone — the same answer the rows
    /// themselves give (`encode::labels_shown`). HEAD's own branch is never one of them (git refuses to delete the
    /// branch it is on), but another branch standing on the same commit can be.
    readonly property string goneChips: pin.graphModel.goneChips
    readonly property string shownLabels: GitFacts.labelsShown(pin.graphModel.headLabels, pin.goneChips)
    readonly property var records: pin.shownLabels === "" ? [] : pin.shownLabels.split(String.fromCharCode(31))
    readonly property color laneColor: Theme.graphLane[pin.graphModel.headColor % Theme.graphLane.length]

    // ---- the three bands ---------------------------------------------------------------------------------------------
    //
    // The row, then the half where the graph goes out, then the half where the rows come back. Everything below hangs
    // off these four numbers, and each of them flips with the edge this rides.

    /// How far past the row the ground stays whole. **The graph has to be gone before the rows start coming back**
    /// — without this the two overlap and neither reads, and what is left below the
    /// stand-in is a row at half strength rather than a stretch of nothing.
    ///
    /// **A quarter of a row, not half**: the whole going-out sits that much higher, so the
    /// commit directly under the stand-in is most of the way back rather than wiped across its middle. It is still the
    /// only room the lanes have to dissolve in, so it cannot go to nothing — a hard edge is what is on the other side.
    readonly property real hold: Theme.graphRowHeight / 4
    /// And how long they take coming back. **What the hold gives up, this takes** — the two
    /// together are a row and a half whatever the split, so the stand-in stands on the same 2.5 rows it always did and
    /// only the shape of the dissolve moves. Shortening the whole instead would put the same drop in brightness across
    /// a shorter distance, which is the one thing a gradient is here to avoid. A row is the floor: less, and the commit
    /// it lands on is cut across.
    readonly property real fadeRoom: Theme.graphRowHeight * 5 / 4
    readonly property real rowY: pin.rowAbove ? 0 : pin.hold + pin.fadeRoom
    readonly property real rowMidY: pin.rowY + Theme.graphRowHeight / 2
    /// Where the lane leaves the node — **a hairline past its edge, not the row's**. The row's
    /// own cell is cut here and the going-out takes over, so the last full-strength pixel of graph is the
    /// one against the face rather than one at the bottom of a row that is mostly air.
    readonly property real nodeOutY: pin.rowMidY
        + (pin.rowAbove ? 1 : -1) * (Metrics.nodeIcon / 2 + Theme.borderWidth)
    /// And where it has gone: the far edge of the hold, so the ground is still whole when the last of the graph goes.
    readonly property real outTo: pin.rowAbove ? Theme.graphRowHeight + pin.hold : pin.fadeRoom

    visible: pin.wanted && (pin.rowAbove || pin.rowBelow)
    height: Theme.graphRowHeight + pin.hold + pin.fadeRoom
    x: pin.view.x
    // The list's own width, so every column in here lands where a row's lands — the message included. `barRoom` is
    // taken off the press area alone (`pinMouse`).
    width: pin.view.width
    y: pin.view.y + (pin.rowAbove ? 0 : pin.view.height - pin.height)
    color: "transparent"

    // **The graph's own ground, and nothing more.** Opaque it has to be where this stands — the rows run under it, and
    // a stand-in they showed through would read as two commits in one band — but a lighter band would be an emphasis
    // that is up for as long as the branch is off screen, and the only thing worth emphasising here is one line of it
    //. What says "this one is yours" is the colour of its words.
    //
    // It holds through the row and half a row past it — the stretch where the graph has gone and nothing has come back
    // yet, which is what makes the two read as one movement rather than as a crossfade — and lets go over the row after
    // that.
    // **The ground is laid inside the list; everything else stands over it out here.** Two things have to hold at once
    // and only this seat holds both. It has to cover the whole width — a row's message runs on under the bar now
    // (規約 §余白), so a band that stopped short of the bar would leave the tail of whatever is scrolling past showing
    // in that strip. And it may not hide the bar. A child of the view is drawn over the rows and
    // under the bar (`AutoScrollBar` takes `z: 1` for exactly this), which is both. The chip, the lanes and the words
    // stay out here, where the lane strip below cannot answer a press for them.
    Item {
        id: band
        parent: pin.view
        visible: pin.visible
        y: pin.rowAbove ? 0 : pin.view.height - pin.height
        width: pin.view.width
        height: pin.height

        Rectangle {
            y: pin.rowAbove ? 0 : pin.fadeRoom
            width: parent.width
            height: Theme.graphRowHeight + pin.hold
            color: Theme.bgSurface
        }
        Rectangle {
            y: pin.rowAbove ? Theme.graphRowHeight + pin.hold : 0
            width: parent.width
            height: pin.fadeRoom
            gradient: Gradient {
                GradientStop { position: 0; color: pin.rowAbove ? Theme.bgSurface : "transparent" }
                GradientStop { position: 1; color: pin.rowAbove ? "transparent" : Theme.bgSurface }
            }
        }
        // Only over the row: what is under it is a way out of this band, not part of the target.
        Rectangle {
            id: pinHover
            y: pin.rowY
            width: parent.width
            height: Theme.graphRowHeight
            color: Theme.bgHover
            visible: pinMouse.containsMouse || pin.pointed
        }
    }

    /// The hover ground as drawn, for the headless run: reading the two conditions back would pass a stand-in whose
    /// ground is not wired to them (verify-ui).
    readonly property alias lit: pinHover.visible

    // The chip column, laid out as a row's is (`GraphRowChips`): the chip against the column's right edge, its names
    // cut to the column, and level with the row.
    RefChip {
        x: pin.labelWidth - width - Theme.spaceXs
        y: pin.rowMidY - height / 2
        records: pin.records
        maxWidth: pin.labelWidth - Theme.spaceSm
    }
    // This commit's lanes and its face, drawn by the very cell a row draws them with — same curves, same node, same
    // clipping, and nothing on it that this commit does not have. **Cut at the node's edge**: what the cell would draw
    // past it is a full-strength lane through the emptiest part of the row, and the going-out below takes that over.
    Item {
        x: pin.labelWidth
        y: pin.rowAbove ? pin.rowY : pin.nodeOutY
        width: pin.graphColWidth
        height: pin.rowAbove ? pin.nodeOutY - pin.rowY : pin.rowY + Theme.graphRowHeight - pin.nodeOutY
        clip: true
        GraphLaneCell {
            y: pin.rowAbove ? 0 : pin.rowY - pin.nodeOutY
            width: parent.width
            height: Theme.graphRowHeight
            xOffset: pin.graphXOffset
            fullWidth: pin.graphFullWidth
            geometry: pin.graphModel.headGeometry
            nodeLane: pin.graphModel.headLane
            // The stand-in shows the commit, not the credits: the badge is a second face, and this row is already
            // standing in for something (規約 §co-author の表示).
            coAuthors: ""
            avatar: pin.graphModel.headAvatar
            avatarUrl: pin.graphModel.headAvatarUrl
            isWip: false
            stashRef: ""
            dimmed: false
        }
    }
    // Where its lanes go: out of the node and into the ground, gone by the far edge of the hold. Only the ones that
    // leave on that side — a curve into the node reaches the top edge and one out of it the bottom (`SegmentKind`).
    Item {
        x: pin.labelWidth
        y: Math.min(pin.nodeOutY, pin.outTo)
        width: pin.graphColWidth
        height: Math.abs(pin.outTo - pin.nodeOutY)
        clip: true
        InkCanvas {
            id: goingOut
            anchors.fill: parent
            // Resizing a canvas scales what it already holds; only a repaint redraws it (`GraphLaneCell`).
            onWidthChanged: requestPaint()
            onHeightChanged: requestPaint()
            onPaint: {
                const ctx = getContext("2d")
                ctx.clearRect(0, 0, width, height)
                if (pin.graphModel.headGeometry === "")
                    return
                ctx.lineWidth = Metrics.laneStroke
                const toks = pin.graphModel.headGeometry.split(";")
                for (let n = 0; n < toks.length; n++) {
                    const t = toks[n]
                    const k = t[0].toLowerCase()
                    if (k !== "t" && k !== (pin.rowAbove ? "o" : "i"))
                        continue
                    const dot = t.indexOf(".")
                    const lane = parseInt(t.substring(1, dot))
                    const hex = Theme.graphLane[parseInt(t.substring(dot + 1)) % Theme.graphLane.length]
                    const x = Metrics.laneInset + lane * Metrics.laneW + Metrics.laneW / 2 - pin.graphXOffset
                    const fade = ctx.createLinearGradient(0, 0, 0, height)
                    fade.addColorStop(0, goingOut.faded(hex, pin.rowAbove ? 1 : 0))
                    fade.addColorStop(1, goingOut.faded(hex, pin.rowAbove ? 0 : 1))
                    ctx.strokeStyle = fade
                    ctx.setLineDash(t[0] === k ? [] : Metrics.laneDash)
                    ctx.beginPath()
                    ctx.moveTo(x, 0)
                    ctx.lineTo(x, height)
                    ctx.stroke()
                }
                ctx.setLineDash([])
            }
            /// A lane's colour at `a` of its strength, as a gradient stop.
            ///
            /// **`Theme.graphLane` holds strings, not colours** — the token is a `var` array, so `.r` / `.g` / `.b` off
            /// one is `undefined` and `Qt.rgba` of that is a transparent black that draws nothing at all
            /// (measured). Qt reads `#AARRGGBB`, so the alpha goes on the front of the string the token already is.
            function faded(hex, a) {
                const v = Math.round(Math.max(0, Math.min(1, a)) * 255)
                return "#" + (v < 16 ? "0" : "") + v.toString(16) + hex.substring(1)
            }
        }
    }
    // A canvas repaints only when it is asked to, and neither of these is its own property.
    onGraphXOffsetChanged: goingOut.requestPaint()
    onRowAboveChanged: goingOut.requestPaint()
    Connections {
        target: pin.graphModel
        function onStatsChanged() { goingOut.requestPaint() }
    }

    // The subject, behind the same tick a row puts before it — the three steps `GraphPane.subjectTextX` adds up.
    Rectangle {
        x: pin.labelWidth + pin.graphColWidth + Theme.spaceSm
        y: pin.rowMidY - height / 2
        width: 2 * Theme.borderWidth
        height: Theme.iconMd
        radius: Theme.borderWidth
        color: pin.laneColor
    }
    CutName {
        id: subject
        x: pin.labelWidth + pin.graphColWidth + Theme.spaceSm + 2 * Theme.borderWidth + Theme.spaceXs
        y: pin.rowMidY - height / 2
        // **The same box a row gives its message**: from the tick to the pane's own inset (デザイン規約 §余白), so this
        // commit's message is cut at exactly the character it is cut at down in the list. Read off the same numbers
        // rather than off `barRoom` — a stand-in that stopped at the bar's box cut a word earlier than the row it
        // stands for, and the message changed length as the reader scrolled it off.
        width: pin.width - subject.x - Theme.spaceXs
        cutAt: "end"
        text: pin.graphModel.headSubject
        pixelSize: Theme.fontMd
        // The whole of the emphasis, and it is the same blue the chip beside it already writes the current branch in —
        // `textLink` here, on the chip, and on this commit's own row down in the list (`GraphRowDelegate.isHead`).
        //
        // **A detached HEAD takes it too**: the colour answers "this is the commit you are on",
        // which is as true without a branch as with one. What is different there is said by the chip, which is the one
        // place a state belongs (規約 §状態).
        color: Theme.textLink
    }

    // **No rule along the edge the rows pass under**. The graph has no horizontal
    // lines anywhere, and one drawn here would be the loudest thing on the pane for as long as the branch is off
    // screen. What this row is is said by the chip, the colour of its words and the graph going out under them.

    // A left press goes to the row this stands for. **The right button is taken and dropped**: unclaimed it would fall
    // through to the row scrolling underneath, and a menu about a commit nobody can see is worse than no menu (the
    // middle button is left alone on purpose — it starts the pane's autoscroll from here as it does from a row).
    MouseArea {
        id: pinMouse
        y: pin.rowY
        // **The one thing that gives the bar room.** A press taken over the trough answered it with a jump to HEAD
        // (`barRoom`); the ink around it does not have to move for that.
        width: parent.width - pin.barRoom
        height: Theme.graphRowHeight
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: mouse => {
            if (mouse.button === Qt.LeftButton)
                pin.activated(pin.headRow)
        }
    }
}
