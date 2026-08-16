import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The one place a picture is reached from. A pointer resting on the
// face raises a badge saying so; pressing it opens the settings card
// with this author already named (デザイン規約 §アバターを与える).
Item {
    id: avatarBox

    /// The identicon code, and the assigned image ("" for none).
    property string face: ""
    property string faceUrl: ""
    /// Whom the badge would be about. "" disables the whole affordance:
    /// nothing to assign a picture to on a row with no author — the
    /// working tree's own row, and a commit not read yet.
    property string email: ""
    /// Stands in for the pointer where headless cannot put one, so the
    /// badge can be photographed (PG_AUTO_ACT=avatar-hover).
    property bool pointedAt: false

    readonly property bool editable: avatarBox.email !== ""
    readonly property bool showBadge:
        avatarBox.editable
        && (avatarArea.containsMouse || avatarBox.pointedAt)

    signal clicked()

    width: Metrics.detailsAvatar
    height: Metrics.detailsAvatar
    IdentIcon {
        anchors.fill: parent
        code: avatarBox.face
        imageUrl: avatarBox.faceUrl
    }
    // Pushed as far into the lower-right as the icon's own square
    // allows — flush with its right and bottom edges, so the least of
    // the face is covered and the layout beside it never moves
    // (デザイン規約 §アバターを与える).
    Rectangle {
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        width: Theme.iconSm + 2 * Theme.borderWidth
        height: width
        radius: width / 2
        color: Theme.bgElevated
        border.color: Theme.borderStrong
        border.width: Theme.borderWidth
        visible: avatarBox.showBadge
        NavIcon {
            anchors.centerIn: parent
            kind: "pen"
            tint: Theme.textPrimary
            width: Theme.iconSm
            height: Theme.iconSm
        }
    }
    MouseArea {
        id: avatarArea
        anchors.fill: parent
        hoverEnabled: true
        enabled: avatarBox.editable
        onClicked: avatarBox.clicked()
    }
    ToolTip.visible: avatarBox.showBadge
    ToolTip.delay: Metrics.tipDelayMs
    // The one word the whole feature goes by (デザイン規約 §アバターを
    // 与える). The article is what splits the two states, not a second
    // noun: the one being changed is the face under the pointer, the
    // one being chosen does not exist yet (§長さ).
    ToolTip.text: avatarBox.faceUrl !== ""
                  ? qsTr("Change the avatar for %1").arg(avatarBox.email)
                  : qsTr("Choose avatar for %1").arg(avatarBox.email)
}
