import QtQuick
import platitude.ui

// The mark every held control wears, on the same 16px grid NavIcon
// draws to: a ring with the sector's leading edge standing at twelve
// o'clock — a gauge sitting at zero — that fills clockwise for as long
// as the press lasts (デザイン規約 §長押し).
//
// It is what lets the words go back to naming the operation and nothing
// else: the row says `Delete`, and the mark ahead of it says how.
//
// Its own component rather than another NavIcon kind: NavIcon repaints
// only when its kind or its tint changes, and this one is repainted some
// thirty times over a single press.
Canvas {
    id: holdIcon

    /// How far into the hold the press has got, 0 to 1.
    property real progress: 0
    /// Drawn in the colour of the words it stands next to, so the mark
    /// and the sentence cannot disagree — including the lift to
    /// `textOnAccent` while the fill runs under both of them.
    property color tint: Theme.textPrimary

    // Smaller than the words it stands next to, and drawn at the same
    // weight relative to its own box as NavIcon is to a full-sized one —
    // the stroke is scaled with the grid rather than kept at its absolute
    // width, which at this size would read as a bold ring around a small
    // hole. The sector loses detail at the small end; that is the trade
    // for a mark that sits in a sentence without shouting over it.
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
        // The ring is the whole of the gesture, drawn whether or not any
        // of it has been done yet.
        ctx.beginPath()
        ctx.arc(cx, cy, r, 0, 2 * Math.PI)
        ctx.stroke()
        if (holdIcon.progress <= 0) {
            // At rest, the sector's leading edge on its own. A bare ring
            // reads as a full stop; a ring with a hand on it reads as
            // something with somewhere to go.
            ctx.beginPath()
            ctx.moveTo(cx, cy)
            ctx.lineTo(cx, cy - r)
            ctx.stroke()
            return
        }
        // Filled from twelve o'clock, clockwise. No minimum sweep of its
        // own: the frame beside it starts at `holdFillMin`, so there is
        // no moment here that has to carry the report alone
        // (デザイン規約 §進行中・長押しの定数).
        ctx.beginPath()
        ctx.moveTo(cx, cy)
        ctx.arc(cx, cy, r, -Math.PI / 2,
                -Math.PI / 2 + 2 * Math.PI * Math.min(holdIcon.progress, 1))
        ctx.closePath()
        ctx.fill()
    }
}
