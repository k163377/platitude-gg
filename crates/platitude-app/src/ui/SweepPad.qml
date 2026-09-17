pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The hand a card's own words are dragged over from the air around them, and what it turns that drag into
// (規約 §hover のツールチップ「出したものは持ち帰れる」).
//
// **It lies under everything the card draws**, in the card's ground, so a press reaches
// it only where nothing else took one — the padding band, the step between two lines, the room beside a short one, and
// nothing else. That is what makes "no hit area changed" structural: nothing is
// layered over anything, so a card's own rows, links and badges cannot lose a press to this (measured qmltestrunner
// `tst_cardpad`: a press in the padding reaches this, a press on the words is the words' and never arrives here).
//
// **The sibling of `SweepRoom`, and different where the shape is different.** That one serves a row with two values on
// a line, so it decides the line from the press and then demands the pointer arrive over a value's own columns — which
// of the two the reader meant cannot be said any other way. A card stacks its lines, so here the **nearest**
// field wins outright: vertical first, sideways to break a tie. Demanding the columns in a card would kill the whole
// padding band, which is the one place this is for.
//
// **A plain `MouseArea`** — the pair measured for the pane holds here too (`SweepRoom`): a passive
// `PointHandler` loses half a drag and a `TapHandler` never fires over a selectable field.
Item {
    id: pad

    /// The card's content, and the subtree the fields are found in. The pad walks it on every press:
    /// a card whose lines come and go with what it is describing would otherwise need that list kept in
    /// step, and the one it forgot is a dead corner nobody sees.
    required property Item content

    /// What the gesture is holding: the field it is writing into, and whether it took the gesture at all.
    property Item field: null
    property bool taking: false

    /// What a run reads back off a sweep (verify-ui): the value it landed on, and whether the keyboard went with the
    /// selection — `Ctrl+C` goes to whatever holds it, so a selection without one is not a value anybody can take away.
    readonly property string endedOn: pad.field ? pad.field.text : ""
    readonly property bool caretLanded: !!pad.field && pad.field.hasCaret

    /// The three the hand below calls, in this item's own coordinates — and the three a run enters, so a pad that was
    /// never wired up reports nothing (verify-ui §壊れない動詞の実装).
    ///
    /// **The anchor is taken at the press**, unlike the pane's, because the nearest field is already known there: a
    /// reader who presses in the padding and drags gets the selection running from the character nearest where they
    /// started, which is what a press in the margin of any other text does.
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
        // **One field per gesture**: changing half-way would re-anchor, which takes the caret off the first and drops
        // the selection the reader had just made (規約 §右のペインの字は掴める, the same rule).
        pad.field.extendFrom(pad, x, y)
    }
    function releaseNow() {
        pad.taking = false
    }

    /// Nothing under this card is holding a selection any more. One selection in the window, and a sweep clears the
    /// board before it puts one anywhere.
    function dropValues() {
        const fields = pad.fields()
        for (let i = 0; i < fields.length; i++)
            fields[i].deselect()
    }

    /// The fields this card is drawing, in the order the walk finds them. A field with nothing in it, no width or no
    /// place on screen is not one: a sweep that landed on it would come away empty and read as a hand that failed.
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

    /// The field nearest a point of this pad. **Vertical first, sideways to break a tie**: a card's lines are stacked,
    /// so the line the press is level with is the one it means, whatever it is level with it at — and where two fields
    /// share a line (a badge's sentence beside it, a hash beside its date), the nearer of the two is the answer.
    /// Distance is to the field's box, so a point inside one is at zero and wins outright.
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

    /// The field a point of this pad is actually on, or null where it is on the card's air. What a real press can
    /// reach this hand at is the second of those and nothing else — the words take their own.
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

    /// Automation: **every part of the card's air, sampled**. A grid is laid over the pad and the
    /// points standing on a field are dropped, which leaves exactly the places a real press could reach this hand —
    /// the padding band on all four sides, the step between two lines, the room beside a short one. A run that pressed
    /// the middle of a card would be pressing on the words, which take their own press, and it would go green with the
    /// whole of this hand taken back out (`details-sweep`: 注入は余白を経由する).
    ///
    /// A reach that works from only one place in the air is the fault the right pane shipped with, and any single
    /// point is a place that hides it — so the count of what was sampled is reported beside the count that worked.
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

    /// Automation: the gesture as a hand makes it, from a point of this pad's air. It enters the same three functions
    /// the `MouseArea` below calls. The drag runs to the far side of whatever field the press picked, **at the height
    /// it started from** — a drag does not jump to the middle of the words, and reading a value out at a height the
    /// reader never used is how a run passes while the gesture does not work.
    function sweepAt(x, y) {
        if (!pad.pressAt(x, y))
            return false
        const f = pad.field
        // **Toward the words.** A card's fields are as wide as the card it stands in, so a press
        // level with a short line is usually out past that line's last glyph — and the far side of the *box* is
        // further out still, where a drag crosses nothing at all (`card_sweep miss=[98,32,236x20,a9,n9]`: a nine
        // letter name in a 236px field, both ends of the drag on character 9). The words begin at the field's near
        // edge, so that is where the drag goes; a press that had already landed on the head of the line goes the
        // other way instead, which is the same gesture carried on past where it started.
        const near = f.mapToItem(pad, 0, 0)
        const away = f.mapToItem(pad, f.width, 0)
        pad.moveAt(near.x, y)
        if (pad.sweptText() === "")
            pad.moveAt(away.x, y)
        pad.releaseNow()
        return true
    }
    /// Automation: every place in this pad's air, swept one after another, as the one line the run reports. Eight
    /// surfaces carry this hand now and each one of them was asking the same four things, so the sentence is written
    /// here once.
    ///
    /// **`all=`**, because how much air a surface has depends on the words in it and on the
    /// machine that drew them — but the count rides along as `reach=`, because a reach that works from only one place
    /// in the air is the fault the right pane shipped with, and a single point is a place that hides it. `miss=` is
    /// the **first** start that came away with nothing and what the sweep saw while it did: a line reporting only its
    /// last try is reporting the one that worked.
    ///
    /// `extra` is whatever else that verb has to say, and it goes in ahead of the numbers so a `must_say` written
    /// against the older shape still reads as one run of words (`card_sweep … hand=true open=true`).
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
    /// Automation: and the numbers behind a sweep that came away with nothing — which field it landed on, how big it
    /// is, and where the two ends of the drag ended up in it. A run that reported only its last try would call the
    /// others green.
    function sweptTrace() {
        const f = pad.field
        return f ? Math.round(f.width) + "x" + Math.round(f.height) + ",a" + f.grabAnchor + ",n" + f.text.length : "-"
    }
    /// Automation: and that there is a hand at all. A run enters the three functions above —
    /// a pointer cannot be injected — so a pad whose `MouseArea` had been taken out, disabled or shrunk would answer
    /// every sweep it was asked and never see a press. This is the half of the wiring a sweep cannot say for itself.
    readonly property bool handStands: hand.enabled && hand.width === pad.width && hand.height === pad.height

    MouseArea {
        id: hand
        anchors.fill: parent
        // Hover is theirs: the card's own words and rows report the pointer for the card that has to know when the
        // hand has left it, and a hovering area under them would take it (app-ui.md §HoverHandler は下の hover を殺す
        // — a plain area does not, unless it asks for hover).
        hoverEnabled: false
        preventStealing: true
        cursorShape: Qt.IBeamCursor
        onPressed: mouse => { mouse.accepted = pad.pressAt(mouse.x, mouse.y) }
        onPositionChanged: mouse => { if (hand.pressed) pad.moveAt(mouse.x, mouse.y) }
        onReleased: pad.releaseNow()
        onCanceled: pad.releaseNow()
    }
}
