pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// A wheel notch, sent across rather than jumped: the rows it passes over have to be seen going by, or the reader
// re-reads the page to find where they are. Amount and curve are Chromium's (`Metrics.wheelRows` /
// `Metrics.wheelGlideMs`). One per surface that answers the wheel
// (rules-refs/app-ui.md「ホイールの 1 ノッチは送られる、跳ばない」).
//
// Read `at`, never the property: mid-flight the property is short of the aim, so a second notch measured from it
// would fight the first instead of stacking.
//
// Held by five surfaces: the graph, the diff, the description box, the summary and the left menu's lists (`NavList`,
// every section's, WORKTREES included). Every other hand that moves one of those views
// calls the surface's `haltGlide()` first, or the notch in flight drags the reader back to the old row: arrow keys
// (`GraphRowWalk.revealStep` / `DiffRowWalk.stepRows`), the bar's arrows and track (handed this glide as
// `AutoScrollBar.stepGlide`, so a step stops it), `jumpToRow`, `GraphFind.goToMatch`, `shiftRows` / `showRowSoon` /
// `anchorTimer` / `DiffScrollPlace`, a box's middle-click drift (`DescriptionBox.driftText` /
// `MessageEditor.driftSummary`), `startAutoScroll`, and the left menu's reveals (the menu key, a name box, an open
// row shown and given back, the middle button's drift). The wheel cannot be injected, so a run comes in through the
// surfaces' functions (`sendRows` / `rollBy`); checking it is regression only.
QtObject {
    id: glide

    /// The flickable whose `contentY` a notch moves; the message boxes hand in the one inside their `ScrollView`.
    required property var view

    /// Where the notch in flight is aiming; read it through `at`.
    property real aim: 0
    readonly property bool sending: send.running
    /// Where the next notch measures from: the aim while one is in flight, and the resting value otherwise.
    readonly property real at: send.running ? glide.aim : (glide.view ? glide.view.contentY : 0)

    /// Send what is under this notch to `value`, already clamped by the caller (each surface has its own bounds).
    function sendTo(value) {
        send.stop()
        glide.aim = value
        send.to = value
        send.start()
    }
    /// Something else is moving this view (an arrow key, a reveal, a restored position): the notch stops, or it would
    /// drag the view back.
    function halt() {
        send.stop()
    }

    property NumberAnimation send: NumberAnimation {
        id: send
        target: glide.view
        property: "contentY"
        duration: Metrics.wheelGlideMs
        // Chromium's wheel easing, CSS `ease-in-out` (`scroll_offset_animation_curve.cc`): at rest at both ends, so
        // a run of notches reads as one movement.
        easing.type: Easing.Bezier
        easing.bezierCurve: [0.42, 0, 0.58, 1, 1, 1]
    }
}
