pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// A write that replays history stands for seconds (`OperationKind::replays_history`), and for all of them the pointer is
// the one thing the reader is looking at. The ring goes beside it, over everything, and belongs to no pane: it
// answers the hand rather than whatever the hand happens to be over.
//
// The sheet draws and nothing more — no handler of its own. Hover is taken by the item it is laid over, so a handler
// here would put every row and cell below it out (規約 §QML 実装ルール); the pointer is read off the
// handler the window declares over the whole of its content, which leaves its own subtree answering
// (`PointerWatch`).
Item {
    id: waitSeat

    /// Where the pointer is (`PointerWatch`), and the page whose replay the mark is about — null while no tab is
    /// open. Handed over rather than reached for: the window owns both.
    required property var hand
    required property var page

    /// Where the hand would have been, for the runs that have none: a point in this sheet, or a negative x
    /// for the ordinary case where a real pointer answers. The same shape every hover stand-in in this app
    /// has (`SharedToolTip.handAcross`, `BandStateGroup.pointedAt`) — hover cannot be injected, so automation
    /// writes what the pointer would have written, into the one answer the mark reads.
    ///
    /// **Offscreen does answer**, with a hand at the origin (`SharedToolTip`), so a run without this
    /// photographs a ring in the window's top corner — drawn, and in no place worth judging it by.
    property point handStandIn: Qt.point(-1, -1)
    readonly property bool handKnown: waitSeat.handStandIn.x >= 0 || waitSeat.hand.known
    readonly property point handAt: waitSeat.handStandIn.x >= 0
                                    ? waitSeat.handStandIn
                                    : Qt.point(waitSeat.hand.handX, waitSeat.hand.handY)
    /// What the mark makes of it. Its own `spinning` rather than its `visible`, for the reason the other mark a
    /// pointer wears gives (`RefusalBadge`): read from another file `visible` comes back stale.
    readonly property bool ringShown: waitRing.spinning

    /// Automation (`PGG_AUTO_ACT=replay-running`): stands a hand at `x, y` in scene coordinates and leaves it there.
    /// A function rather than a binding, so the map runs once against the geometry it is looking at (app-ui.md).
    function holdWaitHand(x, y) {
        waitSeat.handStandIn = waitSeat.mapFromItem(null, x, y)
    }

    WaitRing {
        id: waitRing
        at: waitSeat.handAt
        // Only the tab on screen has a replay, and only a hand that is over this window has a place to be
        // told about it (デザイン規約 §進行中・長押しの定数 — the mark is the whole of what says a wait is on).
        spinning: waitSeat.handKnown && waitSeat.page !== null && waitSeat.page.replayRunning
    }
}
