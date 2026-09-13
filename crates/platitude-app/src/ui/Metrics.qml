// Graph / interaction constants — the timings and amounts a gesture is answered with, and beside each one the reason it
// is that number.
//
// Same standing as `Theme`: this file is where these values live, and the グラフ・インタラクション定数 and
// 進行中・長押しの定数 tables of internal-docs/デザイン規約.md quote it (`cargo xtask docs --sync`). A number that is not
// a token here does not belong in a component.
//
// Grouped by the same three tiers `Theme` uses (規約 §トークンの三層): the timings and amounts a gesture is answered
// with are 基礎 — no screen is in them — while the numbers that lay the ordinary window out are ベーシック. Nothing
// here belongs to one named screen, so this file has no 個別 tier; a per-screen number goes to `Theme`'s third block
// beside the screen it is named for.
pragma Singleton

import QtQuick
import platitude.ui

QtObject {
    // ---- 1. 基礎: how a gesture is answered, anywhere it is made ----
    readonly property real iconStroke: 1.5
    readonly property int spinMs: 1000
    readonly property int holdMs: 500
    readonly property int holdFillMin: Theme.spaceXs
    readonly property int holdBackMs: 150
    readonly property int opticalDrop: 1
    // The one opacity anything is dimmed to (デザイン規約 §暗く落とした段). Same 45% the `*Dim` colours already are — measured, not
    // chosen.
    readonly property real dimFade: 0.45
    // How much of a round wash's radius is its rim — the run over which the paint comes down to `dimFade` of itself
    // instead of stopping at a line (デザイン規約 §当たり判定). A quarter: less and the disc has an edge again, more and it
    // stops being a disc.
    readonly property real washRimShare: 0.25
    // One delay everywhere a tooltip opens: a faster or slower one per place would answer the same gesture
    // differently depending on where the hand made it.
    readonly property int tipDelayMs: 600
    // How long something opened by hover waits before closing, which is not the delay it opened with: opening
    // confirms an intent, closing waits for the hand to finish crossing to the next target. `Qt.callLater` is too
    // short — walking from a line to the card that line opened changes the two hovers on different frames, and a
    // callLater runs between them and closes the card.
    readonly property int hoverKeepMs: 150

    // ---- 2. ベーシック: the ordinary window — its graph column, its rows, its menus ----
    readonly property int laneW: Theme.iconLg
    readonly property int laneInset: Theme.spaceXs
    readonly property int nodeIcon: Theme.iconLg
    readonly property int laneStroke: 2
    readonly property real identiconFill: 0.72
    readonly property int wheelRows: 6
    // Middle-click autoscroll, and the send a dragged tab gets once it crosses the band's edge — both are "how far
    // the hand pointed past what it can reach = how fast", and two sensitivities would make one gesture run at
    // different speeds depending on where it was made.
    readonly property real middleScrollGain: 0.12
    // The chip column's default width. Its floor is not this number and is not in pixels at all: it is a count of
    // characters, measured at run time from the font in use.
    readonly property int labelColW: 152
    readonly property int graphDefaultLanes: 12
    // What the subject column always keeps, which is one of the two things that stop the graph column from taking
    // more room as the window narrows (the other is the lane ceiling).
    readonly property int messageMinW: 160
    readonly property int hoverBodyRows: 4
    readonly property int detailsAvatar: 40
    readonly property int anchorDelayMs: 50
    // Arrow-key stepping settles here before the details load, so a held key loads once at the end rather than at
    // every row. **It has to outlast every OS's key-repeat interval** (macOS 15ms / X11 25ms / Windows ≈32ms) for
    // that, and it does not decide the first step: the wait before a first repeat is longer than this everywhere, so
    // the first press has always settled before the run starts. A repeat is told from a fresh press by the key itself
    // (`KeyEvent.isAutoRepeat`), never by timing.
    readonly property int keyStepSettleMs: 150
    // Repository reload while the page is on screen. **One tick contains a `git status -uall`**, and on a
    // reference-sized repository that read is most of a second (ci/baseline/poll-cost-windows-x64.md) — the
    // measurement is what set this interval, so it is not shortened to feel more responsive.
    readonly property int pollIntervalMs: 10000
    // What the *other* working copies are carrying, on a tick of its own. **One `git status` per copy** — the same
    // read as above and as expensive, times however many copies there are — so it may not ride the interval above;
    // and a window kept open beside another copy is exactly what those rows are for, so it may not wait for somebody
    // to click the window either. Three ticks of the one above, to start from: the wall clock behind the number is
    // still to be measured (ci/baseline/perf-windows-x64.md §未取得), and a pass that outlasts this drops the next
    // tick rather than stacking (`RepoSession::refresh_carried`). **The listing itself is not this** — that is one
    // process and 23ms of it, and it rides the tick above.
    readonly property int copiesIntervalMs: 30000
    // The badge's `n/m` while a replay runs, on a tick of its own. It reads two progress files out of the git
    // directory and **starts no process**, which is why the cost above does not apply to it. Faster than this and
    // what is hit is not the read but the notify side: the work tree's model has one `changed()`, and that one
    // signal re-pulls every binding on the page.
    readonly property int opProgressMs: 250
    // Wide enough that a long-press fill crossing it reads as progress.
    readonly property int menuMinW: 160
    // 1 on, 1 off — the dashes of a WIP or stash edge and a WIP node's outline.
    readonly property var laneDash: [1, 1]

    // ---- outside the tiers: nothing is drawn with this ----
    // How often the window offers its shape to be written down. Nothing is written unless something moved, so this only
    // has to be short enough that a crash loses a layout nobody would miss. Persistence timing is not something drawn,
    // so the design document has no seat for it and this is the one value that lives here alone.
    readonly property int stateFlushMs: 2000
}
