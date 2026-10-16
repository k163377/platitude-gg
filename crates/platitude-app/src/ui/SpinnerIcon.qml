import QtQuick
import platitude.ui

// The one turning mark the window has: the drawn ring (規約 §進行中・回数押しの定数). Drawn exactly while it turns, so
// callers write one condition.
NavIcon {
    id: ring

    /// Something is out and being waited on.
    property bool spinning: false
    /// The pixel size of the words it turns beside; zero for none. Set, the ring is as tall as their capitals and
    /// drawn in their stem, as a mark set among them is (デザイン規約 §アイコンの席): its box a 16-grid whose ring
    /// spans 10, centred where the caller stands it — the capitals' middle, or the `+`'s after the words.
    property real wordSize: 0
    /// The box away from words.
    property real box: Theme.iconMd

    width: ring.wordSize > 0 ? Math.round((WordFace.capital - Metrics.markStem) * ring.wordSize * 16 / 10) : ring.box
    height: ring.width
    stroke: ring.wordSize > 0 ? Metrics.markStem * ring.wordSize : Metrics.iconStroke
    kind: "spinner"
    tint: Theme.textSecondary
    visible: ring.spinning

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
