pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The view's place in a diff that is about to be rebuilt.
//
// A partial write ends by reading the file again, and the answer arrives as a whole new list. Without this the view
// would come back at the top, which on a long diff loses the place being worked through — the same restore
// `GraphPane.shiftRows` does after a graph swap, and delayed for the same reason (`contentHeight` is still the old one
// on the frame the rows land).
Item {
    id: place

    /// The list whose place is being kept.
    required property var view

    property real heldY: -1
    /// Automation: where the restore actually put the view, and -1 until it has run at all. The whole story ends here,
    /// so this is what a verb waits for — a rebuild that emptied the list never gets its count back above zero and so
    /// never restores anything.
    property real landedY: -1
    function hold() {
        // A held place outlives the swap it was taken for when the next write goes out inside the restore's own layout
        // beat — the rows land, the marks wake, and a hand (or `line-run`) presses again before the timer below has
        // fired. At that moment the view is standing in the reset-to-top the swap left it in, so capturing again would
        // keep the top as "the place"; the reader's place is the one already held. The pending restore is stopped —
        // the next swap's own restore finishes the story.
        placeTimer.stop()
        if (place.heldY < 0)
            place.heldY = place.view.contentY
        place.landedY = -1
    }
    function restore() {
        if (place.heldY >= 0)
            placeTimer.restart()
    }
    /// The pane moved to another file (or another side of the same one): what was held is the old diff's place, and
    /// restoring it onto rows it was never read at would carry the scroll somewhere the reader has not been (規約 §diff
    /// を横へ送る「別のファイルは左端から」— the vertical half of the same rule).
    function drop() {
        placeTimer.stop()
        place.heldY = -1
        place.landedY = -1
    }
    // The wait is the graph's (`Metrics.anchorDelayMs`, and `shiftRows` learned it the same way): on the frame the rows
    // land the list has not laid them out yet, so `contentHeight` is still the old one and the clamp below would take
    // the view to the top instead of back to its place.
    Timer {
        id: placeTimer
        interval: Metrics.anchorDelayMs
        onTriggered: {
            // A wheel notch still in flight was aimed into the rows this is putting back (`WheelGlide.halt`).
            place.view.haltGlide()
            const want = place.heldY
            place.heldY = -1
            place.view.contentY = Math.max(0, Math.min(want, place.view.contentHeight - place.view.height))
            place.landedY = place.view.contentY
        }
    }
    /// Automation: read the view away from the top, so that a rebuild can be seen to put it back where it was.
    function scrollTo(y) {
        place.view.contentY = y
    }
}
