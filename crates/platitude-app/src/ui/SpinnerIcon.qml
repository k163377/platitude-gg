import QtQuick
import platitude.ui

// The one turning mark the window has: the drawn ring, never Fusion's
// `BusyIndicator` (規約 §進行中・長押しの定数).
//
// It is drawn exactly while it turns — a ring standing still says nothing
// an empty space does not — so callers write one condition, not two.
NavIcon {
    id: ring

    /// Something is out and being waited on.
    property bool spinning: false

    kind: "spinner"
    tint: Theme.textSecondary
    visible: ring.spinning
    // A ring is drawn only while something is out, so most of them are born invisible — and a `Canvas` that was never
    // visible was never asked to paint (rules-refs/app-ui.md). Turning does not ask either: rotation is a transform
    // over ink that has to be there already. Built in for the same reason the stillness above is.
    onVisibleChanged: if (ring.visible) ring.requestPaint()

    // On the render thread, so it keeps turning while the GUI thread
    // drains models.
    //
    // Held still wherever the window has been (`Motion.stilled`) — a turning ring photographs differently every time
    // and no two runs of the same verb produce the same PNG. Built in rather than left to the caller: this is the
    // fourth ring, and the guard is what the third one forgot.
    RotationAnimator on rotation {
        running: ring.spinning && !Motion.stilled
        loops: Animation.Infinite
        from: 0
        to: 360
        duration: Metrics.spinMs
    }
}
