pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The hand that carries a tab: which tab is in it, where it has got to, and what the strip's order does about that
// (デザイン規約 §タブの所作). The strip owns the tabs and says what a press means; this owns the gesture between the
// taking up and the setting down.
//
// Not the strip's own for the reason `MiddleAutoScroll` is not the graph's: the hand and the thing it moves are
// separate questions, and only one of them is about how a tab is drawn.
//
// An `Item` rather than a `QtObject` because the drift below is a `Timer` and needs somewhere to stand; it draws
// nothing and is never given a size.
Item {
    id: carry

    /// The strip's own list, and the model under it. Both are the strip's — a carry reads where the tabs sit and asks
    /// the model to change their order, and owns neither.
    required property ListView view
    required property var tabsModel

    /// The tab in hand: which one it is, where inside it the hand took hold, and where its left edge has been carried
    /// to in the strip's own coordinates. Held by id rather than by position — the order changes under a drag, and the
    /// row the hand is on is the one thing about it that does not (デザイン規約 §タブの所作).
    property int heldId: -1
    property real heldGrabX: 0
    property real heldX: 0
    /// Where the hand last reported in the scene, and how far past the run it is asking for. A hand that has run out of
    /// strip is still asking, and the distance it asks by is what the strip travels at (デザイン規約 §タブの所作 —— the
    /// sensitivity is the one the app's other autoscroll reads, §グラフを横へ送る).
    property real heldSceneX: 0
    property real heldPush: 0

    /// The tab at `index`, taken up: the hand has carried it past the platform's threshold and is now holding it.
    /// `grabX` is where inside the tab it took hold, which is what keeps that same point of the tab under the hand
    /// however far it travels.
    function takeTab(index, grabX) {
        const tab = carry.view.itemAtIndex(index)
        if (!tab)
            return
        carry.heldId = tab.tab_id
        carry.heldGrabX = grabX
        carry.heldX = tab.x
    }

    /// The hand, moved to `sceneX` with a tab in it. Scene coordinates in, the strip's own out: the tab is drawn where
    /// the hand is rather than where the row sits, so its own coordinates cannot say where the pointer got to.
    ///
    /// The run — what is on screen — is as far as a tab can be put. Past either end of it the tab stays at the edge and
    /// the strip travels underneath instead (`driftRun`), which is the only way a tab reaches a place that is not on
    /// screen when the carry starts.
    function carryTab(index, sceneX) {
        const tab = carry.view.itemAtIndex(index)
        if (!tab)
            return
        carry.heldSceneX = sceneX
        const want = carry.view.contentItem.mapFromItem(null, sceneX, 0).x - carry.heldGrabX
        const runFrom = carry.view.contentX
        const runTo = carry.view.contentX + carry.view.width - tab.width
        carry.heldPush = want < runFrom ? want - runFrom : Math.max(0, want - runTo)
        carry.carryTo(index, Math.max(runFrom, Math.min(want, runTo)))
    }

    /// One tick of the strip travelling under a tab held against its end, at the speed the hand is asking for.
    function driftRun() {
        const at = carry.heldIndex()
        if (at < 0)
            return
        const room = carry.view.contentWidth - carry.view.width
        const asked = carry.view.contentX + carry.heldPush * Metrics.middleScrollGain
        const settled = Math.max(0, Math.min(asked, room))
        if (settled === carry.view.contentX)
            return
        carry.view.contentX = settled
        // The hand has not moved; what it is pointing into the strip at has.
        carry.carryTab(at, carry.heldSceneX)
    }

    /// Where the tab in hand sits right now, or -1 with nothing in hand. Walked rather than remembered: the carry is
    /// what changes it.
    function heldIndex() {
        for (let i = 0; i < carry.view.count; i++) {
            const tab = carry.view.itemAtIndex(i)
            if (tab && tab.tab_id === carry.heldId)
                return i
        }
        return -1
    }

    /// The one place a carried tab is put anywhere: where it is drawn, and — for as long as it keeps passing them —
    /// which neighbours it has changed places with. The hand comes through `carryTab` and the smoke hooks through the
    /// strip's own, so neither can reach an order the other cannot.
    function carryTo(index, left) {
        const tab = carry.view.itemAtIndex(index)
        if (!tab)
            return index
        // The strip is the whole of the run: a tab carried past either end stops there, the way everything else that
        // scrolls here stops (デザイン規約 §QML 実装ルール). Nothing is torn off into a window of its own.
        carry.heldX = Math.max(0, Math.min(left, carry.view.contentWidth - tab.width))
        let at = index
        // Bounded by the strip itself: each step passes one tab, so nothing can be passed more often than there are
        // tabs to pass.
        for (let step = 0; step < carry.view.count; step++) {
            const next = carry.stepOrder(at)
            if (next === at)
                break
            at = next
            // That step changed the order, so the places the next comparison reads have to be the ones the view has
            // just given the tabs rather than the ones they are leaving.
            carry.view.forceLayout()
        }
        return at
    }

    /// One neighbour, passed or not: the carried tab changes places with whichever side it has taken half of.
    ///
    /// The leading edge against the neighbour's middle, rather than middle against middle. Tabs are of different widths
    /// — middles agree only where they are of one width, and a wide tab held against the end of the strip never reaches
    /// a narrow last tab's middle at all, which would leave the last place unreachable by hand.
    function stepOrder(at) {
        const tab = carry.view.itemAtIndex(at)
        if (!tab)
            return at
        const left = at > 0 ? carry.view.itemAtIndex(at - 1) : null
        if (left && carry.heldX < left.x + left.width / 2) {
            carry.tabsModel.moveTab(at, at - 1)
            return at - 1
        }
        const right = at + 1 < carry.view.count ? carry.view.itemAtIndex(at + 1) : null
        if (right && carry.heldX + tab.width > right.x + right.width / 2) {
            carry.tabsModel.moveTab(at, at + 1)
            return at + 1
        }
        return at
    }

    /// Set down. The order is already what it is going to be — the tab only stops being drawn away from its own row,
    /// and the strip stops travelling with it.
    function dropTab() {
        carry.heldId = -1
        carry.heldPush = 0
    }

    // The strip travels while a tab is held past the end of the run. One frame a tick and the sensitivity the app's
    // other autoscroll reads (`MiddleAutoScroll`, デザイン規約 §グラフを横へ送る): the two are the same gesture seen
    // from different ends — a hand asking for somewhere it cannot reach, and the distance saying how badly.
    Timer {
        interval: 16
        repeat: true
        running: carry.heldId >= 0 && carry.heldPush !== 0 && carry.view.contentWidth > carry.view.width
        onTriggered: carry.driftRun()
    }
}
