pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// A wheel notch, sent across rather than jumped. **The wheel names a distance, not a place**, so the rows the notch
// passes over have to be seen going by — landing a screenful away in one frame leaves the reader re-reading the page
// from scratch to find out where they are. Every browser sends the notch for this reason; the amount and the curve
// are Chromium's own (`Metrics.wheelRows` / `Metrics.wheelGlideMs`).
//
// **One of these per surface that answers the wheel** — four do (the graph, the diff, and the two message boxes), and
// a surface working the send out for itself would be a second answer to "how far is one notch".
//
// **Read `at`, never the property.** While a notch is in flight the property is still on its way there, so a second
// notch measured from it would land short — the two would fight instead of stacking.
QtObject {
    id: glide

    /// The flickable a notch moves. Its `contentY` is the one property any of the four surfaces sends — the two
    /// message boxes hand in the flickable inside their `ScrollView`, which is the same kind of thing.
    required property var view

    /// Where the notch in flight is aiming for. Written here and read through `at`.
    property real aim: 0
    readonly property bool sending: send.running
    /// Where the next notch measures from: the aim while one is in flight, and the resting value otherwise.
    readonly property real at: send.running ? glide.aim : (glide.view ? glide.view.contentY : 0)

    /// Send what is under this notch to `value`. The caller has already clamped it — how far this surface may go is
    /// the surface's own answer, and it differs (a list has margins above and below its rows, a text box has not).
    function sendTo(value) {
        send.stop()
        glide.aim = value
        send.to = value
        send.start()
    }
    /// Something else is moving this view — an arrow key, a reveal, a restored reading position. **The notch loses**:
    /// it was aimed at rows the reader is no longer being shown, and left running it would drag them back.
    function halt() {
        send.stop()
    }

    property NumberAnimation send: NumberAnimation {
        id: send
        target: glide.view
        property: "contentY"
        duration: Metrics.wheelGlideMs
        // Chromium's own wheel easing — `cubic-bezier(0.42, 0, 0.58, 1)`, the CSS `ease-in-out`
        // (`scroll_offset_animation_curve.cc`). It leaves and arrives at rest, which is what makes a run of notches
        // read as one movement instead of a string of starts.
        easing.type: Easing.Bezier
        easing.bezierCurve: [0.42, 0, 0.58, 1, 1, 1]
    }
}
