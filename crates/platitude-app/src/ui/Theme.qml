// Design tokens — the values themselves, and beside each one the reason it is that value.
//
// This file is where a value lives. internal-docs/デザイン規約.md says which token a place uses and why that one rather
// than its neighbour, and its 値 cells are generated from here (`cargo xtask docs --sync`, checked by `cargo xtask
// docs`), so a value is changed here and quoted there — written down once. What the document still decides is
// everything a number cannot say, and adding or changing a token needs a person either way (CLAUDE.md 絶対制約).
//
// Take every number a component uses from here: add it with its reason. The set is complete on purpose: a token no
// code uses yet still belongs here — keep it, and let its presence say only that it exists.
//
// Three tiers, in this order (規約 §トークンの三層):
//   1. 基礎     — ink and rhythm: colour, type, the spacing step, radii, icon squares. Screen-agnostic; anyone reads them.
//   2. ベーシック — the ordinary window's structure: the chrome, the panes, and the colours only they raise. Anyone
//                  standing inside that window reads them, dialogs and menus included.
//   3. 個別     — one named screen's own number. **Only that screen reads it**, and a second screen that wants the
//                  same measurement gets its own token.
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
    // here, and the idle step is a further 3/10 of that. The graph's rows run under this
    // bar, so it carries no more ink than leaves the words beneath it readable.
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
    // Two steps down the Tailwind ramp each source came from. For frames, icons and marks only -- dimming *text* is the
    // disabled signal (§無効), and a second meaning for it cannot be read apart. Each is clear of its own source; they
    // are NOT all clear of each other, so only one stands where they have to be told apart (デザイン規約 §暗く落とした段).
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
    // Off the ramp on purpose: the size an editor puts source at, which is not a step in a UI scale. IntelliJ's default
    // (JetBrains Mono 13) is what this matches.
    readonly property int fontCode: 13
    // Also off the ramp, and for one place: the name in a graph row's ref chip (デザイン規約 §タイポグラフィ). A branch name is
    // part of the row, so `fontSm` would read as meta; it is beside what the row is read for, so at
    // `fontMd` it stood level with the subject and read as loud as the message.
    readonly property int fontChip: 13
    readonly property int fontChipLine: 16
    readonly property int fontLg: 16
    readonly property int fontLgLine: 24
    readonly property int fontXl: 20
    readonly property int fontXlLine: 28
    // The one loosening in the app, and the most a word is ever set at (デザイン規約 §タイポグラフィ). Sub-pixel on purpose:
    // this is air a short word is opened with — past about this the letters stop reading as the same
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

    // ---- strokes, corners, icon squares ----
    readonly property int borderWidth: 1
    readonly property int radiusSm: 2
    readonly property int radiusMd: 4
    readonly property int iconXs: 10
    readonly property int iconSm: 12
    readonly property int iconMd: 16
    readonly property int iconLg: 20
    readonly property int iconXl: 24

    // ================================================================
    // 2. ベーシック(通常モードの窓 — その中に立つダイアログ・メニューも読む)
    // ================================================================

    // ---- colors: ref kinds ----
    // A separate axis from the state colours above: a tag is a kind. Local = accent, remote =
    // textSecondary, detached HEAD = warning (that one really is a state).
    readonly property color refTag: "#D946EF"

    // ---- colors: which working copy the reader is standing in ----
    // The name on the WORKTREES row of the copy this window has open, where that copy is a linked worktree. **The
    // section's own green, a step up the ramp**: the green is what the WORKTREES section *is* (its icon, the tree
    // mark, the chip's frame), so the hue says which kind of place this row is and the brightness says the reader is
    // standing on this one — hue for the kind, lightness for where, the split the tag chips are already read by
    // (デザイン規約 §ref の種別). The repository's own copy answers the same question in `textLink`, because that row
    // is named by its branch and a branch says it in `textLink` wherever its name is drawn.
    // Tailwind green 500, one step over `success`'s green 600: 1.53x its luminance, and CIEDE2000 9.6 from it — no
    // narrower than 9.5 under any of the three CVD types (Machado 2009 severity 1.0), over the 8.8 floor colours
    // told apart in one place are held to (規約 §暗く落とした段). 8.9:1 over `bgSurface`, 6.5:1 over `bgSelected`,
    // and no closer than 11.2 to any other name colour in the panel (`textLink`, under tritanopia).
    // **The two it does stand close to are never in a list with it**: `danger` at 7.7 (deuteranopia) is a conflict's
    // mark on a file row, and `diffAddedFg` at 6.8 is an added file's — neither is ever drawn in the WORKTREES
    // section, and a colour read apart by which column it is in is how `success` itself is already read
    // (規約 §ref の種別).
    readonly property color textHereTree: "#22C55E"

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
    // A constant: rows of different families would otherwise stand at
    // different heights in the same list.
    readonly property int rowHeight: 24
    // `nodeIcon` with `spaceXs` over and under it.
    readonly property int graphRowHeight: 28
    readonly property int headerHeight: 32
    // The collapsed left rail's width, and the width of the band's end cells with it, so this one number sets both
    // edges of the window. Being wider than a pane's inset (`spaceXs`) is right: this is the window's outer
    // rim.
    readonly property int railWidth: 36
    // Also the height of a collapsed rail cell: ☰ stands in a cell of the band's full height and the five cells under
    // it must answer to the same depth, or the height that can be pressed changes partway down one column.
    readonly property int toolbarHeight: 40
    readonly property int controlHeight: 28
    // A floor for buttons that draw a frame, and only those — a button without one is a word rather than a box, and a
    // floor under it puts the hover wash and the hold fill wider than the word.
    readonly property int buttonMinWidth: 80
    readonly property int splitterWidth: 4
    // The groove left on an edge a floating bar rides — the style's box is 10 and its
    // thumb 6, so the groove is narrower than the box. Anything laid over a bar takes its width from the bar itself
    // (`ScrollBar.vertical.width`).
    readonly property int scrollBarGutter: 9
    // The left panel's own bar, which is a slab held against the pane's edge:
    // how thick it is, and the room a row of that panel leaves for it.
    readonly property int navBarReach: 5
    readonly property int navBarGutter: 8
    // `fontMdLine` five times plus `spaceSm`. **All four message boxes read this one value** — git puts no ceiling on
    // a message (1MB is accepted), so a box that grows with its content pushes the file list out of the pane and the
    // author row under the window's footer. It is a resting height: the grip at the bottom
    // right raises it, up to where that same accident begins.
    readonly property int messageMaxHeight: 108

    // ================================================================
    // 3. 個別(特別な画面 1 枚ごと — 名指しした画面だけが読む)
    // ================================================================

    // 文を読ませる面: the widest a run of prose is set at (デザイン規約 §レイアウト初期値, 640) — the card a dialog
    // stands in, the screen a repository would not open on, and the sentences under a settings chapter. Every seat
    // is the same amount to read, so those three share one width; the boxes beside them still take
    // whatever the column gives (§設定の画面).
    readonly property int textWidth: 640
    // 起動ゲート: its own column (デザイン規約 §レイアウト初期値). `textWidth` is the width a
    // run of prose is read at, and this screen has none: its longest line is a mono path, which is scanned,
    // and it is the whole of what there is to take away because no repository is open yet and the command
    // log does not exist. Two thirds of the initial window, so the air either side is still the larger part of it.
    readonly property int gateWidth: 960
    // 設定画面: the category rail. The sidebar's 260 is for a column of names nobody chose
    // (branches, remotes, tags), and this one holds a handful of words this app writes itself (デザイン規約 §設定の画面).
    readonly property int settingsRailWidth: 160
}
