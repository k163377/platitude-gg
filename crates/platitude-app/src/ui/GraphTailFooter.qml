pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Window cut: the lanes run on into the footer and the message sits where subjects go. One row tall — one more
// commit's worth of lane is all it takes to read as "and it continues" — unless the message needs more than that. It
// is the only thing explaining the cut, so a narrow subject column grows the footer.
//
// **And the line offers the next step of history**: the whole
// footer is the press, worn like a row's own — the same `bgHover` band, and a hand over it. What it loads is a quarter
// of the window the graph opened with (`session::log_window_step`), and the words lead with that offer, because the
// fading lanes have already said that this is as far as the graph goes.
//
// **And that run of lane goes out.** Full strength where the last row leaves off, gone by the
// bottom of this band: there is nothing past the cut to draw, so what stands for it fades into the ground
// (observed). Nothing is added on top — a mark there said the same thing twice, and the line below already names
// the cut in words.
Item {
    id: tail

    required property var graphModel
    /// The three columns as the rows have them, and how far the lanes have been sent sideways — the strokes below have
    /// to line up with the last row's.
    required property real labelWidth
    required property real graphColWidth
    required property real graphXOffset
    required property real graphFullWidth

    /// Whether a press here would load anything: the walk stopped at the window, and the last press has been answered.
    readonly property bool canLoadMore: tail.graphModel.truncated && !tail.graphModel.growing
    /// Whether the ring is up, off the ring itself. **A press is answered in well under a second and the footer goes
    /// with the answer where the step reaches the end of the history**, so the wait cannot be photographed: this is
    /// what a run reads on the frame after its press instead (`graph-tail-more`).
    readonly property bool waiting: tailWait.visible

    height: tail.graphModel.truncated
            ? Math.max(Theme.graphRowHeight, tailMessage.contentHeight + 2 * Theme.spaceXs)
            : 0
    visible: tail.graphModel.truncated

    /// The press, wherever the hand came in — this footer's own mouse, or the lane strip that covers the middle column
    /// whenever the lanes overflow it (`GraphLanePan.tailPressed`). Answers whether it was taken.
    function loadMore() {
        if (!tail.canLoadMore)
            return false
        tail.graphModel.growWindow()
        return true
    }

    // The band a row wears under the pointer, worn here for the same reason: this is a thing that answers a press.
    // Under the lanes and the words, which are drawn after it.
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: tailMouse.containsMouse && tail.canLoadMore
    }

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
                    // **The lanes go out.** Drawn flat at `dimFade` they put a step between the last
                    // row and this one exactly where the eye is following a line down. Full
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
            /// A lane's colour at `a` of its strength, as a gradient stop. **`Theme.graphLane` holds
            /// strings** — the token is a `var` array, so `.r` off one is `undefined` and `Qt.rgba` of that draws
            /// nothing (measured). Qt reads `#AARRGGBB`, so the alpha goes on the front of the token's own string.
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
    // Bounded like a subject: the message stays in the subject column. Told in
    // the secondary colour: the window is how the graph is meant to work, and nothing is waiting on it
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
        // **Where every subject above it starts** — the three steps `GraphColumnMetrics.subjectTextX` adds up, which
        // are the row's own margin, the coloured tick it puts before its message, and the gap after it. Added here in
        // the same shape the stand-in row adds them (`GraphHeadPin`): the footer borrows the rows' geometry for its
        // lanes, and this line is part of the same borrowing. Without the last two steps the words sat 6px left of
        // every message in the column.
        leftPadding: Theme.spaceSm + 2 * Theme.borderWidth + Theme.spaceXs
        // And where they stop: the pane's own inset, the same a row's message keeps (規約 §余白).
        rightPadding: Theme.spaceXs
        wrapMode: Text.Wrap
        // What the press loads leads; how far the graph has come follows it in brackets, which is the shape of the two
        // being an offer and its footnote. The second number is **the walk's own count**:
        // the WIP row and sifted stash parents move rows off the round window limit, and this footer
        // only stands when the walk hit it.
        text: qsTr("Load %L1 more commits (%L2 loaded)")
                .arg(tail.graphModel.windowStep)
                .arg(tail.graphModel.walkedTotal)
        color: Theme.textSecondary
        font.pixelSize: Theme.fontMd
        font.weight: Font.DemiBold
    }
    // What says the line can be pressed before the hand is anywhere near it. Same words as
    // the author line and the hash plate use for the same thing (規約 §author の hover): **at rest `borderStrong` —
    // one step under the words, so it does not compete with them — and up to the words' own colour under the
    // pointer.** No token is added, and the only ink this puts on a resting window is this 1px.
    //
    // A rule: the same reason the hash plate draws its own (`HashPlate`) — the line is
    // wanted under the words alone, and the label's box is the whole subject column.
    Rectangle {
        x: tailMessage.x + tailMessage.leftPadding
        y: tailMessage.y + tailMessage.height + Theme.borderWidth
        width: Math.min(tailMessage.contentWidth, tail.width - x - Theme.spaceXs)
        height: Theme.borderWidth
        visible: tail.graphModel.truncated
        color: tailMouse.containsMouse && tail.canLoadMore ? Theme.textSecondary : Theme.borderStrong
    }
    // The wait, where the words end: a press here goes to git like any other, and the ring is what this window says
    // about a press it is still waiting on (デザイン規約 §進行中・長押しの定数). Placed off the line's own ink
    // — a seat kept warm for it would push the footer's words out of the subject column the
    // rows above line up in.
    SpinnerIcon {
        id: tailWait
        x: tailMessage.x + tailMessage.leftPadding + tailMessage.contentWidth + Theme.spaceSm
        y: (tail.height - height) / 2
        width: Theme.iconSm
        height: Theme.iconSm
        spinning: tail.graphModel.growing
    }
    // Last, so it is over everything this footer draws. The band above reads its hover; the lane strip that sits over
    // this column in a wide graph answers for itself (`GraphLanePan`).
    MouseArea {
        id: tailMouse
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton
        cursorShape: tail.canLoadMore ? Qt.PointingHandCursor : Qt.ArrowCursor
        onClicked: tail.loadMore()
    }
}
