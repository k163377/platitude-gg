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
    /// The pixel size of the name the badge follows, and where its baseline runs in the badge's y: a state after the
    /// name, drawn to its letters and on their capitals' middle (デザイン規約 §アイコンの席 の名指し).
    property real size: Theme.fontMd
    property real wordBase: 0
    /// The row-end column's seat: the seat of the name's letters (デザイン規約 §アイコンの席 の名指し), one width down
    /// the rows, the mark in its middle.
    readonly property real seat: MarkSeat.cell(badge.size)

    implicitWidth: badge.seat
    implicitHeight: Theme.iconSm

    NavIcon {
        id: badgeMark
        // Its own size, never the layout's: a canvas born sizeless is never asked to paint once a layout sizes it, and
        // a shot waiting on it runs to the ceiling.
        width: badge.seat
        height: Theme.iconSm
        set: "name"
        size: badge.size
        wordBase: badge.wordBase
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
        // Its ink against the mark's ink, not boxes: the `!` is a sliver in the middle of its box, and the seat holds
        // air either side of the mark. The box overhangs into the gutter the row already keeps (デザイン規約 §余白).
        x: badgeMark.x + badgeMark.inkRight - goneMark.inkLeft
        // Up to the row's own air over the badge — any higher draws on the row above, and nothing clips it.
        y: -(badge.rowHeight - Theme.iconSm) / 2
    }
}
