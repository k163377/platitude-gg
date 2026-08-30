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
    // Reflog selector when the selected row is a stash ("" otherwise).
    property string stashRef: ""
    // Whether this commit's message may be rewritten from here — the page
    // decides (`RepoPage.messageEdit` → `offers::message_edit`).
    property bool editable: false
    // A write is already running, so nothing new starts.
    property bool busy: false
    // Why the boxes are read-only, in one line ("" when they are not).
    property string editBlocked: ""
    // A remote already has this commit: the save row says so rather than
    // asking (デザイン規約「push 済みの範囲は尋ねずに言う」).
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
    /// The boxes are a rebase plan's `reword` input right now — the save row wears that verb instead of the amend's
    /// command, since nothing runs on the press (`RepoPage.planReword`).
    property bool intoPlan: false
    /// The plan's stored reword for one commit, so a row revisited opens on
    /// its draft rather than on the message it is replacing — the boxes
    /// filled from the commit would offer to save the original back over
    /// the draft. Empty oid while no plan holds one.
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
    readonly property bool signatureTipShown: authorRow.signatureTipShown
    property bool summaryPointedAt: false
    readonly property bool summaryTipShown: msgEditor.summaryTipShown
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
    // ---- the author row's two cards ---------------------------------
    // `CommitAuthorRow` owns the hover state and raises anchor points; the
    // cards stay here because they open in pane coordinates and must not
    // scroll away with the block the row sits on. The pane forwards the
    // hooks — the automation reads the panes.
    readonly property var coAuthorRecords: authorRow.coAuthorRecords
    function coAuthorName(i) {
        return authorRow.coAuthorName(i)
    }
    /// The credit line ran out of room and was elided — what a headless run
    /// reads in place of an ellipsis it cannot see.
    readonly property bool coAuthorsClipped: authorRow.matesClipped
    function showCoAuthors(on) {
        authorRow.showCoAuthors(on)
    }
    /// Whether the card is on screen — the output side, since the input side
    /// would read true with the binding cut.
    readonly property bool matesCardOpen: mateCard.opened
    /// The row raised an anchor in its own coordinates: map it here and open.
    /// Set on open rather than bound — the answer only matters at the moment
    /// it is asked.
    function openMateCard(at) {
        if (detailsPane.coAuthorRecords.length === 0)
            return
        const p = authorRow.mapToItem(detailsPane, at.x, at.y)
        mateCard.records = detailsPane.coAuthorRecords
        // Measured from where it opens, not from the pane: the card starts
        // partway across, so the pane's width is not what is left for it.
        mateCard.maxRowWidth = detailsPane.width - p.x - 2 * Theme.spaceXs
        mateCard.x = p.x
        // Flush against the underline: a gap is a band the pointer crosses
        // while touching neither, and the card closes under it. Same rule
        // the ref list follows.
        mateCard.y = p.y
        mateCard.open()
    }
    CoAuthorCard {
        id: mateCard
    }
    HoverCardHost {
        id: mateKeep
        card: mateCard
        pointedAt: authorRow.matesPointed
    }

    function showAuthor(on) {
        authorRow.showAuthor(on)
    }
    readonly property bool authorCardOpen: authorCard.opened
    readonly property bool authorNameClipped: authorRow.nameClipped
    function openAuthorCard(at) {
        if (detailsPane.details.authorName === "")
            return
        const p = authorRow.mapToItem(detailsPane, at.x, at.y)
        authorCard.maxRowWidth = detailsPane.width - p.x - 2 * Theme.spaceXs
        authorCard.x = p.x
        authorCard.y = p.y
        authorCard.open()
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
    }
    HoverCardHost {
        id: authorKeep
        card: authorCard
        pointedAt: authorRow.authorPointed
    }

    signal fileActivated(string path, string origPath)
    /// The arrows walked onto another file. Not what a click raises: a click on the file already open closes the diff,
    /// and holding Down must not (規約 §diff のファイル一覧).
    signal fileWalked(string path, string origPath)
    signal parentClicked(string oidHex)
    signal copyRequested(string text)
    /// Automation only: the row that draws the commit's values, handed to a run whole (verify-ui). A dozen one-line
    /// relays here would say nothing this does not.
    readonly property alias valueRow: authorRow
    signal applyStashRequested(string selector)
    signal popStashRequested(string selector)
    signal messageSubmitted(string oidHex, string subject, string body)

    // ---- message editor state: the boxes are filled by hand rather than
    // bound, since typing would break a binding for good and the next
    // commit would land in a box that no longer listens.
    property string baseOid: ""
    property string baseSubject: ""
    property string baseBody: ""
    readonly property bool messageDirty: detailsPane.editable
        && (msgEditor.subjectText !== detailsPane.baseSubject || msgEditor.bodyText !== detailsPane.baseBody)

    /// Adopt the model's message whenever it moves to another commit.
    /// Nothing else can change a message in place — a different message is
    /// a different commit — so an untouched box needs no other cue. A plan
    /// row with a stored reword opens on the draft instead, and the draft
    /// is the resting text: it is already in the plan, so there is nothing
    /// unsaved about it.
    function syncMessage() {
        if (detailsPane.details.shaHex === detailsPane.baseOid)
            return
        detailsPane.baseOid = detailsPane.details.shaHex
        const drafted = detailsPane.details.shaHex !== ""
            && detailsPane.details.shaHex === detailsPane.planDraftOid
            && (detailsPane.planDraftSubject !== "" || detailsPane.planDraftBody !== "")
        detailsPane.baseSubject = drafted ? detailsPane.planDraftSubject
                                          : detailsPane.details.messageSubject
        detailsPane.baseBody = drafted ? detailsPane.planDraftBody : detailsPane.details.messageBody
        msgEditor.setMessage(detailsPane.baseSubject, detailsPane.baseBody)
    }
    /// Put the commit's own message back.
    function revertMessage() {
        msgEditor.setTexts(detailsPane.baseSubject, detailsPane.baseBody)
    }
    /// git took the new message: what was written becomes the resting text,
    /// since the model still holds the old one until the selection follows.
    function noteMessageSaved() {
        detailsPane.baseSubject = msgEditor.subjectText
        detailsPane.baseBody = msgEditor.bodyText
    }
    function submitMessage() {
        // Nothing changed is nothing to do: the button stands from the moment someone is writing rather than from the
        // moment the text differs (`MessageActionsRow.editing`), so this press is ordinary, not a mistake.
        if (!detailsPane.editable || !detailsPane.messageDirty || msgEditor.subjectText.trim() === "")
            return
        detailsPane.messageSubmitted(detailsPane.details.shaHex, msgEditor.subjectText, msgEditor.bodyText)
    }
    /// Smoke hook: type into the boxes the way a keystroke would — including
    /// not at all when they are read-only.
    function setMessageText(subject, body) {
        if (!detailsPane.editable)
            return
        msgEditor.setTexts(subject, body)
    }
    /// Smoke hook: put the caret in the description box, the way a click in
    /// it does. The colour the text takes under a caret is what the shot is of.
    function focusDescription() {
        msgEditor.focusDescription()
    }
    /// What the box paints — the input side (activeFocus) would read green
    /// with the binding cut.
    readonly property color descriptionColor: msgEditor.descriptionColor
    readonly property bool descriptionFocused: msgEditor.descriptionFocused
    /// Smoke hooks: the wheel over the description box, and how much ink its
    /// own bar carries as a result (デザイン規約 §QML 実装ルール のバーの明るさ).
    function rollDescription(dy) { msgEditor.rollDescription(dy) }
    function holdDescriptionBar(on) { msgEditor.holdDescriptionBar(on) }
    readonly property real descriptionBarInk: msgEditor.descriptionBarInk
    readonly property real descriptionAt: msgEditor.descriptionAt

    // -- what this pane lends the message editor --
    //
    // The pair's own geometry lives in `MessageEditor`; everything between
    // the boxes and the file list keeps its own height by construction
    // (デザイン規約 §コミットメッセージの 2 つの枠), so the list is the one
    // thing here that gives.
    /// How much of the pane the block between the two bands may take: all of
    /// it but the list's own band and the two rows that keep a list a list.
    /// Past this the block scrolls rather than running out of the pane's
    /// bottom (規約 §窓の床).
    readonly property real blockRoom: Math.max(0, detailsPane.height - Theme.headerHeight
        - (changesBand.visible ? changesBand.height : 0) - 2 * Theme.rowHeight)
    /// Moves the block by a wheel a box on it could not use: the boxes cover
    /// most of the block, so a box that keeps the wheel at its own end
    /// leaves the block unreachable by wheel.
    function rollBlock(pixels) {
        const max = Math.max(0, blockScroll.contentHeight - blockScroll.height)
        // Taken away, not added: content travels against `contentY`.
        blockScroll.contentY = Math.max(0, Math.min(max, blockScroll.contentY - pixels))
    }
    // -- smoke hooks and readouts, said under the pane's name because the
    // automation reads the panes (`MessageEditor` holds the numbers) --
    function growDescription(dy) { msgEditor.growDescription(dy) }
    function pullDescriptionPast(down) { msgEditor.pullDescriptionPast(down) }
    /// Whether the grip is refusing a pull, and where the hand is while it
    /// does (scene coordinates). The page draws the badge — see `RepoPage`
    /// on why it cannot be drawn in the box.
    readonly property alias descRefuses: msgEditor.descRefuses
    readonly property alias descPoint: msgEditor.descPoint
    readonly property bool descGrips: msgEditor.descGrips
    readonly property real descHeight: msgEditor.descHeight
    readonly property real descWants: msgEditor.descWants
    readonly property real descCap: msgEditor.descCap
    readonly property int descListRows: msgEditor.descListRows
    /// Whether anything in the block is below the fold, and its complement.
    readonly property bool blockScrolls: msgEditor.blockScrolls
    readonly property bool descKeeps: msgEditor.descKeeps
    /// How much of the bottom edge is left bare for the corner text the page
    /// hangs there (the same measurement `WipPane.bottomRoom` makes).
    readonly property real bottomRoom: detailsPane.height - fileList.y
        - Math.max(0, Math.min(fileList.height, fileList.originY + fileList.contentHeight - fileList.contentY))

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
    // with it (`StashActionsBand`). Exactly one stands and both are `headerHeight`, so `blockRoom` reads the token
    // rather than either: a layout gives a hidden child no height, so whichever is down would answer 0.
    PaneHeader {
        visible: detailsPane.stashRef === ""
        text: qsTr("COMMIT")
    }
    StashActionsBand {
        Layout.fillWidth: true
        stashRef: detailsPane.stashRef
        onApplyRequested: selector => detailsPane.applyStashRequested(selector)
        onPopRequested: selector => detailsPane.popStashRequested(selector)
    }
    // Everything between the two bands, in a surface of its own that scrolls
    // when the pane is too short to hold it: the boxes, the author row and
    // the save row keep their heights by construction, so the file list was
    // the only thing that could give and past zero the rest ran out of the
    // pane's bottom (規約 §窓の床).
    Flickable {
        id: blockScroll
        Layout.fillWidth: true
        Layout.preferredHeight: Math.min(blockCol.implicitHeight, detailsPane.blockRoom)
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
            width: blockScroll.width
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
                visible: detailsPane.details.shaHex !== ""

                // Who wrote it, first: avatar + name/date on the left, own
                // hash over parent hash on the right (rows aligned). The pane
                // reads top to bottom the way the commit itself does — whose
                // it is, what it says, what it touched.
                CommitAuthorRow {
                    id: authorRow
                    // Said out loud: this row is an `Item`, and a plain item takes its own width and stops, which parks
                    // the hash plate against the end of the name.
                    Layout.fillWidth: true
                    details: detailsPane.details
                    signatureKind: detailsPane.signatureKind
                    signatureCode: detailsPane.signatureCode
                    signatureSigner: detailsPane.signatureSigner
                    avatarPointedAt: detailsPane.avatarPointedAt
                    signaturePointedAt: detailsPane.signaturePointedAt
                    paneWidth: detailsPane.width
                    mateCardInside: mateCard.pointerInside
                    authorCardInside: authorCard.pointerInside
                    onAvatarClicked: detailsPane.avatarClicked()
                    onCopyRequested: text => detailsPane.copyRequested(text)
                    onParentClicked: oidHex => detailsPane.parentClicked(oidHex)
                    onOpenMateRequested: at => detailsPane.openMateCard(at)
                    onSettleMateRequested: mateKeep.settle()
                    onOpenAuthorRequested: at => detailsPane.openAuthorCard(at)
                    onSettleAuthorRequested: authorKeep.settle()
                }
                // Then the message: the same pair, in the same component the
                // commit editor writes in, one block of one height
                // (デザイン規約 §コミットメッセージの 2 つの枠).
                MessageEditor {
                    id: msgEditor
                    Layout.fillWidth: true
                    Layout.preferredHeight: msgEditor.pairHeight
                    readOnly: !detailsPane.editable
                    blockedTip: detailsPane.editBlocked
                    summaryPointedAt: detailsPane.summaryPointedAt
                    listHeight: fileList.height
                    blockRoom: detailsPane.blockRoom
                    blockHeight: blockCol.implicitHeight
                    onWheelPastEnd: pixels => detailsPane.rollBlock(pixels)
                    // Escape drops the draft and puts the commit's own message back. Nothing asks: the reader said so
                    // (デザイン規約 §コミットメッセージの 2 つの枠).
                    onEscaped: detailsPane.revertMessage()
                }
                // Only once something is actually changed (`MessageActionsRow`).
                MessageActionsRow {
                    Layout.fillWidth: true
                    dirty: detailsPane.messageDirty
                    editing: detailsPane.editable && msgEditor.anyFocused
                    intoPlan: detailsPane.intoPlan
                    published: detailsPane.published
                    busy: detailsPane.busy
                    canSave: msgEditor.subjectText.trim() !== ""
                    committerFace: detailsPane.committerFace
                    committerFaceUrl: detailsPane.committerFaceUrl
                    signature: detailsPane.signsCommits ? "signed" : ""
                    signatureTip: detailsPane.signingTip
                    onSaveRequested: detailsPane.submitMessage()
                }
            }
        }
    }
    // Outside the block above: it is the list's own band, and a list whose
    // heading has scrolled away is a list of nothing in particular.
    DetailsChangesBand {
        id: changesBand
        visible: detailsPane.details.shaHex !== ""
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
        // The pane's own bar, in place of the style's one `AppListView` hands the graph, the diff and the log.
        verticalBar: PaneScrollBar {}
        // Qt's own key navigation moves `currentIndex` and tells nobody; the
        // arrows are answered here instead, where they move the file being
        // read (規約 §diff のファイル一覧).
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
