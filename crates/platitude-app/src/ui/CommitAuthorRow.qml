import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The details pane's author row: avatar + name/date on the left, own hash over the parents' on the right.
//
// Hovering the name or the co-author credit opens a card (デザイン規約 §author の hover / §co-author の表示). The cards
// belong to the pane, which keeps them put while this block scrolls; this row only raises the anchor point.
// An `Item` around the layout: the sweep hand lies under everything and a layout would give it a place in the line
// (`SweepRoom`, 規約 §右のペインの字は掴める).
Item {
    id: authorRow

    implicitWidth: rowContent.implicitWidth
    implicitHeight: rowContent.implicitHeight

    /// Whether this row was given the whole block it stands in: the hash plate is right-aligned to the row, so a row
    /// at its implicit width parks the plate against the name. A PNG cannot tell; judge it where the row is narrower
    /// than its block (--preset authorship) — an overflowing row is clamped and answers true anyway.
    readonly property bool fillsBlock: !!authorRow.parent && authorRow.width + 0.5 >= authorRow.parent.width

    required property var details
    /// What git makes of this commit's signature (`SignatureMark`).
    property string signatureKind: ""
    property string signatureCode: ""
    property string signatureSigner: ""
    /// Headless stand-ins for hover, which cannot be injected (avatar-hover / signature-tip).
    property bool avatarPointedAt: false
    property bool signaturePointedAt: false
    /// The pane's width; the credit line takes half of it.
    property real paneWidth: 0
    /// A stash entry: its parents past the first are the index and untracked commits git files it with, which the graph
    /// sifts out (`rows::sift_batch`), so the line names only the base it was made on.
    property bool stashed: false
    /// The cards' own hover, fed back so the stretch that opened a card and the card itself read as one.
    property bool mateCardInside: false
    property bool authorCardInside: false

    // ---- co-authors -------------------------------------------------
    // `Co-authored-by` trailers (`encode::mates_of`), shown as a credit line under the author
    // (デザイン規約 §co-author の表示).
    readonly property var coAuthorRecords: coBlock.records
    /// Whether the pointer is on the underlined stretch. The real hover and the automation hook write this same one, so
    /// a run cannot go green with the hover unwired.
    property bool matesPointed: false
    /// ...or on the card it opened.
    readonly property bool matesLit: authorRow.matesPointed || authorRow.mateCardInside

    // ---- the parents' card -----------------------------------------
    // Every parent over the parent line, opened from its count (デザイン規約 §右のペインの親).
    /// Whether the pointer is on the count — the same pair of writers as `matesPointed`.
    property bool parentsPointed: false
    /// ...or on the card it opened.
    property bool parentCardInside: false
    readonly property bool parentsLit: authorRow.parentsPointed || authorRow.parentCardInside
    /// The card is out, and the line stands as one parent's would under it (`HashPlate.unrolled`).
    property bool parentsUnrolled: false

    // ---- the author's own card --------------------------------------
    /// Whether the pointer is on the name — the same pair of writers as `matesPointed`.
    property bool authorPointed: false
    readonly property bool authorLit: authorRow.authorPointed || authorRow.authorCardInside

    /// The verdict tooltip is up (`SignatureMark.tipShown` — the output side, so a cut binding cannot read as green).
    readonly property bool signatureTipShown: sigMark.tipShown

    /// The name did not fit — for headless runs, which cannot see an ellipsis (`LineText.clipped`, the output side).
    readonly property bool nameClipped: authorLabel.clipped
    /// The same question for the credit line under it (`CoAuthorLine.clipped`), and for the date beside that.
    readonly property bool matesClipped: coBlock.clipped
    readonly property bool dateClipped: detailsDate.clipped
    /// Whether the credit folded to its face and a count, and the count it says (`CoAuthorLine.folded`).
    readonly property bool matesFolded: coBlock.folded
    readonly property string matesSaid: coBlock.countStands ? coBlock.countSaid : "-"

    signal avatarClicked()
    signal copyRequested(string text)
    signal parentClicked(string oidHex)
    /// Open the card at this anchor (row coordinates), or start the settle beat that closes it. Raised when the lit
    /// state turns, real hover and stand-in alike.
    signal openMateRequested(point at)
    signal settleMateRequested()
    signal openAuthorRequested(point at)
    signal settleAuthorRequested()
    /// The same pair for the parents' card, at the parent line's seat (`HashPlate.parentSeatRect`, row coordinates).
    signal openParentsRequested(rect line)
    signal settleParentsRequested()

    function showCoAuthors(on) {
        authorRow.matesPointed = on
    }
    function showAuthor(on) {
        authorRow.authorPointed = on
    }
    function showParents(on) {
        authorRow.parentsPointed = on
    }
    function coAuthorName(i) {
        return coBlock.nameAt(i)
    }

    /// Automation: a drag cannot be injected, so a run selects a field as `Ctrl+A` does and reads back what it holds;
    /// a value still drawn as a label reports nothing. The clipboard is left alone — that would be testing Qt.
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
            return hashPlate.parentSelected()
        return ""
    }
    /// The width the parent line may take: what neither line's words need, each line keeping a slack of its own
    /// (デザイン規約 §右のペインの親). Both, since the plate is one column as wide as its widest row. Read off the lines'
    /// natural widths, which the plate's width does not move.
    readonly property real parentRoom: rowContent.width - avatarBadge.width - 2 * rowContent.spacing
        - Math.max(nameLine.implicitWidth, authorRow.dateNeed) - Theme.spaceXs
    /// The date line's words at their own widths, with the step before its slack — counted here rather than read off
    /// the line, whose date is held to what the line is handed (`detailsDate`), which the plate's width moves.
    readonly property real dateNeed: detailsDate.implicitWidth + dateLine.spacing
        + (coBlock.visible ? Math.ceil(coBlock.implicitWidth) + dateLine.spacing : 0)
    /// Who gives way on the date line when it is short (デザイン規約 §co-author の表示): the credit's name is cut, then
    /// folded to its face and count, and only then does the date give up its time. Measured top down — the row less
    /// the face and the plate — not off the line, whose items are laid out to these: a floor read off the width it
    /// holds up would hold it up for good.
    readonly property real textRoom: Math.max(0, rowContent.width - avatarBadge.width - 2 * rowContent.spacing
        - hashPlate.implicitWidth)
    /// What the line can spare the date, the credit at its floor.
    readonly property real dateSpare: Math.max(0, authorRow.textRoom
        - dateLine.spacing - (coBlock.visible ? coBlock.foldedWidth + dateLine.spacing : 0))
    /// The date cut back to its day and the mark — the one step it takes: the time goes whole, since a stamp cut
    /// through its figures reads as another time. Under the day's own width there is only the cut the field makes.
    readonly property real dateDayRoom: Math.ceil(dayRuler.implicitWidth) + detailsDate.markWidth
    readonly property real dateRoom: authorRow.dateSpare >= detailsDate.implicitWidth ? detailsDate.implicitWidth
        : Math.min(authorRow.dateSpare, authorRow.dateDayRoom)
    readonly property bool dateShort: authorRow.dateRoom < detailsDate.implicitWidth
    /// The credit stays at its floor while the date is short of its time: what the time gave up is not the name's.
    readonly property real creditCeiling: authorRow.dateShort ? coBlock.foldedWidth : Math.ceil(coBlock.implicitWidth)
    readonly property real creditRoom: Math.min(authorRow.creditCeiling, Math.max(coBlock.foldedWidth,
        authorRow.textRoom - authorRow.dateRoom - 2 * dateLine.spacing))
    /// Automation: the date and the credit as laid out, which reach `dateRoom` / `creditRoom` a pass after a width is
    /// handed down.
    readonly property real dateWidth: detailsDate.width
    readonly property real creditWidth: coBlock.width
    /// …and how the date came out: `whole`, cut back to its `day`, or `cut` by the field under that.
    readonly property string dateSaid: !detailsDate.clipped ? "whole"
        : Math.abs(detailsDate.width - authorRow.dateDayRoom) < 0.5 ? "day" : "cut"
    /// Where the date line's drawn words end, in `item`'s x: the credit's stretch, or the date where nobody is
    /// credited.
    function dateWordsEnd(item) {
        return coBlock.visible ? coBlock.mapToItem(item, coBlock.stretchWidth, 0).x
                               : detailsDate.mapToItem(item, detailsDate.width, 0).x
    }
    // ---- one baseline per line (デザイン規約 §右のペインの親) ----
    // The two columns stand side by side, each centred on its own content, and words of two faces (the name's, the
    // hashes' mono) sit at different heights in boxes of one height. The left column is the reference: the plate moves
    // its rows onto the name's baseline and the date's (`HashPlate.firstBase` / `secondBase`).
    /// How far down this row an item stands as laid out. Transforms are left out: they are what this feeds.
    function laidY(item) {
        let y = 0
        for (let at = item; at !== null && at !== authorRow; at = at.parent)
            y += at.y
        return y
    }
    readonly property real firstBase: authorRow.laidY(authorLabel) + authorLabel.baselineOffset
    readonly property real secondBase: authorRow.laidY(detailsDate) + detailsDate.baselineOffset

    /// Automation (verify-ui `details-align`): where every word of the row sits, in the window's y (-1 for one not
    /// drawn) — the name and the hash share the first line, the date, the credit and the parents the second.
    function baselines() {
        const plate = hashPlate.baselines(null)
        const mates = coBlock.baselines(null)
        return {
            name: authorLabel.mapToItem(null, 0, authorLabel.baselineOffset).y,
            hash: plate.hash,
            date: detailsDate.mapToItem(null, 0, detailsDate.baselineOffset).y,
            dateText: detailsDate.text,
            dateSize: detailsDate.pixelSize,
            face: coBlock.faceMiddle(null),
            mate: mates.name,
            mateCount: mates.count,
            mateCountSize: mates.countSize,
            parent: plate.parent,
            comma: plate.comma,
            parentCount: plate.count,
            parentCountSize: plate.countSize,
            // How far the date line's words stand clear of the parent line's arrow: the two must not meet.
            apart: hashPlate.parents.length > 0
                ? hashPlate.arrowBox(null).x - authorRow.dateWordsEnd(null) : Number.POSITIVE_INFINITY
        }
    }
    /// Automation (verify-ui `details-parents`): the plate, and where the date line's words end in the row's x.
    readonly property alias plate: hashPlate
    function dateLineEnd() {
        return dateLine.mapToItem(authorRow, 0, 0).x + authorRow.dateNeed - dateLine.spacing
    }
    // ---- what a sweep over this row needs to know (`SweepRoom`) ------
    //
    // A gesture belongs to one of the two lines, decided by where the press landed (nearness to the glyphs picks the
    // line above where lines are taller). The border is the middle of the gap, measured on the words: the slack has
    // no height of its own, so a border read off it lands wherever a spacer happens to be centred.
    readonly property real lineBorder:
        (authorLabel.mapToItem(authorRow, 0, authorLabel.height).y + detailsDate.mapToItem(authorRow, 0, 0).y) / 2
    function lineAt(y) {
        return y < authorRow.lineBorder ? 0 : 1
    }
    function lineValues(line) {
        return line === 0
            ? [authorLabel, hashPlate.fieldFor("hash")]
            : [detailsDate].concat(coBlock.valueFields, hashPlate.parentFields())
    }
    /// Whether a press at that point belongs to a control of this row (the avatar badge, the plate's two), which keep
    /// their whole hit areas.
    function claimedAt(item, x, y) {
        const a = avatarBadge.mapFromItem(item, x, y)
        if (a.x >= 0 && a.y >= 0 && a.x < avatarBadge.width && a.y < avatarBadge.height)
            return true
        return hashPlate.claims(item, x, y)
    }
    /// Clear every selection on this row: one selection in the window (規約 §右のペインの字は掴める).
    function dropValues() {
        const fields = authorRow.lineValues(0).concat(authorRow.lineValues(1))
        for (let i = 0; i < fields.length; i++) {
            if (fields[i])
                fields[i].deselect()
        }
    }

    // ---- and what a run needs, on top of that (verify-ui) -------------
    /// The field that draws one value, the slack a hand reaches it through, and its line's band — the whole band,
    /// since the air above and below a value must answer a press too.
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
    /// Whether that value is drawn and laid out to its own width, and the geometry a run watches settle before it
    /// sweeps. A looser test passes a field a fraction of a pixel wide, which answers the same character at both ends
    /// of a drag. Swept values are never cut (cut ones are read with `Ctrl+A`), so full width is the readiness.
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
    /// Where a run probes whether a press is still a control's (the plate's control for the two hashes, the words'
    /// middle elsewhere), and whether those words take a press of their own (false only under a control).
    function controlPoint(which, item) {
        if (which === "hash" || which === "parent")
            return hashPlate.controlPoint(which, item)
        const f = authorRow.fieldFor(which)
        return f ? f.mapToItem(item, f.width / 2, f.height / 2) : Qt.point(0, 0)
    }
    function valueGrabs(which) { const f = authorRow.fieldFor(which); return !!f && f.grabs }
    /// The plate's gestures, entered where its hand enters them (`HashPlate`): the tap copies the whole hash, the drag
    /// selects the shown one.
    function tapHash() { hashPlate.tapAt("hash") }
    function dragHash() { hashPlate.dragAcross("hash") }
    function hashHandStands() { return hashPlate.handStands("hash") }
    /// The plate's tip, raised without a pointer (`HashPlate`): what the copy control offers, and what the tip shows.
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
    /// What the pane is drawing at that value, read off the model: the two have to agree for a run to pass.
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
    onParentsLitChanged: {
        if (authorRow.parentsLit)
            authorRow.openParentsRequested(hashPlate.parentSeatRect(authorRow))
        else
            authorRow.settleParentsRequested()
    }
    onAuthorLitChanged: {
        if (authorRow.authorLit)
            authorRow.openAuthorRequested(authorLabel.mapToItem(authorRow, 0, authorLabel.height))
        else
            authorRow.settleAuthorRequested()
    }

    // The stamp's day in the date's own face: the step the date is cut back to (`dateDayRoom`).
    Text {
        id: dayRuler
        visible: false
        text: Words.stampDay(authorRow.details.authorTime)
        font.family: Theme.uiFamily
        font.pixelSize: detailsDate.pixelSize
    }
    // Takes the presses no control or value took (規約 §右のペインの字は掴める). Declared before the content so it
    // lies beneath it.
    SweepRoom {
        id: sweepHand
        anchors.fill: parent
        row: authorRow
    }
    RowLayout {
        id: rowContent
        anchors.fill: parent

        // The face's own gap from the pane's edge; any wider reads as the name pushed off the face (デザイン規約 §余白).
        spacing: Theme.spaceXs

        // No signature mark on the face: here it hangs off the name (デザイン規約 §署名の表示).
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
            // Name, the signature mark on its shoulder, and — only for a broken signature — a word, so the error is
            // found without hovering (デザイン規約 §署名の表示).
            RowLayout {
                id: nameLine
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                // A field, so the name can be dragged over (規約 §右のペインの字は掴める). It has no `elide`: a long name
                // is clipped with a mark and read in full in the hover card.
                LineText {
                    id: authorLabel
                    text: authorRow.details.authorName
                    // Capped at the name's own width so the mark sits against it, and shrinks with the name clipped
                    // when a long one would crowd the mark out. `LineText.implicitWidth` is already rounded up
                    // (rules-refs/app-ui.md「自然幅の上限は切り上げる」).
                    Layout.fillWidth: true
                    Layout.maximumWidth: authorLabel.implicitWidth
                    color: Theme.textPrimary
                    pixelSize: Theme.fontMd
                    weight: Theme.fontWeightStrong
                    // The "hover opens more" underline, as on the credit line (規約 §co-author の表示).
                    BandRule {
                        visible: authorLabel.text !== ""
                        color: authorRow.authorLit ? Theme.textPrimary : Theme.borderStrong
                    }
                    // A handler is passive, so the card it opens keeps its own hover (規約 §hover のツールチップ).
                    HoverHandler {
                        id: authorHover
                        onHoveredChanged: authorRow.showAuthor(authorHover.hovered)
                    }
                }
                // On the name's shoulder like every `!` in the app: top-aligned, pulled half a gap in, both distances
                // measured to the ink, which differs per mark (rules-refs/app-ui.md「署名の印は名前の右肩」).
                SignatureMark {
                    id: sigMark
                    kind: authorRow.signatureKind
                    code: authorRow.signatureCode
                    signer: authorRow.signatureSigner
                    pointedAt: authorRow.signaturePointedAt
                    // A mark beside a word carries the letters' weight (`NavIcon.stroke`).
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
                // The slack goes past all three so the mark and the word stay against the name; it is also the hand's
                // widest way into this line (`SweepRoom`).
                Item {
                    id: nameSlack
                    Layout.fillWidth: true
                }
            }
            RowLayout {
                id: dateLine
                Layout.fillWidth: true
                // The date's minutes stand from the credit's face as the face from its name.
                spacing: Theme.spaceXs
                LineText {
                    id: detailsDate
                    text: Words.stamp(authorRow.details.authorTime)
                    color: Theme.textSecondary
                    pixelSize: Theme.fontSm
                    // The credit beside it sits on the same baseline, its face and count in another box.
                    Layout.alignment: Qt.AlignBaseline
                    // Held at its room both ways (`dateRoom`), so the credit's name gives way — cut, then folded —
                    // before the date does.
                    Layout.preferredWidth: authorRow.dateRoom
                    Layout.minimumWidth: authorRow.dateRoom
                    Layout.maximumWidth: authorRow.dateRoom
                }
                // The first co-author's face and name, then `+N` (デザイン規約 §co-author の表示).
                CoAuthorLine {
                    id: coBlock
                    records: authorRow.details.coAuthors
                    lit: authorRow.matesLit
                    nameWidth: authorRow.paneWidth / 2
                    // Capped at the names' width and yielding when the row cannot hold them, as the name above does.
                    // Without the pair the line is Fixed — a floor the layout cannot go under — and every box in the
                    // pane paints past the window's edge.
                    Layout.fillWidth: true
                    Layout.maximumWidth: authorRow.creditCeiling
                    Layout.preferredWidth: authorRow.creditRoom
                    // Folded is as small as it draws: under that its face and count would run into the parents.
                    Layout.minimumWidth: coBlock.foldedWidth
                    Layout.alignment: Qt.AlignBaseline
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
            parents: authorRow.stashed && authorRow.details.parentHex !== "" ? [authorRow.details.parentHex]
                                                                             : authorRow.details.parentHexes
            parentRoom: authorRow.parentRoom
            unrolled: authorRow.parentsUnrolled
            firstBase: authorRow.firstBase - authorRow.laidY(hashPlate)
            secondBase: authorRow.secondBase - authorRow.laidY(hashPlate)
            onMorePointedChanged: authorRow.showParents(hashPlate.morePointed)
            onCopyRequested: text => authorRow.copyRequested(text)
            onParentClicked: oidHex => authorRow.parentClicked(oidHex)
        }
    }
}
