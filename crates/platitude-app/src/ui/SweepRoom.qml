pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The hand a range selection over a row of values is taken with, and what it turns a drag into
// (デザイン規約 §右のペインの字は掴める).
//
// **It lies under everything the row draws**, so a press reaches it only where no control and no value took one —
// every gap in the row, and nothing else. That is what makes "no hit area changed" structural:
// nothing is layered over anything, so the hash plate, the parent link and the avatar cannot lose a press
// to this. It is also what closes the dead corners a hand placed patch by patch kept leaving — the air above and
// below a value, the plate's own empty strip beside the parent hash.
//
// **A plain `MouseArea`.** A passive `PointHandler` over the row answered one move of a two-move
// drag inside the pane's Flickable, and none at all once anything else took the press (measured qmltestrunner
// `tst_inflick`); a `TapHandler` never taps at all over a selectable field, which takes the press (`tst_tapselect`).
// `preventStealing` is what keeps the Flickable from taking the drag away part-way through.
//
// **The line comes from where the press landed**, and the values standing on it are the only ones the gesture can
// reach. Reading the line off how near the pointer was to one row of glyphs picked the value above wherever the lines
// were taller (Ubuntu, same day). Across, the pointer does have to arrive over a value's own columns — that is what
// says which of the two on a line the drag went for. **One value per gesture**: changing field half-way would
// re-anchor, which takes the caret off the first and drops the selection the reader had just made.
Item {
    id: room

    /// The row whose values this sweeps. It answers which values stand on which line and which boxes are somebody's
    /// control, and it is the coordinate space the gesture is measured in.
    required property Item row

    /// What the gesture is holding: the field it is writing into, whether it took the gesture at all, and the values
    /// it may reach — the line it started on.
    property LineText field: null
    property bool taking: false
    property var lineFields: []

    /// What a run reads back off a sweep (verify-ui): the value it ended on, whether the keyboard went with the
    /// selection — `Ctrl+C` goes to whatever holds it, so a selection without one is not a value anybody can take
    /// away — and the numbers behind a sweep that came away with nothing.
    readonly property string endedOn: room.field ? room.field.text : ""
    readonly property bool caretLanded: !!room.field && room.field.hasCaret
    property string trace: ""

    /// The three the hand below calls, in this item's own coordinates — and the three a run enters, so a hand that
    /// was never wired up reports nothing (verify-ui §壊れない動詞の実装). A drag leaves the gap it started in as soon
    /// as it reaches a value, which is ordinary for a grabbed area: the points are mapped.
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

    /// The value the pointer has arrived over, in the row's coordinates. **Across is all this decides** — the line
    /// came with the press, and the height does not enter into it at all, which is the whole point of taking the
    /// gesture from the gaps. While the pointer is still out in one this answers nothing, and the anchor waits.
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

    /// Automation: a pointer cannot be injected, so a run enters the three functions above — **from every corner of
    /// the gap beside that value**, near and far, high and low. A reach that works from only one place in it is the
    /// fault this shipped with, and the middle is the one place that hides it.
    function sweepAt(which, fx, fy) {
        const f = room.row.fieldFor(which)
        const slack = room.row.slackFor(which)
        if (!f || !slack)
            return false
        // Across the slack, and down the whole band of that value's line: the air over and under a value belongs to
        // its line too, and a hand that did not answer there is what the reader ran into.
        const band = room.row.bandFor(which)
        const x = slack.mapToItem(room, Math.max(1, Math.min(slack.width - 1, slack.width * fx)), 0).x
        const y = band[0] + 1 + (band[1] - band[0] - 2) * fy
        if (!room.pressAt(x, y))
            return false
        // Onto the value and out its far side. Which side it is reached from depends on where the value sits: the
        // slack is at the end of the left column, so the words beside it are to the left and the plate to the right.
        // **Kept at the height the gesture started at** — a drag does not jump to the middle of the words, and reading
        // a value out at a height the reader never used is how this passed while it did not work.
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
        // Hover is theirs: the row's own cards and lights are somebody else's, and an area hovering under it would
        // take them (app-ui.md §HoverHandler は下の hover を殺す — a plain area does not, unless it asks for hover).
        hoverEnabled: false
        preventStealing: true
        cursorShape: Qt.IBeamCursor
        onPressed: mouse => room.pressAt(mouse.x, mouse.y)
        onPositionChanged: mouse => { if (hand.pressed) room.moveAt(mouse.x, mouse.y) }
        onReleased: room.releaseNow()
        onCanceled: room.releaseNow()
    }
}
