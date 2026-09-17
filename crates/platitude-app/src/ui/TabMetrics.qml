pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// What a tab costs before its name, and what a name is allowed. Nothing here draws: the strip lays out by these
// numbers, the delegate sets its margins by them, and both read the one copy so they agree on what a tab costs
// (after `GraphColumnMetrics`).
QtObject {
    id: tabMetrics

    /// The seat the closing mark stands in — held to its ink
    /// (デザイン規約 §寸法; `TabItemDelegate` carries the reasoning). A step above the `iconSm` the window's other
    /// closing marks take: a tab's `✕` is aimed at (§寸法「タブの `✕` だけは 1 段上」).
    readonly property int markSeat: Theme.iconMd
    /// The air that seat still holds around the ink, asked of the mark:
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
    /// The step the name is set at on the near side. Always kept: it is the one the far side comes down to
    /// (デザイン規約 §ウィンドウの縁).
    readonly property real tabPadL: Theme.spaceXs
    /// The room the far side keeps for the mark when the strip has it to give — the seat, and the same step either
    /// side of the ink. **The first thing a crowded strip takes back**, down to `markRoomMin`, before a single name is
    /// cut (同§). Under that room the name runs on under the mark, and what says so is the fade below.
    readonly property real markRoomFull: 2 * tabMetrics.markGap + tabMetrics.markSeat
    readonly property real markRoomMin: tabMetrics.tabPadL

    /// How many ways the run is cut for one name's ceiling (デザイン規約 レイアウト初期値). A share:
    /// what makes one tab too wide is how much of the band it is holding, and the band is not one size —
    /// a width written here is generous in a wide window and the whole strip in a narrow one.
    readonly property int titleRunParts: 3
    /// That share of the run the strip has — **cut into at most one part per tab**, so the last tab in a
    /// narrow band is not held to a third of a run nobody else is standing in (which is the cut-with-run-to-spare
    /// this ceiling exists to end). Floored at the width a name still says something at: in a window narrow enough
    /// the two meet, and the floor is the one with letters behind it.
    ///
    /// Read off the run the tabs are handed — `TabStrip.tabsWantWidth` asks at
    /// the names' full width for this reason, since a width asked for out of the run it is about to be given back is
    /// one that moves every time it is read.
    function titleCeilingW(run, tabs) {
        const parts = Math.max(1, Math.min(tabMetrics.titleRunParts, tabs))
        return Math.max(tabMetrics.titleMinW(), Math.floor(run / parts))
    }
    /// The shortest, in characters (同表): the same count costs a different number of pixels in each
    /// platform's UI font and at every scaling, so the length comes out of the font. **Three at each end** — the cut is
    /// in the middle (デザイン規約 §タブの所作), so a name has two ends to say itself with and the floor has to hold
    /// both. Below `titleEaseChars`, which is the ceiling on this: a floor above the length names stop being eased at
    /// would make the shortest name the strip allows longer than the ones it calls short.
    readonly property int titleMinChars: 6
    /// Measured in `n`s: `averageCharacterWidth` is the **font's** average, and every family named for
    /// this UI carries Japanese, so it answers with a full-width figure no repository name is written in — and a
    /// different one per platform (rules-refs/app-ui.md carries the measurement). A floor read off it moves with the
    /// font, and where it runs wide it takes letters off the name it stands there to keep.
    ///
    /// Pushed, for `titleEaseFullW`'s reason (`TabStrip.settleTitleCap` is where it is called from).
    function titleMinW() {
        return Math.ceil(tabMetrics.titleFont.advanceWidth("…")
                         + tabMetrics.titleMinChars * tabMetrics.titleFont.advanceWidth("n"))
    }
    /// Where a name stops being helped along (同表). A tab drawn to `ui` comes out narrower than the marks either side
    /// of it, so below this length the tab still narrows — just at half the rate (`titleEaseShare`). **A short name
    /// keeps a short tab**, and a seat every one of them was padded out to made a row of equal
    /// blanks. Counted in `n`s, like the floor above and for its reason.
    readonly property int titleEaseChars: 8
    readonly property real titleEaseShare: 0.5
    /// That length in pixels, with nothing over it — the width the strip **asks** at: the ask is made at the names'
    /// own width, and the ceiling the form below is held under is cut out of the very run the ask is about to be
    /// answered with.
    ///
    /// Pushed: `advanceWidth` is a method, so a binding on it never re-evaluates and freezes at the
    /// default font's answer (rules-refs/app-ui.md §FontMetrics). `TabStrip.settleTitleCap` is where both are called
    /// from, which is already the pushed form.
    function titleEaseFullW() {
        return Math.ceil(tabMetrics.titleEaseChars * tabMetrics.titleFont.advanceWidth("n"))
    }
    /// And the same length held under the ceiling, which is what a tab being laid out is eased by: air added past the
    /// width a name is cut at would be paid for in that name's own letters.
    function titleEaseW(ceiling) {
        return Math.min(tabMetrics.titleEaseFullW(), ceiling)
    }
    /// The air a name of this width is given on top of itself — half of what it falls short by, split evenly over its
    /// two sides. Held to the cap: a strip short of run is already cutting names, and air added there would be paid
    /// for in letters (`TabStrip.settleTitleCap` and the delegate both come through here, off the same numbers).
    function titleEase(naturalW, cap, easeW) {
        return Math.round(Math.max(0, Math.min(easeW, cap) - naturalW) * tabMetrics.titleEaseShare)
    }

    /// Which side of the name that air is set down on. Half and half while the mark keeps its whole room; as the strip
    /// takes that room back the air moves to the **mark's** side first, and only what is left over stays on the near
    /// one. A short name has room to spare in its own tab, so the one that ends up under the mark is a long one —
    /// what the strip is short of is run for all of them, which the tab has already given up in its width.
    function easeRight(ease, markRoom) {
        return Math.max(ease / 2, Math.min(ease, tabMetrics.markRoomFull - markRoom))
    }

    /// How far apart a short name's letters are set, at its shortest. Part of the same easing and for the same reason:
    /// a two-letter name in a tab wider than itself reads as a word pushed into a corner, and opening the letters lets
    /// the extra air belong to the name. Scaled by how far short the name is **in
    /// letters** — its width is what the air is computed from (a loop). Kept small on purpose: past a
    /// point tracking stops reading as air and starts reading as a different typeface.
    function titleTracking(chars) {
        if (chars >= tabMetrics.titleEaseChars)
            return 0
        return Theme.tracking * (tabMetrics.titleEaseChars - chars) / tabMetrics.titleEaseChars
    }

    /// How far a name goes quiet before it reaches the mark standing over it — **in letters**, the measure
    /// the floor and the easing above are already counted in (デザイン規約 レイアウト初期値). The count the floor keeps at
    /// **one end**: what the quiet reaches for is the tail, so it eats at most what that end is allowed
    /// to be.
    ///
    /// **The same count answers the stand-in's own edge** (`TabPin.dissolveW`), where the dark lets go over the
    /// run. What the two hide is different — this tab's tail, against the tabs behind it —
    /// but one band holding two dissolve lengths reads as two materials, so there is one number for both.
    ///
    /// Pushed, for `titleEaseFullW`'s reason (`TabStrip.settleTitleCap` is where it is called from).
    readonly property int fadeChars: 3
    function fadeW() {
        return Math.ceil(tabMetrics.fadeChars * tabMetrics.titleFont.advanceWidth("n"))
    }

    /// The font the strip draws its names in, for the floors above to be measured in.
    readonly property FontMetrics titleFont: FontMetrics {
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontMd
    }
}
