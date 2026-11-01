import QtQuick
import platitude.ui

// The quiet a tab's name goes into under its mark: the tab's ground painted over the name in a disc on the mark, whole
// inside the wash and fading out over `rampW`.
//
// A crowded strip gives up the room kept for the mark first (`TabShare`'s order), so names run on under the `✕`. **They
// run on rather than being cut short of it**: the `✕` stands only on the tab in front and the one under the hand, so a
// cut would shrink the name the moment the mark comes out. A name cut in the middle goes under the same way — cut or
// not, it must not re-lay as the mark appears. **A gradient, not a step** (規約 §省略の表し方): a step says the name ended
// there. **A disc, not a square**: the same circle as the mark's wash (§当たり判定) — squared off, the letters come back
// along a vertical line and leave an empty band against the wash's curve. Only while the mark shows: a tab with no mark
// reads its name to its far edge.
//
// Painted over, not masked: offscreen renders in software, where `ShaderEffect` draws nothing — and `MultiEffect`,
// built on it, likewise, with no warning. A mask would take the name out of every headless picture, and out of a real
// window on a machine with no GPU. A `Canvas` draws a radial gradient in every backend. The ground is "the colour under
// that tab now": the band's (`TabStrip.bandColor`, handed down by `TopBar` — a tab has no ground of its own, so the
// token is named there once) tinted with the hover wash, or the front tab's own (`TabItemDelegate.groundColor`).
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
