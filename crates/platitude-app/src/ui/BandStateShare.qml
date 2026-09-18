pragma ComponentBehavior: Bound

import QtQuick

// How the band's shortfall is handed out among the state badges — and nothing else. No font, no items, no window:
// what comes in is a list of widths somebody else measured and the room they are to fit in, and what goes out is the
// two answers the group is drawn by (デザイン規約 §ウィンドウの縁 の譲る順).
//
// **Split off from the measuring for what it lets be asked** (`TabShare` is the same split, one cell along the same
// row). Which of the group's three shapes a window lands in is arithmetic over measured widths, so a run that names
// a window width is asking about the platform's font as much as about the rule — and the shape it reaches moves
// between machines while the run goes on passing, since a report line nothing is judged against says nothing
// (measured 2026-09-18: `badges 1000:16` folded to the mark on Windows and kept its words on Linux, so the folded
// shape was never photographed there at all). Here the room is a number, so every boundary — nothing narrowed, some
// narrowed, exactly at the floor, under it, and the two crowds that fold the group without narrowing it — is asked
// in one process (`tests/qml/tst_bandshare.qml`).
//
// What a window is still needed for is the other half: that the widths going in are a real font's and the answers
// coming out reach the group (`PGG_AUTO_ACT=badges`).
//
// The costs are `BandStateMetrics`'s to measure and are pushed in, not read from here: this object owns no number of
// its own, so the one copy of each stays where the reasoning for it is written down.
QtObject {
    id: bandShare

    /// What stands between two badges, which the room has to pay for once per gap.
    required property real gap
    /// The narrowest a badge is drawn with words in it, measured off the platform's UI font
    /// (`BandStateMetrics.minW`) — **pushed** rather than bound, for the reason that file gives: `advanceWidth` is a
    /// method and a binding on one freezes at the default font's answer (rules-refs/app-ui.md §FontMetrics).
    property real badgeMinW: 0

    /// What nothing being narrowed is spelled as. A number, so the caller can compare a cap against a badge's own
    /// width without asking first whether there is one.
    readonly property real uncapped: Number.MAX_VALUE

    /// The widest every badge may be drawn and still leave room for all of them: the max-min share of the room left
    /// once the gaps between them are paid for. The widest badges give way and come out equal; the ones already
    /// under their share keep their own width (同§). `want` is every standing badge at its natural width, in any
    /// order — this makes its own sorted copy, since the caller's list is the row's own order.
    ///
    /// Whole pixels, for the reason `BandStateMetrics` gives: a ceiling summed from fractions comes out under what
    /// the same widths add up to once each is rounded, and every word then elides in a band with room to spare.
    function cap(want, room) {
        if (want.length === 0)
            return bandShare.uncapped
        const sorted = want.slice().sort((a, b) => a - b)
        let left = Math.floor(room) - (sorted.length - 1) * bandShare.gap
        for (let i = 0; i < sorted.length; i++) {
            const share = Math.floor(left / (sorted.length - i))
            if (sorted[i] > share)
                return share
            left -= sorted[i]
        }
        return bandShare.uncapped
    }

    /// How many tabs the strip has room for as they are drawn now. Off their real width: a cap is a ceiling the names
    /// may be nowhere near.
    function tabsInView(tabContentWidth, tabRunAvail, tabCount) {
        const each = tabCount > 0 ? tabContentWidth / tabCount : 0
        return each > 0 ? Math.floor(tabRunAvail / each) : 3
    }

    /// Whether the words are given up altogether. **Three ways in, and two of them are the strip's**: the badges
    /// narrowed past the width a word still says something at, a strip so crowded that three tabs no longer stand in
    /// its run, or the window on its floor — where the words go whatever else is true (規約 §ウィンドウの縁).
    function folded(cap, tabContentWidth, tabRunAvail, tabCount, windowAtFloor) {
        const scrolling = tabContentWidth > tabRunAvail
        return cap < bandShare.badgeMinW
            || (scrolling && bandShare.tabsInView(tabContentWidth, tabRunAvail, tabCount) < 3)
            || windowAtFloor
    }
}
