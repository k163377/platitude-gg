import QtQuick
import platitude.ui

// Hand-drawn 16px-grid icons in the common git-client style (branch
// fork, cloud remote, price-tag, archive box, tree, clock). Scaled by
// the item size; single stroke color.
Canvas {
    id: icon
    property string kind: "branch"
    property color tint: Theme.textSecondary
    width: Theme.iconMd
    height: Theme.iconMd
    onKindChanged: requestPaint()
    onTintChanged: requestPaint()
    onPaint: {
        const ctx = getContext("2d")
        const s = width / 16
        ctx.clearRect(0, 0, width, height)
        ctx.strokeStyle = icon.tint
        ctx.fillStyle = icon.tint
        ctx.lineWidth = Metrics.iconStroke
        ctx.lineCap = "round"
        if (icon.kind === "branch") {
            ctx.beginPath()
            ctx.moveTo(5 * s, 5 * s)
            ctx.lineTo(5 * s, 11 * s)
            ctx.stroke()
            ctx.beginPath()
            ctx.moveTo(11 * s, 7 * s)
            ctx.bezierCurveTo(11 * s, 9.5 * s, 8 * s, 9.5 * s, 5.8 * s, 10.2 * s)
            ctx.stroke()
            for (const c of [[5, 3.5], [5, 12.5], [11, 5]]) {
                ctx.beginPath()
                ctx.arc(c[0] * s, c[1] * s, 1.8 * s, 0, 2 * Math.PI)
                ctx.stroke()
            }
        } else if (icon.kind === "remote") {
            ctx.beginPath()
            ctx.arc(6 * s, 9 * s, 3 * s, Math.PI * 0.5, Math.PI * 1.5)
            ctx.arc(8.5 * s, 6.8 * s, 3.2 * s, Math.PI * 0.95, Math.PI * 0.02, false)
            ctx.arc(11 * s, 9.4 * s, 2.6 * s, Math.PI * 1.55, Math.PI * 0.5)
            ctx.closePath()
            ctx.stroke()
        } else if (icon.kind === "tag") {
            ctx.save()
            ctx.translate(8 * s, 8.5 * s)
            ctx.rotate(Math.PI / 4)
            ctx.strokeRect(-3.6 * s, -3.6 * s, 7.2 * s, 7.2 * s)
            ctx.beginPath()
            ctx.arc(-1.4 * s, -1.4 * s, 1 * s, 0, 2 * Math.PI)
            ctx.fill()
            ctx.restore()
        } else if (icon.kind === "stash") {
            ctx.strokeRect(3 * s, 4 * s, 10 * s, 3 * s)
            ctx.strokeRect(4 * s, 7 * s, 8 * s, 6 * s)
            ctx.beginPath()
            ctx.moveTo(6.5 * s, 9.5 * s)
            ctx.lineTo(9.5 * s, 9.5 * s)
            ctx.stroke()
        } else if (icon.kind === "tree") {
            ctx.beginPath()
            ctx.arc(8 * s, 6.5 * s, 4 * s, 0, 2 * Math.PI)
            ctx.stroke()
            ctx.beginPath()
            ctx.moveTo(8 * s, 10.5 * s)
            ctx.lineTo(8 * s, 14 * s)
            ctx.stroke()
        } else if (icon.kind === "pen") {
            ctx.save()
            ctx.translate(8 * s, 8 * s)
            ctx.rotate(Math.PI / 4)
            ctx.strokeRect(-1.4 * s, -6 * s, 2.8 * s, 8.5 * s)
            ctx.beginPath()
            ctx.moveTo(-1.4 * s, 2.5 * s)
            ctx.lineTo(0, 5.5 * s)
            ctx.lineTo(1.4 * s, 2.5 * s)
            ctx.closePath()
            ctx.fill()
            ctx.restore()
        } else if (icon.kind === "plus") {
            ctx.beginPath()
            ctx.moveTo(8 * s, 3.5 * s)
            ctx.lineTo(8 * s, 12.5 * s)
            ctx.moveTo(3.5 * s, 8 * s)
            ctx.lineTo(12.5 * s, 8 * s)
            ctx.stroke()
        } else if (icon.kind === "minus") {
            ctx.beginPath()
            ctx.moveTo(3.5 * s, 8 * s)
            ctx.lineTo(12.5 * s, 8 * s)
            ctx.stroke()
        } else if (icon.kind === "arrow") {
            ctx.beginPath()
            ctx.moveTo(3.5 * s, 8 * s)
            ctx.lineTo(11.5 * s, 8 * s)
            ctx.moveTo(11.5 * s, 8 * s)
            ctx.lineTo(8.8 * s, 5.2 * s)
            ctx.moveTo(11.5 * s, 8 * s)
            ctx.lineTo(8.8 * s, 10.8 * s)
            ctx.stroke()
        } else if (icon.kind === "fetch" || icon.kind === "push") {
            // Network pair, drawn as one shape mirrored: the base line
            // is this repository, and the arrow either lands on it
            // (fetch) or leaves it (push).
            const down = icon.kind === "fetch"
            const tip = down ? 10 : 2.5
            const tail = down ? 2.5 : 10
            const barb = down ? tip - 3.2 : tip + 3.2
            ctx.beginPath()
            ctx.moveTo(8 * s, tail * s)
            ctx.lineTo(8 * s, tip * s)
            ctx.moveTo(4.8 * s, barb * s)
            ctx.lineTo(8 * s, tip * s)
            ctx.lineTo(11.2 * s, barb * s)
            ctx.moveTo(3.5 * s, 13 * s)
            ctx.lineTo(12.5 * s, 13 * s)
            ctx.stroke()
        } else if (icon.kind === "copyicon") {
            ctx.strokeRect(5.5 * s, 3.5 * s, 7 * s, 7 * s)
            ctx.strokeRect(3.5 * s, 5.5 * s, 7 * s, 7 * s)
        } else if (icon.kind === "bang") {
            ctx.beginPath()
            ctx.moveTo(8 * s, 3.5 * s)
            ctx.lineTo(8 * s, 10 * s)
            ctx.stroke()
            ctx.beginPath()
            ctx.arc(8 * s, 12.8 * s, 1 * s, 0, 2 * Math.PI)
            ctx.fill()
        } else if (icon.kind === "hier") {
            ctx.strokeRect(3 * s, 3 * s, 3 * s, 3 * s)
            ctx.beginPath()
            ctx.moveTo(4.5 * s, 6 * s)
            ctx.lineTo(4.5 * s, 12 * s)
            ctx.moveTo(4.5 * s, 8 * s)
            ctx.lineTo(10 * s, 8 * s)
            ctx.moveTo(4.5 * s, 12 * s)
            ctx.lineTo(10 * s, 12 * s)
            ctx.stroke()
            ctx.strokeRect(10 * s, 6.5 * s, 3 * s, 3 * s)
            ctx.strokeRect(10 * s, 10.5 * s, 3 * s, 3 * s)
        } else if (icon.kind === "list") {
            ctx.beginPath()
            for (const ly of [4.5, 8, 11.5]) {
                ctx.moveTo(5.5 * s, ly * s)
                ctx.lineTo(13 * s, ly * s)
            }
            ctx.stroke()
            for (const ly of [4.5, 8, 11.5]) {
                ctx.beginPath()
                ctx.arc(3.4 * s, ly * s, 0.9 * s, 0, 2 * Math.PI)
                ctx.fill()
            }
        } else if (icon.kind === "pr") {
            // GitHub-style pull request: left commit line, right elbow
            // arrow into the merge node.
            ctx.beginPath()
            ctx.moveTo(4.5 * s, 5.5 * s)
            ctx.lineTo(4.5 * s, 10.5 * s)
            ctx.stroke()
            for (const c of [[4.5, 3.8], [4.5, 12.2], [11.5, 12.2]]) {
                ctx.beginPath()
                ctx.arc(c[0] * s, c[1] * s, 1.7 * s, 0, 2 * Math.PI)
                ctx.stroke()
            }
            ctx.beginPath()
            ctx.moveTo(7.4 * s, 3.8 * s)
            ctx.lineTo(9.8 * s, 3.8 * s)
            ctx.quadraticCurveTo(11.5 * s, 3.8 * s, 11.5 * s, 5.5 * s)
            ctx.lineTo(11.5 * s, 10.5 * s)
            ctx.stroke()
            ctx.beginPath()
            ctx.moveTo(8.8 * s, 2.5 * s)
            ctx.lineTo(7.2 * s, 3.8 * s)
            ctx.lineTo(8.8 * s, 5.1 * s)
            ctx.stroke()
        } else if (icon.kind === "check") {
            ctx.beginPath()
            ctx.moveTo(3.5 * s, 8.5 * s)
            ctx.lineTo(6.8 * s, 11.8 * s)
            ctx.lineTo(12.5 * s, 4.5 * s)
            ctx.stroke()
        } else if (icon.kind === "pin") {
            // Map pin: "you are here".
            ctx.beginPath()
            ctx.arc(8 * s, 6.2 * s, 3.4 * s, Math.PI * 0.75, Math.PI * 0.25)
            ctx.lineTo(8 * s, 13.5 * s)
            ctx.closePath()
            ctx.stroke()
            ctx.beginPath()
            ctx.arc(8 * s, 6.2 * s, 1.2 * s, 0, 2 * Math.PI)
            ctx.fill()
        } else if (icon.kind === "folder") {
            ctx.beginPath()
            ctx.moveTo(2.5 * s, 12.5 * s)
            ctx.lineTo(2.5 * s, 4.5 * s)
            ctx.lineTo(6.5 * s, 4.5 * s)
            ctx.lineTo(8 * s, 6 * s)
            ctx.lineTo(13.5 * s, 6 * s)
            ctx.lineTo(13.5 * s, 12.5 * s)
            ctx.closePath()
            ctx.stroke()
        } else if (icon.kind === "clock") {
            ctx.beginPath()
            ctx.arc(8 * s, 8 * s, 5.5 * s, 0, 2 * Math.PI)
            ctx.stroke()
            ctx.beginPath()
            ctx.moveTo(8 * s, 8 * s)
            ctx.lineTo(8 * s, 4.8 * s)
            ctx.moveTo(8 * s, 8 * s)
            ctx.lineTo(10.4 * s, 8 * s)
            ctx.stroke()
        }
    }
}
