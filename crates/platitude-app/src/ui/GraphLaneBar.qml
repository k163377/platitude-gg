import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

/// Horizontal scroll of the lanes when the full graph is wider than its column.
///
/// It lies over the lanes of the last row, so it comes out only while the hand is in the pane, and stays out for as long
/// as it is being dragged, wherever that has taken the pointer (デザイン規約 §グラフを横へ送る).
///
/// **The bar and nothing else.** Whether the hand is in the pane is the pane's own answer and has to stay there: a
/// `HoverHandler` put in a wrapper item over the list **takes the hover away from every row underneath** — hover goes to
/// the topmost item that accepts it, and an item carrying a handler accepts it for its whole area (2026-08-22
/// qmltestrunner で実測, after a report of rows that would not light and cards that would not close).
ScrollBar {
    id: laneBar

    /// The lane column's arithmetic (`GraphColumnMetrics`): where it starts, how wide it is drawn, how wide the whole
    /// graph is, and how far it has been sent. The last one is written back here.
    required property var columns
    /// Whether the pointer is anywhere in the pane — the pane's answer, handed in.
    required property bool pointerInside

    visible: laneBar.columns.graphXMax > 0 && (laneBar.pointerInside || pressed)
    // Fusion draws its handle only in the style's "active" state, which for a bar that is not attached to a Flickable
    // means while the pointer is on the bar itself — a 6px strip on the pane's bottom edge that nobody would find. When
    // the bar is out it is because this pane put it there, so the style stops deciding.
    policy: ScrollBar.AlwaysOn
    orientation: Qt.Horizontal
    x: laneBar.columns.labelW
    width: laneBar.columns.graphColW
    size: laneBar.columns.graphFullW > 0 ? laneBar.columns.graphColW / laneBar.columns.graphFullW : 1
    position: laneBar.columns.graphFullW > 0 ? laneBar.columns.graphX / laneBar.columns.graphFullW : 0
    onPositionChanged: {
        if (!pressed)
            return
        laneBar.columns.graphX = Math.max(0, Math.min(position * laneBar.columns.graphFullW,
                                                      laneBar.columns.graphXMax))
    }
}
