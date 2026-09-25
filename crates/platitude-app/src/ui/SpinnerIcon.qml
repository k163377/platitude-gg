import QtQuick
import platitude.ui

// The one turning mark the window has: the drawn ring (規約 §進行中・長押しの定数). Drawn exactly while it turns, so
// callers write one condition.
NavIcon {
    id: ring

    /// Something is out and being waited on.
    property bool spinning: false

    kind: "spinner"
    tint: Theme.textSecondary
    visible: ring.spinning
    // Most rings are born invisible (rules-refs/app-ui.md「`visible: false` で生まれた `Canvas` は一度も描かれていない」);
    // rotation does not ask for paint either — it only turns ink already there.
    onVisibleChanged: if (ring.visible) ring.requestPaint()

    // On the render thread, so it keeps turning while the GUI thread drains models. Held still under `Motion.stilled`
    // so runs of the same verb photograph the same PNG — here, so no caller can forget it.
    RotationAnimator on rotation {
        running: ring.spinning && !Motion.stilled
        loops: Animation.Infinite
        from: 0
        to: 360
        duration: Metrics.spinMs
    }
}
