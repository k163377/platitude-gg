import QtQuick
import QtQuick.Window
import platitude
import platitude.ui

/// The size and place the window comes back to: restoring the saved one, measuring the frame slop, and reporting for
/// the next launch. Every size the application sets itself goes through here onto the window's floor.
Item {
    id: shape

    required property var window

    /// What the store sends for a coordinate or a length it has never been told.
    readonly property int unplaced: -2147483648

    /// What this window adds to a size on the way in: Qt takes the frame margins from one place when it sets the
    /// geometry and another when it reads it back, so saving the size read back would grow the window every launch.
    property int widthSlop: 0
    property int heightSlop: 0
    /// The size the window was asked for, which the slop is measured from.
    property int askedWidth: 0
    property int askedHeight: 0
    /// The saved monitor's device name, from [`applySavedWindow`] for [`settleSavedWindow`]; empty for none.
    property string savedScreen: ""

    /// Lifts a windowed window standing under its floor (maximised and minimised are the platform's to size): a floor
    /// that rises grows the window — unfolding the left list on a window at its floor is the case, and fold → shrink →
    /// unfold holds only through this — and a saved shape under the floor is lifted at launch. The lifted size goes
    /// into `asked*` too, or the growth would read as frame slop and shrink every later save.
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
    /// screen is 800x800. False unless written from outside: `WindowHarness` sets it off `Harness.automated`, so only
    /// a window a person looks at is pulled in. Not told apart by the settings file: verify-ui always passes
    /// `PGG_CONFIG_DIR`, and told apart wrongly every screenshot comes out 800 wide.
    property bool keepSavedSize: false

    /// A remembered length, kept inside one screen's width or height — not `Screen.desktopAvailableWidth`, which
    /// spans the whole virtual desktop.
    function insideScreen(saved, screen) {
        return shape.keepSavedSize ? saved : Math.min(saved, screen)
    }
    /// One side of the window the screen's side `room` opens it at: [`builtLength`] kept inside the screen
    /// ([`insideScreen`]). The room is the screen, not its work area, which QML is not told; Windows fits the frame to
    /// the work area after (`settleSavedWindow`), and the other platforms' window managers keep a new window on the
    /// screen themselves.
    function firstLength(saved, room) {
        return shape.insideScreen(shape.builtLength(saved, room), room)
    }
    /// One side of the window as `Main` builds it: the remembered length, or for a window never sized,
    /// `Metrics.firstWindowShare` of the room.
    function builtLength(saved, room) {
        return saved === shape.unplaced ? Math.round(room * Metrics.firstWindowShare) : saved
    }

    /// The screen the saved top-left corner falls on, or `null` for an unsaved position or a screen no longer here.
    /// The saved place decides the screen: `Screen` is wherever the platform just put the window (on Windows, the
    /// pointer's screen), and measuring against it sizes and pulls the window to the wrong monitor. What crosses to the
    /// platform is the screen's device name (`AppBackend.fitWindowToScreen`), never a coordinate: Qt's coordinates are
    /// Qt's own and part from Windows' across two screens at different scales. An empty name (first run, the saved
    /// screen gone) fits the window to its own nearest monitor.
    ///
    /// The entries of `Qt.application.screens` are `QQuickScreenInfo` (`qquickscreen_p.h`): `name`, `width`, `height`,
    /// `devicePixelRatio`, `virtualX`, `virtualY`. A property not in that list reads `undefined` without an error, so
    /// a coordinate looked up under another name falls through to "no screen" silently.
    ///
    /// Takes the list so `tst_windowshape` can hold it against a desktop this machine does not have.
    function screenHolding(corner, screens) {
        if (!shape.placed(corner))
            return null
        for (let i = 0; i < screens.length; i++) {
            const s = screens[i]
            if (corner.x >= s.virtualX && corner.x < s.virtualX + s.width
                    && corner.y >= s.virtualY && corner.y < s.virtualY + s.height)
                return s
        }
        return null
    }
    /// Whether a saved top-left corner was saved at all: either axis `unplaced` is no place.
    function placed(corner) {
        return corner.x !== shape.unplaced && corner.y !== shape.unplaced
    }

    // Restores the saved size and place, assigned: from here on the window manager and the user own them. An
    // unsaved size is a share of the screen ([`firstLength`]); an unsaved position stays unset so the platform places
    // the window.
    //
    // In two steps, either side of the strip's start (`Main`). The size and place go on first, so the page built next
    // is laid out for the window it stands in. The floor is a page's to say, so the frame
    // is fitted and the window maximised after ([`settleSavedWindow`]): the page a restored tab brings, or the blank
    // one, is up by then, and where its floor is over the saved shape it has lifted it in between (`holdFloor`), the
    // window still windowed. A run that asks for its folders has no page yet at the second step — git places them
    // later — and is lifted by `holdFloor` alone, with no fit after it.
    function applySavedWindow() {
        const corner = Qt.point(AppBackend.startWindowX(), AppBackend.startWindowY())
        // `Screen` is only the fallback ([`screenHolding`]).
        const home = shape.screenHolding(corner, Qt.application.screens)
        shape.savedScreen = home !== null ? home.name : ""
        const roomW = home !== null ? home.width : Screen.width
        const roomH = home !== null ? home.height : Screen.height
        // Over the floor as far as it is known: the band's alone, with no page up yet.
        const wantWidth = Math.max(
            shape.firstLength(AppBackend.startWindowWidth(), roomW),
            Math.ceil(shape.window.floorWidth))
        const wantHeight = Math.max(
            shape.firstLength(AppBackend.startWindowHeight(), roomH),
            Math.ceil(shape.window.floorHeight))
        shape.window.width = wantWidth
        shape.window.height = wantHeight
        shape.askedWidth = wantWidth
        shape.askedHeight = wantHeight
        if (shape.placed(corner)) {
            shape.window.x = corner.x
            shape.window.y = corner.y
        }
    }

    /// The second step of [`applySavedWindow`], once the strip has started: the shape standing now is the one
    /// `settleTimer` measures the slop from and a maximise comes back to — over the floor of whichever page is up.
    function settleSavedWindow() {
        // The frame has to fit, and it is wider than the window says, which `insideScreen` cannot see: the platform
        // side moves the window back and says whether it had to. Before the maximise, since the shape standing then
        // is what a restore comes back to. Named, so a window placed elsewhere is pulled back to the saved monitor;
        // an empty name fits it to the nearest (first run, unplugged monitor).
        const moved = AppBackend.fitWindowToScreen(shape.savedScreen)
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

    /// Everything the next launch should come back to (core `settings` module — settings.toml / state.toml). The one
    /// place that reports it, on `stateTimer` and not per change: a drag moves the window every frame. `Main` keeps
    /// only a same-named one-line forward (the harness calls it there). Whether to write is Rust's, comparing with
    /// the last write (`Hub::flush_state`). The last report goes out from both `Main.onClosing` and `Hub::shutdown`:
    /// `Qt.quit()` emits no `onClosing`.
    function reportState() {
        // Minimised, a window reports neither its windowed nor its maximised shape (Windows): saving it would write
        // a size wider than the screen and clear the maximised flag. The timer runs on while it is down, and that
        // shape filed as windowed puts the next launch past the screen's edge. Headless: `state minimize`.
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
