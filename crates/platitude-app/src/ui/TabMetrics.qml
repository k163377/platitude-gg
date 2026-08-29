pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// What a tab costs before its name, and what a name is allowed. Nothing here draws: the strip lays out by these
// numbers, the delegate sets its margins by them, and both read the one copy so they agree on what a tab costs
// (手本 `GraphColumnMetrics`).
QtObject {
    id: tabMetrics

    /// The seat the closing mark stands in — held to its ink rather than to the `iconLg` box it gets everywhere else
    /// (デザイン規約 §寸法; `TabItemDelegate` carries the reasoning).
    readonly property int markSeat: Theme.iconSm
    /// The air that seat still holds around the ink, asked of the mark rather than written down as a token beside it:
    /// `close` is two diagonals set well inside their box, so the `spaceXs` it was once called was a pixel and a half
    /// out. The per-kind knowledge is `NavIcon`'s and stays there — `inkRight` is arithmetic on
    /// the kind, the size and the stroke, so this answers with no scene around it.
    readonly property NavIcon closeMark: NavIcon {
        kind: "close"
        width: tabMetrics.markSeat
        height: tabMetrics.markSeat
    }
    readonly property real markAir: tabMetrics.closeMark.width - tabMetrics.closeMark.inkRight
    /// What the tab spends either side of that ink to make each gap its own step, so the `✕` stands as far from the
    /// name and from the tab's far edge as the name stands from the near one (デザイン規約 §余白).
    readonly property real markGap: Theme.spaceXs - tabMetrics.markAir
    /// The two margins, and what a tab costs before its name has a single letter in it — those, the gap before the
    /// mark, and the mark.
    readonly property real tabPadW: Theme.spaceXs + tabMetrics.markGap
    readonly property real tabFixedW: tabMetrics.tabPadW + tabMetrics.markGap + tabMetrics.markSeat

    /// The longest a tab's name is ever drawn (デザイン規約 レイアウト初期値).
    readonly property int titleMaxW: 180
    /// The shortest, in characters rather than pixels (同表): the same count costs a different number of pixels in each
    /// platform's UI font and at every scaling, so the length comes out of the font.
    readonly property int titleMinChars: 3
    readonly property int titleMinW: Math.ceil(tabMetrics.titleFont.advanceWidth("…")
        + tabMetrics.titleMinChars * tabMetrics.titleFont.averageCharacterWidth)
    /// Where a name stops being helped along (同表). A tab drawn to `ui` comes out narrower than the marks either side
    /// of it, so below this length the tab still narrows — just at half the rate (`titleEaseShare`). **Not a floor**: a
    /// short name is meant to make a short tab, and a seat every one of them was padded out to made a row of equal
    /// blanks. Counted in `n`s rather than in the average the floor above uses: that one cuts a
    /// name down, where an average running wide only leaves more of the name standing, and this one hands out air
    /// (rules-refs/app-ui.md carries the measurement).
    readonly property int titleEaseChars: 8
    readonly property real titleEaseShare: 0.5
    /// Pushed rather than bound: `advanceWidth` is a method, so a binding on it never re-evaluates and freezes at the
    /// default font's answer (rules-refs/app-ui.md §FontMetrics). `TabStrip.settleTitleCap` is where it is called from,
    /// which is already the pushed form.
    function titleEaseW() {
        return Math.min(Math.ceil(tabMetrics.titleEaseChars * tabMetrics.titleFont.advanceWidth("n")),
                        tabMetrics.titleMaxW)
    }
    /// The air a name of this width is given on top of itself — half of what it falls short by, split evenly over its
    /// two sides. Never past the cap: a strip short of run is already cutting names, and air added there would be paid
    /// for in letters (`TabStrip.settleTitleCap` and the delegate both come through here, off the same numbers).
    function titleEase(naturalW, cap, easeW) {
        return Math.round(Math.max(0, Math.min(easeW, cap) - naturalW) * tabMetrics.titleEaseShare)
    }

    /// How far apart a short name's letters are set, at its shortest. Part of the same easing and for the same reason:
    /// a two-letter name in a tab wider than itself reads as a word pushed into a corner, and opening the letters lets
    /// the extra air belong to the name instead of standing beside it. Scaled by how far short the name is **in
    /// letters** — never off its width, which is what the air is computed from (a loop). Kept small on purpose: past a
    /// point tracking stops reading as air and starts reading as a different typeface.
    function titleTracking(chars) {
        if (chars >= tabMetrics.titleEaseChars)
            return 0
        return Theme.tracking * (tabMetrics.titleEaseChars - chars) / tabMetrics.titleEaseChars
    }

    /// The font the strip draws its names in, for the floors above to be measured in.
    readonly property FontMetrics titleFont: FontMetrics {
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontMd
    }
}
