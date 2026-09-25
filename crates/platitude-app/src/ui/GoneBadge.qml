import QtQuick
import platitude.ui

// A branch's remote badge, which on git's `[gone]` keeps its shape and takes the state instead of disappearing — a
// dropped badge reads as a branch that never tracked anything (デザイン規約 §ref の種別). Shared by the row and the
// current branch's stand-in, so the `!`'s corner and size are one decision.
Item {
    id: badge

    /// The PR mark instead of the remote cloud.
    property bool pullRequest: false
    property bool gone: false
    /// The row's height — the air over the badge the `!` may lift into (see `y` below).
    property real rowHeight: Theme.rowHeight

    // The size the graph's chips wear it at (デザイン規約 §寸法).
    implicitWidth: Theme.iconSm
    implicitHeight: Theme.iconSm

    NavIcon {
        // Its own size, never the seat's: a canvas born sizeless is never asked to paint once a layout sizes it, and
        // a shot waiting on it runs to the ceiling.
        width: Theme.iconSm
        height: Theme.iconSm
        kind: badge.pullRequest ? "pr" : "remote"
        // The quiet row-mark colour (デザイン規約 §ref の種別); warning when gone, which is a state (§状態).
        tint: badge.gone ? Theme.warning : Theme.textSecondary
    }
    // The `!` in the corner the mark leaves empty, as on a folded button (`ActionButtonSeat.cornerAlert`); a size down,
    // or it covers the badge.
    NavIcon {
        id: goneMark
        visible: badge.gone
        kind: "bang"
        tint: Theme.warning
        width: Theme.iconXs
        height: Theme.iconXs
        // Seated by its ink, not its box: the ink sits left in its box (`NavIcon.inkGrid`), so aligning boxes puts the
        // stroke on the badge. The box overhangs into the gutter the row already keeps (デザイン規約 §余白).
        x: Theme.iconSm - (goneMark.inkRight - goneMark.inkWidth)
        // Up to the row's own air over the badge — any higher draws on the row above, and nothing clips it.
        y: -(badge.rowHeight - Theme.iconSm) / 2
    }
}
