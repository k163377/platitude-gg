import QtQuick
import platitude.ui

// One of the window's own buttons, drawn here: the platform's cannot be styled and its
// maximize mark never becomes a restore mark (P3-確認事項 §ウィンドウ chrome).
//
// The close button is the one exception to the wash — red under the pointer is a convention old enough that departing
// from it would read as a bug.
Rectangle {
    id: winBtn
    property string kind: ""
    property bool danger: false
    signal triggered()

    // **Asked for, not set**: the band lays these out, and a second row that lays the band out with the state group's
    // words (`TopBar`'s `bandAsked`) mirrors each cell by what it asks for — a width set outright asks for nothing.
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
