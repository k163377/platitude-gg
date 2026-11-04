pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The hand a card's own words are dragged over from the air around them, and what it turns that drag into
// (規約 §hover のツールチップ「hover が出した字は選べて、コピーできる」).
//
// It lies under everything the card draws, so a press reaches it only where nothing else took one, and the card's own
// rows, links and badges keep theirs. Unlike `SweepRoom`, the nearest field wins outright: a card stacks its lines,
// and demanding a value's columns would kill the padding band (規約 §右のペインの字は掴める). A plain `MouseArea`
// (rules-refs/app-ui.md「手の実装は素の `MouseArea`」). Under a surface that is itself a target, a press that stays
// put is that target's click instead (`taps`).
//
// Where it goes on a scrolling surface: inside a `Flickable`, the pad lies inside the view (`SettingsDialog`'s
// chapters) — laid outside, it gets no press at all, the view swallowing whatever its children did not take; the
// hand's `preventStealing` keeps a drag from turning into a flick half-way. In a `ListView` each row carries its own
// pad (`ChosenCommitRow`); a hand over the whole list is `DiffTextSelect`'s, since `reuseItems` rebuilds the rows.
Item {
    id: pad

    /// The card's content, walked for fields on every press — a list kept in step with lines that come and go would
    /// leave the one it forgot a dead corner.
    required property Item content
    /// Whether what the pad lies under is itself a target (`ChosenCommitRow`): a press is then its click until it
    /// travels past `drag.threshold`, and only then a sweep, anchored where it went down — the words on a target
    /// (規約 §右のペインの字は掴める「掴み方は 3 つ」の 3, as `RowHand` over one value). Off, the press anchors at once.
    property bool taps: false
    /// A press on a pad that `taps`, let go before it travelled.
    signal tapped()

    /// What the gesture is holding: the field it is writing into, and whether it took the gesture at all.
    property Item field: null
    property bool taking: false
    /// Where the press went down, whether a pad that `taps` is holding one, and whether it has travelled far enough to
    /// be a sweep. A pad that does not tap starts every press travelled.
    property point pressFrom: Qt.point(0, 0)
    property bool pressing: false
    property bool travelled: false

    /// What a run reads back off a sweep (verify-ui): the value it landed on, and whether the keyboard went with the
    /// selection — `Ctrl+C` goes to whatever holds it.
    readonly property string endedOn: pad.field ? pad.field.text : ""
    readonly property bool caretLanded: !!pad.field && pad.field.hasCaret

    /// The hand below calls these, at points in this item's coordinates — and a run enters them, so a pad never wired
    /// up reports nothing (verify-ui §壊れない動詞の実装と反復). The anchor is taken at the press, unlike
    /// `SweepRoom`'s: the nearest field is already known there — on a pad that `taps`, once the press has travelled.
    /// The press drops the last selection either way: one selection in the window, and a click takes no words.
    function pressAt(position) {
        pad.dropValues()
        pad.field = pad.nearestTo(position)
        pad.taking = !!pad.field
        pad.pressFrom = position
        pad.pressing = pad.taps
        pad.travelled = !pad.taps
        if (pad.taking && pad.travelled)
            pad.field.anchorFrom(pad, position)
        return pad.taking || pad.taps
    }
    function moveAt(position) {
        if (!pad.travelled) {
            // A click with a tremor in it is still a click.
            if (Math.abs(position.x - pad.pressFrom.x) < hand.drag.threshold
                    && Math.abs(position.y - pad.pressFrom.y) < hand.drag.threshold)
                return
            pad.travelled = true
            if (pad.taking)
                pad.field.anchorFrom(pad, pad.pressFrom)
        }
        if (!pad.taking)
            return
        // One field per gesture (規約 §右のペインの字は掴める): changing half-way would drop the selection just made.
        pad.field.extendFrom(pad, position)
    }
    function releaseNow() {
        const tap = pad.pressing && !pad.travelled
        pad.cancelNow()
        if (tap)
            pad.tapped()
    }
    /// A press taken away (the view took it, the window lost it): neither a sweep going on nor a click.
    function cancelNow() {
        pad.taking = false
        pad.pressing = false
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
    function nearestTo(position) {
        const fields = pad.fields()
        let best = null
        let bestDown = 0
        let bestAcross = 0
        for (let i = 0; i < fields.length; i++) {
            const f = fields[i]
            const p = f.mapFromItem(pad, position)
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
    function fieldAt(position) {
        const fields = pad.fields()
        for (let i = 0; i < fields.length; i++) {
            const f = fields[i]
            const p = f.mapFromItem(pad, position)
            if (p.x >= 0 && p.x <= f.width && p.y >= 0 && p.y <= f.height)
                return f
        }
        return null
    }

    /// Automation: the card's air, sampled — a grid over the pad minus the points on a field, which leaves exactly the
    /// places a real press could reach this hand. A run pressing on the words would go green with this hand taken out
    /// (as `details-sweep`'s, `AutoActDetailsVerbs.detailsSweepTimer`).
    function airPoints(steps) {
        const out = []
        for (let i = 0; i < steps; i++) {
            for (let j = 0; j < steps; j++) {
                const p = Qt.point(pad.width * (i + 0.5) / steps, pad.height * (j + 0.5) / steps)
                if (!pad.fieldAt(p))
                    out.push(p)
            }
        }
        return out
    }

    /// Automation: the gesture as a hand makes it from a point of this pad's air, through the `MouseArea`'s three
    /// functions. The drag stays at the height it started from: a value read out at a height the reader never used
    /// passes while the gesture does not work.
    function sweepAt(position) {
        // A pad that `taps` takes a press with no field to anchor on.
        if (!pad.pressAt(position) || !pad.field)
            return false
        const f = pad.field
        // Toward the words: a field is as wide as the card, so a press level with a short line is usually past its
        // last glyph, and a drag to the box's far side would cross nothing.
        const near = f.mapToItem(pad, Qt.point(0, 0))
        const away = f.mapToItem(pad, Qt.point(f.width, 0))
        pad.moveAt(Qt.point(near.x, position.y))
        if (pad.sweptText() === "")
            pad.moveAt(Qt.point(away.x, position.y))
        pad.releaseNow()
        return true
    }
    /// Automation: every point of this pad's air swept in turn, as the one line every surface with this hand reports.
    /// `all=`, since how much air there is depends on the words and the machine; `reach=` beside it, since a reach that
    /// works from one point only is what a single point hides; `miss=` is the first start that came away with nothing,
    /// not the last try. `extra` goes ahead of the numbers so a `must_say` naming it reads as one run of words
    /// (`card_sweep … hand=true open=true`). A new surface's verb calls this and writes no line of its own.
    function sweepAir(steps, extra) {
        const air = pad.airPoints(steps)
        let reach = 0
        let miss = ""
        for (let i = 0; i < air.length; i++) {
            if (pad.sweepAt(air[i]) && pad.sweptText() !== "")
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
        // A target's hand, as `RowHand` wears over a value; words otherwise.
        cursorShape: pad.taps ? Qt.PointingHandCursor : Qt.IBeamCursor
        onPressed: mouse => { mouse.accepted = pad.pressAt(Qt.point(mouse.x, mouse.y)) }
        onPositionChanged: mouse => { if (hand.pressed) pad.moveAt(Qt.point(mouse.x, mouse.y)) }
        onReleased: pad.releaseNow()
        onCanceled: pad.cancelNow()
    }
}
