import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Keeps a hover card up while the hand walks between it and what it hangs off, and closes it a beat after the hand is
// out of both. A timer, not `Qt.callLater`: the two hovers change in different frames in no fixed order
// (rules-refs/app-ui.md「hover で開くものの 5 つの罠」(3)). Draws nothing; the owner opens the card and hands it here.
QtObject {
    id: keeper

    required property AppCard card
    /// The pointer is on what the card hangs off. The owner writes it; headless runs write it too, since hover cannot
    /// be injected.
    property bool pointedAt: false
    /// Something other than a pointer holds the card up (a menu on one of its rows, a pin).
    property bool grace: false

    /// The card's ground, and through it the one shared tooltip.
    readonly property Item ground: keeper.card !== null ? keeper.card.contentItem : null
    readonly property var tip: keeper.ground !== null ? keeper.ground.ToolTip.toolTip : null

    /// A row of this card raised the shared tooltip and it is still up
    /// (rules-refs/app-ui.md「hover で保つカードは、自分の行が出したツールチップの間も保つ」).
    readonly property bool tipHeld: keeper.tipStands()
    function tipStands() {
        if (keeper.tip === null || !keeper.tip.visible)
            return false
        for (let at = keeper.tip.parent; at !== null; at = at.parent) {
            if (at === keeper.ground)
                return true
        }
        return false
    }

    /// Anything is asking for the card; it closes a beat after this falls.
    readonly property bool lit:
        keeper.pointedAt || keeper.grace || keeper.card.pointerInside || keeper.tipHeld

    /// Restarts the beat by hand, for an owner that knows the answer changed before `lit` can see it (a menu just
    /// dismissed).
    function settle() {
        keep.restart()
    }

    /// A scroll bar outside this card is held (`Hand.heldBar`): what the card hangs off is sliding from under a hand Qt
    /// no longer hears, so the card goes at once and does not come out until the bar is let go. **A bar inside it is
    /// the hand reading the card** (`RefListPopup`, `SectionPeekPopup`), and keeps it — the hover frozen with it says
    /// the hand is still in (規約 §hover のツールチップ「スクロールバーを掴んだら、hover で開いたものは閉じる」).
    readonly property bool barOutside: Hand.heldBar !== null && !keeper.holds(Hand.heldBar)
    function holds(item) {
        for (let at = item; at !== null; at = at.parent) {
            if (at === keeper.card.contentItem)
                return true
        }
        return false
    }
    onBarOutsideChanged: {
        if (keeper.barOutside)
            keeper.card.close()
    }
    // Opened while such a bar is held — a rest that ran out, a row reused under the hand: down again before anything
    // is drawn (`SharedToolTip.onOpened` says why not at `visibleChanged`).
    property Connections opening: Connections {
        target: keeper.card
        function onOpened() {
            if (keeper.barOutside)
                keeper.card.close()
        }
    }

    property Timer keep: Timer {
        interval: Metrics.hoverKeepMs
        onTriggered: {
            if (!keeper.lit)
                keeper.card.close()
        }
    }

    onLitChanged: keeper.settle()
}
