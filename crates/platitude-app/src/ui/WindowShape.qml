import QtQuick
import QtQuick.Window
import platitude
import platitude.ui

/// The size and place the window comes back to: restoring the saved one, measuring the frame slop, and reporting for
/// the next launch. Every size the application sets itself goes through here onto the window's floor.
Item {
    id: shape

    required property var window

    /// What the store sends for a coordinate it has never been told.
    readonly property int unplaced: -2147483648

    /// What this window adds to a size on the way in: Qt takes the frame margins from one place when it sets the
    /// geometry and another when it reads it back, so saving the size read back would grow the window every launch.
    property int widthSlop: 0
    property int heightSlop: 0
    /// The size the window was asked for, which the slop is measured from.
    property int askedWidth: 0
    property int askedHeight: 0

    /// Lifts a windowed window standing under its floor (maximised and minimised are the platform's to size). The
    /// lifted size goes into `asked*` too, or the growth would read as frame slop and shrink every later save.
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

    /// Keep the configured size whatever the screen holds ([`insideScreen`]) — for headless runs, whose offscreen
    /// screen is 800x800. False unless written from outside.
    property bool keepSavedSize: false

    /// A remembered length, kept inside one screen's width or height — not `Screen.desktopAvailableWidth`, which
    /// spans the whole virtual desktop.
    function insideScreen(saved, screen) {
        return shape.keepSavedSize ? saved : Math.min(saved, screen)
    }

    /// The screen the saved top-left corner falls on, or `null` for an unsaved position or a screen no longer here.
    /// The saved place decides the screen: `Screen` is wherever the platform just put the window (on Windows, the
    /// pointer's screen), and measuring against it sizes and pulls the window to the wrong monitor.
    ///
    /// Takes the list so `tst_windowshape` can hold it against a desktop this machine does not have.
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

    // Restores the saved size and place, assigned: from here on the window manager and the user own them. An
    // unsaved position stays unset so the platform places the window.
    function applySavedWindow() {
        const x = AppBackend.startWindowX()
        const y = AppBackend.startWindowY()
        // `Screen` is only the fallback ([`screenHolding`]).
        const home = shape.screenHolding(x, y, Qt.application.screens)
        const roomW = home !== null ? home.width : Screen.width
        const roomH = home !== null ? home.height : Screen.height
        // Over the floor already: `settleTimer` measures the slop from this, and a maximise comes back to it.
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
        // The frame has to fit, and it is wider than the window says, which `insideScreen` cannot see: the platform
        // side moves the window back and says whether it had to. Before the maximise, since the shape standing then
        // is what a restore comes back to. Named, so a window placed elsewhere is pulled back to the saved monitor;
        // an empty name fits it to the nearest (first run, unplugged monitor).
        const moved = AppBackend.fitWindowToScreen(home !== null ? home.name : "")
        if (AppBackend.startWindowMaximized()) {
            // Through the platform, so it keeps the shape to restore (`WindowChrome.toggleMaximized`).
            if (shape.window.captionMerged)
                AppBackend.setWindowMaximized(true)
            else
                shape.window.visibility = Window.Maximized
        } else if (!moved) {
            // A moved or maximised window no longer has the size it was handed, so no slop is measured.
            settleTimer.restart()
        }
    }

    /// Everything the next launch should come back to (rules-refs/core.md — settings.toml / state.toml).
    function reportState() {
        // Minimised, a window reports neither its windowed nor its maximised shape (Windows): saving it would write
        // a size wider than the screen and clear the maximised flag.
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
            // Only against the size just handed, while nothing else resized it: a maximise's or a snap's difference
            // is not a frame margin.
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
