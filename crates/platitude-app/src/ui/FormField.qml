import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The same frame at the height buttons and combo boxes use: for form
// input rather than the toolbar's slim filters.
TextField {
    id: form
    implicitHeight: Theme.controlHeight
    font.pixelSize: Theme.fontMd
    // The frame-to-word inset every box in the application shares (`SlimField`). The height is what tells a form field
    // from a slim one; the words stand the same distance in either.
    leftPadding: Theme.spaceXs
    rightPadding: Theme.spaceXs
    topPadding: 0
    bottomPadding: 0
    background: Rectangle {
        color: Theme.bgBase
        radius: Theme.radiusSm
        border.color: form.activeFocus ? Theme.borderFocus : Theme.borderDefault
        border.width: Theme.borderWidth
    }
}
