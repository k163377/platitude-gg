import QtQuick
import platitude.ui

// The mark every confirmed control wears (デザイン規約 §回数押し), on NavIcon's 16px grid: a disc split into one sector
// per press by gaps of one width from the centre out, the sectors still to press at `dimFade`. Pressing sweeps the
// disc in clockwise from 12 o'clock, across the gaps it reaches — the last press makes it whole.
// Its own component because it repaints every frame of a step, where NavIcon repaints only when what it draws changes.
InkCanvas {
    id: holdIcon

    /// The fill's progress, 0 to 1 (`HoldDriver.progress`).
    property real progress: 0
    /// How many presses confirm — the number of sectors (`HoldDriver.armedPresses`).
    property int presses: Metrics.holdPressesWarned
    /// The colour of the words beside it, including the lift to `textOnAccent` over the fill — or, where those words
    /// grey because the control cannot be pressed ([`off`]), the colour they wear when it can (`AppMenuItem.holdInk`).
    property color tint: Theme.textPrimary
    /// The control cannot be pressed: the mark keeps its hue a step down (デザイン規約 §暗く落とした段), under grey words.
    property bool off: false
    /// The words' colour, but a white delete's words keep the plain ink alone — the mark carries the warning, as the
    /// fill does (デザイン規約 §回数押しの色).
    readonly property color hue: Qt.colorEqual(holdIcon.tint, Theme.textPrimary) ? Theme.warning : holdIcon.tint
    /// What the mark is drawn in: its hue, or while off that hue's dimmed step — and a grey mark, which has no step of
    /// its own, greys with the words (§無効).
    readonly property color ink: !holdIcon.off ? holdIcon.hue
                               : Qt.colorEqual(holdIcon.hue, Theme.warning) ? Theme.warningDim
                               : Qt.colorEqual(holdIcon.hue, Theme.danger) ? Theme.dangerDim
                               : Qt.colorEqual(holdIcon.hue, Theme.accent) ? Theme.accentDim
                               : Theme.textMuted

    /// The pixel size of the words it heads; zero for none. Set, the disc is as tall as their capitals
    /// (デザイン規約 §アイコンの席「顔と回数押しの印」— the head of a row, or a letter of a button's phrase) — the box
    /// round it a 16-grid whose disc spans 12.5 — and a caller stands its middle on theirs (`WordFace.capitalDrop`).
    property real size: 0

    /// The disc's width: 12.5 of the grid's 16, the rest the box's own air either side.
    readonly property real inkWidth: holdIcon.width * 12.5 / 16

    width: holdIcon.size > 0 ? Math.round(WordFace.capital * holdIcon.size * 16 / 12.5) : Theme.iconSm
    height: holdIcon.width
    onProgressChanged: requestPaint()
    onPressesChanged: requestPaint()
    onInkChanged: requestPaint()

    /// Sector `i` of `n`, clockwise from 12 o'clock, its two edges pulled `h` in from the radii that bound it — the gap
    /// between two sectors is `2h` wide all the way in. Two such edges meet short of the centre; a half disc's are one
    /// line, through the point `h` off the centre.
    function sector(ctx, s, n, i, r, h) {
        const cx = 8 * s
        const cy = 8 * s
        const s0 = -Math.PI / 2 + i * 2 * Math.PI / n
        const s1 = s0 + 2 * Math.PI / n
        const b0 = [h * Math.cos(s0 + Math.PI / 2), h * Math.sin(s0 + Math.PI / 2)]
        const b1 = [h * Math.cos(s1 - Math.PI / 2), h * Math.sin(s1 - Math.PI / 2)]
        const d0 = [Math.cos(s0), Math.sin(s0)]
        const d1 = [Math.cos(s1), Math.sin(s1)]
        const t = Math.sqrt(r * r - h * h)
        const a0 = Math.atan2(b0[1] + t * d0[1], b0[0] + t * d0[0])
        // Clockwise from `a0`: the last sector's end comes back round past ±π.
        let a1 = Math.atan2(b1[1] + t * d1[1], b1[0] + t * d1[0])
        while (a1 <= a0)
            a1 += 2 * Math.PI
        ctx.beginPath()
        ctx.arc(cx, cy, r, a0, a1, false)
        const det = d0[0] * d1[1] - d0[1] * d1[0]
        if (Math.abs(det) < 1e-6) {
            ctx.lineTo(cx + b1[0], cy + b1[1])
            ctx.lineTo(cx + b0[0], cy + b0[1])
        } else {
            const u = ((b1[0] - b0[0]) * d1[1] - (b1[1] - b0[1]) * d1[0]) / det
            ctx.lineTo(cx + b0[0] + u * d0[0], cy + b0[1] + u * d0[1])
        }
        ctx.closePath()
        ctx.fill()
    }

    onPaint: {
        const ctx = getContext("2d")
        const s = width / 16
        // Out to the outer edge of NavIcon's round marks (`no`: a 5.5 ring in `iconStroke`), so it stands as
        // large as they do.
        const r = (5.5 + Metrics.iconStroke / 2) * s
        // Half the gap: one pixel of the 16 grid either side of the radius.
        const h = s
        const n = Math.max(2, holdIcon.presses)
        ctx.clearRect(0, 0, width, height)
        ctx.fillStyle = holdIcon.ink
        ctx.globalAlpha = Metrics.dimFade
        for (let i = 0; i < n; i++)
            holdIcon.sector(ctx, s, n, i, r, h)
        ctx.globalAlpha = 1
        if (holdIcon.progress <= 0)
            return
        // The press sweeps the whole disc, gaps included: a pressed sector reaches the middle of the gaps either side.
        ctx.beginPath()
        ctx.moveTo(8 * s, 8 * s)
        ctx.arc(8 * s, 8 * s, r, -Math.PI / 2, -Math.PI / 2 + 2 * Math.PI * Math.min(holdIcon.progress, 1), false)
        ctx.closePath()
        ctx.fill()
    }
}
