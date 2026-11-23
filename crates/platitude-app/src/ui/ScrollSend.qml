pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// Where a view is being sent along one axis: the one move in flight, of two kinds. A move started while another is in
// flight takes over from it.
//
// - **Glide** (`glideTo`): a wheel notch or a scroll bar's step, sent across rather than jumped — the rows it passes
//   over have to be seen going by, or the reader re-reads the page to find where they are. At rest at both ends
//   (CSS `ease-in-out`, Chromium's wheel curve), so a run of notches reads as one movement. It takes
//   `Metrics.wheelGlideMs` over the rows the person's own setting gives a notch (`Metrics.wheelRows`)
//   (rules-refs/app-ui.md「ホイールの 1 ノッチは送られる、跳ばない」).
// - **Dash** (`dashTo`): a jump the app makes to bring something into view — a search, a jump or a pick landing on a
//   row, a node brought into the lane column, the tab in front brought into the strip. Its reader looks at where it
//   lands, not at what it passes, so it sets off at full speed and brakes into its place (VS Code's `easeOutCubic`):
//   at the same length it reads as quicker than a notch. Each holder names its length (`dashMs`). A dash farther than
//   its landing (`Metrics.dashLandShare` of the view) starts that far short of its aim (user decision — ask before
//   changing): the frames before it would only show what the reader is not looking for and hold back what was asked
//   for, and the landing's braking alone says which way the view went.
//
// Read `at`, never the property: mid-flight the property is short of the aim, so a second move measured from it would
// fight the first instead of stacking. The graph's rows are read where they land (`GraphList.firstVisibleRow` /
// `rowOnScreen`), so a hand that writes them from a row's place lands the move first (`land`, through
// `GraphList.landSend()`) and measures from there: the arrows' step (`GraphRowWalk.revealStep`) and a reread's place
// (`GraphRowWalk.shiftRows`). Every other hand that moves the view calls `halt` first — through the holder's
// `haltSend()` — or the move in flight drags the view back to its aim: the diff's arrow keys (`DiffRowWalk.stepRows`),
// a restore's place (`GraphRowWalk.anchorWait` / `DiffScrollPlace`), the diff's search (`DiffPane.goToHit`), the
// discard log's span (`GraphList.keepToSpan`), the middle button's drift
// (`GraphPane`, `DescriptionBox.driftText` / `MessageEditor.driftSummary`), the left menu's reveals (the menu key, a
// name box, an open row shown and given back, the drift), a hand on the lanes (`GraphColumnMetrics.panTo`) and the
// strip's other writers (`TabRun.halt`). A scroll bar steps on its view's send (`AutoScrollBar.stepSend`), so a step
// and a notch stack and the same hands stop both. The wheel cannot be injected, so a run comes in through the
// holders' functions (`sendRows` / `rollBy`); checking the glide is regression only.
//
// Held one per view and axis that is sent: the graph's rows (`GraphList`, which also dashes) and lanes
// (`GraphColumnMetrics.laneSend`, dash only), the diff (`DiffPane`), the left menu's lists (`NavList`), the description
// box and the summary (`DescriptionBox`, `MessageEditor`), the tab strip (`TabRun`, dash only), and a scroll bar not
// handed its view's (`AutoScrollBar.ownSend`).
QtObject {
    id: send

    /// What is sent: the object whose `axis` a move writes — a flickable (the message boxes hand in the one inside
    /// their `ScrollView`), or the graph's column metrics for the lanes.
    required property var view
    /// The property of `view` that is sent.
    property string axis: "contentY"
    /// How long a dash takes: each holder that dashes names its own length.
    property int dashMs: 0
    /// How much of `view` shows along `axis`: what a dash's landing is measured in.
    property real span: send.view ? (send.axis === "contentX" ? send.view.width : send.view.height) : 0

    /// Where the move in flight is aiming; read it through `at`.
    property real aim: 0
    readonly property bool gliding: glide.running
    readonly property bool dashing: dash.running
    readonly property bool sending: send.gliding || send.dashing
    /// Where the next move measures from: the aim while one is in flight, and the resting value otherwise.
    readonly property real at: send.sending ? send.aim : (send.view ? send.view[send.axis] : 0)

    /// The move stopped: landed, halted, or taken over by the next.
    signal stopped()

    /// Glides the view to `value`, already clamped by the caller (each holder has its own bounds).
    function glideTo(value) {
        send.halt()
        send.aim = value
        glide.to = value
        glide.start()
    }
    /// Dashes the view to `value`, already clamped by the caller.
    function dashTo(value) {
        send.halt()
        const land = Metrics.dashLandShare * send.span
        const from = send.view[send.axis]
        if (Math.abs(value - from) > land)
            send.view[send.axis] = value - land * Math.sign(value - from)
        send.aim = value
        dash.to = value
        dash.start()
    }
    /// Something else is moving this view: the move in flight stops, or it would drag the view back.
    function halt() {
        glide.stop()
        dash.stop()
    }
    /// Something else is about to write the view from where it was going: the move in flight lands there at once.
    function land() {
        if (!send.sending)
            return
        const to = send.aim
        send.halt()
        send.view[send.axis] = to
    }

    property NumberAnimation glide: NumberAnimation {
        id: glide
        target: send.view
        property: send.axis
        duration: Metrics.wheelGlideMs
        // CSS `ease-in-out`, Chromium's wheel curve (`scroll_offset_animation_curve.cc`, `kEaseInOutControlPoints`).
        easing.type: Easing.Bezier
        easing.bezierCurve: [0.42, 0, 0.58, 1, 1, 1]
        onStopped: send.stopped()
    }
    property NumberAnimation dash: NumberAnimation {
        id: dash
        target: send.view
        property: send.axis
        duration: send.dashMs
        // VS Code's smooth scroll (`easeOutCubic`, `scrollable.ts`): three times a steady pace at the start, at rest
        // at the end.
        easing.type: Easing.OutCubic
        onStopped: send.stopped()
    }
}
