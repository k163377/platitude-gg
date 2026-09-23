pragma Singleton

import QtQuick

// The middle-click gesture under way in this window, if there is one (`MiddleAutoScroll`).
//
// **One at a time.** There is one pointer, and every surface here that scrolls has a hand of its own, so a press on a
// second surface is the reader moving on: the first gesture ends there rather than going on sending a list nobody is
// looking at any more.
//
// **And one door for Escape.** The key reaches the page, or the screen standing over it, and never the surface being
// sent — so the page asks here instead of naming every surface that scrolls (`RepoPage.escapePressed`).
QtObject {
    /// The hand whose gesture is running, or null. Written by the hands themselves as they start and stop.
    property Item running: null

    /// Ends the gesture under way. Answers whether there was one, so a key handler takes Escape only when this had
    /// something to take.
    function stop() {
        if (running === null)
            return false
        running.stop()
        return true
    }
    /// A press landed somewhere in the window (`Main`'s watcher hears every one). **The gesture running ends with it**:
    /// a click anywhere is the reader moving on, the way a browser takes one — a surface's own hand covers only that
    /// surface, and a small one is left behind by the first move of the pointer.
    ///
    /// **Looked at a turn later**, so the press is answered first where it landed: on the gesture's own surface that
    /// is the click taking it down (and kept from the row under it), and a middle press on another surface has
    /// started a gesture there in this one's place. Either way the gesture seen running is not the one running by
    /// then, and this leaves it alone.
    function pressLanded() {
        const was = running
        if (was !== null)
            Qt.callLater(() => {
                if (running === was)
                    was.stop()
            })
    }
}
