import QtQuick
import QtQuick.Window
import platitude

// What a title bar does, since the band is one: maximise, minimise, the grab-runs, and how the window is dressed.
// An `Item` (rules-refs/structure.md「切り出した非表示のホストは `Item` にする」).
Item {
    id: chrome
    visible: false

    /// The window these moves act on (`Main`).
    required property var window
    /// The band whose grab-runs the hit test is told about (`TopBar.grabRunItem` / `seamRunItem`).
    required property var topBar

    /// Through the platform, so the button and the band's double-click agree: Qt maximises a frameless window by
    /// resizing it, leaving the platform no maximised state to restore. `visibility` still reads the state, and sets
    /// it where there is no platform command.
    function toggleMaximized() {
        const wanted = chrome.window.visibility !== Window.Maximized
        if (chrome.window.captionMerged)
            AppBackend.setWindowMaximized(wanted)
        else
            chrome.window.visibility = wanted ? Window.Maximized : Window.Windowed
    }
    /// Through the platform too: assigning `Minimized` makes Qt clear the platform's restore-to-maximised flag, so a
    /// maximised window comes back ordinary. `visibility` carries it where there is no platform command.
    function minimizeWindow() {
        if (chrome.window.captionMerged)
            AppBackend.minimizeWindow()
        else
            chrome.window.visibility = Window.Minimized
    }
    /// Whether the frame is handed back to the scene — the grab-runs and the resize edges. A press on them goes to the
    /// platform (HTCAPTION, HTLEFT …) and the scene hears nothing, so a menu that closes on an outside press would
    /// survive clicks there; while one stands every point is client area, and the window gives up drag, snap, the
    /// double-click and edge resizing.
    ///
    /// Only the menus and lists that close on an outside press, which write themselves in (`OutsidePress`) — a modal
    /// dialog has nothing to close, and would only make the window undraggable.
    readonly property bool frameYielded: OutsidePress.any
    onFrameYieldedChanged: AppBackend.setFrameYielded(chrome.frameYielded)

    /// Automation (`PGG_AUTO_ACT=app-menu`): the run past the tabs as last sent, `none` before the first — no
    /// screenshot shows where the hit test was told it is. It stands for both runs, which move together.
    property string sentStrip: "none"

    /// Tells the hit test where the band's grab-runs are, in scene coordinates. Called on the strip's layout changes
    /// and on a window resize, which the strip cannot see. Two runs — past the last tab, and the seam before the
    /// window buttons — both reaching the band's top, so one bottom carries them.
    function reportCaptionStrip() {
        if (!chrome.window.captionMerged || chrome.topBar.grabRunItem === null)
            return
        const run = chrome.topBar.grabRunItem
        const at = run.mapToItem(null, 0, 0)
        const gap = chrome.topBar.seamRunItem
        const gapAt = gap.mapToItem(null, 0, 0)
        chrome.sentStrip = Math.round(at.x) + "-" + Math.round(at.x + run.width)
        AppBackend.setCaptionStrips(at.x, at.x + run.width,
                                    gapAt.x, gapAt.x + gap.width,
                                    at.y + run.height)
    }
    /// What Windows has to be told about this window — a turned-away run's too. Not the icon: the exe's resource hands
    /// it to the window before it is up (`assets/platitude.rc`).
    function decorateWindow() {
        // Windows 11 rounds the window and the desktop shows through the corners. Needs the window up, which it is
        // (`visible` is set at the window).
        AppBackend.squareWindowCorners()
        // Asking for no drawn buttons took the system's own gestures with them; this puts those back (`Main.flags`).
        if (!chrome.window.captionMerged)
            return
        AppBackend.keepWindowGestures()
        // The hit test just installed reads the strip from here on; hand it the shape the band settled into while
        // loading.
        chrome.reportCaptionStrip()
    }
}
