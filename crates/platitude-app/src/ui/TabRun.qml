pragma ComponentBehavior: Bound

import QtQuick

// The run the strip of tabs stands in: where it has got to, what its two ends are, and the travel that sends it
// somewhere. The strip owns the tabs and says what a press means, the hand owns the carry (`TabCarry`); this owns the
// one thing that moves the strip with nothing in hand.
//
// Its own file for the reason the hand has one: laying tabs out and moving the row they are laid out in are
// separate questions. A `QtObject` — nothing here draws, and nothing here needs a child of its own.
QtObject {
    id: tabRun

    /// The strip's own list. This moves it and lays nothing out.
    required property ListView view

    /// Whether the strip is still on its way somewhere.
    readonly property bool travelling: tabRun.travel.running

    /// Where in the run a given offset lands. `contentX` is assigned by hand from every place that moves the strip,
    /// and one left outside its run draws a band of nothing past the last tab. **The run starts at the list's origin,
    /// not at 0** (`originX`): when tabs before the ones on screen change width, the list keeps the ones on screen
    /// where they stand and moves its origin instead, and a bound read from 0 then stops a travel that far short of
    /// the far end, with the tab it was sent for still cut.
    function clamp(x) {
        const origin = tabRun.view.originX
        return Math.max(origin, Math.min(x, origin + Math.max(0, tabRun.view.contentWidth - tabRun.view.width)))
    }

    /// How far the strip has travelled, and whether it has reached its far end. A strip with nothing to scroll has no
    /// end to reach and answers false (`PGG_AUTO_ACT=tab-edge`).
    function offset() {
        return Math.round(tabRun.view.contentX)
    }
    function atEnd() {
        return tabRun.view.contentWidth > tabRun.view.width
            && tabRun.view.contentX >= tabRun.view.originX + tabRun.view.contentWidth - tabRun.view.width - 1
    }

    /// A travel in flight, called off. Whatever else is about to move the strip outranks it: two hands on `contentX`
    /// is one of them drawing over the other.
    function halt() {
        tabRun.travel.stop()
    }

    /// Where the strip has to stand for `tab` to be whole in the run — the least it can move and still have all of
    /// that tab on screen, which is the landing the sidebar's own rows are sent to (`NavList` / `ListView.Contain`).
    /// Its own function because two arrivals go there and differ in nothing else.
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

    /// The strip, travelled until `tab` is whole in the run. Quick (デザイン規約 §アニメーション の
    /// 200ms): a strip that jumps leaves the reader working out which way it went and how far, which is the question
    /// the press was asking. Answers false when there is nowhere to go.
    function showTab(tab) {
        if (!tab)
            return false
        const to = tabRun.wholeAt(tab)
        if (to === tabRun.view.contentX)
            return false
        tabRun.travel.stop()
        tabRun.travel.from = tabRun.view.contentX
        tabRun.travel.to = to
        tabRun.travel.start()
        return true
    }

    /// The same landing, arrived at — what a strip that has stood nowhere yet does with the
    /// tab it is handed (デザイン規約 §タブの所作). A travel is read against where the strip was, and a strip coming up
    /// has no such place, so the 200ms would be saying nothing to nobody.
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

    /// Automation: the strip, sent to whichever end of the run leaves `tab` off screen (`PGG_AUTO_ACT=tab-pin`). The
    /// far end — the near one would leave the tab standing in the run, and a stand-in that never
    /// stood is what that verb is there to catch. A strip that fits has no end to send it to and answers false.
    function sendAway(tab) {
        const room = Math.max(0, tabRun.view.contentWidth - tabRun.view.width)
        if (room <= 0 || !tab)
            return false
        tabRun.travel.stop()
        const origin = tabRun.view.originX
        tabRun.view.contentX = tab.x - origin < tabRun.view.contentWidth / 2 ? origin + room : origin
        return true
    }

    // The one place `contentX` is put back inside the run after the fact: every other hand on it clamps as it writes,
    // where a travel is settled against a run that can move under it — a window resized in flight leaves the
    // destination outside the bound it was chosen for.
    readonly property NumberAnimation travel: NumberAnimation {
        target: tabRun.view
        property: "contentX"
        duration: 200
        onStopped: tabRun.view.contentX = tabRun.clamp(tabRun.view.contentX)
    }
}
