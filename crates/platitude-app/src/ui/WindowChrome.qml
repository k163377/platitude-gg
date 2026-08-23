import QtQuick
import QtQuick.Window
import platitude

// What a title bar does, now that the band is one: maximise, minimise, where the grab-run is, and how the window is
// dressed. Pure functions over the window and the band — a QtObject, because nothing here draws or owns children (if
// something ever needs a child object, this becomes an Item — rules-refs/structure.md).
QtObject {
    id: chrome

    /// The window these moves act on (`Main`).
    required property var window
    /// The band whose grab-run the hit test is told about (`TopBar.grabRunItem`).
    required property var topBar

    /// The button and the band's double-click have to mean the same thing, so both go to the platform: Qt maximises
    /// a frameless window by resizing it, leaving the platform no maximised state to put back — the button left the
    /// window large while the platform-handled double-click restored it (reported 2026-08-09). `visibility` still
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
    /// platform's restore-to-maximised flag on the way down. The window came back an ordinary one (reported
    /// 2026-08-22). `visibility` still carries it where there is no platform command.
    function minimizeWindow() {
        if (chrome.window.captionMerged)
            AppBackend.minimizeWindow()
        else
            chrome.window.visibility = Window.Minimized
    }
    /// Tells the hit test where the band's grab-run is, in scene coordinates — the one stretch it answers HTCAPTION
    /// for, which gives the band drag, snap, double-click maximise and the window menu as the platform's own
    /// gestures. Called from the strip's layout changes and from the one shift it cannot see: the window resizing,
    /// which maximising is.
    function reportCaptionStrip() {
        if (!chrome.window.captionMerged || chrome.topBar.grabRunItem === null)
            return
        const run = chrome.topBar.grabRunItem
        const at = run.mapToItem(null, 0, 0)
        AppBackend.setCaptionStrip(at.x, at.x + run.width, at.y + run.height)
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
