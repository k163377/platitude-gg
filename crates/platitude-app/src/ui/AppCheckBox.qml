import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The one check box (デザイン規約 §QML 実装ルール). Drawn from tokens: Fusion's indicator is `palette.base` — the
// pane behind it — framed darker still, so on this theme the box vanishes (unchecked reads as a blank before the
// words, checked as a mark floating in the air). Unchecked wears an input field's own frame (`bgBase` face,
// `borderDefault` 1px, `borderFocus` for focus) so it reads as something to press; checked is an `accent` face with
// the `check` mark in `textOnAccent`; under the pointer the frame goes `borderStrong`. Disabled moves §無効's palette
// table onto the face (`highlight` → `accentMuted` face, `highlightedText` → `textMuted` mark).
// Headless: `amend-author` / `remote-url origin:marked` / `commands`.
CheckBox {
    id: appCheck
    font.pixelSize: Theme.fontMd
    // Users keep this default; only the band's `Background reads` (`CommandsPane`) drops to `iconLg`.
    implicitHeight: Theme.controlHeight
    // The box's seat: whatever holds the box keeps its step on the left, and the same step comes before the words —
    // the style's own 6 on the left would stack on it (デザイン規約 §アイコンの席「④ の席」— the box's seat is
    // `spaceXs` | mark | `spaceXs`).
    leftPadding: 0
    spacing: Theme.spaceXs
    indicator: Rectangle {
        implicitWidth: Theme.iconMd
        implicitHeight: Theme.iconMd
        x: appCheck.leftPadding
        y: appCheck.topPadding + (appCheck.availableHeight - height) / 2
        radius: Theme.radiusSm
        color: !appCheck.checked ? Theme.bgBase
             : appCheck.enabled ? Theme.accent : Theme.accentMuted
        border.width: Theme.borderWidth
        border.color: appCheck.visualFocus ? Theme.borderFocus
                    : appCheck.checked ? color
                    : appCheck.hovered ? Theme.borderStrong : Theme.borderDefault
        NavIcon {
            anchors.fill: parent
            kind: "check"
            visible: appCheck.checked
            tint: appCheck.enabled ? Theme.textOnAccent : Theme.textMuted
        }
    }
}
