pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Right pane, commit-details mode.
ColumnLayout {
    id: detailsPane

    required property var details
    /// The commits a choice holds (`GraphModel.chosenRows` — one record per row: `oid`, `sha8`, `subject`, `body`,
    /// `author`, `atime`, `avatar`, `avatarUrl`, `mates`); empty while one commit is being read
    /// (デザイン規約 §複数のコミットを選ぶ).
    property var chosenCommits: []
    /// The commit whose card is out, so its row keeps its band (the page's: `RowHoverHost.rowCardOid`).
    property string rowCardOid: ""
    /// A row of the commit list was rested on, or left. The page answers: one card of its own keeps the two lists from
    /// opening two.
    signal rowHoverRequested(var row, bool inside)
    /// A held press on a listed commit drops it from the choice, which is the page's.
    signal commitDropRequested(string oidHex)
    /// The height the two lists divide while a choice is up.
    readonly property real listRoom: Math.max(0, detailsPane.height - 2 * Theme.headerHeight)
    /// Read off the model: the file list below answers to it, and the two must turn over together.
    readonly property bool choosing: detailsPane.details.selectionCount > 1
    // Reflog selector when the selected row is a stash ("" otherwise).
    property string stashRef: ""
    /// Whether the commit on show is a stash entry (`CommitAuthorRow.stashed`). Taken as a commit's details land, not
    /// read off `stashRef`: the selection moves at the click and the details a beat later, and between the two the
    /// commit still on show would have its parents counted by the next one's rule.
    property bool stashShown: false
    /// The commit `stashShown` was taken for.
    property string stashJudged: ""
    function judgeStash() {
        if (detailsPane.details.shaHex === detailsPane.stashJudged)
            return
        detailsPane.stashJudged = detailsPane.details.shaHex
        detailsPane.stashShown = detailsPane.stashRef !== ""
    }
    // The page decides (`RepoPage.messageEdit` → `offers::message_edit`).
    property bool editable: false
    // A write is already running, so nothing new starts.
    property bool busy: false
    // Why the boxes are read-only, in one line ("" when they are not).
    property string editBlocked: ""
    // A remote already has this commit: the save row says so (デザイン規約 §長押し「ローカルの履歴書き換えはクリック」).
    property bool published: false
    // What git makes of the signature: "" (unsigned, or not answered yet), "verified", "signed" or "bad".
    // `signatureCode` is git's `%G?`, for the tooltip: an unjudged signature has one of five reasons.
    property string signatureKind: ""
    property string signatureCode: ""
    property string signatureSigner: ""
    /// The reader's own face: a rewrite keeps the author and replaces the committer.
    property int committerFace: -1
    property string committerFaceUrl: ""
    /// The boxes are a rebase plan's `reword` input right now — the save row wears that verb,
    /// since nothing runs on the press (`RepoPage.planReword`).
    property bool intoPlan: false
    /// The plan's stored reword for one commit, so a revisited row opens on its draft. Empty oid while no plan holds
    /// one.
    property string planDraftOid: ""
    property string planDraftSubject: ""
    property string planDraftBody: ""
    property bool signsCommits: false
    property string signingTip: ""
    /// The badge was pressed: the settings card opens knowing whom it is about.
    signal avatarEditRequested(string name, string email)
    /// `*PointedAt` stand in for the pointer headless cannot put (avatar-hover / signature-tip / stash-tip /
    /// path-tip); what each shows is read back off the ToolTip's own `visible`, so a cut binding cannot read green.
    /// `pointedTipRow` -1 points at no row.
    property bool avatarPointedAt: false
    property bool signaturePointedAt: false
    readonly property bool signatureTipShown: block.signatureTipShown
    property bool summaryPointedAt: false
    readonly property bool summaryTipShown: block.summaryTipShown
    property int pointedTipRow: -1
    /// A right-click menu of the page's stands over this pane (デザイン規約 §メニュー).
    property bool menuStanding: false
    /// Which changed file the middle pane is reading, so its row can say so (デザイン規約 §diff のファイル一覧).
    /// Empty while the graph is in the middle.
    property string readPath: ""
    function avatarClicked() {
        if (detailsPane.details.authorEmail !== "")
            detailsPane.avatarEditRequested(detailsPane.details.authorName, detailsPane.details.authorEmail)
    }
    /// Automation-only exposures, like `GraphPane.view` (rules-refs/app-ui.md; `AutoActTipVerbs`).
    readonly property alias authorCards: cards
    readonly property alias messageBlock: block

    DetailsAuthorCards {
        id: cards
        host: detailsPane
        block: block
        details: detailsPane.details
        onParentPicked: oidHex => detailsPane.parentClicked(oidHex)
    }

    signal fileActivated(string path, string origPath)
    /// The arrows walked onto another file. Not `fileActivated`: a click on the file already open closes the diff,
    /// and holding Down walks on (規約 §diff のファイル一覧).
    signal fileWalked(string path, string origPath)
    signal parentClicked(string oidHex)
    signal copyRequested(string text)
    /// Automation only: the row that draws the commit's values (verify-ui).
    readonly property alias valueRow: block.valueRow
    signal applyStashRequested(string selector)
    signal popStashRequested(string selector)
    signal messageSubmitted(string oidHex, string subject, string body)

    // ---- message editor state: the boxes are filled by hand — typing would break a binding for good, and the next
    // commit would land in a box that no longer listens.
    property string baseOid: ""
    property string baseSubject: ""
    property string baseBody: ""
    /// Apart from `messageDirty`: a row leaving `reword` locks the boxes with the typed text still in them —
    /// unsavable, and what a closing plan takes.
    readonly property bool boxMoved: block.subjectText !== detailsPane.baseSubject
                                     || block.bodyText !== detailsPane.baseBody
    readonly property bool messageDirty: detailsPane.editable && detailsPane.boxMoved
    /// A plan holds a typed reword for the commit on screen, so the boxes show it: the commit's own message would
    /// offer to save the original back over the draft.
    readonly property bool planHoldsDraft: detailsPane.details.shaHex !== ""
        && detailsPane.details.shaHex === detailsPane.planDraftOid
        && (detailsPane.planDraftSubject !== "" || detailsPane.planDraftBody !== "")

    /// Only on a commit change: a different message is a different commit.
    function syncMessage() {
        if (detailsPane.details.shaHex === detailsPane.baseOid)
            return
        detailsPane.fillMessage(detailsPane.planHoldsDraft)
    }
    /// The resting text a save is measured against: the plan's draft where `drafted` (already saved into the plan),
    /// the commit's message otherwise. The editor is written only when the text moves: the same text assigned back
    /// still sends the caret to the top (`MessageEditor.setMessage`), and a plan taking a typed reword lands here with
    /// the caret in the box.
    function fillMessage(drafted) {
        detailsPane.baseOid = detailsPane.details.shaHex
        detailsPane.baseSubject = drafted ? detailsPane.planDraftSubject
                                          : detailsPane.details.messageSubject
        detailsPane.baseBody = drafted ? detailsPane.planDraftBody : detailsPane.details.messageBody
        if (block.subjectText !== detailsPane.baseSubject
                || block.bodyText !== detailsPane.baseBody)
            block.setMessage(detailsPane.baseSubject, detailsPane.baseBody)
        // Last: the write above reaches `noteTyping` like any keystroke, so a fill made while a plan stands would
        // otherwise leave the text marked as typed under it.
        detailsPane.boxFromPlan = drafted
    }
    /// The boxes' text was written while the plan routed them (that row's `reword` input), not into the plain amend.
    property bool boxFromPlan: false
    /// Latched: by the time a closing plan asks, the state the text was written in is gone.
    function noteTyping() {
        if (detailsPane.intoPlan)
            detailsPane.boxFromPlan = true
    }
    Connections {
        target: block
        function onSubjectTextChanged() { detailsPane.noteTyping() }
        function onBodyTextChanged() { detailsPane.noteTyping() }
    }
    /// The plan's hold moved without the commit moving, which `syncMessage`'s guard misses (a row left `reword` —
    /// `RebasePlanModel::set_action` — or the draft for the row up arrived). A dropped draft left resting reads as
    /// unchanged, and the next `reword` would refuse with nothing on screen to say why.
    onPlanHoldsDraftChanged: detailsPane.fillMessage(detailsPane.planHoldsDraft)
    /// The plan is gone: a reword left in the boxes would stand under the plain amend as a one-press rewrite
    /// (デザイン規約 §コミットメッセージの 2 つの枠). Only what was written under the plan — an amend half-written
    /// before it is the reader's own, and nothing on screen could give it back.
    function dropDraft() {
        if (!detailsPane.boxFromPlan)
            return
        detailsPane.fillMessage(false)
    }
    function revertMessage() {
        block.setTexts(detailsPane.baseSubject, detailsPane.baseBody)
    }
    /// git took the new message: what was written becomes the resting text, since the model still holds the old one
    /// until the selection follows.
    function noteMessageSaved() {
        detailsPane.baseSubject = block.subjectText
        detailsPane.baseBody = block.bodyText
    }
    function submitMessage() {
        // Silent: the button stands from the moment someone is writing (`MessageActionsRow.editing`), so a press with
        // nothing changed is ordinary.
        if (!detailsPane.editable || !detailsPane.messageDirty || block.subjectText.trim() === "")
            return
        detailsPane.messageSubmitted(detailsPane.details.shaHex, block.subjectText, block.bodyText)
    }
    /// Smoke hook: type the way a keystroke would — not at all while read-only.
    function setMessageText(subject, body) {
        if (!detailsPane.editable)
            return
        block.setTexts(subject, body)
    }
    /// Smoke hook.
    readonly property alias boxSubject: block.subjectText

    /// A reader was sent here from somewhere that could not hold the whole message (the graph row's hover card): the
    /// boxes mark themselves until the reader's next press (デザイン規約 §hover のツールチップ; `RepoPage.notePress`).
    function callAttention() {
        block.callAttention()
    }
    function dropAttention() {
        block.dropAttention()
    }
    readonly property alias attention: block.attention

    /// Smoke hook: the caret into the description box, as a click puts it.
    function focusDescription() {
        block.focusDescription()
    }
    /// What the box paints — the input side (activeFocus) would read green with the binding cut.
    readonly property color descriptionColor: block.descriptionColor
    readonly property bool descriptionFocused: block.descriptionFocused
    /// Smoke hooks: the wheel over the description box, and the ink its bar carries after
    /// (デザイン規約 §QML 実装ルール のバーの明るさ).
    function rollDescription(dy) { block.rollDescription(dy) }
    function holdDescriptionBar(on) { block.holdDescriptionBar(on) }
    readonly property real descriptionBarInk: block.descriptionBarInk
    readonly property real descriptionAt: block.descriptionAt

    // -- what this pane lends the message editor --
    // The pair's geometry lives in `MessageEditor`; everything else keeps its own height
    // (デザイン規約 §コミットメッセージの 2 つの枠), so the file list is the one thing here that gives.
    /// Leaves the file list its band and two rows; past this the block scrolls and the pane's bottom holds
    /// (規約 §窓の床).
    readonly property real blockRoom: Math.max(0, detailsPane.height - Theme.headerHeight
        - (changesBand.visible ? changesBand.height : 0) - 2 * Theme.rowHeight)
    // -- smoke hooks and readouts: automation reads the panes, `MessageEditor` holds the numbers --
    function growDescription(dy) { block.growDescription(dy) }
    function pullDescriptionPast(down) { block.pullDescriptionPast(down) }
    /// Whether the grip is refusing a pull, and where the hand is while it does (scene coordinates). The page draws
    /// the badge (`RepoPage.refusalSource`).
    readonly property alias descRefuses: block.descRefuses
    readonly property alias descPoint: block.descPoint
    readonly property bool descGrips: block.descGrips
    readonly property real descHeight: block.descHeight
    readonly property real descWants: block.descWants
    readonly property real descCap: block.descCap
    readonly property int descListRows: block.descListRows
    /// Whether anything in the block is below the fold, and its complement.
    readonly property bool blockScrolls: block.blockScrolls
    readonly property bool descKeeps: block.descKeeps
    /// How much of the bottom edge is left bare for the corner text the page hangs there (as `WipPane.bottomRoom`).
    readonly property real bottomRoom: detailsPane.height - fileList.y
        - Math.max(0, Math.min(fileList.height, fileList.originY + fileList.contentHeight - fileList.contentY))
    /// Whether that measurement is about this commit yet: a list still being read leaves the whole edge bare, the
    /// same number an empty pane gives. Automation only (`PGG_AUTO_ACT=corner`) — a binding re-evaluates when the list
    /// arrives, a photograph does not.
    readonly property bool bottomRoomSettled: !detailsPane.details.loading
        && (fileList.count === 0 || fileList.contentHeight > 0)

    Connections {
        target: detailsPane.details
        function onChanged() {
            detailsPane.syncMessage()
            detailsPane.judgeStash()
        }
    }
    Component.onCompleted: {
        detailsPane.syncMessage()
        detailsPane.judgeStash()
    }

    // Where the arrows stand is where the diff is: the page says so through `readPath`, whoever moved it.
    FileRowWalk {
        id: fileWalk
        sides: [{ view: fileList, model: detailsPane.details }]
        readBucket: ""
        readPath: detailsPane.readPath
        onLanded: (bucket, path, origPath) => detailsPane.fileWalked(path, origPath)
    }
    /// Automation only: for a run that presses the arrows and reads where they landed (verify-ui).
    readonly property alias filesWalk: fileWalk
    /// Automation only: the list's bar, for a run to read what it paints and hold it for a shot. It stands in for
    /// every `PaneScrollBar` — same component, same two states (verify-ui).
    readonly property ScrollBar filesBar: fileList.ScrollBar.vertical

    /// How far this pane's content runs past its right edge, in px — headless cannot see a cut glyph. `fileList`
    /// fills with no margins, so its width is the column's laid-out width (rules-refs/app-ui.md, the
    /// `Layout.fillWidth` line).
    readonly property real contentOverflow: Math.max(0, fileList.width - detailsPane.width)
    /// The same the other way up, for `window-floor`: how far the column runs past the pane's bottom once the file
    /// list has given everything it has.
    readonly property real contentOverHeight: Math.max(0, detailsPane.implicitHeight - detailsPane.height)

    spacing: 0

    // `COMMIT` or the stash's band: exactly one stands and both are `headerHeight`, so `blockRoom` reads the token —
    // a layout gives a hidden child no height. The word is the only tell of how the files below were read: what three
    // or more commits all changed, or what differs between exactly two (デザイン規約 §複数のコミットを選ぶ).
    PaneHeader {
        visible: detailsPane.stashRef === ""
        text: !detailsPane.choosing ? qsTr("COMMIT")
                                    : detailsPane.details.comparing ? qsTr("COMPARING") : qsTr("COMMITS")
        count: detailsPane.choosing ? detailsPane.details.selectionCount : -1
    }
    StashActionsBand {
        Layout.fillWidth: true
        stashRef: detailsPane.stashRef
        onApplyRequested: selector => detailsPane.applyStashRequested(selector)
        onPopRequested: selector => detailsPane.popStashRequested(selector)
    }
    // A picture of the choice, whose only press drops a commit: picking is the graph's, and a second place to pick
    // is a second place the choice could disagree with itself.
    AppListView {
        id: chosenList
        visible: detailsPane.choosing
        Layout.fillWidth: true
        // Divided as the working tree's buckets are (規約 §バケツごとの一覧). This list's want is counted, not read
        // off its content: a view's content height is 0 for the frame its model is replaced in, which would flash the
        // pane to the list below each time the choice changes. The top margin is counted too, or the view stands
        // short of its own content and scrolls with nothing to reach.
        Layout.preferredHeight: detailsPane.choosing
            ? Math.min(chosenList.topMargin + detailsPane.chosenCommits.length * Theme.rowHeight,
                       Math.max(detailsPane.listRoom / 2, detailsPane.listRoom - fileList.contentHeight))
            : 0
        // A centred face's slack, so the first face stands as far under the band as the faces from each other
        // (デザイン規約 §余白 — 行内の詰め).
        topMargin: (Theme.rowHeight - Theme.iconLg) / 2
        model: detailsPane.chosenCommits
        verticalBar: PaneScrollBar {}
        delegate: ChosenCommitRow {
            cardOid: detailsPane.rowCardOid
            onHoverRequested: (row, inside) => detailsPane.rowHoverRequested(row, inside)
            onCopyRequested: text => detailsPane.copyRequested(text)
            onDropRequested: oidHex => detailsPane.commitDropRequested(oidHex)
        }
    }
    /// Automation only (verify-ui).
    function chosenRowAt(index) {
        return chosenList.itemAtIndex(index)
    }
    /// Automation only: for a run that scrolls the list itself and reads where it went.
    readonly property alias chosenView: chosenList
    /// Automation only (verify-ui): the list has laid its rows out, and how far its content runs past the view,
    /// margins counted — two pixels of scroll look like none in a picture.
    readonly property bool chosenListDrawn: chosenList.visible && chosenList.contentHeight > 0
    readonly property real chosenListSpare: Math.max(
        0, chosenList.topMargin + chosenList.contentHeight - chosenList.height)
    DetailsMessageBlock {
        id: block
        visible: !detailsPane.choosing
        Layout.fillWidth: true
        Layout.preferredHeight: Math.min(block.wants, detailsPane.blockRoom)
        details: detailsPane.details
        blockRoom: detailsPane.blockRoom
        listHeight: fileList.height
        paneWidth: detailsPane.width
        editable: detailsPane.editable
        editBlocked: detailsPane.editBlocked
        busy: detailsPane.busy
        published: detailsPane.published
        intoPlan: detailsPane.intoPlan
        messageDirty: detailsPane.messageDirty
        signatureKind: detailsPane.signatureKind
        signatureCode: detailsPane.signatureCode
        signatureSigner: detailsPane.signatureSigner
        committerFace: detailsPane.committerFace
        committerFaceUrl: detailsPane.committerFaceUrl
        stashed: detailsPane.stashShown
        signsCommits: detailsPane.signsCommits
        signingTip: detailsPane.signingTip
        avatarPointedAt: detailsPane.avatarPointedAt
        signaturePointedAt: detailsPane.signaturePointedAt
        summaryPointedAt: detailsPane.summaryPointedAt
        mateCardInside: cards.matesPointerInside
        authorCardInside: cards.authorPointerInside
        parentCardInside: cards.parentsPointerInside
        parentCardOpen: cards.parentCardOpen
        onAvatarClicked: detailsPane.avatarClicked()
        onCopyRequested: text => detailsPane.copyRequested(text)
        onParentClicked: oidHex => detailsPane.parentClicked(oidHex)
        onOpenMateRequested: at => cards.openMateCard(at)
        onSettleMateRequested: cards.settleMates()
        onOpenParentsRequested: line => cards.openParentCard(line)
        onSettleParentsRequested: cards.settleParents()
        onOpenAuthorRequested: at => cards.openAuthorCard(at)
        onSettleAuthorRequested: cards.settleAuthor()
        onEscaped: detailsPane.revertMessage()
        onSaveRequested: detailsPane.submitMessage()
    }
    // Outside the scrolling block: a list whose heading has scrolled away is a list of nothing in particular.
    DetailsChangesBand {
        id: changesBand
        visible: detailsPane.details.shaHex !== "" || detailsPane.choosing
        Layout.fillWidth: true
        count: detailsPane.details.fileTotal
        treeView: detailsPane.details.treeView
        onChosen: tree => detailsPane.details.setTreeView(tree)
    }
    AppListView {
        id: fileList
        Layout.fillWidth: true
        Layout.fillHeight: true
        model: detailsPane.details
        // The pane's own bar — the style's is what `AppListView` hands the graph, the diff and the log.
        verticalBar: PaneScrollBar {}
        // The arrows move the file being read (規約 §diff のファイル一覧); Qt's own key navigation moves
        // `currentIndex` and tells nobody.
        keyNavigationEnabled: false
        Keys.onUpPressed: event => event.accepted = fileWalk.stepFile(-1, event.isAutoRepeat)
        Keys.onDownPressed: event => event.accepted = fileWalk.stepFile(1, event.isAutoRepeat)
        delegate: FileRowDelegate {
            listWidth: fileList.width
            pointedTipRow: detailsPane.pointedTipRow
            menuStanding: detailsPane.menuStanding
            readPath: detailsPane.readPath
            // The press that opened the diff landed in this list, so the keyboard is here; a folder row does not
            // take it, since nothing is read from one.
            onActivated: (bucket, path, origPath) => {
                fileList.forceActiveFocus()
                detailsPane.fileActivated(path, origPath)
            }
            onFolderToggled: key => detailsPane.details.toggleFolder(key)
        }
    }
}
