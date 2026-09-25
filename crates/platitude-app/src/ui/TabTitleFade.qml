import QtQuick
import platitude.ui

// The quiet a tab's name goes into under its mark (デザイン規約 §タブの所作): the tab's ground painted over the name
// in a disc on the mark, whole inside the wash and fading out over `rampW`.
//
// Painted over, not masked: offscreen renders in software, where `ShaderEffect` / `MultiEffect` draw nothing
// (rules-refs/app-ui.md「`✕` の下に入った名前の消し方」). A `Canvas` draws a radial gradient in every backend.
InkCanvas {
    id: fade

    /// Exactly what lies under the name: the tab's ground with its wash folded in (`Qt.tint`) — anything else leaves a
    /// wrong-coloured patch at the tab's end.
    required property color ground
    /// The mark's centre in this item and the wash's radius, so the wash and the quiet are one circle.
    required property real markX
    required property real markR
    /// How far past that disc the name fades back in (`TabMetrics.fadeChars`).
    required property real rampW

    onWidthChanged: fade.requestPaint()
    onHeightChanged: fade.requestPaint()
    onGroundChanged: fade.requestPaint()
    onMarkXChanged: fade.requestPaint()
    onMarkRChanged: fade.requestPaint()
    onRampWChanged: fade.requestPaint()
    onPaint: {
        const ctx = getContext("2d")
        ctx.clearRect(0, 0, width, height)
        const cy = height / 2
        const quiet = ctx.createRadialGradient(fade.markX, cy, fade.markR, fade.markX, cy, fade.markR + fade.rampW)
        // The ground at zero alpha: `transparent` is black and would ramp through a colour the tab has not got.
        quiet.addColorStop(0, fade.ground)
        quiet.addColorStop(1, Qt.rgba(fade.ground.r, fade.ground.g, fade.ground.b, 0))
        ctx.fillStyle = quiet
        ctx.fillRect(0, 0, width, height)
    }
}
