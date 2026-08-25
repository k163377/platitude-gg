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
// **What was clicked last is not held here.** A delegate is recycled the moment its row scrolls off, so the memory
// belongs to whatever outlives it (the sidebar's `SidebarRowGestures`, the graph's list, the chip's card) — this reads
// it back through `activeKey` and never writes it.
QtObject {
    id: gesture

    /// What this target answers to. Compared against `activeKey`, so the two have to be written in the same shape.
    property string key: ""
    /// The key the last click landed on, held by whoever outlives the delegates.
    property string activeKey: ""
    /// Whether this target has a name that can be changed at all. A folder is not one, and neither is a row with
    /// nothing on it to name.
    property bool nameable: false

    /// The wait ran out with no second half to the double-click: this target is being named.
    signal renameAsked()

    /// Whether this target is holding the wait the name box opens after, and whether a click landing now would still
    /// be counted as the other half of a double-click. What a headless run reads to see the gesture armed, and to know
    /// when a second click of its own counts as a second (app-ui.md §UI 自動化の因果性).
    readonly property bool armed: renameTimer.running
    readonly property bool guarded: doubleGuard.running

    /// A left click landed. Answers whether it is a click of its own — the second one of a double-click is not, the
    /// gesture there being the double — so the caller does what a click does only when this says so.
    ///
    /// The arming is decided here, before the caller has answered the click: what makes a second click a second is
    /// that the target was already the one clicked, and the caller's own answer is what makes it that.
    function click() {
        if (doubleGuard.running)
            return false
        const wasActive = gesture.key !== "" && gesture.key === gesture.activeKey
        doubleGuard.restart()
        if (wasActive && gesture.nameable)
            renameTimer.restart()
        return true
    }
    /// The second click came inside the window after all: the gesture was the double-click, and the box it was about
    /// to open is not what was meant.
    function drop() {
        renameTimer.stop()
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
        onTriggered: gesture.renameAsked()
    }
}
