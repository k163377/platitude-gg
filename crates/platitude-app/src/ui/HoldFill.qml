import QtQuick
import platitude.ui

/// The report every held control in this app gives (デザイン規約 §長押し): a band filling the control from the left for as
/// long as the press lasts, and reaching the far end at the moment it fires. The owner keeps the gesture — this is only
/// the paint, so the fills in the app cannot drift apart the way the state machine behind them cannot (`HoldDriver`).
Rectangle {
    id: holdFill

    /// How far into the hold the press has got, 0 to 1 — `HoldDriver.progress`.
    required property real progress
    /// What the hold costs, in the colour the control wears to say so.
    required property color tone
    /// How far in from the control's own edges the band starts: the border's width where the control has a frame, so
    /// the frame stays a frame while it fills. Zero — the default — fills edge to edge, the way a held menu row does.
    property real inset: 0

    anchors.left: parent.left
    anchors.top: parent.top
    anchors.bottom: parent.bottom
    anchors.margins: holdFill.inset
    // At least `holdFillMin` wide while it runs: proportional from zero, the first tenth of the hold is a
    // sub-pixel sliver, so the press reads as not having taken and the whole gesture feels longer than it is
    // (デザイン規約 §進行中・長押しの定数).
    width: holdFill.progress > 0
           ? Math.max(Metrics.holdFillMin, (parent.width - 2 * holdFill.inset) * holdFill.progress)
           : 0
    radius: Theme.radiusSm
    color: holdFill.tone
    visible: holdFill.progress > 0
}
