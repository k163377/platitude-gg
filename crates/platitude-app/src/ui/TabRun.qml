pragma ComponentBehavior: Bound

import QtQuick

// Where the strip's run has scrolled to, its two ends, and the travel that moves it with nothing in hand (the carry
// is `TabCarry`'s).
QtObject {
    id: tabRun

    required property ListView view

    readonly property bool travelling: tabRun.travel.running

    /// An offset held inside the run: `contentX` is written by hand, and one outside the run draws blank band past the
    /// last tab. The run starts at `originX`, not 0 — when tabs before the visible ones change width the list moves its
    /// origin instead, and a bound from 0 stops a travel short with its tab still cut.
    function clamp(x) {
        const origin = tabRun.view.originX
        return Math.max(origin, Math.min(x, origin + Math.max(0, tabRun.view.contentWidth - tabRun.view.width)))
    }

    /// Automation (`tab-edge`): how far the strip has scrolled, and whether it is at its far end — false for a strip
    /// with nothing to scroll.
    function offset() {
        return Math.round(tabRun.view.contentX)
    }
    function atEnd() {
        return tabRun.view.contentWidth > tabRun.view.width
            && tabRun.view.contentX >= tabRun.view.originX + tabRun.view.contentWidth - tabRun.view.width - 1
    }

    /// Calls off a travel in flight: anything else about to move the strip outranks it (two writers on `contentX` draw
    /// over each other).
    function halt() {
        tabRun.travel.stop()
    }

    /// Where the strip has to stand for `tab` to be whole in the run: the least move (as `ListView.Contain`).
    function wholeAt(tab) {
        let to = tabRun.view.contentX
        if (!tab)
            return to
        if (tab.x < to)
            to = tab.x
        else if (tab.x + tab.width > to + tabRun.view.width)
            to = tab.x + tab.width - tabRun.view.width
        return tabRun.clamp(to)
    }

    /// Travels the strip until `tab` is whole in the run, animated rather than jumped (デザイン規約 §タブの所作).
    /// Answers false when there is nowhere to go, or a travel is already headed there — restarting mid-flight would
    /// begin the 200ms over.
    function showTab(tab) {
        if (!tab)
            return false
        const to = tabRun.wholeAt(tab)
        if (to === tabRun.view.contentX || (tabRun.travel.running && tabRun.travel.to === to))
            return false
        tabRun.travel.stop()
        tabRun.travel.from = tabRun.view.contentX
        tabRun.travel.to = to
        tabRun.travel.start()
        return true
    }

    /// The same landing without the travel, for a strip that has stood nowhere yet (デザイン規約 §タブの所作): there is
    /// no previous place to read the motion against.
    function landOn(tab) {
        if (!tab)
            return false
        const to = tabRun.wholeAt(tab)
        if (to === tabRun.view.contentX)
            return false
        tabRun.travel.stop()
        tabRun.view.contentX = to
        return true
    }

    /// Automation (`tab-pin`): the strip sent to the end of the run far from `tab`, so the stand-in has to appear.
    /// False for a strip that fits.
    function sendAway(tab) {
        const room = Math.max(0, tabRun.view.contentWidth - tabRun.view.width)
        if (room <= 0 || !tab)
            return false
        tabRun.travel.stop()
        const origin = tabRun.view.originX
        tabRun.view.contentX = tab.x - origin < tabRun.view.contentWidth / 2 ? origin + room : origin
        return true
    }

    // Clamped again when it stops: the run can move under a travel (a window resized mid-flight) and leave the
    // destination outside it. Every other writer of `contentX` clamps as it writes.
    readonly property NumberAnimation travel: NumberAnimation {
        target: tabRun.view
        property: "contentX"
        duration: 200
        onStopped: tabRun.view.contentX = tabRun.clamp(tabRun.view.contentX)
    }
}
