pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Everything between the two bands, in a surface of its own that scrolls
// when the pane is too short to hold it: the boxes, the author row and
// the save row keep their heights by construction, so the file list was
// the only thing that could give and past zero the rest ran out of the
// pane's bottom (規約 §窓の床).
//
// The root is the `Flickable` it was inside the pane, so the layout attachments stay where they were and the item
// tree is the same depth it was (rules-refs/structure.md).
Flickable {
    id: block

    /// What the pane hands down: the commit being read, the room it is given, and the values the row and the
    /// save line wear.
    required property var details
    required property real blockRoom
    required property real listHeight
    required property real paneWidth
    required property bool editable
    required property string editBlocked
    required property bool busy
    required property bool published
    required property bool intoPlan
    required property bool messageDirty
    required property string signatureKind
    required property string signatureCode
    required property string signatureSigner
    required property int committerFace
    required property string committerFaceUrl
    required property bool signsCommits
    required property string signingTip
    /// The pointer stand-ins, and whether a card the pane holds has the hand — the cards open in pane
    /// coordinates, so they cannot live in here.
    required property bool avatarPointedAt
    required property bool signaturePointedAt
    required property bool summaryPointedAt
    required property bool mateCardInside
    required property bool authorCardInside

    /// Back up to the pane: what the row raised, the two cards asking to be placed or settled, the escape out of
    /// the boxes, and the press on the save line.
    signal avatarClicked()
    signal copyRequested(string text)
    signal parentClicked(string oidHex)
    signal openMateRequested(point at)
    signal settleMateRequested()
    signal openAuthorRequested(point at)
    signal settleAuthorRequested()
    signal escaped()
    signal saveRequested()

    /// What the block would take with nothing in its way — the pane holds it against `blockRoom`.
    readonly property real wants: blockCol.implicitHeight

    // -- what the pane reads back out of here --
    /// The row that draws the commit's values, handed on whole: a dozen one-line relays would say nothing this
    /// does not.
    readonly property alias valueRow: authorRow
    readonly property alias coAuthorRecords: authorRow.coAuthorRecords
    readonly property alias matesClipped: authorRow.matesClipped
    readonly property alias nameClipped: authorRow.nameClipped
    readonly property alias matesPointed: authorRow.matesPointed
    readonly property alias authorPointed: authorRow.authorPointed
    readonly property alias signatureTipShown: authorRow.signatureTipShown
    readonly property alias subjectText: msgEditor.subjectText
    readonly property alias bodyText: msgEditor.bodyText
    readonly property alias summaryTipShown: msgEditor.summaryTipShown
    readonly property alias descriptionColor: msgEditor.descriptionColor
    readonly property alias descriptionFocused: msgEditor.descriptionFocused
    readonly property alias descriptionBarInk: msgEditor.descriptionBarInk
    readonly property alias descriptionAt: msgEditor.descriptionAt
    readonly property alias descRefuses: msgEditor.descRefuses
    readonly property alias descPoint: msgEditor.descPoint
    readonly property alias descGrips: msgEditor.descGrips
    readonly property alias descHeight: msgEditor.descHeight
    readonly property alias descWants: msgEditor.descWants
    readonly property alias descCap: msgEditor.descCap
    readonly property alias descListRows: msgEditor.descListRows
    readonly property alias blockScrolls: msgEditor.blockScrolls
    readonly property alias descKeeps: msgEditor.descKeeps

    // -- and what it does on the pane's word --
    function coAuthorName(i) { return authorRow.coAuthorName(i) }
    function showCoAuthors(on) { authorRow.showCoAuthors(on) }
    function showAuthor(on) { authorRow.showAuthor(on) }
    function setMessage(subject, body) { msgEditor.setMessage(subject, body) }
    function setTexts(subject, body) { msgEditor.setTexts(subject, body) }
    function callAttention() { msgEditor.callAttention() }
    function dropAttention() { msgEditor.dropAttention() }
    readonly property alias attention: msgEditor.attention
    function focusDescription() { msgEditor.focusDescription() }
    function rollDescription(dy) { msgEditor.rollDescription(dy) }
    function holdDescriptionBar(on) { msgEditor.holdDescriptionBar(on) }
    function growDescription(dy) { msgEditor.growDescription(dy) }
    function pullDescriptionPast(down) { msgEditor.pullDescriptionPast(down) }
    /// Moves the block by a wheel a box on it could not use: the boxes cover
    /// most of the block, so a box that keeps the wheel at its own end
    /// leaves the block unreachable by wheel.
    function rollBlock(pixels) {
        const max = Math.max(0, block.contentHeight - block.height)
        // Taken away, not added: content travels against `contentY`.
        block.contentY = Math.max(0, Math.min(max, block.contentY - pixels))
    }

    contentWidth: width
    contentHeight: blockCol.implicitHeight
    clip: true
    // Hard stop at the ends, as everywhere else that scrolls (デザイン規約 §QML 実装ルール).
    boundsBehavior: Flickable.StopAtBounds
    // What moves here is the block; the words inside the message boxes
    // move under a bar of their own, and that one is the style's.
    ScrollBar.vertical: PaneScrollBar {}
    ColumnLayout {
        id: blockCol
        width: block.width
        spacing: 0

        // Inset on all four sides, one step each — the message box carries
        // its own frame, and flush against the header band the two borders
        // read as one welded block, so band → summary → description →
        // author → band is one even rhythm (デザイン規約 §余白). The right
        // is the exception and is not padding: it is the gutter this
        // block's scroll bar is drawn in, and anything short of it draws
        // the bar over the boxes' frame and the parent hash's tail.
        ColumnLayout {
            Layout.fillWidth: true
            Layout.margins: Theme.spaceXs
            Layout.rightMargin: Theme.navBarGutter
            spacing: Theme.spaceXs
            visible: block.details.shaHex !== ""

            // Who wrote it, first: avatar + name/date on the left, own
            // hash over parent hash on the right (rows aligned). The pane
            // reads top to bottom the way the commit itself does — whose
            // it is, what it says, what it touched.
            CommitAuthorRow {
                id: authorRow
                // Said out loud: this row is an `Item`, and a plain item takes its own width and stops, which parks
                // the hash plate against the end of the name.
                Layout.fillWidth: true
                details: block.details
                signatureKind: block.signatureKind
                signatureCode: block.signatureCode
                signatureSigner: block.signatureSigner
                avatarPointedAt: block.avatarPointedAt
                signaturePointedAt: block.signaturePointedAt
                paneWidth: block.paneWidth
                mateCardInside: block.mateCardInside
                authorCardInside: block.authorCardInside
                onAvatarClicked: block.avatarClicked()
                onCopyRequested: text => block.copyRequested(text)
                onParentClicked: oidHex => block.parentClicked(oidHex)
                onOpenMateRequested: at => block.openMateRequested(at)
                onSettleMateRequested: block.settleMateRequested()
                onOpenAuthorRequested: at => block.openAuthorRequested(at)
                onSettleAuthorRequested: block.settleAuthorRequested()
            }
            // Then the message: the same pair, in the same component the
            // commit editor writes in, one block of one height
            // (デザイン規約 §コミットメッセージの 2 つの枠).
            MessageEditor {
                id: msgEditor
                Layout.fillWidth: true
                Layout.preferredHeight: msgEditor.pairHeight
                readOnly: !block.editable
                blockedTip: block.editBlocked
                summaryPointedAt: block.summaryPointedAt
                listHeight: block.listHeight
                blockRoom: block.blockRoom
                blockHeight: blockCol.implicitHeight
                onWheelPastEnd: pixels => block.rollBlock(pixels)
                // Escape drops the draft and puts the commit's own message back. Nothing asks: the reader said so
                // (デザイン規約 §コミットメッセージの 2 つの枠).
                onEscaped: block.escaped()
            }
            // Only once something is actually changed (`MessageActionsRow`).
            MessageActionsRow {
                Layout.fillWidth: true
                dirty: block.messageDirty
                editing: block.editable && msgEditor.anyFocused
                intoPlan: block.intoPlan
                published: block.published
                busy: block.busy
                canSave: msgEditor.subjectText.trim() !== ""
                committerFace: block.committerFace
                committerFaceUrl: block.committerFaceUrl
                signature: block.signsCommits ? "signed" : ""
                signatureTip: block.signingTip
                onSaveRequested: block.saveRequested()
            }
        }
    }
}
