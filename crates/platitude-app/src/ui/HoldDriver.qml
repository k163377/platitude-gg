import QtQuick

/// The press-count state machine every confirmed control shares (デザイン規約 §回数押し). The owner keeps its gestures and
/// its paint; this owns the count, the progress and the completion's phases.
QtObject {
    id: drive

    /// How many presses confirm. Zero disarms the control; every entry point checks it, so owners drive this
    /// unconditionally.
    property int holdPresses: 0
    /// Presses counted in the gesture under way.
    property int count: 0
    /// The fill and the mark, 0 to 1: a step of `1 / presses` per press, 1 once the last press lands.
    property real progress: 0
    /// The last press landed: the answer. **Every owner acts here**, while the gesture's latches still hold — what runs
    /// is what was pressed for, whatever has moved since. Nothing answering it may take the control away: an owner
    /// whose answer would (its row leaving the list) keeps the answer here and gives it at [`landed`].
    signal finished()
    /// The floor after the last press has passed (or, automated, right after [`finished`]): the full fill has been
    /// seen. Owners let go here — a menu closes, a question goes back up — and **decide nothing**: the latches are
    /// already released, so anything read now may belong to whatever replaced the gesture's target.
    signal landed()

    /// A hand or a key is down on the control.
    property bool pressed: false
    /// The last press has landed: the control is full and takes no more presses until it settles.
    property bool done: false
    /// Full after the last press: the fill steps down to the tone's dimmed step — the work is running.
    readonly property bool dimmed: drive.done && drive.progress >= 1
    /// The owner's work is still out (`ActionButton.busy`). Past the floor the full fill waits on it.
    property bool waiting: false
    /// The floor has passed since the last press.
    property bool floorPassed: false
    /// Past the floor and still waiting: the owner turns the ring in the mark's seat.
    readonly property bool loading: drive.done && drive.floorPassed && drive.waiting
    /// Before the floor: the full fill has to be seen, so nothing blanks it.
    readonly property bool settling: drive.done && !drive.floorPassed

    /// A gesture is under way — a hand down (a plain press too), presses counted, the fill moving, or the completion
    /// still on screen. The latches below hold through all of it.
    readonly property bool gesturing: drive.pressed || drive.count > 0 || drive.progress > 0 || drive.done

    /// The count the gesture under way was given; the live `holdPresses` between gestures. Owners whose wording or
    /// frame follows the same condition read this too (rules-refs/app-ui.md「回数押しか否かは最初の押下で確定する」).
    property int armedPresses: drive.holdPresses
    readonly property Binding arm: Binding {
        target: drive
        property: "armedPresses"
        value: drive.holdPresses
        when: !drive.gesturing
        restoreMode: Binding.RestoreNone
    }
    /// What the presses are aimed at besides their count (tab, branch, destination, list row). A control whose target
    /// can be swapped out between presses must set it, or the last press fires at the new target. Empty: nothing can
    /// re-point this control.
    property string premise: ""
    property string armedPremise: drive.premise
    readonly property Binding armTarget: Binding {
        target: drive
        property: "armedPremise"
        value: drive.premise
        when: !drive.gesturing
        restoreMode: Binding.RestoreNone
    }
    /// The count or the premise moved before the last press: the presses so far are dropped, and a release still under
    /// the hand answers nothing (owners read this before treating a release as a plain click). Latched until the next
    /// [`begin`].
    property bool stale: false
    onHoldPressesChanged: drive.noteSpoiled()
    onPremiseChanged: drive.noteSpoiled()
    function noteSpoiled() {
        if (drive.done || !drive.gesturing)
            return
        if (drive.holdPresses !== drive.armedPresses || drive.premise !== drive.armedPremise) {
            drive.stale = true
            drive.drop()
        }
    }
    /// The presses so far go, and the fill with them; a hand still down keeps the latches until it lets go, so its
    /// release is judged against the gesture it began.
    function drop() {
        drive.count = 0
        idle.stop()
        step.stop()
        back.stop()
        drive.progress = 0
    }
    /// Focus left between key presses: the count belongs to the keyboard gesture that made it. A pointer's count stays
    /// (it leaves with the pointer — [`leave`]).
    function focusLost() {
        if (!drive.byKey || drive.done || !drive.gesturing)
            return
        drive.stale = true
        drive.pressed = false
        drive.drop()
    }
    property bool byKey: false
    /// Set by [`runOut`]: the gesture had no hand behind it, and its floor is not waited out.
    property bool automated: false

    /// A press landed (a pointer's or a key's): count it. Presses after the last one are dropped until the control
    /// settles — the last press locks it.
    function begin() {
        drive.pressed = true
        drive.press()
    }
    /// One press counted, with no hand to track (`runOut`'s presses).
    function press() {
        if (drive.done)
            return
        if (drive.count === 0) {
            drive.stale = false
            drive.byKey = false
        }
        if (drive.armedPresses <= 0)
            return
        drive.count += 1
        back.stop()
        if (drive.count >= drive.armedPresses) {
            drive.complete()
            return
        }
        idle.restart()
        step.to = drive.count / drive.armedPresses
        step.duration = Metrics.holdStepMs
        step.restart()
    }
    /// The last press. **Nothing here touches the driver after its last signal**: a `landed` answer can take the
    /// control away with it (a list row's delegate going with the entry it removed).
    function complete() {
        drive.done = true
        idle.stop()
        const fire = !drive.stale
        if (drive.automated) {
            // Settled first: nothing is left to do after the signals. The run is synchronous, so nothing the latches
            // held can have moved before `finished` reads it.
            drive.settle()
            if (fire)
                drive.finished()
            drive.landed()
            return
        }
        step.to = 1
        // The last step is the quicker half: the full fill and its dimmed step are what this press is answered with.
        step.duration = Metrics.holdStepMs / 2
        step.restart()
        floor.restart()
        if (fire)
            drive.finished()
    }
    /// The hand or the key let go. Presses are counted as they land, so this only ends the press.
    function letUp() {
        drive.pressed = false
    }
    /// The pointer left the control part way: the count goes back to zero (デザイン規約 §回数押し).
    function leave() {
        if (drive.done || drive.count === 0)
            return
        drive.cancel()
    }
    /// Back to zero with the fill sliding back — the answer a gesture given up on gets.
    function cancel() {
        drive.count = 0
        idle.stop()
        step.stop()
        if (drive.progress > 0)
            back.restart()
    }
    /// Called off with nothing to slide back for (whatever was pressed has gone). A completion is left alone while its
    /// floor runs or its work is out — the control going deaf under its own work is what the full fill is there for.
    function blank() {
        drive.pressed = false
        if (drive.settling || (drive.done && drive.waiting))
            return
        drive.settle()
    }
    /// The completion is over: back to rest at once — the work has gone out, and nothing is left to say.
    function settle() {
        drive.count = 0
        drive.done = false
        drive.floorPassed = false
        drive.automated = false
        idle.stop()
        floor.stop()
        step.stop()
        back.stop()
        drive.progress = 0
    }
    onWaitingChanged: if (drive.done && drive.floorPassed && !drive.waiting) drive.settle()
    /// Automation: the whole gesture at once, with no hand behind it and no floor waited out.
    function runOut() {
        if (drive.done || drive.holdPresses <= 0)
            return
        drive.automated = true
        // Counted up front, and nothing after the loop: the last press settles the control and may take it away.
        const left = (drive.count === 0 ? drive.holdPresses : drive.armedPresses) - drive.count
        for (let i = 0; i < left; i++)
            drive.press()
    }
    /// The keys that stand in for the press (デザイン規約 §回数押し).
    function holdKey(key) {
        return key === Qt.Key_Space || key === Qt.Key_Return || key === Qt.Key_Enter
    }
    /// One of those keys, arriving fresh — auto-repeat is dropped on both edges (デザイン規約 §回数押し): a key held down
    /// would otherwise count presses nobody made. Owners with their own answer for the same key ask this too.
    function ownsKey(event) {
        return !event.isAutoRepeat && drive.holdKey(event.key)
    }
    /// A press from the keyboard, taking the event when the control is armed and the key is its own; answers whether it
    /// did. Accepting it keeps `AbstractButton` from reading Space as a press too, which would count it twice.
    function pressKey(event) {
        if (drive.holdPresses <= 0 || !drive.ownsKey(event))
            return false
        const fresh = drive.count === 0 && !drive.done
        drive.begin()
        if (fresh)
            drive.byKey = true
        event.accepted = true
        return true
    }
    /// The release, judged against [`armedPresses`]: a key taken at the press is answered here whatever changed since,
    /// or the control underneath reads it as a plain click.
    function releaseKey(event) {
        if (drive.armedPresses <= 0 || !drive.ownsKey(event))
            return false
        drive.letUp()
        event.accepted = true
        return true
    }

    readonly property NumberAnimation step: NumberAnimation {
        target: drive
        property: "progress"
        easing.type: Easing.OutCubic
    }
    readonly property NumberAnimation back: NumberAnimation {
        target: drive
        property: "progress"
        to: 0
        duration: Metrics.holdBackMs
        easing.type: Easing.OutCubic
    }
    /// No press for this long: the count goes back to zero.
    readonly property Timer idle: Timer {
        interval: Metrics.holdResetMs
        onTriggered: drive.cancel()
    }
    /// The full fill stays this long after the last press, whatever the work does.
    readonly property Timer floor: Timer {
        interval: Metrics.holdFloorMs
        // Settled before it says so, and nothing after: the answer may take the control away (`complete`).
        onTriggered: {
            drive.floorPassed = true
            if (!drive.waiting)
                drive.settle()
            drive.landed()
        }
    }
}
