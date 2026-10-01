pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The graph pane's column arithmetic: what each of the three columns may be and is, and how far the lanes are sent
// sideways. Draws nothing; the pane, rows, dividers and find bar read their geometry here.
QtObject {
    id: metrics

    /// The width the three columns are laid out in.
    required property real paneW
    /// The widest the graph gets, in lanes (`GraphModel.maxLanes`).
    required property int maxLanes

    // Widths a divider drag wrote (labels / graph); -1 = automatic.
    property real labelWManual: -1
    property real graphColWManual: -1
    /// A chip name's font (`RefChip`), the family named: nothing here has a window to inherit it from.
    readonly property FontMetrics chipFont: FontMetrics {
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontChip
    }
    /// The cut mark's advance. A `TextMetrics`, whose `advanceWidth` is a property: `FontMetrics.advanceWidth()` is a
    /// method, so a binding reads it once, on the default font.
    readonly property TextMetrics chipCutInk: TextMetrics {
        font: metrics.chipFont.font
        text: "…"
    }
    /// What one kept character costs: a measured `n`, as the tab titles' floor (`TabMetrics.titleMinW`). Not
    /// `averageCharacterWidth` (rules-refs/app-ui.md「字数の床の値付けは実測の字送り」).
    readonly property TextMetrics chipLetterInk: TextMetrics {
        font: metrics.chipFont.font
        text: "n"
    }
    /// How many leading characters, plus the cut mark, the chip column keeps at its narrowest — counted in characters
    /// like a tab's (規約 §ウィンドウの縁).
    readonly property int labelMinChars: 3
    readonly property real chipNameMinW: Math.ceil(chipCutInk.advanceWidth
        + labelMinChars * chipLetterInk.advanceWidth)
    /// The locked-worktree mark, only to be priced: a seat is the mark's ink (規約 §余白), which only the icon knows
    /// (`NavIcon.inkGrid`). Never drawn — `inkWidth` is arithmetic and answers with no scene around it.
    readonly property NavIcon chipLockMark: NavIcon {
        kind: "lock"
        width: Theme.iconSm
        height: Theme.iconSm
    }
    /// The badge at the chip's other end, priced the same way. The cloud: it shares one slot with the PR mark
    /// (`RefChip.hasBadge`) and is the wider of the two.
    readonly property NavIcon chipBadgeMark: NavIcon {
        kind: "remote"
        width: Theme.iconSm
        height: Theme.iconSm
    }
    /// The fan of sheets behind the card, asked of the stack itself (its own arithmetic). Draws nothing.
    readonly property RefChipStack chipFan: RefChipStack {}
    /// The `+N` count (`RefChip`) at the widest the floor prices it: two digits, where real repositories stop. Measured
    /// off a `Label` — metrics come out a few pixels tight, and a tight floor still elides
    /// (`GraphRowChips.nameBoxMinW`). Draws nothing; the family is named as in `chipFont`. A deeper count is not
    /// crushed: the card measures its own (`RefChip.countW`), and the extra digit comes out of the name.
    readonly property Label chipCountInk: Label {
        text: "+99"
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontSm
    }
    /// Everything a chip spends on what is not the name: fan, count, badge and padlock, each with its half gap
    /// (`RefChip`'s row spacing), plus the frame's padding either side. Every mark counts though it comes and goes —
    /// the column must fit the worst-dressed chip (rules-refs/app-ui.md「チップ列の狭める床」). The tree mark shares the
    /// padlock's seat (`RefChip.hasTree`) and is narrower, so it is not a term.
    readonly property real chipFurnitureW: chipFan.fanMaxW
        + chipCountInk.implicitWidth + Theme.spaceXs / 2
        + chipBadgeMark.inkWidth + Theme.spaceXs / 2
        + chipLockMark.inkWidth + Theme.spaceXs / 2
        + 2 * Theme.spaceXs
    // The chip column's floor — name, furniture, and the `spaceSm` `GraphRowChips` keeps in front of the chip; move
    // any of those and this moves too — and its ceiling, where the next two columns can still say something
    // (規約 §グラフ列は最も広い所のレーンまで).
    readonly property real labelColWMin: chipNameMinW + chipFurnitureW + Theme.spaceSm
    readonly property real labelColWMax: Math.max(labelColWMin, paneW - graphColWMin - Metrics.messageMinW)
    // The automatic width does not follow the pane: the lanes give instead (`graphColWMax`), and the window's floor
    // reserves this column (規約 §窓の床). A column that followed would crush chips and jump when the left menu folds.
    // Only a dragged width is clamped, so the message column survives a later narrowing.
    //
    // Rounded here and in `graphColW`: the rows' ticks snap to whole pixels (RowLayout) while the hover lines read
    // these raw, so on a fractional width tick and line miss each other.
    readonly property real labelW: Math.round(labelWManual >= 0
        ? Math.max(labelColWMin, Math.min(labelWManual, labelColWMax))
        : Metrics.labelColW)
    // The narrowest the pane lays out with all three columns saying something; the page's floor is built on it.
    readonly property real contentMinW: Metrics.labelColW + graphColWMin + Metrics.messageMinW
    readonly property real graphFullW: Metrics.laneInset + Math.max(1, maxLanes) * Metrics.laneW + Theme.spaceSm
    // The floor: the message tick against lane 0's co-author badge, the widest ink on that lane. Mirrors
    // GraphLaneCell's badge geometry — move one, move both. The tick stands `spaceSm` past the column's edge, where the
    // ink's clipper also reaches (the dissolve's far end), so that much comes off.
    readonly property real graphColWMin: Math.ceil(Metrics.laneInset + Metrics.laneW / 2
        + Metrics.nodeIcon / 2 - 2 * Theme.borderWidth + (Theme.iconSm + Theme.borderWidth) / 2)
        - Theme.spaceSm
    // The ceiling: the lanes' full width, or earlier where the message column would vanish
    // (規約 §グラフ列は最も広い所のレーンまで).
    readonly property real graphColWMax: Math.max(graphColWMin,
        Math.min(graphFullW, paneW - labelW - Metrics.messageMinW))
    /// Nothing but the tail gap between ceiling and floor, so a drag would do nothing: the divider drops its line and
    /// turns the cursor away (規約 §グラフ列は最も広い所のレーンまで). A one-lane history still has a drag.
    readonly property bool graphColWFixed: graphColWMax <= graphColWMin + Theme.spaceSm
    // Rounded for the same one-grid reason as `labelW`.
    readonly property real graphColW: Math.round(Math.min(graphColWMax,
        graphColWManual >= 0 ? Math.max(graphColWManual, graphColWMin)
                             : Metrics.laneInset + Metrics.graphDefaultLanes * Metrics.laneW + Theme.spaceSm))
    property real graphX: 0
    readonly property real graphXMax: Math.max(0, graphFullW - graphColW)
    onGraphXMaxChanged: graphX = Math.min(graphX, graphXMax)
    /// The lane bar's height, written by the bar (`GraphLaneBar`). The list adds it to its run-out while the lanes
    /// overflow, or the bar covers the oldest row for good (`GraphList.bottomMargin`, デザイン規約 §グラフを横へ送る).
    property real laneBarRoom: 0
    /// Where a subject's first character sits. Must match GraphRowDelegate's third column, which lays out the same
    /// steps; the find bar's cap is measured from here.
    readonly property real subjectTextX: labelW + graphColW + Theme.spaceSm + 2 * Theme.borderWidth + Theme.spaceXs
}
