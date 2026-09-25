pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The chip column of one commit: the front card (`RefChip`) and behind it one sheet per colour the row carries — the
// front card's own colour may take a second, saying "and more of these" — while the card's `+N` counts the names
// (デザイン規約 §重ね表示). The fan goes down and right within the column's width, pushing the front card left; the
// card the chip unfolds into grows the same way over it (§グラフ行のダブルクリック).
Item {
    id: stack

    /// The row's records, in the order core sorted them: the current branch (or the detached HEAD marker) first, then
    /// locals, remotes and tags (`build_label_map`). The front card is the first of them.
    property var records: []
    /// How wide the column leaves this: the front card is cut to what is left of it after the fan.
    property real maxWidth: Metrics.labelColW
    /// The front card has taken a second click and is waiting out the double-click window (`RefChip.waiting`).
    property bool waiting: false
    /// The unfolded card stands over this, so the sheets are not drawn: they would peek out from under it.
    property bool unstacked: false

    /// The front card. The unfolded list is placed against the stack, not this: the stack is what it covers.
    readonly property alias chipItem: frontChip
    /// The mark as the card is actually wearing it, for the run that photographs the wait (`RefChip.waiting`).
    readonly property alias chipWaiting: frontChip.waiting

    /// How far each sheet stands right of the one in front: the card's half gap, so what shows is a card's edge.
    readonly property real step: Theme.spaceXs / 2
    /// How far each sheet falls: the steepest whole step the row's sheet count fits, one rate for the whole row,
    /// between a border and [`step`] (デザイン規約 §重ね表示).
    readonly property real drop: {
        if (stack.sheets.length === 0)
            return stack.step
        const held = Math.floor(stack.fanDepthMax / stack.sheets.length)
        return Math.max(Theme.borderWidth, Math.min(stack.step, held))
    }
    /// The most sheets behind one card: one per colour less the card's own, which the repeat takes back (`sheets`).
    readonly property int maxSheets: frontChip.kindKeys.length - 1
    /// The room on one side of the card: half of what the row is taller than a chip.
    readonly property real fanRoom: (Theme.graphRowHeight - frontChip.height) / 2
    /// How deep the fan may go: both sides' room, since `lift` centres card and fan together, less a border either
    /// side to keep the deepest sheet off the neighbouring rows' ink.
    readonly property real fanDepthMax: 2 * stack.fanRoom - 2 * Theme.borderWidth

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
        const out = seen[order[0]] > 1 ? [order[0]] : []
        for (let j = 1; j < order.length; ++j)
            out.push(order[j])
        return out.slice(0, stack.maxSheets)
    }
    /// Where each sheet stands off the front card's corner: one step and one drop apiece, the repeat sheet included.
    readonly property var layout: {
        const out = []
        for (let i = 0; i < stack.sheets.length; ++i)
            out.push({ key: stack.sheets[i], x: (i + 1) * stack.step, y: (i + 1) * stack.drop })
        return out
    }
    readonly property real fanW: stack.sheets.length * stack.step
    readonly property real fanDepth: stack.sheets.length * stack.drop
    /// The widest fan, which the column's floor keeps back (`GraphColumnMetrics.chipFurnitureW`).
    readonly property real fanMaxW: stack.maxSheets * stack.step
    /// How far the stack rises off the row's middle, so the row centres card and fan together, the odd pixel above
    /// (デザイン規約 §重ね表示).
    readonly property real lift: Math.floor(stack.fanDepth / 2)

    implicitWidth: frontChip.width + stack.fanW
    implicitHeight: frontChip.height
    width: implicitWidth
    height: implicitHeight

    // Each sheet draws only the L it shows (right strip, floor band): a card that is not a tag has no ground, so a
    // whole sheet behind it would show through the name.
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
