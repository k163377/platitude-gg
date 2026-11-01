import QtQuick
import platitude.ui

// The root of every hand-drawn mark: a `Canvas` that owes `Ink` until it first paints, only while it could paint
// (rules-refs/app-ui.md「`Canvas` は生まれたフレームにインクを持たない」). `onPaint` is the caller's, so this hangs off
// `painted()`.
//
// A canvas has no ink in the frame it is born: its 2D context is made by a queued call, so the first `onPaint` comes
// one turn later, and a picture taken in between shows the rows' names with the marks' seats empty. A headless run
// falls in that gap by construction — rows are built in the turn the verb completes, and `AutoShotDriver` shoots
// there — so `AutoShotDriver` drops a frame that still owes ink and takes it again at `Ink.owed === 0` (`ink=` in its
// line is how many it waited for). Only the first paint is counted: a mark that paints every frame would never let a
// picture through. Only a canvas in a window and with a size is counted: any other is never painted, and counting it
// would send every run to the watchdog.
Canvas {
    id: canvas

    /// Whether this mark has been put on screen once.
    property bool inked: false
    /// Whether a picture taken now would be missing it. `visible` is the effective one: a mark under a hidden parent
    /// (a `NavIcon` / `ChangeIcon` in a part not shown) is never painted, and counted it would hold the picture.
    /// Hidden, it lets go of its debt; shown again before its first paint, it owes again (`tst_ink.qml`).
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
    // shows a commit that carries one. Only the unpainted: a pane coming back from behind another (`centreStack`
    // bringing the graph back from under a diff) shows every mark in it at once, and those hold their pictures. Not
    // per part either: every mark goes through here.
    //
    // Where the gap comes from: `ListView` builds its `cacheBuffer` rows asynchronously (`qquicklistview.cpp`
    // `addVisibleItems`), a row gets its window before its bindings run (`qqmlincubator.cpp`: `setInitialState` →
    // `finalize`), and the bindable width / height go in after every other binding (`qqmlobjectcreator.cpp`
    // `finalize`). The build's slice is a third of the primary screen's frame, two thirds while nothing animates
    // (`qquickwindow.cpp` `QQuickWindowIncubationController`: 3 / 6 ms on a 100 Hz window; offscreen 10 ms, its
    // `QPlatformScreen` defaulting to 60 Hz and its render loop not riding animations). It is wall clock, so a higher
    // refresh rate or a slower build cuts it more often; offscreen only cuts less often, so a headless green is no
    // proof the fault is gone. Hit headless, a visible unpainted mark never pays `Ink.owed`, and the ceiling row's
    // `ink=` stays above 0. What a reader sees: the seat's width is taken; the hover card is another instance and is
    // drawn; scrolling back does not draw it (inside the buffer it is the same instance, and the pool can hand back
    // an unpainted one); a tab switch rebuilds the page, its visible rows built synchronously, and draws it.
    // Whether Qt's first ask (queued twice: `sceneGraphInitialized` → `requestPaint`) beats the sizes is a race with
    // the next slice. Both orders are the product's own build and both are accepted: no rebuild, count or added time
    // waits for the unpainted order (`tst_ink` sets that order by hand once, and builds twice the product's way,
    // judging the pixels after the show). A mark in a card asks nothing of its own either: a canvas back in a window
    // is asked again by Qt (`itemChange`), so a menu row's mark is drawn each time its card opens.
    onVisibleChanged: if (canvas.visible && !canvas.inked) canvas.requestPaint()
    onPainted: canvas.inked = true
    // A mark born with its size never moves `owing`, so the first reading is taken here.
    Component.onCompleted: canvas.settle()
    Component.onDestruction: {
        canvas.inked = true
        canvas.settle()
    }
}
