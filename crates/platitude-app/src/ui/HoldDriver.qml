import QtQuick

/// The long-press state machine every held control shares (デザイン規約 §長押し): the fill climbs while the press lasts, slides
/// back out when the press stops short, and blanks once it has fired. The owner keeps its gestures and its paint — this
/// owns only the progress and the two animations, so the holds in the app cannot drift apart.
QtObject {
    id: drive

    /// How long the press has to last. Zero disarms the hold, and every entry point below checks it, so owners drive
    /// this unconditionally.
    property int holdMs: 0
    /// How far into the hold the press has got, 0 to 1.
    property real progress: 0
    /// Held all the way down. What that runs is the owner's to say.
    signal finished()

    /// A gesture is under way: the press itself, and the fill's own tail after it — a press that stopped short slides
    /// back out, and the shape holds under the hand while it does.
    property bool pressed: false
    readonly property bool gesturing: drive.pressed || drive.progress > 0

    /// **The length the press under way was given** (デザイン規約 §長押し「長押しか否かは押した瞬間に確定」). `holdMs` is
    /// worked out from state that moves on its own — a fetch answering, a check coming back, git refusing — so read
    /// live it changes mid-press: a hold begun on a red button came back as a click and ran the plain command, and a
    /// click begun on a plain one was dropped altogether when the button turned into a hold under the hand.
    ///
    /// Declared with the live value so it reads right from the start; the `Binding` is what keeps it, dropped while a
    /// gesture is under way with nothing put back after (`RestoreNone`) — which is the freeze itself. **Owners whose
    /// wording or frame is worked out from the same condition read this too**, so nothing about the control changes
    /// under the hand (`ActionButton.armedMs` / `AppMenuItem.armedMs`).
    property int armedMs: drive.holdMs
    readonly property Binding arm: Binding {
        target: drive
        property: "armedMs"
        value: drive.holdMs
        when: !drive.gesturing
        restoreMode: Binding.RestoreNone
    }
    /// **What the press is against besides its length**, as one string: the tab, the branch, where it is going, which
    /// row of a list it is standing on. The length says which command and how much confirmation it asks for; **what
    /// that command is aimed at is this one's**, and a control whose target can be swapped out from under the hand
    /// has to say so here. A band button re-pointed at another tab arms the very same length and sends that tab's
    /// branch (measured, `tst_holdlatch`); a list delegate re-used for another row runs on whatever it is pointed at
    /// when the fill runs out. Empty is "the length is the whole of it", which is what a control nothing can re-point
    /// says.
    property string premise: ""
    property string armedPremise: drive.premise
    readonly property Binding armTarget: Binding {
        target: drive
        property: "armedPremise"
        value: drive.premise
        when: !drive.gesturing
        restoreMode: Binding.RestoreNone
    }
    /// **The premise the press was made under has gone** — the length it was given, or what it was aimed at. A gesture
    /// that outlives either has nothing left to mean, so the owner's signal is not raised at all and the release
    /// answers nothing either (デザイン規約 §長押し). The gesture itself is left to run out: blanking it here would
    /// hand the release back to the live answer, which is the one answer it is held apart from.
    ///
    /// **Latched.** Read as a comparison with what is true now, a target that went and came back put
    /// the gesture back in business — a tab switched away from and switched back to answered the press as though
    /// nothing had happened (measured). What was lost stays lost until the next press, which is what [`begin`]
    /// clears.
    property bool stale: false
    onHoldMsChanged: drive.noteSpoiled()
    onPremiseChanged: drive.noteSpoiled()
    function noteSpoiled() {
        if (drive.gesturing && (drive.holdMs !== drive.armedMs || drive.premise !== drive.armedPremise))
            drive.stale = true
    }
    /// **The hand let go somewhere this control cannot hear.** A key press is answered by a key release, and a
    /// release goes wherever the focus is — so a press made here and let go after the focus moved leaves the fill
    /// running, and it runs out and fires (measured, `tst_holdlatch`). Only the keyboard's gesture: a pointer keeps
    /// its grab wherever the focus goes, and blanking that one would call off a hold nobody let go of.
    function focusLost() {
        if (!drive.byKey || !drive.gesturing)
            return
        drive.stale = true
        drive.blank()
    }
    /// Whether the gesture under way was started from the keyboard ([`pressKey`]).
    property bool byKey: false

    /// A press landed — or automation stands in for one: fill from zero.
    ///
    /// The latch closes first, so everything from here to the release reads the length this press was given.
    function begin() {
        drive.stale = false
        drive.byKey = false
        drive.pressed = true
        if (drive.armedMs <= 0)
            return
        back.stop()
        fill.restart()
    }
    /// The press let go, or slid off: stop filling. The animation's own onStopped decides between sliding back and
    /// blanking.
    ///
    /// **Whoever calls this reads [`armedMs`] and [`stale`] first**: the latch opens here for a press that left no
    /// fill behind it, and the click that follows a release arrives in the same delivery.
    function letUp() {
        drive.pressed = false
        fill.stop()
    }
    /// Called off without the hand letting go — whatever was held has gone. Blanks the fill:
    /// the slide is an answer to a press that stopped short, and there is nothing left here for it to be an
    /// answer about.
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
    /// Whether a key event is one of those arriving fresh. Auto-repeat is dropped on both edges: a held key repeats its
    /// press on every platform and its release on some, and either edge would restart the fill from zero for as long as
    /// the key was down — the hold could then never complete. Owners with their own answer for the same key read this
    /// to ask it the same question the hold does.
    function ownsKey(event) {
        return !event.isAutoRepeat && drive.holdKey(event.key)
    }
    /// The hold's other hand: focus the control, then hold Space or Enter. Fills from zero and takes the event when the
    /// hold is armed and the key is its own — and answers whether it did, so an owner that also has a use for the key
    /// can tell "held" from "not ours". Accepting it is what keeps `AbstractButton` from reading Space as a press as
    /// well, which would drive the same fill from `down` a second time.
    function pressKey(event) {
        if (drive.holdMs <= 0 || !drive.ownsKey(event))
            return false
        drive.begin()
        drive.byKey = true
        event.accepted = true
        return true
    }
    /// And the release. **Against the length the press was given**: a key taken at the press
    /// has to be answered here whatever has happened since, or the control underneath reads the release as a plain
    /// one and runs the click a hold was being made instead of.
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
        // The length this press was given, which is what the fill is drawing out (see `armedMs`).
        duration: Math.max(drive.armedMs, 1)
        // A press that stopped short slides back out: a held control reports no click at all, so
        // without this the only answer to a plain click is nothing happening (デザイン規約 §長押し). Pressed all the way
        // through, it has already fired and there is nothing left to say — that one blanks.
        onStopped: {
            if (drive.progress >= 1)
                drive.progress = 0
            else if (drive.progress > 0)
                back.restart()
        }
        // **Nothing is raised for a premise that has gone** (see `stale`). The fill ran out under a hand that was
        // holding for one command and would be answered with another, so the gesture ends here and the control goes
        // back to reading what is true now.
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
