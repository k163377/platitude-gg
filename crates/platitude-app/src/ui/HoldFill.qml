import QtQuick
import platitude.ui

/// The fill every confirmed control paints (デザイン規約 §回数押し). Only the paint; the state machine is `HoldDriver`.
Rectangle {
    id: holdFill

    /// `HoldDriver.progress`.
    required property real progress
    /// The control's `holdTone`: a warning, or the plain ink a white delete names itself by.
    required property color tone
    /// `HoldDriver.dimmed`: full after the last press, so the fill takes the tone's step down — the work is running.
    property bool dimmed: false
    /// The border's width where the control has a frame, so the frame stays visible while it fills; 0 fills edge to
    /// edge (menu rows).
    property real inset: 0
    /// How far past the control's own sides the fill reaches: a menu row fills the card's side padding too, so the
    /// mark at the row's edge has as much fill on its left as on its right (デザイン規約 §回数押し).
    property real bleed: 0

    /// A white delete keeps the plain ink for its words only: the fill carries the warning, as its mark does
    /// (デザイン規約 §回数押しの色).
    readonly property color full: Qt.colorEqual(holdFill.tone, Theme.textPrimary) ? Theme.warning : holdFill.tone
    /// The step down each tone has (デザイン規約 §暗く落とした段).
    readonly property color down: Qt.colorEqual(holdFill.full, Theme.warning) ? Theme.warningDim
                                : Qt.colorEqual(holdFill.tone, Theme.danger) ? Theme.dangerDim
                                : Qt.colorEqual(holdFill.tone, Theme.accent) ? Theme.accentDim
                                : holdFill.tone

    anchors.left: parent.left
    anchors.top: parent.top
    anchors.bottom: parent.bottom
    anchors.margins: holdFill.inset
    anchors.leftMargin: holdFill.inset - holdFill.bleed
    // Floored at `holdFillMin`: proportional from zero, the first frames of a step are a sub-pixel sliver.
    width: holdFill.progress > 0
           ? Math.max(Metrics.holdFillMin,
                      (parent.width - 2 * holdFill.inset + 2 * holdFill.bleed) * Math.min(holdFill.progress, 1))
           : 0
    radius: Theme.radiusSm
    color: holdFill.dimmed ? holdFill.down : holdFill.full
    Behavior on color {
        ColorAnimation {
            duration: Metrics.holdStepMs / 2
        }
    }
    visible: holdFill.progress > 0
}
