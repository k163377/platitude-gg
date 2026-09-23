pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Right pane, commit-details mode: message editor, author row, stash
// actions when the selected row is a stash, and the changed-file list.
ColumnLayout {
    id: detailsPane

    required property var details
    /// The commits a choice holds, as the graph named them (`GraphModel.chosenRows` — one record per row: `oid`,
    /// `sha8`, `subject`, `body`, `author`, `atime`, `avatar`, `avatarUrl`, `mates`) — empty while one commit is
    /// what is being read. The rows this pane lists (デザイン規約 §複数のコミットを選ぶ).
    property var chosenCommits: []
    /// The commit whose card is out, so the row it came off keeps its band under it. Written by the page, which owns
    /// the card (`RowHoverHost.rowCardOid`).
    property string rowCardOid: ""
    /// A row of the commit list has been rested on, or left. **The page answers it**: the card is a popup of the
    /// page's, and one card is what keeps the two lists from opening two (デザイン規約 §複数のコミットを選ぶ).
    signal rowHoverRequested(var row, bool inside)
    /// A held press landed on one of the listed commits: it leaves the choice. **The page answers it** — the choice is
    /// the page's, and this list is a picture of it (デザイン規約 §複数のコミットを選ぶ).
    signal commitDropRequested(string oidHex)
    /// The place the two lists divide while a choice is up: the pane, less the band each of them stands under.
    readonly property real listRoom: Math.max(0, detailsPane.height - 2 * Theme.headerHeight)
    /// Whether the pane is showing a choice of commits. **Read off the model** — the
    /// model is what the file list below answers to, and the two must turn over together.
    readonly property bool choosing: detailsPane.details.selectionCount > 1
    // Reflog selector when the selected row is a stash ("" otherwise).
    property string stashRef: ""
    // Whether this commit's message may be rewritten from here — the page
    // decides (`RepoPage.messageEdit` → `offers::message_edit`).
    property bool editable: false
    // A write is already running, so nothing new starts.
    property bool busy: false
    // Why the boxes are read-only, in one line ("" when they are not).
    property string editBlocked: ""
    // A remote already has this commit: the save row says so
    // (デザイン規約「push 済みの範囲はタグで言うだけ」).
    property bool published: false
    // What git makes of the signature: "" (unsigned, or not answered yet),
    // "verified", "signed" or "bad". The code behind it is git's own `%G?`,
    // which the tooltip needs — the mark says one of three things, but the
    // reason a signature could not be judged is one of five.
    property string signatureKind: ""
    property string signatureCode: ""
    property string signatureSigner: ""
    /// Who the rewrite is attributed to — the reader's own identity, since
    /// git keeps the author and replaces the committer.
    property int committerFace: -1
    property string committerFaceUrl: ""
    /// The boxes are a rebase plan's `reword` input right now — the save row wears that verb,
    /// since nothing runs on the press (`RepoPage.planReword`).
    property bool intoPlan: false
    /// The plan's stored reword for one commit, so a row revisited opens on
    /// its draft — the boxes filled from the commit would offer to save
    /// the original back over the draft. Empty oid while no plan holds
    /// one.
    property string planDraftOid: ""
    property string planDraftSubject: ""
    property string planDraftBody: ""
    property bool signsCommits: false
    property string signingTip: ""
    /// The badge was pressed: the settings card opens knowing whom it is about.
    signal avatarEditRequested(string name, string email)
    /// `*PointedAt` stands in for the pointer where headless cannot put one,
    /// so hovers can be photographed (avatar-hover / signature-tip /
    /// stash-tip / path-tip). What each one shows is read back off the
    /// ToolTip's own `visible` — the output side, so a cut binding cannot
    /// read as green. `pointedTipRow` -1 points at no row.
    property bool avatarPointedAt: false
    property bool signaturePointedAt: false
    readonly property bool signatureTipShown: block.signatureTipShown
    property bool summaryPointedAt: false
    readonly property bool summaryTipShown: block.summaryTipShown
    property int pointedTipRow: -1
    /// A right-click menu of the page's is standing over this pane — the
    /// menus are the page's, so only it can say (デザイン規約 §メニュー).
    property bool menuStanding: false
    /// Which of the changed files the middle pane is reading, so its row can
    /// say so (デザイン規約 §diff のファイル一覧を矢印で送る). Empty while the
    /// graph is in the middle: nothing is being read then.
    property string readPath: ""
    function avatarClicked() {
        if (detailsPane.details.authorEmail !== "")
            detailsPane.avatarEditRequested(detailsPane.details.authorName, detailsPane.details.authorEmail)
    }
    /// The two cards the author row opens, and the block the row is in. **Automation-only exposures**, the same one
    /// `GraphPane.view` is (app-ui.md): what a run asks about the credits and the two people is the block's answer or
    /// the cards' own, and a pane that mirrored each of those would be ten forwards that say nothing
    /// (`AutoActTipVerbs`).
    readonly property alias authorCards: cards
    readonly property alias messageBlock: block

    DetailsAuthorCards {
        id: cards
        host: detailsPane
        block: block
        details: detailsPane.details
    }

    signal fileActivated(string path, string origPath)
    /// The arrows walked onto another file. A signal of its own: a click on the file already open closes the diff,
    /// and holding Down walks on (規約 §diff のファイル一覧).
    signal fileWalked(string path, string origPath)
    signal parentClicked(string oidHex)
    signal copyRequested(string text)
    /// Automation only: the row that draws the commit's values, handed to a run whole (verify-ui). A dozen one-line
    /// relays here would say nothing this does not.
    readonly property alias valueRow: block.valueRow
    signal applyStashRequested(string selector)
    signal popStashRequested(string selector)
    signal messageSubmitted(string oidHex, string subject, string body)

    // ---- message editor state: the boxes are filled by hand, since
    // typing would break a binding for good and the next
    // commit would land in a box that no longer listens.
    property string baseOid: ""
    property string baseSubject: ""
    property string baseBody: ""
    /// The boxes have moved off their resting text. Apart from `messageDirty` because a row that leaves `reword`
    /// locks the boxes with the typed text still in them — unsavable, and exactly what a closing plan takes.
    readonly property bool boxMoved: block.subjectText !== detailsPane.baseSubject
                                     || block.bodyText !== detailsPane.baseBody
    readonly property bool messageDirty: detailsPane.editable && detailsPane.boxMoved
    /// Whether a plan is holding a typed reword for the very commit on
    /// screen: what the boxes have to show, since the commit's own
    /// message would offer to save the original back
    /// over the draft.
    readonly property bool planHoldsDraft: detailsPane.details.shaHex !== ""
        && detailsPane.details.shaHex === detailsPane.planDraftOid
        && (detailsPane.planDraftSubject !== "" || detailsPane.planDraftBody !== "")

    /// Adopt the model's message whenever it moves to another commit.
    /// Nothing else can change a message in place — a different message is
    /// a different commit — so an untouched box needs no other cue.
    function syncMessage() {
        if (detailsPane.details.shaHex === detailsPane.baseOid)
            return
        detailsPane.fillMessage(detailsPane.planHoldsDraft)
    }
    /// Fills the boxes from what is resting behind them, and takes that as
    /// the text a save is measured against: the plan's draft where
    /// `drafted`, the commit's own message otherwise. The draft is resting
    /// text too — it is already in the plan, so there is nothing unsaved
    /// about it. The editor is only written when the text actually moves:
    /// the same text assigned back still sends the caret to the box's top
    /// (`MessageEditor.setMessage`), and the plan taking a typed reword
    /// lands here with the caret still in the box.
    function fillMessage(drafted) {
        detailsPane.baseOid = detailsPane.details.shaHex
        detailsPane.baseSubject = drafted ? detailsPane.planDraftSubject
                                          : detailsPane.details.messageSubject
        detailsPane.baseBody = drafted ? detailsPane.planDraftBody : detailsPane.details.messageBody
        if (block.subjectText !== detailsPane.baseSubject
                || block.bodyText !== detailsPane.baseBody)
            block.setMessage(detailsPane.baseSubject, detailsPane.baseBody)
        // Last, and after the boxes: the write above reaches `noteTyping` like any keystroke, so a fill made while
        // a plan stands would otherwise leave the text marked as typed under it.
        detailsPane.boxFromPlan = drafted
    }
    /// Whose the text standing in the boxes is — a question about where it was typed.
    /// Everything written while the plan is routing these boxes is that row's `reword` input, saved into the plan
    /// or not; everything else was typed into the plain amend, on a commit the plan never moved the pane off.
    property bool boxFromPlan: false
    /// The boxes took a character. Latched: what decides it is the state the text was written in, and
    /// by the time a closing plan asks, that state is gone.
    function noteTyping() {
        if (detailsPane.intoPlan)
            detailsPane.boxFromPlan = true
    }
    Connections {
        target: block
        function onSubjectTextChanged() { detailsPane.noteTyping() }
        function onBodyTextChanged() { detailsPane.noteTyping() }
    }
    /// The plan's hold moved without the commit moving, so `syncMessage`'s
    /// own guard would sit still: a row's verb left `reword` and took the
    /// draft with it (`RebasePlanModel::set_action`), or the draft for the
    /// row already up arrived. A dropped draft left resting in the boxes
    /// reads as unchanged, so the next `reword` would find nothing to save
    /// and refuse the press with nothing on screen to say why.
    onPlanHoldsDraftChanged: detailsPane.fillMessage(detailsPane.planHoldsDraft)
    /// The plan is gone, and with it the row that was to carry whatever is
    /// in these boxes. The commit's own message goes back: the closing
    /// plan hands the boxes to the plain amend, and a reword left resting
    /// in them would stand there as a one-press rewrite of the history the
    /// reader just walked away from (デザイン規約 §コミットメッセージの 2 つの枠).
    ///
    /// **Only what was written under the plan.** A half-written amend from before it stood is the reader's own
    /// sentence on the reader's own commit, and nothing on screen could give it back.
    function dropDraft() {
        if (!detailsPane.boxFromPlan)
            return
        detailsPane.fillMessage(false)
    }
    /// Put the commit's own message back.
    function revertMessage() {
        block.setTexts(detailsPane.baseSubject, detailsPane.baseBody)
    }
    /// git took the new message: what was written becomes the resting text,
    /// since the model still holds the old one until the selection follows.
    function noteMessageSaved() {
        detailsPane.baseSubject = block.subjectText
        detailsPane.baseBody = block.bodyText
    }
    function submitMessage() {
        // Nothing changed is nothing to do: the button stands from the moment someone is writing
        // (`MessageActionsRow.editing`), so this press is ordinary.
        if (!detailsPane.editable || !detailsPane.messageDirty || block.subjectText.trim() === "")
            return
        detailsPane.messageSubmitted(detailsPane.details.shaHex, block.subjectText, block.bodyText)
    }
    /// Smoke hook: type into the boxes the way a keystroke would — including
    /// not at all when they are read-only.
    function setMessageText(subject, body) {
        if (!detailsPane.editable)
            return
        block.setTexts(subject, body)
    }
    /// Smoke hook: what the boxes are holding. `boxMoved` says the text left
    /// its resting point.
    readonly property alias boxSubject: block.subjectText

    /// A reader was sent here from somewhere that could not hold the whole
    /// message (the graph row's hover card), so the boxes say which they
    /// are (デザイン規約 §hover のツールチップ). The mark stands until the
    /// reader's next press — they arrived, and the first thing they do is
    /// the proof of it (`RepoPage.notePress`).
    function callAttention() {
        block.callAttention()
    }
    function dropAttention() {
        block.dropAttention()
    }
    readonly property alias attention: block.attention

    /// Smoke hook: put the caret in the description box, the way a click in
    /// it does. The colour the text takes under a caret is what the shot is of.
    function focusDescription() {
        block.focusDescription()
    }
    /// What the box paints — the input side (activeFocus) would read green
    /// with the binding cut.
    readonly property color descriptionColor: block.descriptionColor
    readonly property bool descriptionFocused: block.descriptionFocused
    /// Smoke hooks: the wheel over the description box, and how much ink its
    /// own bar carries as a result (デザイン規約 §QML 実装ルール のバーの明るさ).
    function rollDescription(dy) { block.rollDescription(dy) }
    function holdDescriptionBar(on) { block.holdDescriptionBar(on) }
    readonly property real descriptionBarInk: block.descriptionBarInk
    readonly property real descriptionAt: block.descriptionAt

    // -- what this pane lends the message editor --
    //
    // The pair's own geometry lives in `MessageEditor`; everything between
    // the boxes and the file list keeps its own height by construction
    // (デザイン規約 §コミットメッセージの 2 つの枠), so the list is the one
    // thing here that gives.
    /// How much of the pane the block between the two bands may take: all of
    /// it but the list's own band and the two rows that keep a list a list.
    /// Past this the block scrolls and the pane's bottom holds
    /// (規約 §窓の床).
    readonly property real blockRoom: Math.max(0, detailsPane.height - Theme.headerHeight
        - (changesBand.visible ? changesBand.height : 0) - 2 * Theme.rowHeight)
    // -- smoke hooks and readouts, said under the pane's name because the
    // automation reads the panes (`MessageEditor` holds the numbers) --
    function growDescription(dy) { block.growDescription(dy) }
    function pullDescriptionPast(down) { block.pullDescriptionPast(down) }
    /// Whether the grip is refusing a pull, and where the hand is while it
    /// does (scene coordinates). The page draws the badge — see `RepoPage`
    /// on why it cannot be drawn in the box.
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
    /// How much of the bottom edge is left bare for the corner text the page
    /// hangs there (the same measurement `WipPane.bottomRoom` makes).
    readonly property real bottomRoom: detailsPane.height - fileList.y
        - Math.max(0, Math.min(fileList.height, fileList.originY + fileList.contentHeight - fileList.contentY))
    /// Whether that measurement is about this commit yet. **A list still being read leaves the whole bottom edge
    /// bare**, which is an empty pane's answer rather than this one's — and the two are the same number, so nothing
    /// downstream can tell them apart (observed: a headless run reported the corner standing with 823 pixels to
    /// spare over a list that runs off the pane). Automation reads this before it reads `bottomRoom`
    /// (`PGG_AUTO_ACT=corner`); nothing in the product needs it, because a binding re-evaluates when the list
    /// arrives and a photograph does not.
    readonly property bool bottomRoomSettled: !detailsPane.details.loading
        && (fileList.count === 0 || fileList.contentHeight > 0)

    Connections {
        target: detailsPane.details
        function onChanged() { detailsPane.syncMessage() }
    }
    Component.onCompleted: detailsPane.syncMessage()

    // Where the arrows stand is where the diff is: the page says so through
    // `readPath`, whoever moved it, and the walk sends the light and the
    // reading on from there (規約 §diff のファイル一覧).
    FileRowWalk {
        id: fileWalk
        sides: [{ view: fileList, model: detailsPane.details }]
        readBucket: ""
        readPath: detailsPane.readPath
        onLanded: (bucket, path, origPath) => detailsPane.fileWalked(path, origPath)
    }
    /// Automation only: the walk itself, for a run that presses the arrows
    /// and reads where they landed (verify-ui).
    readonly property alias filesWalk: fileWalk
    /// Automation only: the bar that list wears, so a run can read what it
    /// paints and hold it there for a shot — one panel's bar stands in for
    /// both, same component and same two states (verify-ui).
    readonly property ScrollBar filesBar: fileList.ScrollBar.vertical

    /// How far this pane's own content runs past its right edge, in px —
    /// headless cannot see a cut glyph, so this is the number instead. A
    /// child with no `Layout.fillWidth` of its own is Fixed (only a nested
    /// layout fills by default) and the layout hands a Fixed child its
    /// implicit width and never a pixel less, so one row that will not give
    /// is a floor the whole column sits on: the column lays itself out at
    /// that floor while the pane keeps the width the splitter set, and every
    /// box in it, sized to fill, paints over the window's edge with the
    /// glyphs cut in half. `fileList` fills and carries no margins, so its
    /// width *is* that laid-out width.
    readonly property real contentOverflow: Math.max(0, fileList.width - detailsPane.width)
    /// The same question the other way up, for `window-floor`: how far the
    /// column runs past the pane's own bottom once the file list has given
    /// everything it has.
    readonly property real contentOverHeight: Math.max(0, detailsPane.implicitHeight - detailsPane.height)

    spacing: 0

    // The pane's band: `COMMIT`, or — while the selected row is a stash — what that stash is and what can be done
    // with it (`StashActionsBand`). Exactly one stands and both are `headerHeight`, so `blockRoom` reads the token:
    // a layout gives a hidden child no height, so whichever is down would answer 0.
    // **The band is what says how the files below were read.** Three or more commits are listed as themselves and the
    // list under them is what they all changed; exactly two are being compared, and the list is what differs between
    // them (デザイン規約 §複数のコミットを選ぶ). Those are different questions with the same answer shape, so the word
    // above the commits is the only place a reader can tell them apart.
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
    // The commits a choice holds. **A picture, and the only press is the one that drops a commit**: they are picked
    // in the graph, and a second place the choice appeared to be made is a second place it could disagree with itself.
    AppListView {
        id: chosenList
        visible: detailsPane.choosing
        Layout.fillWidth: true
        // Two lists in one pane, and the place is divided the way the working tree's buckets divide theirs
        // (規約 §バケツごとの一覧): **an equal share each, and whatever the other one cannot use on top**. A choice of
        // forty commits over one file leaves the file list a row and this one the rest.
        //
        // What the file list wants is read off its own content — the tree view adds
        // folder rows, so a count of files is not its height. **What this list wants is counted**: a view's own
        // content height is zero for the frame its model is being replaced in, and a share worked out from that hands
        // the whole pane to the list below for one frame, which is a flash every time a commit joins or leaves the
        // choice. The rows here are one line each, so a count is the same answer and it is there before the view is.
        //
        // **The margin above the first row is counted with them**: it is the view's own, and a
        // view standing short of its own content scrolls with nothing down there to reach.
        Layout.preferredHeight: detailsPane.choosing
            ? Math.min(chosenList.topMargin + detailsPane.chosenCommits.length * Theme.rowHeight,
                       Math.max(detailsPane.listRoom / 2, detailsPane.listRoom - fileList.contentHeight))
            : 0
        // Half a row's slack, which is all a face centred in a row has above it: with this the first face stands the
        // same distance under the band as the faces below it stand from each other, so the column begins the way it
        // goes on (デザイン規約 §余白 — 行内の詰め).
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
    /// Automation only: a row of the commit list, handed to a run whole (verify-ui).
    function chosenRowAt(index) {
        return chosenList.itemAtIndex(index)
    }
    /// Automation only: the commit list itself, for a run that sends it by its own hand and reads where it went.
    readonly property alias chosenView: chosenList
    /// Automation only: the commit list has laid its rows out, and how far past the view its content runs — **the
    /// view's own margins counted** (verify-ui). A picture answers neither: two pixels of scroll on a list that has
    /// nothing below the fold frames exactly like none.
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
        signsCommits: detailsPane.signsCommits
        signingTip: detailsPane.signingTip
        avatarPointedAt: detailsPane.avatarPointedAt
        signaturePointedAt: detailsPane.signaturePointedAt
        summaryPointedAt: detailsPane.summaryPointedAt
        mateCardInside: cards.matesPointerInside
        authorCardInside: cards.authorPointerInside
        onAvatarClicked: detailsPane.avatarClicked()
        onCopyRequested: text => detailsPane.copyRequested(text)
        onParentClicked: oidHex => detailsPane.parentClicked(oidHex)
        onOpenMateRequested: at => cards.openMateCard(at)
        onSettleMateRequested: cards.settleMates()
        onOpenAuthorRequested: at => cards.openAuthorCard(at)
        onSettleAuthorRequested: cards.settleAuthor()
        onEscaped: detailsPane.revertMessage()
        onSaveRequested: detailsPane.submitMessage()
    }
    // Outside the block above: it is the list's own band, and a list whose
    // heading has scrolled away is a list of nothing in particular.
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
        // The arrows are answered here, where they move the file being read:
        // Qt's own key navigation moves `currentIndex` and tells
        // nobody (規約 §diff のファイル一覧).
        keyNavigationEnabled: false
        Keys.onUpPressed: event => event.accepted = fileWalk.stepFile(-1, event.isAutoRepeat)
        Keys.onDownPressed: event => event.accepted = fileWalk.stepFile(1, event.isAutoRepeat)
        delegate: FileRowDelegate {
            listWidth: fileList.width
            pointedTipRow: detailsPane.pointedTipRow
            menuStanding: detailsPane.menuStanding
            readPath: detailsPane.readPath
            // The press that opened the diff landed in this list, so this is
            // where the keyboard is (規約 §diff のファイル一覧). A folder row
            // does not take it: nothing is being read from one.
            onActivated: (bucket, path, origPath) => {
                fileList.forceActiveFocus()
                detailsPane.fileActivated(path, origPath)
            }
            onFolderToggled: key => detailsPane.details.toggleFolder(key)
        }
    }
}
