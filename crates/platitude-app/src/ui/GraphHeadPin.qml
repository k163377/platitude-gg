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
/// **The pane adopts this** (`GraphPane`), which is the other way round from the sidebar's. The lane
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
    /// the pane, and the ground is laid under the bar (`band`).
    required property real barRoom

    /// The stand-in was pressed: go to the row it stands for.
    /// The stand-in was pressed. `modifiers` rides along because this is the row: what Ctrl and Shift do to the
    /// choice is decided where every other press decides it (デザイン規約 §複数のコミットを選ぶ).
    signal activated(int row, int modifiers)
    /// The pointer came onto the chip: unfold it, as a row's chip unfolds (`GraphRowDelegate.openPointed`). The row
    /// comes with it — every name in the card is on that one commit, and the card's rows are that row's.
    signal chipExpandRequested(string oidHex, int atRow, var records, var anchor)
    /// And left it — put it back, unless it went into the card (only the owner can tell).
    signal chipCollapseRequested()

    /// The pointer resting on this, for the headless run — hover cannot be injected, so what the pointer would light is
    /// written in the same one place the pointer's own arrival writes (verify-ui).
    property bool pointed: false
    /// Where along the stand-in the pointer is, or -1 for "not on it" — the row's own `pointerRowX`, written by the
    /// press area below and by a headless run the same way.
    property real pointerX: -1
    /// The card the chip unfolds into is standing on this stand-in's chip — the page's answer, read back through the
    /// list the way a row reads it (`GraphRowDelegate.listOnThisChip`).
    readonly property bool listOnThisChip: pin.view.chipListAnchor === pinStack
    /// The pointer is on the chip's side of the stand-in. **The row's own division** (`GraphRowDelegate.partAt`): the
    /// chip column's edge, which is a line on screen, and not the chip's frame.
    readonly property bool chipPointed: pin.pointerX >= 0 && pin.pointerX < pin.labelWidth && pin.records.length > 0
    onChipPointedChanged: pin.settleChip()
    /// A second click is waiting out its window somewhere in this graph, and hover is held still while it runs — the
    /// row's rule (`GraphRowDelegate.renameWaiting`). The card's rows take that click, and a card standing here is one.
    readonly property bool renameWaiting: pin.view.renameWaiting
    onRenameWaitingChanged: if (!pin.renameWaiting && pin.pointerX >= 0) pin.settleChip()
    /// **Unfolded at once, as a row's chip is** (規約 §hover のツールチップ「展開は即時」) — a chip wearing a `+N` has
    /// been pointed at on purpose.
    function settleChip() {
        if (pin.renameWaiting)
            return
        if (pin.chipPointed)
            pin.chipExpandRequested(pin.graphModel.oidAt(pin.headRow), pin.headRow, pin.records, pinStack)
        else
            pin.chipCollapseRequested()
    }
    /// What the pointer skips while the card stands on the chip (see the area below) it takes back the moment the card
    /// goes, or a hand coming back is taken for one that never left.
    onListOnThisChipChanged: {
        if (!pin.listOnThisChip && !pinMouse.containsMouse)
            pin.pointerX = -1
    }

    readonly property int headRow: pin.graphModel.headRow
    /// Where that row sits in the list's own content coordinates. Worked out: the row is
    /// two thousand rows away and the view has not built it (`GraphRowWalk.revealStep` does the same arithmetic).
    readonly property real rowTop: pin.view.originY + pin.headRow * Theme.graphRowHeight
    readonly property bool rowAbove: pin.rowTop < pin.view.contentY
    readonly property bool rowBelow: pin.rowTop + Theme.graphRowHeight > pin.view.contentY + pin.view.height
    /// A HEAD outside the loaded window has no row to lead to, and one whose chips have not arrived yet has no name to
    /// show — neither is a stand-in worth drawing (`models::graph::head`).
    readonly property bool wanted: pin.headRow >= 0 && pin.graphModel.headLabels.length > 0
    /// The list's selection is on the row this stands in for — the same question a row asks of itself
    /// (`GraphRowDelegate.selected`), asked from out here because the stand-in is not one of the list's delegates.
    /// **The grounds below read this**: Qt's `visible` is the effective one, so a
    /// band that asked its neighbour would be answering "is the stand-in drawn at all" in the same breath.
    readonly property bool selected: pin.view.currentIndex === pin.headRow
    /// The chips the stand-in draws, less the ones the window has already said are gone — the same answer the rows
    /// themselves give (`encode::chips_shown`). HEAD's own branch is never one of them (git refuses to delete the
    /// branch it is on), but another branch standing on the same commit can be.
    readonly property var goneChips: pin.graphModel.goneChips
    readonly property var records: GitFacts.chipsShown(pin.graphModel.headLabels, pin.goneChips)
    readonly property color laneColor: Theme.graphLane[pin.graphModel.headColor % Theme.graphLane.length]
    /// The search passed the row this stands for over — said the way the row says it (`GraphRowDelegate.dimmed`):
    /// the chip, the lanes, the face and the message go down, and the lanes going out start from that strength. Read
    /// off the model, because the row is off screen whenever this is up; a stand-in that stayed lit would light the
    /// row as it scrolled off.
    readonly property bool dimmed: pin.view.findOn && !pin.graphModel.headMatched

    // ---- the three bands ---------------------------------------------------------------------------------------------
    //
    // The row, then the half where the graph goes out, then the half where the rows come back — and, at the top edge
    // only, the list's own sliver above the row. Everything below hangs off these numbers, and each of them flips with
    // the edge this rides.

    /// How far past the row the ground stays whole. **The graph has to be gone before the rows start coming back**
    /// — without this the two overlap and neither reads, and what is left below the
    /// stand-in is a row at half strength rather than a stretch of nothing.
    ///
    /// **A quarter of a row**: the whole going-out sits that much higher, so the
    /// commit directly under the stand-in is most of the way back. It is still the
    /// only room the lanes have to dissolve in, so it cannot go to nothing — a hard edge is what is on the other side.
    readonly property real hold: Theme.graphRowHeight / 4
    /// And how long they take coming back. **What the hold gives up, this takes** — the two
    /// together are a row and a half whatever the split, so the stand-in stands on the same 2.5 rows it always did and
    /// only the shape of the dissolve moves. Shortening the whole instead would put the same drop in brightness across
    /// a shorter distance, which is the one thing a gradient is here to avoid. A row is the floor: less, and the commit
    /// it lands on is cut across.
    readonly property real fadeRoom: Theme.graphRowHeight * 5 / 4
    /// The sliver the list keeps above its own first row, kept above this one as well while it rides that edge
    /// (`GraphList.topMargin`). **The stand-in stands where the row it stands for stands**: without it the stand-in
    /// begins at the pane's own top and its foot lands a few pixels above the bands either side of the graph, which
    /// is the one line that margin is there to meet — so the row appears to step up the moment it is scrolled off.
    /// Nothing of the kind belongs at the other edge: the run-out down there says the history is cut, and has no band
    /// to line up with.
    readonly property real topRoom: pin.rowAbove ? pin.view.topMargin : 0
    readonly property real rowY: pin.rowAbove ? pin.topRoom : pin.hold + pin.fadeRoom
    readonly property real rowMidY: pin.rowY + Theme.graphRowHeight / 2
    /// Where the lane leaves the node — **a hairline past the node's own edge**. The row's
    /// own cell is cut here and the going-out takes over, so the last full-strength pixel of graph is the
    /// one against the face.
    readonly property real nodeOutY: pin.rowMidY
        + (pin.rowAbove ? 1 : -1) * (Metrics.nodeIcon / 2 + Theme.borderWidth)
    /// How much of this the ground holds whole: the sliver above, the row, and the hold past it.
    readonly property real groundRoom: pin.topRoom + Theme.graphRowHeight + pin.hold
    /// And where it has gone: the far edge of the hold, so the ground is still whole when the last of the graph goes.
    readonly property real outTo: pin.rowAbove ? pin.groundRoom : pin.fadeRoom

    visible: pin.wanted && (pin.rowAbove || pin.rowBelow)
    height: pin.groundRoom + pin.fadeRoom
    x: pin.view.x
    // The list's own width, so every column in here lands where a row's lands — the message included. `barRoom` is
    // taken off the press area alone (`pinMouse`).
    width: pin.view.width
    y: pin.view.y + (pin.rowAbove ? 0 : pin.view.height - pin.height)
    color: "transparent"

    // **The graph's own ground, and nothing more.** Opaque it has to be where this stands — the rows run under it, and
    // a stand-in they showed through would read as two commits in one band — but a lighter band would be an emphasis
    // that is up for as long as the branch is off screen, and the only thing worth emphasising here is one line of
    // it. What says "this one is yours" is the colour of its words.
    //
    // It holds through the row and half a row past it — the stretch where the graph has gone and nothing has come back
    // yet, which is what makes the two read as one movement — and lets go over the row after
    // that. **The sliver this keeps above itself at the top edge is inside it** (`topRoom`): a ground that began at the
    // row would let whatever is scrolling past show in the strip the list leaves empty when it is at rest.
    // **The ground is laid inside the list; everything else stands over it out here.** Two things have to hold at once
    // and only this seat holds both. It has to cover the whole width — a row's message runs on under the bar
    // (規約 §余白), so a band that stopped short of the bar would leave the tail of whatever is scrolling past showing
    // in that strip. And the bar stays in sight. A child of the view is drawn over the rows and
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
            height: pin.groundRoom
            color: Theme.bgSurface
        }
        Rectangle {
            y: pin.rowAbove ? pin.groundRoom : 0
            width: parent.width
            height: pin.fadeRoom
            gradient: Gradient {
                GradientStop { position: 0; color: pin.rowAbove ? Theme.bgSurface : "transparent" }
                GradientStop { position: 1; color: pin.rowAbove ? "transparent" : Theme.bgSurface }
            }
        }
        // The two grounds a row draws, in the row's own order and with the row's own one winning
        // (`GraphRowDelegate`): the commit that was clicked keeps saying so while it is off screen — losing the
        // selection at the edge is losing the one thing the reader put there — and the pointer says nothing on top of
        // that, because a selected row does not brighten under it either.
        //
        // **Only over the row, and over the sliver it keeps above itself** — what is under it is a way out of this
        // band, while the sliver is the row's own (`topRoom`): a band that stopped at the
        // row's top edge would leave a dark strip between it and the bands either side of the graph, which is what
        // the first row's own bleed is there to close.
        Rectangle {
            id: pinPicked
            y: pin.rowY - pin.topRoom
            width: parent.width
            height: Theme.graphRowHeight + pin.topRoom
            color: Theme.bgSelected
            visible: pin.selected
        }
        Rectangle {
            id: pinHover
            y: pinPicked.y
            width: parent.width
            height: pinPicked.height
            color: Theme.bgHover
            // The card the chip unfolds into holds it up as well, the way it holds a row's: the card takes the
            // pointer off this the moment it is drawn (`GraphRowDelegate.lit`).
            visible: (pinMouse.containsMouse || pin.pointed || pin.listOnThisChip) && !pin.selected
        }
    }

    /// The two grounds as drawn, for the headless run: reading their conditions back would pass a stand-in whose
    /// ground is not wired to them (verify-ui).
    readonly property alias lit: pinHover.visible
    readonly property alias picked: pinPicked.visible
    /// The message's strength as drawn, for the headless run: what tells a stand-in the search passed over from a lit one.
    readonly property alias wordsOpacity: subject.opacity
    /// The chip, sheets and all — what the card it unfolds into stands on, for the headless run that points at it and
    /// reads the card against it.
    readonly property alias chipItem: pinStack

    // The chip column, laid out as a row's is (`GraphRowChips`): the chip against the column's right edge, its names
    // cut to the column, and level with the row.
    RefChipStack {
        id: pinStack
        x: pin.labelWidth - width - Theme.spaceXs
        y: pin.rowMidY - height / 2 - pinStack.lift
        opacity: pin.dimmed ? Metrics.dimFade : 1
        records: pin.records
        maxWidth: pin.labelWidth - Theme.spaceSm
        // The card takes the fan's ground, as it does on a row (`GraphRowChips.listOpen`).
        unstacked: pin.listOnThisChip
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
            // The stand-in shows the commit: the badge is a second face, and this row is already
            // standing in for something (規約 §co-author の表示).
            coAuthors: []
            avatar: pin.graphModel.headAvatar
            avatarUrl: pin.graphModel.headAvatarUrl
            isWip: false
            stashRef: ""
            dimmed: pin.dimmed
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
                ctx.lineWidth = Metrics.laneStroke
                // Out of the node at the strength the row's own lanes have, so a dimmed row does not grow a lit tail.
                const full = pin.dimmed ? Metrics.dimFade : 1
                for (const seg of pin.graphModel.headGeometry) {
                    if (seg.kind !== "through" && seg.kind !== (pin.rowAbove ? "out" : "into"))
                        continue
                    const hex = Theme.graphLane[seg.color % Theme.graphLane.length]
                    const x = Metrics.laneInset + seg.lane * Metrics.laneW + Metrics.laneW / 2 - pin.graphXOffset
                    const fade = ctx.createLinearGradient(0, 0, 0, height)
                    fade.addColorStop(0, goingOut.faded(hex, pin.rowAbove ? full : 0))
                    fade.addColorStop(1, goingOut.faded(hex, pin.rowAbove ? 0 : full))
                    ctx.strokeStyle = fade
                    ctx.setLineDash(seg.dashed ? Metrics.laneDash : [])
                    ctx.beginPath()
                    ctx.moveTo(x, 0)
                    ctx.lineTo(x, height)
                    ctx.stroke()
                }
                ctx.setLineDash([])
            }
            /// A lane's colour at `a` of its strength, as a gradient stop.
            ///
            /// **`Theme.graphLane` holds strings** — the token is a `var` array, so `.r` / `.g` / `.b` off
            /// one is `undefined` and `Qt.rgba` of that is a transparent black that draws nothing at all
            /// (measured). Qt reads `#AARRGGBB`, so the alpha goes on the front of the string the token already is.
            function faded(hex, a) {
                const v = Math.round(Math.max(0, Math.min(1, a)) * 255)
                return "#" + (v < 16 ? "0" : "") + v.toString(16) + hex.substring(1)
            }
        }
    }
    // A canvas repaints only when it is asked to, and none of these is its own property.
    onGraphXOffsetChanged: goingOut.requestPaint()
    onRowAboveChanged: goingOut.requestPaint()
    onDimmedChanged: goingOut.requestPaint()
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
        opacity: pin.dimmed ? Metrics.dimFade : 1
        color: pin.laneColor
    }
    CutName {
        id: subject
        x: pin.labelWidth + pin.graphColWidth + Theme.spaceSm + 2 * Theme.borderWidth + Theme.spaceXs
        y: pin.rowMidY - height / 2
        // **The same box a row gives its message**: from the tick to the pane's own inset (デザイン規約 §余白), so this
        // commit's message is cut at exactly the character it is cut at down in the list. Read off the same numbers
        // — a stand-in that stopped at the bar's box cut a word earlier than the row it
        // stands for, and the message changed length as the reader scrolled it off.
        width: pin.width - subject.x - Theme.spaceXs
        opacity: pin.dimmed ? Metrics.dimFade : 1
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

    // **The edge the rows pass under is bare**. The graph has no horizontal
    // lines anywhere, and one drawn here would be the loudest thing on the pane for as long as the branch is off
    // screen. What this row is is said by the chip, the colour of its words and the graph going out under them.

    // A left press goes to the row this stands for. **The right button is taken and dropped**: unclaimed it would fall
    // through to the row scrolling underneath, and a menu about a commit nobody can see is worse than no menu (the
    // middle button is left alone on purpose — it starts the pane's autoscroll from here as it does from a row).
    MouseArea {
        id: pinMouse
        // The ground's own box, sliver and all (`pinPicked`): a strip that lights under the pointer and takes no press
        // is a target that moves as the reader arrives at it.
        y: pin.rowY - pin.topRoom
        // **The one thing that gives the bar room.** A press taken over the trough answered it with a jump to HEAD
        // (`barRoom`); the ink around it does not have to move for that.
        width: parent.width - pin.barRoom
        height: Theme.graphRowHeight + pin.topRoom
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        // At the press, as the rows below answer theirs (`GraphRowDelegate`). The right button is still only taken
        // and dropped, which needs no handler at all.
        onPressed: mouse => {
            if (mouse.button === Qt.LeftButton)
                pin.activated(pin.headRow, mouse.modifiers)
        }
        onPositionChanged: mouse => pin.pointerX = mouse.x
        onContainsMouseChanged: {
            if (pinMouse.containsMouse) {
                pin.pointerX = pinMouse.mouseX
                return
            }
            // The card opens *on* the chip, so this loses the pointer the instant it is drawn — and that leave says
            // nothing about where the hand went (`GraphRowDelegate`, the same skip).
            if (pin.listOnThisChip)
                return
            pin.pointerX = -1
        }
    }
}
