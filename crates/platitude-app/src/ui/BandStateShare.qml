pragma ComponentBehavior: Bound

import QtQuick

// How the band's shortfall is handed out among the state badges, and nothing else: measured widths and the room in,
// the group's two answers out (デザイン規約 §ウィンドウの縁 の譲る順). No font, so every boundary is asked in one
// process (`tests/qml/tst_bandshare.qml`), not through window widths that land differently per platform font; that
// real widths reach the group is `PGG_AUTO_ACT=badges`. The costs are pushed in from `BandStateMetrics`.
QtObject {
    id: bandShare

    /// What stands between two badges, which the room has to pay for once per gap.
    required property real gap
    /// The narrowest a badge is drawn with words in it (`BandStateMetrics.minW`, pushed — that file says why).
    property real badgeMinW: 0

    /// "Nothing is narrowed", as a number so a cap compares against a badge's width without a check first.
    readonly property real uncapped: Number.MAX_VALUE

    /// The max-min share of the room left after the gaps: the widest badges give way to one equal width. `want` is
    /// every standing badge's natural width, sorted on a copy — the caller's list is the row's order. Whole pixels
    /// (`BandStateMetrics` says why).
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

    /// Whether the words are given up: narrowed past the floor, fewer than three tabs in view of a scrolling strip, or
    /// the window on its floor (規約 §ウィンドウの縁).
    function folded(cap, tabContentWidth, tabRunAvail, tabCount, windowAtFloor) {
        const scrolling = tabContentWidth > tabRunAvail
        return cap < bandShare.badgeMinW
            || (scrolling && bandShare.tabsInView(tabContentWidth, tabRunAvail, tabCount) < 3)
            || windowAtFloor
    }
}
