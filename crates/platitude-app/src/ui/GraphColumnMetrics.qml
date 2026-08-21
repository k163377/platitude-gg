pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The graph pane's column arithmetic, in one place: what each of the three columns is allowed to be, what it actually
// is, and how far the lanes have been sent sideways. Nothing here draws — the pane, the rows, the dividers and the find
// bar all read their geometry off this.
QtObject {
    id: metrics

    /// The width the three columns are laid out in.
    required property real paneW
    /// The widest the graph gets, in lanes (`GraphModel.maxLanes`).
    required property int maxLanes

    // Adjustable column widths (labels / graph); -1 = automatic. The graph column starts at a default lane count and
    // scrolls horizontally when the full graph is wider.
    property real labelWManual: -1
    property real graphColWManual: -1
    /// The font a chip writes its name in: `RefChip`'s label takes the window's family (Main) and `fontChip`, and the
    /// floor below is a count of characters, which only the family that draws them can price.
    readonly property FontMetrics chipFont: FontMetrics {
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontChip
    }
    /// The cut mark's own advance. Measured by a TextMetrics rather than asked of `chipFont`: `advanceWidth()` is a
    /// method, so a binding on it takes no dependency, settles once, and settles on the default font — before the
    /// family above has arrived (app-ui.md). `TextMetrics.advanceWidth` is a property, and follows.
    readonly property TextMetrics chipCutInk: TextMetrics {
        font: metrics.chipFont.font
        text: "…"
    }
    /// How much of a name the chip column keeps at its narrowest: the first characters and the mark that says the rest
    /// was cut. **Characters, not pixels**, for the reason the tab titles' floor is one (規約 §ウィンドウの縁) — the same
    /// count costs a different number of pixels in each platform's UI font and at every scaling, so the length comes
    /// out of the font. Three of them, the same count a tab keeps.
    readonly property int labelMinChars: 3
    readonly property real chipNameMinW: Math.ceil(chipCutInk.advanceWidth
        + labelMinChars * chipFont.averageCharacterWidth)
    /// The mark a chip wears when another working copy holds the branch, kept here only to be priced. **A seat is the
    /// mark's ink, not its box** (規約 §余白), and how much of the 16-grid a kind fills is the icon's own knowledge —
    /// `NavIcon.inkGrid` says the caller cannot carry that number — so the width comes off a mark rather than out of
    /// the tokens. It draws nothing: `inkWidth` is arithmetic on the kind, the size and the stroke, and answers the
    /// same 7.5 with no scene around it (2026-08-22 実測 qmltestrunner: no window, no warning).
    readonly property NavIcon chipHeldMark: NavIcon {
        kind: "tree"
        width: Theme.iconSm
        height: Theme.iconSm
    }
    /// That ink's seat, the way `RefChip.heldSeat` takes it: half a gap narrower, the half the mark gives back to the
    /// name beside it. Left fractional rather than rounded up, so the floor reserves exactly what the chip subtracts.
    readonly property real chipHeldSeatW: chipHeldMark.inkWidth - Theme.spaceXs / 2
    /// The badge at the chip's other end, priced the same way and for the same reason. **The cloud, not the pull
    /// request**: the two share one slot (`RefChip.hasBadge`) so only one of them is ever drawn, and the floor takes
    /// the wider — 12.72 of the grid against the PR mark's 10.4.
    readonly property NavIcon chipBadgeMark: NavIcon {
        kind: "remote"
        width: Theme.iconSm
        height: Theme.iconSm
    }
    /// Its seat, the way `RefChip.badgeSeat` takes it.
    readonly property real chipBadgeSeatW: chipBadgeMark.inkWidth - Theme.spaceXs / 2
    /// Everything a chip spends on what is not the name: the `+N` seat, the remote/PR badge and the gap before it, the
    /// held mark and the gap after it, and its own padding on either side. **Each one is a term `RefChip` takes off the
    /// name**, so each one is here, and a floor measured on the bare chip leaves the row that wears them with nothing
    /// but the cut mark (実測).
    ///
    /// **The marks come and go and the floor still counts them all**: a column may not be narrowed to a width that
    /// would crush the chip that turns up in it later. The worst-dressed chip is a branch another working copy holds
    /// that is also on a remote, on a commit some other ref names too — measured at `fontChip` in Yu Gothic UI, the
    /// held mark alone is 9.5 of the 28 the floor keeps for a name, so a floor that leaves it out gives
    /// `feature/topic-a` one character where three were promised (2026-08-22 実測).
    readonly property real chipFurnitureW: Theme.spaceLg + chipBadgeSeatW + 3 * Theme.spaceXs
        + chipHeldSeatW + Theme.spaceXs
    // The narrowest the chip column goes, and the widest. The floor is that much name, that much furniture, and the
    // gap the column keeps in front of the chip (`GraphRowChips` hands it the column less `spaceSm`). **Move any of
    // those and this moves too** — a floor that forgets one draws two characters where it promised three. The far end
    // is where the two columns after it would stop being able to say anything: one lane, and enough message column to
    // show that there is a message (規約 §グラフ列は最も広い所のレーンまで).
    readonly property real labelColWMin: chipNameMinW + chipFurnitureW + Theme.spaceSm
    readonly property real labelColWMax: Math.max(labelColWMin, paneW - graphColWMin - Metrics.messageMinW)
    // The chip column does not give when the pane narrows — the lanes do (`graphColWMax`), and the window's floor holds
    // this one's width in reserve (規約 §窓の床). Squeezing it was tried and taken back out: a column narrower than a chip
    // draws a crushed one, and a column that follows the pane changes width whenever the left menu folds, which is a
    // thing moving on screen that nobody asked to move (2026-08-09 ユーザー報告 — 3 つの症状が全部これだった).
    //
    // A column somebody dragged is still held inside what the pane can lay out: that one is as wide as a hand made it,
    // and the message column has to survive the window being narrowed afterwards.
    //
    // Read back rounded — here and in `graphColW` — because the two kinds of reader sit on different grids: the rows'
    // ticks are laid out by RowLayouts, which snap to whole pixels, while the divider hover lines read these raw. On a
    // fractional width (a drag, or a fractional floor) the tick and the line straddle the same half pixel differently
    // and stop meeting (2026-08-11 報告). Rounding what everyone reads keeps one grid without touching what the drag
    // wrote down.
    readonly property real labelW: Math.round(labelWManual >= 0
        ? Math.max(labelColWMin, Math.min(labelWManual, labelColWMax))
        : Metrics.labelColW)
    // The narrowest this pane can be laid out with all three columns still saying something. The page's floor is built
    // on it (RepoPage), which is what keeps the window from being dragged past it.
    readonly property real contentMinW: Metrics.labelColW + graphColWMin + Metrics.messageMinW
    readonly property real graphFullW: Metrics.laneInset + Math.max(1, maxLanes) * Metrics.laneW + Theme.spaceSm
    // The narrowest the column goes: the message tick brought up against lane 0's co-author badge without touching it.
    // The badge is the widest ink any row puts on that lane, and its geometry — author up-left by a border, badge
    // centre a border inside the node's edge, outline half in half out — mirrors what **GraphLaneCell's canvases**
    // draw. The ceiling lands that edge on a whole pixel; the tick stands `spaceSm` past the column's edge (the subject
    // column's own margin), so that much comes back off the width. The faces are not cut on the way down: their clipper
    // leans the same `spaceSm` past the column (the cell), so the floor is where ink meets ink, not where the clip ran
    // out. One place, so the display, the divider's clamp and the question of whether there is a drag in it at all
    // agree.
    readonly property real graphColWMin: Math.ceil(Metrics.laneInset + Metrics.laneW / 2
        + Metrics.nodeIcon / 2 - 2 * Theme.borderWidth + (Theme.iconSm + Theme.borderWidth) / 2)
        - Theme.spaceSm
    // How far the divider may be pulled: as wide as the lanes ever get, and no wider — a column past the last lane is
    // emptiness taken from the message column. A narrow window stops it earlier still, where the message column would
    // stop showing that a message is there (規約 §グラフ列は最も広い所のレーンまで).
    readonly property real graphColWMax: Math.max(graphColWMin,
        Math.min(graphFullW, paneW - labelW - Metrics.messageMinW))
    /// Whether the column is the only width it can be — nothing left between its ceiling and its floor but the tail
    /// gap. Even a one-lane history keeps a real drag now: its full width holds the gap after the last lane, the floor
    /// tucks the tick against the badge, and the stretch between the two does something. The divider draws no line and
    /// turns the cursor away only when the window has squeezed the ceiling down onto the floor and a drag would come to
    /// nothing (規約 §グラフ列は最も広い所のレーンまで).
    readonly property bool graphColWFixed: graphColWMax <= graphColWMin + Theme.spaceSm
    // Rounded for the same one-grid reason as `labelW`.
    readonly property real graphColW: Math.round(Math.min(graphColWMax,
        graphColWManual >= 0 ? Math.max(graphColWManual, graphColWMin)
                             : Metrics.laneInset + Metrics.graphDefaultLanes * Metrics.laneW + Theme.spaceSm))
    property real graphX: 0
    readonly property real graphXMax: Math.max(0, graphFullW - graphColW)
    onGraphXMaxChanged: graphX = Math.min(graphX, graphXMax)
    /// Where a subject's first character sits — the two columns, then the tick and the gap after it. **Must match
    /// GraphRowDelegate's third column**, whose RowLayout lays out the same three steps; the find bar measures its cap
    /// from here (§コミットを探す).
    readonly property real subjectTextX: labelW + graphColW + Theme.spaceSm + 2 * Theme.borderWidth + Theme.spaceXs
}
