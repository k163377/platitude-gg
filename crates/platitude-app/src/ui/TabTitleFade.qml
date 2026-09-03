import QtQuick
import platitude.ui

// The quiet a tab's name goes into where the mark stands over it (デザイン規約 §タブの所作).
//
// The tab's own ground, laid over the name in a **disc on the mark**: whole inside the mark's own wash, then down to
// nothing over the run beyond it — so what the mark stands on has gone rather than been crossed out, and the name comes
// back along the same circle the wash ends on. A band would clear the name to the tab's full height, which leaves the
// letters returning on a straight edge the wash does not have: a line, exactly where the eye is already looking.
//
// **Not a mask on the name**: the offscreen scene renders in software, where `ShaderEffect` — and the `MultiEffect`
// built on it — draws nothing at all, so a masked name would be missing from every headless picture, and from any
// machine whose Qt falls back to software (rules-refs/app-ui.md carries the measurement). A `Canvas` is what draws a
// radial gradient in every backend, which is the same reason the graph's own fades are drawn in one.
InkCanvas {
    id: fade

    /// What lies under the name here: the tab's ground with whatever wash it is wearing already folded in (`Qt.tint`).
    /// A coat of anything else leaves a rectangle of the wrong colour standing at the tab's end.
    required property color ground
    /// Where the mark's ink stands in this item, and how far the quiet reaches around it — the wash's own disc, so the
    /// paint the pointer lights and the run the name is missing from are the one circle.
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
        // The same ground at none of itself rather than `transparent`, which is a transparent **black** and takes the
        // ramp through a colour the tab has not got on its way out (デザイン規約 §色).
        quiet.addColorStop(0, fade.ground)
        quiet.addColorStop(1, Qt.rgba(fade.ground.r, fade.ground.g, fade.ground.b, 0))
        ctx.fillStyle = quiet
        ctx.fillRect(0, 0, width, height)
    }
}
