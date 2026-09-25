pragma Singleton

import QtQuick

// The middle-click gesture under way in this window, if any (`MiddleAutoScroll`). One at a time: a press on a second
// surface is the reader moving on. And one door for Escape: the key reaches the page, never the surface being sent,
// so the page asks here (`RepoPage.escapePressed`).
QtObject {
    /// The hand whose gesture is running, or null; the hands write it as they start and stop.
    property Item running: null

    /// Ends the gesture under way. Answers whether there was one, so a key handler takes Escape only when this had
    /// something to take.
    function stop() {
        if (running === null)
            return false
        running.stop()
        return true
    }
    /// A press landed somewhere in the window (`Main`'s watcher hears every one): the running gesture ends — a
    /// surface's own hand covers only that surface.
    ///
    /// Looked at a turn later, so the press is answered first where it landed (the gesture's own click, or a new
    /// gesture on another surface); if the running gesture changed by then, this leaves it alone.
    function pressLanded() {
        const was = running
        if (was !== null)
            Qt.callLater(() => {
                if (running === was)
                    was.stop()
            })
    }
}
