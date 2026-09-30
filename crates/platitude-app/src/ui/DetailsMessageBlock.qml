pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Everything between the two bands, scrolling when the pane is too short: past a file list at zero nothing else
// gives, and the rest would run out of the pane's bottom (規約 §窓の床). The root is that surface
// (rules-refs/structure.md).
Flickable {
    id: block

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
    /// The commit is a stash entry, whose parents past the first are git's bookkeeping (`CommitAuthorRow.stashed`).
    required property bool stashed
    required property bool signsCommits
    required property string signingTip
    /// Pointer stand-ins, and whether the pane's cards have the pointer (the cards open in pane coordinates).
    required property bool avatarPointedAt
    required property bool signaturePointedAt
    required property bool summaryPointedAt
    required property bool mateCardInside
    required property bool authorCardInside
    required property bool parentCardInside
    required property bool parentCardOpen

    signal avatarClicked()
    signal copyRequested(string text)
    signal parentClicked(string oidHex)
    signal openMateRequested(point at)
    signal settleMateRequested()
    signal openParentsRequested(rect line)
    signal settleParentsRequested()
    signal openAuthorRequested(point at)
    signal settleAuthorRequested()
    signal escaped()
    signal saveRequested()

    /// What the block would take with nothing in its way — the pane holds it against `blockRoom`.
    readonly property real wants: blockCol.implicitHeight

    // -- what the pane reads back out of here --
    /// The row that draws the commit's values, handed on whole rather than relayed piece by piece.
    readonly property alias valueRow: authorRow
    readonly property alias coAuthorRecords: authorRow.coAuthorRecords
    readonly property alias matesClipped: authorRow.matesClipped
    readonly property alias matesFolded: authorRow.matesFolded
    readonly property alias matesSaid: authorRow.matesSaid
    readonly property alias nameClipped: authorRow.nameClipped
    readonly property alias matesPointed: authorRow.matesPointed
    readonly property alias authorPointed: authorRow.authorPointed
    readonly property alias parentsPointed: authorRow.parentsPointed
    readonly property alias signatureTipShown: authorRow.signatureTipShown
    readonly property alias subjectText: msgEditor.subjectText
    readonly property alias bodyText: msgEditor.bodyText
    readonly property alias summaryTipShown: msgEditor.summaryTipShown
    readonly property alias descriptionColor: msgEditor.descriptionColor
    readonly property alias descriptionFocused: msgEditor.descriptionFocused
    readonly property alias descriptionBarInk: msgEditor.descriptionBarInk
    readonly property alias descriptionAt: msgEditor.descriptionAt
    readonly property alias summaryAt: msgEditor.summaryAt
    readonly property alias descRefuses: msgEditor.descRefuses
    readonly property alias descPoint: msgEditor.descPoint
    readonly property alias descGrips: msgEditor.descGrips
    readonly property alias descHeight: msgEditor.descHeight
    readonly property alias descWants: msgEditor.descWants
    readonly property alias descCap: msgEditor.descCap
    readonly property alias descListRows: msgEditor.descListRows
    readonly property alias blockScrolls: msgEditor.blockScrolls
    readonly property alias descKeeps: msgEditor.descKeeps
    /// Automation only: the block's middle-button hand and the boxes' two (a middle button cannot be injected).
    readonly property alias hand: hand
    readonly property alias summaryHand: msgEditor.summaryHand
    readonly property alias descriptionHand: msgEditor.descriptionHand

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
    /// Moves the block by a wheel a box on it could not use (`DescriptionBox.wheelPastEnd`).
    function rollBlock(pixels) {
        const max = Math.max(0, block.contentHeight - block.height)
        // Taken away: content travels against `contentY`.
        block.contentY = Math.max(0, Math.min(max, block.contentY - pixels))
    }

    contentWidth: width
    contentHeight: blockCol.implicitHeight
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    ScrollBar.vertical: PaneScrollBar {}
    // The block's middle-button hand, on the view's frame rather than in its content. A box with a hand of its own
    // takes the press on it (`MiddleAutoScroll.claimedAt`).
    MiddleAutoScroll {
        id: hand
        parent: block
        anchors.fill: parent
        visible: block.ScrollBar.vertical.visible
        // `rollBlock` takes the wheel's sense.
        onDrifted: dy => block.rollBlock(-dy)
    }
    ColumnLayout {
        id: blockCol
        width: block.width
        spacing: 0

        // One step on every side (デザイン規約 §余白) but the right: that is the gutter the block's bar is drawn in,
        // and anything short of it draws the bar over the boxes' frame and the parent hash.
        ColumnLayout {
            Layout.fillWidth: true
            Layout.margins: Theme.spaceXs
            Layout.rightMargin: Theme.navBarGutter
            spacing: Theme.spaceXs
            visible: block.details.shaHex !== ""

            CommitAuthorRow {
                id: authorRow
                // An `Item` does not fill by default; without this the hash plate parks against the name.
                Layout.fillWidth: true
                details: block.details
                signatureKind: block.signatureKind
                signatureCode: block.signatureCode
                signatureSigner: block.signatureSigner
                avatarPointedAt: block.avatarPointedAt
                signaturePointedAt: block.signaturePointedAt
                paneWidth: block.paneWidth
                stashed: block.stashed
                mateCardInside: block.mateCardInside
                authorCardInside: block.authorCardInside
                parentCardInside: block.parentCardInside
                parentsUnrolled: block.parentCardOpen
                onAvatarClicked: block.avatarClicked()
                onCopyRequested: text => block.copyRequested(text)
                onParentClicked: oidHex => block.parentClicked(oidHex)
                onOpenMateRequested: at => block.openMateRequested(at)
                onSettleMateRequested: block.settleMateRequested()
                onOpenParentsRequested: line => block.openParentsRequested(line)
                onSettleParentsRequested: block.settleParentsRequested()
                onOpenAuthorRequested: at => block.openAuthorRequested(at)
                onSettleAuthorRequested: block.settleAuthorRequested()
            }
            // The same pair the commit editor writes in (デザイン規約 §コミットメッセージの 2 つの枠).
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
                // Escape drops the draft and puts the resting text back (the commit's own message, or the reword a
                // plan holds for it), unasked (デザイン規約 §コミットメッセージの 2 つの枠).
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
