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

    // ---- colors: scroll bar (the style's own bar — thumb ink, drawn over the content) ----
    // One ink, three amounts of it, chosen so that over `bgSurface` they land on palette steps counted up from the
    // ground: the idle bar reads `bgElevated`, the one being used `borderDefault`, the one being held `borderStrong`
    // (デザイン規約 §スクロールバー). Fusion draws the thumb at `opacity 0.75`, so what lands is 0.75 of the alpha
    // here, and the idle step is a further 3/10 of that rather than a fourth colour. The graph's rows run under this
    // bar rather than stopping short of it, so it carries no more ink than leaves the words beneath it readable.
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
    // `fontMd` it stood level with the subject and read as loud as the message.
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
    // which is where kanji turn Chinese (デザイン規約 §タイポグラフィ, observed).
    readonly property var fontFamilyUi: ["Yu Gothic UI", "Hiragino Sans", "Noto Sans CJK JP", "Noto Sans JP",
                                         "Noto Sans", "DejaVu Sans"]
    readonly property var fontFamilyMono: ["Cascadia Mono", "Consolas", "Menlo", "Noto Sans Mono CJK JP",
                                           "Noto Sans Mono", "DejaVu Sans Mono"]

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
    // The left panel's own bar, which is a slab held against the pane's edge rather than the style's floating pill:
    // how thick it is, and the room a row of that panel leaves for it.
    readonly property int navBarReach: 5
    readonly property int navBarGutter: 8
    readonly property int messageMaxHeight: 108
    // The widest a run of prose is set in this window (デザイン規約 §レイアウト初期値, 640): the card a dialog
    // stands in, the git gate's lines, the screen a repository would not open on, and the sentences under a settings
    // chapter. A different seat is not a different amount to read, so they share one width — the boxes beside them
    // still take whatever the column gives (§設定の画面).
    readonly property int textWidth: 640
    // The startup gate's own column (デザイン規約 §レイアウト初期値). **Not `textWidth`** — that number is the width a
    // run of prose is read at, and this screen has none: its longest line is a mono path, which is scanned rather
    // than read, and it is the whole of what there is to take away because no repository is open yet and the command
    // log does not exist. Two thirds of the initial window, so the air either side is still the larger part of it.
    readonly property int gateWidth: 960
    // The settings screen's category rail. **Not the sidebar's 260** — that width is for a column of names nobody
    // chose (branches, remotes, tags), and this one holds a handful of words this app writes itself
    // (デザイン規約 §設定の画面).
    readonly property int settingsRailWidth: 160
}
