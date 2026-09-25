pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The hand a card's own words are dragged over from the air around them, and what it turns that drag into
// (規約 §hover のツールチップ「hover が出した字は選べて、コピーできる」).
//
// It lies under everything the card draws, so a press reaches it only where nothing else took one, and the card's own
// rows, links and badges keep theirs. Unlike `SweepRoom`, the nearest field wins outright: a card stacks its lines,
// and demanding a value's columns would kill the padding band (規約 §右のペインの字は掴める). A plain `MouseArea`
// (rules-refs/app-ui.md「手の実装は素の `MouseArea`」).
Item {
    id: pad

    /// The card's content, walked for fields on every press — a list kept in step with lines that come and go would
    /// leave the one it forgot a dead corner.
    required property Item content

    /// What the gesture is holding: the field it is writing into, and whether it took the gesture at all.
    property Item field: null
    property bool taking: false

    /// What a run reads back off a sweep (verify-ui): the value it landed on, and whether the keyboard went with the
    /// selection — `Ctrl+C` goes to whatever holds it.
    readonly property string endedOn: pad.field ? pad.field.text : ""
    readonly property bool caretLanded: !!pad.field && pad.field.hasCaret

    /// The three the hand below calls, in this item's coordinates — and the three a run enters, so a pad never wired up
    /// reports nothing (verify-ui §壊れない動詞の実装と反復). The anchor is taken at the press, unlike `SweepRoom`'s:
    /// the nearest field is already known there.
    function pressAt(x, y) {
        pad.dropValues()
        pad.field = pad.nearestTo(x, y)
        pad.taking = !!pad.field
        if (pad.taking)
            pad.field.anchorFrom(pad, x, y)
        return pad.taking
    }
    function moveAt(x, y) {
        if (!pad.taking)
            return
        // One field per gesture (規約 §右のペインの字は掴める): changing half-way would drop the selection just made.
        pad.field.extendFrom(pad, x, y)
    }
    function releaseNow() {
        pad.taking = false
    }

    /// Clears every selection under this card: one selection in the window, and a sweep clears the board first.
    function dropValues() {
        const fields = pad.fields()
        for (let i = 0; i < fields.length; i++)
            fields[i].deselect()
    }

    /// The fields this card is drawing, in walk order. Empty, sizeless or hidden ones are skipped: a sweep landing
    /// there would come away empty.
    function fields() {
        return pad.gather(pad.content, [])
    }
    function gather(item, out) {
        if (!item || !item.visible)
            return out
        if (item.sweepable === true) {
            if (item.width > 0 && item.height > 0 && item.text !== "")
                out.push(item)
            // A field's own children are its ruler and its box; nothing inside one is another field.
            return out
        }
        const kids = item.children
        for (let i = 0; i < kids.length; i++)
            pad.gather(kids[i], out)
        return out
    }

    /// The field nearest a point of this pad: vertical distance first, sideways to break a tie between fields sharing a
    /// line. Distance is to the field's box, so a point inside one wins outright.
    function nearestTo(x, y) {
        const fields = pad.fields()
        let best = null
        let bestDown = 0
        let bestAcross = 0
        for (let i = 0; i < fields.length; i++) {
            const f = fields[i]
            const p = f.mapFromItem(pad, x, y)
            const down = Math.max(0, Math.max(-p.y, p.y - f.height))
            const across = Math.max(0, Math.max(-p.x, p.x - f.width))
            if (!best || down < bestDown || (down === bestDown && across < bestAcross)) {
                best = f
                bestDown = down
                bestAcross = across
            }
        }
        return best
    }

    /// The field a point of this pad is on, or null on the card's air — the only places a real press reaches this hand.
    function fieldAt(x, y) {
        const fields = pad.fields()
        for (let i = 0; i < fields.length; i++) {
            const f = fields[i]
            const p = f.mapFromItem(pad, x, y)
            if (p.x >= 0 && p.x <= f.width && p.y >= 0 && p.y <= f.height)
                return f
        }
        return null
    }

    /// Automation: the card's air, sampled — a grid over the pad minus the points on a field, which leaves exactly the
    /// places a real press could reach this hand. A run pressing on the words would go green with this hand taken out
    /// (verbs.md の `details-sweep`「余白経由が要」).
    function airPoints(steps) {
        const out = []
        for (let i = 0; i < steps; i++) {
            for (let j = 0; j < steps; j++) {
                const x = pad.width * (i + 0.5) / steps
                const y = pad.height * (j + 0.5) / steps
                if (!pad.fieldAt(x, y))
                    out.push(Qt.point(x, y))
            }
        }
        return out
    }

    /// Automation: the gesture as a hand makes it from a point of this pad's air, through the `MouseArea`'s three
    /// functions. The drag stays at the height it started from: a value read out at a height the reader never used
    /// passes while the gesture does not work.
    function sweepAt(x, y) {
        if (!pad.pressAt(x, y))
            return false
        const f = pad.field
        // Toward the words: a field is as wide as the card, so a press level with a short line is usually past its
        // last glyph, and a drag to the box's far side would cross nothing.
        const near = f.mapToItem(pad, 0, 0)
        const away = f.mapToItem(pad, f.width, 0)
        pad.moveAt(near.x, y)
        if (pad.sweptText() === "")
            pad.moveAt(away.x, y)
        pad.releaseNow()
        return true
    }
    /// Automation: every point of this pad's air swept in turn, as the one line every surface with this hand reports.
    /// `all=`, since how much air there is depends on the words and the machine; `reach=` beside it, since a reach that
    /// works from one point only is what a single point hides; `miss=` is the first start that came away with nothing,
    /// not the last try. `extra` goes ahead of the numbers so a `must_say` naming it reads as one run of words.
    function sweepAir(steps, extra) {
        const air = pad.airPoints(steps)
        let reach = 0
        let miss = ""
        for (let i = 0; i < air.length; i++) {
            if (pad.sweepAt(air[i].x, air[i].y) && pad.sweptText() !== "")
                reach++
            else if (miss === "")
                miss = Math.round(air[i].x) + "," + Math.round(air[i].y) + "," + pad.sweptTrace()
        }
        return "all=" + (air.length > 0 && reach === air.length)
            + " caret=" + pad.caretLanded
            + " hand=" + pad.handStands
            + (extra ? " " + extra : "")
            + " reach=" + reach + "/" + air.length
            + " miss=[" + miss + "]"
            + " text=" + pad.sweptText()
    }
    /// Automation: what the sweep came away with.
    function sweptText() {
        return pad.field ? pad.field.selected : ""
    }
    /// Automation: the numbers behind a sweep that came away with nothing — the field's size, where the drag anchored
    /// and the text's length.
    function sweptTrace() {
        const f = pad.field
        return f ? Math.round(f.width) + "x" + Math.round(f.height) + ",a" + f.grabAnchor + ",n" + f.text.length : "-"
    }
    /// Automation: that there is a hand at all. A run enters the functions above, so a pad whose `MouseArea` was taken
    /// out, disabled or shrunk would still answer every sweep.
    readonly property bool handStands: hand.enabled && hand.width === pad.width && hand.height === pad.height

    MouseArea {
        id: hand
        anchors.fill: parent
        // Hover stays with the card's own words and rows, which report the pointer for the card; a hovering area would
        // take it (rules-refs/app-ui.md「行に重ねる面の `HoverHandler` は祖先が持つ」).
        hoverEnabled: false
        preventStealing: true
        cursorShape: Qt.IBeamCursor
        onPressed: mouse => { mouse.accepted = pad.pressAt(mouse.x, mouse.y) }
        onPositionChanged: mouse => { if (hand.pressed) pad.moveAt(mouse.x, mouse.y) }
        onReleased: pad.releaseNow()
        onCanceled: pad.releaseNow()
    }
}
