// Design tokens — a verbatim mirror of internal-docs/デザイン規約.md. Edit the document first, then reflect changes here.
// Never introduce values that are not in the document's tables. The mirror is complete on purpose: a token no code uses
// yet still belongs here — do not prune it, and do not read its presence as "the default to pick".
pragma Singleton

import QtQuick

QtObject {
    // ---- colors: backgrounds ----
    readonly property color bgBase: "#020617"
    readonly property color bgSurface: "#020617"
    readonly property color bgElevated: "#0F172A"
    readonly property color bgSelected: "#172554"
    readonly property color bgHover: "#14FFFFFF"
    readonly property color bgPressed: "#1FFFFFFF"

    // ---- colors: borders ----
    readonly property color borderSubtle: "#1E293B"
    readonly property color borderDefault: "#334155"
    readonly property color borderStrong: "#475569"
    readonly property color borderFocus: "#3B82F6"

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
    // Told apart by "does it reach past me / is it stuck", not by whether it can be undone -- see デザイン規約 §状態.
    readonly property color success: "#16A34A"
    readonly property color warning: "#D97706"
    readonly property color danger: "#EF4444"

    // ---- colors: ref kinds ----
    // A separate axis from the three above: a tag is a kind, not a severity. Local = accent, remote = textSecondary,
    // detached HEAD = warning (that one really is a state).
    readonly property color refTag: "#D946EF"

    // ---- colors: dimmed (state, hue kept) ----
    // Two steps down the Tailwind ramp each source came from. For frames, icons and marks only -- dimming *text* is the
    // disabled signal (§無効), and a second meaning for it cannot be read apart. Each is clear of its own source; they
    // are NOT all clear of each other, so never put two of them where they have to be told apart (デザイン規約 §暗く落とした段).
    readonly property color accentDim: "#1D4ED8"
    readonly property color refTagDim: "#A21CAF"
    readonly property color successDim: "#166534"
    readonly property color warningDim: "#92400E"
    readonly property color dangerDim: "#B91C1C"

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

    // ---- typography ----
    readonly property int fontSm: 12
    readonly property int fontSmLine: 16
    readonly property int fontMd: 14
    readonly property int fontMdLine: 20
    // Off the ramp on purpose: the size an editor puts source at, which is not a step in a UI scale. IntelliJ's default
    // (JetBrains Mono 13) is what this matches.
    readonly property int fontCode: 13
    // Also off the ramp, and for one place: the name in a graph row's ref chip (デザイン規約 §タイポグラフィ). A branch name is
    // not meta about the row, so it does not go down to `fontSm`; it is not what the row is read for either, so at
    // `fontMd` it stood level with the subject and read as loud as the message (2026-08-21 ユーザー判断).
    readonly property int fontChip: 13
    readonly property int fontChipLine: 16
    readonly property int fontLg: 16
    readonly property int fontLgLine: 24
    readonly property int fontXl: 20
    readonly property int fontXlLine: 28
    // The one loosening in the app, and the most a word is ever set at (デザイン規約 §タイポグラフィ). Sub-pixel on purpose:
    // this is air a short word is opened with, not a style — past about this the letters stop reading as the same
    // word set wider and start reading as a different face.
    readonly property real tracking: 0.5
    // Families each OS ships that also carry Japanese: a glyph missing from the named family falls back by OS locale,
    // which is where kanji turn Chinese (デザイン規約 §タイポグラフィ, 2026-08-08).
    readonly property var fontFamilyUi: ["Yu Gothic UI", "Hiragino Sans", "Noto Sans CJK JP", "Noto Sans JP", "Noto Sans", "DejaVu Sans"]
    readonly property var fontFamilyMono: ["Cascadia Mono", "Consolas", "Menlo", "Noto Sans Mono CJK JP", "Noto Sans Mono", "DejaVu Sans Mono"]

    // The QML font value type has no `families` list, so the fallback chains above are resolved once against the
    // installed fonts here. Empty string → Qt's default font.
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

    // ---- dimensions ----
    readonly property int rowHeight: 24
    readonly property int graphRowHeight: 28
    readonly property int headerHeight: 32
    readonly property int railWidth: 36
    readonly property int toolbarHeight: 40
    readonly property int controlHeight: 28
    readonly property int buttonMinWidth: 80
    readonly property int iconXs: 10
    readonly property int iconSm: 12
    readonly property int iconMd: 16
    readonly property int iconLg: 20
    readonly property int iconXl: 24
    readonly property int borderWidth: 1
    readonly property int radiusSm: 2
    readonly property int radiusMd: 4
    readonly property int splitterWidth: 4
    readonly property int scrollBarGutter: 9
    readonly property int messageMaxHeight: 108
}
