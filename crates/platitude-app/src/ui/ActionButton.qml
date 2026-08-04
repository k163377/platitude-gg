import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Toolbar action named twice over: an icon to find it by shape, the
// word to be sure of it. Both halves dim together when disabled.
HoverToolButton {
    id: actionBtn
    property string kind: ""
    /// Colour of both halves while the button is live.
    property color tone: Theme.textPrimary
    readonly property color fg: enabled ? tone : Theme.textMuted
    contentItem: RowLayout {
        spacing: Theme.spaceXs
        NavIcon {
            kind: actionBtn.kind
            tint: actionBtn.fg
            Layout.alignment: Qt.AlignVCenter
        }
        Label {
            text: actionBtn.text
            color: actionBtn.fg
            font.pixelSize: Theme.fontMd
            Layout.alignment: Qt.AlignVCenter
        }
    }
}
