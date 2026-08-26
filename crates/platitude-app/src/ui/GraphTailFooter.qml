pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Window cut: the lanes run on into the footer and the message sits where subjects go. One row tall — one more
// commit's worth of lane is all it takes to read as "and it continues" — unless the message needs more than that. It
// is the only thing explaining the cut, so a narrow subject column grows the footer rather than eliding it.
//
// **And that run of lane goes out rather than stopping.** Full strength where the last row leaves off, gone by the
// bottom of this band: there is nothing past the cut to draw, so what stands for it fades into the ground (2026-08-22
// ユーザー判断). Nothing is added on top — a mark there said the same thing twice, and the line below already names the
// cut in words.
Item {
    id: tail

    required property var graphModel
    /// The three columns as the rows have them, and how far the lanes have been sent sideways — the strokes below have
    /// to line up with the last row's.
    required property real labelWidth
    required property real graphColWidth
    required property real graphXOffset
    required property real graphFullWidth

    height: tail.graphModel.truncated
            ? Math.max(Theme.graphRowHeight, tailMessage.contentHeight + 2 * Theme.spaceXs)
            : 0
    visible: tail.graphModel.truncated

    Item {
        x: tail.labelWidth
        width: tail.graphColWidth
        height: parent.height
        clip: true
        InkCanvas {
            id: tailCanvas
            x: -tail.graphXOffset
            width: tail.graphFullWidth
            height: parent.height
            onPaint: {
                const ctx = getContext("2d")
                ctx.clearRect(0, 0, width, height)
                if (tail.graphModel.tailGeometry === "")
                    return
                ctx.lineWidth = Metrics.laneStroke
                // Same tokens as a row's geometry (uppercase = dashed leash): a stash or WIP row whose target sits past
                // the cut keeps dotting through here.
                const toks = tail.graphModel.tailGeometry.split(";")
                for (let n = 0; n < toks.length; n++) {
                    const t = toks[n]
                    const dot = t.indexOf(".")
                    const lane = parseInt(t.substring(1, dot))
                    const color = parseInt(t.substring(dot + 1))
                    const x = Metrics.laneInset + lane * Metrics.laneW + Metrics.laneW / 2
                    // **The lanes go out rather than stop.** Drawn flat at `dimFade` they put a step between the last
                    // row and this one exactly where the eye is following a line down (2026-08-22 ユーザー報告). Full
                    // strength where the last row leaves off, gone by the bottom — the history past the cut is not
                    // there to be drawn, so what stands for it fades out.
                    const fade = ctx.createLinearGradient(0, 0, 0, height)
                    const hex = Theme.graphLane[color % Theme.graphLane.length]
                    fade.addColorStop(0, tailCanvas.faded(hex, 1))
                    fade.addColorStop(1, tailCanvas.faded(hex, 0))
                    ctx.strokeStyle = fade
                    ctx.setLineDash(t[0] === t[0].toLowerCase() ? [] : Metrics.laneDash)
                    ctx.beginPath()
                    ctx.moveTo(x, 0)
                    ctx.lineTo(x, height)
                    ctx.stroke()
                }
                ctx.setLineDash([])
            }
            /// A lane's colour at `a` of its strength, as a gradient stop. **`Theme.graphLane` holds strings, not
            /// colours** — the token is a `var` array, so `.r` off one is `undefined` and `Qt.rgba` of that draws
            /// nothing (2026-08-22 実測). Qt reads `#AARRGGBB`, so the alpha goes on the front of the token's own string.
            function faded(hex, a) {
                const v = Math.round(Math.max(0, Math.min(1, a)) * 255)
                return "#" + (v < 16 ? "0" : "") + v.toString(16) + hex.substring(1)
            }
            Connections {
                target: tail.graphModel
                function onStatsChanged() { tailCanvas.requestPaint() }
            }
        }
    }
    // Bounded like a subject: the message stays in the subject column rather than running under the next pane. Told in
    // the secondary colour, not a state one: the window is how the graph is meant to work, and nothing is waiting on it
    // (デザイン規約 §状態).
    Label {
        id: tailMessage
        x: tail.labelWidth + tail.graphColWidth
        width: tail.width - x
        // Its own laid-out height, centered in whatever the footer ends up being: reading the footer's height back here
        // is the binding loop, since the footer is sized from this. Nothing elides — the height follows the wrap, so
        // every line of the message is shown.
        height: contentHeight
        y: (parent.height - height) / 2
        leftPadding: Theme.spaceSm
        rightPadding: Theme.spaceSm
        wrapMode: Text.Wrap
        // The walk's own count, not the row count: the WIP row and sifted stash parents move rows off the round window
        // limit, and this footer only stands when the walk hit it.
        text: qsTr("Only the first %L1 commits are loaded").arg(tail.graphModel.walkedTotal)
        color: Theme.textSecondary
        font.pixelSize: Theme.fontMd
        font.weight: Font.DemiBold
    }
}
