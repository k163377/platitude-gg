import QtQuick
import platitude.ui

// The face beside an author: the picture they were given, or — far more often — a 5x5 mirrored pattern generated from
// their name.
//
// Both wear the same circle and the same ring, so a list of authors reads as one column of faces whether or not anybody
// has been given a picture. The pattern is what stands in for the avatar services this application cannot use
// (CLAUDE.md 絶対制約: git is the only network there is).
Item {
    id: ident
    property int code: 0
    /// A `file:` URL from the store, empty for the generated pattern.
    property string imageUrl: ""
    width: Theme.iconLg
    height: Theme.iconLg

    Canvas {
        id: pattern
        anchors.fill: parent
        onImageLoaded: requestPaint()
        onPaint: {
            const ctx = getContext("2d")
            const r = width / 2
            ctx.clearRect(0, 0, width, height)
            ctx.save()
            ctx.beginPath()
            ctx.arc(r, r, r, 0, 2 * Math.PI)
            ctx.clip()
            if (ident.imageUrl !== "") {
                if (pattern.isImageLoaded(ident.imageUrl))
                    ctx.drawImage(ident.imageUrl, 0, 0, width, height)
                ctx.restore()
                ctx.strokeStyle = Theme.borderStrong
                ctx.lineWidth = Theme.borderWidth
                ctx.beginPath()
                ctx.arc(r, r, r - 0.5, 0, 2 * Math.PI)
                ctx.stroke()
                return
            }
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
    // A picture is cut to the circle by the canvas's own clip, so nothing outside it is painted: a node in the graph
    // has lane curves running in behind it, and anything opaque in the corners would take bites out of them. The
    // alternative, masking an Image with a shader, draws nothing at all on the offscreen platform (measured) — which
    // would mean a feature that works but cannot be photographed.
    Connections {
        target: ident
        function onCodeChanged() { pattern.requestPaint() }
        function onImageUrlChanged() {
            if (ident.imageUrl !== "")
                pattern.loadImage(ident.imageUrl)
            pattern.requestPaint()
        }
    }
    Component.onCompleted: if (ident.imageUrl !== "") pattern.loadImage(ident.imageUrl)
}
