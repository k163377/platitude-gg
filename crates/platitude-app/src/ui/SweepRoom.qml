pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The hand a range selection over a row of values is taken with, and what it turns a drag into
// (デザイン規約 §右のペインの字は掴める). It lies under everything the row draws, so a press reaches it only in the
// row's gaps, and the hash plate, the parent link and the avatar keep theirs.
//
// A plain `MouseArea` (rules-refs/app-ui.md「手の実装は素の `MouseArea`」); `preventStealing` keeps the pane's
// Flickable from taking the drag part-way through.
//
// The line comes from where the press landed — reading it off the nearest row of glyphs picks the value above where
// lines are taller. Across, the pointer has to arrive over a value's own columns, which says which of a line's two
// values the drag went for. One value per gesture: changing field half-way re-anchors, dropping the selection made.
Item {
    id: room

    /// The row whose values this sweeps: it answers which values stand on which line and which boxes are somebody's
    /// control, and is the coordinate space the gesture is measured in.
    required property Item row

    /// What the gesture is holding: the field it is writing into, whether it took the gesture at all, and the values
    /// it may reach — the line it started on.
    property LineText field: null
    property bool taking: false
    property var lineFields: []

    /// What a run reads back off a sweep (verify-ui): the value it ended on, whether the keyboard went with the
    /// selection (`Ctrl+C` goes to whatever holds it), and the numbers behind a sweep that came away with nothing.
    readonly property string endedOn: room.field ? room.field.text : ""
    readonly property bool caretLanded: !!room.field && room.field.hasCaret
    property string trace: ""

    /// The three the hand below calls, in this item's coordinates — and the three a run enters, so a hand never wired
    /// up reports nothing (verify-ui §壊れない動詞の実装と反復). Points are mapped into the row: a drag leaves its gap
    /// as soon as it reaches a value.
    function pressAt(x, y) {
        const p = room.mapToItem(room.row, x, y)
        room.field = null
        room.lineFields = room.row.lineValues(room.row.lineAt(p.y))
        room.taking = !room.row.claimedAt(room.row, p.x, p.y)
        if (room.taking)
            room.row.dropValues()
        return room.taking
    }
    function moveAt(x, y) {
        if (!room.taking)
            return
        const p = room.mapToItem(room.row, x, y)
        if (!room.field) {
            const on = room.fieldUnder(p.x, p.y)
            if (!on)
                return
            room.field = on
            on.anchorFrom(room.row, p.x, p.y)
        }
        room.field.extendFrom(room.row, p.x, p.y)
    }
    function releaseNow() {
        room.taking = false
    }

    /// The value of the press's line the pointer has arrived over, in the row's coordinates — decided across only.
    /// Null while the pointer is still in a gap, and the anchor waits.
    function fieldUnder(x, y) {
        const fields = room.lineFields
        for (let i = 0; i < fields.length; i++) {
            const f = fields[i]
            if (!f || !f.visible || f.width <= 0 || f.text === "")
                continue
            const p = f.mapFromItem(room.row, x, y)
            if (p.x >= 0 && p.x <= f.width)
                return f
        }
        return null
    }

    /// Automation: the gesture from a point (`fx`, `fy`) of the gap beside that value, entered through the three
    /// functions above. Runs start from every corner of the gap: the middle is the one place that hides a reach that
    /// works from one point only.
    function sweepAt(which, fx, fy) {
        const f = room.row.fieldFor(which)
        const slack = room.row.slackFor(which)
        if (!f || !slack)
            return false
        // Across the slack, and down the whole band of that value's line: the air over and under a value is its line's.
        const band = room.row.bandFor(which)
        const x = slack.mapToItem(room, Math.max(1, Math.min(slack.width - 1, slack.width * fx)), 0).x
        const y = band[0] + 1 + (band[1] - band[0] - 2) * fy
        if (!room.pressAt(x, y))
            return false
        // Onto the value and out its far side, from whichever side the value sits on, at the height the gesture
        // started at — a value read out at a height the reader never used passes while the gesture does not work.
        const mid = f.mapToItem(room, f.width / 2, f.height / 2)
        const leftFirst = x < mid.x
        const near = f.mapToItem(room, leftFirst ? 0 : f.width, f.height / 2)
        const far = f.mapToItem(room, leftFirst ? f.width : 0, f.height / 2)
        room.moveAt(near.x, y)
        room.moveAt(far.x, y)
        room.releaseNow()
        room.trace = Math.round(f.width) + "x" + Math.round(f.height)
                     + ",slack" + Math.round(slack.width) + "x" + Math.round(slack.height)
                     + ",near" + Math.round(near.x) + ",far" + Math.round(far.x)
                     + ",y" + Math.round(y) + ",a" + f.grabAnchor + ",c" + f.caretPos
        return true
    }

    /// Automation: the other half — a press on a control is left alone. The plate keeps its hit area only for as long
    /// as this answers false there.
    function pressOnControl(which) {
        const at = room.row.controlPoint(which, room)
        const took = room.pressAt(at.x, at.y)
        room.releaseNow()
        return took
    }

    MouseArea {
        id: hand
        anchors.fill: parent
        // Hover stays with the row's own cards and lights; a hovering area would take it
        // (rules-refs/app-ui.md「行に重ねる面の `HoverHandler` は祖先が持つ」).
        hoverEnabled: false
        preventStealing: true
        cursorShape: Qt.IBeamCursor
        onPressed: mouse => room.pressAt(mouse.x, mouse.y)
        onPositionChanged: mouse => { if (hand.pressed) room.moveAt(mouse.x, mouse.y) }
        onReleased: room.releaseNow()
        onCanceled: room.releaseNow()
    }
}
