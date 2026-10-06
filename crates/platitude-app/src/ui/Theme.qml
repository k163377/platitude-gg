// Design tokens — the values, and beside each one the reason it is that value. デザイン規約.md says which token a
// place uses; its 値 cells are generated from here (`cargo xtask docs --sync`).
//
// The set is complete on purpose: a token no code uses yet still belongs here.
//
// Three tiers, in this order (規約 §トークンの三層).
pragma Singleton

import QtQuick

QtObject {
    // ================================================================
    // 1. 基礎(全ページ共通)
    // ================================================================

    // ---- colors: backgrounds ----
    readonly property color bgBase: "#020617"
    readonly property color bgSurface: "#020617"
    readonly property color bgElevated: "#0F172A"
    // The operation panel, a second row of chrome that has to read as one. Halfway between `bgElevated` and
    // `borderSubtle`: a whole step up makes it the brightest wide surface in the window, louder than every graph row.
    readonly property color bgRaised: "#17202F"
    readonly property color bgSelected: "#172554"
    readonly property color bgHover: "#14FFFFFF"
    readonly property color bgPressed: "#1FFFFFFF"

    // ---- colors: borders ----
    readonly property color borderSubtle: "#1E293B"
    readonly property color borderDefault: "#334155"
    readonly property color borderStrong: "#475569"
    readonly property color borderFocus: "#3B82F6"

    // ---- colors: scroll bar (the style's own bar — thumb ink, drawn over the content) ----
    // One ink, landing on palette steps over `bgSurface` (デザイン規約 §スクロールバー). Fusion draws the thumb at
    // `opacity 0.75`, so the effective alpha is 0.75 of the one here (and the idle bar a further 3/10 of that).
    readonly property color scrollBarThumb: "#867D9CB4"
    readonly property color scrollBarThumbHeld: "#B47D9CB4"

    // ---- colors: text ----
    readonly property color textPrimary: "#E2E8F0"
    readonly property color textSecondary: "#94A3B8"
    readonly property color textMuted: "#64748B"
    readonly property color textOnAccent: "#FFFFFF"
    readonly property color textLink: "#60A5FA"

    // ---- colors: accent ----
    readonly property color accent: "#3B82F6"
    readonly property color accentHover: "#60A5FA"
    readonly property color accentPressed: "#2563EB"
    readonly property color accentMuted: "#172554"

    // ---- colors: states ----
    // Told apart by "does it reach past me / is it stuck" -- see デザイン規約 §状態.
    readonly property color success: "#16A34A"
    readonly property color warning: "#D97706"
    readonly property color danger: "#EF4444"

    // ---- colors: dimmed (state, hue kept) ----
    // Two steps down each source's Tailwind ramp. Frames, icons and marks only, and one to a place
    // (デザイン規約 §暗く落とした段).
    readonly property color accentDim: "#1D4ED8"
    readonly property color refTagDim: "#A21CAF"
    readonly property color successDim: "#166534"
    readonly property color warningDim: "#92400E"
    readonly property color dangerDim: "#B91C1C"

    // ---- typography ----
    readonly property int fontSm: 12
    readonly property int fontSmLine: 16
    readonly property int fontMd: 14
    readonly property int fontMdLine: 20
    // Off the ramp on purpose: the size editors set source at (IntelliJ's default, JetBrains Mono 13).
    readonly property int fontCode: 13
    // Off the ramp, for the graph row's ref chip only (デザイン規約 §タイポグラフィ): `fontSm` reads as meta, and
    // `fontMd` stands level with the subject.
    readonly property int fontChip: 13
    readonly property int fontChipLine: 16
    readonly property int fontLg: 16
    readonly property int fontLgLine: 24
    readonly property int fontXl: 20
    readonly property int fontXlLine: 28
    // The one loosening in the app (デザイン規約 §タイポグラフィ). Sub-pixel on purpose: past about this the letters read
    // as a different face rather than the same word set wider.
    readonly property real tracking: 0.5
    // The one weight past Normal (デザイン規約 §タイポグラフィ). Bold on Ubuntu: Noto Sans CJK JP's Medium comes only
    // with fonts-noto-cjk-extra, so DemiBold draws Medium on one machine and Bold on the next — and Yu Gothic UI's
    // Semibold carries its Bold's outlines, so Bold is the weight Windows shows.
    readonly property int fontWeightStrong: Qt.platform.os === "linux" ? Font.Bold : Font.DemiBold
    // Families each OS ships that also carry Japanese (デザイン規約 §タイポグラフィ).
    readonly property var fontFamilyUi: ["Yu Gothic UI", "Hiragino Sans", "Noto Sans CJK JP", "Noto Sans JP",
                                         "Noto Sans", "DejaVu Sans"]
    readonly property var fontFamilyMono: ["Cascadia Mono", "Consolas", "Menlo", "Noto Sans Mono CJK JP",
                                           "Noto Sans Mono", "DejaVu Sans Mono"]

    // The QML font value type has no `families` list, so the chains above are resolved once against the installed
    // fonts. Empty string → Qt's default font.
    readonly property string uiFamily: _pickFamily(fontFamilyUi)
    readonly property string monoFamily: _pickFamily(fontFamilyMono)
    function _pickFamily(preferred) {
        const available = Qt.fontFamilies()
        for (let i = 0; i < preferred.length; i++) {
            if (available.indexOf(preferred[i]) !== -1)
                return preferred[i]
        }
        return ""
    }

    // ---- spacing (4px grid) ----
    readonly property int spaceXs: 4
    readonly property int spaceSm: 8
    readonly property int spaceMd: 12
    readonly property int spaceLg: 16
    readonly property int spaceXl: 24
    readonly property int spaceXxl: 32

    // ---- strokes, corners, icon squares ----
    readonly property int borderWidth: 1
    readonly property int radiusSm: 2
    readonly property int radiusMd: 4
    readonly property int iconXs: 10
    readonly property int iconSm: 12
    readonly property int iconMd: 16
    readonly property int iconLg: 20
    readonly property int iconXl: 24
    // Not a step: what a mark on another's shoulder never shrinks under. Its size is worked out from the mark it sits
    // on — half of it — so the sixth number is this floor alone (`ShoulderBadge`; デザイン規約 §寸法 の右肩の印).
    readonly property int iconBadgeMin: 8

    // ================================================================
    // 2. ベーシック(通常モードの窓 — その中に立つダイアログ・メニューも読む)
    // ================================================================

    // ---- colors: ref kinds ----
    // A separate axis from the state colours: a kind, not a state (デザイン規約 §ref の種別).
    readonly property color refTag: "#D946EF"

    // ---- colors: which worktree the reader is standing in ----
    // The WORKTREES green a step up the ramp: Tailwind green 500 over `success`'s 600 (デザイン規約 §ref の種別).
    // CIEDE2000 9.6 from `success`, no narrower than 9.5 under any CVD type (Machado 2009 severity 1.0): over the 8.8
    // floor (§暗く落とした段).
    readonly property color textHereTree: "#22C55E"
    // The tab in front's ground while it stands in a linked worktree: green 950's hue and saturation at 5.5% lightness
    // (the ramp's darkest step is 10%), since at matched luminance the green reads louder than `bgSelected`
    // (デザイン規約 §ref の種別).
    readonly property color bgHereTree: "#03190C"

    // ---- colors: git / diff ----
    readonly property color diffAddedFg: "#4ADE80"
    readonly property color diffAddedBg: "#14301D"
    readonly property color diffAddedEmphBg: "#144927"
    readonly property color diffRemovedFg: "#F87171"
    readonly property color diffRemovedBg: "#351515"
    readonly property color diffRemovedEmphBg: "#5B1E1F"
    readonly property color diffHunkHeaderFg: "#94A3B8"
    readonly property color diffHunkHeaderBg: "#0F172A"
    readonly property color statusStaged: "#16A34A"
    readonly property color statusUnstaged: "#D97706"
    readonly property color statusUntracked: "#64748B"
    readonly property color statusConflict: "#EF4444"

    // ---- colors: commit graph lanes (cycled modulo length) ----
    // Okabe-Ito color-universal-design palette (black swapped for a light gray that survives the dark background).
    readonly property var graphLane: [
        "#56B4E9", "#E69F00", "#009E73", "#CC79A7",
        "#F0E442", "#D55E00", "#0072B2", "#DDDDDD"
    ]

    // ---- the window's own structure ----
    // A constant, not from font metrics: rows of different families would stand at different heights in one list.
    readonly property int rowHeight: 24
    // `nodeIcon` with `spaceXs` over and under it.
    readonly property int graphRowHeight: 28
    readonly property int headerHeight: 32
    // The collapsed rail's width and the band's end cells' — one number for both edges of the window.
    readonly property int railWidth: 36
    // Also the collapsed rail's cell height (デザイン規約 §左メニューを畳む).
    readonly property int toolbarHeight: 40
    // 操作パネル: the branch on two lines (name, then upstream) — two `fontMd` lines (36) with `spaceSm` over and under.
    readonly property int opsBarHeight: 52
    readonly property int controlHeight: 28
    // Only for buttons that draw a frame: under a frameless one it puts the wash wider than the word.
    readonly property int buttonMinWidth: 80
    readonly property int splitterWidth: 4
    // The groove, not the bar's width (the style's box is 10): anything laid over a bar takes its width from the bar
    // (`ScrollBar.vertical.width`).
    readonly property int scrollBarGutter: 9
    // The pinned bar (`PaneScrollBar`): its thickness, and the groove a row leaves for it.
    readonly property int navBarReach: 5
    readonly property int navBarGutter: 8
    // `fontMdLine` × 5 + `spaceSm`, a resting height and a ceiling (デザイン規約 §寸法): git puts no ceiling on a
    // message, and a box that grows with it pushes the file list out of the pane.
    readonly property int messageMaxHeight: 108

    // ================================================================
    // 3. 個別(特別な画面 1 枚ごと — 名指しした画面だけが読む)
    // ================================================================

    // 文を読ませる面: the widest a run of prose is set at (デザイン規約 §レイアウト初期値).
    readonly property int textWidth: 640
    // 起動ゲート (デザイン規約 §レイアウト初期値): not `textWidth` — its longest line is a mono path, scanned rather than
    // read. Two thirds of the initial window.
    readonly property int gateWidth: 960
    // 設定画面: the category rail (デザイン規約 §レイアウト初期値 — not the sidebar's 260).
    readonly property int settingsRailWidth: 160
}
