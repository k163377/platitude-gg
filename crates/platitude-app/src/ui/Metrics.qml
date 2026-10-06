// Graph / interaction constants, each beside the reason it is that number. The design document's
// グラフ・インタラクション定数 and 進行中・長押しの定数 tables quote these values (`cargo xtask docs --sync`).
//
// Tiered like `Theme` (規約 §トークンの三層). Nothing here belongs to one named screen, so there is no 個別 tier — a
// per-screen number goes to `Theme`'s third block.
pragma Singleton

import QtQuick
import platitude.ui

QtObject {
    id: metrics

    // ---- 1. 基礎: how a gesture is answered, anywhere it is made ----
    readonly property real iconStroke: 1.5
    readonly property int spinMs: 1000
    readonly property int holdMs: 500
    readonly property int holdFillMin: Theme.spaceXs
    readonly property int holdBackMs: 150
    readonly property int opticalDrop: 1
    // The one opacity anything is dimmed to — the 45% the `*Dim` colours are (デザイン規約 §暗く落とした段).
    readonly property real dimFade: 0.45
    // How much of a round wash's radius is its rim, over which the paint falls to `dimFade` (デザイン規約 §当たり判定).
    // Less and the disc has an edge again, more and it stops being a disc.
    readonly property real washRimShare: 0.25
    // One delay everywhere a tooltip opens, so one gesture is answered the same wherever it is made.
    readonly property int tipDelayMs: 600
    // How long something opened by hover waits before closing, for the hand to finish crossing to the next target.
    // `Qt.callLater` is too short: the two hovers change on different frames, and a callLater between them closes it.
    readonly property int hoverKeepMs: 150
    // One on or off of a blink — what points at a place something just went (the discard log's seat). A quarter of
    // the second a flash takes elsewhere (animate.css `flash`: out and back twice over its 1 s).
    readonly property int blinkMs: 250

    // ---- 2. ベーシック: the ordinary window — its graph column, its rows, its menus ----
    readonly property int laneW: Theme.iconLg
    readonly property int laneInset: Theme.spaceXs
    readonly property int nodeIcon: Theme.iconLg
    readonly property int laneStroke: 2
    readonly property real identiconFill: 0.72
    readonly property int wheelRows: 3
    // The three below are one curve, read through `handSent`.
    //
    // Pixels per millisecond per distance^`middleScrollCurve` — Chromium's own (`autoscroll_controller.cc`:
    // `pow(fabs(distance), 2.2) * -0.000008`).
    readonly property real middleScrollGain: 0.000008
    // A curve, not a line: fine control near the anchor, out of the way far out.
    readonly property real middleScrollCurve: 2.2
    // The pad around the anchor where the hand is holding still — without it a pixel of drift is a speed. Per axis,
    // so a hand going straight down does not creep sideways.
    readonly property int middleScrollDeadZone: 15
    // How long a wheel notch takes to land, so the eye can follow the rows across. Chromium's own
    // (`scroll_offset_animation_curve.cc`: 9 frames / 60); `WheelGlide` draws its ease-in-out.
    readonly property int wheelGlideMs: 150
    // A scroll bar's arrow: a press sends one step, glided like a notch (`wheelGlideMs`); held, a run starts after a
    // pause and goes at one speed until the hand lets go or leaves the arrow. All three are Chromium's Fluent bar,
    // measured: the same whatever the view's height, and whether it is the page or a box inside it.
    readonly property int arrowStep: 40
    readonly property int arrowRepeatDelayMs: 250
    // Pixels per second: a step twenty times a second.
    readonly property int arrowRepeatSpeed: 800
    // A press on a scroll bar's track sends this share of the view, glided; held, pages run on twenty a second after
    // the arrow's pause, until the thumb reaches the hand. Chromium's Fluent bar, measured (787 of a 900 view, 437 of
    // 500).
    readonly property real trackPageShare: 0.875
    // The chip column's default width; its floor is a character count measured from the font in use.
    readonly property int labelColW: 152
    readonly property int graphDefaultLanes: 12
    readonly property int messageMinW: 160
    readonly property int askFieldMinW: 160
    readonly property int hoverBodyRows: 4
    readonly property int detailsAvatar: 40
    readonly property int anchorDelayMs: 50
    // Arrow-key stepping settles here before the details load, so a held key loads once. It has to outlast every OS's
    // key-repeat interval (macOS 15ms / X11 25ms / Windows ≈32ms); the first repeat's delay is longer everywhere, so a
    // first press has always settled.
    readonly property int keyStepSettleMs: 150
    // How often the repository is read while the page is on screen is the session's, not a constant here: the pace
    // follows how heavy the reads are (`session::pace`).
    // The badge's `n/m` while a replay runs. It reads two files and starts no process; faster than this costs the
    // notify side — the work tree model's one `changed()` re-pulls every binding on the page.
    readonly property int opProgressMs: 250
    // Wide enough that a long-press fill crossing it reads as progress.
    readonly property int menuMinW: 160
    // 1 on, 1 off — the dashes of a WIP or stash edge and a WIP node's outline.
    readonly property var laneDash: [1, 1]

    // ---- outside the tiers: nothing is drawn with this ----
    // How often the window offers its shape to be written down (only if something moved) — short enough that a crash
    // loses nothing anyone would miss. Not drawn, so the design document has no seat for it.
    readonly property int stateFlushMs: 2000

    /// How far a hand pointing `away` pixels past what it can reach sends the thing under it in `ms` milliseconds
    /// (デザイン規約 §グラフを横へ送る / §タブの所作). Signed like `away`, zero inside the dead zone. The one curve for
    /// both the middle-click autoscroll and a tab carried past the band's edge.
    function handSent(away, ms) {
        const off = Math.abs(away)
        if (off <= metrics.middleScrollDeadZone)
            return 0
        return Math.sign(away) * Math.pow(off, metrics.middleScrollCurve) * metrics.middleScrollGain * ms
    }
}
