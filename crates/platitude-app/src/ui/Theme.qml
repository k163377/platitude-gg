// Design tokens — a verbatim mirror of internal-docs/デザイン規約.md.
// Edit the document first, then reflect changes here. Never introduce
// values that are not in the document's tables.
pragma Singleton

import QtQuick

QtObject {
    // ---- colors: backgrounds ----
    readonly property color bgBase: "#020617"
    readonly property color bgSurface: "#0F172A"
    readonly property color bgElevated: "#1E293B"
    readonly property color bgSelected: "#1E3A8A"
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
    readonly property color accentMuted: "#1E3A8A"

    // ---- colors: states ----
    readonly property color success: "#22C55E"
    readonly property color warning: "#F59E0B"
    readonly property color danger: "#EF4444"

    // ---- colors: git / diff ----
    readonly property color diffAddedFg: "#4ADE80"
    readonly property color diffAddedBg: "#052E16"
    readonly property color diffRemovedFg: "#F87171"
    readonly property color diffRemovedBg: "#450A0A"
    readonly property color diffHunkHeaderFg: "#94A3B8"
    readonly property color diffHunkHeaderBg: "#1E293B"
    readonly property color statusStaged: "#22C55E"
    readonly property color statusUnstaged: "#F59E0B"
    readonly property color statusUntracked: "#64748B"
    readonly property color statusConflict: "#EF4444"

    // ---- colors: commit graph lanes (cycled modulo length) ----
    readonly property var graphLane: [
        "#60A5FA", "#4ADE80", "#FBBF24", "#F87171",
        "#C084FC", "#22D3EE", "#FB923C", "#F472B6"
    ]

    // ---- typography ----
    readonly property int fontXs: 11
    readonly property int fontXsLine: 16
    readonly property int fontSm: 12
    readonly property int fontSmLine: 16
    readonly property int fontMd: 14
    readonly property int fontMdLine: 20
    readonly property int fontLg: 16
    readonly property int fontLgLine: 24
    readonly property int fontXl: 20
    readonly property int fontXlLine: 28
    readonly property var fontFamilyUi: ["Segoe UI", "SF Pro Text", "Ubuntu", "Noto Sans", "DejaVu Sans"]
    readonly property var fontFamilyMono: ["Cascadia Mono", "Consolas", "SF Mono", "Menlo", "Noto Sans Mono", "DejaVu Sans Mono"]

    // The QML font value type has no `families` list, so the fallback
    // chains above are resolved once against the installed fonts here.
    // Empty string → Qt's default font.
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
    readonly property int headerHeight: 32
    readonly property int toolbarHeight: 40
    readonly property int controlHeight: 28
    readonly property int iconSm: 12
    readonly property int iconMd: 16
    readonly property int iconLg: 20
    readonly property int borderWidth: 1
    readonly property int radiusSm: 2
    readonly property int radiusMd: 4
    readonly property int splitterWidth: 4
    readonly property int scrollBarWidth: 12
}
