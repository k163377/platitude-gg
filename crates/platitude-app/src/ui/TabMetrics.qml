pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// What a tab costs before its name, and what a name is allowed. The strip lays out by these numbers and the delegate
// sets its margins by them — one copy, so the two agree.
QtObject {
    id: tabMetrics

    /// The seat the closing mark stands in: a step above the other closing marks' `iconSm`
    /// (デザイン規約 §寸法「タブの `✕` だけは 1 段上」).
    readonly property int markSeat: Theme.iconMd
    /// The air that seat still holds around the ink, asked of the mark (デザイン規約 §余白). `NavIcon.inkRight` is
    /// arithmetic, so it answers with no scene around it.
    readonly property NavIcon closeMark: NavIcon {
        kind: "close"
        width: tabMetrics.markSeat
        height: tabMetrics.markSeat
    }
    readonly property real markAir: tabMetrics.closeMark.width - tabMetrics.closeMark.inkRight
    /// What the tab spends either side of that ink so each gap around the `✕` comes to one step, the same as the
    /// name's near margin (デザイン規約 §余白).
    readonly property real markGap: Theme.spaceXs - tabMetrics.markAir
    /// The name's near-side step. Always kept: the far side comes down to it (デザイン規約 §ウィンドウの縁).
    readonly property real tabPadL: Theme.spaceXs
    /// The room the far side keeps for the mark: the seat and a step either side of the ink. The first thing a crowded
    /// strip takes back, down to `markRoomMin`, before any name is cut (同§); under it the name runs on under the
    /// mark, into the fade (`fadeChars`).
    readonly property real markRoomFull: 2 * tabMetrics.markGap + tabMetrics.markSeat
    readonly property real markRoomMin: tabMetrics.tabPadL

    /// The seat of the mark naming the copy a tab stands in: the step of the word it is set in, `fontSm`
    /// (デザイン規約 §寸法). At `iconSm` the crown stands taller than a `fontSm` capital (9.75 against 8.6px in
    /// Yu Gothic UI) and reads as a picture dropped into the word; `iconXs` comes out at 8.13.
    ///
    /// Nothing is spent either side of it: it is set in the run as a letter is, not stood in a row
    /// (デザイン規約 §余白 / §タブの所作).
    readonly property int treeSeat: Theme.iconXs
    /// The step between the repository's name and the run's mark — a row's step, since the two runs are two things
    /// the tab says (デザイン規約 §余白).
    readonly property real treeRunGap: Theme.spaceXs

    /// How many ways the run is cut for one name's ceiling (デザイン規約 §レイアウト初期値). A share, not a width: a
    /// fixed width is generous in a wide window and the whole strip in a narrow one. At most one part per tab
    /// (`TabShare.ceiling`), so a lone tab in a narrow band is not held to a third of a run nobody else uses.
    readonly property int titleRunParts: 3
    /// The shortest a name is cut to, in characters (同表), so the pixels come out of each platform's font: three at
    /// each end, since the cut is in the middle (デザイン規約 §タブの所作). Held under `titleEaseChars` — a floor above
    /// it makes the shortest name the strip allows longer than the ones it calls short.
    readonly property int titleMinChars: 6
    /// Measured in `n`s, not `averageCharacterWidth` (rules-refs/app-ui.md「字数の床の値付けは実測の字送り」).
    /// Pushed, for `titleEaseFullW`'s reason.
    function titleMinW() {
        return Math.ceil(tabMetrics.titleFont.advanceWidth("…")
                         + tabMetrics.titleMinChars * tabMetrics.titleFont.advanceWidth("n"))
    }
    /// Where a name stops being helped along (同表): below this length the tab still narrows, at `titleEaseShare` of the
    /// rate — a short name keeps a short tab (padding them all to one seat makes a row of equal blanks).
    readonly property int titleEaseChars: 8
    readonly property real titleEaseShare: 0.5
    /// That length in pixels, uncapped — also the width the strip asks at (`TabShare.settle`).
    ///
    /// Pushed: `advanceWidth` is a method, so a binding on it freezes at the default font's answer
    /// (rules-refs/app-ui.md「`FontMetrics.advanceWidth()` も同じ側」).
    function titleEaseFullW() {
        return Math.ceil(tabMetrics.titleEaseChars * tabMetrics.titleFont.advanceWidth("n"))
    }
    /// The air a name of this width is given on top of itself: half of what it falls short by, held to the cap (air
    /// past it would be paid for in letters). The tab and the stand-in come through here and the strip's pass calls
    /// `TabShare.ease` directly — one implementation, so a tab's drawn width and the run it was handed cannot disagree.
    function titleEase(naturalW, cap, easeW) {
        return tabMetrics.share.ease(naturalW, cap, easeW)
    }

    /// How much of that air goes on the mark's side: half while the mark keeps its whole room; as the strip takes the
    /// room back, the mark's side fills first — so the name that ends up under the mark is a long one, never a short
    /// one with room to spare.
    function easeRight(ease, markRoom) {
        return tabMetrics.share.easeRight(ease, markRoom)
    }

    /// How far apart a short name's letters are set, so the extra air belongs to the name. Scaled by how far short it
    /// is in letters, not width — the air is computed from the width (a loop).
    function titleTracking(chars) {
        if (chars >= tabMetrics.titleEaseChars)
            return 0
        return Theme.tracking * (tabMetrics.titleEaseChars - chars) / tabMetrics.titleEaseChars
    }

    /// How far a name goes quiet before it reaches the mark over it, in letters (デザイン規約 §レイアウト初期値): the
    /// floor's count at one end, since the quiet eats the tail. The stand-in's edge dissolve (`TabPin.dissolveW`) uses
    /// the same count — two dissolve lengths in one band read as two materials. `fadeW` is pushed, for
    /// `titleEaseFullW`'s reason.
    readonly property int fadeChars: 3
    function fadeW() {
        return Math.ceil(tabMetrics.fadeChars * tabMetrics.titleFont.advanceWidth("n"))
    }

    /// The arithmetic that hands the run out (`TabShare`), fed this object's costs: bound here, except the font's
    /// answers, which [`priceShare`] pushes.
    readonly property TabShare share: TabShare {
        padL: tabMetrics.tabPadL
        markRoomFull: tabMetrics.markRoomFull
        markRoomMin: tabMetrics.markRoomMin
        runParts: tabMetrics.titleRunParts
        easeShare: tabMetrics.titleEaseShare
    }
    /// Pushes the font's two answers into `share` on every call, not once: the family arrives after the first pass,
    /// and a value read once stays the default font's.
    function priceShare() {
        tabMetrics.share.minW = tabMetrics.titleMinW()
        tabMetrics.share.easeFullW = tabMetrics.titleEaseFullW()
    }

    /// One pass over the names the strip measured (`TabShare.settle`). The only door into it: nothing else pushes the
    /// font's answers in first.
    function settle(nat, run) {
        tabMetrics.priceShare()
        return tabMetrics.share.settle(nat, run)
    }

    readonly property FontMetrics titleFont: FontMetrics {
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontMd
    }
}
