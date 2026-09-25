pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The hand that carries a tab: which tab is in it, where it has got to, and what the strip's order does about that
// (デザイン規約 §タブの所作). The strip owns the tabs and says what a press means; this owns the gesture between the
// taking up and the setting down. An `Item` only so the drift `Timer` has somewhere to stand; it draws nothing.
Item {
    id: carry

    /// The strip's list and model: a carry reads where the tabs sit and asks the model to reorder them, and owns
    /// neither.
    required property ListView view
    required property var tabsModel

    /// The tab in hand — by id, since the order changes under a drag — where inside it the hand took hold, and where
    /// its left edge has been carried to in the strip's coordinates.
    property int heldId: -1
    property real heldGrabX: 0
    property real heldX: 0
    /// Where the hand last reported in the scene, and how far past the run it is asking — the distance the strip
    /// travels at (デザイン規約 §タブの所作).
    property real heldSceneX: 0
    property real heldPush: 0

    /// The tab at `index`, taken up past the platform's threshold; `grabX` keeps that point of the tab under the hand.
    function takeTab(index, grabX) {
        const tab = carry.view.itemAtIndex(index)
        if (!tab)
            return
        carry.heldId = tab.tab_id
        carry.heldGrabX = grabX
        carry.heldX = tab.x
    }

    /// The hand, moved to `sceneX` with a tab in it — scene coordinates, since the tab moves with the hand. A tab is
    /// put no further than the run on screen; past either end it stays at the edge and the strip travels under it
    /// (`driftRun`).
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
        // The run from the list's origin (`TabRun.clamp`).
        const origin = carry.view.originX
        const room = carry.view.contentWidth - carry.view.width
        const asked = carry.view.contentX + Metrics.handSent(carry.heldPush, run.interval)
        const settled = Math.max(origin, Math.min(asked, origin + room))
        if (settled === carry.view.contentX)
            return
        carry.view.contentX = settled
        // The hand has not moved; what it is pointing into the strip at has.
        carry.carryTab(at, carry.heldSceneX)
    }

    /// Where the tab in hand sits right now, or -1 with nothing in hand. Walked: the carry is what changes it.
    function heldIndex() {
        for (let i = 0; i < carry.view.count; i++) {
            const tab = carry.view.itemAtIndex(i)
            if (tab && tab.tab_id === carry.heldId)
                return i
        }
        return -1
    }

    /// The one place a carried tab is put: where it is drawn, and which neighbours it has passed. The hand (`carryTab`)
    /// and the smoke hooks (the strip's) both come through here, so neither can reach an order the other cannot.
    function carryTo(index, left) {
        const tab = carry.view.itemAtIndex(index)
        if (!tab)
            return index
        // A tab carried past either end of the strip stops there, as everything that scrolls here does
        // (デザイン規約 §QML 実装ルール).
        const origin = carry.view.originX
        carry.heldX = Math.max(origin, Math.min(left, origin + carry.view.contentWidth - tab.width))
        let at = index
        // Each step passes one tab, so the tab count bounds the loop.
        for (let step = 0; step < carry.view.count; step++) {
            const next = carry.stepOrder(at)
            if (next === at)
                break
            at = next
            // The order changed: the next comparison has to read the places the view has just given the tabs.
            carry.view.forceLayout()
        }
        return at
    }

    /// One neighbour, passed or not: the carried tab's leading edge against the neighbour's middle. Middle against
    /// middle fails for tabs of different widths — a wide tab held at the strip's end never reaches a narrow last tab's
    /// middle.
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

    /// Set down: the order is already final, so the tab only returns to its row and the strip stops travelling.
    function dropTab() {
        carry.heldId = -1
        carry.heldPush = 0
    }

    // The strip travels while a tab is held past the end of the run: one frame a tick, on the app's other autoscroll
    // curve (`Metrics.handSent`, デザイン規約 §グラフを横へ送る), dead zone included — the first pixels past the edge
    // are a hand steadying a tab on the last place.
    Timer {
        id: run
        interval: 16
        repeat: true
        running: carry.heldId >= 0 && carry.heldPush !== 0 && carry.view.contentWidth > carry.view.width
        onTriggered: carry.driftRun()
    }
}
