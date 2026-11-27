import QtQuick
import platitude.ui

/// The fill every confirmed control paints (デザイン規約 §回数押し). Only the paint; the state machine is `HoldDriver`.
/// Lays itself over its parent, out to `bleed` either side: the presses' fill grows from the left, and where presses
/// were dropped under the hand (`HoldDriver.spoiled`) the red comes over the whole of it from the right, pushing that
/// fill back.
Item {
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
    /// mark at the row's edge has as much fill on its left as on its right — inside the row only, the air left of the
    /// mark is narrower than on its right.
    property real bleed: 0
    /// The fill dropped presses had reached (`HoldDriver.spoilKept`), standing until the red has come over it.
    property real kept: 0
    /// How far the red has come from the right, 0 to 1, and how strongly it is drawn as it goes
    /// (`HoldDriver.spoilSweep` / `spoilSweepFade`).
    property real sweep: 0
    property real sweepFade: 0

    /// A white delete keeps the plain ink for its words only: the fill carries the warning, as its mark does
    /// (デザイン規約 §回数押しの色). Not a fill in the ground's blue-grey (`borderStrong`): pressed, it cannot be told
    /// from the hover wash.
    readonly property color full: Qt.colorEqual(holdFill.tone, Theme.textPrimary) ? Theme.warning : holdFill.tone
    /// The step down each tone has (デザイン規約 §暗く落とした段).
    readonly property color down: Qt.colorEqual(holdFill.full, Theme.warning) ? Theme.warningDim
                                : Qt.colorEqual(holdFill.tone, Theme.danger) ? Theme.dangerDim
                                : Qt.colorEqual(holdFill.tone, Theme.accent) ? Theme.accentDim
                                : holdFill.tone
    /// Where the red's left edge stands, in this item: the red covers the frame too, the fill only inside it.
    readonly property real redLeft: holdFill.width * (1 - holdFill.sweep)

    anchors.fill: parent
    anchors.leftMargin: -holdFill.bleed
    anchors.rightMargin: -holdFill.bleed

    Rectangle {
        readonly property real span: holdFill.width - 2 * holdFill.inset
        /// The presses' own fill, else what dropped presses left of it, up to where the red has come.
        readonly property real reach: holdFill.progress > 0 ? span * Math.min(holdFill.progress, 1)
                                                            : Math.min(span * holdFill.kept,
                                                                       holdFill.redLeft - holdFill.inset)
        x: holdFill.inset
        y: holdFill.inset
        height: holdFill.height - 2 * holdFill.inset
        // Floored at `holdFillMin`: proportional from zero, the first frames of a step are a sub-pixel sliver.
        width: holdFill.progress > 0 ? Math.max(Metrics.holdFillMin, reach) : Math.max(0, reach)
        radius: Theme.radiusSm
        color: holdFill.dimmed ? holdFill.down : holdFill.full
        Behavior on color {
            ColorAnimation {
                duration: Metrics.holdStepMs / 2
            }
        }
        visible: width > 0
    }
    // Over the frame as well: the press was refused, not the fill drawn back.
    Rectangle {
        x: holdFill.redLeft
        width: holdFill.width - holdFill.redLeft
        height: holdFill.height
        radius: Theme.radiusSm
        color: Theme.danger
        opacity: holdFill.sweepFade
        visible: holdFill.sweep > 0 && holdFill.sweepFade > 0
    }
}
