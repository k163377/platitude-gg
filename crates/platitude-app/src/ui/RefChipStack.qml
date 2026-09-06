pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The chip column of one commit: the front card with the first name on it (`RefChip`), and behind it one sheet per
// **colour** the row carries.
//
// **One sheet per colour, not per name — the card's own `+N` is what counts them** (デザイン規約 §重ね表示). The two say
// different things about the same row and neither can be read off the other: a colour is one sheet however many names
// wear it, so a commit wearing forty tags fans exactly as one wearing two does. Per name, the fan said the same thing
// over and over instead — measured on `JetBrains/kotlin`, 4,351 commits carry more than one ref and 3,650 of them are
// one commit wearing several tags, so five sixths of a per-name fan would have been the colour already on the front
// card. **The one colour that is allowed twice is the front card's own** — a second sheet right behind it says "and
// more of these", and the sheets further back are one apiece. Everything the row holds is still read whole in the card
// the chip unfolds into.
//
// **The fan goes down and to the right, and the stack is what the column holds** — the deepest sheet's right edge
// stands where a bare chip's would, and the front card is pushed left by as many sheets as there are — a name's left
// edge moving row to row is the price of it. The card the chip unfolds into opens at the front card's top-left and
// grows the same way, so the ground the fan is on is ground that card takes (§グラフ行のダブルクリック).
//
// **One step and one drop for every sheet, and the row centres the fan with the card** — no depth has a slope of its
// own and no depth pushes the card out of the middle of its row (デザイン規約 §重ね表示).
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

    /// How far each sheet stands right of the one in front of it — the half gap the card's own contents are spaced by,
    /// so what shows of each sheet is the edge of a card rather than a band of its colour.
    readonly property real step: Theme.spaceXs / 2
    /// And how far it falls: **a border's worth, so the sheets meet**. At a deeper drop the row's own ground shows
    /// between one edge and the next, and the deepest fan no longer fits the room under the card — which costs the
    /// last sheet of the deepest row a slope of its own.
    readonly property real drop: Theme.borderWidth
    /// How many sheets can ever stand behind one card: one per colour, less the card's own — the colour that repeats
    /// takes that one back (`sheets`). Read off the list of colours rather than written down, so a kind added to
    /// `RefChip.kindKeyOf` is counted here without anybody remembering to.
    readonly property int maxSheets: frontChip.kindKeys.length - 1
    /// The room the fan has under the card before the row's own floor: half of what the row is taller than a chip.
    /// **What the deepest fan may not outgrow** — five sheets at a border apiece come to exactly this.
    readonly property real fanRoom: (Theme.graphRowHeight - frontChip.height) / 2

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
    /// Where each sheet stands, in whole pixels off the front card's own corner. **One step and one drop apiece, the
    /// sheet that repeats the card's colour included** — the fan opens at one rate all the way back, so no row has a
    /// slope its neighbours do not.
    readonly property var layout: {
        const out = []
        for (let i = 0; i < stack.sheets.length; ++i)
            out.push({ key: stack.sheets[i], x: (i + 1) * stack.step, y: (i + 1) * stack.drop })
        return out
    }
    /// What the fan spends beside the card, and how deep it goes.
    readonly property real fanW: stack.sheets.length * stack.step
    readonly property real fanDepth: stack.sheets.length * stack.drop
    /// What the widest-dressed chip could ever spend, which is what the column's floor keeps back for it
    /// (`GraphColumnMetrics.chipFurnitureW`).
    readonly property real fanMaxW: stack.maxSheets * stack.step
    /// How far the whole stack stands off the middle of the row. **The row centres the card and the fan together**:
    /// centred on the card alone, a deep row hangs its whole fan into the space under it and the stack reads as
    /// pinned to the floor. **The odd pixel of the row goes above**, the way a chip seats its own ink (§余白).
    ///
    /// A card rises by half of what its own fan is deep, so a name's height moves with the depth of its row — by two
    /// pixels at the deepest, which is what a fan of borders costs.
    readonly property real lift: Math.floor(stack.fanDepth / 2)

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

            // The strip down the right edge, from where the card in front stops to the sheet's own edge.
            Item {
                x: sheet.modelData.x - stack.step + frontChip.width
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
                y: sheet.modelData.y - stack.drop + frontChip.height
                width: frontChip.width - stack.step
                height: stack.drop
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
