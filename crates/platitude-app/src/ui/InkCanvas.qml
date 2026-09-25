import QtQuick
import platitude.ui

// The root of every hand-drawn mark: a `Canvas` that owes `Ink` until it first paints, only while it could paint
// (rules-refs/app-ui.md「`Canvas` は生まれたフレームにインクを持たない」). `onPaint` is the caller's, so this hangs off
// `painted()`.
Canvas {
    id: canvas

    /// Whether this mark has been put on screen once.
    property bool inked: false
    /// Whether a picture taken now would be missing it.
    readonly property bool owing: !canvas.inked && canvas.visible && canvas.width > 0 && canvas.height > 0
                                  && canvas.Window.window !== null
    /// What the tally was last told, so each mark is counted once and let go once.
    property bool counted: false

    /// Brings the tally in line with `owing`. Called on every edge that moves it, destruction included — an unpaid
    /// debt would hold every later picture.
    function settle() {
        if (canvas.owing === canvas.counted)
            return
        canvas.counted = canvas.owing
        Ink.owed += canvas.counted ? 1 : -1
    }

    onOwingChanged: canvas.settle()
    onPainted: canvas.inked = true
    // A mark born with its size never moves `owing`, so the first reading is taken here.
    Component.onCompleted: canvas.settle()
    Component.onDestruction: {
        canvas.inked = true
        canvas.settle()
    }
}
