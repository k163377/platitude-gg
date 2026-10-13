import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The first click does what a click does; a second one on the same target, once a double-click is ruled out, asks
// for the name (デザイン規約 §左メニューの所作「行き先はダブルクリック」) — **two clicks there in a row**: anything
// between them (`Hand.acted` — a press elsewhere, the history landing on a commit, the window left) makes the next
// click a first one (`broken`), and so does a double-click, which spends them (`drop`). One component for the
// sidebar's rows, the graph's rows and a chip's unstacked names, so there is one answer to "how long is a
// double-click".
//
// **One of these per surface, never per row**: the graph pools its delegates, so a wait held by a row either dies
// with it or comes back on a recycled row naming another commit.
QtObject {
    id: gesture

    /// The target the last click landed on. Written only here: a surface that wrote it would set it before the next
    /// click could read it.
    property string activeKey: ""
    /// The target of the running wait and the caller's value for it, handed back untouched by `renameAsked`. Taken
    /// at the click: by the time the wait ends the row may show something else.
    property string armedKey: ""
    property var armedNames: null
    /// Something came between the last click and now (`settle`): the next click is a first one wherever it lands, the
    /// target last clicked included (デザイン規約 §左メニューの所作「間を空けた 2 回目」). `activeKey` stays — the
    /// sidebar marks the row last clicked with it, and that mark does not move.
    property bool broken: false
    /// The input under way (`Hand.acted`) and whether a target here took it (`take`). Read once it is through
    /// (`settle`): the window hears a press before the target answers it, a card after.
    property bool heard: false
    property bool taken: false

    /// The wait ran out with no second half to the double-click: this target is being named.
    signal renameAsked(string key, var names)

    /// Whether the double-click window a click opened is still running: what a headless run waits out before its
    /// second click. It is the surface's window though `click` drops only a second click on the same target, so a
    /// run waits a little longer than it has to (the safe side).
    readonly property bool guarded: doubleGuard.running
    /// Whether any target is waiting. The surface opens and closes nothing while it runs: anything that changed under
    /// the wait would read as the answer to the click.
    readonly property bool armed: renameTimer.running
    function armedFor(key) {
        return renameTimer.running && gesture.armedKey === key
    }

    /// A left click landed — at the press on most targets, at the release on the left menu's rows (which claimed the
    /// press with `take`). Answers whether it is a click of its own (the second press of a double-click is not), so
    /// the caller does what a click does only when this says true. `names` is what a second click here would name,
    /// or null where there is no name to change (a folder, a marker, a row with no ref).
    ///
    /// Both halves of a double-click arrive as presses — Qt withholds the second click (`tst_pressorder`) — so the
    /// guard below is what stops the second press selecting again and arming a name box. Arming is decided before
    /// the caller answers the press, since that answer is what makes this target the one already pressed.
    function click(key, names) {
        // The rest of a gesture that began on something standing over this surface (`hush`).
        if (hushTimer.running)
            return false
        gesture.take()
        // The second click of a double-click, dropped only on the first one's target: a guard for the whole surface
        // would swallow a quick click on a different row.
        if (doubleGuard.running && key === gesture.activeKey)
            return false
        const window = Application.styleHints.mouseDoubleClickInterval
        const again = key !== "" && key === gesture.activeKey && !gesture.broken
        gesture.activeKey = key
        gesture.broken = false
        doubleGuard.interval = window
        doubleGuard.restart()
        renameTimer.stop()
        if (again && names) {
            gesture.armedKey = key
            gesture.armedNames = names
            // Qt measures a double-click press to press, so from this press the whole window is still to come.
            renameTimer.interval = window
            renameTimer.start()
        }
        return true
    }
    /// A press inside this target that was not its own click (a line the open row put out —
    /// `SidebarRowGestures.followLine`): it becomes the target last clicked and arms nothing, so the next click on
    /// the row itself counts as the second.
    function land(key) {
        gesture.take()
        renameTimer.stop()
        doubleGuard.stop()
        gesture.activeKey = key
        gesture.broken = false
    }
    /// The input under way is a target's here. `click` and `land` take their own; **a target that answers at the
    /// release takes its press at the press and hears its release** (`NavItemDelegate.takePress` / `endPress`) — by
    /// its click the press is long settled, and a press that makes no click is something between two clicks.
    function take() {
        gesture.taken = true
        Qt.callLater(gesture.settle)
    }
    /// An input happened (`Hand.acted`, or a release the target heard itself): unless a target here takes it before it
    /// is through, it came between this target's clicks.
    function hear() {
        gesture.heard = true
        Qt.callLater(gesture.settle)
    }
    /// Once the input is through: one that no target here took came between this target's clicks.
    function settle() {
        if (gesture.heard && !gesture.taken) {
            gesture.broken = true
            // Qt counts a double-click only between two presses in a row.
            doubleGuard.stop()
        }
        gesture.heard = false
        gesture.taken = false
    }
    /// A click on something standing over this surface took it down (`RefListPopup.followed`): the second press of
    /// that double-click would fall on whatever is under the hand, and a double-click here moves the worktree.
    /// For one double-click window every press here is ignored (`hushed`, which the surface's double-click reads too).
    function hush() {
        renameTimer.stop()
        hushTimer.interval = Application.styleHints.mouseDoubleClickInterval
        hushTimer.restart()
    }
    readonly property bool hushed: hushTimer.running
    /// A double-click (the second click came inside the window after all) or a click building a choice: no name box,
    /// and the two clicks are spent — the next one is a first one.
    function drop() {
        renameTimer.stop()
        gesture.broken = true
    }
    /// The gesture is spent (a peek closed, a name box came down): kept, the memory would make the next first click
    /// a second one.
    function forget() {
        renameTimer.stop()
        gesture.activeKey = ""
    }

    property Connections handWatch: Connections {
        target: Hand
        function onActed() {
            gesture.hear()
        }
    }
    // The system decides what counts as a double-click.
    property Timer doubleGuard: Timer {
        interval: Application.styleHints.mouseDoubleClickInterval
    }
    property Timer hushTimer: Timer {
        interval: Application.styleHints.mouseDoubleClickInterval
    }
    property Timer renameTimer: Timer {
        interval: Application.styleHints.mouseDoubleClickInterval
        onTriggered: gesture.renameAsked(gesture.armedKey, gesture.armedNames)
    }
}
