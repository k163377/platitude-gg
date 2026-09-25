import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

/// Horizontal scroll of the lanes when the full graph is wider than its column. Laid over the last row, so it is the
/// one bar that comes and goes: out only while the hand is in the pane or dragging it (デザイン規約 §グラフを横へ送る).
///
/// The bar and nothing else: "hand in the pane" stays the pane's answer — a `HoverHandler` on a wrapper over the list
/// takes the hover from every row underneath.
AutoScrollBar {
    id: laneBar

    /// `GraphColumnMetrics`; `graphX` and `laneBarRoom` are written back here.
    required property var columns
    /// Whether the pointer is in the pane — handed in, since this bar has no flickable to ask (`AutoScrollBar.inArea`).
    required property bool pointerInside

    inArea: laneBar.pointerInside
    visible: laneBar.columns.graphXMax > 0 && (laneBar.pointerInside || pressed)
    // Unattached, Fusion would draw the handle only while the pointer is on the bar itself; `visible` decides instead.
    policy: ScrollBar.AlwaysOn
    orientation: Qt.Horizontal
    x: laneBar.columns.labelW
    width: laneBar.columns.graphColW
    size: laneBar.columns.graphFullW > 0 ? laneBar.columns.graphColW / laneBar.columns.graphFullW : 1
    position: laneBar.columns.graphFullW > 0 ? laneBar.columns.graphX / laneBar.columns.graphFullW : 0
    onPositionChanged: {
        // Unattached, its own position is what says the lanes were sent, by whichever hand.
        laneBar.moved()
        if (!pressed)
            return
        laneBar.columns.graphX = Math.max(0, Math.min(position * laneBar.columns.graphFullW,
                                                      laneBar.columns.graphXMax))
    }
    /// The bar's height, for the list's run-out (`GraphColumnMetrics.laneBarRoom`).
    Binding {
        target: laneBar.columns
        property: "laneBarRoom"
        value: laneBar.height
    }
}
