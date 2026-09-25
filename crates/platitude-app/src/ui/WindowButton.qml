import QtQuick
import platitude.ui

// One of the window's own buttons, drawn here: the platform's cannot be styled and its maximize mark never becomes
// a restore mark. Close goes red under the pointer (デザイン規約 §ウィンドウの縁).
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
