import QtQuick
import platitude.ui

// The face beside an author: the picture they were given, or a 5x5 mirrored pattern generated from their name, both in
// the same circle and ring. The pattern stands in for avatar services this app cannot use (CLAUDE.md 絶対制約).
Item {
    id: ident
    property int code: 0
    /// A `file:` URL from the store, empty for the generated pattern.
    property string imageUrl: ""

    /// The picture has loaded (the canvas reads it asynchronously), so a shot can wait on it. A function: a binding on
    /// the `isImageLoaded` call would never re-evaluate.
    function pictureReady() {
        return ident.imageUrl === "" || pattern.isImageLoaded(ident.imageUrl)
    }

    width: Theme.iconLg
    height: Theme.iconLg

    InkCanvas {
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
    // Pictures are cut by the canvas clip, not painted corners or a shader mask
    // (rules-refs/app-ui.md「顔は `IdentIcon` の 1 つが両方描き」).
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
