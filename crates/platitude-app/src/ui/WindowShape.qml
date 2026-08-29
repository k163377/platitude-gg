import QtQuick
import QtQuick.Window
import platitude
import platitude.ui

/// The size and place the window comes back to: what a saved run is restored into, what the frame slop is measured
/// from, and what the next launch is told. The window owns the floor — read off the band and the page — and this keeps
/// every size the application sets itself standing on it.
Item {
    id: shape

    required property var window

    /// What the store sends for a coordinate it has never been told.
    readonly property int unplaced: -2147483648

    /// What this window adds to a size on the way in. A size does not read back the way it was written (measured on
    /// the merged chrome: asked for 1200 it calls itself 1206, so writing down what it says grew the window 6px every
    /// launch). Qt takes the frame margins from one place when it sets the geometry and another when it reads it back,
    /// so the difference is read off the window itself and taken away again on the way out.
    property int widthSlop: 0
    property int heightSlop: 0
    /// The size the window was asked for, which the slop is measured from.
    property int askedWidth: 0
    property int askedHeight: 0

    /// Puts a window standing under its floor back on it — only in its own shape: maximised and minimised are the
    /// platform's to size. The lifted size also goes into `asked*`, or the growth itself would read as frame slop and
    /// every later launch would write the window down that much smaller (measured: lifted from a 320-wide file the
    /// window stood at 704 and 510 went into the file).
    function holdFloor() {
        if (shape.window.visibility !== Window.Windowed)
            return
        if (shape.window.width < shape.window.floorWidth) {
            shape.window.width = Math.ceil(shape.window.floorWidth)
            shape.askedWidth = shape.window.width
        }
        if (shape.window.height < shape.window.floorHeight) {
            shape.window.height = Math.ceil(shape.window.floorHeight)
            shape.askedHeight = shape.window.height
        }
    }

    /// A remembered length, kept inside the screen the window comes up on. `Screen.width`, *not*
    /// `Screen.desktopAvailableWidth` — that is the whole virtual desktop (measured on a three-monitor machine: 5760,
    /// so nothing is ever wider). Automated runs are exempt: the offscreen platform reports an 800x800 screen that
    /// would cut every screenshot to fit.
    function insideScreen(saved, screen) {
        return AppBackend.automated ? saved : Math.min(saved, screen)
    }

    // The size and place the window was left in. Assigned rather than bound: from here on the window manager and the
    // person dragging it own these. An unsaved position stays unset so the platform places the window itself — a first
    // run should not open at 0,0.
    function applySavedWindow() {
        // Over the floor on the way in, not after: what is assigned here is what `settleTimer` measures the frame slop
        // from, and what a maximise would come back to.
        const wantWidth = Math.max(
            shape.insideScreen(AppBackend.startWindowWidth(), Screen.width),
            Math.ceil(shape.window.floorWidth))
        const wantHeight = Math.max(
            shape.insideScreen(AppBackend.startWindowHeight(), Screen.height),
            Math.ceil(shape.window.floorHeight))
        shape.window.width = wantWidth
        shape.window.height = wantHeight
        shape.askedWidth = wantWidth
        shape.askedHeight = wantHeight
        const x = AppBackend.startWindowX()
        const y = AppBackend.startWindowY()
        if (x !== shape.unplaced && y !== shape.unplaced) {
            shape.window.x = x
            shape.window.y = y
        }
        // The *frame* has to fit, and it is wider than the window says it is (measured, a remembered 1920
        // came back as a 1936-wide frame at x=-5 on a 1920 screen). `insideScreen` sees neither number; the platform
        // side moves the window back and says whether it had to. Before the maximise, not after: the shape standing
        // when a window is maximised is the shape a restore comes back to.
        const moved = AppBackend.fitWindowToScreen()
        if (AppBackend.startWindowMaximized()) {
            // Through the platform, so it holds the shape to come back to (`toggleMaximized`). Where there is no
            // platform command, `visibility` still carries it.
            if (shape.window.captionMerged)
                AppBackend.setWindowMaximized(true)
            else
                shape.window.visibility = Window.Maximized
        } else if (!moved) {
            // A run that was moved or maximised measures nothing: the slop is the difference between the size the
            // window was handed and the size it reports, and neither of those is that.
            settleTimer.restart()
        }
    }

    /// Everything the next launch should come back to (rules-refs/core.md — settings.toml / state.toml).
    function reportState() {
        // A minimised window says nothing. Measured on Windows: while down it reports neither its windowed nor its
        // maximised numbers and its visibility is no longer Maximized — a report from here wrote a window wider than
        // the screen and cleared the flag that would have restored the maximised one.
        if (shape.window.visibility !== Window.Minimized)
            AppBackend.saveWindow(shape.window.x, shape.window.y,
                                  shape.window.width - shape.widthSlop,
                                  shape.window.height - shape.heightSlop,
                                  shape.window.visibility === Window.Maximized)
        if (shape.window.curPage !== null)
            shape.window.curPage.reportLayout()
        AppBackend.flushState()
    }

    Timer {
        id: settleTimer
        // One beat, so the window has answered the size it was given: the answer arrives as a queued platform event,
        // not inside the assignment.
        interval: Metrics.anchorDelayMs
        onTriggered: {
            // Only against a size this window was just handed, and only while nothing else has resized it — a maximise
            // or a snap resizes on the way, and a difference read off that is not a frame margin.
            if (shape.askedWidth <= 0 || shape.window.visibility !== Window.Windowed)
                return
            shape.widthSlop = shape.window.width - shape.askedWidth
            shape.heightSlop = shape.window.height - shape.askedHeight
        }
    }

    Timer {
        id: stateTimer
        interval: Metrics.stateFlushMs
        repeat: true
        // A run that was turned away holds an empty store, so its reports would reach no file.
        running: !AppBackend.alreadyRunning
        onTriggered: shape.reportState()
    }
}
