import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A face that opens the avatar settings for its author (デザイン規約 §アバターを与える): a pen badge bottom right under
// the pointer, and top right the signature mark where the caller has no name to hang it off (§署名の表示).
Item {
    id: avatarBox

    /// The identicon code, and the assigned image ("" for none).
    property string face: ""
    property string faceUrl: ""
    /// Whom the badge would be about; "" (no author — the working tree's row, a commit not read yet) disables it.
    property string email: ""
    /// Stands in for the pointer where headless cannot put one, so the badge can be photographed
    /// (PGG_AUTO_ACT=avatar-hover).
    property bool pointedAt: false
    /// The top-right mark (`SignatureMark`); an empty `signatureKind` puts nothing there.
    property string signatureKind: ""
    property string signatureCode: ""
    property string signatureSigner: ""
    property bool signaturePointedAt: false
    /// The sentence the mark's own tooltip carries. Left alone it reads git's verdict code; the editor's badge is a
    /// setting and hands its own in.
    property alias signatureTip: sigMark.tip
    /// The ink both corner badges are drawn at; a face smaller than the details pane's hands in `iconXs`
    /// (§アバターを与える).
    property int badgeInk: Theme.iconSm
    /// How far the signature mark's ink stands past the face's square, up and to the right — set by the caller, which
    /// knows what stands next to it (§署名の表示). The mark's own air is added back (`SignatureMark.inkAir*`).
    property real badgeTopOut: 0
    property real badgeRightOut: 0

    readonly property bool editable: avatarBox.email !== ""
    readonly property bool showBadge: avatarBox.editable && (avatarHover.hovered || avatarBox.pointedAt)
    /// What the mark made of it, for a caller drawing the broken-signature word beside the face (規約 §署名の表示).
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
    // Flush in the lower-right of the face's square (デザイン規約 §アバターを与える).
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
    // Shown without hover: a signature is a state of the commit.
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
        // Anchored, so the implicit size has to be taken by hand.
        width: sigMark.implicitWidth
        height: sigMark.implicitHeight
    }
    // Clicks only; hover is the handler's — a hover MouseArea over the face would take it from the corner mark
    // (rules-refs/app-ui.md 「行を覆う MouseArea」).
    MouseArea {
        id: avatarArea
        anchors.fill: parent
        enabled: avatarBox.editable
        onClicked: avatarBox.clicked()
    }
    HoverHandler { id: avatarHover }
    // One pointer opens one thing (規約 §hover のツールチップ): the face's tip stands down over the mark.
    ToolTip.visible: avatarBox.showBadge && !sigMark.pointed
    ToolTip.delay: Metrics.tipDelayMs
    // The articles differ on purpose (デザイン規約 §アバターを与える).
    ToolTip.text: avatarBox.faceUrl !== "" ? qsTr("Change the avatar for %1").arg(avatarBox.email)
                  : qsTr("Choose avatar for %1").arg(avatarBox.email)
}
