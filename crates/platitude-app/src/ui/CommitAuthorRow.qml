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
// **An `Item` around the row**, because the hand a range selection is taken with lies under everything it draws and a
// layout would give that hand a place in the line (`SweepRoom`, 規約 §右のペインの字は掴める).
Item {
    id: authorRow

    implicitWidth: rowContent.implicitWidth
    implicitHeight: rowContent.implicitHeight

    /// Whether this row was given the whole of the block it stands in. **The hash plate is right-aligned against this
    /// row's edge**, so a row that kept only its own width parks the plate against the end of the name and the pane's
    /// right column stops being one (observed — the row became an Item, and a plain item in a layout
    /// takes its implicit width and stops). A photograph shows a plate either way; it does not say which edge it was
    /// meant to be on. **Judge it where the row is narrower than its block** (--preset authorship: 237 against 388)
    /// — where the content already overflows, a row that never filled is clamped to the same width and answers true.
    readonly property bool fillsBlock: !!authorRow.parent && authorRow.width + 0.5 >= authorRow.parent.width

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
    readonly property bool signatureTipShown: sigMark.tipShown

    /// The name did not fit the room the row gave it. Nothing is drawn differently for it — the ellipsis already says
    /// so — but a headless run cannot see an ellipsis, and the width rule against `authorLabel` is the whole of what
    /// this row promises the name (`LineText.clipped`, the output side).
    readonly property bool nameClipped: authorLabel.clipped
    /// The same question for the credit line under it (`CoAuthorLine.clipped`).
    readonly property bool matesClipped: coBlock.clipped

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

    /// Automation: a drag cannot be injected, so a run picks a field out the way `Ctrl+A` does and reads back what it
    /// holds — the same pair `tip-copy` uses to prove a tooltip is a field (verify-ui). The name is the field's own,
    /// so a value still drawn as a label reports nothing. The clipboard itself is left alone: writing to it would be
    /// testing Qt rather than this pane.
    function selectValue(which) {
        if (which === "author")
            authorLabel.selectAll()
        else if (which === "date")
            detailsDate.selectAll()
        else if (which === "mate")
            coBlock.selectNames()
        else if (which === "hash")
            hashPlate.selectSha()
        else if (which === "parent")
            hashPlate.selectParent()
    }
    function selectedValue(which) {
        if (which === "author")
            return authorLabel.selected
        if (which === "date")
            return detailsDate.selected
        if (which === "mate")
            return coBlock.namesSelected
        if (which === "hash")
            return hashPlate.shaSelected
        if (which === "parent")
            return hashPlate.parentSelected
        return ""
    }
    // ---- what a sweep over this row needs to know (`SweepRoom`) ------
    //
    // The row draws its values on two lines, and a gesture belongs to one of them. **Which one is decided by where
    // the press landed** — reading it off how near the pointer happened to be to one row of glyphs picked the value
    // above wherever the lines were taller (Ubuntu, observed). The border is the middle of the gap between the two
    // lines, **measured on the words themselves**: the slack beside them carries no content and so no height of its
    // own, and a border read off one lands wherever a spacer happened to be centred (3 of 9 sweeps, same day).
    readonly property real lineBorder:
        (authorLabel.mapToItem(authorRow, 0, authorLabel.height).y + detailsDate.mapToItem(authorRow, 0, 0).y) / 2
    function lineAt(y) {
        return y < authorRow.lineBorder ? 0 : 1
    }
    function lineValues(line) {
        return line === 0
            ? [authorLabel, hashPlate.fieldFor("hash")]
            : [detailsDate].concat(coBlock.valueFields, [hashPlate.fieldFor("parent")])
    }
    /// Whether a press at that point belongs to a control of this row — the avatar badge and the plate's two. A press
    /// there is the control's, whole: this is what keeps every hit area the size it has always been.
    function claimedAt(item, x, y) {
        const a = avatarBadge.mapFromItem(item, x, y)
        if (a.x >= 0 && a.y >= 0 && a.x < avatarBadge.width && a.y < avatarBadge.height)
            return true
        return hashPlate.claims(item, x, y)
    }
    /// Nothing on this row is holding a selection any more. One selection in the window (規約 §右のペインの字は掴める),
    /// and a sweep clears the board before it puts one anywhere.
    function dropValues() {
        const fields = authorRow.lineValues(0).concat(authorRow.lineValues(1))
        for (let i = 0; i < fields.length; i++) {
            if (fields[i])
                fields[i].deselect()
        }
    }

    // ---- and what a run needs, on top of that (verify-ui) -------------
    /// The field that draws one value, the slack a hand reaches for it through, and how far that value's line runs —
    /// **the whole band, not the words' own height**, because the air above and below a value is the row's too and a
    /// hand that did not answer there is what the reader ran into.
    function fieldFor(which) {
        if (which === "author")
            return authorLabel
        if (which === "date")
            return detailsDate
        if (which === "mate")
            return coBlock.valueFields[0] ?? null
        return hashPlate.fieldFor(which)
    }
    function slackFor(which) {
        return which === "author" || which === "hash" ? nameSlack : dateSlack
    }
    function bandFor(which) {
        return which === "author" || which === "hash"
            ? [0, authorRow.lineBorder]
            : [authorRow.lineBorder, authorRow.height]
    }
    /// Whether that value is drawn and **laid out to its own width**, and the geometry a run watches settle before it
    /// sweeps. A field a fraction of a pixel wide passes every looser test and then answers the same character at
    /// both ends of a drag, so the sweep comes away with nothing; and the pane lays out more than once on its way to
    /// a commit, so a single sample can be of a frame nobody sees. None of the values a sweep is asked for is ever
    /// cut — the cut ones are read with `Ctrl+A` — so "as wide as its words" is the honest readiness.
    function valueReady(which) {
        const f = authorRow.fieldFor(which)
        const slack = authorRow.slackFor(which)
        return !!f && f.visible && f.text !== "" && f.width + 0.5 >= f.implicitWidth
            && !!slack && slack.width > 0
    }
    function valueGeom(which) {
        const f = authorRow.fieldFor(which)
        const slack = authorRow.slackFor(which)
        return (f ? Math.round(f.width) + "x" + Math.round(f.height) : "-")
            + "," + (slack ? Math.round(slack.width) + "x" + Math.round(slack.height) : "-")
    }
    /// Where a run aims to ask "is a press here still somebody's control?" — the middle of the control on the two the
    /// plate draws, the middle of the words on the ones nothing stands over — and whether those words answer a press
    /// of their own, which is false only where a control stands over them.
    function controlPoint(which, item) {
        if (which === "hash" || which === "parent")
            return hashPlate.controlPoint(which, item)
        const f = authorRow.fieldFor(which)
        return f ? f.mapToItem(item, f.width / 2, f.height / 2) : Qt.point(0, 0)
    }
    function valueGrabs(which) { const f = authorRow.fieldFor(which); return !!f && f.grabs }
    /// The plate's own gesture, entered where its hand enters it — a run has no pointer to press or drag with
    /// (`HashPlate`). The tap is what puts the whole hash on the clipboard; the drag is what leaves the shown one
    /// picked out instead.
    function tapHash() { hashPlate.tapAt("hash") }
    function dragHash() { hashPlate.dragAcross("hash") }
    function hashHandStands() { return hashPlate.handStands("hash") }
    /// The plate's one word, from either side: what the copy control is offering, and what the tip is carrying on
    /// screen. Raised without a pointer, which is the only way a run can raise one at all (`HashPlate`).
    function forceHashTip(on) { hashPlate.forceTip(on) }
    function hashTipWords() { return hashPlate.tipWords() }
    function hashTipSaid() { return hashPlate.tipSaid() }
    /// What the plate's own control hands over, which is more than the row is showing.
    readonly property string fullShown: authorRow.details.shaHex
    /// The sweep as a hand makes it, and what it came away with (`SweepRoom`).
    function sweepAt(which, fx, fy) { return sweepHand.sweepAt(which, fx, fy) }
    function pressOnControl(which) { return sweepHand.pressOnControl(which) }
    function sweptField() { return sweepHand.endedOn }
    function sweptCaret() { return sweepHand.caretLanded }
    function sweptTrace() { return sweepHand.trace }
    /// What the pane is drawing at that value, read off the model rather than off the field: the two have to agree for
    /// a run to pass.
    function shownValue(which) {
        if (which === "author")
            return authorRow.details.authorName
        if (which === "date")
            return Words.stamp(authorRow.details.authorTime)
        if (which === "mate")
            return coBlock.nameAt(0)
        if (which === "hash")
            return authorRow.details.sha8
        if (which === "parent")
            return authorRow.details.parentHex.substring(0, 8)
        return ""
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

    // The hand a range selection is taken with, under everything the row draws: a press reaches it only where no
    // control and no value took one — which is every gap in the row and nothing else (規約 §右のペインの字は掴める).
    // Declared before the content so it lies beneath it.
    SweepRoom {
        id: sweepHand
        anchors.fill: parent
        row: authorRow
    }
    RowLayout {
        id: rowContent
        anchors.fill: parent

        // The pane's own inset, not the default step: the face stands `spaceXs` off the pane's edge, so a wider gap on its
        // other side reads as the name having been pushed away from a face that is where it belongs (デザイン規約 §余白 —
        // 行内の詰め).
        spacing: Theme.spaceXs

        // The face here carries the pen and nothing else: this row shows the name, and the name is what the signature is
        // hung off (デザイン規約 §署名の表示). The editor's face wears the mark in its corner because out there no name is
        // written.
        AvatarButton {
            id: avatarBadge
            face: authorRow.details.avatar
            faceUrl: authorRow.details.avatarUrl
            email: authorRow.details.authorEmail
            pointedAt: authorRow.avatarPointedAt
            Layout.preferredWidth: Metrics.detailsAvatar
            Layout.preferredHeight: Metrics.detailsAvatar
            onClicked: authorRow.avatarClicked()
        }
        ColumnLayout {
            spacing: 0
            Layout.fillWidth: true
            // Name, the mark on its shoulder, and the one word a signature ever spends. The verdict is about the person,
            // and this is where the person is named (デザイン規約 §署名の表示) — a broken one also reads as the error
            // message it is, and an error nobody can find without hovering is not one, so that single case spends a word
            // as well, out past the mark and still against the name it contradicts.
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                // A field rather than a label: the name is the first thing a reader takes off this pane, and here it is
                // taken the way any other text in this window is — by dragging over it (規約 §右のペインの字は掴める). A
                // field has no `elide`, so a name too long for the row is clipped with a mark and read in full in the card
                // the hover below opens.
                LineText {
                    id: authorLabel
                    text: authorRow.details.authorName
                    // Grows no further than the name itself, so the mark sits against the name rather than being pushed
                    // across to the hash — and shrinks, with the name eliding, when a long one would otherwise crowd the
                    // mark out. **Rounded up**: a natural width lands on a fraction of a pixel as often as not, and the
                    // layout hands the item the whole pixel below it, which is a ceiling one hair under the name's own
                    // width — the name then elides against a rule meant to fit it (app-ui.md §自然幅の上限は切り上げる).
                    Layout.fillWidth: true
                    Layout.maximumWidth: authorLabel.implicitWidth
                    color: Theme.textPrimary
                    pixelSize: Theme.fontMd
                    weight: Font.DemiBold
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
                // The shoulder every `!` in this app stands on: raised to the top of the name's own line rather than set
                // level with it, and pulled half a gap in, so it reads as part of the name instead of as the next column
                // (`NameCell`, `AppMenuItem`, the toolbar's `push -f` — デザイン規約 §git 用語のコード表記).
                //
                // Both distances are measured to the **ink**, never to the box (§余白): the tick's ink starts 3.5 of its
                // grid in and the bang's 7, so a seat written to the box would stand a broken signature two and a half
                // pixels further off the name than a good one. The right-hand air comes off the same way, so what follows
                // is a gap from the mark rather than from where the mark's square happens to end.
                SignatureMark {
                    id: sigMark
                    kind: authorRow.signatureKind
                    code: authorRow.signatureCode
                    signer: authorRow.signatureSigner
                    pointedAt: authorRow.signaturePointedAt
                    // A mark beside a word carries the letters' weight, not a badge's (`NavIcon.stroke`).
                    stroke: Metrics.iconStroke * Theme.iconSm / Theme.iconMd
                    Layout.alignment: Qt.AlignTop
                    Layout.leftMargin: -(Theme.spaceXs / 2 + sigMark.inkAirLeft)
                    Layout.rightMargin: -sigMark.inkAirRight
                }
                Label {
                    visible: sigMark.broken
                    text: qsTr("Bad signature")
                    color: sigMark.tone
                    font.pixelSize: Theme.fontSm
                    Layout.alignment: Qt.AlignVCenter
                }
                // The slack lives here, past all three, so the mark and the word stay against the name. It is also the
                // widest place a hand can reach for this line, so it never closes entirely (`SweepRoom`).
                Item {
                    id: nameSlack
                    Layout.fillWidth: true
                }
            }
            // Date, and beside it whoever the message credits along with the author. A commit with no trailer shows only
            // the date.
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceSm
                LineText {
                    id: detailsDate
                    text: Words.stamp(authorRow.details.authorTime)
                    color: Theme.textSecondary
                    pixelSize: Theme.fontSm
                }
                // The face and name of the first co-author, then a count of the rest — the same "+N" the co-author line
                // uses, so the row's width never moves. One rule runs under the lot, the way the hash and its copy icon share
                // one: the two are one target.
                CoAuthorLine {
                    id: coBlock
                    packed: authorRow.details.coAuthors
                    lit: authorRow.matesLit
                    // Half the pane, the share the hover card gives the same line out of the graph pane. The date holds the
                    // left of this row; this is the rest.
                    nameWidth: authorRow.paneWidth / 2
                    // Grows no further than the names themselves, and gives way when the row cannot hold them -- the rule
                    // the author's name above already follows, rounding up with it. Without the pair this line is Fixed,
                    // and a Fixed item is a floor the layout cannot go under: the row then lays out at its own width and
                    // every box in the pane, sized to fill it, paints past the window's edge (measured at 483 against a
                    // 384px pane).
                    Layout.fillWidth: true
                    Layout.maximumWidth: Math.ceil(coBlock.implicitWidth)
                    Layout.alignment: Qt.AlignVCenter
                    onPointerChanged: inside => authorRow.showCoAuthors(inside)
                }
                Item {
                    id: dateSlack
                    Layout.fillWidth: true
                }
            }
        }
        HashPlate {
            id: hashPlate
            Layout.alignment: Qt.AlignRight
            sha8: authorRow.details.sha8
            fullSha: authorRow.details.shaHex
            parentSha: authorRow.details.parentHex
            onCopyRequested: text => authorRow.copyRequested(text)
            onParentClicked: oidHex => authorRow.parentClicked(oidHex)
        }
    }
}
