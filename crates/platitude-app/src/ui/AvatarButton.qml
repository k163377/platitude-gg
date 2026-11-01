import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A face that opens the avatar settings for its author (デザイン規約 §アバターを与える): a pen badge bottom right under
// the pointer, and top right the signature mark where the caller has no name to hang it off (§署名の表示) — the commit
// button's and the message-save button's phrases name a branch, not a person. Where a name stands, the face uses one
// corner only: the signature is on the name's shoulder (`CommitAuthorRow`).
//
// The pen opens the settings screen on `Application` with this person named: the box holds `Name <address>` and focus
// is on `Choose avatar…` (`SettingsAppPane.focusPrefill`), so one more press opens the picker — a stop in between would
// only be a stop. Only a face with an author has it (not the worktree's row, not a commit not read yet). The badge is
// flush in the face square's lower-right corner — the farthest out it goes; what it covers is set by its size alone.
Item {
    id: avatarBox

    /// The identicon code, and the assigned image ("" for none).
    property string face: ""
    property string faceUrl: ""
    /// Whom the badge would be about; "" (no author — the worktree's row, a commit not read yet) disables it.
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
    /// The ink both corner badges are drawn at; a face smaller than the details pane's hands in `iconXs`, so the
    /// badge keeps its share of the face.
    property int badgeInk: Theme.iconSm
    /// How far the signature mark's ink stands past the face's square, up and to the right — set by the caller, which
    /// knows what stands next to it: out past the corner the round face leaves empty, short of the neighbour's frame.
    /// The mark's own air is added back (`SignatureMark.inkAir*`): the
    /// mark's box holds the 16-grid's air, so 1px given to the box would leave the ink 3px off.
    property real badgeTopOut: 0
    property real badgeRightOut: 0

    readonly property bool editable: avatarBox.email !== ""
    readonly property bool showBadge: avatarBox.editable && (avatarHover.hovered || avatarBox.pointedAt)
    /// What the mark made of it, for a caller drawing the broken-signature word beside the face (`SignatureMark`).
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
    // Flush in the lower-right of the face's square: an `iconSm + 2 × borderWidth` disc round an `iconSm` pen.
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
    // (rules-refs/app-ui.md「行を覆う `MouseArea` はその下の hover を全部奪う」).
    MouseArea {
        id: avatarArea
        anchors.fill: parent
        enabled: avatarBox.editable
        onClicked: avatarBox.clicked()
    }
    HoverHandler { id: avatarHover }
    readonly property TipAsk badgeTip: TipAsk {
        target: avatarBox
        // One pointer opens one thing (規約 §hover のツールチップ): the face's tip stands down over the mark.
        asked: avatarBox.showBadge && !sigMark.pointed
        // The articles differ on purpose (デザイン規約 §アバターを与える). No road to an empty `%1`: with no address
        // there is no badge (`editable`). That the window's box says the address is `avatar-tip`'s `named=` — the
        // badge's picture frames an empty sentence the same.
        words: avatarBox.faceUrl !== "" ? qsTr("Change the avatar for %1").arg(avatarBox.email)
               : qsTr("Choose avatar for %1").arg(avatarBox.email)
    }
}
