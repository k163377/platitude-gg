pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The chip column of one commit: the front card with the first name on it (`RefChip`), and behind it one sheet per
// **colour** the row carries — the whole of what the row says about the names it is not showing.
//
// **One sheet per colour, not per name.** A colour is what the chip has to say in the first
// place (デザイン規約 §ref の種別: 枠 = 種別), and counting names instead said the same thing over and over: measured on
// `JetBrains/kotlin`, 4,351 commits carry more than one ref and 3,650 of them are one commit wearing several tags, so
// five sixths of what the old `+N` counted was the colour already on the front card. **The one colour that is allowed
// twice is the front card's own** — a second sheet right behind it says "and more of these", which is the whole of
// what a count was worth here; the sheets further back are one apiece. Everything the row holds is still read whole in
// the card the chip unfolds into.
//
// **The fan goes down and to the right, and the stack is what the column holds** — the deepest sheet's right edge
// stands where a bare chip's would, and the front card is pushed left by as many sheets as there are — a name's left
// edge moving row to row is the price of it. The card the chip unfolds into opens at the front card's top-left and
// grows the same way, so the ground the fan is on is ground that card takes (§グラフ行のダブルクリック).
Item {
    id: stack

    /// The row's records, in the order core sorted them: the current branch (or the detached HEAD marker) first, then
    /// locals, remotes and tags (`build_label_map`). The front card is the first of them.
    property var records: []
    /// How wide the column leaves this: the front card is cut to what is left of it after the fan.
    property real maxWidth: Metrics.labelColW
    /// The front card has taken a second click and is waiting out the double-click window (`RefChip.waiting`).
    property bool waiting: false
    /// The card this chip unfolds into is standing on it, so the sheets are not drawn: the card takes the stack's
    /// place, and one left peeking out from under it is a stack the card did not stand in for.
    property bool unstacked: false

    /// The front card itself. **What owners anchor the card by, and what the unfolded list is placed against.**
    readonly property alias chipItem: frontChip
    /// The mark as the card is actually wearing it, for the run that photographs the wait (`RefChip.waiting`).
    readonly property alias chipWaiting: frontChip.waiting

    /// How far each sheet stands right of the one in front of it, and the gap that goes in when the two are the same
    /// colour — without it a sheet of the card's own colour reads as the card having grown, not as a second one.
    readonly property real step: Theme.spaceXs
    readonly property real sameGap: Theme.borderWidth
    /// The room the fan has under the card before the row's own floor: half of what the row is taller than a chip.
    readonly property real fanRoom: (Theme.graphRowHeight - frontChip.height) / 2
    /// The two slopes a sheet can take, and how far the card may rise to make room for them.
    ///
    /// **The slope flattens as the stack grows, and the card starts rising with it.** The first sheets take the half
    /// gap the card's own contents are spaced by; once the fan is deeper than the room under the card plus a single
    /// lift, the ones behind take a border's worth instead, and the card comes up by whatever is still over.
    /// **The card keeps a border's worth of the row above it** — the fan below reaches the
    /// row's own floor, so a card that rose to the ceiling would meet the row above's deepest sheet with nothing in
    /// between.
    readonly property real steepStep: Theme.spaceXs / 2
    readonly property real shallowStep: Theme.borderWidth
    readonly property real maxLift: stack.fanRoom - Theme.borderWidth
    /// How many sheets can ever stand behind one card: one per colour, less the card's own — the colour that repeats
    /// takes that one back (`sheets`). Read off the list of colours rather than written down, so a kind added to
    /// `RefChip.kindKeyOf` is counted here without anybody remembering to.
    readonly property int maxSheets: frontChip.kindKeys.length - 1

    /// One key per sheet behind the front card, in the row's own order: every colour on the row once, and the front
    /// card's own a second time when the row carries more than one of it.
    readonly property var sheets: {
        if (stack.unstacked)
            return []
        const order = []
        const seen = ({})
        for (let i = 0; i < stack.records.length; ++i) {
            const key = frontChip.kindKeyOf(stack.records[i])
            if (seen[key] === undefined) {
                seen[key] = 1
                order.push(key)
            } else {
                seen[key] += 1
            }
        }
        if (order.length === 0)
            return []
        // The front card is the first of them; it keeps a sheet of its own only where the row repeats its colour.
        const out = seen[order[0]] > 1 ? [order[0]] : []
        for (let j = 1; j < order.length; ++j)
            out.push(order[j])
        return out.slice(0, stack.maxSheets)
    }
    /// Where each sheet stands, in whole pixels off the front card's own corner, and how wide the fan comes out.
    /// **One pass**: the offsets accumulate, because the gap a same-coloured sheet takes moves every sheet behind it.
    readonly property var layout: {
        const out = []
        let x = 0
        let y = 0
        let front = stack.records.length > 0 ? frontChip.kindKeyOf(stack.records[0]) : ""
        for (let i = 0; i < stack.sheets.length; ++i) {
            const key = stack.sheets[i]
            const gap = key === front ? stack.sameGap : 0
            const steep = y + gap + stack.steepStep <= stack.fanRoom + stack.steepStep
            const down = (steep ? stack.steepStep : stack.shallowStep) + gap
            out.push({ key: key, down: down, gap: gap, prevX: x, prevY: y,
                       x: x + stack.step + gap, y: y + down })
            x += stack.step + gap
            y += down
            front = key
        }
        return out
    }
    /// How many gaps the fan is carrying, for the width it comes to.
    readonly property int gaps: {
        let n = 0
        let front = stack.records.length > 0 ? frontChip.kindKeyOf(stack.records[0]) : ""
        for (let i = 0; i < stack.sheets.length; ++i) {
            if (stack.sheets[i] === front)
                n += 1
            front = stack.sheets[i]
        }
        return n
    }
    /// What the fan spends beside the card, and how deep it goes.
    readonly property real fanW: stack.sheets.length * stack.step + stack.gaps
    readonly property real fanDepth: stack.layout.length > 0 ? stack.layout[stack.layout.length - 1].y : 0
    /// What the widest-dressed chip could ever spend, which is what the column's floor keeps back for it
    /// (`GraphColumnMetrics.chipFurnitureW`). One gap: only the sheet against the front card can share its colour.
    readonly property real fanMaxW: stack.maxSheets * stack.step + stack.sameGap
    /// How far the whole stack is lifted to keep the fan off the row below. **Only the deep ones lift**: a card sits
    /// where a bare one would until the fan has used up the room under it, so an ordinary row's name stands exactly
    /// where its neighbours' do, and the rise starts with the third sheet.
    readonly property real lift: Math.min(Math.max(0, stack.fanDepth - stack.fanRoom), stack.maxLift)

    implicitWidth: frontChip.width + stack.fanW
    implicitHeight: frontChip.height
    width: implicitWidth
    height: implicitHeight

    // **Each sheet is drawn as the L it actually shows**, and nothing else: the strip down its right edge and the band
    // along its floor that the card in front does not cover. A card that is not a tag has no ground of its own, so a
    // whole sheet left standing behind one shows straight through the name.
    //
    // **A sheet is the card it stands for, cut down to its edge** — the same frame and the same ground that kind wears
    // when it is the one in front. **Not the kind's colour poured into the exposed four pixels**: a solid block reads
    // as neither a frame nor a card beside an outlined one (tried, and taken back).
    Repeater {
        model: stack.layout
        Item {
            id: sheet
            required property var modelData
            readonly property color ink: frontChip.kindColourFor(sheet.modelData.key)
            readonly property color ground: frontChip.kindGroundFor(sheet.modelData.key)
            visible: frontChip.visible

            // The strip down the right edge, from where the card in front stops (plus its gap) to the sheet's own edge.
            Item {
                x: sheet.modelData.prevX + frontChip.width + sheet.modelData.gap
                y: sheet.modelData.y
                width: stack.step
                height: frontChip.height
                clip: true
                Rectangle {
                    x: -(frontChip.width - stack.step)
                    width: frontChip.width
                    height: parent.height
                    radius: Theme.radiusSm
                    color: sheet.ground
                    border.width: Theme.borderWidth
                    border.color: sheet.ink
                }
            }
            // The band along the floor, under the part of the sheet the card in front still covers.
            Item {
                id: floorBand
                x: sheet.modelData.x
                y: sheet.modelData.prevY + frontChip.height + sheet.modelData.gap
                width: frontChip.width - stack.step
                height: sheet.modelData.down - sheet.modelData.gap
                clip: true
                Rectangle {
                    y: -(frontChip.height - floorBand.height)
                    width: frontChip.width
                    height: frontChip.height
                    radius: Theme.radiusSm
                    color: sheet.ground
                    border.width: Theme.borderWidth
                    border.color: sheet.ink
                }
            }
        }
    }
    RefChip {
        id: frontChip
        records: stack.records
        maxWidth: stack.maxWidth - stack.fanW
        waiting: stack.waiting
    }
}
