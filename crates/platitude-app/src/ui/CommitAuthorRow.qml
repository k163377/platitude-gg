import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The details pane's author row: avatar + name/date on the left, own hash over parent hash on the right (rows aligned).
//
// The name says who; the card under it says which address that is, and — only when they are two — who put the commit
// here and when (デザイン規約 §author の hover). The co-author credit follows the same shape, so the pane has one way of
// naming a person rather than two. The cards themselves stay with the pane: they open in pane coordinates and must not
// scroll away with the block this row sits on, so this row raises the anchor point and the pane opens the card there.
RowLayout {
    id: authorRow

    required property var details
    /// What git makes of this commit's signature (`SignatureMark`).
    property string signatureKind: ""
    property string signatureCode: ""
    property string signatureSigner: ""
    /// Stand-ins for the pointer where headless cannot put one (avatar-hover / signature-tip) — hover cannot be
    /// injected.
    property bool avatarPointedAt: false
    property bool signaturePointedAt: false
    /// The pane's width: the credit line's share is half of it, and the pane, not this row, is what the splitter sizes.
    property real paneWidth: 0
    /// The cards' own hover, fed back in: walking down into a card takes the pointer off the stretch that opened it,
    /// and the two must not fight over it.
    property bool mateCardInside: false
    property bool authorCardInside: false

    // ---- co-authors -------------------------------------------------
    // Packed by encode::encode_co_authors; unpacked here the way the graph rows unpack their chips. A commit object
    // holds one author, so everyone else arrives as a `Co-authored-by` trailer and is shown as what it is: a line the
    // message credits, under the author rather than beside them (デザイン規約 §co-author).
    readonly property var coAuthorRecords: coBlock.records
    /// Whether the pointer is on the underlined stretch. The real hover and the automation hook write this same one, so
    /// a run cannot go green with the hover unwired.
    property bool matesPointed: false
    /// ...or on the card it opened.
    readonly property bool matesLit: authorRow.matesPointed || authorRow.mateCardInside

    // ---- the author's own card --------------------------------------
    /// Whether the pointer is on the name — the same pair of writers as `matesPointed`.
    property bool authorPointed: false
    readonly property bool authorLit: authorRow.authorPointed || authorRow.authorCardInside

    /// The verdict tooltip is up (`SignatureMark.tipShown` — the output side, so a cut binding cannot read as green).
    readonly property bool signatureTipShown: avatarBadge.signatureTipShown

    signal avatarClicked()
    signal copyRequested(string text)
    signal parentClicked(string oidHex)
    /// Open the card at this anchor (row coordinates), or start the settle beat that closes it. Raised whenever the lit
    /// state turns, which covers the real hover and the automation stand-in alike — the anchor is computed at the
    /// moment it is asked for, never bound (the answer only matters then).
    signal openMateRequested(point at)
    signal settleMateRequested()
    signal openAuthorRequested(point at)
    signal settleAuthorRequested()

    /// Smoke hook and hover handler both land here.
    function showCoAuthors(on) {
        authorRow.matesPointed = on
    }
    function showAuthor(on) {
        authorRow.authorPointed = on
    }
    function coAuthorName(i) {
        return coBlock.nameAt(i)
    }

    onMatesLitChanged: {
        if (authorRow.matesLit)
            authorRow.openMateRequested(coBlock.mapToItem(authorRow, 0, coBlock.height))
        else
            authorRow.settleMateRequested()
    }
    onAuthorLitChanged: {
        if (authorRow.authorLit)
            authorRow.openAuthorRequested(authorLabel.mapToItem(authorRow, 0, authorLabel.height))
        else
            authorRow.settleAuthorRequested()
    }

    spacing: Theme.spaceSm

    // The face carries the signature in its top-right corner (デザイン規約 §署名の表示): the mark is about the person
    // whose commit this is, and the corner is where the editor's own face carries the same thing.
    AvatarButton {
        id: avatarBadge
        face: authorRow.details.avatar
        faceUrl: authorRow.details.avatarUrl
        email: authorRow.details.authorEmail
        pointedAt: authorRow.avatarPointedAt
        signatureKind: authorRow.signatureKind
        signatureCode: authorRow.signatureCode
        signatureSigner: authorRow.signatureSigner
        signaturePointedAt: authorRow.signaturePointedAt
        // Out of the round face's empty corner and onto the pane's ground, as far as the air above the row allows: one
        // pixel short of the band's own hairline (§署名の表示).
        badgeTopOut: Theme.spaceXs - Theme.borderWidth
        badgeRightOut: Theme.spaceXs - Theme.borderWidth
        Layout.preferredWidth: Metrics.detailsAvatar
        Layout.preferredHeight: Metrics.detailsAvatar
        onClicked: authorRow.avatarClicked()
    }
    ColumnLayout {
        spacing: 0
        Layout.fillWidth: true
        // Name, and beside it the one word a signature ever spends. The verdict itself is a mark on the face
        // (デザイン規約 §署名の表示) — but a broken one reads as the error message it is, and an error nobody can
        // find without hovering is not one, so that single case keeps a word out here beside the name it contradicts.
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            Label {
                id: authorLabel
                text: authorRow.details.authorName
                elide: Text.ElideRight
                // Grows no further than the name itself, so the mark sits against the name rather than being pushed
                // across to the hash — and shrinks, with the name eliding, when a long one would otherwise crowd the
                // mark out.
                Layout.fillWidth: true
                Layout.maximumWidth: authorLabel.implicitWidth
                color: Theme.textPrimary
                font.pixelSize: Theme.fontMd
                font.weight: Font.DemiBold
                // Drawn at rest one step down from the name it underlines, and up to the name's own value under the
                // pointer — the same rule the credit line carries, because it says the same thing: there is more here,
                // and hovering opens it (規約 §co-author の表示).
                BandRule {
                    visible: authorLabel.text !== ""
                    color: authorRow.authorLit ? Theme.textPrimary : Theme.borderStrong
                }
                // A handler, not a `MouseArea`: handlers are passive, so the card it opens keeps its own hover (規約
                // §hover のツールチップ).
                HoverHandler {
                    id: authorHover
                    onHoveredChanged: authorRow.showAuthor(authorHover.hovered)
                }
            }
            Label {
                visible: avatarBadge.signatureBroken
                text: qsTr("Bad signature")
                color: avatarBadge.signatureTone
                font.pixelSize: Theme.fontSm
                Layout.alignment: Qt.AlignVCenter
            }
            // The slack lives here, past both of them, so the word stays against the name.
            Item { Layout.fillWidth: true }
        }
        // Date, and beside it whoever the message credits along with the author. A commit with no trailer shows only
        // the date.
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            Label {
                id: detailsDate
                text: Words.stamp(authorRow.details.authorTime)
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
            // The face and name of the first co-author, then a count of the rest — the same "+N" the graph chips use,
            // so the row's width never moves. One rule runs under the lot, the way the hash and its copy icon share
            // one: the two are one target.
            CoAuthorLine {
                id: coBlock
                packed: authorRow.details.coAuthors
                lit: authorRow.matesLit
                // Half the pane, the share the hover card gives the same line out of the graph pane. The date holds the
                // left of this row; this is the rest.
                nameWidth: authorRow.paneWidth / 2
                // Grows no further than the names themselves, and gives way when the row cannot hold them -- the rule
                // the author's name above already follows. Without the pair this line is Fixed, and a Fixed item is a
                // floor the layout cannot go under: the row then lays out at its own width and every box in the pane,
                // sized to fill it, paints past the window's edge (measured at 483 against a 384px pane).
                Layout.fillWidth: true
                Layout.maximumWidth: coBlock.implicitWidth
                Layout.alignment: Qt.AlignVCenter
                onPointerChanged: inside => authorRow.showCoAuthors(inside)
            }
            Item { Layout.fillWidth: true }
        }
    }
    HashPlate {
        Layout.alignment: Qt.AlignRight
        sha8: authorRow.details.sha8
        fullSha: authorRow.details.shaHex
        parentSha: authorRow.details.parentHex
        onCopyRequested: text => authorRow.copyRequested(text)
        onParentClicked: oidHex => authorRow.parentClicked(oidHex)
    }
}
