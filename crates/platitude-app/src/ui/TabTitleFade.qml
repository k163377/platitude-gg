import QtQuick
import platitude.ui

// The run at a tab's far edge where the name goes quiet under the mark standing over it (デザイン規約 §タブの所作).
//
// The tab's own ground, laid over the name: nothing where the name is still read, whole from the mark's own room
// onwards — so what the mark stands on has gone rather than been crossed out. **Not a mask on the name**: the offscreen
// scene renders in software, where `ShaderEffect` — and the `MultiEffect` built on it — draws nothing at all, so a
// masked name would be missing from every headless picture, and from any machine whose Qt falls back to software
// (rules-refs/app-ui.md carries the measurement).
Rectangle {
    id: fade

    /// What lies under the name here: the tab's ground with whatever wash it is wearing already folded in (`Qt.tint`).
    /// A coat of anything else leaves a rectangle of the wrong colour standing at the tab's end.
    required property color ground
    /// How much of this run the name fades over; the rest of it is the mark's own room, laid whole so that the clear
    /// step either side of the ink comes out the same width (`TabMetrics.markRoomFull` / `fadeW`).
    required property real rampW
    readonly property real rampEnd: fade.width > 0 ? Math.min(1, fade.rampW / fade.width) : 0

    gradient: Gradient {
        orientation: Gradient.Horizontal
        // The same ground at none of itself rather than `transparent`, which is a transparent **black** and takes the
        // ramp through a colour the tab has not got on its way out (デザイン規約 §色).
        GradientStop { position: 0; color: Qt.rgba(fade.ground.r, fade.ground.g, fade.ground.b, 0) }
        GradientStop { position: fade.rampEnd; color: fade.ground }
        GradientStop { position: 1; color: fade.ground }
    }
}
