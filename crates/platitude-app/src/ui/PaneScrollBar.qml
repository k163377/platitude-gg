import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

/// The bar a pane owns: a slab of ink held against the pane's inner edge, opaque, and running the whole height of the
/// view it belongs to (デザイン規約 §QML 実装ルール の摘みの項). The left panel's lists and the right panel's — its two file
/// lists and the block the message boxes sit in — all wear it.
///
/// The style's own bar stays where the reader is over the words themselves: the graph, the diff, and the text boxes
/// inside the right panel. **A box being typed into keeps the style's bar** — it is the one place a bar is inside the
/// content rather than at a pane's edge (2026-08-27 ユーザー指示).
///
/// Three things it does not take from the style's bar:
///
///  - **it does not float.** The style's handle stops a step short of the edge, which leaves it reading as part of the
///    content it is drawn over rather than as the pane's own edge.
///  - **it is not see-through.** At 0.75 opacity the colour the eye reads is a blend with whatever passes underneath,
///    and it moves as the view scrolls (qmltestrunner 実測: `#273346` over the pane's ground, `#2C3A55` over a selected
///    row, `#5F6B7C` over body text). The gutter already keeps ink from under the bar, so there is nothing to read
///    through it — the blend over the pane's ground is written down as `Theme.navBarInk` and painted flat. Both panels
///    stand on `Theme.bgSurface`, so that one value is the colour both were already showing.
///  - **it does not stop short of the ends.** The style keeps a step of padding at both ends of the track, so a view
///    scrolled hard against its top frame left the slab hanging a step below it (2026-08-27 ユーザー報告). Only the far
///    side keeps its step, where it is grabbing room rather than a gap: the box stays wider than the ink.
AutoScrollBar {
    id: paneBar

    rightPadding: 0
    topPadding: 0
    bottomPadding: 0
    contentItem: Rectangle {
        implicitWidth: Theme.navBarReach
        implicitHeight: Theme.navBarReach
        // Round on the free side only: a slab rounded on all four corners reads as floating over the pane rather than
        // as belonging to its edge, and the ends have to meet the frames square to sit against them.
        radius: Theme.radiusSm
        topRightRadius: 0
        bottomRightRadius: 0
        color: paneBar.pressed ? Theme.navBarInkHeld : Theme.navBarInk
        // Flat, and always: the bar itself is what comes and goes, and it does that without the style's fade
        // (`AutoScrollBar.visible`).
    }
}
