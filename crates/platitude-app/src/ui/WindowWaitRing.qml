pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The ring beside the pointer while a write that replays history is out (`OperationKind::replays_history`), over
// every pane (デザイン規約 §進行中・回数押しの定数). One, on a bare sheet inside `mainUi`; the condition is the page's
// `replayRunning` (`replaying` plus the automation's latch). Headless: `replay-running` (`--preset replay`).
//
// Why at the pointer: the wait is long, and the reader is looking at the pointer — the band's badge and the buttons
// gone unpressable are out of view. And the OS cursor is in no screenshot, so only a mark the app draws leaves this
// state in the PNGs of both OSes. Placed as `RefusalBadge` (`WaitRing`); the OS cursor is left as it is.
//
// The sheet has no handler of its own (rules-refs/app-ui.md「窓全体のポインタ位置は `HandWatch`」).
Item {
    id: waitSeat

    /// Where the pointer is (`HandWatch`, in scene coordinates), and the page whose replay the mark is about (null
    /// with no tab open).
    required property var hand
    required property var page

    /// Automation's stand-in for the pointer (hover cannot be injected): a point in this sheet, or a negative x where
    /// the real pointer answers. Offscreen reports a hand at the origin, so without it the ring is shot in the corner.
    property point handStandIn: Qt.point(-1, -1)
    readonly property bool handKnown: waitSeat.handStandIn.x >= 0 || waitSeat.hand.known
    readonly property point handAt: waitSeat.handStandIn.x >= 0
                                    ? waitSeat.handStandIn
                                    : waitSeat.mapFromItem(null, waitSeat.hand.at.x, waitSeat.hand.at.y)
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
