pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

/// HEAD's row, standing in at the edge it scrolled out of until the row itself is back on screen; a press goes to that
/// row (デザイン規約 §グラフの中で HEAD を見失わない). Drawn as that commit's own row — its chip, its own lanes and node
/// (`GraphLaneCell`), its subject — then dissolving into the ground, so no hard edge cuts the row underneath.
///
/// Adopted by the pane, not the list: inside the list, the lane column's press strip (`GraphLanePan`) would answer for
/// the row underneath.
Rectangle {
    id: pin

    required property var graphModel
    /// The list this rides; which edge it takes, and whether it shows, is read off it.
    required property var view
    /// The column geometry the rows use.
    required property real labelWidth
    required property real graphColWidth
    required property real graphFullWidth
    required property real graphXOffset
    /// The right strip that is the list's scroll bar. Only the press area stops short of it, or the trough jumps to
    /// HEAD; the ink and ground run under the bar as a row's do.
    required property real barRoom

    /// The stand-in was pressed. `modifiers` ride along: Ctrl / Shift are decided where every row press is
    /// (デザイン規約 §複数のコミットを選ぶ).
    signal activated(int row, int modifiers)
    /// The pointer came onto the chip: unfold it, as a row's chip unfolds (`GraphRowDelegate.openPointed`).
    signal chipExpandRequested(string oidHex, int atRow, var records, var anchor)
    /// And left it — put it back, unless it went into the card (only the owner can tell). `anchor` is the chip it left.
    signal chipCollapseRequested(var anchor)

    /// The pointer resting on this, for the headless run (hover cannot be injected).
    property bool pointed: false
    /// Where along the stand-in the pointer is, or -1 — the row's `pointerRowX`; a headless run writes it too.
    property real pointerX: -1
    /// The chip's card stands on this stand-in's chip (as `GraphRowDelegate.listOnThisChip`).
    readonly property bool listOnThisChip: pin.view.chipListAnchor === pinStack
    /// The pointer is on the chip column — divided at the column's edge, not the chip's frame, as a row divides it
    /// (`GraphRowDelegate.partAt`).
    readonly property bool chipPointed: pin.pointerX >= 0 && pin.pointerX < pin.labelWidth && pin.records.length > 0
    onChipPointedChanged: pin.settleChip()
    /// A second click is waiting out its window; hover holds still meanwhile, as on a row
    /// (`GraphRowDelegate.renameWaiting`), and what the hand did meanwhile — a leave included — settles when it ends.
    readonly property bool renameWaiting: pin.view.renameWaiting
    onRenameWaitingChanged: if (!pin.renameWaiting) pin.settleChip()
    /// Unfolds at once, as a row's chip does (規約 §hover のツールチップ「展開は即時に開く」).
    function settleChip() {
        if (pin.renameWaiting)
            return
        if (pin.chipPointed)
            pin.chipExpandRequested(pin.graphModel.oidAt(pin.headRow), pin.headRow, pin.records, pinStack)
        else
            pin.chipCollapseRequested(pinStack)
    }
    /// The hand came onto the stand-in at `x`, or left it — **every leave answered**, as on a row
    /// (`GraphRowDelegate.pointerCrossed`, which a run enters the same way).
    function pointerCrossed(inside, x) {
        pin.pointerX = inside ? x : -1
    }

    readonly property int headRow: pin.graphModel.headRow
    /// That row's top in the list's content coordinates — computed, since the view may not have built it (as
    /// `GraphRowWalk.revealStep`).
    readonly property real rowTop: pin.view.originY + pin.headRow * Theme.graphRowHeight
    readonly property bool rowAbove: pin.rowTop < pin.view.contentY
    readonly property bool rowBelow: pin.rowTop + Theme.graphRowHeight > pin.view.contentY + pin.view.height
    /// HEAD is inside the loaded window and its chips have arrived (`models::graph::head`).
    readonly property bool wanted: pin.headRow >= 0 && pin.graphModel.headLabels.length > 0
    /// The list's selection is on HEAD's row. The grounds read this, not each other's `visible` — that is the
    /// effective one, false whenever the stand-in is hidden.
    readonly property bool selected: pin.view.currentIndex === pin.headRow
    /// Chips the window has already said are gone, left out as the rows leave them out (`encode::chips_shown`) —
    /// HEAD's own branch never is, but another on the same commit can be.
    readonly property var goneChips: pin.graphModel.goneChips
    readonly property var records: GitFacts.chipsShown(pin.graphModel.headLabels, pin.goneChips)
    readonly property color laneColor: Theme.graphLane[pin.graphModel.headColor % Theme.graphLane.length]
    /// The search passed HEAD's row over, as `GraphRowDelegate.dimmed`. Read off the model: the row is off screen
    /// whenever this is up.
    readonly property bool dimmed: pin.view.findOn && !pin.graphModel.headMatched
    /// The row's box as its grounds add up (`GraphRowDelegate.laneGround`), read off the two as drawn: what its ink
    /// sinks into at the column's edge. The hold under it is the band's plain ground.
    readonly property color rowGround: pin.picked ? Theme.bgSelected
                                       : pin.lit ? Qt.tint(Theme.bgSurface, Theme.bgHover) : Theme.bgSurface

    // ---- the three bands --------------------------------------------------------------------------------------------
    //
    // The row, where the graph goes out, and where the rows come back — plus, at the top edge only, the list's sliver
    // above the row. Each flips with the edge this rides.

    /// How far past the row the ground stays whole: the graph has to be gone before the rows come back, or the two
    /// overlap and neither reads. The lanes dissolve in it, so it cannot go to zero.
    readonly property real hold: Theme.graphRowHeight / 4
    /// How long the rows take coming back. Hold and fade together stay a row and a half — shortening the whole
    /// steepens the gradient — and the fade keeps at least a row, or it cuts the commit it lands on.
    readonly property real fadeRoom: Theme.graphRowHeight * 5 / 4
    /// The list's sliver above its first row (`GraphList.topMargin`), kept at the top edge so the stand-in's foot meets
    /// the bands either side as the row's does. None at the bottom edge.
    readonly property real topRoom: pin.rowAbove ? pin.view.topMargin : 0
    readonly property real rowY: pin.rowAbove ? pin.topRoom : pin.hold + pin.fadeRoom
    readonly property real rowMidY: pin.rowY + Theme.graphRowHeight / 2
    /// A hairline past the node's edge: the row's own cell is cut here and the going-out takes over.
    readonly property real nodeOutY: pin.rowMidY
        + (pin.rowAbove ? 1 : -1) * (Metrics.nodeIcon / 2 + Theme.borderWidth)
    readonly property real groundRoom: pin.topRoom + Theme.graphRowHeight + pin.hold
    /// Where the going-out ends: the hold's far edge, while the ground is still whole.
    readonly property real outTo: pin.rowAbove ? pin.groundRoom : pin.fadeRoom

    visible: pin.wanted && (pin.rowAbove || pin.rowBelow)
    height: pin.groundRoom + pin.fadeRoom
    x: pin.view.x
    // The list's full width, so the message cuts where a row's does; `barRoom` comes off the press area only.
    width: pin.view.width
    y: pin.view.y + (pin.rowAbove ? 0 : pin.view.height - pin.height)
    color: "transparent"

    // The graph's own ground: opaque so the rows under it do not show through, and no lighter — the emphasis is the
    // colour of the words. Whole through the sliver, the row and the hold, then fading.
    //
    // Parented into the list (rules-refs/app-ui.md「張り付き行の地はリストの中・それ以外はペインの上」): full width, under
    // the bar (`AutoScrollBar` takes `z: 1`). Everything else stays out here.
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
        // A row's two grounds, in its order with selection winning (`GraphRowDelegate`), over the row and its sliver
        // (as `GraphRowDelegate.topBleed`).
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
            // The chip's card holds it lit, as on a row — the card takes the pointer the moment it is drawn
            // (`GraphRowDelegate.lit`).
            visible: (pinMouse.containsMouse || pin.pointed || pin.listOnThisChip) && !pin.selected
        }
    }

    /// The two grounds as drawn, for the headless run — not their conditions, which would pass unwired grounds.
    readonly property alias lit: pinHover.visible
    readonly property alias picked: pinPicked.visible
    /// The message's strength as drawn, for the headless run.
    readonly property alias wordsOpacity: subject.opacity
    /// The chip, sheets and all — what its card stands on, for the headless run.
    readonly property alias chipItem: pinStack

    // Laid out as a row's chip column is (`GraphRowChips`).
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
    // This commit's lanes and face, drawn by a row's own cell, cut at the node's edge where the going-out takes over.
    // Cut there only: sideways the cell reaches, and sinks, as far as a row's (`spaceSm` past the column).
    Item {
        x: pin.labelWidth
        y: pin.rowAbove ? pin.rowY : pin.nodeOutY
        width: pin.graphColWidth + Theme.spaceSm
        height: pin.rowAbove ? pin.nodeOutY - pin.rowY : pin.rowY + Theme.graphRowHeight - pin.nodeOutY
        clip: true
        GraphLaneCell {
            y: pin.rowAbove ? 0 : pin.rowY - pin.nodeOutY
            width: pin.graphColWidth
            height: Theme.graphRowHeight
            xOffset: pin.graphXOffset
            fullWidth: pin.graphFullWidth
            geometry: pin.graphModel.headGeometry
            nodeLane: pin.graphModel.headLane
            // No co-author badge on a stand-in (規約 §co-author の表示).
            coAuthors: []
            avatar: pin.graphModel.headAvatar
            avatarUrl: pin.graphModel.headAvatarUrl
            isWip: false
            stashRef: ""
            dimmed: pin.dimmed
            ground: pin.rowGround
        }
    }
    // The lanes leaving on this side, fading from the node to the hold's far edge (`into` reaches the top edge, `out`
    // the bottom — `SegmentKind`), and sinking at the column's edge as the row's do — where the two fades cross, the
    // ink is at the product of both.
    Item {
        x: pin.labelWidth
        y: Math.min(pin.nodeOutY, pin.outTo)
        width: pin.graphColWidth + Theme.spaceSm
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
            /// A lane's colour at `a` of its strength. `Theme.graphLane` holds strings, so `.r` / `.g` / `.b` are
            /// undefined and `Qt.rgba` of them draws nothing; the alpha goes on the front as `#AARRGGBB`.
            function faded(hex, a) {
                const v = Math.round(Math.max(0, Math.min(1, a)) * 255)
                return "#" + (v < 16 ? "0" : "") + v.toString(16) + hex.substring(1)
            }
        }
        // The edge, one strip per ground under it — the row's box, then the hold — each trimmed to the going-out by
        // this box's clip.
        LaneDissolve {
            x: pin.graphColWidth
            y: pin.rowY - parent.y
            height: Theme.graphRowHeight
            ground: pin.rowGround
        }
        LaneDissolve {
            x: pin.graphColWidth
            y: (pin.rowAbove ? pin.rowY + Theme.graphRowHeight : pin.rowY - pin.hold) - parent.y
            height: pin.hold
            ground: Theme.bgSurface
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

    // The message tick and subject, at the steps `GraphPane.subjectTextX` adds up.
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
        // The box a row gives its message, to the pane's inset (デザイン規約 §余白), so it cuts at the same character.
        width: pin.width - subject.x - Theme.spaceXs
        opacity: pin.dimmed ? Metrics.dimFade : 1
        cutAt: "end"
        text: pin.graphModel.headSubject
        pixelSize: Theme.fontMd
        // The whole emphasis: the current branch's blue, as on HEAD's own row (`GraphRowDelegate.isHead`). A detached
        // HEAD takes it too — the chip says what differs (規約 §状態).
        color: Theme.textLink
    }

    // No rule along the edge: the graph has no horizontal lines, and one here would be the loudest thing on the pane.

    // A left press goes to the row. The right button is taken and dropped, or it falls through to the row underneath;
    // the middle is left alone so the pane's autoscroll starts from here too.
    MouseArea {
        id: pinMouse
        // The ground's box, sliver and all (`pinPicked`) — what lights must take the press.
        y: pin.rowY - pin.topRoom
        // The only part that gives the bar room (`barRoom`).
        width: parent.width - pin.barRoom
        height: Theme.graphRowHeight + pin.topRoom
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        // On press, as the rows answer (`GraphRowDelegate`).
        onPressed: mouse => {
            if (mouse.button === Qt.LeftButton)
                pin.activated(pin.headRow, mouse.modifiers)
        }
        onPositionChanged: mouse => pin.pointerX = mouse.x
        onContainsMouseChanged: pin.pointerCrossed(pinMouse.containsMouse, pinMouse.mouseX)
    }
}
