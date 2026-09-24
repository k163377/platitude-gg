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

    /// The window comes up at the size it was configured with, whatever the screen says it will hold
    /// ([`insideScreen`]). Written from outside and false wherever nobody wrote it: what it is for is a machine
    /// nobody is at, whose offscreen platform reports an 800x800 screen that would cut every window down to fit.
    property bool keepSavedSize: false

    /// A remembered length, kept inside the screen the window comes up on. `Screen.width` —
    /// `Screen.desktopAvailableWidth` is the whole virtual desktop (measured on a three-monitor machine: 5760,
    /// so nothing is ever wider).
    function insideScreen(saved, screen) {
        return shape.keepSavedSize ? saved : Math.min(saved, screen)
    }

    /// The screen the saved top-left corner falls on, or `null` for a position nobody saved and for one whose screen
    /// is not here any more — a monitor unplugged since the run that wrote it.
    ///
    /// **The saved place decides the screen.** Before this, everything about the
    /// restore was measured against wherever the platform had just put the window — which on Windows is the screen
    /// the pointer is on — so a window saved on one monitor was sized to another monitor's width and pulled into that
    /// monitor's work area (2026-09-05 実測, P3-確認事項). Both of those are answers to "where is it now", and the
    /// question is "where was it left".
    ///
    /// Takes the list, so the arithmetic can be held against a
    /// desktop this machine does not have (`tst_windowshape`).
    function screenHolding(x, y, screens) {
        if (x === shape.unplaced || y === shape.unplaced)
            return null
        for (let i = 0; i < screens.length; i++) {
            const s = screens[i]
            if (x >= s.virtualX && x < s.virtualX + s.width
                    && y >= s.virtualY && y < s.virtualY + s.height)
                return s
        }
        return null
    }

    // The size and place the window was left in. Assigned: from here on the window manager and the
    // person dragging it own these. An unsaved position stays unset so the platform places the window itself — a
    // first run opens where the platform puts it.
    function applySavedWindow() {
        const x = AppBackend.startWindowX()
        const y = AppBackend.startWindowY()
        // The screen the saved place is on, decided before anything is measured against one ([`screenHolding`]).
        // `Screen` — the window's own, which is wherever the platform has just put it — is the fallback for a first
        // run and for a screen that has since gone.
        const home = shape.screenHolding(x, y, Qt.application.screens)
        const roomW = home !== null ? home.width : Screen.width
        const roomH = home !== null ? home.height : Screen.height
        // Over the floor on the way in: what is assigned here is what `settleTimer` measures the frame slop
        // from, and what a maximise would come back to.
        const wantWidth = Math.max(
            shape.insideScreen(AppBackend.startWindowWidth(), roomW),
            Math.ceil(shape.window.floorWidth))
        const wantHeight = Math.max(
            shape.insideScreen(AppBackend.startWindowHeight(), roomH),
            Math.ceil(shape.window.floorHeight))
        shape.window.width = wantWidth
        shape.window.height = wantHeight
        shape.askedWidth = wantWidth
        shape.askedHeight = wantHeight
        if (x !== shape.unplaced && y !== shape.unplaced) {
            shape.window.x = x
            shape.window.y = y
        }
        // The *frame* has to fit, and it is wider than the window says it is (measured, a remembered 1920
        // came back as a 1936-wide frame at x=-5 on a 1920 screen). `insideScreen` sees neither number; the platform
        // side moves the window back and says whether it had to. Before the maximise: the shape standing
        // when a window is maximised is the shape a restore comes back to.
        //
        // **Named**: the platform side is told which monitor to fit to, so a
        // window the platform has put somewhere else is pulled back to the saved one.
        // An empty name leaves it to answer from the window, which is what a first run and an
        // unplugged monitor both want — the nearest monitor.
        const moved = AppBackend.fitWindowToScreen(home !== null ? home.name : "")
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
        // maximised numbers and its visibility is no longer Maximized — a report from here would write a window wider
        // than the screen and clear the flag that restores the maximised one.
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
        // One beat, so the window has answered the size it was given: the answer arrives as a queued platform
        // event.
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
