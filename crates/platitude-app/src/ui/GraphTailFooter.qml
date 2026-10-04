pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Window cut: the lanes run on into the footer, fading out, and the message sits where subjects go. One row tall unless
// the message wraps — it is the only thing explaining the cut. The whole footer is a press, worn like a row's, that
// loads the next step of history (`session::log_window_step`); no mark on the lanes, since the words name the cut.
Item {
    id: tail

    required property var graphModel
    /// The rows' column geometry, so the strokes line up with the last row's.
    required property real labelWidth
    required property real graphColWidth
    required property real graphXOffset
    required property real graphFullWidth
    required property bool findOn

    /// The search passed the last row over, so the lanes here start from its dimmed strength (規約 §コミットを探す) —
    /// as they do while the discard log's entry is on the graph, which this end is never part of.
    readonly property bool dimmed: (tail.findOn && !tail.graphModel.tailMatched) || tail.graphModel.provisionalOn
    onDimmedChanged: tailCanvas.requestPaint()

    /// Whether a press here would load anything: the walk stopped at the window, and the last press has been answered.
    readonly property bool canLoadMore: tail.graphModel.truncated && !tail.graphModel.growing
    /// For the runs: whether the ring is up. The wait is too short to photograph, so `graph-tail-more` reads this on
    /// the frame after its press.
    readonly property bool waiting: tailWait.visible

    height: tail.graphModel.truncated
            ? Math.max(Theme.graphRowHeight, tailMessage.contentHeight + 2 * Theme.spaceXs)
            : 0
    visible: tail.graphModel.truncated

    /// The press, from this footer's own mouse or the lane strip (`GraphLanePan.tailPressed`); answers whether it was
    /// taken.
    function loadMore() {
        if (!tail.canLoadMore)
            return false
        tail.graphModel.growWindow()
        return true
    }

    // A row's hover band: this answers a press too.
    Rectangle {
        id: tailHover
        anchors.fill: parent
        color: Theme.bgHover
        visible: tailMouse.containsMouse && tail.canLoadMore
    }

    // Reaching, and sinking at the column's edge, as far as a row's lanes (`GraphLaneCell`).
    Item {
        x: tail.labelWidth
        width: tail.graphColWidth + Theme.spaceSm
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
                ctx.lineWidth = Metrics.laneStroke
                // A row's geometry records, all `through` (`encode::tail_lanes`); a stash or WIP leash whose target
                // is past the cut stays dashed.
                for (const seg of tail.graphModel.tailGeometry) {
                    const x = Metrics.laneInset + seg.lane * Metrics.laneW + Metrics.laneW / 2
                    // From the last row's own strength (`dimmed`) down to nothing — drawn flat, it would step where
                    // the eye follows the line.
                    const fade = ctx.createLinearGradient(0, 0, 0, height)
                    const hex = Theme.graphLane[seg.color % Theme.graphLane.length]
                    fade.addColorStop(0, tailCanvas.faded(hex, tail.dimmed ? Metrics.dimFade : 1))
                    fade.addColorStop(1, tailCanvas.faded(hex, 0))
                    ctx.strokeStyle = fade
                    ctx.setLineDash(seg.dashed ? Metrics.laneDash : [])
                    ctx.beginPath()
                    ctx.moveTo(x, 0)
                    ctx.lineTo(x, height)
                    ctx.stroke()
                }
                ctx.setLineDash([])
            }
            /// A lane's colour at `a` of its strength. `Theme.graphLane` holds strings (`.r` is undefined, and
            /// `Qt.rgba` of it draws nothing), so the alpha is prefixed as `#AARRGGBB`.
            function faded(hex, a) {
                const v = Math.round(Math.max(0, Math.min(1, a)) * 255)
                return "#" + (v < 16 ? "0" : "") + v.toString(16) + hex.substring(1)
            }
            Connections {
                target: tail.graphModel
                function onStatsChanged() { tailCanvas.requestPaint() }
            }
        }
        LaneDissolve {
            x: tail.graphColWidth
            height: parent.height
            ground: tailHover.visible ? Qt.tint(Theme.bgSurface, Theme.bgHover) : Theme.bgSurface
        }
    }
    // Kept in the subject column, in the secondary colour: the window is normal, not a state (デザイン規約 §状態).
    Label {
        id: tailMessage
        x: tail.labelWidth + tail.graphColWidth
        width: tail.width - x
        // Its own height, centred: the footer is sized from this, so reading the footer's height here is a loop.
        height: contentHeight
        y: (parent.height - height) / 2
        // Where every subject above starts: the three steps of `GraphColumnMetrics.subjectTextX` (margin, tick, gap),
        // added as `GraphHeadPin` adds them.
        leftPadding: Theme.spaceSm + 2 * Theme.borderWidth + Theme.spaceXs
        // And where they stop: the pane's own inset, the same a row's message keeps (規約 §余白).
        rightPadding: Theme.spaceXs
        wrapMode: Text.Wrap
        // The offer leads, the count follows. `walkedTotal` is the walk's own count: the WIP row and sifted stash
        // parents move the row count off the window limit.
        text: qsTr("Load %L1 more commits (%L2 loaded)")
                .arg(tail.graphModel.windowStep)
                .arg(tail.graphModel.walkedTotal)
        color: Theme.textSecondary
        font.pixelSize: Theme.fontMd
        font.weight: Theme.fontWeightStrong
    }
    // The underline that says the line can be pressed: `borderStrong` at rest, the words' colour under the pointer
    // (規約 §author の hover). Drawn, since the label's box is the whole subject column.
    Rectangle {
        x: tailMessage.x + tailMessage.leftPadding
        y: tailMessage.y + tailMessage.height + Theme.borderWidth
        width: Math.min(tailMessage.contentWidth, tail.width - x - Theme.spaceXs)
        height: Theme.borderWidth
        visible: tail.graphModel.truncated
        color: tailMouse.containsMouse && tail.canLoadMore ? Theme.textSecondary : Theme.borderStrong
    }
    // The wait ring where the words end (デザイン規約 §進行中・長押しの定数), placed off their ink — a reserved seat
    // would push the words out of the subject column.
    SpinnerIcon {
        id: tailWait
        x: tailMessage.x + tailMessage.leftPadding + tailMessage.contentWidth + Theme.spaceSm
        y: (tail.height - height) / 2
        width: Theme.iconSm
        height: Theme.iconSm
        spinning: tail.graphModel.growing
    }
    // Last, over everything here; in a wide graph the lane strip over this takes its own presses (`GraphLanePan`).
    MouseArea {
        id: tailMouse
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton
        cursorShape: tail.canLoadMore ? Qt.PointingHandCursor : Qt.ArrowCursor
        onClicked: tail.loadMore()
    }
}
