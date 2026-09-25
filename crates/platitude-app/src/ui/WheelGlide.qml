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
