import QtQuick

// A wait for what was just changed to be taken in: it fires once a frame's polish has run since `restart()` — the
// polish that lays out a list whose rows were replaced (a reset zeroes `contentY` there, and `contentHeight` is the
// old one until it), and the frame that comes after the platform's queued answer to a size the window was handed.
// Waited on the frame itself, not a time: how long a frame takes is the screen's and the machine's, so a fixed wait
// comes early on a slow machine and late on a fast screen (rules-refs/app-ui.md「入れ替えの後は次の polish を待つ」).
//
// The frame is asked for (`QQuickWindow::update`: the basic and software loops draw nothing without it) and heard at
// `afterAnimating`, which both render loops emit on this thread right after the frame's polish. The answer runs a turn
// later (`Qt.callLater`), between frames as a `Timer`'s does, and the type is shaped like one (`restart` / `stop` /
// `running` / `triggered`). Not `FrameAnimation`: the basic loop ticks animations off a timer of their own, so two
// ticks can pass with no polish between them.
Item {
    id: later

    readonly property bool running: later.waiting
    property bool waiting: false
    /// Counts `restart()`, so a frame heard for an earlier ask does not answer a later one.
    property int asked: 0
    property Connections frameWatch: Connections {
        target: later.waiting ? later.Window.window : null
        function onAfterAnimating() {
            const at = later.asked
            Qt.callLater(() => later.heard(at))
        }
    }

    signal triggered()
    /// The frame heard for the `restart()` counted `at`, a turn after it. The call handed to `Qt.callLater` only emits
    /// this, so a page taken down meanwhile hears nothing (`TurnLater`).
    signal heard(int at)

    function restart() {
        later.asked++
        later.waiting = true
        if (later.Window.window !== null)
            later.Window.window.update()
    }
    function stop() {
        later.waiting = false
    }
    function answer(at) {
        if (!later.waiting || at !== later.asked)
            return
        later.waiting = false
        later.triggered()
    }

    onHeard: at => later.answer(at)
}
