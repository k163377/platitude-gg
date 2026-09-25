pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The ring beside the pointer while a write that replays history is out (`OperationKind::replays_history`), over
// every pane.
//
// The sheet has no handler of its own (rules-refs/app-ui.md「窓全体のポインタ位置は `PointerWatch`」).
Item {
    id: waitSeat

    /// Where the pointer is (`PointerWatch`), and the page whose replay the mark is about (null with no tab open).
    required property var hand
    required property var page

    /// Automation's stand-in for the pointer (hover cannot be injected): a point in this sheet, or a negative x where
    /// the real pointer answers. Offscreen reports a hand at the origin, so without it the ring is shot in the corner.
    property point handStandIn: Qt.point(-1, -1)
    readonly property bool handKnown: waitSeat.handStandIn.x >= 0 || waitSeat.hand.known
    readonly property point handAt: waitSeat.handStandIn.x >= 0
                                    ? waitSeat.handStandIn
                                    : Qt.point(waitSeat.hand.handX, waitSeat.hand.handY)
    /// The ring's own `spinning`: `visible` read from another file is stale (as with `RefusalBadge`).
    readonly property bool ringShown: waitRing.spinning

    /// Automation (`PGG_AUTO_ACT=replay-running`): stands a hand at scene `x, y` and leaves it there. A function, so
    /// the map runs once against the geometry it is looking at (rules/app-ui.md: a method takes no binding dependency).
    function holdWaitHand(x, y) {
        waitSeat.handStandIn = waitSeat.mapFromItem(null, x, y)
    }

    WaitRing {
        id: waitRing
        at: waitSeat.handAt
        spinning: waitSeat.handKnown && waitSeat.page !== null && waitSeat.page.replayRunning
    }
}
