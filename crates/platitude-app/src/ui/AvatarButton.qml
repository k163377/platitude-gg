import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The one place a picture is reached from. A pointer resting on the face raises a badge saying so; pressing it opens
// the settings card with this author already named (デザイン規約 §アバターを与える).
//
// The face carries what is said about the person wearing it, a corner apiece: the pen that assigns a picture at the
// bottom right, and what git makes of their signature at the top right (§署名の表示). Both panes wear the same pair,
// so a face means the same thing wherever it stands — the commit editor's is simply smaller, and hands in a smaller
// ink for the two badges with it.
Item {
    id: avatarBox

    /// The identicon code, and the assigned image ("" for none).
    property string face: ""
    property string faceUrl: ""
    /// Whom the badge would be about. "" disables the whole affordance: nothing to assign a picture to on a row with no
    /// author — the working tree's own row, and a commit not read yet.
    property string email: ""
    /// Stands in for the pointer where headless cannot put one, so the badge can be photographed
    /// (PG_AUTO_ACT=avatar-hover).
    property bool pointedAt: false
    /// What rides the other corner (`SignatureMark`): git's verdict on this commit's signature in the details pane,
    /// and whether the next commit will be signed in the editor. Empty `signatureKind` puts nothing there.
    property string signatureKind: ""
    property string signatureCode: ""
    property string signatureSigner: ""
    property bool signaturePointedAt: false
    /// The sentence the mark's own tooltip carries. Left alone it reads git's verdict code; the editor's badge is a
    /// setting rather than a verdict and hands its own in.
    property alias signatureTip: sigMark.tip
    /// The ink both corner badges are drawn at. `iconSm` on the details pane's 40px face; a face smaller than that
    /// hands in `iconXs`, since the badge's share of what it covers is the only lever there is (§アバターを与える).
    property int badgeInk: Theme.iconSm
    /// How far the signature mark's **ink** is to stand past the face's square, up and to the right. The round face
    /// leaves that corner empty, so the mark lands on the pane's own ground rather than on anybody's picture — and how
    /// far it may go is set by what stands next to it, which only the pane knows (§署名の表示). Two numbers, because
    /// the air above the face and the air beside it are rarely the same.
    ///
    /// Ink, not box: the mark's own square holds air past its tick, and this adds that back (`SignatureMark.inkAir*`),
    /// so what the pane asks for is the distance it can actually see.
    property real badgeTopOut: 0
    property real badgeRightOut: 0

    readonly property bool editable: avatarBox.email !== ""
    readonly property bool showBadge: avatarBox.editable && (avatarHover.hovered || avatarBox.pointedAt)
    /// What the mark made of it, for whatever stands beside the face and has to agree with it — the one word a broken
    /// signature spends is the only thing that does (規約 §署名の表示).
    readonly property bool signatureBroken: sigMark.broken
    readonly property color signatureTone: sigMark.tone
    readonly property bool signatureTipShown: sigMark.tipShown

    signal clicked()

    width: Metrics.detailsAvatar
    height: Metrics.detailsAvatar
    IdentIcon {
        anchors.fill: parent
        code: avatarBox.face
        imageUrl: avatarBox.faceUrl
    }
    // Pushed as far into the lower-right as the icon's own square allows — flush with its right and bottom edges, so
    // the least of the face is covered and the layout beside it never moves (デザイン規約 §アバターを与える).
    Rectangle {
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        width: avatarBox.badgeInk + 2 * Theme.borderWidth
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
            width: avatarBox.badgeInk
            height: avatarBox.badgeInk
        }
    }
    // The other corner, pushed out of it by however much the pane says. Always out when there is a signature to speak
    // of, since it is a state of the commit rather than an affordance of the pointer.
    SignatureMark {
        id: sigMark
        anchors.right: parent.right
        anchors.rightMargin: -(avatarBox.badgeRightOut + sigMark.inkAirRight)
        anchors.top: parent.top
        anchors.topMargin: -(avatarBox.badgeTopOut + sigMark.inkAirTop)
        kind: avatarBox.signatureKind
        code: avatarBox.signatureCode
        signer: avatarBox.signatureSigner
        pointedAt: avatarBox.signaturePointedAt
        ink: avatarBox.badgeInk
    }
    // Clicks only. Hover is a handler's, because handlers are passive: a hoverEnabled MouseArea over the whole face
    // takes the hover from everything under it, and the mark in the corner would never say why it is the colour it is
    // (rules-refs/app-ui.md 「行を覆う MouseArea」).
    MouseArea {
        id: avatarArea
        anchors.fill: parent
        enabled: avatarBox.editable
        onClicked: avatarBox.clicked()
    }
    HoverHandler { id: avatarHover }
    // One pointer opens one thing (規約 §hover のツールチップ): on the mark's own corner the verdict is what was
    // reached for, so the face's sentence stands down while it is there.
    ToolTip.visible: avatarBox.showBadge && !sigMark.pointed
    ToolTip.delay: Metrics.tipDelayMs
    // The one word the whole feature goes by (デザイン規約 §アバターを 与える). The article is what splits the two states, not a
    // second noun: the one being changed is the face under the pointer, the one being chosen does not exist yet (§長さ).
    ToolTip.text: avatarBox.faceUrl !== "" ? qsTr("Change the avatar for %1").arg(avatarBox.email)
                  : qsTr("Choose avatar for %1").arg(avatarBox.email)
}
