pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// What a tab costs before its name, and what a name is allowed. The strip lays out by these numbers and the delegate
// sets its margins by them — one copy, so the two agree. A new dimension goes here; `TabStrip` only declares seats and
// forwards. A tab's width is the sum of its real parts (`TabItemDelegate.width`), not a name plus `2 * spaceSm`: that
// disagreed with the margins' total, and what could not be handed out fell after the last part, widening the right
// side only. Measuring (font and tokens, here) and handing out (`TabShare`, numbers only) are two objects.
QtObject {
    id: tabMetrics

    /// The seat the closing mark stands in: a step above the other closing marks' `iconSm`
    /// (デザイン規約 §寸法「タブの `✕` だけは 1 段上」; user decision — ask before changing: a browser tab's `✕` size,
    /// the one closing mark meant to be aimed at). Its ink stays the others' `textSecondary`. The seat is `iconMd`
    /// too, not `iconLg`: the wider seat's air outruns the tab's `spaceXs` step (a negative gap), so the seat would
    /// hang into the next tab and take its hover and wash.
    readonly property int markSeat: Theme.iconMd
    /// The air that seat still holds around the ink, asked of the mark (デザイン規約 §余白). `NavIcon.inkRight` is
    /// arithmetic, so it answers with no scene around it. Not a near token: the `iconMd` seat holds 3.75px, and
    /// `spaceXs` (4) written in its place would draw the `✕` that much towards both neighbours.
    readonly property NavIcon closeMark: NavIcon {
        kind: "close"
        width: tabMetrics.markSeat
        height: tabMetrics.markSeat
    }
    readonly property real markAir: tabMetrics.closeMark.width - tabMetrics.closeMark.inkRight
    /// What the tab spends either side of that ink so each gap around the `✕` comes to one step, the same as the
    /// name's near margin (デザイン規約 §余白).
    readonly property real markGap: Theme.spaceXs - tabMetrics.markAir
    /// The name's near-side step. Always kept: the far side comes down to it (`TabShare`'s order).
    readonly property real tabPadL: Theme.spaceXs
    /// The room the far side keeps for the mark: the seat and a step either side of the ink. The first thing a crowded
    /// strip takes back, down to `markRoomMin`, before any name is cut (`TabShare`); under it the name runs on under
    /// the mark, into the fade (`fadeChars`).
    readonly property real markRoomFull: 2 * tabMetrics.markGap + tabMetrics.markSeat
    readonly property real markRoomMin: tabMetrics.tabPadL

    /// The step between the repository's name and the run's mark — a row's step, since the two runs are two things
    /// the tab says (デザイン規約 §余白).
    readonly property real treeRunGap: Theme.spaceXs

    /// How many ways the run is cut for one name's ceiling (デザイン規約 §レイアウト初期値). A share, not a width: a
    /// fixed width is generous in a wide window and the whole strip in a narrow one, and a pixel ceiling cuts names
    /// while the band still has room — a tab is built at its name's width, so the run a cut frees goes to nobody.
    /// Not a browser's (Chrome / Vivaldi) pixel ceiling either: their tabs lean on a favicon, here the name is all a
    /// tab says. At most one part per tab (`TabShare.ceiling`), so a lone tab in a narrow band is not held to a third
    /// of a run nobody else uses — the same "room left, name cut". A share alone does not end that either: it caps one
    /// tab's take, and past it the max-min sharing (`TabShare.widest`) decides.
    readonly property int titleRunParts: 3
    /// The shortest a name is cut to, in characters (同表), so the pixels come out of each platform's font and zoom: a
    /// floor in pixels shows no letter at all on one of them. The pixels differ per platform by design (`fontMd`:
    /// Yu Gothic UI 58px on a real Windows window, Noto Sans JP 68px headless, Noto Sans CJK JP 66px on Linux).
    /// Three at each end, since the cut is in the middle (デザイン規約 §タブの所作): a name says itself at both ends,
    /// and one end's worth stops where neither end can be read. Held under `titleEaseChars` — a floor above it makes
    /// the shortest name the strip allows longer than the ones it calls short. Raising it only starts the scroll
    /// sooner; the window's floor does not move (the page's floor wins, 規約 §窓の床) — accepted (user decision).
    readonly property int titleMinChars: 6
    /// Measured in `n`s, not `averageCharacterWidth` (rules-refs/app-ui.md「字数の床の値付けは実測の字送り」).
    /// Pushed, for `titleEaseFullW`'s reason.
    function titleMinW() {
        return Math.ceil(tabMetrics.titleFont.advanceWidth("…")
                         + tabMetrics.titleMinChars * tabMetrics.titleFont.advanceWidth("n"))
    }
    /// Where a name stops being helped along (同表): below this length the tab still narrows, at `titleEaseShare` of the
    /// rate — a short name keeps a short tab (padding them all to one seat makes a row of equal blanks; no floor —
    /// user decision). Without the help a strip of short names narrower than their marks reads as a row of marks.
    /// Counted in `n`s too: the average advance is the whole family's, CJK included, and this side hands out air, so
    /// whatever it comes out wide is wasted.
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

    /// How much of that air goes on the mark's side: half while the mark keeps its whole room — the air opens where
    /// the name sits, since the `✕` stands its step off the far edge whatever the name is; as the strip takes the
    /// room back, the mark's side fills first — so the name that ends up under the mark is a long one, never a short
    /// one with room to spare.
    function easeRight(ease, markRoom) {
        return tabMetrics.share.easeRight(ease, markRoom)
    }

    /// How far apart a short name's letters are set, so the extra air belongs to the name — air round the name alone
    /// leaves the word pushed into a corner of a wider box. Scaled by how far short it is in letters, not width — the
    /// air is computed from the width (a loop). Only slightly: letters set further apart start to read as another
    /// face (user decision).
    function titleTracking(chars) {
        if (chars >= tabMetrics.titleEaseChars)
            return 0
        return Theme.tracking * (tabMetrics.titleEaseChars - chars) / tabMetrics.titleEaseChars
    }

    /// How far a name goes quiet before it reaches the mark over it, in letters, counted in the easing's `n`: it
    /// begins that far before the mark's whole air (`spaceXs`) and is 0 where that air begins. The floor's count at one
    /// end, since the quiet eats the tail — so it never takes more than the end the floor keeps. The stand-in's edge
    /// dissolve (`TabPin.dissolveW`) uses the same count — two dissolve lengths in one band read as two materials.
    /// `fadeW` is pushed, for `titleEaseFullW`'s reason.
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
