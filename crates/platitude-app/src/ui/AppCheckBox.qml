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
    spacing: Theme.spaceSm
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
