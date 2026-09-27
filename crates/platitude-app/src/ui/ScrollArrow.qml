import QtQuick
import platitude.ui

// One end of an upright scroll bar: a triangle with rounded corners, `span` wide in ink, centred in its box and
// pointing out of the bar (デザイン規約 §スクロールバーの矢印). Its ink is handed in by the bar.
//
// The shape is Chromium's Fluent arrow, measured drawn at device scale 8: the sharp triangle behind the ink is 1.24 ×
// the ink's width across and 0.89 × down, and every corner is rounded by 0.16 × the ink's width.
InkCanvas {
    id: arrow

    /// Points down (the bar's far end) instead of up.
    property bool down: false
    /// Centred on the box's right edge, so the edge cuts it in half: the arrow of `PaneScrollBar`, whose slab is
    /// likewise half of a thumb sunk into the pane's edge. The one square corner is the cut, as the slab's are.
    property bool buried: false
    /// The ink's width of the whole shape (a buried arrow shows half of it).
    property real span: 0
    property color ink: "transparent"
    /// The ink's height per unit of `span`, set by the shape below: what a bar sizes its ends by.
    readonly property real tallShare: 0.89 - 0.16 * (1 / Math.sin(Math.atan(1.24 / 2 / 0.89)) - 1)

    onDownChanged: arrow.requestPaint()
    onBuriedChanged: arrow.requestPaint()
    onSpanChanged: arrow.requestPaint()
    onInkChanged: arrow.requestPaint()
    onWidthChanged: arrow.requestPaint()
    onHeightChanged: arrow.requestPaint()
    // Most arrows come into the scene inside a bar that has nowhere to go yet, some with no size either (a list laid
    // out after it is made), and a `Canvas` not visible then asks for no first paint (rules-refs/app-ui.md「`visible:
    // false` で生まれた `Canvas` は一度も描かれていない」): the bar would come up with its arrows never drawn.
    onVisibleChanged: if (arrow.visible) arrow.requestPaint()
    onPaint: {
        const ctx = getContext("2d")
        ctx.clearRect(0, 0, width, height)
        if (arrow.span <= 0)
            return
        const across = 1.24 * arrow.span
        const tall = 0.89 * arrow.span
        const r = 0.16 * arrow.span
        // The sharp tip stands past the ink by a circle of radius r in a corner of its half-angle (`tallShare`).
        const inkTall = arrow.tallShare * arrow.span
        // The flat side on a whole pixel, as Chromium's is: a side straddling two rows draws as a smear.
        const flat = arrow.down ? Math.round((height - inkTall) / 2) : Math.round((height + inkTall) / 2)
        const tip = arrow.down ? flat + tall : flat - tall
        const mid = arrow.buried ? width : width / 2
        const pts = [[mid, tip], [mid + across / 2, flat], [mid - across / 2, flat]]
        ctx.fillStyle = arrow.ink
        ctx.beginPath()
        ctx.moveTo((pts[2][0] + pts[0][0]) / 2, (pts[2][1] + pts[0][1]) / 2)
        for (let i = 0; i < 3; i++) {
            const p = pts[i]
            const q = pts[(i + 1) % 3]
            ctx.arcTo(p[0], p[1], q[0], q[1], r)
        }
        ctx.closePath()
        ctx.fill()
    }
}
