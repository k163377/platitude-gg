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
    // A mark never painted draws itself as it is shown. Qt asks a canvas for its first picture once, as it enters a
    // window, and that ask is spent on a canvas with no size; a size arriving later asks again only of a canvas that
    // is visible or read by an effect (`qquickcanvasitem.cpp` — `sceneGraphInitialized` / `geometryChange`). A list
    // builds the rows past its edge in the background, a time slice at a go, and such a row has its window before
    // its bindings have run, sizes last of all: when a slice ends in between and the ask runs in the gap, the marks
    // hidden on that row — a chip's cloud or tree on a commit with no name — are left unpainted, and the row later
    // shows a commit that carries one. Only the unpainted: a pane coming back from behind another shows every mark
    // in it at once, and those hold their pictures.
    onVisibleChanged: if (canvas.visible && !canvas.inked) canvas.requestPaint()
    onPainted: canvas.inked = true
    // A mark born with its size never moves `owing`, so the first reading is taken here.
    Component.onCompleted: canvas.settle()
    Component.onDestruction: {
        canvas.inked = true
        canvas.settle()
    }
}
