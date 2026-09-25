import QtQuick
import platitude.ui

/// The fill every held control paints (デザイン規約 §長押し). Only the paint; the state machine is `HoldDriver`.
Rectangle {
    id: holdFill

    /// `HoldDriver.progress`.
    required property real progress
    /// The warning colour the control wears.
    required property color tone
    /// The border's width where the control has a frame, so the frame stays visible while it fills; 0 fills edge to
    /// edge (menu rows).
    property real inset: 0

    anchors.left: parent.left
    anchors.top: parent.top
    anchors.bottom: parent.bottom
    anchors.margins: holdFill.inset
    // Floored at `holdFillMin`: proportional from zero, the first frames are a sub-pixel sliver (デザイン規約 §長押し).
    width: holdFill.progress > 0
           ? Math.max(Metrics.holdFillMin, (parent.width - 2 * holdFill.inset) * holdFill.progress)
           : 0
    radius: Theme.radiusSm
    color: holdFill.tone
    visible: holdFill.progress > 0
}
