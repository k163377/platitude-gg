import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Drawn from tokens: Fusion's indicator is `palette.base` framed darker still, and vanishes on this theme
// (rules-refs/app-ui.md「チェックボックスは `AppCheckBox` の 1 か所」). The frame is the fields' `borderDefault`
// (デザイン規約 §色); disabled maps §無効's palette table (`highlight` → `accentMuted`, `highlightedText` → `textMuted`).
CheckBox {
    id: appCheck
    font.pixelSize: Theme.fontMd
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
