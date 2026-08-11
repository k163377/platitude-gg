import QtQuick

/// The long-press state machine every held control shares (デザイン規約
/// §長押し): the fill climbs while the press lasts, slides back out when
/// the press stops short, and blanks once it has fired. The owner keeps
/// its gestures and its paint — this owns only the progress and the two
/// animations, so the holds in the app cannot drift apart.
QtObject {
    id: drive

    /// How long the press has to last. Zero disarms the hold, and every
    /// entry point below checks it, so owners drive this unconditionally.
    property int holdMs: 0
    /// How far into the hold the press has got, 0 to 1.
    property real progress: 0
    /// Held all the way down. What that runs is the owner's to say.
    signal finished()

    /// A press landed — or automation stands in for one: fill from zero.
    function begin() {
        if (drive.holdMs <= 0)
            return
        back.stop()
        fill.restart()
    }
    /// The press let go, or slid off: stop filling. The animation's own
    /// onStopped decides between sliding back and blanking.
    function letUp() {
        fill.stop()
    }
    /// Called off without the hand letting go — whatever was held has
    /// gone. Blanks the fill rather than sliding it back: the slide is an
    /// answer to a press that stopped short, and there is nothing left
    /// here for it to be an answer about.
    function blank() {
        fill.stop()
        back.stop()
        drive.progress = 0
    }
    /// The keys that stand in for the press (デザイン規約 §長押し).
    function holdKey(key) {
        return key === Qt.Key_Space || key === Qt.Key_Return
                || key === Qt.Key_Enter
    }

    readonly property NumberAnimation fill: NumberAnimation {
        target: drive
        property: "progress"
        from: 0
        to: 1
        duration: Math.max(drive.holdMs, 1)
        // A press that stopped short slides back out instead of blanking:
        // a held control reports no click at all, so without this the
        // only answer to a plain click is nothing happening (デザイン規約
        // §長押し). Pressed all the way through, it has already fired and
        // there is nothing left to say — that one blanks.
        onStopped: {
            if (drive.progress >= 1)
                drive.progress = 0
            else if (drive.progress > 0)
                back.restart()
        }
        onFinished: drive.finished()
    }
    readonly property NumberAnimation back: NumberAnimation {
        target: drive
        property: "progress"
        to: 0
        duration: Metrics.holdBackMs
        easing.type: Easing.OutCubic
    }
}
