import QtQuick
import platitude.ui

// The remote badge a branch wears when the upstream it is measured against is one git cannot reach — git's own
// `[gone]`. The mark keeps its shape and takes the state instead of going: the far side deleting the ref leaves no
// remote-tracking ref for the badge's ordinary reason, so a row that answered by dropping it would read as a branch
// that never tracked anything (デザイン規約 §ref の種別).
//
// **Both the row and the stand-in for the current branch draw it from here**, so the corner the `!` sits in and the
// size it is drawn at are one decision — the reading `RefusalBadge` is kept as a part for. Drawn on the second of the
// two by hand once, the mark came out on top of the badge and the pair read as one shape nobody can name.
Item {
    id: badge

    /// Which mark the badge itself is — the pull request's, or the cloud every other remote-bearing row wears.
    property bool pullRequest: false
    /// Whether the upstream is gone: the mark takes the state, and the `!` comes out beside it.
    property bool gone: false
    /// The row's own height, which is the air this has over it to lift the mark into (see `y` below).
    property real rowHeight: Theme.rowHeight

    // The size the graph's chips wear the same badge at: one question, one mark, one size (デザイン規約 §寸法).
    implicitWidth: Theme.iconSm
    implicitHeight: Theme.iconSm

    NavIcon {
        // **Its own size, never the seat's.** A canvas born without one is never asked to paint when it is given
        // one later, and the debt it owes the picture is never paid — the shot waits it out and the run dies at the
        // ceiling (measured: the stand-in's badge, whose seat is a layout that sizes it a pass after it is built).
        width: Theme.iconSm
        height: Theme.iconSm
        kind: badge.pullRequest ? "pr" : "remote"
        // One slot, one colour: which of the two marks it is answers the question, so the colour is free to be the
        // quiet one every row mark wears (デザイン規約 §ref の種別) — until the reading it stands for is not there,
        // which is a state (§状態).
        tint: badge.gone ? Theme.warning : Theme.textSecondary
    }
    // The state, on the corner the mark leaves empty — the same `!` a folded button wears in the same place
    // (`ActionButtonSeat.cornerAlert`). **A size down**: over a badge of the badge's own size a mark at the badge's
    // size covers it (measured).
    NavIcon {
        id: goneMark
        visible: badge.gone
        kind: "bang"
        tint: Theme.warning
        width: Theme.iconXs
        height: Theme.iconXs
        // **Seated by its ink, not by its box**: this mark's ink is narrow and sits to the left of the box it is
        // drawn in (`NavIcon.inkGrid`), so a box set against the badge's right edge puts the stroke back on top of
        // the badge. The ink begins where the badge's box ends, and the box overhangs into the panel's gutter — air
        // the row already keeps (デザイン規約 §余白).
        x: Theme.iconSm - (goneMark.inkRight - goneMark.inkWidth)
        // As high as the row's own air over the badge goes: any further and the mark is drawn on the row above,
        // which nothing here clips.
        y: -(badge.rowHeight - Theme.iconSm) / 2
    }
}
