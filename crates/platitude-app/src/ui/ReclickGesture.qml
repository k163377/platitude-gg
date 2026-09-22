import QtQuick
import QtQuick.Controls.Fusion

// The two clicks a row answers with one gesture: the first does what a click does, and a second one on the same target
// — once a double-click has been ruled out — asks for the name (デザイン規約 §左メニューの所作「行き先はダブルクリック、名前は間を空けた
// 2 回目のクリック」).
//
// Held in one place because three surfaces answer the same gesture: the sidebar's rows, the graph's rows, and the
// names a graph chip unstacks. A surface that worked the wait out for itself would be a second answer to "how long is
// a double-click", and the whole of the gesture is that answer.
//
// **One of these per surface.** What was clicked last, the wait a second click opened, and what that click was
// aimed at all outlive the row they were made on: the graph pools its delegates on every pass, and a wait that
// travelled with the row would either go down with it (nothing happens, and nothing says why) or come back on a
// recycled row and name whatever commit that row is now showing. Only one target can be waiting at a time, which
// is exactly what one of these holds.
QtObject {
    id: gesture

    /// The target the last click landed on. Written here and nowhere else — a surface that wrote it itself would set
    /// it before the next click could read what it was.
    property string activeKey: ""
    /// The target of the wait now running, and what that click was aimed at — the caller's own value, handed back
    /// untouched when the wait runs out (a chip for the graph, the row's own fields for the sidebar).
    /// **Read at the click**: by the time the wait ends the row may be showing something else.
    property string armedKey: ""
    property var armedNames: null

    /// The wait ran out with no second half to the double-click: this target is being named.
    signal renameAsked(string key, var names)

    /// Whether the window a click opened is still running, and whether a given target is holding the wait the name box
    /// opens after. What a headless run reads to put its second click in as a second (app-ui.md §UI 自動化の因果性).
    ///
    /// **`guarded` is the surface's window**: only a click on the target the window belongs to is dropped (see
    /// `click`), so a run waiting on this waits a little longer than it strictly has to — which is the safe way
    /// round for a run.
    readonly property bool guarded: doubleGuard.running
    /// Whether any target is waiting. What the surface holds still while it runs: a wait is a beat with nothing to
    /// show for it, and anything that opened or closed under it would be the answer to a different question
    /// (by design — the hover cards were changing under the wait).
    readonly property bool armed: renameTimer.running
    function armedFor(key) {
        return renameTimer.running && gesture.armedKey === key
    }

    /// A left press landed. Answers whether it is a press of its own — the second one of a double-click is not, the
    /// gesture there being the double — so the caller does what a press does only when this says so.
    ///
    /// **Both halves of a double-click come here, and both come as presses**: Qt raises the second press before it
    /// raises the double and withholds the second click altogether (measured, `tst_pressorder`). A surface that
    /// answered at the release would never see the second half at all — which is why the guard below is what stops
    /// the second press from selecting again and arming a name box under a hand that is going somewhere.
    ///
    /// `names` is what a second press on this target would name, or nothing where there is no name to change (a
    /// folder, a marker, a row with no ref on it).
    ///
    /// The arming is decided here, before the caller has answered the press: what makes a second press a second is
    /// that the target was already the one pressed, and the caller's own answer is what makes it that.
    function click(key, names) {
        // The second click of a double-click, dropped — **but only on the target the first one landed on**. One of
        // these serves a whole surface, so a guard that answered for all of it would swallow the second of two quick
        // clicks on *different* rows, which is a hand reading down a list (the per-row timers this replaced could
        // not make that mistake).
        if (doubleGuard.running && key === gesture.activeKey)
            return false
        const window = Application.styleHints.mouseDoubleClickInterval
        const wasActive = key !== "" && key === gesture.activeKey
        gesture.activeKey = key
        doubleGuard.interval = window
        doubleGuard.restart()
        renameTimer.stop()
        if (wasActive && names) {
            gesture.armedKey = key
            gesture.armedNames = names
            // **Qt measures the double-click press to press, and this is a press**, so the whole window is still to
            // come: a press landing after it is no longer a double-click whatever this does.
            renameTimer.interval = window
            renameTimer.start()
        }
        return true
    }
    /// The second click came inside the window after all: the gesture was the double-click, and the box it was about
    /// to open is not what was meant.
    function drop() {
        renameTimer.stop()
    }
    /// The surface these rows are on has gone (a peek closed, a card came down). The memory goes with it: kept, the
    /// next visit's first click would come up as a second one (app-ui.md).
    function forget() {
        renameTimer.stop()
        gesture.activeKey = ""
    }

    // Long enough that the second click of a double-click falls inside it; the system's own setting, since it is the
    // system that decides what counts as a double-click.
    property Timer doubleGuard: Timer {
        interval: Application.styleHints.mouseDoubleClickInterval
    }
    // A second click on a target already clicked means the name, but only once a double-click can be ruled out — the
    // same wait Explorer makes (デザイン規約 §左メニューの所作).
    property Timer renameTimer: Timer {
        interval: Application.styleHints.mouseDoubleClickInterval
        onTriggered: gesture.renameAsked(gesture.armedKey, gesture.armedNames)
    }
}
