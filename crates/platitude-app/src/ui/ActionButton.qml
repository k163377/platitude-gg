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
    /// git is on the network for this button: the icon turns in its place.
    property bool busy: false
    /// Hold the turn still (automation — a spinning icon photographs
    /// differently every time).
    property bool still: false
    readonly property color fg: enabled ? tone : Theme.textMuted
    contentItem: RowLayout {
        spacing: Theme.spaceXs
        Item {
            implicitWidth: Theme.iconMd
            implicitHeight: Theme.iconMd
            Layout.alignment: Qt.AlignVCenter
            NavIcon {
                anchors.fill: parent
                kind: actionBtn.kind
                tint: actionBtn.fg
                visible: !actionBtn.busy
            }
            // Its own item rather than a rotation on the one above: an
            // animator leaves the angle where it stopped, and the icon
            // that returns must not come back tilted.
            NavIcon {
                anchors.fill: parent
                kind: "spinner"
                tint: actionBtn.fg
                visible: actionBtn.busy
                // On the render thread, so it keeps turning while the GUI
                // thread drains models.
                RotationAnimator on rotation {
                    running: actionBtn.busy && !actionBtn.still
                    loops: Animation.Infinite
                    from: 0
                    to: 360
                    duration: Metrics.spinMs
                }
            }
        }
        Label {
            text: actionBtn.text
            color: actionBtn.fg
            font.pixelSize: Theme.fontMd
            Layout.alignment: Qt.AlignVCenter
        }
    }
}
