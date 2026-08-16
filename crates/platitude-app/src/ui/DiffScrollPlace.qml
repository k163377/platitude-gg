pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// The view's place in a diff that is about to be rebuilt.
//
// A partial write ends by reading the file again, and the answer arrives as
// a whole new list. Without this the view would come back at the top, which
// on a long diff loses the place being worked through — the same restore
// `GraphPane.shiftRows` does after a graph swap, and delayed for the same
// reason (`contentHeight` is still the old one on the frame the rows land).
Item {
    id: place

    /// The list whose place is being kept.
    required property var view

    property real heldY: -1
    function hold() {
        place.heldY = place.view.contentY
    }
    function restore() {
        if (place.heldY >= 0)
            placeTimer.restart()
    }
    // The wait is the graph's (`Metrics.anchorDelayMs`, and `shiftRows`
    // learned it the same way): on the frame the rows land the list has
    // not laid them out yet, so `contentHeight` is still the old one and
    // the clamp below would take the view to the top instead of back to
    // its place.
    Timer {
        id: placeTimer
        interval: Metrics.anchorDelayMs
        onTriggered: {
            const want = place.heldY
            place.heldY = -1
            place.view.contentY = Math.max(0, Math.min(
                want, place.view.contentHeight - place.view.height))
            if (AppBackend.autoAct !== "")
                AppBackend.report("diff_place " + Math.round(place.view.contentY))
        }
    }
    /// Automation: read the view away from the top, so that a rebuild can
    /// be seen to put it back where it was.
    function scrollTo(y) {
        place.view.contentY = y
    }
}
