import QtQuick

/// The press-count state machine every confirmed control shares (デザイン規約 §回数押し). The owner keeps its gestures and
/// its paint; this owns the count, the progress and the completion's phases.
///
/// A press counts the moment it lands, so quick presses go through (user decision — ask before changing): no wait
/// after the first — a control that needs a pause feels like click, mode switch, double-click, and the quick second
/// press is swallowed. Counted on the input side, never waiting for the paint. `AbstractButton` delivers a fast second
/// press as a press and adds the double-click on another path (`qquickabstractbutton.cpp` `handlePress` /
/// `mouseDoubleClickEvent`), so every part counts one per press. The count goes back to zero when the pointer leaves
/// ([`leave`]) or after `holdResetMs` with no press (`idle`) — both sliding the fill back over `holdBackMs` — and at
/// once when focus leaves a keyboard count ([`focusLost`]). The first press fixes the count and the target; the last
/// locks.
///
/// The fill stops part way at each press: full at the first, it reads as "did it run?". At the last it fills, takes
/// the tone's dimmed step at once ([`dimmed`] — the work is running), holds `holdFloorMs` so the full fill is seen, and
/// past the floor the owner turns the ring in the mark's seat while its work is out ([`loading`]); then zero at once.
///
/// What runs is decided at the last press and runs there ([`finished`]). An answer arriving during the floor (not
/// merged, refused), a swapped target or a closed menu does not change it: answered after the floor, a consent to a
/// plain delete would run as a forced one, and a card reopened on another target would take the old confirmation.
/// Owners take one of three shapes. One that waits on its own work (a toolbar button, an exit card's row) runs it under
/// the full fill and turns to the ring past the floor; the diff's `Discard hunk` does not wait — it settles at the
/// floor and the hunk goes with the diff's reread. One whose leaving is all that waits (a menu row's menu closing, the
/// ask bar's pill going up) leaves at [`landed`]; the work runs behind and its progress is said where it lands (graph,
/// left menu, log). One whose answer takes it off the screen at once (the settings avatar's `Remove`, the plan's
/// `Discard` and run) accepts at the last press — its content fixed there, and through the floor it takes no edit, swap
/// or other button — and gives that content once at [`landed`]. It calls the answer off only when the target expired
/// (the plan closed: the tip moved, an operation started, the tab moved to another worktree; the person's picture
/// changed or went), never because the control went unpressable or busy, which the operation itself mostly causes; an
/// expired answer neither comes back nor lands on another target.
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
    /// [`begin`]: compared with the live values instead, a target that went and came back would revive the gesture.
    /// What decides `holdPresses` moves in the background (a fetch's answer, the reach walk, git's refusal), so read
    /// between presses it would change the presses' meaning half way (`tst_holdlatch`).
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
    /// The pointer left the control part way: the count goes back to zero.
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
    /// Automation: the whole gesture at once, with no hand behind it and no floor waited out — `finished` and
    /// `landed` are both out before it returns, so a verb that compares a count across the write counts before
    /// calling it (`avatar-remove`).
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
    /// One of those keys, arriving fresh — auto-repeat is dropped on both edges: a held key repeats its press on every
    /// platform and its release on some, and one key held down would count presses nobody made and press it through.
    /// Owners with their own answer for the same key ask this too.
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
