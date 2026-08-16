import QtQuick
import platitude
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

    // On the render thread, so it keeps turning while the GUI thread
    // drains models.
    //
    // Held still for the headless runs, where a turning ring photographs
    // differently every time and no two runs of the same verb produce the
    // same PNG. Built in rather than left to the caller: this is the
    // fourth ring, and the guard is what the third one forgot.
    RotationAnimator on rotation {
        running: ring.spinning && AppBackend.shotDir === ""
        loops: Animation.Infinite
        from: 0
        to: 360
        duration: Metrics.spinMs
    }
}
