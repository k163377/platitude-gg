import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

/// The bar a pane owns: a slab of ink held against the pane's inner edge, opaque, and running the whole height of the
/// view it belongs to (デザイン規約 §QML 実装ルール の摘みの項). The left panel's lists and the right panel's — its two file
/// lists and the block the message boxes sit in — all wear it.
///
/// The style's own bar stays where the reader is over the words themselves: the graph, the diff, and the text boxes
/// inside the right panel. **A box being typed into keeps the style's bar** — it is the one place a bar is inside the
/// content rather than at a pane's edge.
///
/// **Its three states climb the palette from the ground it stands on**: `bgElevated` idle over a pane, the one
/// step there is between a pane's ground and the lines it divides itself with; `borderDefault` while the view is being
/// sent, the ink a frame is drawn in; `borderStrong` under a held thumb. The style's see-through bar lands on the same
/// three over a pane's ground by carrying that much of its one ink, so the two families read alike by two routes — and
/// this one, opaque on purpose (below), cannot say a step by carrying less, so it names the colours (`dimsItself`).
/// **Only the idle step is the ground's to move** (`idleColor`): the other two are frames, and a frame is the same
/// ink wherever it is drawn.
///
/// Three things it does not take from the style's bar:
///
///  - **it does not float.** The style's handle stops a step short of the edge, which leaves it reading as part of the
///    content it is drawn over rather than as the pane's own edge.
///  - **it is not see-through.** The colour the eye reads through a translucent thumb is a blend with whatever passes
///    underneath, so it changes as the view scrolls — measured over the pane's ground, over a selected row and over
///    body text, one thumb read as three colours. The gutter already keeps ink from under this bar, so there is
///    nothing to read through it, and what it paints is a named colour, flat.
///  - **it does not stop short of the ends.** The style keeps a step of padding at both ends of the track, so a view
///    scrolled hard against its top frame left the slab hanging a step below it. Only the far
///    side keeps its step, where it is grabbing room rather than a gap: the box stays wider than the ink.
AutoScrollBar {
    id: paneBar

    /// The idle step, which is **the one step above whatever ground this bar stands on** — the rest of the climb
    /// (`borderDefault`, `borderStrong`) is the same wherever it stands. A pane's ground is `bgSurface`, so the
    /// default is the step above that; the settings screen's ground is `bgElevated` itself, where this colour would
    /// be the ground exactly and the resting bar would not be there at all (実測: 辺の 5px が `#0F172A`). That screen
    /// hands in the next step up instead (デザイン規約 §ペインのスクロールバー).
    property color idleColor: Theme.bgElevated

    // Named colours, not a share of one (above).
    dimsItself: false

    /// What the slab is painting, for a run to read back — the painted side, so a cut binding cannot read as green
    /// (verify-ui).
    readonly property alias slabColor: slab.color

    rightPadding: 0
    topPadding: 0
    bottomPadding: 0
    contentItem: Rectangle {
        id: slab
        implicitWidth: Theme.navBarReach
        implicitHeight: Theme.navBarReach
        // Round on the free side only: a slab rounded on all four corners reads as floating over the pane rather than
        // as belonging to its edge, and the ends have to meet the frames square to sit against them.
        radius: Theme.radiusSm
        topRightRadius: 0
        bottomRightRadius: 0
        // Idle a step under the ramp, and on it while the view is being sent. Up at once, down over 400ms — the two
        // halves every bar in the window shares (規約 §QML 実装ルール のバーの明るさ).
        color: paneBar.idleColor
        states: [
            State { name: "gone"; when: !paneBar.visible },
            State {
                name: "lit"
                when: paneBar.bright
                PropertyChanges {
                    slab.color: paneBar.pressed ? Theme.borderStrong : Theme.borderDefault
                }
            }
        ]
        transitions: Transition {
            from: "lit"
            to: ""
            ColorAnimation { duration: 400 }
        }
    }
}
