import QtQuick
import platitude.ui

// The mark every held control wears (デザイン規約 §長押し), on NavIcon's 16px grid. Its own component because it repaints
// every frame of a press, where NavIcon repaints only on a kind or tint change.
InkCanvas {
    id: holdIcon

    /// How far into the hold the press has got, 0 to 1.
    property real progress: 0
    /// The colour of the words beside it, including the lift to `textOnAccent` over the fill.
    property color tint: Theme.textPrimary

    // The stroke scales with the grid, unlike NavIcon's: at `iconSm` its absolute width reads as a bold ring
    // (デザイン規約 §長押し).
    width: Theme.iconSm
    height: Theme.iconSm
    onProgressChanged: requestPaint()
    onTintChanged: requestPaint()

    onPaint: {
        const ctx = getContext("2d")
        const s = width / 16
        const cx = 8 * s
        const cy = 8 * s
        const r = 5.5 * s
        ctx.clearRect(0, 0, width, height)
        ctx.strokeStyle = holdIcon.tint
        ctx.fillStyle = holdIcon.tint
        ctx.lineWidth = Metrics.iconStroke * s
        ctx.lineCap = "round"
        ctx.beginPath()
        ctx.arc(cx, cy, r, 0, 2 * Math.PI)
        ctx.stroke()
        if (holdIcon.progress <= 0) {
            // At rest, the leading edge alone: a bare ring reads as a full stop.
            ctx.beginPath()
            ctx.moveTo(cx, cy)
            ctx.lineTo(cx, cy - r)
            ctx.stroke()
            return
        }
        // Proportional from zero, unlike the fill beside it (デザイン規約 §長押し).
        ctx.beginPath()
        ctx.moveTo(cx, cy)
        ctx.arc(cx, cy, r, -Math.PI / 2, -Math.PI / 2 + 2 * Math.PI * Math.min(holdIcon.progress, 1))
        ctx.closePath()
        ctx.fill()
    }
}
