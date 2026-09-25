import QtQuick

/// The long-press state machine every held control shares (デザイン規約 §長押し). The owner keeps its gestures and its
/// paint; this owns only the progress and the two animations.
QtObject {
    id: drive

    /// How long the press has to last. Zero disarms the hold; every entry point checks it, so owners drive this
    /// unconditionally.
    property int holdMs: 0
    /// How far into the hold the press has got, 0 to 1.
    property real progress: 0
    /// Held all the way down.
    signal finished()

    /// A gesture (`gesturing`) is the press plus the fill's slide back after a short one; the latches below hold
    /// through both.
    property bool pressed: false
    readonly property bool gesturing: drive.pressed || drive.progress > 0

    /// The length the press under way was given; the live `holdMs` between presses. Owners whose wording or frame
    /// follows the same condition read this too (rules-refs/app-ui.md「長押しか否かは押した瞬間に確定する」).
    property int armedMs: drive.holdMs
    readonly property Binding arm: Binding {
        target: drive
        property: "armedMs"
        value: drive.holdMs
        when: !drive.gesturing
        restoreMode: Binding.RestoreNone
    }
    /// What the press is aimed at besides its length, as one string (tab, branch, destination, list row). A control
    /// whose target can be swapped out from under the hand must set it, or the hold fires at the new target. Empty:
    /// nothing can re-point this control.
    property string premise: ""
    property string armedPremise: drive.premise
    readonly property Binding armTarget: Binding {
        target: drive
        property: "armedPremise"
        value: drive.premise
        when: !drive.gesturing
        restoreMode: Binding.RestoreNone
    }
    /// The length or the premise moved during the gesture: `finished` is not raised and the release answers nothing.
    /// The gesture is left to run out — blanking it here would hand the release back to the live answer. Latched
    /// until [`begin`]: compared with the live values instead, a tab switched away and back would revive the press.
    property bool stale: false
    onHoldMsChanged: drive.noteSpoiled()
    onPremiseChanged: drive.noteSpoiled()
    function noteSpoiled() {
        if (drive.gesturing && (drive.holdMs !== drive.armedMs || drive.premise !== drive.armedPremise))
            drive.stale = true
    }
    /// Focus left mid-press: the key release goes wherever the focus is, so the fill would run out and fire. Keyboard
    /// gestures only — a pointer keeps its grab, and blanking it would call off a hold nobody let go of.
    function focusLost() {
        if (!drive.byKey || !drive.gesturing)
            return
        drive.stale = true
        drive.blank()
    }
    property bool byKey: false

    /// A press landed, or automation stands in for one: fill from zero.
    function begin() {
        drive.stale = false
        drive.byKey = false
        drive.pressed = true
        if (drive.armedMs <= 0)
            return
        back.stop()
        fill.restart()
    }
    /// The press let go or slid off. **Callers read [`armedMs`] and [`stale`] first**: for a press that left no fill
    /// the latch opens here, before the click of the same release arrives.
    function letUp() {
        drive.pressed = false
        fill.stop()
    }
    /// Called off without the hand letting go (whatever was held has gone): blank instead of sliding back.
    function blank() {
        drive.pressed = false
        fill.stop()
        back.stop()
        drive.progress = 0
    }
    /// The keys that stand in for the press (デザイン規約 §長押し).
    function holdKey(key) {
        return key === Qt.Key_Space || key === Qt.Key_Return || key === Qt.Key_Enter
    }
    /// One of those keys, arriving fresh — auto-repeat is dropped on both edges (デザイン規約 §長押し). Owners with their
    /// own answer for the same key ask this too.
    function ownsKey(event) {
        return !event.isAutoRepeat && drive.holdKey(event.key)
    }
    /// Starts the hold from the keyboard, taking the event when the hold is armed and the key is its own; answers
    /// whether it did. Accepting it keeps `AbstractButton` from reading Space as a press too, which would drive the
    /// fill from `down` a second time.
    function pressKey(event) {
        if (drive.holdMs <= 0 || !drive.ownsKey(event))
            return false
        drive.begin()
        drive.byKey = true
        event.accepted = true
        return true
    }
    /// The release, judged against [`armedMs`]: a key taken at the press is answered here whatever changed since, or
    /// the control underneath reads it as a plain click.
    function releaseKey(event) {
        if (drive.armedMs <= 0 || !drive.ownsKey(event))
            return false
        drive.letUp()
        event.accepted = true
        return true
    }

    readonly property NumberAnimation fill: NumberAnimation {
        target: drive
        property: "progress"
        from: 0
        to: 1
        duration: Math.max(drive.armedMs, 1)
        // Stopped short: slide back — the only answer a plain click gets. Run out: blank (デザイン規約 §長押し).
        onStopped: {
            if (drive.progress >= 1)
                drive.progress = 0
            else if (drive.progress > 0)
                back.restart()
        }
        onFinished: if (!drive.stale) drive.finished()
    }
    readonly property NumberAnimation back: NumberAnimation {
        target: drive
        property: "progress"
        to: 0
        duration: Metrics.holdBackMs
        easing.type: Easing.OutCubic
    }
}
