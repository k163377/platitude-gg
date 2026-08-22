pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Right pane, working-tree (WIP) mode: the commit editor pinned on top of the grouped changed-file list, with
// stage/unstage affordances.
ColumnLayout {
    id: wipPane

    required property var repoTab
    required property var workTree
    /// One model per bucket run, each showing its own run and answering about the whole tree
    /// (`NavSectionModel::attach_worktree`). `worktreeModel` is the unstaged one and is also the pane's way of asking
    /// the tree a question — any of the three would answer the same.
    required property var worktreeModel
    required property var conflictsModel
    required property var stagedModel
    // Mirrored page state: whether the editor is in amend mode and whether HEAD is already on a remote (shows the
    // warning tag).
    property bool amending: false
    property bool headPublished: false
    /// Stands in for the pointer on a file row, so a cut-down paths-view row's tooltip can be photographed
    /// (PG_AUTO_ACT=path-tip). -1 points at no row.
    property int pointedTipRow: -1
    /// A right-click menu of the page's is standing over this pane. The
    /// menus are the page's, so only it can say (デザイン規約 §メニュー).
    property bool menuStanding: false
    /// Which working-tree file the middle pane is reading, handed down by the page: what the arrows walk from, and
    /// empty while the graph is in the middle (規約 §diff のファイル一覧). The light that says so is this list's own choice —
    /// see `readOne`.
    property string readBucket: ""
    property string readPath: ""

    /// Whoever the message credits as it stands, packed the way the details pane's line is fed
    /// (`encode::encode_co_authors`). Nothing has been committed, so the only place a trailer exists is the text.
    ///
    /// A binding over a slot: what it follows is the box's own `bodyText`, which is a property, so every keystroke
    /// re-asks — and the rule for what counts as a trailer stays in one place, on the Rust side (`co_authors_of`).
    readonly property string messageMates: wipPane.repoTab.coAuthorsOf(msgEditor.bodyText)
    /// How many of them there are — one record per person, counted where the record shape lives
    /// (`GitFacts.recordCount` / `encode::RECORD_SEP`).
    readonly property int mateCount: GitFacts.recordCount(wipPane.messageMates)
    /// Stands in for the pointer on the signing tick in the commit button — hover cannot be injected (`commit-face`).
    property bool signingPointedAt: false
    /// What it puts on screen, for the headless report: the tooltip's own visible (the output side).
    readonly property bool signingTipShown: commitButton.phraseSignatureTipShown
    /// Where a press would land it. Detached, git has left no branch to name and the chip says so in git's own word —
    /// naming nothing at all would read as "somewhere".
    readonly property string commitTarget:
        wipPane.workTree.detached ? "HEAD" : wipPane.workTree.branch

    signal amendToggled(bool on)
    signal commitClicked()
    signal fileActivated(string bucket, string path, string origPath)
    /// The arrows walked onto another file. Not the signal a click raises: a click on the file already open closes the
    /// diff, and holding Down must not (規約 §diff のファイル一覧).
    signal fileWalked(string bucket, string path, string origPath)
    /// Right-click on a file row; the page owns the menu because delegates are recycled out from under an open popup.
    signal fileMenuRequested(string bucket, string path)

    /// Tree or flat paths, for the whole pane. One choice, three lists: the toggle in the header band is about how the
    /// working tree is shown, not about one bucket of it.
    function setTreeView(tree) {
        wipPane.conflictsModel.setTreeView(tree)
        wipPane.worktreeModel.setTreeView(tree)
        wipPane.stagedModel.setTreeView(tree)
    }

    /// Automation: run one of the stopped operation's held rows to its end, named by its flag (`OpExitCard`).
    function completeOpExit(code) {
        return opExitCard.completeOpExit(code)
    }
    /// Automation: whether the card offers that row at all.
    function offersOpExit(code) {
        return opExitCard.offersOpExit(code)
    }
    /// Automation: press the button that finishes things, the way a click does.
    function pressCommit() {
        if (!commitButton.enabled)
            return false
        wipPane.commitClicked()
        return true
    }

    // ---- how the buckets share the pane ------------------------------
    // **Even shares, and a share a bucket cannot fill goes to the ones that can** (デザイン規約 §その他の操作). Three files
    // against three and three against six both come out even — in neither is there room going spare. One against six
    // does not: five of the six can be shown without taking anything the other bucket had a use for.

    /// Whether any bucket is standing. **A clean tree stands none of them** (デザイン規約 §その他の操作): the lists are
    /// empty throughout, and a `(0)` heads nothing.
    readonly property bool bucketsStanding: wipPane.worktreeModel.total > 0
    /// The buckets that are standing, top to bottom. The conflicted one is not there most of the time, and everything
    /// that reaches across the buckets — the choice, the walk, the automation — reads them from here.
    readonly property var bucketPanes:
        conflictsBucket.visible ? [conflictsBucket, unstagedBucket, stagedBucket]
                                : [unstagedBucket, stagedBucket]
    /// Rows above the unstaged bucket, which is what turns a row number spanning the pane into one of a bucket's own.
    readonly property int conflictRows: conflictsBucket.visible ? conflictsBucket.rows : 0
    /// What the buckets take before a single file row is shown: a heading each, and the hairline of ground between one
    /// bucket and the next. Neither is a bucket's to give up, so neither is in the room they share.
    readonly property real bucketFrame:
        wipPane.bucketPanes.length * Theme.rowHeight
        + (wipPane.bucketPanes.length - 1) * Theme.borderWidth
    /// The room the buckets have between them.
    readonly property real bucketRoom: Math.max(0, buckets.height - wipPane.bucketFrame)
    /// What each bucket's rows get, conflicted / unstaged / staged, whether or not it is standing.
    readonly property var bucketSeats: wipPane.shareOut(wipPane.bucketRoom, [
        conflictsBucket.visible ? conflictsBucket.wants : -1,
        unstagedBucket.wants,
        stagedBucket.wants])
    /// Shares `room` out between the buckets. `wants` is what each would take with nothing in its way, in the order
    /// they stand, and **-1 for a bucket that is not standing**; the answer is what each one gets, in the same order.
    ///
    /// Smallest want first: a bucket asking for less than an even share settles for what it asked, and what it did not
    /// take is shared out again among the ones still asking. Whatever is left once every bucket is satisfied goes to
    /// the last one standing, so the bare ground is at the foot of the pane — where one list would have left it.
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
        out[standing[standing.length - 1]] += left
        return out
    }

    // ---- which rows are chosen -------------------------------------
    // A plain click takes one, Ctrl adds or removes, Shift reaches from the last one clicked. Held here, keyed
    // `<bucket>:<path>`, because a delegate is recycled the moment its row scrolls off.
    property var chosenKeys: ({})
    property int chosenCount: 0
    /// The row the next Shift-click reaches from.
    property int anchorRow: -1
    function isChosen(bucket, path) {
        return wipPane.chosenKeys[bucket + ":" + path] === true
    }
    /// Rows in the order they stand, so a caller can act on what was chosen. The buckets are separate lists and one
    /// choice: a Ctrl-click reaches from one into the next, so every walk over the rows walks the lot of them.
    function chosenRows() {
        const out = []
        for (let i = 0; i < wipPane.rowCount(); i++) {
            const row = wipPane.rowAt(i)
            if (row && !row.folder && wipPane.isChosen(row.bucket, row.fullName))
                out.push(row)
        }
        return out
    }
    /// Rows across every standing bucket, and the one at a place in them — the numbering the choice is anchored and
    /// reached by, and the one a headless run names a row with.
    function rowCount() {
        const standing = wipPane.bucketPanes
        let out = 0
        for (let i = 0; i < standing.length; i++)
            out += standing[i].list.count
        return out
    }
    function rowAt(index) {
        const standing = wipPane.bucketPanes
        let at = index
        for (let i = 0; i < standing.length; i++) {
            const list = standing[i].list
            if (at < list.count)
                return list.itemAtIndex(at)
            at -= list.count
        }
        return null
    }
    /// Makes one row the whole of the choice — what a plain click does, and what a right-click outside the choice does
    /// before opening the menu (the menu acts on what is highlighted).
    function chooseOnly(bucket, path) {
        wipPane.applyClick(bucket, path, Qt.NoModifier)
    }
    /// One file is being read now, and it is not a click that said so: the side under the reader ran out and the pane
    /// went on to the next file (`RepoPage.followEmptySide`), or an arrow key walked there. The choice comes too — the
    /// lit row is the file being read, and a light left behind names one nobody is looking at (デザイン規約 §diff の
    /// ファイル一覧を矢印で送る).
    ///
    /// Nothing to do where it is already the whole of the choice, which is every ordinary click: that walk over the
    /// rows has just been made.
    function readOne(bucket, path) {
        if (wipPane.chosenCount !== 1 || !wipPane.isChosen(bucket, path))
            wipPane.chooseOnly(bucket, path)
    }

    // ---- walking the files with the arrow keys ------------------------
    // The light this list already had is the choice, so a step moves the choice; the reading follows a beat later (規約
    // §diff のファイル一覧).
    FileRowWalk {
        id: fileWalk
        // The buckets standing, in the order they stand: the arrows cross out of one list and into the next, so the
        // walk is over the lot of them (規約 §diff のファイル一覧).
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
    /// Automation only: the walk itself, for a run that has to press the arrows and read where they landed — the same
    /// kind of exposure `GraphPane.view` is (verify-ui).
    readonly property alias filesWalk: fileWalk
    function clearChoice() {
        wipPane.chosenKeys = ({})
        wipPane.chosenCount = 0
        wipPane.anchorRow = -1
    }
    /// Applies a click to the choice. Returns whether the diff should follow it: adding to a choice is about the
    /// choice, not about which file is being read.
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
    function rowIndexOf(key) {
        for (let i = 0; i < wipPane.rowCount(); i++) {
            const row = wipPane.rowAt(i)
            if (row && !row.folder && row.bucket + ":" + row.fullName === key)
                return i
        }
        return -1
    }
    function chooseRange(from, to) {
        if (from < 0 || to < 0)
            return
        const lo = Math.min(from, to)
        const hi = Math.max(from, to)
        const next = ({})
        for (let i = lo; i <= hi; i++) {
            const row = wipPane.rowAt(i)
            if (row && !row.folder)
                next[row.bucket + ":" + row.fullName] = true
        }
        wipPane.chosenKeys = next
        wipPane.chosenCount = Object.keys(next).length
    }

    // ---- the marks that come out together ---------------------------
    // A press on one row's `+` moves every highlighted row that can go the same way, so the marks on those rows come
    // out as soon as the pointer reaches one of them: what a press is about to move is seen before it is pressed
    // (デザイン規約 §その他の操作). Held here, keyed the way the choice is, because a delegate is recycled the moment its row
    // scrolls off. Automation writes it directly — hover cannot be injected.
    property string stageHotKey: ""
    function showStageTools(bucket, path) {
        wipPane.stageHotKey = bucket + ":" + path
    }
    /// Names the file whose line-ending sentence the hover should carry. The empty string clears it. Automation writes
    /// it directly, the same way it writes `showStageTools` — hover cannot be injected.
    function pointEol(path) {
        wipPane.worktreeModel.pointEol(path)
        if (path === "") {
            eolCard.close()
            return
        }
        // Under the row rather than under the pointer: these rows are a pane wide at most, so the two are never far
        // apart, and a card placed from the row lands in the same place whether a pointer or the automation named it.
        const row = wipPane.rowFor(path)
        if (row === null)
            return
        const at = row.mapToItem(wipPane, 0, row.height)
        eolCard.path = path
        eolCard.notice = wipPane.pointedEolText
        eolCard.x = at.x + Theme.spaceMd
        eolCard.anchorY = at.y
        eolCard.above = false
        eolCard.open()
    }
    /// Puts the commit button's card out without a pointer, the way the rows' is put out — hover cannot be injected.
    property bool pointAtCommit: false
    onPointAtCommitChanged: wipPane.settleCommitCard()
    /// **The pointer is not the only thing that moves.** This is placed by a function rather than by a binding — a card
    /// above its anchor needs a measured height, so it cannot be one — and a function only runs when something calls
    /// it. Called on the pointer's edges alone, it answers for the state the pane was in when the pointer arrived, and
    /// a pointer that arrives first is the ordinary case: someone presses `Stage all` and moves to the button while the
    /// index is still being written, so the button is disabled — and therefore not warning — for the whole of the only
    /// moment it would have been asked (2026-08-18 実測: the card never came). So the warning re-asks too, from both
    /// sides it can change on: whether it is warning at all, and how many files it is warning about.
    Connections {
        target: wipPane.workTree
        function onEolStagedCountChanged() { wipPane.settleCommitCard() }
    }
    function settleCommitCard() {
        if (!commitButton.eolWarned || !(commitHover.containsMouse || wipPane.pointAtCommit)) {
            if (eolCard.path === "")
                eolCard.close()
            return
        }
        // No one file to name: the button speaks for the whole index, and so has to hold for all four cases at once.
        // **Not "change"** — only one of them is a change. A new file has nothing to have changed from, a mixed one is
        // a file disagreeing with itself, and a file that never had an ending has only gained its first.
        //
        // **`may`, and it is doing work.** Two of the four are read off a sample of the neighbouring files, so the app
        // does not know they are problems — a new file deliberately written with the other ending is not one. Said
        // flatly, the summary would decide what each row's own card is careful not to.
        eolCard.path = ""
        eolCard.notice = wipPane.workTree.eolStagedCount === 1
            ? qsTr("1 staged file may have line-ending problems")
            : qsTr("%1 staged files may have line-ending problems").arg(wipPane.workTree.eolStagedCount)
        // Above the button, not under it: the button is pinned to the pane's bottom edge, so under it is off the
        // window (measured — the card's own top hairline was the last row of pixels in the shot).
        const at = commitButton.mapToItem(wipPane, 0, 0)
        eolCard.x = at.x
        eolCard.anchorY = at.y
        eolCard.above = true
        eolCard.open()
    }
    // The one card both hovers open: only one pointer, so only one of them is ever out. Owned here rather than by a
    // row, which is recycled the moment it scrolls off (app-ui.md).
    EolHoverCard {
        id: eolCard
        /// What the card hangs off, in pane coordinates, and which side of it the card is on.
        property real anchorY: 0
        property bool above: false
        /// **Bound, not assigned.** A popup handed its text is not its final height in that same frame (規約 §hover
        /// のツールチップ 「出す前に採寸する」), and a card placed *above* its anchor needs that height to be placed at
        /// all — assigned, it would open one card-height too low every time.
        y: eolCard.above ? eolCard.anchorY - eolCard.height - Theme.spaceXs : eolCard.anchorY
    }
    /// The card itself is up. Read by the headless runs — reporting what asked for it would go green with the wiring
    /// cut.
    readonly property bool eolCardOpen: eolCard.opened
    /// What the named row is saying — for the headless report to read, and for the rows themselves, which are in three
    /// lists and read the one answer. Asked of the unstaged model because the pointer is written there: the marks
    /// arrived on all three and the words come out of the path, so any of them would say the same.
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
        // The row the pointer is actually on answers too: under a real pointer it is already showing its mark, and this
        // is the only way a headless run can put that mark on screen (hover cannot be injected — verify-ui スキル).
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
    /// A press on a row's `+` / `−`: every highlighted row that can go the same way moves with it, and one git command
    /// carries the lot, whatever the count (デザイン規約 §その他の操作). The buckets are separate lists, so the press
    /// comes in from whichever of them the row is in.
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
    // Read off the headings themselves rather than off the conditions that put them there: an emptied bucket keeps its
    // heading, and the whole of that claim is whether the heading is in the scene (app-ui.md §UI 自動化の因果性).
    /// Whether a bucket has a heading on screen at all.
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

    readonly property string subjectText: msgEditor.subjectText
    readonly property string bodyText: msgEditor.bodyText

    // -- the message a stopped merge already has --------------------
    //
    // A merge is the one operation finished from this seat, and git wrote its message down when it stopped. The page
    // puts it in the boxes (`absorbOpMessage`); these say what the boxes fall back to when it is typed out of them —
    // the placeholder underneath, and the message the press commits (デザイン規約 §進行中の操作から出る).
    readonly property string standingSubject: wipPane.workTree.opMerging ? wipPane.workTree.opSubject : ""
    readonly property string standingBody: wipPane.workTree.opMerging ? wipPane.workTree.opBody : ""
    /// Whether the boxes are standing empty over one, which is the state that presses with nothing typed. Both boxes:
    /// a description with no summary is not "empty", and git would write a commit whose first line is blank.
    ///
    /// **Never in amend mode.** A merge's message belongs to the commit the merge is about to make, not to the one
    /// before it — and `git commit --amend` under `MERGE_HEAD` replaces that earlier commit with a merge commit,
    /// leaving what was there in the reflog alone. An amend goes on asking for a summary of its own.
    readonly property bool onStandingMessage:
        !wipPane.amending && wipPane.standingSubject !== ""
        && msgEditor.subjectText.trim() === "" && msgEditor.bodyText.trim() === ""
    /// What a press would record. Read by the page rather than the boxes themselves, so the fallback is decided once.
    readonly property string outgoingSubject:
        wipPane.onStandingMessage ? wipPane.standingSubject : msgEditor.subjectText
    readonly property string outgoingBody:
        wipPane.onStandingMessage ? wipPane.standingBody : msgEditor.bodyText

    /// The label a stash made now would take: the summary already standing in the box, when git would have it as one
    /// (デザイン規約 §変更を退避する). Empty means the entry goes unnamed, which is what git writes its own `WIP on …` for.
    ///
    /// **The typed line only, never a merge's own sentence.** That sentence is about the commit the merge is going to
    /// make, not a name the reader gave these changes — and it reaches the box two ways, as `outgoingSubject`'s
    /// fallback under an empty one *and as text*, put there by `absorbOpMessage` so the press can read it. So it is
    /// held against the standing message rather than against emptiness: a stash pressed over a stopped merge is a
    /// stash of the work, and the entry git names itself says more about it than a merge that no longer exists
    /// (2026-08-22 実測: the press takes `MERGE_HEAD` with it). One word typed over it makes it the reader's again.
    /// **And never in amend mode**, where the box is holding the message of a commit that already exists
    /// (`absorbHeadMessage`) — a stash called after it would be naming someone else's work.
    ///
    /// The text is not taken away by the write that reads it: the boxes are the one thing here that cannot be read
    /// back off disk (see `absorbOpMessage`), and a stash is undone by a `pop` that lands the work back in a tree
    /// this pane is already describing. What a pop does put back is the *entry's* name, and only into an empty box
    /// (`RepoPage.absorbPopLabel`).
    ///
    /// Asked of core per keystroke, the same pure function the rename box asks — one line with something on it, and no
    /// rule of this pane's own on top (規約 §同名).
    readonly property string stashName:
        !wipPane.amending && msgEditor.subjectText !== wipPane.standingSubject
        && wipPane.repoTab.validStashMessage(msgEditor.subjectText)
        ? msgEditor.subjectText : ""
    // Whether the amend should also put the current identity on the commit it replaces (git keeps the original author
    // otherwise).
    readonly property bool resetAuthor: authorBox.checked
    // Swapping in HEAD's message (amend) and clearing after a commit both open at the editor's own rest height and
    // first line — the caret treatment is the editor's (MessageEditor.setMessage).
    function setMessage(subject, body) {
        msgEditor.setMessage(subject, body)
    }
    function clearMessage() {
        msgEditor.setMessage("", "")
    }
    function setAmendChecked(on) {
        amendBox.checked = on
    }
    function setResetAuthorChecked(on) {
        authorBox.checked = on
    }
    /// Smoke hook: the caret in the description box, the way a click in it puts it there (see DetailsPane — same box,
    /// same reason).
    function focusDescription() {
        msgEditor.focusDescription()
    }
    readonly property color descriptionColor: msgEditor.descriptionColor
    readonly property bool descriptionFocused: msgEditor.descriptionFocused

    // -- what this pane lends the message editor --
    //
    // The pair's own geometry lives in `MessageEditor`; what this pane owns is what stands around it. The file list is
    // the only thing that gives, and the checkboxes, the commit button and the exit card keep their own height by
    // construction, so two rows of list is the whole of the bound (デザイン規約 §コミットメッセージの 2 つの枠).
    /// How much of the pane the block under the list may take: all of it but the two rows that keep a list a list — the
    /// same bound the grip stops at (デザイン規約 §コミットメッセージの 2 つの枠). Past this the block scrolls rather than running out of
    /// the pane's bottom.
    ///
    /// The buckets' own frame comes off first: a heading is not one of the two rows, and left in, a pane squeezed to
    /// the window's floor would keep nothing but headings — two places named and no file under either of them.
    ///
    /// A clean tree stands no bucket at all, and neither the frame nor the two rows are owed to a list that is not
    /// there: what the block may take is everything under the heading band.
    readonly property real blockRoom:
        Math.max(0, wipPane.height - headerBand.height
                    - (wipPane.bucketsStanding ? wipPane.bucketFrame + 2 * Theme.rowHeight : 0))
    /// Moves the block by a wheel a box on it could not use. The boxes cover most of the block, so without this the
    /// surface they stand on has no way to be reached by wheel at all (2026-08-09 ユーザー報告: the description box's own
    /// scrolling swallowed it and the block would not go down).
    function rollBlock(pixels) {
        const max = Math.max(0, blockScroll.contentHeight - blockScroll.height)
        // Taken away, not added: `pixels` is how far the wheel wanted the content to travel, and content travels
        // against `contentY` — the same subtraction the box makes on its own text. Added, the block went the other way,
        // which is a wheel that scrolls up and drags the surface down under it (2026-08-09 ユーザー報告).
        blockScroll.contentY = Math.max(0, Math.min(max, blockScroll.contentY - pixels))
    }
    // -- smoke hooks and readouts, said under the pane's name because the automation reads the panes (MessageEditor) --
    function growDescription(dy) { msgEditor.growDescription(dy) }
    function pullDescriptionPast(down) { msgEditor.pullDescriptionPast(down) }
    /// Whether the grip is refusing a pull, and where — see `DetailsPane.descRefuses`.
    readonly property alias descRefuses: msgEditor.descRefuses
    readonly property alias descPoint: msgEditor.descPoint
    readonly property bool descGrips: msgEditor.descGrips
    readonly property real descHeight: msgEditor.descHeight
    readonly property real descWants: msgEditor.descWants
    readonly property real descCap: msgEditor.descCap
    readonly property int descListRows: msgEditor.descListRows
    /// Whether anything in the block is below the fold, and its complement — the editor holds the numbers, this pane
    /// says them out loud for the headless runs (`PG_AUTO_ACT=window-floor wip`).
    readonly property bool blockScrolls: msgEditor.blockScrolls
    readonly property bool descKeeps: msgEditor.descKeeps
    /// How much of the pane's bottom edge is left bare, for the corner text the page hangs there to step aside by.
    /// Nothing is: the commit block is pinned to that edge and is never empty — a summary box stands there on a clean
    /// tree with nothing to commit. So this pane never lends the corner a seat, and the version it names is read off
    /// the commit-details pane instead (`DetailsPane.bottomRoom`).
    readonly property real bottomRoom: 0

    spacing: 0

    Rectangle {
        id: headerBand
        Layout.fillWidth: true
        implicitHeight: Theme.headerHeight
        color: Theme.bgElevated
        // The hairline every pane header closes with (see PaneHeader).
        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: Theme.borderWidth
            color: Theme.borderSubtle
            z: 1
        }
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
            // The count stands at the caption's own step, the way every heading band in the window carries its own
            // (NavHeader, CommandsPane, WipBucketHeader) — see DetailsPane's for why the step is not what says it is
            // a count.
            Label {
                text: "(" + wipPane.worktreeModel.total + ")"
                font.pixelSize: Theme.fontMd
                color: Theme.textMuted
            }
            Item { Layout.fillWidth: true }
            TreeViewToggle {
                treeView: wipPane.worktreeModel.treeView
                onChosen: tree => wipPane.setTreeView(tree)
            }
        }
    }

    // The files first, and the editor under them (デザイン規約 §コミットメッセージの 2 つの枠): what is being
    // committed is read before what it will be called, and the button that does it is the last thing on the way down.
    //
    // **One list per bucket, not one list of every bucket** (デザイン規約 §その他の操作): stacked end to end, a working
    // tree with seventy unstaged files pushes the staged ones off the bottom of the pane, and the side a commit is
    // actually made of is the side nobody can see. Each bucket scrolls inside a share of its own, and the shares are
    // the pane's to hand out (`bucketSeats`).
    //
    // No question bar over these lists: what a file row throws away is held down on the menu row that names it, where
    // the hand already is (デザイン規約 §長押し).
    //
    // Placed rather than laid out: what each bucket gets is worked out from the room they stand in, and a layout that
    // sized itself from its children would be reading that answer back out of its own question.
    Item {
        id: buckets
        Layout.fillWidth: true
        Layout.fillHeight: true
        // On a clean tree there is nothing to head: a lone `(0)` above `(0)` heads nothing (デザイン規約 §その他の操作).
        visible: wipPane.bucketsStanding

        WipBucketPane {
            id: conflictsBucket
            section: "conflicts"
            // The one bucket that comes and goes: git is not in the middle of anything most of the time, and a heading
            // for a state the tree is not in is a place nothing can ever land (デザイン規約 §その他の操作). Read off the
            // bucket's own list, like the heading's count — two feeds carry the same status, and reading one for the
            // frame and the other for the words opens a queue-order window where they disagree.
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
            // The hairline of the pane's own ground between one bucket and the next (2026-08-17 ユーザー指示): both
            // headings wear `bgElevated`, so an emptied bucket stacked straight onto the one below reads as one band
            // with two lines of text in it rather than as two buckets, one of which is empty.
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

    // The room the buckets were holding, on a clean tree where they stand down. Something has to go on holding it, or
    // the pane has nothing in it that can grow: a layout whose items are all at their own height shares what is left
    // over evenly between them and centres each one in its share, which is a heading band hanging off the pane's top
    // edge and a commit block adrift in the middle of it (2026-08-22 ユーザー報告). Bare ground, and it stands where the
    // files would have — the block below it does not move when the last change in the tree is committed.
    Item {
        Layout.fillWidth: true
        Layout.fillHeight: true
        visible: !wipPane.bucketsStanding
    }

    // Everything under the file list, pinned to the pane's bottom, in a surface of its own that scrolls when the pane
    // is too short to hold it.
    //
    // What is in here keeps its height by construction — the editor, the commit button, the exit card a stopped
    // operation puts up — so the list was the only thing that could give, and past zero the rest was simply laid out
    // below the pane's own edge (measured 2026-08-09: in a 320px window a stopped rebase drew `--continue` and `--skip`
    // and left `--quit` and `--abort` under the window, with nothing to scroll to reach them). The window's floor
    // cannot answer that on its own: the card comes and goes with what git is in the middle of, and a window that grew
    // itself because a rebase stopped would be a stranger thing than a pane that scrolls (規約 §窓の床).
    //
    // The list keeps two rows throughout, which is the bound the description box's grip already stops at, so the two
    // scrolling surfaces never share an edge: a wheel is over exactly one of them.
    //
    // The hairline is the one every band in the app closes with (see `PaneHeader`): without it the last file row and
    // the summary box's own frame stand a bare inset apart, and a list that simply stops has no edge saying where it
    // stopped.
    Rectangle {
        Layout.fillWidth: true
        implicitHeight: Theme.borderWidth
        color: Theme.borderSubtle
    }
    Flickable {
        id: blockScroll
        Layout.fillWidth: true
        Layout.preferredHeight: Math.min(blockCol.implicitHeight, wipPane.blockRoom)
        contentWidth: width
        contentHeight: blockCol.implicitHeight
        clip: true
        // Hard stop at the ends, as everywhere else that scrolls (デザイン規約 §QML 実装ルール).
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: AutoScrollBar {}
        ColumnLayout {
            id: blockCol
            width: blockScroll.width
            spacing: 0

            // Message editor — identical shape in commit details, amend and new-commit creation, inset
            // the same way (see DetailsPane) so switching modes doesn't move the box.
            ColumnLayout {
                Layout.fillWidth: true
                Layout.margins: Theme.spaceXs
                // The gutter this block's scroll bar is drawn in — see DetailsPane, which insets the same pair the
                // same way.
                Layout.rightMargin: Theme.spaceSm
                spacing: Theme.spaceXs
                // The pair is one block of one height, in the same component the details pane reads messages in (デザイン規約
                // §コミットメッセージの 2 つの枠).
                MessageEditor {
                    id: msgEditor
                    Layout.fillWidth: true
                    Layout.preferredHeight: msgEditor.pairHeight
                    standingSubject: wipPane.standingSubject
                    standingBody: wipPane.standingBody
                    listHeight: buckets.height
                    blockRoom: wipPane.blockRoom
                    blockHeight: blockCol.implicitHeight
                    onWheelPastEnd: pixels => wipPane.rollBlock(pixels)
                }
                // Amend replaces the newest commit instead of adding one, so it starts from that commit's message
                // rather than an empty editor.
                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spaceXs
                    AppCheckBox {
                        id: amendBox
                        text: qsTr("Amend the last commit")
                        font.pixelSize: Theme.fontSm
                        onToggled: wipPane.amendToggled(checked)
                    }
                    Item { Layout.fillWidth: true }
                    // Said, not asked: rewriting a pushed commit is undone by a switch or a reset, so the amend goes
                    // ahead and this tag is all the warning it gets.
                    Label {
                        visible: wipPane.amending && wipPane.headPublished
                        text: qsTr("already pushed")
                        color: Theme.warning
                        font.pixelSize: Theme.fontSm
                        ToolTip.visible: amendPushedHover.containsMouse
                        ToolTip.delay: Metrics.tipDelayMs
                        ToolTip.text: qsTr("Already on a remote — anyone who has it will be out of step")
                        MouseArea {
                            id: amendPushedHover
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.NoButton
                        }
                    }
                }
                // git records who committed, but leaves the author alone: an amend of someone else's commit — or of
                // one's own made under a different name — keeps the name it had. Offered only where the two
                // identities actually differ, and unchecked again whenever it goes away.
                //
                // **On its own line, under the box it depends on.** Beside it there is no room: the three words the
                // amend row can be carrying at once want about 392px between them, which is past the 400 this pane
                // opens at and well past the 300 it can be dragged to — and none of the three elides, so what went
                // over came off the tag at the end. Stacking is also how a dependent option reads: one under the box
                // it hangs off, at this spacing and with no indent.
                AppCheckBox {
                    id: authorBox
                    visible: wipPane.amending && wipPane.repoTab.headAuthorDiffers
                    text: qsTr("Make me the author")
                    font.pixelSize: Theme.fontSm
                    onVisibleChanged: if (!visible) checked = false
                    ToolTip.visible: hovered
                    ToolTip.delay: Metrics.tipDelayMs
                    ToolTip.text: qsTr("Replaces the author %1 <%2> with you, dated now")
                                  .arg(wipPane.repoTab.headAuthorName)
                                  .arg(wipPane.repoTab.headAuthorEmail)
                }
                // The framed button the rest of the app uses, at the size this pane needs. Its frame, its tone and
                // its `!` are the ones the toolbar's buttons already wear, so the state this button can be in is
                // said in the vocabulary someone has read elsewhere — rather than in a filled face that only this
                // one button has.
                ActionButton {
                    id: commitButton
                    Layout.fillWidth: true
                    // **The one button in the pane that is the pane's own action**, and the only one told to fill a
                    // width — so it is measured at the step above the controls beside it (`fontLg`, the size a
                    // commit's summary is typed at) in the band the toolbar's own row stands at. A control-height
                    // box around this wording read as a strip rather than as the thing being reached for
                    // (2026-08-18 報告).
                    font.pixelSize: Theme.fontLg
                    implicitHeight: Theme.toolbarHeight
                    /// Something staged says its line endings changed, so the commit is about to carry it. **Only
                    /// the index counts** — a file marked on its working-tree side is not in this commit
                    /// (`session::EolMark::staged`).
                    readonly property bool eolWarned:
                        wipPane.workTree.eolStagedCount > 0 && commitButton.enabled
                    // The other side a resting pointer's card can change under: the frame and the `!` are bindings and
                    // follow this on their own, the card has to be asked (`settleCommitCard`).
                    onEolWarnedChanged: wipPane.settleCommitCard()
                    // **No mark beside the word.** The label already names the command and the branch it lands on,
                    // which is two things to read; a tick in front of them says nothing a reader did not already
                    // have, and the one mark this button does need — the `!` — has to stand out from it
                    // (2026-08-18 ユーザー判断).
                    centred: true
                    // **The word stays plain in both states.** A coloured word is what `Remove` and a stopped fetch
                    // wear, and both of those are held rather than clicked; this one is a click either way, and
                    // borrowing their colour for the word would borrow the gesture with it. The frame and the mark
                    // carry the state instead.
                    tone: Theme.textPrimary
                    // **The frame goes with the words.** A toolbar button is measured to its content, so a frame
                    // left bright around dimmed words still reads as a small live thing; one this wide reads as a
                    // live button with grey words in it.
                    frameColor: !commitButton.enabled ? Theme.borderDefault
                                : commitButton.eolWarned ? Theme.warning : Theme.accent
                    // The mark the button family already puts at the end of its own word when the thing it does
                    // needs reading first.
                    alert: commitButton.eolWarned
                    alertTone: Theme.warning
                    alertTight: true
                    // **The command, and where it lands.** Both ends of the phrase are things git spells, so both
                    // wear the chip that says so (デザイン規約 §git 用語のコード表記); the words between them are
                    // the app's own and stay plain and translatable. The branch takes the accent, because the one
                    // thing a reader can be wrong about here is which branch they are on — it is a switch away
                    // from being another.
                    phraseHead: wipPane.amending ? "commit --amend" : "commit"
                    phraseTail: wipPane.commitTarget
                    phraseTailTint: commitButton.enabled ? Theme.accent : Theme.textMuted
                    // **And whom it will be attributed to.** The face ends the sentence the button is: the command,
                    // what goes, where it lands, by whom — which is the whole of what a commit records, and the
                    // one question the editor no longer has a row of its own for (デザイン規約 §アバターを与える).
                    // `+N` when the message credits others, the same mark the graph's chips and the details pane's
                    // credit line use; it follows the text being typed, so it appears as the trailer is written.
                    phraseFace: wipPane.repoTab.authorAvatar
                    phraseFaceUrl: wipPane.repoTab.authorAvatarUrl
                    phraseMates: wipPane.mateCount
                    // A tick on that face when signing is on, never the green one: green is git's word that a
                    // signature held, and nothing has been signed yet (規約 §署名の表示). What it is worth saying
                    // is that a press may put a passphrase prompt on screen — the passphrase is the agent's, and a
                    // commit that stops there has no reason on screen for having stopped.
                    phraseSignature: wipPane.repoTab.signsCommits ? "signed" : ""
                    phraseSignaturePointedAt: wipPane.signingPointedAt
                    phraseSignatureTip: wipPane.repoTab.signingFormat === "ssh"
                                        ? qsTr("Signed with your ssh key")
                                        : wipPane.repoTab.signingFormat === "x509"
                                          ? qsTr("Signed with your x509 certificate")
                                          : qsTr("Signed with your gpg key")
                    // **What goes, and where it lands.** The count is of files rather than of anything git would
                    // call a change: it is the number the list above is showing, so the button and the list agree
                    // without the reader converting between them. An amend with nothing staged is a message-only
                    // rewrite and says so by naming no count at all.
                    // The count stands apart from the words so the words can give way without taking it: how many
                    // files go is the one number here, and it costs nothing to keep.
                    phraseCount: wipPane.workTree.stagedCount === 0
                                 ? "" : wipPane.workTree.stagedCount.toString()
                    // With no count there is no noun for it to count: a message-only amend and a merge whose
                    // resolution records nothing both press with nothing staged, and `commit files to main` names a
                    // quantity that is not there.
                    text: wipPane.workTree.stagedCount === 0 ? qsTr("to")
                          : wipPane.workTree.stagedCount === 1 ? qsTr("file to") : qsTr("files to")
                    // An amend can stand on its own (message only); a new commit needs staged content and a
                    // summary, and git needs an identity to attribute either one to.
                    //
                    // A file still waiting on a decision stops it: git will not write a commit over an index that
                    // holds unmerged paths, whatever is staged beside them (デザイン規約 §可否・警告の出し場所).
                    //
                    // **A stopped merge asks for neither of the other two.** It carries its own message, so an
                    // empty box is not an empty commit message; and a merge whose resolution records nothing at
                    // all still has to be finished, so nothing staged is not nothing to do — git writes the empty
                    // merge commit either way (実測 2.55). This seat is the whole way out of a merge
                    // (§進行中の操作から出る), and a way out that will not press is not one.
                    enabled: wipPane.repoTab.busyCount === 0 && wipPane.repoTab.identityReady
                             && wipPane.workTree.conflictCount === 0
                             && (wipPane.onStandingMessage
                                 || (msgEditor.subjectText.trim() !== ""
                                     && (wipPane.amending || wipPane.workTree.stagedCount > 0
                                         || wipPane.standingSubject !== "")))
                    // **Still one click.** Committing is a daily operation and a confirmation on a daily operation
                    // becomes something people press without reading, which spends the effect where it is really
                    // needed (デザイン規約 §可否・警告の出し場所). The frame, the mark and the hover say what is in
                    // it; the decision stays the reader's.
                    onActivated: wipPane.commitClicked()
                    // Why it cannot be pressed. The other thing this button has to say — that the index carries a
                    // line-ending change — is said by the card `settleCommitCard` opens instead, and the two cannot
                    // both be true: the warning wants a button that can be pressed.
                    ToolTip.visible: commitHover.containsMouse && !enabled
                    ToolTip.delay: Metrics.tipDelayMs
                    ToolTip.text: !wipPane.repoTab.identityReady
                                  ? qsTr("No name or email set for commits")
                                  : wipPane.workTree.conflictCount > 0
                                  ? qsTr("Files are still waiting on a decision")
                                  : msgEditor.subjectText.trim() === ""
                                  ? qsTr("A commit needs a summary")
                                  : qsTr("Stage something to commit")
                    MouseArea {
                        id: commitHover
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.NoButton
                        onContainsMouseChanged: wipPane.settleCommitCard()
                    }
                }
                // The way out of a stopped operation, under the button that finishes things (`OpExitCard`) — the file
                // list moves down to make room for it rather than the rows covering the list.
                OpExitCard {
                    id: opExitCard
                    Layout.fillWidth: true
                    Layout.topMargin: Theme.spaceXs
                    repoTab: wipPane.repoTab
                    workTree: wipPane.workTree
                }
            }
        }
    }
}
