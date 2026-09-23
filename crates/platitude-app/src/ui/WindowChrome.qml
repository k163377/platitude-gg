import QtQuick
import QtQuick.Window
import platitude

// What a title bar does, now that the band is one: maximise, minimise, where the grab-run is, and how the window is
// dressed. Pure functions over the window and the band. An invisible Item, so the first Timer or
// Component someone adds has somewhere to stand (rules-refs/structure.md §切り出した非表示のホスト).
Item {
    id: chrome
    visible: false

    /// The window these moves act on (`Main`).
    required property var window
    /// The band whose grab-runs the hit test is told about (`TopBar.grabRunItem` / `dividerRunItem`).
    required property var topBar

    /// The button and the band's double-click have to mean the same thing, so both go to the platform: Qt maximises
    /// a frameless window by resizing it, leaving the platform no maximised state to put back — the button left the
    /// window large while the platform-handled double-click restored it (observed). `visibility` still
    /// says what the window *is*, and still carries the state where the platform has no command of its own.
    function toggleMaximized() {
        const wanted = chrome.window.visibility !== Window.Maximized
        if (chrome.window.captionMerged)
            AppBackend.setWindowMaximized(wanted)
        else
            chrome.window.visibility = wanted ? Window.Maximized : Window.Windowed
    }
    /// Down onto the taskbar — and through the platform for the same reason the maximise is, only sharper: assigning
    /// `Minimized` hands Qt a state that is *only* minimised, so it reads the maximise as given up and clears the
    /// platform's restore-to-maximised flag on the way down. The window came back an ordinary one
    /// (observed). `visibility` still carries it where there is no platform command.
    function minimizeWindow() {
        if (chrome.window.captionMerged)
            AppBackend.minimizeWindow()
        else
            chrome.window.visibility = Window.Minimized
    }
    /// Whether the scene wants its grab-runs back for a while. They are the parts of the window a press never
    /// reaches — the hit test answers HTCAPTION for them, so the platform takes the press and the scene is told nothing
    /// — and a card that is meant to close on a press outside it would stand through every click landing there.
    /// While such a card is up the runs are ordinary client area again, so the press closes it; the band gives up
    /// drag, snap and the double-click for exactly that long, which is what a platform menu does with the click that
    /// dismisses it.
    ///
    /// Only the cards that close on an outside press belong here: the ☰'s and the two the panel's names drop
    /// (`TopBar`'s `standMenu` / `branchMenu`). A modal dialog is not one of them — nothing about it would close, and
    /// the window would merely stop being draggable while it stood.
    readonly property bool captionYielded:
        chrome.topBar.appMenuOpen || chrome.topBar.standMenuOpen || chrome.topBar.branchMenuOpen
    onCaptionYieldedChanged: chrome.reportCaptionStrip()

    /// Automation: what the platform was last told the run past the tabs is, `none` while the scene has it back
    /// (`PGG_AUTO_ACT=app-menu`). Which side owns the runs is not a thing a screenshot holds — the band frames the same
    /// either way. The one run stands for both: they are yielded and reported together.
    property string sentStrip: "none"

    /// Tells the hit test where the band's grab-runs are, in scene coordinates — the stretches it answers HTCAPTION
    /// for, which give the band drag, snap, double-click maximise and the window menu as the platform's own
    /// gestures. Called from the strip's layout changes and from the one shift it cannot see: the window resizing,
    /// which maximising is.
    ///
    /// Two runs: the empty band past the last tab, and the one the divider before the window's buttons stands in.
    /// Both stand in this band and reach up to its top edge, so the one bottom carries them.
    function reportCaptionStrip() {
        if (!chrome.window.captionMerged || chrome.topBar.grabRunItem === null)
            return
        // Empty strips are how "there is no caption here" is said: the hit test takes `x1 > x0` as the question of each
        // run (`winframe::hit_test`), so nothing else has to know about the yield. Both go — the divider's run closes
        // the same card the same way.
        if (chrome.captionYielded) {
            chrome.sentStrip = "none"
            AppBackend.setCaptionStrips(0, 0, 0, 0, 0)
            return
        }
        const run = chrome.topBar.grabRunItem
        const at = run.mapToItem(null, 0, 0)
        const gap = chrome.topBar.dividerRunItem
        const gapAt = gap.mapToItem(null, 0, 0)
        chrome.sentStrip = Math.round(at.x) + "-" + Math.round(at.x + run.width)
        AppBackend.setCaptionStrips(at.x, at.x + run.width,
                                    gapAt.x, gapAt.x + gap.width,
                                    at.y + run.height)
    }
    /// What Windows has to be told about this window. A run that was turned away gets a window too, and undecorated
    /// it would sit on the taskbar as a second application wearing the shell's generic icon.
    function decorateWindow() {
        // Windows 11 rounds the window itself and leaves the corner pixels transparent, so the desktop shows through
        // them. The window is up by now (`visible` is set at the window), which is all the switch needs.
        AppBackend.squareWindowCorners()
        // Without this the window wears the shell's generic icon, in the title bar and on the taskbar button alike.
        AppBackend.setWindowIcon()
        // Asking for no drawn buttons took the system's own gestures with them; this puts those back (`Main.flags`).
        if (!chrome.window.captionMerged)
            return
        AppBackend.keepWindowGestures()
        // The hit test just installed reads the strip from here on; hand it the shape the band settled into while
        // loading.
        chrome.reportCaptionStrip()
    }
}
