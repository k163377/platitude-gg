import QtQuick
import platitude.ui

// Author identicon (same packed code as the graph nodes): 5x5 mirrored
// pattern, local substitute for network avatars.
Canvas {
    id: ident
    property int code: 0
    width: Theme.iconLg
    height: Theme.iconLg
    onCodeChanged: requestPaint()
    onPaint: {
        const ctx = getContext("2d")
        const r = width / 2
        ctx.clearRect(0, 0, width, height)
        ctx.save()
        ctx.beginPath()
        ctx.arc(r, r, r, 0, 2 * Math.PI)
        ctx.clip()
        ctx.fillStyle = Theme.bgElevated
        ctx.fillRect(0, 0, width, height)
        ctx.fillStyle = Theme.graphLane[(ident.code >> 15) & 0x7]
        const inner = width * Metrics.identiconFill
        const cell = inner / 5
        const o = (width - inner) / 2
        for (let row = 0; row < 5; row++) {
            for (let col = 0; col < 3; col++) {
                if ((ident.code >> (row * 3 + col)) & 1) {
                    ctx.fillRect(o + col * cell, o + row * cell, cell + 0.5, cell + 0.5)
                    if (col < 2)
                        ctx.fillRect(o + (4 - col) * cell, o + row * cell, cell + 0.5, cell + 0.5)
                }
            }
        }
        ctx.restore()
        ctx.strokeStyle = Theme.borderStrong
        ctx.lineWidth = Theme.borderWidth
        ctx.beginPath()
        ctx.arc(r, r, r - 0.5, 0, 2 * Math.PI)
        ctx.stroke()
    }
}
