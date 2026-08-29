import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Drawn from tokens rather than left to Fusion, for the reason `AppCombo` and `AppMenu` are: Fusion builds the
// indicator out of the palette's own two darkest roles — the ground is `palette.base` (`bgBase`, the same colour as
// the pane behind it) and the frame is that ground darkened again, which on this theme lands under `#010409`. Both
// disappear, so an unchecked box is a word with nothing in front of it and a checked one is a tick floating in the
// air.
//
// The frame is the one buttons and input fields already wear (`borderDefault`, デザイン規約 §色) at the field's own corner,
// so a box reads as a thing to press rather than as a mark; checked, it fills with `accent` and carries the family's
// own `check`. Disabled follows the palette table of §無効 without going through the palette: `highlight` → `accentMuted`
// for the face and `highlightedText` → `textMuted` for the tick.
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
