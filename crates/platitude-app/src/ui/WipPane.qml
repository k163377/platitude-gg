pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Right pane, working-tree (WIP) mode: the changed files, one list per bucket with stage/unstage affordances, over
// the commit editor pinned to the pane's bottom.
ColumnLayout {
    id: wipPane

    required property var repoTab
    required property var workTree
    /// One model per bucket run, each showing its own run and answering about the whole tree
    /// (`NavSectionModel::attach_worktree`) — so `worktreeModel`, the unstaged one, is where the pane asks the tree.
    required property var worktreeModel
    required property var conflictsModel
    required property var stagedModel
    // Mirrored page state; `headPublished` shows the warning tag.
    property bool amending: false
    property bool headPublished: false
    /// Stands in for the pointer on a file row, for the cut-down row's tooltip (PGG_AUTO_ACT=path-tip); -1 is none.
    property int pointedTipRow: -1
    /// A page's right-click menu is standing over this pane — the menus are the page's, so only it can say
    /// (デザイン規約 §メニュー).
    property bool menuStanding: false
    /// The working-tree file the middle pane is reading, empty while the graph is there: what the arrows walk from
    /// (規約 §diff のファイル一覧). Its light is this list's own choice — see `readOne`.
    property string readBucket: ""
    property string readPath: ""
    /// The file being read, keyed as the rows are, and **empty unless both halves are there**: the page empties them
    /// one binding at a time (`RepoPage.closeDiff`), and a half-state key names no row (rules-refs/app-ui.md の
    /// `readKey` の行).
    readonly property string readKey:
        wipPane.readBucket === "" || wipPane.readPath === ""
        ? "" : wipPane.readBucket + ":" + wipPane.readPath
    /// The row the reading was last on.
    property string wasRead: ""
    /// The reading going away, by any door, takes its row out of the choice (デザイン規約 §diff のファイル一覧) —
    /// **only where that row was the whole of it**: several lit rows are a choice made for staging or discarding.
    onReadKeyChanged: {
        if (wipPane.readKey === "" && wipPane.soleChosen !== "" && wipPane.soleChosen === wipPane.wasRead)
            wipPane.clearChoice()
        wipPane.wasRead = wipPane.readKey
    }

    /// Whoever the typed message credits, in the records the details pane's line is fed (`encode::mates_of`).
    readonly property var messageMates: GitFacts.coAuthorsOf(commitBlock.bodyText)
    /// One record per person.
    readonly property int mateCount: wipPane.messageMates.length
    /// Stands in for the pointer on the signing tick in the commit button (`commit-face`).
    property bool signingPointedAt: false
    /// What it puts on screen, for the headless report: the tooltip's own visible (the output side).
    readonly property bool signingTipShown: commitBlock.signingTipShown
    /// Where a press lands it. Detached, the chip says git's own `HEAD` — naming nothing would read as "somewhere".
    readonly property string commitTarget:
        wipPane.workTree.detached ? "HEAD" : wipPane.workTree.branch

    signal amendToggled(bool on)
    signal commitClicked()
    signal fileActivated(string bucket, string path, string origPath)
    /// The arrows walked onto another file. A signal of its own: a click on the file already open closes the
    /// diff, and holding Down keeps it open (規約 §diff のファイル一覧).
    signal fileWalked(string bucket, string path, string origPath)
    /// Right-click on a file row; the page owns the menu because delegates are recycled out from under an open popup.
    signal fileMenuRequested(string bucket, string path)

    /// Tree or flat paths — one choice for all three lists. **The page applies it**: the choice outlives this pane,
    /// and the pane another working copy is read in has the same toggle (`RepoPage.setWipTreeView`).
    signal treeViewChosen(bool tree)

    /// Automation: run one of the stopped operation's held rows to its end, named by its flag (`OpExitCard`).
    function completeOpExit(code) {
        return commitBlock.completeOpExit(code)
    }
    /// Automation: whether the card offers that row at all.
    function offersOpExit(code) {
        return commitBlock.offersOpExit(code)
    }
    /// Automation: press the button that finishes things, the way a click does.
    function pressCommit() {
        if (!commitBlock.commitEnabled)
            return false
        wipPane.commitClicked()
        return true
    }

    // ---- how the buckets share the pane ------------------------------
    // Even shares, and a share a bucket cannot fill goes to the ones that can (デザイン規約 §バケツごとの一覧と、ペインの分け方).

    /// **A clean tree stands no bucket**: a `(0)` heads nothing.
    readonly property bool bucketsStanding: wipPane.worktreeModel.total > 0
    /// The standing buckets, top to bottom — what everything reaching across buckets reads.
    readonly property var bucketPanes:
        conflictsBucket.visible ? [conflictsBucket, unstagedBucket, stagedBucket]
                                : [unstagedBucket, stagedBucket]
    /// Rows above the unstaged bucket in the pane's numbering.
    readonly property int conflictRows: conflictsBucket.visible ? conflictsBucket.rows : 0
    /// A heading each and the hairline between neighbours: both always stand, so neither is in the room shared out.
    readonly property real bucketFrame:
        wipPane.bucketPanes.length * Theme.rowHeight
        + (wipPane.bucketPanes.length - 1) * Theme.borderWidth
    readonly property real bucketRoom: Math.max(0, buckets.height - wipPane.bucketFrame)
    /// Each bucket's row room, always in the order conflicted / unstaged / staged.
    readonly property var bucketSeats: wipPane.shareOut(wipPane.bucketRoom, [
        conflictsBucket.visible ? conflictsBucket.wants : -1,
        unstagedBucket.wants,
        stagedBucket.wants])
    /// Shares `room` out between the buckets. `wants` is what each would take unhindered, in the same order as the
    /// answer, and **-1 for a bucket that is not standing**. Spare room is split evenly — handed to the last one, one
    /// file against none draws an empty bucket holding the whole pane over the file pressed into a single row.
    function shareOut(room, wants) {
        const out = []
        const standing = []
        for (let i = 0; i < wants.length; i++) {
            out.push(0)
            if (wants[i] >= 0)
                standing.push(i)
        }
        if (standing.length === 0)
            return out
        const asked = standing.slice().sort((a, b) => wants[a] - wants[b])
        let left = room
        for (let i = 0; i < asked.length; i++) {
            const at = asked[i]
            out[at] = Math.min(wants[at], left / (asked.length - i))
            left -= out[at]
        }
        const spare = left / standing.length
        for (let i = 0; i < standing.length; i++)
            out[standing[i]] += spare
        return out
    }

    // ---- which rows are chosen -------------------------------------
    // Held here, keyed `<bucket>:<path>`, because a delegate is recycled the moment its row scrolls off.
    property var chosenKeys: ({})
    property int chosenCount: 0
    /// The row the next Shift-click reaches from.
    property int anchorRow: -1
    /// The one chosen row, as `<bucket>:<path>`, and empty unless exactly one is.
    readonly property string soleChosen: {
        if (wipPane.chosenCount !== 1)
            return ""
        for (const key in wipPane.chosenKeys)
            return key
        return ""
    }
    function isChosen(bucket, path) {
        return wipPane.chosenKeys[bucket + ":" + path] === true
    }
    /// The chosen files across all buckets, in standing order, as `{ bucket, fullName }`. **Asked of the models**: a
    /// walk over delegates would hand a discard or a stash just the rows on screen
    /// (rules-refs/app-ui.md「WIP の選択は模型が解く」).
    function chosenRows() {
        const out = []
        for (let i = 0; i < wipPane.rowCount(); i++) {
            const key = wipPane.keyAt(i)
            if (key !== "" && wipPane.chosenKeys[key] === true)
                out.push(wipPane.rowOfKey(key))
        }
        return out
    }
    /// The bucket never holds a colon, so the first one is the divide.
    function rowOfKey(key) {
        const cut = key.indexOf(":")
        return { bucket: key.substring(0, cut), fullName: key.substring(cut + 1) }
    }
    /// The pane's row numbering across standing buckets, counted off the models, and the key at a place in it (empty
    /// for a folder row).
    function rowCount() {
        const standing = wipPane.bucketPanes
        let out = 0
        for (let i = 0; i < standing.length; i++)
            out += standing[i].rows
        return out
    }
    function keyAt(index) {
        const standing = wipPane.bucketPanes
        let at = index
        for (let i = 0; i < standing.length; i++) {
            if (at < standing[i].rows)
                return standing[i].model.fileKeyAt(at)
            at -= standing[i].rows
        }
        return ""
    }
    /// The delegate at a place in the rows — only for the automation's clicks and placing a card against its row. Null
    /// for a row the view has not built; walks over the choice go through [`keyAt`].
    function rowAt(index) {
        const standing = wipPane.bucketPanes
        let at = index
        for (let i = 0; i < standing.length; i++) {
            if (at < standing[i].rows)
                return standing[i].list.itemAtIndex(at)
            at -= standing[i].rows
        }
        return null
    }
    /// Makes one row the whole of the choice — a plain click, or a right-click outside the choice before its menu
    /// (the menu acts on what is lit).
    function chooseOnly(bucket, path) {
        wipPane.applyClick(bucket, path, Qt.NoModifier)
    }
    /// A file is being read without a click saying so (`RepoPage.followEmptySide`, or an arrow): the choice follows it
    /// (デザイン規約 §diff のファイル一覧).
    function readOne(bucket, path) {
        if (wipPane.chosenCount !== 1 || !wipPane.isChosen(bucket, path))
            wipPane.chooseOnly(bucket, path)
    }

    // ---- walking the files with the arrow keys ------------------------
    // The light this list already had is the choice, so a step moves the choice; the reading follows a beat later (規約
    // §diff のファイル一覧).
    FileRowWalk {
        id: fileWalk
        // The arrows cross from one bucket's list into the next.
        sides: {
            const standing = wipPane.bucketPanes
            const out = []
            for (let i = 0; i < standing.length; i++)
                out.push({ view: standing[i].list, model: standing[i].model })
            return out
        }
        readBucket: wipPane.readBucket
        readPath: wipPane.readPath
        onStepped: (bucket, path) => wipPane.chooseOnly(bucket, path)
        onLanded: (bucket, path, origPath) => wipPane.fileWalked(bucket, path, origPath)
    }
    /// Automation only: the walk itself, for a run that presses the arrows and reads where they landed (as
    /// `GraphPane.view`).
    readonly property alias filesWalk: fileWalk
    function clearChoice() {
        wipPane.chosenKeys = ({})
        wipPane.chosenCount = 0
        wipPane.anchorRow = -1
    }
    /// Returns whether the diff should follow the click: adding to a choice is about the choice.
    function applyClick(bucket, path, modifiers) {
        const key = bucket + ":" + path
        const row = wipPane.rowIndexOf(key)
        if (modifiers & Qt.ShiftModifier && wipPane.anchorRow >= 0) {
            wipPane.chooseRange(wipPane.anchorRow, row)
            return false
        }
        if (modifiers & Qt.ControlModifier) {
            // A fresh object every time: the rows follow this property, and assigning the same one back changes nothing
            // to follow.
            const next = ({})
            for (const k in wipPane.chosenKeys)
                next[k] = true
            if (next[key] === true)
                delete next[key]
            else
                next[key] = true
            wipPane.chosenKeys = next
            wipPane.chosenCount = Object.keys(next).length
            wipPane.anchorRow = row
            return false
        }
        const only = ({})
        only[key] = true
        wipPane.chosenKeys = only
        wipPane.chosenCount = 1
        wipPane.anchorRow = row
        return true
    }
    /// Where in the pane's numbering a key sits; -1 for a row no bucket is showing. **Each bucket is asked once** —
    /// a per-row walk would cross the bridge once per changed file on every click; a key shows in exactly one run, so
    /// the first owner is the answer.
    function rowIndexOf(key) {
        const named = wipPane.rowOfKey(key)
        const standing = wipPane.bucketPanes
        let before = 0
        for (let i = 0; i < standing.length; i++) {
            const at = standing[i].model.rowOfFileIn(named.bucket, named.fullName)
            if (at >= 0)
                return before + at
            before += standing[i].rows
        }
        return -1
    }
    /// Every file row between two places in the pane's numbering, ends included — a Shift-click's reach, **rows
    /// scrolled past included**.
    function chooseRange(from, to) {
        if (from < 0 || to < 0)
            return
        const lo = Math.min(from, to)
        const hi = Math.max(from, to)
        const next = ({})
        for (let i = lo; i <= hi; i++) {
            const key = wipPane.keyAt(i)
            if (key !== "")
                next[key] = true
        }
        wipPane.chosenKeys = next
        wipPane.chosenCount = Object.keys(next).length
    }

    // ---- the marks that come out together ---------------------------
    // The marks on every row a press would move come out as the pointer reaches one of them (デザイン規約
    // §バケツごとの一覧と、ペインの分け方). Held here, keyed like the choice, because delegates are recycled.
    property string stageHotKey: ""
    function showStageTools(bucket, path) {
        wipPane.stageHotKey = bucket + ":" + path
    }
    /// A row's mark under the hand, or the commit button, is asking for the line-ending card. The hand inside the card
    /// is the third asker, and `eolKeep` reads that one off the card itself.
    property bool eolAsked: false
    /// Names the file the line-ending card speaks for; "" lets it go. Automation calls it directly.
    function pointEol(path) {
        if (path === "") {
            // Not closed here: the hand may be walking into the card, which holds for a beat (`eolKeep`); the pointed
            // row goes with the card (`onClosed`).
            wipPane.eolAsked = false
            eolKeep.settle()
            return
        }
        wipPane.worktreeModel.pointEol(path)
        // Under the row, placed from the row: a row is a pane wide at most, so never far from the hand, and a pointer
        // and the automation land the card alike.
        const row = wipPane.rowFor(path)
        if (row === null)
            return
        const at = row.mapToItem(wipPane, 0, row.height)
        eolCard.path = path
        eolCard.notice = wipPane.pointedEolText
        eolCard.x = at.x + Theme.spaceMd
        eolCard.anchorY = at.y
        eolCard.above = false
        wipPane.eolAsked = true
        eolCard.open()
    }
    /// Puts the commit button's card out without a pointer, the way the rows' is put out.
    property bool pointAtCommit: false
    onPointAtCommitChanged: wipPane.settleCommitCard()
    /// **The warning re-asks too.** The card is placed by a function, and called on the pointer's edges alone it
    /// answers for the pane as the pointer found it — usually mid-write after `Stage all`, not warning yet. So the
    /// warning (`onCardAsked`) and its count call it as well.
    Connections {
        target: wipPane.workTree
        function onEolStagedCountChanged() { wipPane.settleCommitCard() }
    }
    /// The button is warning and the pointer (or a headless run) is on it.
    readonly property bool commitCardAsked:
        commitBlock.commitWarned && (commitBlock.commitPointed || wipPane.pointAtCommit)
    function settleCommitCard() {
        if (!wipPane.commitCardAsked) {
            commitRest.stop()
            // Settles rather than closes, as the rows' card does: the hand may be walking into it (`pointEol`).
            if (eolCard.path === "") {
                wipPane.eolAsked = false
                eolKeep.settle()
            }
            return
        }
        // It waits the pointer out (規約 §hover のツールチップ「補足は待ってから開く」). **Already out means only the
        // count moved**: swap the words in place — sitting the rest out again would take the card down under a reading
        // hand (規約「戻る手は即通す」). **A running rest is not restarted** when the count moves: the wait is of the
        // hand, and the hand has not moved.
        if (eolCard.opened && eolCard.path === "")
            wipPane.openCommitCard()
        else if (!commitRest.running)
            commitRest.restart()
    }
    // **It asks again when it runs out**: the pointer may have left or the warning stopped while it ran.
    Timer {
        id: commitRest
        interval: Metrics.tipDelayMs
        onTriggered: if (wipPane.commitCardAsked) wipPane.openCommitCard()
    }
    function openCommitCard() {
        // The button speaks for the whole index, so the words hold for all four cases at once: **"problems"**, since
        // only one case is a change, and **`may`**, since two are guesses from a sample (デザイン規約 §改行コードの警告).
        eolCard.path = ""
        eolCard.notice = wipPane.workTree.eolStagedCount === 1
            ? qsTr("1 staged file may have line-ending problems")
            : qsTr("%1 staged files may have line-ending problems").arg(wipPane.workTree.eolStagedCount)
        // Above the button: it is pinned to the pane's bottom edge, so under it is off the window.
        const at = commitBlock.commitSeat.mapToItem(wipPane, 0, 0)
        eolCard.x = at.x
        eolCard.anchorY = at.y
        eolCard.above = true
        wipPane.eolAsked = true
        eolCard.open()
    }
    // The one card both hovers open (one pointer, one card). Owned here: a row is recycled the moment it scrolls off.
    EolHoverCard {
        id: eolCard
        /// What the card hangs off, in pane coordinates, and which side of it the card is on.
        property real anchorY: 0
        property bool above: false
        /// **Bound** — assigned, it opens a card-height too low (rules-refs/app-ui.md の commit ボタンのカードの行).
        /// **Flush on either side**: the hand has to walk in to read and copy (規約 §hover のツールチップ).
        y: eolCard.above ? eolCard.anchorY - eolCard.height : eolCard.anchorY
        // What put the card out is forgotten only when the card itself goes: the row's mark stays lit while the card
        // stands, and the card's sentence is read off the pointed row.
        onClosed: {
            eolCard.path = ""
            wipPane.worktreeModel.pointEol("")
        }
    }
    // The beat between the card and its mark: the card goes only when the hand is out of both.
    HoverCardHost {
        id: eolKeep
        card: eolCard
        pointedAt: wipPane.eolAsked
    }
    /// The card itself is up. Read by the headless runs — reporting what asked for it would go green with the wiring
    /// cut.
    readonly property bool eolCardOpen: eolCard.opened
    /// What the named row is saying, for the headless report and the rows of all three lists — read off the unstaged
    /// model, where `pointEol` writes.
    readonly property string pointedEolPath: wipPane.worktreeModel.pointedEolPath
    readonly property string pointedEolKind: wipPane.worktreeModel.pointedEolKind
    readonly property string pointedEolFrom: wipPane.worktreeModel.pointedEolFrom
    readonly property string pointedEolTo: wipPane.worktreeModel.pointedEolTo
    readonly property int pointedEolLines: wipPane.worktreeModel.pointedEolLines
    readonly property string pointedEolScope: wipPane.worktreeModel.pointedEolScope
    readonly property string pointedEolExt: wipPane.worktreeModel.pointedEolExt
    readonly property string pointedEolText:
        wipPane.pointedEolKind !== ""
        ? Words.lineEndings(wipPane.pointedEolKind, wipPane.pointedEolFrom, wipPane.pointedEolTo,
                            wipPane.pointedEolLines, wipPane.pointedEolScope, wipPane.pointedEolExt)
        : ""
    /// Whether a row should put its mark out because the pointer is on the mark of another row that would move with it.
    /// Only rows on the same side answer: `+` stages what is not staged, `−` takes back what is.
    function stagePeerOf(bucket, path) {
        const key = bucket + ":" + path
        if (wipPane.stageHotKey === "")
            return false
        // The pointed row answers too: a real pointer already shows its mark, and this is how a headless run does.
        if (wipPane.stageHotKey === key)
            return true
        if (!wipPane.isChosen(bucket, path))
            return false
        const cut = wipPane.stageHotKey.indexOf(":")
        const hotBucket = wipPane.stageHotKey.substring(0, cut)
        if (!wipPane.isChosen(hotBucket, wipPane.stageHotKey.substring(cut + 1)))
            return false
        return (hotBucket === "staged") === (bucket === "staged")
    }
    /// The highlighted rows that would move with a press on `bucket`'s side — the whole of what one press takes. A row
    /// that was not highlighted takes only itself.
    function stageTargets(bucket, path) {
        if (!wipPane.isChosen(bucket, path))
            return [path]
        const out = []
        const rows = wipPane.chosenRows()
        for (let i = 0; i < rows.length; i++)
            if ((rows[i].bucket === "staged") === (bucket === "staged"))
                out.push(rows[i].fullName)
        return out.length > 0 ? out : [path]
    }
    /// A press on a row's `+` / `−`: every lit row that can go the same way moves with it, in one git command
    /// (デザイン規約 §バケツごとの一覧と、ペインの分け方).
    function moveStage(bucket, path) {
        const paths = wipPane.stageTargets(bucket, path)
        if (paths.length === 1) {
            if (bucket === "staged")
                wipPane.repoTab.unstagePath(paths[0])
            else
                wipPane.repoTab.stagePath(paths[0])
            return
        }
        wipPane.repoTab.beginPaths()
        for (let i = 0; i < paths.length; i++)
            wipPane.repoTab.addPath(paths[i])
        if (bucket === "staged")
            wipPane.repoTab.unstagePaths()
        else
            wipPane.repoTab.stagePaths()
    }

    // ---- the bucket headings, as the scene has them ------------------
    // Read off the headings themselves: an emptied bucket keeps its heading, and the whole of that claim is whether the
    // heading is in the scene (app-ui.md §UI 自動化).
    function bucketHeaded(section) {
        const standing = wipPane.bucketPanes
        for (let i = 0; i < standing.length; i++)
            if (standing[i].section === section)
                return standing[i].headed
        return false
    }
    /// Automation: press a bucket heading's whole-bucket button, where a hand presses it.
    function moveBucket(section) {
        const standing = wipPane.bucketPanes
        for (let i = 0; i < standing.length; i++)
            if (standing[i].section === section)
                return standing[i].moveAll()
        return false
    }
    /// The row a path is on — for the automation hooks, which enter a click where the row itself enters it. App code
    /// goes through the signals.
    function rowFor(path) {
        for (let i = 0; i < wipPane.rowCount(); i++) {
            const row = wipPane.rowAt(i)
            if (row && row.fullName === path)
                return row
        }
        return null
    }

    readonly property string subjectText: commitBlock.subjectText
    readonly property string bodyText: commitBlock.bodyText

    // -- the message a stopped merge already has --------------------
    //
    // A merge is the one operation finished from this seat, and git wrote its message when it stopped. The page puts
    // it in the boxes (`absorbOpMessage`); these are what the boxes fall back to when emptied — the placeholder, and
    // what the press commits (デザイン規約 §進行中の操作から出る).
    readonly property string standingSubject: wipPane.workTree.opMerging ? wipPane.workTree.opSubject : ""
    readonly property string standingBody: wipPane.workTree.opMerging ? wipPane.workTree.opBody : ""
    /// The boxes stand empty over one — **both** boxes: a description with no summary would commit a blank first
    /// line. **Outside amend mode**: the message belongs to the commit the merge is about to make, not to the one an
    /// amend replaces.
    readonly property bool onStandingMessage:
        !wipPane.amending && wipPane.standingSubject !== ""
        && commitBlock.subjectText.trim() === "" && commitBlock.bodyText.trim() === ""
    /// What a press would record. Read by the page, so the fallback is decided once.
    readonly property string outgoingSubject:
        wipPane.onStandingMessage ? wipPane.standingSubject : commitBlock.subjectText
    readonly property string outgoingBody:
        wipPane.onStandingMessage ? wipPane.standingBody : commitBlock.bodyText

    /// The label a stash made now would take: the typed summary, or empty for git's own `WIP on …` (デザイン規約
    /// §変更を退避する). **Not the standing merge message**, though `absorbOpMessage` put it in the box as text
    /// (rules-refs/app-ui.md「止まった merge の stash 名は git 任せ」), **nor in amend mode**, where the box holds an
    /// existing commit's message (`absorbHeadMessage`).
    ///
    /// The stash leaves the text in the box: the boxes are the one thing here not read back off disk, and a `pop`
    /// lands the work back under them. What a pop puts back is the entry's name, only into an empty box
    /// (`RepoPage.absorbPopLabel`).
    readonly property string stashName:
        !wipPane.amending && commitBlock.subjectText !== wipPane.standingSubject
        && GitFacts.validStashMessage(commitBlock.subjectText)
        ? commitBlock.subjectText : ""
    // Amend with the current identity as author (git keeps the original author otherwise).
    readonly property bool resetAuthor: commitBlock.resetAuthor
    // Both open at the editor's rest height and first line (`MessageEditor.setMessage`).
    function setMessage(subject, body) {
        commitBlock.setMessage(subject, body)
    }
    function clearMessage() {
        commitBlock.setMessage("", "")
    }
    function setAmendChecked(on) {
        commitBlock.setAmendChecked(on)
    }
    function setResetAuthorChecked(on) {
        commitBlock.setResetAuthorChecked(on)
    }
    /// Smoke hook: the caret in the description box, the way a click in it puts it there.
    function focusDescription() {
        commitBlock.focusDescription()
    }
    readonly property color descriptionColor: commitBlock.descriptionColor
    readonly property bool descriptionFocused: commitBlock.descriptionFocused

    // -- what this pane lends the message editor --
    /// What the block under the list may take before it scrolls: all of the pane but two rows of list, the grip's bound
    /// (デザイン規約 §コミットメッセージの 2 つの枠), and **the bucket frame** — a heading counted as one of the two rows
    /// leaves a pane at the window's floor nothing but headings. A clean tree owes the list nothing.
    readonly property real blockRoom:
        Math.max(0, wipPane.height - headerBand.height
                    - (wipPane.bucketsStanding ? wipPane.bucketFrame + 2 * Theme.rowHeight : 0))
    /// Moves the block by a wheel a box on it could not use (`WipCommitBlock.rollBlock`).
    function rollBlock(pixels) {
        commitBlock.rollBlock(pixels)
    }
    // -- smoke hooks and readouts, said under the pane's name because the automation reads the panes (MessageEditor) --
    function growDescription(dy) { commitBlock.growDescription(dy) }
    function pullDescriptionPast(down) { commitBlock.pullDescriptionPast(down) }
    /// Whether the grip is refusing a pull, and where — see `DetailsPane.descRefuses`.
    readonly property alias descRefuses: commitBlock.descRefuses
    readonly property alias descPoint: commitBlock.descPoint
    readonly property bool descGrips: commitBlock.descGrips
    readonly property real descHeight: commitBlock.descHeight
    readonly property real descWants: commitBlock.descWants
    readonly property real descCap: commitBlock.descCap
    readonly property int descListRows: commitBlock.descListRows
    /// Automation only: the block itself, handed whole rather than relayed per hand (as `DetailsPane.messageBlock`).
    readonly property alias commitBlock: commitBlock
    /// Whether anything in the block is below the fold, and its complement, for the headless runs
    /// (`PGG_AUTO_ACT=window-floor wip`).
    readonly property bool blockScrolls: commitBlock.blockScrolls
    readonly property bool descKeeps: commitBlock.descKeeps
    /// The bare bottom edge the page's corner text steps aside by — none: the commit block is pinned there and never
    /// empty (`DetailsPane.bottomRoom` is the one that answers).
    readonly property real bottomRoom: 0

    spacing: 0

    Rectangle {
        id: headerBand
        Layout.fillWidth: true
        implicitHeight: Theme.headerHeight
        color: Theme.bgElevated
        BandRule { z: 1 }
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.spaceXs
            anchors.rightMargin: Theme.spaceXs
            spacing: Theme.spaceXs
            Label {
                text: qsTr("UNCOMMITTED CHANGES")
                font.pixelSize: Theme.fontMd
                font.weight: Font.DemiBold
                color: Theme.textSecondary
            }
            // The count stands at the caption's own step, as in every heading band (see DetailsChangesBand's).
            Label {
                text: "(" + wipPane.worktreeModel.total + ")"
                font.pixelSize: Theme.fontMd
                color: Theme.textMuted
            }
            Item { Layout.fillWidth: true }
            TreeViewToggle {
                treeView: wipPane.worktreeModel.treeView
                onChosen: tree => wipPane.treeViewChosen(tree)
            }
        }
    }

    // One list per bucket, each scrolling in a share the pane hands out (`bucketSeats`). Placed by hand: the shares
    // are worked out from this item's height, and a layout sizing itself from its children would read that answer
    // back into its own question.
    Item {
        id: buckets
        Layout.fillWidth: true
        Layout.fillHeight: true
        visible: wipPane.bucketsStanding

        WipBucketPane {
            id: conflictsBucket
            section: "conflicts"
            // Read off its own list, like the heading's count: two feeds carry the same status, and reading one for
            // the frame and the other for the words opens a queue-order window where they disagree.
            visible: conflictsBucket.model.runFiles > 0
            pane: wipPane
            repoTab: wipPane.repoTab
            workTree: wipPane.workTree
            model: wipPane.conflictsModel
            tipRow: wipPane.pointedTipRow
            width: buckets.width
            y: 0
            height: conflictsBucket.headHeight + wipPane.bucketSeats[0]
        }
        WipBucketPane {
            id: unstagedBucket
            section: "unstaged"
            pane: wipPane
            repoTab: wipPane.repoTab
            workTree: wipPane.workTree
            model: wipPane.worktreeModel
            tipRow: wipPane.pointedTipRow - wipPane.conflictRows
            width: buckets.width
            // A hairline of ground between buckets: both headings wear `bgElevated`, and stacked flush an emptied
            // bucket and the next read as one band.
            y: conflictsBucket.visible ? conflictsBucket.height + Theme.borderWidth : 0
            height: unstagedBucket.headHeight + wipPane.bucketSeats[1]
        }
        WipBucketPane {
            id: stagedBucket
            section: "staged"
            pane: wipPane
            repoTab: wipPane.repoTab
            workTree: wipPane.workTree
            model: wipPane.stagedModel
            tipRow: wipPane.pointedTipRow - wipPane.conflictRows - unstagedBucket.rows
            width: buckets.width
            y: unstagedBucket.y + unstagedBucket.height + Theme.borderWidth
            height: stagedBucket.headHeight + wipPane.bucketSeats[2]
        }
    }

    // Holds the buckets' room on a clean tree: with nothing that can grow, the layout centres each item in an even
    // share of the leftover — the commit block adrift mid-pane. Standing where the files were, it keeps the block
    // still when the last change is committed.
    Item {
        Layout.fillWidth: true
        Layout.fillHeight: true
        visible: !wipPane.bucketsStanding
    }

    // Under the list, the block that scrolls on its own when the pane is too short (`WipCommitBlock`). The list keeps
    // two rows throughout (the grip's bound), so the two scrolling surfaces never share an edge: a wheel is over
    // exactly one of them.
    //
    // The hairline is the one every band closes with (`PaneHeader`): without it the list just stops a bare inset above
    // the summary box.
    Rectangle {
        Layout.fillWidth: true
        implicitHeight: Theme.borderWidth
        color: Theme.borderSubtle
    }
    WipCommitBlock {
        id: commitBlock
        Layout.fillWidth: true
        Layout.preferredHeight: Math.min(commitBlock.wants, wipPane.blockRoom)
        repoTab: wipPane.repoTab
        workTree: wipPane.workTree
        standingSubject: wipPane.standingSubject
        standingBody: wipPane.standingBody
        onStandingMessage: wipPane.onStandingMessage
        amending: wipPane.amending
        headPublished: wipPane.headPublished
        mateCount: wipPane.mateCount
        commitTarget: wipPane.commitTarget
        signingPointedAt: wipPane.signingPointedAt
        blockRoom: wipPane.blockRoom
        listHeight: buckets.height
        onAmendToggled: on => wipPane.amendToggled(on)
        onCommitClicked: wipPane.commitClicked()
        onCardAsked: wipPane.settleCommitCard()
    }
}
