import QtQuick
import platitude.ui

// The `Canvas` every hand-drawn mark in this window is: it does its own drawing exactly as a bare one would, and says
// whether it has been drawn yet (`Ink`). Written once here, because what a picture of a
// half-built scene loses is **the canvases and nothing else** — a name's text node is built in the polish that creates
// its item, and a canvas cannot draw before the turn after (`Ink`).
//
// A mark owes its ink only while it could be in a picture at all. A canvas with no window — a menu nobody has opened —
// or with no size never paints. Hidden marks (including a hidden parent) owe nothing either.
//
// **`onPaint` stays the caller's**: a signal handler written in the file that uses this one replaces the handler here,
// so the drawing is hung off `painted()`, which Qt emits after the caller's own has run.
Canvas {
    id: canvas

    /// Whether this mark has been put on screen once.
    property bool inked: false
    /// Whether a picture taken now would be missing it.
    readonly property bool owing: !canvas.inked && canvas.visible && canvas.width > 0 && canvas.height > 0
                                  && canvas.Window.window !== null
    /// What the tally was last told, so each mark is counted once and let go once.
    property bool counted: false

    /// Bring the tally in line with this mark. Called from every edge that can move `owing`, including the last one:
    /// a row scrolled off before its mark was drawn takes the debt with it, and a debt nobody can pay would hold every
    /// picture after it.
    function settle() {
        if (canvas.owing === canvas.counted)
            return
        canvas.counted = canvas.owing
        Ink.owed += canvas.counted ? 1 : -1
    }

    onOwingChanged: canvas.settle()
    onPainted: canvas.inked = true
    // A mark born with its size already on it never moves `owing` afterwards, so the first reading is taken here as
    // well. Saying it twice costs nothing — `settle` compares before it counts.
    Component.onCompleted: canvas.settle()
    Component.onDestruction: {
        canvas.inked = true
        canvas.settle()
    }
}
