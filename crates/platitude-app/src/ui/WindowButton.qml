import QtQuick
import platitude.ui

// One of the window's own buttons, drawn here: the platform's cannot be given a height, a hover or a mark, and its
// maximize mark never becomes a restore mark. A `railWidth` cell the band's whole height, as the ☰ at the other end
// (`TopBar`'s header); the mark `iconMd`, a step under the ☰'s `iconLg` — a window button's mark is smaller than the
// app's by convention. Hover is every button's `bgHover`; close goes red, its mark flipped to `textOnAccent` — a close
// button that does not is read as a fault.
// That red (`danger`) stands outside the status colours (規約 §状態): it is the OS's convention for close, not a
// warning (user decision — ask before changing). It reads the real pointer's `containsMouse` alone, which headless
// cannot inject — no verb shows it.
Rectangle {
    id: winBtn
    property string kind: ""
    property bool danger: false
    signal triggered()

    // `implicitWidth`, not `width`: `TopBar`'s `bandAsked` shadow row mirrors each cell by what it asks for.
    implicitWidth: Theme.railWidth
    height: parent ? parent.height : Theme.toolbarHeight
    color: !winBtnMouse.containsMouse ? "transparent" : winBtn.danger ? Theme.danger : Theme.bgHover
    NavIcon {
        anchors.centerIn: parent
        width: Theme.iconMd
        height: Theme.iconMd
        kind: winBtn.kind
        tint: winBtnMouse.containsMouse && winBtn.danger ? Theme.textOnAccent : Theme.textPrimary
    }
    MouseArea {
        id: winBtnMouse
        anchors.fill: parent
        hoverEnabled: true
        onClicked: winBtn.triggered()
    }
}
