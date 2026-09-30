import QtQuick
import platitude.ui

// The face beside an author: the picture they were given, or a 5x5 mirrored pattern generated from their name, both in
// the same circle and ring. The pattern stands in for avatar services this app cannot use (CLAUDE.md 絶対制約).
Item {
    id: ident
    property int code: 0
    /// A `file:` URL from the store, empty for the generated pattern.
    property string imageUrl: ""
    /// How far under the box's top the face is drawn, less than a pixel: a holder centring the face on something a
    /// whole pixel cannot centre it on (`CoAuthorLine`) moves the box by whole pixels and hands the rest here, drawn
    /// by the painter rather than by shifting the texture, which would blur every row. The canvas runs past the box.
    property real drop: 0

    /// The picture has loaded (the canvas reads it asynchronously), so a shot can wait on it. A function: a binding on
    /// the `isImageLoaded` call would never re-evaluate.
    function pictureReady() {
        return ident.imageUrl === "" || pattern.isImageLoaded(ident.imageUrl)
    }

    width: Theme.iconLg
    height: Theme.iconLg

    InkCanvas {
        id: pattern
        width: ident.width
        height: ident.height + Math.ceil(ident.drop)
        onImageLoaded: requestPaint()
        onPaint: {
            const ctx = getContext("2d")
            const d = width
            const r = d / 2
            ctx.setTransform(1, 0, 0, 1, 0, 0)
            ctx.clearRect(0, 0, width, height)
            ctx.translate(0, ident.drop)
            ctx.save()
            ctx.beginPath()
            ctx.arc(r, r, r, 0, 2 * Math.PI)
            ctx.clip()
            if (ident.imageUrl !== "") {
                if (pattern.isImageLoaded(ident.imageUrl))
                    ctx.drawImage(ident.imageUrl, 0, 0, d, d)
                ctx.restore()
                ctx.strokeStyle = Theme.borderStrong
                ctx.lineWidth = Theme.borderWidth
                ctx.beginPath()
                ctx.arc(r, r, r - 0.5, 0, 2 * Math.PI)
                ctx.stroke()
                return
            }
            ctx.fillStyle = Theme.bgElevated
            ctx.fillRect(0, 0, d, d)
            ctx.fillStyle = Theme.graphLane[(ident.code >> 15) & 0x7]
            const inner = d * Metrics.identiconFill
            const cell = inner / 5
            const o = (d - inner) / 2
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
    // Pictures are cut by the canvas clip, not painted corners or a shader mask
    // (rules-refs/app-ui.md「顔は `IdentIcon` の 1 つが両方描き」).
    Connections {
        target: ident
        function onCodeChanged() { pattern.requestPaint() }
        function onDropChanged() { pattern.requestPaint() }
        function onImageUrlChanged() {
            if (ident.imageUrl !== "")
                pattern.loadImage(ident.imageUrl)
            pattern.requestPaint()
        }
    }
    Component.onCompleted: if (ident.imageUrl !== "") pattern.loadImage(ident.imageUrl)
}
