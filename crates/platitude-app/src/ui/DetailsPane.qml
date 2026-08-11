pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Right pane, commit-details mode: message, author card, stash
// actions when the selected row is a stash, and the changed-file list.
// The message boxes are the editor for that commit's message — the
// same pair the working-tree pane commits with, so a message is
// written and rewritten in the same place.
ColumnLayout {
    id: detailsPane

    required property var details
    // Reflog selector when the selected row is a stash ("" otherwise).
    property string stashRef: ""
    // Whether this commit's message may be rewritten from here. The
    // page decides: only commits the working tree stands on can be
    // amended or replayed, and a stash is a commit but not one of those.
    property bool editable: false
    // A write is already running, so nothing new starts.
    property bool busy: false
    // HEAD's own commit: anything older is replayed instead of amended,
    // which the editor says out loud.
    property string headOid: ""
    // Why the boxes are read-only, in one line ("" when they are not).
    // A box that refuses typing without saying why reads as broken.
    property string editBlocked: ""
    // Something wants to move off this commit while the message is
    // half-written. The question belongs to the text, so it is asked
    // where the text is: the row under the boxes turns into it.
    property bool asking: false
    // A remote already has this commit. Rewriting is not asked about,
    // but 実装計画.md §13 wants it said, so the save row carries the warning.
    property bool published: false
    // What git makes of this commit's signature, once the check comes
    // back: "" (unsigned, or not answered yet), "verified", "signed" or
    // "bad". The letter behind it is git's own `%G?` code, which is what
    // the tooltip needs -- the mark says one of three things, but the
    // reason a signature could not be judged is one of five.
    property string signatureKind: ""
    property string signatureCode: ""
    property string signatureSigner: ""
    /// The badge was pressed: the settings card opens already knowing whom
    /// it is about. There is no menu in between and no second place
    /// pictures are managed from — assigning, replacing and removing all
    /// live in the one list, and this is the way in that saves naming the
    /// person.
    signal avatarEditRequested(string name, string email)
    /// Stands in for the pointer where headless cannot put one, so the
    /// badge can be photographed (PG_AUTO_ACT=avatar-hover).
    property bool avatarPointedAt: false
    /// The same stand-in for the verdict mark and the read-only summary
    /// box, so their tooltips can be photographed (signature-tip /
    /// stash-tip). Reported through the ToolTip's own visible — the
    /// output side, so a cut binding cannot read as green.
    property bool signaturePointedAt: false
    readonly property bool signatureTipShown: signatureMark.ToolTip.visible
    property bool summaryPointedAt: false
    readonly property bool summaryTipShown: subjectArea.ToolTip.visible
    /// The same stand-in for a file row, so a cut-down paths-view row's
    /// tooltip can be photographed (path-tip). -1 points at no row.
    property int pointedTipRow: -1
    function avatarClicked() {
        if (detailsPane.details.authorEmail !== "")
            detailsPane.avatarEditRequested(detailsPane.details.authorName,
                                            detailsPane.details.authorEmail)
    }
    /// Conclusion first, one line (デザイン規約 §hover のツールチップ).
    readonly property string signatureTip:
        detailsPane.signatureCode === "G"
        ? (detailsPane.signatureSigner !== ""
           ? qsTr("Signed by %1").arg(detailsPane.signatureSigner)
           : qsTr("Signed by a key you trust"))
        : detailsPane.signatureCode === "B"
          ? qsTr("The content changed after it was signed")
        : detailsPane.signatureCode === "U"
          // git names no signer for this one: the key it read is not one
          // it can put a name to.
          ? qsTr("The key is not one you have vouched for")
        : detailsPane.signatureCode === "X"
          ? qsTr("The signature has expired")
        : detailsPane.signatureCode === "Y"
          ? qsTr("The signing key has expired")
        : detailsPane.signatureCode === "R"
          ? qsTr("The signing key was revoked")
        : qsTr("Cannot be checked — the key is not here")

    // ---- co-authors -------------------------------------------------
    // Packed by encode::encode_co_authors; unpacked here the way the
    // graph rows unpack their chips. A commit object holds one author,
    // so everyone else arrives as a `Co-authored-by` trailer and is
    // shown as what it is: a line the message credits, under the author
    // rather than beside them (デザイン規約 §co-author).
    readonly property var coAuthorRecords: coBlock.records
    function coAuthorName(i) {
        return coBlock.nameAt(i)
    }
    /// Whether the pointer is on the underlined stretch. The real hover
    /// and the automation hook write this same one, so a run cannot go
    /// green with the hover unwired.
    property bool matesPointed: false
    /// ...or on the card it opened: walking down into the card takes the
    /// pointer off the stretch, and the two must not fight over it.
    readonly property bool matesLit:
        detailsPane.matesPointed || mateCard.pointerInside
    /// Smoke hook and hover handler both land here.
    function showCoAuthors(on) {
        detailsPane.matesPointed = on
    }
    /// Whether the card is on screen — what automation reports, since
    /// the input side would read true with the binding cut.
    readonly property bool matesCardOpen: mateCard.opened
    onMatesLitChanged: {
        if (detailsPane.matesLit)
            detailsPane.openMateCard()
        else
            detailsPane.settleMateCard()
    }
    function openMateCard() {
        if (detailsPane.coAuthorRecords.length === 0)
            return
        const at = coBlock.mapToItem(detailsPane, 0, coBlock.height)
        mateCard.records = detailsPane.coAuthorRecords
        // Measured from where it opens, not from the pane: the card
        // starts partway across, so the pane's width is not what is
        // left for it. Set on open rather than bound — the answer only
        // matters at the moment it is asked.
        mateCard.maxRowWidth = detailsPane.width - at.x - 2 * Theme.spaceSm
        mateCard.x = at.x
        // Flush against the underline: a gap is a band the pointer
        // crosses while touching neither, and the card closes under it
        // (2026-08-09 report). Same rule the ref list follows.
        mateCard.y = at.y
        mateCard.open()
    }
    // The card opens flush under the stretch, so walking into it takes
    // the pointer off the stretch on the way and walking back out puts
    // it on again. Both hovers change in the same frame and in no fixed
    // order, so the answer waits for the end of this round of events.
    function settleMateCard() {
        mateSettle.restart()
    }
    // A beat, not a turn of the event loop: the two hovers change in
    // different frames when the pointer walks from the line into the
    // card, and `Qt.callLater` lands between them (2026-08-09 report).
    Timer {
        id: mateSettle
        interval: Metrics.hoverKeepMs
        onTriggered: {
            if (!mateCard.pointerInside && !detailsPane.matesPointed)
                mateCard.close()
        }
    }
    CoAuthorCard {
        id: mateCard
        onPointerInsideChanged: detailsPane.settleMateCard()
    }

    // ---- the author's own card --------------------------------------
    // The name says who; the card under it says which address that is,
    // and — only when they are two — who put the commit here and when
    // (デザイン規約 §author の hover). Everything here is the shape the
    // co-author card already established, including how it is opened,
    // so the pane has one way of naming a person rather than two.
    /// Whether the pointer is on the name. The real hover and the
    /// automation hook write this same one, so a run cannot go green
    /// with the hover unwired.
    property bool authorPointed: false
    readonly property bool authorLit:
        detailsPane.authorPointed || authorCard.pointerInside
    /// Smoke hook and hover handler both land here.
    function showAuthor(on) {
        detailsPane.authorPointed = on
    }
    /// Whether the card is on screen — what automation reports, since
    /// the input side would read true with the binding cut.
    readonly property bool authorCardOpen: authorCard.opened
    onAuthorLitChanged: {
        if (detailsPane.authorLit)
            detailsPane.openAuthorCard()
        else
            authorSettle.restart()
    }
    function openAuthorCard() {
        if (detailsPane.details.authorName === "")
            return
        const at = authorLabel.mapToItem(detailsPane, 0, authorLabel.height)
        // Measured from where it opens, not from the pane: the card
        // starts partway across, so the pane's width is not what is left
        // for it.
        authorCard.maxRowWidth = detailsPane.width - at.x - 2 * Theme.spaceSm
        authorCard.x = at.x
        // Flush against the name: a gap is a band the pointer crosses
        // while touching neither, and the card closes under it.
        authorCard.y = at.y
        authorCard.open()
    }
    // A beat, not a turn of the event loop: the name's hover and the
    // card's change in different frames when the pointer walks from one
    // into the other, and `Qt.callLater` lands between them.
    Timer {
        id: authorSettle
        interval: Metrics.hoverKeepMs
        onTriggered: {
            if (!authorCard.pointerInside && !detailsPane.authorPointed)
                authorCard.close()
        }
    }
    AuthorCard {
        id: authorCard
        authorName: detailsPane.details.authorName
        authorEmail: detailsPane.details.authorEmail
        authorFace: detailsPane.details.avatar
        authorFaceUrl: detailsPane.details.avatarUrl
        authoredAt: detailsPane.details.authorTime
        committerName: detailsPane.details.committerName
        committerEmail: detailsPane.details.committerEmail
        committerFace: detailsPane.details.committerAvatar
        committerFaceUrl: detailsPane.details.committerAvatarUrl
        committedAt: detailsPane.details.committerTime
        committerDiffers: detailsPane.details.committerDiffers
        timeDiffers: detailsPane.details.commitTimeDiffers
        onPointerInsideChanged: authorSettle.restart()
    }

    signal fileActivated(string path, string origPath)
    signal parentClicked(string oidHex)
    signal copyRequested(string text)
    signal applyStashRequested(string selector)
    signal popStashRequested(string selector)
    /// Save was pressed.
    signal messageSubmitted(string oidHex, string subject, string body)
    /// The leaving question was answered: true drops the edits and lets
    /// the move through, false stays on this commit.
    signal leaveResolved(bool discard)

    // ---- message editor state --------------------------------------
    // The boxes are filled by hand rather than bound: typing would
    // break a binding for good, and the next commit would arrive in a
    // box that no longer listens.
    property string baseOid: ""
    property string baseSubject: ""
    property string baseBody: ""
    readonly property bool messageDirty:
        detailsPane.editable
        && (subjectArea.text !== detailsPane.baseSubject
            || bodyArea.text !== detailsPane.baseBody)

    /// Adopt the model's message whenever it moves to another commit.
    /// Nothing else can change a message in place — a different message
    /// is a different commit — so an untouched box needs no other cue.
    function syncMessage() {
        if (detailsPane.details.shaHex === detailsPane.baseOid)
            return
        detailsPane.baseOid = detailsPane.details.shaHex
        detailsPane.baseSubject = detailsPane.details.messageSubject
        detailsPane.baseBody = detailsPane.details.messageBody
        subjectArea.text = detailsPane.baseSubject
        bodyArea.text = detailsPane.baseBody
    }
    /// Put the commit's own message back.
    function revertMessage() {
        subjectArea.text = detailsPane.baseSubject
        bodyArea.text = detailsPane.baseBody
    }
    /// git took the new message. What was written becomes the resting
    /// text: the commit it belonged to is gone under that hash, and the
    /// model still holds the old one until the selection follows.
    function noteMessageSaved() {
        detailsPane.baseSubject = subjectArea.text
        detailsPane.baseBody = bodyArea.text
    }
    function submitMessage() {
        if (!detailsPane.editable || subjectArea.text.trim() === "")
            return
        detailsPane.messageSubmitted(detailsPane.details.shaHex,
                                     subjectArea.text, bodyArea.text)
    }
    /// Smoke hook: type into the boxes the way a keystroke would —
    /// including not at all when they are read-only.
    function setMessageText(subject, body) {
        if (!detailsPane.editable)
            return
        subjectArea.text = subject
        bodyArea.text = body
    }
    /// Smoke hook: put the caret in the description box, the way a click
    /// in it does. Headless has no pointer, and the colour the text
    /// takes under a caret is what the shot is of.
    function focusDescription() {
        bodyArea.takeCaret()
    }
    /// What the box paints — reporting the input side (activeFocus)
    /// would read green with the binding cut.
    readonly property color descriptionColor: bodyArea.textColor
    readonly property bool descriptionFocused: bodyArea.focused

    // -- what this pane lends the description box --
    //
    // The box carries the ceiling and the grip (`DescriptionBox`); the
    // bound is the pane's, because only the pane knows what stands under
    // it (デザイン規約 §コミットメッセージの 2 つの枠). The file list is
    // the one thing here that gives, and two rows is where it stops being
    // a list — so what is left above that is the whole of the room, and
    // everything between the boxes and it (the author, the credit line,
    // the CHANGES band) keeps its own height by construction.
    readonly property real descRoom:
        Math.max(0, fileList.height - 2 * Theme.rowHeight)
    /// The far side of the same measure: how far past the bound the pane
    /// already is, which is what the hand has to give back. The list is
    /// the only thing that gives, so it hits zero and stops answering
    /// while the column keeps growing past the pane's edge — the second
    /// term is that overflow, and without it the give-back stalls at the
    /// last 48 pixels the list still had (measured: the command log
    /// opening under a pulled-open box).
    /// Measured against the room the block is allowed rather than the
    /// height it was laid out at: the block's height follows what the box
    /// does, so reading it back would put the two in a ring where every
    /// pull is handed straight back (see WipPane, where that was measured).
    readonly property real descOwed:
        Math.max(0, 2 * Theme.rowHeight - fileList.height)
        + Math.max(0, blockCol.implicitHeight - detailsPane.blockRoom)
    /// How much of the pane the block between the two bands may take: all
    /// of it but the list's own band and the two rows that keep a list a
    /// list. Past this the block scrolls rather than running out of the
    /// pane's bottom (規約 §窓の床).
    readonly property real blockRoom:
        Math.max(0, detailsPane.height - paneHeader.height
                    - (changesBand.visible ? changesBand.height : 0)
                    - 2 * Theme.rowHeight)
    /// Moves the block by a wheel a box on it could not use — the same
    /// pair the working-tree pane has, and for the same reason: the boxes
    /// cover most of the block, so a box that keeps the wheel at its own
    /// end leaves the block unreachable by wheel (2026-08-09 ユーザー報告).
    function rollBlock(pixels) {
        const max = Math.max(0, blockScroll.contentHeight - blockScroll.height)
        // Taken away, not added — see WipPane: content travels against
        // `contentY`, and adding sent the block the other way from the
        // wheel that reached it.
        blockScroll.contentY =
            Math.max(0, Math.min(max, blockScroll.contentY - pixels))
    }
    function rollSummary(flick, dy) {
        const pixels = dy / 120 * (Metrics.wheelRows * Theme.fontMdLine)
        const max = Math.max(0, flick.contentHeight - flick.height)
        const next = Math.max(0, Math.min(max, flick.contentY - pixels))
        if (Math.abs(next - flick.contentY) > 0.5) {
            flick.contentY = next
            return
        }
        detailsPane.rollBlock(pixels)
    }
    // -- smoke hooks, forwarded to the box --
    function growDescription(dy) { bodyArea.grow(dy) }
    function pullDescriptionPast(down) { bodyArea.pullPast(down) }
    /// Whether the grip is refusing a pull, and where the hand is while it
    /// does (scene coordinates). The page draws the badge — see `RepoPage`
    /// on why it cannot be drawn in the box.
    readonly property alias descRefuses: bodyArea.gripRefused
    readonly property alias descPoint: bodyArea.gripPoint
    readonly property bool descGrips: bodyArea.grips
    readonly property real descHeight: bodyArea.boxHeight
    readonly property real descWants: bodyArea.wants
    readonly property real descCap: bodyArea.cap
    /// How many rows the file list is left with, which is the bound the
    /// pull stops at. Rounded: the layout hands out fractions and what
    /// this is about is rows.
    readonly property int descListRows:
        Math.round(fileList.height / Theme.rowHeight)
    /// Whether the block is taller than the room it was given — anything
    /// in it below the fold. What a pulled-open box pushes past the pane's
    /// edge is exactly what the block ends up scrolling by, so this is
    /// also the answer to "has the box given back what it owes". A shot
    /// frames alike either way (see contentOverflow).
    readonly property bool blockScrolls:
        blockCol.implicitHeight > detailsPane.blockRoom + 1
    readonly property bool descKeeps: !detailsPane.blockScrolls
    /// The same measurement the working-tree pane makes, off this pane's
    /// own list: how much of the bottom edge is left bare for the corner
    /// text the page hangs there (see WipPane.bottomRoom).
    readonly property real bottomRoom:
        detailsPane.height - fileList.y
        - Math.max(0, Math.min(fileList.height,
                               fileList.originY + fileList.contentHeight
                               - fileList.contentY))

    Connections {
        target: detailsPane.details
        function onChanged() { detailsPane.syncMessage() }
    }
    Component.onCompleted: detailsPane.syncMessage()

    /// How far this pane's own content runs past its right edge, in px.
    /// A child with no `Layout.fillWidth` of its own is Fixed -- only a
    /// nested layout fills by default -- and the layout hands a Fixed
    /// child its implicit width and never a pixel less. So one row that
    /// will not give is a floor the whole column sits on. The column then
    /// lays itself out at that floor while the pane keeps the width the
    /// splitter set, and every box in it, sized to fill, paints over the
    /// window's edge with the glyphs cut in half. `fileList` fills and
    /// carries no margins, so its width *is* that laid-out width.
    /// Headless cannot see a cut glyph — this is the number instead.
    readonly property real contentOverflow:
        Math.max(0, fileList.width - detailsPane.width)
    /// The same question the other way up: how far the column runs past
    /// the pane's own bottom once the file list has given everything it
    /// has. Reported for `window-floor`, which is where the answer to
    /// "does this pane need a scroll of its own" comes from.
    readonly property real contentOverHeight:
        Math.max(0, detailsPane.implicitHeight - detailsPane.height)

    spacing: 0

    PaneHeader {
        id: paneHeader
        text: qsTr("COMMIT")
    }
    // Everything between the two bands, in a surface of its own that
    // scrolls when the pane is too short to hold it — the same shape the
    // working-tree pane's block has, and for the same reason: the boxes,
    // the author row and the save row keep their heights by construction,
    // so the file list was the only thing that could give and past zero
    // the rest ran out of the pane's bottom (measured at the window's own
    // floor, on the longest subject git allows: `details_fit overH=160
    // paneH=200`). 規約 §窓の床.
    Flickable {
        id: blockScroll
        Layout.fillWidth: true
        Layout.preferredHeight: Math.min(blockCol.implicitHeight,
                                         detailsPane.blockRoom)
        contentWidth: width
        contentHeight: blockCol.implicitHeight
        clip: true
        // Hard stop at the ends, as everywhere else that scrolls
        // (デザイン規約 §QML 実装ルール).
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: AutoScrollBar {}
        ColumnLayout {
            id: blockCol
            width: blockScroll.width
            spacing: 0

            // Stash actions when the selected row is a stash.
            Rectangle {
                visible: detailsPane.stashRef !== ""
                Layout.fillWidth: true
                implicitHeight: Theme.headerHeight
                color: Theme.bgElevated
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.spaceSm
                    anchors.rightMargin: Theme.spaceXs
                    spacing: Theme.spaceXs
                    NavIcon {
                        kind: "stash"
                        tint: Theme.textSecondary
                        width: Theme.iconMd
                        height: Theme.iconMd
                    }
                    Label {
                        text: detailsPane.stashRef
                        font.family: Theme.monoFamily
                        font.pixelSize: Theme.fontSm
                        color: Theme.textSecondary
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                    HoverToolButton {
                        text: qsTr("Apply")
                        font.pixelSize: Theme.fontSm
                        ToolTip.visible: hovered
                        ToolTip.delay: Metrics.tipDelayMs
                        ToolTip.text: qsTr("Apply this stash, keeping it")
                        onClicked: detailsPane.applyStashRequested(detailsPane.stashRef)
                    }
                    HoverToolButton {
                        text: qsTr("Pop")
                        font.pixelSize: Theme.fontSm
                        ToolTip.visible: hovered
                        ToolTip.delay: Metrics.tipDelayMs
                        ToolTip.text: qsTr("Apply this stash and drop it")
                        onClicked: detailsPane.popStashRequested(detailsPane.stashRef)
                    }
                }
            }
            // Inset on all four sides — the message box carries its own frame,
            // and flush against the header band the two borders read as one
            // welded block. Vertically the inset is the same step the rows
            // inside use, so band → summary → description → author → band is
            // one even rhythm; horizontally it is the pane inset, which puts
            // the card's edge under the header labels.
            ColumnLayout {
                Layout.fillWidth: true
                Layout.margins: Theme.spaceSm
                Layout.topMargin: Theme.spaceXs
                Layout.bottomMargin: Theme.spaceXs
                spacing: Theme.spaceXs
                visible: detailsPane.details.shaHex !== ""

                // -- message first, like the commit editor: a prominent summary
                // box and a dimmer description box --
                Rectangle {
                    Layout.fillWidth: true
                    // Capped and scrolled, the way the description box below it
                    // already is. Nothing in git bounds a summary — it took a
                    // megabyte in the same measurement — and one pasted paragraph
                    // grew this box to 650px, which pushed the description off
                    // the pane and left the author row drawn over the window's
                    // own footer (measured at a 2,000-byte subject).
                    Layout.preferredHeight: Math.min(subjectArea.implicitHeight
                                                     + Theme.spaceSm,
                                                     Theme.messageMaxHeight)
                    color: Theme.bgBase
                    radius: Theme.radiusMd
                    // While the question stands, the boxes it is about carry it:
                    // the click that asked it happened over on the graph, and
                    // nothing else would draw the eye back here.
                    border.color: detailsPane.asking ? Theme.warning : Theme.borderDefault
                    border.width: Theme.borderWidth
                    ScrollView {
                        id: subjectView
                        anchors.fill: parent
                        anchors.margins: Theme.spaceXs
                        // ScrollView keeps its Flickable private -- reach it
                        // once it exists. Not interactive, for the reason
                        // the description box carries: the flickable
                        // answering the same wheel as the handler moved the
                        // text twice.
                        Component.onCompleted: {
                            contentItem.boundsBehavior = Flickable.StopAtBounds
                            contentItem.interactive = false
                        }
                        SummaryArea {
                            id: subjectArea
                            readOnly: !detailsPane.editable
                            placeholderText: detailsPane.editable ? qsTr("Commit summary") : ""
                            ToolTip.visible: (hovered || detailsPane.summaryPointedAt)
                                             && detailsPane.editBlocked !== ""
                            ToolTip.delay: Metrics.tipDelayMs
                            ToolTip.text: detailsPane.editBlocked
                            WheelHandler {
                                acceptedDevices: PointerDevice.Mouse
                                                 | PointerDevice.TouchPad
                                onWheel: event => detailsPane.rollSummary(
                                    subjectView.contentItem, event.angleDelta.y)
                            }
                        }
                    }
                }
                // Always shown, even empty, and two lines tall from the start —
                // the pair mirrors the commit editor's fields, and this half of
                // the pair is the same component in both panes.
                DescriptionBox {
                    id: bodyArea
                    readOnly: !detailsPane.editable
                    placeholderText: detailsPane.editable ? qsTr("Description") : ""
                    border.color: detailsPane.asking ? Theme.warning : Theme.borderSubtle
                    room: detailsPane.descRoom
                    owed: detailsPane.descOwed
                    onWheelPastEnd: pixels => detailsPane.rollBlock(pixels)
                }
                // Only once something is actually changed: until then the pane
                // keeps its resting shape and nothing invites a rewrite.
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spaceXs
                    visible: detailsPane.messageDirty
                    // The newest commit is amended in place and costs nothing;
                    // an older one is replayed, and everything built on it
                    // comes back as different commits. Only the second case is
                    // worth a line.
                    Label {
                        Layout.fillWidth: true
                        visible: !detailsPane.asking
                                 && detailsPane.details.shaHex !== detailsPane.headOid
                        wrapMode: Text.Wrap
                        text: qsTr("Saving replays this commit, so every commit after it gets a new identity.")
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontSm
                    }
                    // Said, not asked, like the amend editor's tag: the save
                    // still goes ahead, and this line is the warning it gets.
                    Label {
                        Layout.fillWidth: true
                        visible: !detailsPane.asking && detailsPane.published
                        wrapMode: Text.Wrap
                        text: qsTr("This commit is on a remote. Rewriting it leaves anyone who already has it out of step.")
                        color: Theme.warning
                        font.pixelSize: Theme.fontSm
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: detailsPane.asking
                        wrapMode: Text.Wrap
                        text: qsTr("Moving to another commit leaves this text behind.")
                        color: Theme.warning
                        font.pixelSize: Theme.fontSm
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.spaceSm
                        Item { Layout.fillWidth: true }
                        HoverToolButton {
                            visible: !detailsPane.asking
                            text: qsTr("Cancel")
                            font.pixelSize: Theme.fontSm
                            onClicked: detailsPane.revertMessage()
                        }
                        ActionButton {
                            visible: !detailsPane.asking
                            implicitHeight: Theme.controlHeight
                            kind: "check"
                            besideWord: true
                            frameColor: enabled ? Theme.accent : Theme.borderDefault
                            activeFocusOnTab: true
                            text: qsTr("Save message")
                            enabled: !detailsPane.busy && subjectArea.text.trim() !== ""
                            onActivated: detailsPane.submitMessage()
                        }
                        // The two ways out of the question. Staying is the
                        // framed one: it is the answer that loses nothing.
                        HoverToolButton {
                            visible: detailsPane.asking
                            text: qsTr("Discard edits")
                            font.pixelSize: Theme.fontSm
                            onClicked: detailsPane.leaveResolved(true)
                        }
                        ActionButton {
                            visible: detailsPane.asking
                            implicitHeight: Theme.controlHeight
                            kind: "pen"
                            besideWord: true
                            frameColor: Theme.accent
                            activeFocusOnTab: true
                            text: qsTr("Keep editing")
                            onActivated: detailsPane.leaveResolved(false)
                        }
                    }
                }
                // -- author card: avatar + name/date on the left, own hash over
                // parent hash on the right (rows aligned) --
                RowLayout {
                    id: authorRow
                    spacing: Theme.spaceSm
                    // The one place a picture is reached from. A pointer resting on
                    // the face raises a badge saying so; pressing it opens the
                    // settings card with this author already named.
                    Item {
                        id: avatarBox
                        width: Metrics.detailsAvatar
                        height: Metrics.detailsAvatar
                        Layout.preferredWidth: Metrics.detailsAvatar
                        Layout.preferredHeight: Metrics.detailsAvatar
                        /// Nothing to assign a picture to on a row with no author:
                        /// the working tree's own row, and a commit not read yet.
                        readonly property bool editable:
                            detailsPane.details.authorEmail !== ""
                        readonly property bool showBadge:
                            avatarBox.editable
                            && (avatarArea.containsMouse || detailsPane.avatarPointedAt)
                        IdentIcon {
                            anchors.fill: parent
                            code: detailsPane.details.avatar
                            imageUrl: detailsPane.details.avatarUrl
                        }
                        // Pushed as far into the lower-right as the icon's own
                        // square allows — flush with its right and bottom edges,
                        // so the least of the face is covered and the layout
                        // beside it never moves (デザイン規約 §アバターを与える).
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
                            onClicked: detailsPane.avatarClicked()
                        }
                        ToolTip.visible: avatarBox.showBadge
                        ToolTip.delay: Metrics.tipDelayMs
                        // The one word the whole feature goes by (デザイン規約
                        // §アバターを与える). The article is what splits the
                        // two states, not a second noun: the one being
                        // changed is the face under the pointer, the one
                        // being chosen does not exist yet (§長さ).
                        ToolTip.text: detailsPane.details.avatarUrl !== ""
                                      ? qsTr("Change the avatar for %1")
                                        .arg(detailsPane.details.authorEmail)
                                      : qsTr("Choose avatar for %1")
                                        .arg(detailsPane.details.authorEmail)
                    }
                    ColumnLayout {
                        spacing: 0
                        Layout.fillWidth: true
                        // Name, and beside it what git makes of the signature.
                        // An unsigned commit gets nothing: the ordinary case
                        // carries no mark, the same rule the graph's state
                        // badges follow. The verdict rides up here rather than
                        // sitting with the date because the date's row is where
                        // the co-authors go, and a signed commit with one would
                        // have had three things in ~230px (デザイン規約 §co-author).
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceXs
                            Label {
                                id: authorLabel
                                text: detailsPane.details.authorName
                                elide: Text.ElideRight
                                // Grows no further than the name itself, so the
                                // mark sits against the name rather than being
                                // pushed across to the hash — and shrinks, with
                                // the name eliding, when a long one would
                                // otherwise crowd the mark out.
                                Layout.fillWidth: true
                                Layout.maximumWidth: authorLabel.implicitWidth
                                color: Theme.textPrimary
                                font.pixelSize: Theme.fontMd
                                font.weight: Font.DemiBold
                                // Drawn at rest one step down from the name it
                                // underlines, and up to the name's own value
                                // under the pointer — the same rule the credit
                                // line carries, because it says the same thing:
                                // there is more here, and hovering opens it
                                // (規約 §co-author の表示).
                                Rectangle {
                                    visible: authorLabel.text !== ""
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.bottom: parent.bottom
                                    height: Theme.borderWidth
                                    color: detailsPane.authorLit ? Theme.textPrimary
                                                                 : Theme.borderStrong
                                }
                                // A handler, not a `MouseArea`: handlers are
                                // passive, so the card it opens keeps its own
                                // hover (規約 §hover のツールチップ).
                                HoverHandler {
                                    id: authorHover
                                    onHoveredChanged:
                                        detailsPane.showAuthor(authorHover.hovered)
                                }
                            }
                            // A signature that holds is a tick and nothing more;
                            // only one that contradicts the content spends words
                            // (規約 §署名の表示). The broken case reads as the
                            // error message it is, and being the one wide thing
                            // on the row is how an error should read.
                            //
                            // Green stays with the signatures git actually
                            // vouched for. One it could read but not judge gets
                            // the same tick in textSecondary: the shape says a
                            // signature is there, the colour says nobody here
                            // checked it.
                            RowLayout {
                                id: signatureMark
                                visible: detailsPane.signatureKind !== ""
                                spacing: Theme.spaceXs
                                Layout.alignment: Qt.AlignVCenter
                                readonly property bool broken:
                                    detailsPane.signatureKind === "bad"
                                readonly property color tone:
                                    detailsPane.signatureKind === "verified" ? Theme.success
                                    : signatureMark.broken ? Theme.danger
                                    : Theme.textSecondary
                                NavIcon {
                                    kind: signatureMark.broken ? "bang" : "check"
                                    tint: signatureMark.tone
                                    width: Theme.iconSm
                                    height: Theme.iconSm
                                    Layout.alignment: Qt.AlignVCenter
                                }
                                Label {
                                    visible: signatureMark.broken
                                    text: qsTr("Bad signature")
                                    color: signatureMark.tone
                                    font.pixelSize: Theme.fontSm
                                    Layout.alignment: Qt.AlignVCenter
                                }
                                ToolTip.visible: signatureHover.hovered
                                                 || detailsPane.signaturePointedAt
                                ToolTip.delay: Metrics.tipDelayMs
                                ToolTip.text: detailsPane.signatureTip
                                // A handler, not a MouseArea: an item inside a
                                // layout is sized by the layout, and anchoring
                                // one to fill its parent is undefined behaviour.
                                HoverHandler { id: signatureHover }
                            }
                            // The slack lives here, past both of them, so the
                            // mark stays against the name.
                            Item { Layout.fillWidth: true }
                        }
                        // Date, and beside it whoever the message credits along
                        // with the author. A commit with no trailer shows only
                        // the date, the way it always did.
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: Theme.spaceSm
                            Label {
                                id: detailsDate
                                text: Qt.formatDateTime(new Date(detailsPane.details.authorTime * 1000),
                                                        "yyyy-MM-dd HH:mm")
                                color: Theme.textSecondary
                                font.pixelSize: Theme.fontSm
                            }
                            // The face and name of the first co-author, then a
                            // count of the rest — the same "+N" the graph chips
                            // use, so the row's width never moves. One rule runs
                            // under the lot, the way the hash and its copy icon
                            // share one: the two are one target.
                            CoAuthorLine {
                                id: coBlock
                                packed: detailsPane.details.coAuthors
                                lit: detailsPane.matesLit
                                // Half the pane, the share the hover card gives
                                // the same line out of the graph pane. The date
                                // holds the left of this row; this is the rest.
                                nameWidth: detailsPane.width / 2
                                // Grows no further than the names themselves, and
                                // gives way when the row cannot hold them -- the
                                // rule the author's name above already follows.
                                // Without the pair this line is Fixed, and a
                                // Fixed item is a floor the layout cannot go
                                // under: the row then lays out at its own width
                                // and every box in the pane, sized to fill it,
                                // paints past the window's edge (measured at 483
                                // against a 384px pane).
                                Layout.fillWidth: true
                                Layout.maximumWidth: coBlock.implicitWidth
                                Layout.alignment: Qt.AlignVCenter
                                onPointerChanged: inside => detailsPane.showCoAuthors(inside)
                            }
                            Item { Layout.fillWidth: true }
                        }
                    }
                    ColumnLayout {
                        spacing: 0
                        Layout.alignment: Qt.AlignRight
                        // The hash is the button, not just the icon beside it —
                        // a 16px glyph was too small to aim at. Hovering
                        // underlines the hash and lights the icon so the whole
                        // plate reads as one control.
                        // Not a HoverToolButton: the style's panel would make
                        // the plate taller than one line and drop this hash out
                        // of step with the author name beside it, so it draws
                        // the same wash over its own flat face.
                        ToolButton {
                            id: hashCopy
                            Layout.alignment: Qt.AlignRight
                            hoverEnabled: true
                            text: detailsPane.details.sha8
                            leftPadding: Theme.spaceXs
                            rightPadding: Theme.spaceXs
                            topPadding: 0
                            bottomPadding: 0
                            readonly property bool lit: hovered || visualFocus
                            ToolTip.visible: hovered
                            ToolTip.delay: Metrics.tipDelayMs
                            ToolTip.text: qsTr("Copy full hash")
                            onClicked: detailsPane.copyRequested(detailsPane.details.shaHex)
                            background: Rectangle {
                                radius: Theme.radiusSm
                                color: hashCopy.down ? Theme.bgPressed
                                     : hashCopy.lit ? Theme.bgHover
                                     : "transparent"
                                MouseArea {
                                    anchors.fill: parent
                                    acceptedButtons: Qt.NoButton
                                    cursorShape: Qt.PointingHandCursor
                                }
                                // Drawn here rather than as the label's font
                                // underline so the rule runs under the icon too —
                                // the hash and the icon are one target, so they
                                // get one line.
                                Rectangle {
                                    visible: hashCopy.lit
                                    color: Theme.textPrimary
                                    height: Theme.borderWidth
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.bottom: parent.bottom
                                    anchors.leftMargin: hashCopy.leftPadding
                                    anchors.rightMargin: hashCopy.rightPadding
                                }
                            }
                            contentItem: RowLayout {
                                spacing: Theme.spaceXs
                                Label {
                                    text: hashCopy.text
                                    font.family: Theme.monoFamily
                                    font.pixelSize: Theme.fontMd
                                    color: Theme.textPrimary
                                    Layout.alignment: Qt.AlignVCenter
                                }
                                NavIcon {
                                    kind: "copyicon"
                                    tint: hashCopy.lit ? Theme.textPrimary
                                                       : Theme.textSecondary
                                    Layout.alignment: Qt.AlignVCenter
                                }
                            }
                        }
                        Label {
                            id: parentLink
                            visible: detailsPane.details.parentHex !== ""
                            Layout.alignment: Qt.AlignRight
                            text: detailsPane.details.parentHex.substring(0, 8)
                            font.family: Theme.monoFamily
                            color: Theme.textLink
                            font.pixelSize: Theme.fontSm
                            // The mark is drawn, not typed. The fonts disagree
                            // about `←`: Cascadia Mono holds it in one cell
                            // (7px of ink) where Noto Sans Mono CJK JP gives it
                            // a full-width one (12px), so Ubuntu grew a tail
                            // nobody chose — the same way `⚑` came out a
                            // different shape on each of the three.
                            //
                            // The seat is the mark's ink, not its box, so the
                            // `spaceXs` lands where the eye measures it — the
                            // same gap the plate above spends between its hash
                            // and copy icon (規約 §余白「印が自分で持っている
                            // 余白は、隣の詰めに数える」).
                            leftPadding: parentBack.inkWidth + Theme.spaceXs
                            ToolTip.visible: parentHover.containsMouse
                            ToolTip.delay: Metrics.tipDelayMs
                            ToolTip.text: qsTr("Go to parent commit")
                            NavIcon {
                                id: parentBack
                                kind: "arrow"
                                // The family draws it leaving; this one points
                                // back, and turning the mark is how `FoldBlock`
                                // faces its chevrons too.
                                rotation: 180
                                tint: Theme.textLink
                                width: Theme.iconSm
                                height: Theme.iconSm
                                // The grid shrinks and the line shrinks with it,
                                // or the mark carries more weight than the
                                // digits beside it (§語の隣に立つ印).
                                stroke: Metrics.iconStroke * Theme.iconSm / Theme.iconMd
                                // Hung off the left by the air it keeps inside
                                // its box, so the ink starts where the link does
                                // and the line under the pair starts with it.
                                x: -(parentBack.width - parentBack.inkWidth) / 2
                                anchors.verticalCenter: parent.verticalCenter
                            }
                            // One line under the mark and the hash: they are one
                            // target, the way the hash and its copy icon share
                            // theirs. `font.underline` cannot reach the mark
                            // now that the mark is not a letter.
                            Rectangle {
                                visible: parentHover.containsMouse
                                color: Theme.textLink
                                height: Theme.borderWidth
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.bottom: parent.bottom
                            }
                            MouseArea {
                                id: parentHover
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: detailsPane.parentClicked(detailsPane.details.parentHex)
                            }
                        }
                    }
                }
            }
        }
    }
    // CHANGES header with the tree ⇄ path view toggle. Outside the block
    // above: it is the list's own band, and a list whose heading has
    // scrolled away is a list of nothing in particular.
    Rectangle {
        id: changesBand
        visible: detailsPane.details.shaHex !== ""
        Layout.fillWidth: true
        implicitHeight: Theme.headerHeight
        color: Theme.bgElevated
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.spaceSm
            anchors.rightMargin: Theme.spaceXs
            spacing: Theme.spaceXs
            Label {
                text: qsTr("CHANGES (%1)").arg(detailsPane.details.fileTotal)
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
                color: Theme.textSecondary
            }
            Item { Layout.fillWidth: true }
            HoverToolButton {
                padding: 0
                implicitWidth: Theme.iconLg
                implicitHeight: Theme.iconLg
                ToolTip.visible: hovered
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: qsTr("Tree view")
                onClicked: detailsPane.details.setTreeView(true)
                contentItem: NavIcon {
                    kind: "hier"
                    // The one not in use is still the way to switch, so it
                    // keeps the hue and drops a step rather than falling to
                    // the muted colour, which would read as unavailable
                    // (§暗く落とした段 / §無効).
                    tint: detailsPane.details.treeView ? Theme.accent
                                                       : Theme.accentDim
                }
            }
            HoverToolButton {
                padding: 0
                implicitWidth: Theme.iconLg
                implicitHeight: Theme.iconLg
                ToolTip.visible: hovered
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: qsTr("Paths view")
                onClicked: detailsPane.details.setTreeView(false)
                contentItem: NavIcon {
                    kind: "list"
                    tint: detailsPane.details.treeView ? Theme.accentDim
                                                       : Theme.accent
                }
            }
        }
    }
    ListView {
        id: fileList
        Layout.fillWidth: true
        Layout.fillHeight: true
        clip: true
        model: detailsPane.details
        reuseItems: true
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: AutoScrollBar {}
        delegate: FileRowDelegate {
            listWidth: fileList.width
            pointedTipRow: detailsPane.pointedTipRow
            onActivated: (bucket, path, origPath) =>
                detailsPane.fileActivated(path, origPath)
            onFolderToggled: key => detailsPane.details.toggleFolder(key)
        }
    }
}
