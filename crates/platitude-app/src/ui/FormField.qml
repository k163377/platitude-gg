import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The same frame at the height buttons and combo boxes use: for form
// input rather than the toolbar's slim filters.
TextField {
    id: form
    implicitHeight: Theme.controlHeight
    font.pixelSize: Theme.fontMd
    leftPadding: Theme.spaceSm
    rightPadding: Theme.spaceSm
    topPadding: 0
    bottomPadding: 0
    background: Rectangle {
        color: Theme.bgBase
        radius: Theme.radiusSm
        border.color: form.activeFocus ? Theme.borderFocus : Theme.borderDefault
        border.width: Theme.borderWidth
    }
}
