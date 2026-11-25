pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui
import platitude.ui.input
import platitude.ui.words

// **A focus scope**, so Escape stays answerable: a pane swapped off the screen lets the keyboard go
// (規約 §矢印で履歴を辿る), and in a plain item focus then falls out to the window's content item, off this page's key
// path — Escape goes quiet until the next press (`tests/qml/tst_escape.qml`'s `test_six` / `test_seven`). As a scope,
// the page catches the fall itself. Only the front tab has a page (`RepoPageStack`'s `Loader.active`), so no page
// behind holds the keys; a press that walks a caret away hands the keyboard here too (`FocusRelease.home`, which
// `Main` sets to `curPage`). Headless: `diff-escape`'s `hand=true kept=true`.
//
// **Not split further** — past the length backstop, exempt on the ledger (rules-refs/structure.md §分割しない判断).
// Out are `PageLayout`, `CommitMenuState`, `NameVerdict`, the question flows (`LeaveFlow` / `UpstreamFlow` /
// `RenameCarryFlow` / `PublishFlow`), `RowHoverHost`, `SplitBarWatch`, `CommandsOwner`, `GitHubLinks`, `RecoverEntries`
// and the harness (`PageHarness`). Next out is the discard log (`recover*`, `DiscardModel`): a part handed the page as
// `PageLayout` is, but an `Item` (its timer, blink and `Connections` go with it), once the page's writes into its state
// are calls into it — `absorbWriteResult` clears `recoverAwaits`, `leaveWorktree` puts the pick and its walk down —
// and what `RecoverPane` reads off `curPage` (`menuStanding` with it) is the part's to hand on. Past it is one tab's
// state machine: following the selection, opening and closing the diff, settling write results and owning the models,
// meshed through `selectedOid` / `repoTab`, which moves only with the redesign in which only the page writes page
// state (a child writes the page only as `PageLayout` does). The redesign goes one operation at a time (the shape is in
// `ops/mod.rs`). Answered by their own id already: the commit, the stash that empties the tree, the pop, a worktree's
// add and move, the pushes, the branch's plain delete (the page's `absorb*` for each), with what a delete or a stage
// shows ahead of git held in core (`ops::StandIn` / `StageAhead`). Left: following the selection and the diff's open
// and close, which have no owner of their own, and the writes nobody waits for by name (`absorbLeftoverAnswer`;
// P3-確認事項, the write group). When it is done, take this file off the ledger and pin it in
// `crates/xtask/structure-baseline.txt`.
FocusScope {
    id: page
    required property int index
    /// The tab this page stands on (`Hub`'s key for its session); undefined on the blank page, which stands on none.
    required property var tab_id
    readonly property bool blank: tab_id === undefined

    /// The blank page's "Open repository…" button (folder picker).
    signal openRepositoryPicker()
    /// A worktree asked for by path (a WORKTREES row, a press on a branch another worktree holds — `switchToRef`): a
    /// worktree of this repository stands this tab in it (デザイン規約 §タブの所作「同じリポジトリのタブは 1 枚」).
    signal openRepositoryPathRequested(string path)
    /// _PGG_AUTO_ACT=settings wants the window's settings dialog open for the screenshot.
    signal settingsDialogRequested()
    /// A conflicted file has nowhere to be opened: the git settings screen is where the merge editor is named.
    signal gitSettingsRequested()
    /// The settings card, opened from an avatar and carrying whom it was opened on.
    signal avatarSettingsRequested(string name, string email)
    /// _PGG_AUTO_PERF completion after every requested measurement output.
    signal perfFinished()
    /// The failed-open screen's "Close tab" button.
    signal closeTabRequested()
    /// The worktree this tab is standing in would not open, and the repository has its own one to stand in
    /// instead (`RepoTab.standHomeAsked`). The strip is what moves the tab.
    signal standHomeRequested()

    /// What the right pane is reading, keyed as the graph tells its rows apart (`GraphModel.keyAt` / `rowOfKey`): a
    /// commit (`kind` `commit`, its `oid`), this window's uncommitted changes (`here`) or another worktree's
    /// (`worktree`, its `path`); undefined while a landing is owed. **Every landing writes it through `readRow`**, and
    /// `selectedOid` / `wipShown` / `carriedPath` are read off it, so the pane is about one thing at a time.
    property var readKey: undefined
    readonly property string selectedOid: page.readKey !== undefined && page.readKey.kind === "commit"
                                          ? page.readKey.oid : ""

    /// The reader's fold, which is remembered (and the diff's and the plan's, given back below). What stands on screen is
    /// `listFolded`.
    property bool sidebarCollapsed: false
    /// The list on its rail: folded by the reader, or for the room — open by the reader in a window with no room for
    /// it (`PageLayout.listCrowded`), unless the reader asked it open there (`listHeldOpen`).
    readonly property bool listFolded: page.sidebarCollapsed || (pageLayout.listCrowded && !page.listHeldOpen)
    /// The reader unfolded the list in a window with no room for it (`foldByHand`): it stays open, the middle giving
    /// way (`PageLayout.centreHeldWidth`), until the window has room for it again — across tabs too, handed over as the
    /// log's flag is (`PageLayout.reportLayout` / `applySavedLayout`).
    property bool listHeldOpen: false
    readonly property bool listCrowded: pageLayout.listCrowded
    onListCrowdedChanged: if (!page.listCrowded) page.listHeldOpen = false
    /// The fold the diff put on: closing the diff takes back only that; a fold by hand stays.
    property bool foldedByDiff: false
    /// The reader's own fold — not the diff's nor the plan's, which give theirs back (`PageLayout.foldedForOther`).
    readonly property bool foldedByHand: page.sidebarCollapsed && !page.foldedByDiff && !page.foldedByPlan
    function foldForDiff(open) {
        if (open) {
            if (page.sidebarCollapsed)
                return
            page.sidebarCollapsed = true
            page.foldedByDiff = true
        } else if (page.foldedByDiff) {
            page.sidebarCollapsed = false
            page.foldedByDiff = false
        }
    }
    /// The same for the plan (規約 §フル interactive rebase; folding to the rail and giving back only its own fold is a
    /// user decision — ask before changing). **The list comes back when the plan goes**, run included —
    /// the reader is back on the graph watching the rewrite land, and the doors stay held without the fold
    /// (`doorsHeldWhy`). No fold by hand meanwhile: the rail's control is inside the frozen pane (`sidebarFrozen`).
    property bool foldedByPlan: false
    function foldForPlan(open) {
        if (open) {
            if (page.sidebarCollapsed)
                return
            page.sidebarCollapsed = true
            page.foldedByPlan = true
        } else if (page.foldedByPlan) {
            page.sidebarCollapsed = false
            page.foldedByPlan = false
        }
    }
    /// A plan's run is handed over and its replay not started — `replaying` rises only when the queue *starts* the
    /// write, so the hold begins here at the press. **Not a `writeSeq` wait**: the timer's fetch may answer first and
    /// would free the doors with the rebase still to come. Let go once `replayRunning` takes over.
    property bool planRunOut: false
    readonly property bool replayRunning: repoTab.replaying || page.autoReplayHeld
    onReplayRunningChanged: if (page.replayRunning) page.planRunOut = false
    /// Automation (`AutoActNavVerbs`, _PGG_AUTO_ACT=doors-held): a replay this run really started, held until its
    /// picture is taken (rules-refs/app-ui.md「一瞬だけ立つ状態は signal で観測して latch する」).
    property bool autoReplayHeld: false
    /// **The left pane is out two different ways** (デザイン規約 §フル interactive rebase). This one is the plan's: the
    /// whole pane but the `>_` band goes to the disabled step, from the press (`SidebarPane.frozen`).
    readonly property bool sidebarFrozen: page.planShown
    /// The other, from where the plan ends: a replay is running (`RepoTab.replaying`) or its run is out and not
    /// started — and while a worktree's rename is out (`worktreeMoveOut`). Nothing freezes; each door is held and
    /// says this line until git answers (`SidebarPane.doorsHeld` / `AppMenu.heldReason`) — one line, so the pane
    /// and the menus cannot disagree.
    readonly property string doorsHeldWhy:
        page.replayRunning || page.planRunOut || page.worktreeMoveOut ? Words.otherCommandRunning : ""
    /// A worktree's rename is out, from the press to its answer (`absorbWorktreeMove`): its answer is waited for by
    /// name, one at a time, and the session of a tab standing in that worktree has stood down for it
    /// (`RepoSession::move_worktree`) — nothing else may be written meanwhile.
    property bool worktreeMoveOut: false
    /// What a menu's rows say while they wait for a git command this window asked for (`AppMenu.runningWhy`): until
    /// what it left has been read again and the models hold it (`RepoTab.settling`) and the doors are let go (a plan's
    /// run is asked for elsewhere, and counted only once it starts). A menu opened under one stands again on that
    /// reading the moment it ends. `busyCount` alone ends too early: it drops at git's answer, before the reading.
    /// Not `moveLanding`: a move's landing is inside the reading again already, and one that failed to come down would
    /// keep every menu waiting.
    readonly property string runningWhy:
        repoTab.busyCount > 0 || repoTab.settling || page.doorsHeldWhy !== "" ? Words.otherCommandRunning : ""
    /// The worktree as it stood when the command the menus wait for was asked: what a menu raised under it stands its
    /// rows on (`CommitMenuState.tree`, `RefRowMenu.tree`, `FileRowMenu.tree`), whatever the write. The poll reads
    /// straight through the writes that replay — a rebase's detached HEAD and `REBASING` partway, the `MERGE_HEAD` /
    /// `CHERRY_PICK_HEAD` a merge or a cherry-pick leaves while its commit is made — and rows standing on that are not
    /// the rows the menu has with nothing running (デザイン規約 §メニュー). Null while nothing is out: a menu standing
    /// reads the worktree, and one up as it goes null stands again on it (`CommitMenuState.follow`, …). The model's
    /// `reading` kept in a `var` property is what the property held then: kept in a JS variable, it would read what the
    /// property holds now (`encode::tests::wire_qml`).
    property var treeAtAsk: null
    onRunningWhyChanged: {
        if (page.runningWhy === "")
            page.treeAtAsk = null
        else if (page.treeAtAsk === null)
            page.treeAtAsk = worktree.reading
    }
    /// Bumped each time something a menu's rows stand on has been read again and moved: what the menus read of the
    /// worktree (`WorktreeModel.readingRevision`), a list's rows (`NavSectionModel.changed`, said as they move and as a
    /// fold or the filter shows them otherwise — a stand that finds nothing moved writes nothing), a graph pass
    /// (`GraphModel.finishCount`). A menu standing follows it while nothing runs — its rows stand again on
    /// the tree as it is, keeping their shape (`CommitMenuState.follow`, `RefRowMenu.follow`, `FileRowMenu.follow`;
    /// デザイン規約 §メニュー); under a command it waits on `treeAtAsk`, and follows as that goes.
    property int menuReadRevision: 0
    readonly property int menuReadingSeen: worktree.readingRevision
    onMenuReadingSeenChanged: page.menuReadRevision++
    readonly property int menuWalkSeen: graphModel.finishCount
    onMenuWalkSeenChanged: page.menuReadRevision++
    Connections {
        target: branchesModel
        function onChanged() {
            page.menuReadRevision++
        }
    }
    Connections {
        target: worktreesModel
        function onChanged() {
            page.menuReadRevision++
        }
    }
    Connections {
        target: tagsModel
        function onChanged() {
            page.menuReadRevision++
        }
    }
    Connections {
        target: remotesModel
        function onChanged() {
            page.menuReadRevision++
        }
    }
    Connections {
        target: stashesModel
        function onChanged() {
            page.menuReadRevision++
        }
    }
    /// A press landed away from whatever held the keyboard (Main's `FocusRelease`), at `scenePos` — `null` for a press
    /// with no place of its own (the headless run's door). The left menu's name box goes unasked
    /// (デザイン規約 §左メニューの所作), and so does whatever over the graph stands on an empty box (§コミットを探す).
    /// A press, not the box losing focus: a row merely scrolled away would take a half-typed name with it.
    function releasePressedAway(scenePos) {
        sidebarPane.stopEdit()
        graphPane.dropEmptyBoxes(scenePos)
        diffPane.dropEmptyFind(scenePos)
    }
    /// A press landed anywhere in the window (Main's `FocusRelease`): the first press after arriving takes down the
    /// mark that sent the reader here (デザイン規約 §hover のツールチップ).
    function notePress() {
        detailsPane.dropAttention()
        page.dropWithheld()
        // And the commands mark (`raiseCommands`). A link answers on the release, after this press, so the mark it
        // raises stands.
        page.commandsAttention = false
    }
    /// A rename the second click opened no box for: its reason stands until the next input — a press (`notePress`),
    /// Escape, a key the history answers (arrows, the menu key: `GraphPane.keyReached`), the search (`startFind`), a
    /// menu, a gesture that opens a box — as a refused name's reason goes (`offers::worktree_rename`). This is the one
    /// put-down; the holders are `SidebarRowGestures.withholdRename` / `GraphPane.withholdRename`.
    function dropWithheld() {
        sidebarPane.dropWithheld()
        graphPane.dropWithheld()
    }
    /// And Escape, which every standing thing in this window answers (デザイン規約 §hover のツールチップ).
    /// **A key handler, not a shortcut**: this is reached only when no bar, popup or box took the key first, and two
    /// enabled `StandardKey.Cancel` shortcuts fire neither (`AskBar.held`, `tests/qml/tst_escape.qml`) —
    /// anything else that wants Escape joins as a key handler. **Topmost first, the marks last** — with nothing else
    /// standing, the mark is what is left; accepted only when something was taken.
    /// Headless: `diff-escape` / `plan-escape` / `plan-escape-held` / `recover-escape`.
    function escapePressed() {
        // First: a middle-click autoscroll holds the pointer and the view, on whichever surface (`MiddleHand`).
        if (MiddleHand.stop())
            return true
        // A rename's reason standing where no box opened — the topmost thing a key can take down (`notePress`).
        if (sidebarPane.renameWithheld || graphPane.renameWithheld) {
            page.dropWithheld()
            return true
        }
        // The plan's own way out — **only while that door is a press**. With something composed to lose it is a
        // hold (規約 §回数押し — `RebasePlanPane.discards`), and one key down must not throw away what the hold guards.
        if (page.planShown) {
            if (page.planDiscards)
                return false
            planModel.cancelPlan()
            return true
        }
        // The same press as the diff's `✕`, so never held back; arrows back to the graph (`closeDiffToGraph`), and the
        // left menu the diff folded comes back with it (`foldForDiff`).
        if (page.diffShown) {
            page.closeDiffToGraph()
            return true
        }
        // The discard log's entry on the graph, the same press as its band's `✕` — under the diff and the plan, which
        // the band stands aside for (`RecoverBand`), over the log. The keyboard goes back to the list the entry was
        // picked from (user decision — ask before changing): the graph stops walking what the reader has put away.
        if (page.recoverPicked !== null) {
            page.pickRecover(page.recoverPick)
            sidebarPane.takeRecoverKeyboard()
            return true
        }
        // The log (デザイン規約 §git が言ったことを読む場所), its fourth way out after `>_`, `Clear` and the closing mark
        // (user decision — ask before changing) — under the centre's (diff / plan), over the marks. This panel stands
        // up without a press (a failed command raises it): left out, folding a board the reader never put up would be
        // the one gesture unlike the rest. Taking its own place behind a splitter is no exception: the left menu
        // ignores Escape only because it never stands up by itself. By `shutCommands`, as the reader's press: a panel
        // the reader put away is not one the next landing fetch may take over. Its attention mark goes down with it
        // (`onCommandsOpenChanged`). Headless: `commands-escape`.
        if (page.commandsOpen) {
            page.shutCommands()
            return true
        }
        // Last, the mark the hover card's note raised (`MessageEditor.attention`). Headless: `card-message-esc` — no
        // picture answers it, `took=` does.
        if (!detailsPane.attention)
            return false
        detailsPane.dropAttention()
        return true
    }
    Keys.onEscapePressed: event => {
        event.accepted = page.escapePressed()
    }
    function foldByHand(collapse) {
        page.sidebarCollapsed = collapse
        // Asked open in a window with no room for it, the list stays open until the window has room again.
        page.listHeldOpen = !collapse && pageLayout.listCrowded
        page.foldedByDiff = false
        // Every way the list comes back (the band's block) arrives here, so "unfolding closes the diff" has no
        // exceptions elsewhere; a rail cell's peek and a rename box (`SidebarRowGestures.startEdit`) keep the fold.
        if (!collapse && page.diffShown)
            page.closeDiff()
    }

    /// The right pane is the uncommitted changes. The question bar stands over the graph, not here, so this going down
    /// by itself — the tree emptied, outside or by a commit or stash — leaves a standing question up, saying the tree
    /// as it now is (`LeaveFlow`); the presses that leave it put the question down themselves (`activateRow`,
    /// `jumpToRef`). This window's or another worktree's (`readKey`).
    readonly property bool wipShown: page.readKey !== undefined && page.readKey.kind !== "commit"
    // Selected stash row's reflog selector ("" = not a stash).
    property string selectedStashRef: ""
    /// Reads `key` (`readKey`; undefined for nothing until a landing is taken). **What the pane was about is let go of
    /// here and nowhere else**: a worktree being read stops handing the pane its list the moment the pane is about
    /// anything else, whichever road it left by.
    function readRow(key) {
        const leaving = page.carriedPath
        page.readKey = key
        if (leaving === "" || page.carriedPath === leaving)
            return
        page.carriedName = ""
        page.carriedHome = false
        page.carriedOn = ""
        // Or the worktree's reads go on handing the pane a list nobody shows.
        repoTab.leaveCarriedStatus()
    }
    function showWip() {
        // This window's tree: every road here is about it, and `openWipFor` alone puts a worktree back afterwards.
        page.readRow(GitFacts.hereKey())
        page.pendingHeadSelect = false
        page.pendingHeadAsked = false
        page.pendingWipSelect = false
        page.selectedStashRef = ""
        // The worktree's row is not a commit, so nothing is held; the rows light the current one
        // (`GraphRowDelegate.selected`).
        page.chooseOnly("")
        page.closeDiff()
    }

    // ---- which worktree the WIP pane is about ---------------------
    //
    // Another worktree's uncommitted row is keyed by that worktree's path (`readKey`). **Empty means this window's own
    // tree, and only then can anything here be written.**
    readonly property string carriedPath: page.readKey !== undefined && page.readKey.kind === "worktree"
                                          ? page.readKey.path : ""
    property string carriedName: ""
    /// That worktree is the repository's own, whose name wears the house (規約 §ref の種別「名前の印」).
    property bool carriedHome: false
    /// The commit that worktree's row stood on when last drawn (`GraphModel.carriedAt`'s `head`) — the row's parent,
    /// kept for the pass that takes the row while the worktree stands on nothing drawn (`settleCarriedAfterPass`).
    property string carriedOn: ""
    /// The last worktree whose changes were asked here, as it went by then (`NavSectionModel.worktreeName`) — the name
    /// such a report says, which carries the folder alone, and whether it was the repository's own, the mark that name
    /// wears (`showReport`); undefined before the first.
    property var appliedFrom: undefined
    /// Whether the pane may write what it shows — the one property every write control reads (デザイン規約 §無効).
    readonly property bool wipWritable: page.carriedPath === ""
    /// Another worktree's pane is the right pane's face (`carriedSeat`), which its first standing builds.
    readonly property bool carriedShown: page.wipShown && !page.wipWritable
    onCarriedShownChanged: if (page.carriedShown) carriedSeat.active = true
    /// The list the right pane shows: our unstaged run, or another worktree's one folded list (`CarriedPane`). Every
    /// question about the files on screen goes here; both answer alike, being built from one whole status
    /// (`models::nav::Source::files`).
    readonly property var wipUnstaged: page.wipWritable ? unstagedModel : carriedModel
    /// The diff in two columns or one — the band's toggle, and the saved choice as the page opens
    /// (`PageLayout.applySavedLayout`; the machine's choice, `LayoutState.diff_split`).
    function setDiffSplit(split) {
        diffModel.setSplit(split)
    }
    /// Tree or flat paths, for our three lists and the worktree's one at once: the choice is the pane's, so stepping
    /// onto another worktree and back keeps it, and one answer is saved at close.
    function setWipTreeView(tree) {
        conflictsModel.setTreeView(tree)
        unstagedModel.setTreeView(tree)
        stagedModel.setTreeView(tree)
        carriedModel.setTreeView(tree)
    }
    /// Opens the WIP pane on whichever worktree row `row` is — this window's own, or another worktree's.
    function openWipFor(row) {
        const carried = row >= 0 ? graphModel.carriedAt(row) : undefined
        const at = carried !== undefined ? carried.path : ""
        // Already the worktree being read: clearing the pane would close the file opened from it under the clicking
        // hand.
        if (at !== "" && at === page.carriedPath)
            return
        // The list's light names a file of the worktree being left, which the arriving worktree may also have. Our
        // own row to itself is not a step.
        if (at !== page.carriedPath)
            wipPane.clearChoice()
        page.showWip()
        if (at !== "")
            page.standOnWorktree(at, carried)
    }
    /// Points the pane at one worktree and reads its files; that worktree's paced reads keep them current while it
    /// stands (`pollCarried`). `carried` is its row's record (`GraphModel.carriedAt`).
    function standOnWorktree(at, carried) {
        page.readRow(GitFacts.worktreeKey(at))
        page.carriedName = carried.name
        page.carriedHome = carried.home
        page.carriedOn = carried.head
        repoTab.readCarriedStatus(at, carried.name)
    }
    /// A pane on another worktree is about this window's own tree again (`leaveWorktree`).
    function dropCarried() {
        if (page.carriedPath !== "")
            page.readRow(GitFacts.hereKey())
    }
    /// Follows the worktree being read across a graph pass, and lets go where the worktree has gone clean and taken
    /// its row. **Addressed by its key**: rows move under a rebuild (`GraphModel.rowOfKey`).
    function settleCarriedAfterPass() {
        if (page.carriedPath === "")
            return
        const row = graphModel.rowOfKey(page.readKey)
        if (row >= 0) {
            // The repository's own worktree goes by the branch it has out, which a switch over there changes under
            // the pane: the bands take the row's name again.
            const carried = graphModel.carriedAt(row)
            if (carried.name !== page.carriedName)
                page.standOnWorktree(page.carriedPath, carried)
            page.carriedOn = carried.head
            page.selectedRow = row
            graphPane.setCurrentRow(row)
            return
        }
        // The worktree committed or stashed: **land where that worktree now stands** — the row named a worktree, not
        // a branch (`NavSectionModel.headOfWorktree`). A listing a tick behind lands on its previous HEAD, still its
        // own history. One that stands on nothing drawn — a branch with no commits yet, or gone from the listing
        // (removed, pruned) — lands on the commit its row stood on, the row's parent; that gone too, the opening
        // default.
        const head = worktreesModel.headOfWorktree(page.carriedPath)
        const headRow = head === "" ? -1 : graphModel.rowOf(head)
        const landRow = headRow >= 0 ? headRow : page.carriedOn === "" ? -1 : graphModel.rowOf(page.carriedOn)
        page.readRow(undefined)
        if (landRow >= 0) {
            graphPane.setCurrentRow(landRow)
            page.activateRow(graphModel.oidAt(landRow), landRow, true)
            return
        }
        page.trySelectDefault()
    }

    // ---- commit editor -------------------------------------------
    property bool amending: false
    // Whether a remote already has HEAD's commit — the amend's `already pushed`. Kept beside HEAD by the session, so a
    // HEAD that moved wears no answer about the commit it left.
    readonly property bool headPublished: worktree.headPublished
    /// Whether the tab was standing in another worktree at the last drain (`RepoTab.standing`). Every property of
    /// that model shares one notify, so the edge is seen only by keeping the last value
    /// (rules-refs/app-ui.md「『まだ答えが無い』と値 0 / false を分ける」).
    property bool wasStanding: false
    Connections {
        target: repoTab
        function onChanged() {
            if (page.wasStanding !== repoTab.standing) {
                page.wasStanding = repoTab.standing
                if (!page.wasStanding)
                    page.standSettled()
            }
            page.absorbHeadMessage()
            page.absorbMoveAsk()
            page.absorbWriteResult()
            page.absorbFetchRecovery()
            page.absorbPacedReads()
        }
        // Only the first failure of a run: an offline machine would otherwise re-raise the panel every interval (デザイン規約
        // §git が言ったことを読む場所).
        function onFetchFirstFailed() {
            // Before the raise: whether a recovered fetch takes the panel back down (`absorbFetchRecovery`) turns on
            // whether one was already standing.
            commandsOwner.fetchRaises(page.commandsOpen)
            page.commandsOpen = true
        }
    }

    // A tab whose repository would not open: the screen takes the panes' seat (`OpenFailedScreen`).
    readonly property bool openFailed: !page.blank && repoTab.state === "error"
    /// Automation (`_PGG_AUTO_ACT=open-fail-sweep`): that screen's hand, which its three lines are dragged over from;
    /// null while the screen is not up (`failedSeat`).
    readonly property var openFailedHand: failedSeat.item !== null ? failedSeat.item.pad : null

    property int seenHeadCommitSeq: 0
    property bool wantHeadMessage: false
    function amendToggled(on) {
        page.amending = on
        if (on) {
            page.wantHeadMessage = true
            repoTab.requestHeadCommit()
        } else {
            page.clearCommitEditor()
            // A stopped merge takes the tick away itself (`WipCommitBlock.amendBarred`), on the same status that brings
            // the merge's message — which may already have passed the box over while it held HEAD's. Asked again, so
            // the emptied box takes it.
            if (worktree.opMerging) {
                page.seenOpMessage = ""
                page.absorbOpMessage()
            }
        }
    }
    /// Whether an amend is in view — the amend box ticked, or HEAD's own commit open in the details pane, whose save is
    /// an amend too. While one is, core asks whether it would leave HEAD changing nothing (`RepoTab.amendEmpties`) and
    /// asks again as the index or HEAD moves, whoever moves them (`RepoTab.watchAmend`). The page says only this; when
    /// to ask is core's — a key of counts built here would miss another git staging as many files. Headless:
    /// `amend-tip --preset undone` (waits for the tip alone; `empties=` is no wait condition),
    /// `edit-message-refused reworded --preset undone`, `amend-row feature/clash --preset clashing` (the box ticked,
    /// then the merge stops and takes the tick off).
    readonly property bool amendInView: !page.blank && repoTab.state === "open"
        && (page.amending || page.messageEdit === "amend")
    onAmendInViewChanged: repoTab.watchAmend(page.amendInView)
    function absorbHeadMessage() {
        if (repoTab.headCommitSeq === page.seenHeadCommitSeq)
            return
        page.seenHeadCommitSeq = repoTab.headCommitSeq
        if (!page.wantHeadMessage)
            return
        page.wantHeadMessage = false
        wipPane.setMessage(repoTab.headSubject, repoTab.headBody)
    }
    function clearCommitEditor() {
        wipPane.clearMessage()
        page.opFilledSubject = ""
        page.opFilledBody = ""
    }

    /// A stopped merge opens the box holding what it will record — `git commit` there writes the commit `--continue`
    /// would (デザイン規約 §進行中の操作から出る). **Once per message, only into empty boxes**: typed text is the one thing here that
    /// cannot be read back off disk. **Taken back out when the merge goes**, but only while untouched (the pair below).
    property string seenOpMessage: ""
    property string opFilledSubject: ""
    property string opFilledBody: ""
    function absorbOpMessage() {
        const subject = worktree.opMerging ? worktree.opSubject : ""
        const body = worktree.opMerging ? worktree.opBody : ""
        const message = subject === "" ? "" : subject + "\n" + body
        if (message === page.seenOpMessage)
            return
        page.seenOpMessage = message
        const ours = page.opFilledSubject !== ""
                     && wipPane.subjectText === page.opFilledSubject
                     && wipPane.bodyText === page.opFilledBody
        page.opFilledSubject = ""
        page.opFilledBody = ""
        if (subject === "") {
            if (ours)
                wipPane.clearMessage()
            return
        }
        if (!ours && (wipPane.subjectText !== "" || wipPane.bodyText !== ""))
            return
        page.opFilledSubject = subject
        page.opFilledBody = body
        wipPane.setMessage(subject, body)
    }

    /// The name of the entry a pop brings back, held until git says the pop landed. **Read at the press**: by the
    /// answer the entry is off the list. Empty for one git named itself (`WIP on …`), which names the commit, not the
    /// work.
    property string pendingPopLabel: ""
    /// The id the queue gave the pop at the press (`RepoTab.popStash`); zero while no pop is out. **Every stash
    /// operation answers under the same word** (`writeAnswerOp` cannot say whose): an `apply` pressed just before
    /// answers first, and counted by turn its landing would pass for the pop's. Only the pop's own answer disarms it.
    property int pendingPopId: 0
    /// Both ways in to a pop — the graph row's menu and the details pane's band — so the name comes back from one
    /// place. A pop gives the name back to the commit box (user decision — ask before changing): the work returns to
    /// the tree and the entry goes, so the box is the one place left to hold it. An `apply` keeps the entry, so its
    /// name stays there. Only a name somebody gave (`stash::label_in`): `WIP on <branch>: <sha> <subject>` is about
    /// the commit the work stood on. The entry is looked up whatever the list's filter (`NavSectionModel.nameOfFull`).
    /// Nets: core's `session_integration::operations`, the app's `repo_tab::tests::drain`. Headless: `stash-pop-row`,
    /// whose `graph_settled … back=` reads the box's `subjectText`.
    function popStash(selector) {
        page.pendingPopLabel = GitFacts.stashLabel(stashesModel.nameOfFull(selector))
        page.pendingPopId = repoTab.popStash(selector)
        page.selectedStashRef = ""
    }
    /// The answer to that pop, found by id among this notify's (`RepoTab.writeAnswerIndex`); a notify without it leaves
    /// the wait standing. Read ahead of `absorbLeftoverAnswer`, so a refusal reaches here too. The words go in only
    /// where the pop landed, and **only into empty boxes** (`absorbOpMessage`) — both: a description with no summary is
    /// not an empty editor. A pop that conflicted landed (git keeps the entry): the name then stands in the box and the
    /// list both, but the work is in the tree, so the words about it belong in the box.
    function absorbPopLabel() {
        if (page.pendingPopId === 0)
            return
        const answer = repoTab.writeAnswerIndex(page.pendingPopId)
        if (answer === undefined)
            return
        const carried = page.pendingPopLabel
        const landed = !repoTab.writeAnswerFailed(answer)
        page.pendingPopLabel = ""
        page.pendingPopId = 0
        if (landed && carried !== "" && wipPane.subjectText === "" && wipPane.bodyText === "")
            wipPane.setMessage(carried, "")
    }

    // An amend runs straight even when HEAD is pushed (デザイン規約 §回数押し「ローカルの履歴書き換えはクリック」). The
    // slot records the queue's id itself (`ops::Press`).
    function commitNow() {
        // Another worktree's changes show in `CarriedPane`, which has no commit button; this is the belt under it.
        if (!page.wipWritable)
            return
        repoTab.commit(wipPane.outgoingSubject, wipPane.outgoingBody, page.amending, wipPane.resetAuthor)
    }
    /// The answer to that commit — **its own answer** among those this notify carried (`RepoTab.commitAnswer`,
    /// undefined for none): one drain can bring a fetch's too, and read off a shared property a landed commit would
    /// keep its message and a hook's refusal would go untold. Everything the page does with a commit is here;
    /// `absorbLeftoverAnswer` never reaches it.
    function absorbCommitAnswer() {
        const answer = repoTab.commitAnswer()
        if (answer === undefined)
            return
        const landed = !repoTab.writeAnswerFailed(answer)
        if (landed) {
            page.clearCommitEditor()
            wipPane.setAmendChecked(false)
            page.amending = false
            page.readDiffForAnswer()
        } else {
            // A rejected commit keeps its text (it cannot be read back off disk).
            page.tellRefusal(answer)
        }
        page.commitAnswered(landed)
    }
    /// The answer to the stash press that emptied the tree — its own answer (`RepoTab.stashAnswer`, undefined for
    /// none). The emptied tree is a second wait (`leaveWipWhenDone`); this half is over as soon as git speaks.
    function absorbStashAnswer() {
        const answer = repoTab.stashAnswer()
        if (answer === undefined)
            return
        if (repoTab.writeAnswerFailed(answer)) {
            page.tellRefusal(answer)
            return
        }
        page.readDiffForAnswer()
    }
    /// The answer to a worktree a menu asked for — its own answer (`RepoTab.worktreeAnswer`, undefined for none).
    /// Made, the tab goes and stands in it, as the box and the row said it would: the worktree is where the reader went
    /// to work (デザイン規約 §worktree を作る). Refused, the yellow report comes down over the graph, the log left shut (user
    /// decision — ask before changing; its lines: `Words.writeReported` `worktree-*`). A failed
    /// `post-checkout` is both: git keeps the worktree, so the tab goes all the same, the report riding along — the bar
    /// is the page's, and the page is what stands anew. Both halves are user decisions (ask before changing): the tab
    /// stands in it as in any made worktree, since it is there and the reader made it to work in, and the report's
    /// heading says the tab went there — "opened" says "made" (`Words.writeReported` `worktree-hook`).
    function absorbWorktreeAnswer() {
        const answer = repoTab.worktreeAnswer()
        if (answer === undefined)
            return
        if (repoTab.writeAnswerFailed(answer)) {
            const report = repoTab.writeAnswerReport(answer)
            const made = report !== undefined && report.kind === "worktree-hook"
            page.tellRefusal(answer)
            if (!made)
                return
        }
        page.openRepositoryPathRequested(repoTab.worktreeAnswerPath)
    }
    /// The answer to a branch's push, the toolbar's or the branch row's — its own answer (`RepoTab.pushAnswer`,
    /// undefined for none). Only a refusal is the page's; the button's mark is `PublishFlow.noteWriteAnswer`'s. **An
    /// answer with an owner never falls to the group's `lastWriteError`**, so without this step neither
    /// `push-outdated`'s report nor `push --preset unreachable`'s log comes out. Headless: `push` / `push-retry`.
    function absorbPushAnswer() {
        const answer = repoTab.pushAnswer()
        if (answer === undefined || !repoTab.writeAnswerFailed(answer))
            return
        page.tellRefusal(answer)
    }
    /// The answer to a push a **ref row** sent — the `push --delete`s, the replace, the pairs that do something here
    /// as well as over there (`RepoTab.refPushAnswer`, undefined for none). **Its own answer**: read by word, a fetch
    /// in the same drain would swallow the refusal. The rows a delete took come back through the delete's own owner;
    /// this carries only why.
    function absorbRefPushAnswer() {
        const answer = repoTab.refPushAnswer()
        if (answer === undefined || !repoTab.writeAnswerFailed(answer))
            return
        page.tellRefusal(answer)
    }
    /// A refusal somebody outside made (a hook, a remote newer than our picture) comes down as a report
    /// (デザイン規約 §答えの要らない報せ). Returns whether there was one; otherwise each caller does its own thing.
    /// Read off the answer: one drain can carry several reports (`RepoTab.writeAnswerReport`).
    function reportRefusal(answer) {
        const report = repoTab.writeAnswerReport(answer)
        if (report === undefined)
            return false
        page.showReport(report)
        // The `>_` mark goes quiet with the report (`answeredFailures`).
        if (!commandsModel.failed)
            page.answeredFailures++
        commandsModel.noteAnswered()
        page.writeReported(report)
        return true
    }
    /// A press's refusal: a report where somebody outside wrote the words, otherwise the log comes up
    /// (デザイン規約 §git が言ったことを読む場所). The delete card calls `reportRefusal` alone: its row is the answer.
    function tellRefusal(answer) {
        if (page.reportRefusal(answer))
            return
        commandsOwner.newsTakes()
        page.commandsOpen = true
    }
    /// Automation: the editor's commit was answered and the boxes dealt with — no picture can tell.
    signal commitAnswered(bool landed)

    // ---- moving between branches and commits ----------------------
    // Uncommitted changes come along (デザイン規約 §未コミット変更がある状態での移動). A move is handed about as core reads it
    // (`CheckoutTarget`, from `GitFacts.switchMove` / `moveOnto`); every one lands on a branch, `local`
    // (デザイン規約 §ブランチ・コミットへの移動).

    // ---- the same chip, pressed again ------------------------------
    /// The branch a move already sent is putting HEAD on; `""` while none is on its way. Meanwhile `switchToRef` sends
    /// nothing: the screen takes a move in long after the write answered (refs are published after the answer,
    /// `session::write`), so a second press on a remote branch would send the same `switch --create` and be refused.
    /// **Ends at the move's own landing** — HEAD on the branch and the branch listed (`offers::move_landed`; not a
    /// seq, which moves for reads nobody asked for) — or at a refusal or the move's question (`absorbMoveAsk`).
    property string moveLanding: ""
    function absorbMoveLanding() {
        if (page.moveLanding !== ""
                && GitFacts.moveLanded(page.moveLanding, worktree.standing,
                                       branchesModel.oidOfName(page.moveLanding) !== ""))
            page.moveLanding = ""
    }

    function switchTo(move, leaving) {
        if (page.standsInTheWay(leaving)) {
            leaveFlow.startAsk(move.local, function (leave) { page.switchTo(move, leave) })
            return
        }
        // Marked where the move goes out: the question a blocked move raises comes back through here, and a press
        // held behind a bar has sent nothing yet.
        page.moveLanding = move.local
        repoTab.checkout(move, leaving === true)
    }

    // ---- moving out of what stands in a move's way --------------------
    // What blocks a move (an operation standing, unmerged paths) is core's rule (offers::moves_blocked), and clearing
    // it is one question for both — every blocker is answered on this bar (user decision — ask before changing; デザイン規約
    // §進行中の操作から出る). **Asked before anything is sent** — git's refusal would put a red log line where a question
    // belongs. **Gated in both `switchToRef` (the hand's road) and `switchTo` (automation and the force road)**: the
    // first alone lets `checkoutMovingBranch`'s "what would be lost" read run under the standing operation; the second
    // alone raises `Move here?` and the abort question back to back. **The agreement travels as the `leaving`
    // argument**, never a page property: re-entered, the gate would read its own answer and stand for ever. Headless:
    // `switch-mark <branch>:asks|plain`, `switch-stopped[-go]`, `switch-conflicted[-go]`, `switch-held`.
    function standsInTheWay(leaving) {
        return leaving !== true && worktree.movesBlocked
    }
    // Raised by `switchToRef` and `switchTo` with the move to send again, leaving what stands in its way or not as the
    // bar's answer says (`LeaveFlow.startAsk`).
    LeaveFlow {
        id: leaveFlow
        worktree: worktree
        graphPane: graphPane
        // The hold is the danger's: only a leave that undoes work is held.
        onAskRequested: (oidHex, label, detail, danger, run, code, refName, refTint) =>
            page.startRowAsk(oidHex, label, detail, danger, "", run, danger, "", null, code, refName, refTint, "branch")
    }

    // ---- what a chip leads to --------------------------------------
    // `chip` is the chip as it is drawn (`encode::Chip`); null is a row that draws none.
    function activateChip(chip) {
        if (chip)
            page.switchToRef(chip.kind, chip.name)
    }
    /// Answers whether the press did anything — a move sent, or a question raised in front of one. `false` is a press
    /// turned away, which the headless double press reads (動詞 `switch-remote-twice`): from outside, both presses
    /// look alike.
    function switchToRef(kind, name, leaving) {
        // A worktree on no branch, named by its path (the menus' `Open`, `RefRowMenu.offerOn`): the tab goes and
        // stands in it, as its WORKTREES row's double-click does — ahead of the gate below, since it writes nothing.
        if (kind === "worktree") {
            if (worktreesModel.worktreeFacts(name) === undefined)
                return false
            page.openRepositoryPathRequested(name)
            return true
        }
        // `busyCount` alone is not the gate: it rises only when the queue starts the write and is down before the
        // screen catches up (`moveLanding`); the held doors cover a plan's run not yet started (`doorsHeldWhy`).
        // Every graph door comes through here.
        if (repoTab.state !== "open" || repoTab.busyCount > 0 || page.moveLanding !== ""
                || page.doorsHeldWhy !== "")
            return false
        // Which move the lookups add up to is core's rule (offers::switch_action); a remote row's holder is asked of
        // the local branch it lands on — the row read as its menu reads it, through the same part (`RefMenuFacts`).
        const lead = refFacts.leadOf(kind, name, "")
        const local = lead.local
        const press = GitFacts.switchMove(lead, worktree.standing, refFacts.landsListed(lead))
        const action = press.action
        // A branch another worktree holds is the one refusal no stash or put-down can clear (offers::SwitchAction), so
        // **the press goes to that worktree** (`openRepositoryPathRequested`). Unasked — it writes nothing; the worded
        // rows name the worktree first (`RefRowMenu`'s `Open`), and for the wordless doors (the graph's double-click, a
        // chip's press) the landing names it: the tab's tree mark and folder, the WORKTREES row's green (§タブの所作).
        // Before the leave question: no put-down clears the holder road, so undoing a rebase first would spend the
        // undo on a move that was never possible.
        if (action === "holder") {
            page.openRepositoryPathRequested(lead.path)
            return true
        }
        // The branch already out, a tag or the detached marker moves nothing, so it raises nothing either — ahead of
        // the leave question, which would put the work down for a move never sent (`switch-conflicted main`). A tag's
        // row offers a branch at its commit instead (`startNaming`).
        if (action === "")
            return false
        // Ahead of the branches below: the one that lands on an existing local branch asks git what the move would
        // cost before it moves, and that read is worth nothing while an operation is standing.
        if (page.standsInTheWay(leaving)) {
            leaveFlow.startAsk(local, function (leave) { page.switchToRef(kind, name, leave) })
            return true
        }
        // `switch` and `materialize` are a checkout core spells (`SwitchAction::checkout`).
        if (press.checkout !== undefined)
            page.switchTo(press.checkout, leaving)
        else if (action === "move") {
            // Whether to ask is git's to answer — a branch that only fell behind loses nothing. The question comes back
            // as `moveAskSeq`, the operation still standing for its answer to undo (core asks before it aborts).
            page.moveLanding = local
            repoTab.checkoutMovingBranch(local, name, leaving === true)
        }
        return true
    }

    // Landing on the remote branch moves the existing local one onto it — the one move here that can take commits off
    // a branch, `lost` of them (`SessionEvent::MoveNeedsAsk.lost`, the second line). Raised only when `lost` > 0 —
    // a branch that only fell behind moves unasked (`switch -C`) — so it is always a hold: something is thrown away.
    function askMoveBranchOnto(local, remoteRef, lost) {
        // The branch in the heading through its seat, so it takes the colour and the mark a name takes there
        // (`AskBar.labelSentence`).
        page.startRowAsk(
            remotesModel.oidOfName(remoteRef),
            qsTr("Move %1 here?"),
            Words.losesCommits(local, lost),
            false,
            qsTr("Move"),
            function () { page.switchTo(GitFacts.moveOnto(local, remoteRef)) },
            true, "", null, "", local,
            GitFacts.isCurrent(worktree.standing, local) ? Theme.textLink : Theme.textPrimary, "branch",
            { detail: Words.marksOf(name => Words.losesCommits(name, lost), [[local, "branch"]]) })
        page.moveBranchAsked(local)
    }
    /// Automation: the question above was raised, about `local` — the bar carries no name a run could read back.
    signal moveBranchAsked(string local)

    /// Ctrl+F is quietly refused while a plan covers the graph, as a standing question refuses it
    /// (`GraphPane.startFind`). While the diff stands where the graph was, it is the diff that is searched
    /// (デザイン規約 §diff の中を探す): what is not on screen cannot be asked.
    function startFind() {
        page.dropWithheld()
        if (page.planShown)
            return
        if (page.diffShown)
            diffPane.startFind()
        else
            graphPane.startFind()
    }

    // ---- the interactive-rebase plan ---------------------------------
    // Composing runs nothing; the run is one queued write pinned to the tip the plan opened on. The page holds what
    // could move the history from inside this window (`planShown`); a terminal or another session is answered by the
    // model putting the plan away when the tip moves (`noteStanding` below).
    /// A plan stands with its rows: **the one answer** for everything that needs a draft (the right pane's boxes, the
    /// run bar) — a second spelling would disagree while `planLoadHeld` holds the opening face.
    readonly property bool planActive: planModel.active && !page.planLoadHeld
    /// The plan's face has the graph's seat: its read is out, or its rows have arrived. **Taken at the press** — the
    /// read at the root of a real history is far past a press's acknowledgement (ci/baseline/code-costs-windows-x64.md;
    /// the pane shows `RebasePlanPane.waiting`). The mode's restrictions read this too — the run button is the one way
    /// into a write: the left menu (`SidebarPane.frozen`), `BandPushButton` / `BandStashButton` and `TopBar`'s Search
    /// dim, `startFind` refuses without a word; fetch alone stays — the automatic fetch does the same behind the
    /// screen, so freezing its button alone would lie. The freeze starts at the press too (user decision — ask before
    /// changing). Only what needs a draft waits for `planActive`.
    readonly property bool planShown: planModel.active || planModel.loading
    /// Automation (`_PGG_AUTO_ACT=plan-loading`): the opening face, held from the real edge
    /// (verify-ui implement.md「中間状態は実 edge を latch する」). It holds the *whole* face: holding only the pane would
    /// photograph a run bar and a frozen menu over a centre still waiting, a screen the app never puts up.
    property bool planLoadHeld: false
    /// Automation: the plan model itself.
    readonly property var rebasePlan: planModel
    /// The rewrite warning's count for the plan's own range, recounted when the refs move under it
    /// (`RebasePlanModel.pushedCount`); 0 while no plan stands.
    readonly property int planPushed: planModel.pushedCount
    /// Whether the details pane's boxes are a plan row's reword input. **The pane must show that very commit**: a
    /// selection moved any other way leaves the typing with the message on screen. The model owns which row it is,
    /// so a reorder cannot detach the two.
    readonly property bool planReword: page.planActive && planModel.selectedAction === "reword"
                                       && planModel.selectedOid !== ""
                                       && planModel.selectedOid === detailsModel.shaHex
    /// What `Discard` is about to take away, which nothing else holds a copy of — so the button is held (デザイン規約
    /// §回数押し; user decision — ask before changing; the words do not change): the plan's edits, and a reword in the
    /// right pane's boxes not yet given to the plan. **Not a half-written amend from before the plan** — closing the
    /// plan leaves it (`DetailsPane.dropDraft`), so `Discard` stays a click (`guard=false`). **Read off
    /// `boxMoved`**, not off whether the boxes take typing: a row walked back out of `reword` locks the boxes with the
    /// text still in them.
    readonly property bool planDiscards: page.planActive
        && (planModel.dirty || (detailsPane.boxFromPlan && detailsPane.boxMoved))
    function startRebasePlan(oidHex) {
        planModel.open(oidHex)
    }
    // On the seat's edge: a diff open over the graph goes at the press. Every way out of the read (rows, a refusal, a
    // failure, `Discard` on the empty face) comes through here, so the fold has no exceptions.
    onPlanShownChanged: {
        if (page.planShown) {
            page.closeDiff()
            // The plan takes the graph, and the menu it folds is frozen under it: the discard log goes, with its entry.
            if (page.recoverOpen)
                page.toggleRecover()
        }
        page.foldForPlan(page.planShown)
    }
    onPlanActiveChanged: {
        if (page.planActive) {
            // Land on the plan's newest row whatever face was up — the WIP face cannot stay under an open plan. The
            // plan's *own* selection is already there: set from here it would fire the model's one `changed()` inside
            // the binding delivering it (`RebasePlanModel::take`).
            page.activateRow(planModel.expectHead)
            // The run bar the first plan builds (`planRunSeat`).
            planRunSeat.active = true
        } else {
            // However the plan went, a `reword` left in the boxes has no row to carry it, and the commit on screen did
            // not move (`DetailsPane.syncMessage` would keep the text for the plain amend) — its own message goes back.
            detailsPane.dropDraft()
        }
    }
    // The pushed count moves with the remote-tracking refs (fetch runs under the freeze, 規約 §フル interactive rebase),
    // so it is asked again on `refsMoved` — the every-tick `refsSettled` would spawn a rev-list at the status rate. A
    // branch's refusal goes with the branch on the same news.
    Connections {
        target: branchesModel
        function onRefsMoved() {
            planModel.refreshPushed()
            publishFlow.forgetUnlisted(name => branchesModel.oidOfName(name) !== "")
        }
    }
    Connections {
        target: planModel
        // Set only once a run really went out: `runPlan` turns away a plan that asks for nothing, and a hold for a
        // write never sent would never be let go.
        function onPlanRan() {
            page.planRunOut = true
        }
        // Three ways a range cannot be replayed. The detail is the row menu's too (`Words.rewriteRefusedWhy`); the
        // heading and `warning` are the plan's own (規約 §答えの要らない報せ): the row menu's report says one heading in `danger`,
        // since its gesture is over, and the plan's door one heading per shape in `warning`, since the range is still
        // being composed — two colours on purpose, the axis of §状態 (user decision — ask before changing).
        function onRefusedPlan(kind) {
            page.showNotice(Words.planRefused(kind), Words.rewriteRefusedWhy(kind), "warning")
        }
        function onStalePlan() {
            page.showNotice(qsTr("The branch tip moved"), qsTr("The plan was put away; nothing has run."), "warning")
        }
        function onStandingOp() {
            page.showNotice(qsTr("An operation started"),
                            qsTr("The plan was put away; nothing has run. Finish what is in progress first."),
                            "warning")
        }
    }
    // Tip and standing operation, fed on every snapshot while a plan stands: either moving puts the plan away (a merge
    // stopped on a conflict leaves the tip where it was).
    Connections {
        target: worktree
        function onChanged() {
            if (!page.planActive)
                return
            planModel.noteStanding(worktree.standing)
        }
    }

    // ---- the standing question --------------------------------------
    // One bar over the graph (デザイン規約 §可否・警告の出し場所). Only questions about a ref reach it: taking one named
    // thing away is a hold (デザイン規約 §回数押し).
    property var rowAskRun: null
    function startRowAsk(oidHex, label, detail, danger, acceptText, run,
                         hold = false, tip = "", form = null, code = "", refName = "", refTint = Theme.textLink,
                         refMark = "", marks = null) {
        // Whoever was dressing the bar lets go first, or that flow would think it still stands and its check timer
        // go on firing round trips at a remote. `startAsking`'s assignments beat a live `Binding` outright; the flow
        // raising this one turns its own back on afterwards.
        publishFlow.publishAsking = false
        upstreamFlow.asking = false
        renameCarryFlow.asking = false
        leaveFlow.asking = false
        page.rowAskRun = run
        // Over the discard band, the question takes its place at its height (`BarTravel`).
        if (!graphPane.asking && recoverBand.visible) {
            graphPane.keepRowsInPlace()
            graphPane.askEnterFrom = recoverBand.height
        }
        graphPane.startAsking(oidHex, label, detail, acceptText, danger, hold, tip, form, code, refName, refTint,
                              refMark, marks)
    }
    function stopRowAsk() {
        page.rowAskRun = null
        publishFlow.publishAsking = false
        upstreamFlow.asking = false
        renameCarryFlow.asking = false
        leaveFlow.asking = false
        graphPane.stopAsking()
    }
    /// The question leaves the middle in one frame (`AskBar.handingOver`) — answered or walked away from with something
    /// behind it, as its pill's floor passes if it was pressed through, or for a report coming over it. What comes next
    /// stands at its height — the report waiting for that answer first (`noticeWaits`), else the band — and the graph's
    /// rows stay where they stood.
    function askHandsOver(height) {
        graphPane.keepRowsInPlace()
        if (page.noticeWaits)
            page.raiseNotice(height)
        else if (noticeBar.shut && page.recoverBandWanted)
            recoverBand.enterFrom = height
    }
    function answerRowAsk() {
        const run = page.rowAskRun
        page.stopRowAsk()
        if (run)
            run()
    }

    // On its own counter: the same move can be asked about twice in a row, and only a fresh answer may raise the
    // question.
    property int seenMoveAskSeq: 0
    function absorbMoveAsk() {
        if (repoTab.moveAskSeq === page.seenMoveAskSeq)
            return
        page.seenMoveAskSeq = repoTab.moveAskSeq
        // The move came back as a question: nothing is on its way to the screen, so the next press may go.
        page.moveLanding = ""
        page.askMoveBranchOnto(repoTab.moveAskLocal, repoTab.moveAskStart, repoTab.moveAskLost)
    }

    // ---- what a branch is measured against --------------------------
    UpstreamFlow {
        id: upstreamFlow
        repoTab: repoTab
        remotesModel: remotesModel
        graphPane: graphPane
        worktree: worktree
        onAskRequested: (oidHex, label, accept, run, form) =>
            page.startRowAsk(oidHex, label, "", false, accept, run, false, "", form, "")
    }
    /// The branch card's row, from either menu it is carried by; the question is marked on that branch's own row.
    /// `unsettable`: it has an upstream an emptied name takes off.
    function startUpstreamAsk(branch, counterpart, unsettable) {
        upstreamFlow.startAsk(branch, branchesModel.oidOfName(branch), counterpart, unsettable)
    }

    // ---- what a name changed here does to the remote ----------------
    RenameCarryFlow {
        id: renameCarryFlow
        repoTab: repoTab
        remotesModel: remotesModel
        graphPane: graphPane
        onAskRequested: (oidHex, label, accept, run, form) =>
            page.startRowAsk(oidHex, label, "", false, accept, run, false, "", form, "")
        onCarryAsked: (kind, from, to) => page.renameCarryAsked(kind, from, to)
    }
    /// Automation: the question above was raised, and the two names it is between.
    signal renameCarryAsked(string kind, string from, string to)
    /// A branch took a new name here and its upstream still names the remote branch under the old one. `remoteRef`
    /// is the old upstream's row, and stays whole: the answers send the row itself, and the tab reads which branch
    /// over there it is (`RepoTab.replaceRemoteBranch`). Only the new side's name is asked for, to show and to check
    /// (`GitFacts.remoteRefBeside`).
    function carryBranchRenameOver(remoteRef, name) {
        const comes = GitFacts.remoteRefBeside(remoteRef, name, repoTab.remoteNames)
        // Not where the remote already carries the new name: both answers would push, which fast-forwards the branch
        // over there and reports success — somebody else's branch would move. The box refuses it too; this catches
        // the way in that has no box.
        if (comes === "" || comes === remoteRef
                || remotesModel.hasRemoteBranch(GitFacts.remoteBranchOf(remoteRef, repoTab.remoteNames).remote, name))
            return
        // The commit the old name's row shows, empty where the row is out of the list — gone before the rename, or
        // pruned by a fetch while it ran: asked all the same — the upstream still names the old branch —, with the
        // answers that are left (`RenameCarryFlow.gone`).
        renameCarryFlow.startBranchAsk(remoteRef, comes, name, remotesModel.oidOfName(remoteRef))
    }
    /// Automation: a row of the chooser inside it, which no injected click can reach, picked; answers whether it was
    /// (`RenameCarryFlow.pickChoice`).
    function pickCarryChoice(index) { return renameCarryFlow.pickChoice(index) }

    // ---- sending the branch to its remote ---------------------------
    PublishFlow {
        id: publishFlow
        repoTab: repoTab
        worktree: worktree
        remotesModel: remotesModel
        graphPane: graphPane
        keepBuilt: page.keepBuilt
        // The first push asks where the branch goes, in the one question bar.
        onAskRequested: (oidHex, label, run, form, code, refName, refTint) =>
            page.startRowAsk(oidHex, label, "", false, "", run, false, "", form, code, refName, refTint, "branch")
    }
    /// What the window's toolbar reads off the page it is showing: the button lives up there, and the state machine
    /// behind it down here (`TopBar`). The push's answer goes up whole (`PublishFlow.pushFacts`); only what the page
    /// adds to it — whether its doors are open, what git said last — goes beside.
    readonly property alias pushFacts: publishFlow.pushFacts
    readonly property alias canPush: publishFlow.canPush
    readonly property alias canForcePush: publishFlow.canForcePush
    readonly property alias pushFailed: publishFlow.pushFailed
    readonly property alias pushFailReason: publishFlow.pushFailReason
    function pushNow() {
        publishFlow.pushNow()
    }
    function forcePush() {
        publishFlow.forcePush()
    }
    /// The push row on a branch never sent, from either menu it is carried by: the question names that branch and is
    /// marked on its own row (`PublishFlow.publishLocal`).
    function startPublishAskFor(branch, upstream) {
        publishFlow.startPublishAsk(branch, upstream, branchesModel.oidOfName(branch))
    }

    /// One of this page's right-click menus is standing: nothing behind a menu is being hovered, so no card or tip may
    /// come out over the rows the hand is reading. A menu nobody has raised yet is not standing.
    readonly property bool menuStanding: page.menuShowing(refMenuSeat) || page.menuShowing(remoteMenuSeat)
                                         || page.menuShowing(commitMenuSeat) || page.menuShowing(fileMenuSeat)
                                         || page.menuShowing(commitFileMenuSeat) || page.menuShowing(diffMenuSeat)
    /// Where the standing menu was raised, and on what — one door (`menuDoor`), null while none stands. `area` is the
    /// surface — `sidebar` (the left panel's rows), `peek` (the rows of the folded rail's open section), `refList`
    /// (the rows of a chip's stacked list), `graph`, `files`, `changes` (a commit's changed files), `diff`; `key` the
    /// thing the row stood for (`<kind>:<ref>` as the left panel keys its rows, a row's commit, a file's path). What
    /// hover opened stays under a menu raised on its own rows and goes behind any other (デザイン規約 §メニュー). The
    /// doors note the pair before the menu shows and it is read only while a seat shows, so no reader sees the menu
    /// up before its origin. **A reader weighing both at once reads the door**: `menuArea` and `menuKey` are two
    /// properties, updated one after the other.
    readonly property var menuDoor: page.menuStanding ? { "area": page.menuDoorArea, "key": page.menuDoorKey } : null
    readonly property string menuArea: page.menuDoor !== null ? page.menuDoor.area : ""
    readonly property string menuKey: page.menuDoor !== null ? page.menuDoor.key : ""
    property string menuDoorArea: ""
    property string menuDoorKey: ""
    /// Where a menu is being raised and on what (`menuDoor`). Each door then raises its own seat by the seat's id: a
    /// seat handed in as an argument is past what `cargo xtask waits` follows (its reach stops at the menu's calls).
    function noteMenuDoor(area, key) {
        page.menuDoorArea = area
        page.menuDoorKey = key
    }
    function menuShowing(seat) {
        return seat.item !== null && seat.item.showing
    }

    /// The six menu seats, and the popups and strips the parts build on their first opening (the add-remote dialog,
    /// the row's card and the chip's list, the author row's cards, the line-ending card, the two find strips, the
    /// folded rail's open section), stand built from the start — written only by the harness (`PageHarness`), so its
    /// verbs can read them before they open. What a page shows off its state is not in this — the plan's face, the
    /// command log, the failure screen, the exit card, the plan's run bar, the discard log and band, the report's and
    /// the question's words, another worktree's pane: their verbs raise them as a hand does.
    property bool keepBuilt: false

    // ---- context menu on a sidebar row ------------------------------
    // Each menu is built when first raised (rules-refs/app-ui.md — the `WindowDialogSeat` line). The door activates the
    // seat, then calls in synchronously; the seat fills the page so the menu measures the window (`AppMenu.ownerItem`).
    Loader {
        id: refMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: RefRowMenu {
            // Every row moves something, so the whole card is held while the doors are, raised from a graph chip too
            // (デザイン規約 §メニュー「入口が違っても同じ操作は同じ文」).
            heldReason: page.doorsHeldWhy
            runningWhy: page.runningWhy
            repoTab: repoTab
            worktree: worktree
            treeAtAsk: page.treeAtAsk
            readRevision: page.menuReadRevision
            graphModel: graphModel
            branchesModel: branchesModel
            worktreesModel: worktreesModel
            tagsModel: tagsModel
            remotesModel: remotesModel
            stashesModel: stashesModel
            githubLinks: githubLinks
            onCopyRequested: text => clipboard.copy(text)
            onSwitchRequested: (kind, name) => page.switchToRef(kind, name)
            onBranchHereRequested: oidHex => page.startBranchAt(oidHex)
            onTagHereRequested: oidHex => page.startTagAt(oidHex)
            onWorktreeHereRequested: oidHex => page.startWorktreeAt(oidHex)
            onWorktreeAddRequested: (mode, branch, start, path, name) =>
                                        page.addWorktreeAt(path, mode, branch, start, name)
            onDeleteRequested: (kind, id, name, oidHex) => page.deleteRow(kind, id, name, oidHex)
            onDropStashRequested: selector => page.dropStashNow(selector)
            onUpstreamRequested: (branch, counterpart, unsettable) =>
                page.startUpstreamAsk(branch, counterpart, unsettable)
            onPublishRequested: (branch, upstream) => page.startPublishAskFor(branch, upstream)
            onRemoveWorktreeRequested: (path, name, holds) => repoTab.removeWorktree(path, name, holds)
            onRenameFollowed: (from, to) => page.followRenamedDoor(from, to)
        }
    }
    Loader {
        id: remoteMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: RemoteRowMenu {
            heldReason: page.doorsHeldWhy
            runningWhy: page.runningWhy
            treeAtAsk: page.treeAtAsk
            repoTab: repoTab
            githubLinks: githubLinks
            onUrlRequested: name => publishFlow.startEditRemote(name)
            onCopyRequested: text => clipboard.copy(text)
        }
    }
    /// The one door into that menu, from the left panel's rows (`area`: `sidebar` / `peek`). Says whether it opened.
    function openRemoteMenu(name, area) {
        page.noteMenuDoor(area === undefined ? "sidebar" : area, "remote:" + name)
        remoteMenuSeat.active = true
        return remoteMenuSeat.item.offerOn(name)
    }
    /// Which surface raised the standing ref menu: its branch row opens a name box on the row the hand is already on
    /// (デザイン規約 §可否・警告の出し場所).
    property bool refMenuInSidebar: false
    /// The row that raised it, by the key the sidebar's rows answer to (`NavList.keyOf`) — apart from what the menu
    /// acts on: a WORKTREES row opens the menu of the branch it has out, and the box still opens on that row.
    property string refMenuRowKind: ""
    property string refMenuRowId: ""
    /// The one door into that menu: the left panel's rows (`area`: `sidebar` / `peek`) and the automation, whose calls
    /// without it take the graph's side. Says whether it opened. A worktree's row names that worktree (`full` is its
    /// path), which carries its own card (`RefRowMenu.offerOn`). `aim` is a remote the reader named on its own (a
    /// TAGS row's carrier line), which the menu then acts on.
    function openRefMenu(kind, name, full, oidHex, area, aim) {
        const from = area === undefined || area === "" ? "graph" : area
        page.refMenuInSidebar = from !== "graph"
        page.refMenuRowKind = kind
        page.refMenuRowId = full
        page.noteMenuDoor(from, kind + ":" + (full !== "" ? full : name))
        refMenuSeat.active = true
        return kind === "worktree" ? refMenuSeat.item.offerOn(kind, name, full, oidHex, full)
                                   : refMenuSeat.item.offerOn(kind, name, full, oidHex, undefined,
                                                              aim === undefined ? "" : aim)
    }
    /// The left menu's branch renamed with the menu up, the menu gone with it (`RefRowMenu.renameFollowed`): the row
    /// that raised it answers to the new name now (`NavList.keyOf`), so the name boxes its rows open stand on it and
    /// the door names it. A WORKTREES row's key is the worktree's, which a rename leaves as it is.
    function followRenamedDoor(from, to) {
        if (page.refMenuRowKind === "branch" && page.refMenuRowId === from)
            page.refMenuRowId = to
        if (page.menuDoorKey === "branch:" + from)
            page.menuDoorKey = "branch:" + to
    }
    /// A name box on a commit, asked for from a menu, opens where that menu was raised. The sidebar's half
    /// (`inSidebar`, handed the row's kind and id) opens on the row that raised the menu, standing whenever that half
    /// is taken (only its door sets `refMenuInSidebar`); the graph's (`onGraph`) on the commit's row.
    function openNameBoxAt(oidHex, inSidebar, onGraph) {
        if (oidHex === "")
            return
        if (page.refMenuInSidebar)
            inSidebar(page.refMenuRowKind, page.refMenuRowId, oidHex)
        else
            onGraph(oidHex)
    }
    /// A new branch on that commit.
    function startBranchAt(oidHex) {
        page.openNameBoxAt(oidHex, sidebarPane.beginBranchAt, graphPane.startNaming)
    }
    /// And a tag.
    function startTagAt(oidHex) {
        page.openNameBoxAt(oidHex, sidebarPane.beginTagAt, graphPane.startTagging)
    }
    /// And a branch out in a worktree of its own (`Create worktree here…`).
    function startWorktreeAt(oidHex) {
        page.openNameBoxAt(oidHex, sidebarPane.beginWorktreeAt, graphPane.startWorktreeNaming)
    }
    /// What either box sends once a name is typed: the branch made on `oidHex`, out in a worktree where
    /// `newWorktreeFor` puts it. The box has already turned down every name that cannot go there.
    function worktreeFromBox(oidHex, name) {
        const place = worktreesModel.newWorktreeFor(name)
        if (place !== undefined)
            page.addWorktreeAt(place.path, "new", name, oidHex, place.name)
    }
    /// The one door to `git worktree add` (both rows and both boxes). Its answer is waited for by name: landed, the
    /// tab goes and stands in the new worktree (`absorbWorktreeAnswer`).
    function addWorktreeAt(path, mode, branch, start, name) {
        if (path === "" || repoTab.state !== "open" || page.doorsHeldWhy !== "")
            return
        repoTab.addWorktree(path, mode, branch, start, name)
    }

    // ---- what the sidebar's rows ask for ---------------------------
    /// Unconfirmed: a name is not history. A tag and a stash are re-made under the new name by core — the only rename
    /// git has for them.
    function renameRow(kind, id, oid, name) {
        if (kind === "branch") {
            // Which remote this branch speaks for has to be read before the rename: afterwards the row answers to the
            // new name. One already gone from the list is asked about too — the setting still names it.
            page.pendingRenameRemote = branchesModel.upstreamToCarry(id, repoTab.remoteNames)
            page.pendingRenameTo = name
            repoTab.renameBranch(id, name, false)
        } else if (kind === "tag") {
            // Which remote carries this name, read before the rename for the same reason (デザイン規約 §手元の改名の後のリモート).
            page.armRenameTagRemote(id, name)
            repoTab.renameTag(id, name)
        } else if (kind === "stash") {
            repoTab.renameStash(id, oid, name)
        } else if (kind === "worktreeFolder") {
            page.renameWorktreeFolder(id, name)
        } else if (kind === "remote") {
            // A remote branch's box leads to the replace over there, whose question carries both names
            // (`askReplaceRemote`) — so, unlike every rename, the box comes down now: left waiting, a dismissed
            // question would leave it standing with nothing coming (デザイン規約 §答えの要らない報せ).
            page.noteRenameLanded()
            page.askReplaceRemote(id, name)
        }
    }

    /// A worktree on no branch takes the folder `name` beside its own (`git worktree move`); `path` is where it is.
    /// Both boxes turned down every name that cannot go there. Its answer is waited for by name (`absorbWorktreeMove`).
    function renameWorktreeFolder(path, name) {
        const facts = worktreesModel.worktreeFacts(path)
        const place = worktreesModel.renamedWorktreeFor(path, name)
        // Gone from the listing since the box opened (or no folder's name, which the box never sends): nothing to ask,
        // and the box stops waiting. What came into the way at the new path since is core's to turn down, with its
        // report (`worktrees::check_move`).
        // A box opened before the doors were held (another rename out) is not sent either.
        if (facts === undefined || place === undefined || page.doorsHeldWhy !== "") {
            page.noteRenameRefused()
            return
        }
        page.worktreeMoveOut = repoTab.moveWorktree(path, place.path, facts.name)
        if (!page.worktreeMoveOut)
            page.noteRenameRefused()
    }
    /// The answer to a worktree's rename (`RepoTab.worktreeMoveAnswer`, undefined for none): either box comes down on
    /// it, or — refused — stays open holding the name with no refusal on it, so Enter tries again; the bar says why and
    /// points at the log for git's words (デザイン規約 §答えの要らない報せ). Nothing goes into the box: the name's own faults were
    /// turned down before the press, and what is left is git's alone — a worktree holding submodules is the usual one,
    /// which knowing beforehand would cost a read at every open. **A tab standing in that worktree stands again**: its
    /// session stood down for the move (`RepoSession::move_worktree`), so it takes up the new folder — or the old one,
    /// refused — on a session of its own, the graph staying (デザイン規約 §タブの所作「立ち位置を変えても、画面は建て直さない」).
    function absorbWorktreeMove() {
        const answer = repoTab.worktreeMoveAnswer()
        if (answer === undefined)
            return
        page.worktreeMoveOut = false
        const failed = repoTab.writeAnswerFailed(answer)
        if (failed) {
            page.noteRenameRefused()
            page.tellRefusal(answer)
        } else {
            page.noteRenameLanded()
        }
        if (repoTab.worktreeMoveStanding)
            page.worktreeFollowRequested(failed ? repoTab.worktreeMoveFrom : repoTab.worktreeMoveTo)
    }
    /// The tab stands in `path` again — the worktree it stood in, its folder renamed or kept
    /// (`TabsModel.followWorktree`).
    signal worktreeFollowRequested(string path)

    /// The remote branch a just-renamed local one spoke for, and the name it took — the question about what the remote
    /// does with it waits until git says the local rename landed.
    property string pendingRenameRemote: ""
    property string pendingRenameTo: ""

    /// The same for a tag: the remote carrying the name too, and the two names — the tags section's answers
    /// (`NavSectionModel.tagSides`), which stop for the old name once the rename lands, so taken before it.
    property string pendingRenameTagRemote: ""
    property string pendingRenameTagFrom: ""
    property string pendingRenameTagTo: ""
    /// And the commit the bar marks, **taken before the write** (a rename never moves the object, [`tag::rename`]):
    /// asked for the new name when the answer lands, it races the read that rebuilds the rows.
    property string pendingRenameTagOid: ""
    /// Reads them **only where the pair over there would change the name and nothing else** (デザイン規約
    /// §手元の改名の後のリモート): the remote's copy must stand where ours does, or the push and delete would move the
    /// mark too; and the new name must be free there, or the push is refused outright.
    function armRenameTagRemote(from, to) {
        page.forgetRenameTagRemote()
        const remote = repoTab.defaultRemote
        if (remote === "" || from === to)
            return
        if (tagsModel.tagSides(from) !== "both" || tagsModel.remoteTagDrift(remote, from) !== "")
            return
        // The new name taken anywhere: here git's rename refuses first, over there the push would.
        if (tagsModel.tagSides(to) !== "")
            return
        page.pendingRenameTagRemote = remote
        page.pendingRenameTagFrom = from
        page.pendingRenameTagTo = to
        page.pendingRenameTagOid = tagsModel.oidOfName(from)
    }
    function forgetRenameTagRemote() {
        page.pendingRenameTagRemote = ""
        page.pendingRenameTagFrom = ""
        page.pendingRenameTagTo = ""
        page.pendingRenameTagOid = ""
    }
    /// Both halves let go of — the branch's wait and the tag's: the rename they wait on will not land.
    function forgetRenameCarry() {
        page.pendingRenameRemote = ""
        page.pendingRenameTo = ""
        page.forgetRenameTagRemote()
    }

    /// A rename's box stays open until git answers; these are the two words back to it. **Both boxes are told** and
    /// only the one that was waiting acts — the left menu's row and the graph's chip know nothing of each other
    /// (デザイン規約 §答えの要らない報せ).
    function noteRenameLanded() {
        sidebarPane.noteRenameLanded()
        graphPane.renameLanded()
    }
    function noteRenameRefused() {
        sidebarPane.noteRenameRefused()
        graphPane.renameRefused()
    }

    /// Replacing a branch on a remote with one under a new name, which git has no command for: core pushes the new
    /// name and deletes the old, so the question is asked first and its answer is pressed through — this is the one
    /// write here that another machine keeps (デザイン規約 §回数押し). Composed of a push, a `push --delete` and a
    /// `--set-upstream-to` (`remote::replace_remote_branch`).
    ///
    /// Raised from a remote branch's name box — the slow second click on its REMOTES row, a graph chip or the stacked
    /// list's row, the gesture every rename shares. The press sends the remote-tracking ref (`replace_remote_branch`):
    /// the question is about a name, so local commits further on stay here.
    ///
    /// **Never named a rename** (user decision — ask before changing), on screen or in identifiers (this, the verbs
    /// `replace-remote*`): the far side sees a create and a delete. Rename is kept for real renames only — a local
    /// branch, tag or stash, the shared gesture and box that open one (`renameRow` / `beginRename` /
    /// `editMode "rename"`), and the question after one (`RenameCarryFlow`, `rename_carry`, the verbs
    /// `rename-local-upstream*` / `rename-tag-remote*`, named for the local rename that sets them off).
    function askReplaceRemote(remoteRef, name) {
        // The new side's name, as `carryBranchRenameOver` asks for it, and the same stop at a name already over
        // there, which the box turns down: a plain push onto an existing name passes as a success wherever it
        // fast-forwards.
        const comes = GitFacts.remoteRefBeside(remoteRef, name, repoTab.remoteNames)
        if (comes === "" || comes === remoteRef
                || remotesModel.hasRemoteBranch(GitFacts.remoteBranchOf(remoteRef, repoTab.remoteNames).remote, name))
            return
        // The commit the bar marks, taken as it is raised: the old name's delete is leased to it
        // (`remote::replace_remote_branch`), so a remote that moved while the question stood is refused. A row gone
        // from the list has none, and nothing is asked: this question has the one answer, which could only be turned
        // away (デザイン規約 §リモートブランチの差し替え).
        const shown = remotesModel.oidOfName(remoteRef)
        if (shown === "")
            return
        page.startRowAsk(
            shown,
            page.replaceQuestion(remoteRef, comes),
            qsTr("The old branch is deleted, not moved."),
            false,
            qsTr("Replace"),
            function () { repoTab.replaceRemoteBranch(remoteRef, name, shown) },
            true,
            page.replaceTip(comes, remoteRef),
            null, "", "", Theme.textLink, "",
            { label: Words.marksOf(page.replaceQuestion, [[remoteRef, "remote"], [comes, "remote"]]),
              tip: Words.marksOf(page.replaceTip, [[comes, "remote"], [remoteRef, "remote"]]) })
        page.replaceRemoteAsked(remoteRef, name)
    }
    /// The replace question: the old remote branch `was`, the new one `comes` — one spelling for the words and the
    /// seats of their marks.
    function replaceQuestion(was, comes) {
        //: %1 and %2 are remote branches, e.g. origin/main. The old name goes and the new one is made.
        return qsTr("Replace %1 with %2?").arg(was).arg(comes)
    }
    /// The replace question's pill tip: the new remote branch `comes`, the old one `was`. Why this is not a forge
    /// rename lives here (デザイン規約 §回数押し): the bar's line is taken by what is lost (`The old branch is deleted,
    /// not moved.`), with no room for "the far side sees a create and a delete; the pull request stays behind". The
    /// same words as the "delete and create" answer after a local rename (`RenameCarryFlow`).
    function replaceTip(comes, was) {
        return qsTr("%1 to replace. %2 goes up first, then %3 comes off — so a push the far side turns down leaves the old name where it is. Anything it carried — an open pull request, a running check — does not follow the new name.")
            .arg(Words.pressCount(Metrics.holdPressesWarned)).arg(comes).arg(was)
    }
    /// Automation: the question above was raised, and the two names it is between.
    signal replaceRemoteAsked(string from, string to)

    /// The failures a report has already answered — armed by the report when the row it is about has not reached the
    /// log yet (`absorbWriteResult`), spent by that row's own arrival. Counted, because the refusal and the write
    /// result arrive on separate paths, in no fixed order.
    property int answeredFailures: 0
    /// The answer to the plain delete a menu stayed up for — its own answer (`RepoTab.branchDeleteAnswer`, undefined
    /// for none), which no count of answers could tell from the delete before it.
    function absorbBranchDelete() {
        const answer = repoTab.branchDeleteAnswer()
        if (answer === undefined)
            return
        // Landed: the menu goes by itself (`BranchDeleteRow`). Refused by git itself: the card turns the row into the
        // held `-D` and **the row is the answer** — nothing is raised over it (git measures against the upstream, so
        // a refusal need not mean commits would be lost).
        if (!repoTab.writeAnswerFailed(answer))
            return
        // Left: the far side keeping the branch, which the row cannot answer — the bar carries it.
        page.reportRefusal(answer)
    }

    // ---- what the window is already showing as gone ------------------
    //
    // **A delete takes the row away at the press** (デザイン規約 §消す操作は先に画面から消す): `busyCount` does not
    // cover the refs read and walk after the write (ci/baseline/code-costs-windows-x64.md).
    //
    // **Which rows, and for how long, is `ops::StandIn`'s** (per tab in the hub, so it outlives this page); this page
    // hands each name to the list that draws it. One key per kind — a delete touches at most one of each; empty is
    // nothing shown as gone.
    readonly property string goneBranch: repoTab.goneBranch
    readonly property string goneRemote: repoTab.goneRemote
    readonly property string goneTag: repoTab.goneTag
    readonly property string goneStash: repoTab.goneStash
    readonly property string goneWorktree: repoTab.goneWorktree
    /// What the worktree going marks elsewhere: its chip on the graph where it is on no branch, the branch it has out
    /// where it has one (that branch's frame on the graph, its mark in the branches list).
    readonly property string goneWorktreeChip: repoTab.goneWorktreeChip
    readonly property string goneWorktreeBranch: repoTab.goneWorktreeBranch
    /// The chips that go with the rows are the graph model's to key (`GraphModel.setGone` / `encode::gone_keys`). A
    /// dropped stash has no chip: it is a graph row, not a name on one, which the session lays out again without
    /// (`session::leaving`).
    function syncGone() {
        graphModel.setGone(repoTab.goneRows)
    }
    onGoneBranchChanged: {
        branchesModel.setHidden(page.goneBranch)
        page.syncGone()
    }
    onGoneRemoteChanged: {
        remotesModel.setHidden(page.goneRemote)
        page.syncGone()
    }
    onGoneTagChanged: {
        tagsModel.setHidden(page.goneTag)
        page.syncGone()
    }
    onGoneStashChanged: stashesModel.setHidden(page.goneStash)
    onGoneWorktreeChanged: worktreesModel.setHidden(page.goneWorktree)
    onGoneWorktreeChipChanged: page.syncGone()
    onGoneWorktreeBranchChanged: {
        branchesModel.setReleased(page.goneWorktreeBranch)
        page.syncGone()
    }

    // ---- what the window is already showing as staged ------------------
    //
    // **A stage moves its rows at the press** (デザイン規約 §バケツごとの一覧と、ペインの分け方): the rows, the buckets' counts and the Commit
    // button's file count move on the press's turn, and git runs behind it. **Which rows, and for how long, is
    // `ops::StageAhead`'s** (per tab in the hub); each move of what it holds moves `RepoTab.stagingSeq`, and the three
    // lists, the counts and the open diff (a hunk or a line pressed, デザイン規約 §diff の中のステージ) draw what they
    // hold again on that turn. So do a discard's rows (§消す操作は先に画面から消す). The diff open on a moved file
    // follows on that turn, where the status after the presses will have it (`followEmptySide`), and waits in place
    // for that status only where the list does not show where the file goes yet (`followLanding`).
    readonly property int stagingSeq: repoTab.stagingSeq
    onStagingSeqChanged: page.restage()
    function restage() {
        conflictsModel.restage()
        unstagedModel.restage()
        stagedModel.restage()
        worktree.restage()
        diffModel.restage()
        // A press that moved no row moves no list: the neighbour is noted here too (`noteDiffNeighbour`).
        page.noteDiffNeighbour()
        page.followEmptySide()
    }

    /// Automation: holds a row taken away before git answers, for its picture — asked before the press, since a demo
    /// repository answers before the grab (verify-ui スキル §壊れない動詞の実装と反復). Held by withholding
    /// `listingDrawn`, the news the owner puts the rows down on.
    property bool holdGoneRows: false

    /// Automation only: whether the last delete asked of this page went out to git or was dropped — "did the input
    /// land" for both entrances (app-ui.md §UI 自動化), as `RefBranchMenu.deleteAsked` is for the menu. A dropped
    /// request leaves nothing for a write barrier to wait on.
    property bool deleteRowAsked: false
    function deleteRow(kind, id, name, oidHex) {
        page.deleteRowAsked = false
        if (kind !== "branch")
            return
        // The menu stays open for git's answer, so it can be clicked twice; the second click is the same request.
        if (repoTab.busyCount > 0)
            return
        // `-d`: git's refusal turns the row into the held `-D` (デザイン規約 §左メニューの所作). The tab keeps which
        // branch the answer is about and the BRANCH card reads it there for the delete rows (`RefBranchMenu.refused`);
        // the same slot takes the row off screen (`ops_delete`).
        repoTab.deleteBranch(id, false)
        page.deleteRowAsked = true
    }
    /// Held (デザイン規約 §回数押し).
    function dropStashNow(ref) {
        repoTab.dropStash(ref)
        if (page.selectedStashRef === ref)
            page.selectedStashRef = ""
    }

    // ---- context menu on a worktree file row --------------------
    function openFileMenu(bucket, path) {
        // A right-click is a click: it walks away from a standing question, which may be about another row.
        page.stopRowAsk()
        page.noteMenuDoor("files", GitFacts.fileKey(bucket, path))
        fileMenuSeat.active = true
        fileMenuSeat.item.offer(bucket, path)
    }
    Loader {
        id: fileMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: FileRowMenu {
            runningWhy: page.runningWhy
            repoTab: repoTab
            worktree: worktree
            treeAtAsk: page.treeAtAsk
            readRevision: page.menuReadRevision
            wipPane: wipPane
            onMergeToolWanted: page.gitSettingsRequested()
            onCopyRequested: text => clipboard.copy(text)
        }
    }

    // ---- context menu on a commit's changed file ---------------------
    /// The one door into that menu: a right-click on a row of a commit's changed files, and the menu key on the one
    /// the arrows stand on. Its rows are GitHub's links alone, so with none to offer nothing opens and the row is left
    /// as it was. Otherwise it acts on what is lit: the file is read first where it is not (デザイン規約 §メニュー).
    /// Says whether it opened.
    function openCommitFileMenu(path, origPath, change) {
        // As `openFileMenu`.
        page.stopRowAsk()
        const links = page.commitFileLinks(path, change)
        if (links.length === 0 || links[0].holders.length === 0)
            return false
        if (page.diffKind !== "commit" || page.diffPath !== path)
            page.openDiff("commit", path, origPath)
        // Not read — a plan holds the centre (`openDiff`): the row is not lit, so no menu acts on it.
        if (page.diffKind !== "commit" || page.diffPath !== path)
            return false
        page.noteMenuDoor("changes", path)
        commitFileMenuSeat.active = true
        return commitFileMenuSeat.item.offerOn(links)
    }
    /// The link a commit's changed file offers on GitHub (デザイン規約 §GitHub のページへのリンク): the file as it stands
    /// in the commit — or the newer of two compared —, and a file it deletes as it stood before. None for a stack of
    /// several commits' changes, which name no one commit.
    function commitFileLinks(path, change) {
        if (detailsModel.selectionCount > 2)
            return []
        // A file the commit deletes is on the older side alone.
        const side = page.diffSideAt(change !== "D")
        return side === undefined ? [] : githubLinks.ofFile(side.held, side.at, path)
    }
    Loader {
        id: commitFileMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: CommitFileMenu {
            onCopyRequested: text => clipboard.copy(text)
        }
    }

    // ---- context menu on the diff's own text ------------------------
    /// The menu acts on the selection, settled before asking (`DiffTextSelect.askMenu`), so no row travels here.
    function openCodeMenu() {
        page.stopRowAsk()
        page.noteMenuDoor("diff", "")
        diffMenuSeat.active = true
        diffMenuSeat.item.standGithub(page.diffLineLinks())
        diffMenuSeat.item.offer()
    }
    /// The link the selected lines offer on GitHub (デザイン規約 §GitHub のページへのリンク): the lines of a commit's file
    /// — one commit's, or the newer of two compared — or of the file before it where only removed lines are selected.
    /// None for a working tree's file, nor for a stack of several commits' patches, which name no one commit.
    function diffLineLinks() {
        if (!page.diffShown || page.diffKind !== "commit" || detailsModel.selectionCount > 2)
            return []
        const span = diffModel.selectionLines()
        if (span === undefined)
            return []
        const side = page.diffSideAt(span.newer)
        if (side === undefined)
            return []
        const path = span.newer || page.diffOrigPath === "" ? page.diffPath : page.diffOrigPath
        return githubLinks.ofLines(side.held, side.at, path, span.first, span.last)
    }
    /// Where a link to one side of the commit card's diff points (`GitFacts.githubSideAt`), `undefined` for none.
    function diffSideAt(newer) {
        return GitFacts.githubSideAt(newer, detailsModel.comparing, detailsModel.shaHex, detailsModel.parentHex,
                                     detailsModel.compareFrom, detailsModel.compareTo)
    }
    Loader {
        id: diffMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: DiffRowMenu {
            diffModel: diffModel
            onCopyRequested: text => clipboard.copy(text)
        }
    }

    // ---- context menu on a graph row -------------------------------
    /// The first chip a graph row draws (what a right-click hands over — `GraphRowDelegate.nameChip`), for callers with
    /// no row in hand; null where it draws neither a ref nor a worktree's folder. Chips already taken off screen
    /// are filtered as the delegate does, so a gone name puts no card up (デザイン規約 §消す操作は先に画面から消す).
    /// The delegate's own answer, from the model: a "first of each kind" search would aim at a name not on screen.
    /// The menu reads only `chip.kind` (`GitFacts.namedKind`) and `chip.name`, so a chip a run builds by hand needs no
    /// more. Headless: `chip-menu <branch>` / `chip-menu-current` / `list-menu <row>:<card row>`.
    function rowChipAt(oidHex) {
        const row = graphModel.rowOf(oidHex)
        if (row < 0)
            return null
        const shown = GitFacts.chipsShown(graphModel.labelsAt(row), graphModel.goneChips)
        if (shown.length === 0)
            return null
        return GitFacts.namedKind(shown[0].kind) === "" ? null : shown[0]
    }

    /// The one door into that menu: graph rows, the rows of a chip's stacked list (`onList` — the menu then stands on
    /// the list rather than over it, `menuArea` = `refList`), and automation. `chip` is the name it is aimed at; null
    /// aims it at nothing, and left out it is asked of the model (`rowChipAt`). The first level is the same commit
    /// either way — the name only picks which card comes up (デザイン規約 §グラフ行の右クリック).
    ///
    /// While the discard log's entry is on the graph, a row it does not draw as a commit has no menu
    /// (`GraphModel.provisionalMenuShutAt`), and a row it does is aimed at no name: what the graph is showing is the
    /// commit, and the rows that act on a name would reach past it.
    function openRowMenu(oidHex, chip, onList) {
        if (graphModel.provisionalMenuShutAt(graphModel.rowOf(oidHex)))
            return
        // A right-click in the chip's card is one on its row: the node comes on before the menu, as the row's own
        // right-click brings it (`GraphRowDelegate.askMenu`) — and, as there, not on a row that answers with no menu.
        if (onList === true)
            graphPane.revealNode(oidHex)
        page.noteMenuDoor(onList === true ? "refList" : "graph", oidHex)
        const named = graphModel.provisionalOn ? null : chip === undefined ? page.rowChipAt(oidHex) : chip
        commitMenuSeat.active = true
        const menu = commitMenuSeat.item
        const kind = named ? GitFacts.namedKind(named.kind) : ""
        // The chip of a worktree with no branch carries only its folder's name: the worktree is the one of that name
        // standing on this commit.
        const target = kind === "" ? ""
                     : kind === "worktree" ? worktreesModel.worktreeAt(named.name, oidHex) : named.name
        menu.targetKind = target === "" ? "" : kind
        menu.targetName = target
        commitMenuState.openRowMenu(oidHex)
    }
    /// A right-click on a row inside a choice of two: the menu on the pair, which takes them in the order the right
    /// pane compares them (`DetailsModel.compareFrom` / `compareTo`). Where it has nothing to offer, the press is any
    /// other right-click's — the choice down to the row and the row's menu (デザイン規約 §複数のコミットを選ぶ).
    function openChoiceMenu(oidHex, atRow, chip) {
        // As `openFileMenu`, and as the row's own press does.
        page.stopRowAsk()
        page.noteMenuDoor("graph", oidHex)
        commitMenuSeat.active = true
        const menu = commitMenuSeat.item
        menu.releaseShape()
        if (detailsModel.comparing
                && menu.offerChoice(githubLinks.ofPair(detailsModel.compareFrom, detailsModel.compareTo)))
            return
        page.pickRow(oidHex, atRow, Qt.NoModifier)
        page.openRowMenu(oidHex, chip)
    }
    /// The menu on another worktree's uncommitted row, which is no commit: its own (デザイン規約 §別の worktree を読む).
    function openCarriedMenu(path, name) {
        page.noteMenuDoor("graph", path)
        commitMenuSeat.active = true
        commitMenuState.openCarriedMenu(path, name)
    }

    readonly property CommitMenuState commitMenuState: CommitMenuState {
        id: commitMenuState
        repoTab: repoTab
        worktree: worktree
        treeAtAsk: page.treeAtAsk
        readRevision: page.menuReadRevision
        graphModel: graphModel
        worktreesModel: worktreesModel
        branchesModel: branchesModel
        tagsModel: tagsModel
        remotesModel: remotesModel
        githubLinks: githubLinks
        // Null until the menu is first raised; the one function that reads it is the door that raises it.
        menu: commitMenuSeat.item
    }
    /// What every GITHUB card stands on, asked as its menu opens (デザイン規約 §GitHub のページへのリンク).
    readonly property GitHubLinks githubLinks: GitHubLinks {
        id: githubLinks
        repoTab: repoTab
        graphModel: graphModel
        remotesModel: remotesModel
        tagsModel: tagsModel
    }

    Loader {
        id: commitMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: CommitRowMenu {
            // Held on the same answer as the left pane's rows: a reset or drop mid-replay is the same accident a
            // switch would be.
            heldReason: page.doorsHeldWhy
            runningWhy: page.runningWhy
            onWaitEnded: commitMenuState.standAgain("wait")
            onCopyRequested: text => clipboard.copy(text)
            branch: worktree.heldBranch
            oid: commitMenuState.menuOid
            stashRef: commitMenuState.menuStashRef
            published: commitMenuState.menuPublished
            rebasePublished: commitMenuState.menuRebasePublished
            canSwitch: commitMenuState.menuCanSwitch
            switchAsks: commitMenuState.menuSwitchAsks
            heldLeaf: commitMenuState.menuHeldLeaf
            heldHome: commitMenuState.menuHeldHome
            pullRuns: commitMenuState.menuPullRuns
            pullOut: commitMenuState.menuPullOut
            pullUpstream: commitMenuState.menuPullUpstream
            canCherryPick: commitMenuState.menuCanCherryPick
            canRevert: commitMenuState.menuCanRevert
            canIntegrate: commitMenuState.menuCanIntegrate
            canEditHistory: commitMenuState.menuCanEditHistory
            canMoveBranch: commitMenuState.menuCanMoveBranch
            canBranchHere: commitMenuState.menuCanBranchHere
            opStanding: commitMenuState.menuOpStanding
            stashCanWrite: commitMenuState.menuStashCanWrite
            hardResetTakes: commitMenuState.menuHardResetTakes
            hardResetNotCopied: commitMenuState.menuHardResetNotCopied
            tipHeldElsewhere: commitMenuState.menuTipHeldElsewhere
            // git's answers to the branch's plain delete, live while the menu stands (`RefBranchMenu`).
            refusedDelete: repoTab.branchDeleteRefused
            landedDelete: repoTab.branchDeleteLanded
            checkedBranch: repoTab.branchDeleteAsked
            checkedMerged: repoTab.branchDeleteMerged
            // The same road the row's double-click takes, held on the same answers (`switchToRef`).
            onSwitchRequested: (kind, name) => page.switchToRef(kind, name)
            // Straight to the graph row: this menu is only ever raised on one.
            onBranchHereRequested: oidHex => graphPane.startNaming(oidHex)
            onWorktreeHereRequested: oidHex => graphPane.startWorktreeNaming(oidHex)
            onWorktreeAddRequested: (mode, branch, start, path, name) =>
                                        page.addWorktreeAt(path, mode, branch, start, name)
            onTagHereRequested: oidHex => graphPane.startTagging(oidHex)
            onSquashRequested: oidHex => page.squashCommit(oidHex)
            onDropRequested: oidHex => page.dropCommit(oidHex)
            onPlanRequested: oidHex => page.startRebasePlan(oidHex)
            onResetRequested: mode => page.moveBranchHere(mode)
            onApplyStashRequested: selector => repoTab.applyStash(selector)
            onPopStashRequested: selector => page.popStash(selector)
            onDropStashRequested: selector => page.dropStashNow(selector)
            // Another worktree's uncommitted row: the door its double-click opens, and its work applied here.
            onCarriedOpenRequested: path => page.openRepositoryPathRequested(path)
            onCarriedApplyRequested: path => {
                // The report names the worktree by its folder alone; whether it was the repository's own, and what it
                // goes by, is known here on the way in (`showReport`). No second apply can be asked before the report:
                // the menus hold while a write is out.
                page.appliedFrom = worktreesModel.worktreeName(path)
                repoTab.applyCarried(path)
            }
            // The BRANCH card's signals, raised by its rows and the first level's, answered where the ref menu's are.
            onDeleteRequested: (kind, id, name, oidHex) => page.deleteRow(kind, id, name, oidHex)
            onUpstreamRequested: (branch, counterpart, unsettable) =>
                page.startUpstreamAsk(branch, counterpart, unsettable)
            onUnsetUpstreamRequested: branch => repoTab.unsetUpstream(branch)
            onPushBranchRequested: (branch, force, expect) => repoTab.pushNamed(branch, force, expect)
            onPublishRequested: (branch, upstream) => page.startPublishAskFor(branch, upstream)
            onRemoveWorktreeRequested: (path, name, holds) => repoTab.removeWorktree(path, name, holds)
            onCherryPickRequested: oidHex => repoTab.cherryPick(oidHex)
            onRevertRequested: oidHex => repoTab.revert(oidHex)
            onMergeRequested: ref => repoTab.merge(ref, false, false, "")
            onRebaseRequested: ref => repoTab.rebase(ref, "")
            onPullRequested: branch => repoTab.pull(branch)
            onCheckDeleteRequested: branch => repoTab.checkBranchDelete(branch)
            onForceDeleteRequested: branch => repoTab.deleteBranch(branch, true)
            onDeleteRemoteRequested: (remoteRef, expect) => repoTab.deleteRemoteBranch(remoteRef, expect)
            onDeleteEverywhereRequested: (branch, remoteRef, forced, expect) =>
                repoTab.deleteBranchEverywhere(branch, remoteRef, forced, expect)
            onPushTagRequested: (remote, tag, expect) => repoTab.pushTag(remote, tag, expect)
            onDeleteTagRequested: tag => repoTab.deleteTag(tag)
            onDeleteRemoteTagRequested: (remote, tag, rowGoes, expect) =>
                repoTab.deleteRemoteTag(remote, tag, rowGoes, expect)
            onDeleteTagEverywhereRequested: (remote, tag, expect) => repoTab.deleteTagEverywhere(remote, tag, expect)
        }
    }

    ClipboardHelper {
        id: clipboard
    }

    // ---- the second click, spaced, on a graph row -------------------
    /// The row chip's name goes into a box where the chip is (デザイン規約 §グラフ行のダブルクリック / §左メニューの所作),
    /// from the row or the card its chip unfolds into. The box holds the ref's own name — a remote branch without its
    /// remote (`GitFacts.remoteBranchOf`), the shape `renameRow` takes, as from the left menu. Headless `graph-reclick`
    /// (to the box) / `graph-rename` (through to git) / `graph-reclick-scrolled` (the row scrolled away first).
    function startRename(oidHex, chip) {
        // An entry of the discard log picked under a second click's wait: the wait ends on a graph that renames nothing
        // (`GraphList.noteClick`).
        if (repoTab.state !== "open" || !chip || graphModel.provisionalOn)
            return
        // The chip of a worktree with no branch renames the worktree's folder (`startFolderRename`).
        if (GitFacts.namedKind(chip.kind) === "worktree") {
            page.startFolderRename(oidHex, chip)
            return
        }
        const kind = GitFacts.refKind(chip.kind)
        // The detached-HEAD marker names no ref.
        if (kind === "")
            return
        const id = chip.name
        // The card the chip unfolds into is standing on the column the box opens in.
        rowHost.closeRefList()
        graphPane.startRenaming(oidHex, kind, id,
                                kind === "remote" ? GitFacts.remoteBranchOf(id, repoTab.remoteNames).branch : id)
        // デザイン規約 §左メニューの所作「開く行は必ず見える所へ送る」 — the gesture's wait is long enough to scroll away in.
        const row = graphModel.rowOf(oidHex)
        if (row >= 0)
            graphPane.showRowSoon(row)
    }
    /// The chip of a worktree on no branch, named by its folder: the box opens on the folder's name (`Worktree name`),
    /// or — where git would keep the folder — says why on the chip instead, or does nothing on the repository's own
    /// worktree (`GitFacts.worktreeRename`, デザイン規約 §グラフ行のダブルクリック). The chip carries the folder; which
    /// worktree it is, is told by the commit it stands on (`NavSectionModel.worktreeAt`).
    function startFolderRename(oidHex, chip) {
        const facts = worktreesModel.worktreeFacts(worktreesModel.worktreeAt(chip.name, oidHex))
        if (facts === undefined || page.doorsHeldWhy !== "")
            return
        const answer = GitFacts.worktreeRename(facts.change)
        rowHost.closeRefList()
        if (answer === "box")
            graphPane.startRenaming(oidHex, "worktreeFolder", facts.full, facts.name)
        else if (answer !== "")
            graphPane.withholdRename(oidHex, Words.worktreeRenameWithheld(answer, facts.name), facts.name)
        else
            return
        const row = graphModel.rowOf(oidHex)
        if (row >= 0)
            graphPane.showRowSoon(row)
    }
    /// What the graph's name box may accept, and why not (`NameVerdict`).
    readonly property NameVerdict graphNameVerdict: NameVerdict {
        id: graphNameVerdict
        repoTab: repoTab
        remotesModel: remotesModel
        worktreesModel: worktreesModel
        branchesModel: branchesModel
        tagsModel: tagsModel
        open: graphPane.namingOid !== ""
        mode: graphPane.namingMode
        kind: graphPane.namingKind
        rowId: graphPane.namingId
        text: graphPane.namingText
    }

    // ---- double-click on a graph row -------------------------------
    // A row with no chip is offered one.
    function rowDoubleClicked(oidHex, chip) {
        if (repoTab.state !== "open")
            return
        if (chip) {
            page.activateChip(chip)
            return
        }
        // Held on the same answers as the switch (`switchToRef`): the box is a branch about to be written, and a plan's
        // run is out before `busyCount` says anything.
        if (repoTab.busyCount === 0 && page.doorsHeldWhy === "")
            graphPane.startNaming(oidHex)
    }

    RowHoverHost {
        id: rowHost
        graphPane: graphPane
        branchesModel: branchesModel
        remotesModel: remotesModel
        tagsModel: tagsModel
        tagAgainst: repoTab.defaultRemote
        menuDoor: page.menuDoor
        chosenHand: detailsPane.hand
        keepBuilt: page.keepBuilt
        // As the row's own double-click answers (`GraphRowDelegate.doubleClick`): while the discard log's entry is on
        // the graph no name is moved to, a commit it draws is offered a new one, and the rows it leaves answer nothing.
        onRecordActivated: (chip, oidHex) => {
            if (!graphModel.provisionalOn)
                page.activateChip(chip)
            else if (!graphModel.provisionalMenuShutAt(graphModel.rowOf(oidHex)))
                page.rowDoubleClicked(oidHex, null)
        }
        // A plain click in the card is a click on its row: every name in it is on that one commit.
        onRecordChosen: (oidHex, atRow) => page.activateRow(oidHex, atRow)
        // The note under a cut message: the row's click, then the pane holding the whole message names itself — the
        // card's sentence is gone by the time the reader arrives (デザイン規約 §hover のツールチップ「送った先は名乗る」).
        onMessageAsked: (oidHex, atRow) => {
            page.activateRow(oidHex, atRow)
            // From a graph row's card, the row's node comes with the click as with the row's own (`revealNode`).
            if (atRow >= 0)
                graphPane.revealNode(oidHex)
            detailsPane.callAttention()
        }
        onRecordMenuAsked: (oidHex, chip) => page.openRowMenu(oidHex, chip, true)
        // The line under a card's name: the same jump its left-panel row makes.
        onMateFollowed: oidHex => page.jumpToRef(oidHex)
    }
    // The list a menu was raised on went — the rows moved under it: the menu, a row of that list's, has nothing left
    // to stand on and goes with it (as `CommitRowMenu.withdraw` takes a menu off a commit that is gone).
    Connections {
        target: rowHost
        function onListKeyChanged() {
            if (page.menuArea === "refList" && rowHost.listKey === "" && commitMenuSeat.item !== null)
                commitMenuSeat.item.withdraw()
        }
    }

    // ---- rewriting one commit --------------------------------------
    // Run straight even for a pushed commit (デザイン規約 §回数押し「ローカルの履歴書き換えはクリック」).
    function squashCommit(oidHex) {
        repoTab.squashIntoParent(oidHex)
    }

    /// One place for both ways in: the row is a hold or a click depending on whether anything else holds the branch
    /// tip, and runs the same either way (デザイン規約 §履歴を合流させる).
    function dropCommit(oidHex) {
        repoTab.dropCommit(oidHex)
    }

    // ---- taking the branch back to an earlier commit ----------------
    function moveBranchHere(mode) {
        repoTab.resetTo(commitMenuState.menuOid, mode)
    }

    // ---- editing the selected commit's message ---------------------
    // Run straight even for a pushed commit, as `squashCommit`.
    function saveMessage(oidHex, subject, body) {
        // A reword leaves the history's shape alone, so the rewritten commit lands on the same row.
        page.rewordRow = graphModel.rowOf(oidHex)
        repoTab.rewordCommit(oidHex, subject, body)
    }
    // Row to re-select once the rewritten graph arrives (-1 = none).
    property int rewordRow: -1

    // What the details pane's boxes do with the commit on screen, in core's word (`offers::message_edit`): `amend`
    // where typing lands (HEAD's own commit only, user decision), `stash` / `not-head` / `standing` where it does not,
    // `""` with no message. A live binding: a commit that stops being HEAD stops taking keys there and then. No
    // `replays` word: the boxes run only the amend, so the save button's phrase is `commit --amend` alone (core's
    // `RepoSession::reword` replays a commit under HEAD, but nothing in the UI hands it one).
    readonly property string messageEdit: GitFacts.messageEdit(
        !page.blank && repoTab.state === "open", worktree.standing, detailsModel.shaHex, page.selectedStashRef !== "")

    // Asked on every selection and read only when the answer names the commit on screen — verifying runs gpg or
    // ssh-keygen and lands after the details. A signature changes only with the hash, so nothing asks twice.
    function askSignature(oidHex) {
        if (oidHex !== "" && repoTab.state === "open")
            repoTab.checkSignature(oidHex)
    }
    readonly property bool signatureIsForSelection:
        detailsModel.shaHex !== "" && repoTab.signatureOid === detailsModel.shaHex
    readonly property string selectedSignatureKind:
        page.signatureIsForSelection ? repoTab.signatureKind : ""
    readonly property string selectedSignatureCode:
        page.signatureIsForSelection ? repoTab.signatureCode : ""
    readonly property string selectedSignatureSigner:
        page.signatureIsForSelection ? repoTab.signatureSigner : ""

    // Whether a remote already has the selected commit (the save row's warning). Only HEAD takes typing, so this is
    // HEAD's own answer — nothing is asked on the keystroke. Under a plan the reword chip carries the plan's warning.
    readonly property bool selectedPublished: !page.planActive && page.selectedOid !== ""
                                              && page.selectedOid === worktree.headOid && worktree.headPublished

    // An avatar assignment is not git's, so no refresh brings it: everything that shows a face re-reads the store.
    Connections {
        target: AppBackend
        function onAvatarsChanged() {
            graphModel.refreshAvatars()
            detailsModel.refreshAvatar()
            // The commit editor wears the identity's face.
            repoTab.refreshAvatar()
            // The chosen list's faces are packed off the rows.
            page.refreshChosenRecords()
        }
    }

    // ---- the harness -----------------------------------------------
    // This tab's verification harness (`HarnessSeat`, app-ui.md): its module cannot see this file's ids, so everything
    // the verbs and measurements act on is handed over here. Menus and the plan's face go as their seats, built only
    // when asked for (`keepBuilt`).
    HarnessSeat {
        id: harness
        anchors.fill: parent
        part: "PageHarness.qml"
        wanted: AppBackend.harnessPresent
        seats: ({
            page: page,
            repoTab: repoTab,
            worktree: worktree,
            graphModel: graphModel,
            detailsModel: detailsModel,
            diffModel: diffModel,
            branchesModel: branchesModel,
            remotesModel: remotesModel,
            unstagedModel: unstagedModel,
            worktreesModel: worktreesModel,
            stashesModel: stashesModel,
            tagsModel: tagsModel,
            graphPane: graphPane,
            sidebarPane: sidebarPane,
            detailsPane: detailsPane,
            diffPane: diffPane,
            wipPane: wipPane,
            carriedSeat: carriedSeat,
            planSeat: planSeat,
            gitCorner: gitCorner,
            refMenuSeat: refMenuSeat,
            fileMenuSeat: fileMenuSeat,
            diffMenuSeat: diffMenuSeat,
            commitMenuSeat: commitMenuSeat,
            remoteMenuSeat: remoteMenuSeat,
            commitFileMenuSeat: commitFileMenuSeat,
            rowHost: rowHost,
            clipboard: clipboard,
            commitMenuState: commitMenuState,
            publishFlow: publishFlow,
            upstreamFlow: upstreamFlow
        })
    }

    // ---- a report with nothing to answer ----------------------------
    // A write that did not happen, and whoever said no (デザイン規約 §答えの要らない報せ). Headless: `remote-refused` /
    // `tag-refused` / `push-outdated` / `commit-refused` / `notice-over-diff` / `stale-part` /
    // `report-tone half-rename` / `rename-tag-box`, and on the presets where git's run refuses, `push` / `force-push` /
    // `drop-commit-go` / `rename-stash` / `worktree-add` — each says the bar as `write_notice … place= says=`.

    /// The one door, so a headless run handing it a report enters the same body the answer does. Both lines are ours
    /// (`Words.writeReported`, `Words.writeReportedWhy`), off core's record whole (`RepoTab.writeAnswerReport`:
    /// `kind`, `remote`, `name`, and `fact`, the one thing core hands the second line besides the names).
    function showReport(report) {
        const kind = report.kind
        const remote = report.remote
        const name = report.name
        const fact = report.fact
        const names = Words.reportNamesWorktree(kind)
        // The only reports that can name the repository's own worktree (it is never removed or made), which goes by
        // the branch it keeps (デザイン規約 §ref の種別「リポジトリ自身の worktree の呼び名」).
        const home = kind.startsWith("carried-") && page.appliedFrom !== undefined && page.appliedFrom.home
        const shown = home ? page.appliedFrom.called : name
        // Every name either line says wears its kind's mark (デザイン規約 §ref の種別「名前の印」): the worktree's tree
        // or house, the branch's, the tag's, the remote's cloud.
        const nameMark = names ? (home ? "home" : "tree")
                       : Words.reportNamesRef(kind) ? page.refMarkOf(kind, name) : ""
        page.showNotice(Words.writeReported(kind, remote, shown),
                        Words.writeReportedWhy(kind, remote, shown, fact),
                        Words.reportTone(kind),
                        Words.reportMarks(kind, remote, shown, nameMark),
                        Words.reportPlace(kind),
                        Words.reportWhyMarks(kind, remote, shown, fact, nameMark))
    }
    /// The mark a report's ref wears: the tag where the report is about one, or the repository holds a tag by that
    /// name, else the branch.
    function refMarkOf(kind, name) {
        return kind === "tag-elsewhere" || tagsModel.tagSides(name) !== "" ? "tag" : "branch"
    }
    /// Automation only: git's answer to a write has been taken all the way — bar raised, mark down, standing questions
    /// cleared. What it was about rides the signal: a drain can bring several reports, so the tab holds none to read
    /// back.
    signal writeReported(var report)
    /// `label` is what did not happen, `detail` the line under it, `tone` the state colour, if any; `marks` the names
    /// in `label` that wear their marks, left out for none (`NoticeBar.marks`); `place` the word of `detail` that opens
    /// the command log, left out for none (`NoticeBar.detailPlace`); `detailMarks` the names in `detail` that wear
    /// theirs.
    function showNotice(label, detail, tone, marks, place, detailMarks) {
        // Dressed, then raised — so nothing is written on a bar the reader can see (`NoticeBar.open`).
        noticeBar.label = label
        noticeBar.detail = detail
        noticeBar.tone = tone
        noticeBar.marks = marks === undefined ? [] : marks
        noticeBar.detailMarks = detailMarks === undefined ? [] : detailMarks
        noticeBar.detailPlace = place === undefined ? "" : place
        noticeBar.detailHref = noticeBar.detailPlace === "" ? "" : Words.commandsHref
        // Not over a pill being pressed through or its full fill (`GraphPane.askAnswering`; user decision — ask before
        // changing): the answer under way keeps the middle, and the report takes the question's place as it is over —
        // as the question goes (`askHandsOver`), or over it still standing, a count given up on (`raiseNotice`).
        page.noticeWaits = !noticeBar.open && graphPane.askAnswering
        if (!page.noticeWaits)
            page.raiseNotice()
    }
    /// A report dressed and waiting for the question's answer under way (`showNotice`): the question gives way to it
    /// (`AskBar.givesWay`).
    property bool noticeWaits: false
    /// Brings the report down over the middle, standing at `from` — the height of the bar it takes the place of, read
    /// off what stands there when left out: the band, or the question, standing or on its way up, which goes as the
    /// report comes (`askHandsOver`; `BarTravel`).
    function raiseNotice(from) {
        page.noticeWaits = false
        if (!noticeBar.open)
            noticeBar.enterFrom = from !== undefined ? from
                                : recoverBand.visible ? recoverBand.height
                                : !graphPane.askShut ? graphPane.askHeight : 0
        noticeBar.open = true
    }
    /// Only lowered: the words stay while the bar slides up.
    function hideNotice() {
        // What waits behind it takes its place at its height (`centreColumn`): the question first, else the band.
        if (graphPane.asking) {
            graphPane.keepRowsInPlace()
            graphPane.askEnterFrom = noticeBar.height
        } else if (page.recoverBandWanted) {
            recoverBand.enterFrom = noticeBar.height
        }
        noticeBar.open = false
    }
    /// Automation only, like `GraphPane.askCard`: a run reads `settled` / `shut` / `label` and presses `dismiss()`.
    readonly property alias noticeCard: noticeBar
    /// Automation only: whether the discard band stands, or steps aside for the report or the question
    /// (`centreColumn`).
    readonly property alias recoverBandStands: recoverBand.visible
    /// An entry of the discard log is on the graph, so its band stands over it once nothing comes before it
    /// (`centreColumn`).
    readonly property bool recoverBandWanted: page.recoverPicked !== null && !page.planShown && !page.diffShown
    /// Automation: whether the centre stands under the report, not behind it — a picture cannot tell. In page
    /// coordinates, so it holds for graph and diff alike.
    readonly property bool noticeClears:
        centreStack.mapToItem(page, Qt.point(0, 0)).y >= noticeBar.mapToItem(page, Qt.point(0, noticeBar.height)).y

    property int seenWriteSeq: 0
    /// Whether the open file was already re-read for this notify's answers: a drain can bring several that moved it,
    /// and one read covers them all.
    property bool diffReadForAnswers: false
    /// Re-reads the open file because a write answered, and tells the tab — which keeps the status published behind
    /// that write from reading the same file as news (`ops::DiffReread`). The one door, so the count lives here.
    /// **At the answer, not at the status**: the answer (`WriteFinished`) lands before the status is published, and git
    /// has moved the index by then, so the file's diff is already new. Only the file list waits for the status
    /// (`followEmptySide`). A refusal re-reads too (`absorbLeftoverAnswer`).
    function readDiffForAnswer() {
        if (page.diffReadForAnswers)
            return
        page.diffReadForAnswers = true
        repoTab.noteDiffRead()
        page.reloadDiff()
    }
    // What an answer *means* is settled on the tab, where the op names are known (`drain::settle_write`); this
    // function only sequences the screen off those classified properties — reads, landings, menus.
    function absorbWriteResult() {
        if (repoTab.writeSeq === page.seenWriteSeq)
            return
        page.seenWriteSeq = repoTab.writeSeq
        page.diffReadForAnswers = false
        // The press has its answer; the rest of the wait is the read (`diffSettling`).
        page.diffAwaits = false
        page.recoverAwaits = false
        publishFlow.noteWriteAnswer()
        page.absorbPushAnswer()
        page.absorbRefPushAnswer()
        page.absorbPopLabel()
        page.absorbCommitAnswer()
        page.absorbBranchDelete()
        page.absorbStashAnswer()
        page.absorbWorktreeAnswer()
        page.absorbWorktreeMove()
        // The status and this answer drain apart: the tree a stash or a commit of ours emptied may already be read,
        // with nothing left to ask. Whose doing an empty tree is, the press says (`takeStashLanding` /
        // `takeCommitLanding`, asked inside); a poll with no edge of its own moves nobody.
        page.leaveWipWhenDone(false)
        page.absorbLeftoverAnswer()
    }
    /// This notify's answers nobody waited for by name (`RepoTab::fold_into_group`): the timer's fetch, a write from a
    /// page since gone. Its early returns end only this; the named presses are answered before it, where a refusal in
    /// this group cannot reach them.
    function absorbLeftoverAnswer() {
        if (repoTab.writeRefused) {
            // Nothing moved, so no landing will put `moveLanding` down; this is its one put-down without one.
            page.moveLanding = ""
            // Rows taken away for this write are already back: the delete's owner restored them on the answer itself
            // (`ops_delete::delete_answered`).
            //
            // A stale-diff refusal: the tally watch misses a drift that moves no bucket count, and left alone the same
            // press is refused forever.
            if (repoTab.writeStaleDiff)
                page.readDiffForAnswer()
            const report = repoTab.writeReport()
            // Unless git turned the rename down, the box has nothing to answer and closes as on a landing (a
            // half-finished rename too: its row is what could not be found).
            if (report === undefined || !Words.renameStillOpen(report.kind))
                page.noteRenameLanded()
            // Refused from outside: a report, as `reportRefusal`.
            if (report !== undefined) {
                // A refused rename leaves its box open holding the name, to try again; the bar says why
                // (デザイン規約 §答えの要らない報せ).
                if (Words.renameStillOpen(report.kind))
                    page.noteRenameRefused()
                page.showReport(report)
                if (!commandsModel.failed)
                    page.answeredFailures++
                commandsModel.noteAnswered()
                page.forgetRenameCarry()
                page.writeReported(report)
                return
            }
            // Otherwise the log comes up, as `tellRefusal`. A fetch here is not other news: the tab already raised the
            // panel for it, and taking the panel over would pin a failure that later heals (`CommandsOwner`).
            if (!repoTab.writeFetched)
                commandsOwner.newsTakes()
            page.commandsOpen = true
            // A rename that did not happen has nothing to carry over.
            page.forgetRenameCarry()
            return
        }
        // The name went in, so its box comes down — only once the queue has drained: a fetch queued ahead answers
        // first, and closing on it would leave the rename's refusal nowhere to go.
        if (repoTab.busyCount === 0)
            page.noteRenameLanded()
        // The remote is still under the old name; asked only now the rename landed (デザイン規約 §手元の改名の後のリモート).
        if (repoTab.writeBranchOp && page.pendingRenameRemote !== "") {
            const spokenFor = page.pendingRenameRemote
            const took = page.pendingRenameTo
            page.pendingRenameRemote = ""
            page.pendingRenameTo = ""
            page.carryBranchRenameOver(spokenFor, took)
        }
        // The tag's half, on the same terms.
        if (repoTab.writeTagOp && page.pendingRenameTagRemote !== "") {
            const on = page.pendingRenameTagRemote
            const was = page.pendingRenameTagFrom
            const now = page.pendingRenameTagTo
            const at = page.pendingRenameTagOid
            page.forgetRenameTagRemote()
            renameCarryFlow.startTagAsk(on, was, now, at)
        }
        // Where the answer sends the reader, read per answer this notify carried (`RepoTab::write_answers`): a write
        // starting in the same drain (the fetch after a run) would already have lowered a single stop flag. The two
        // landings are exclusive, and the later answer wins.
        for (let i = 0; i < repoTab.writeAnswerCount(); i++) {
            // git stopped part-way: the answer is the worktree — the conflicted rows and the way out
            // (デザイン規約 §進行中の操作から出る). Armed, since the status carrying those rows has not arrived yet.
            if (repoTab.writeAnswerStopped(i)) {
                page.pendingWipSelect = true
                page.pendingHeadSelect = false
                page.pendingHeadAsked = false
            }
            // A commit at the tip (merge / cherry-pick / revert, 規約 §履歴を合流させる): the selection goes there and
            // the view follows — the answer to the asked work is there. The view moves only where that row is off
            // screen (moving the selection alone would take the focus out of sight), and only for a write this window
            // asked for: a commit from a terminal, or the selected commit rewritten under a poll, is nobody's request
            // here. A merge already taken in (`Already up to date`) lands on the tip too: "the tip is here" says that
            // nothing happened. An empty cherry-pick is checked on `demo-repo diverged`'s tree with the same change
            // committed on both branches.
            else if (repoTab.writeAnswerAtTip(i)) {
                page.pendingWipSelect = false
                page.pendingHeadSelect = true
                // The first report that may answer it: the report in hand may be from before the write.
                page.pendingHeadFromSeq = repoTab.writeAnswerHeadSeq(i)
                page.pendingHeadAsked = true
            }
        }
        // The report that answers the landing may already be in hand (above).
        page.tryPendingHeadSelect()
        // An open diff is stale (`readDiffForAnswer`).
        if (repoTab.writeStaleDiff)
            page.readDiffForAnswer()
        // The editor stops offering to save, and keeps the words until the selection reaches the rewritten commit.
        if (repoTab.writeReworded)
            detailsPane.noteMessageSaved()
        // Moving HEAD rewrites the tree and index under the diff, so the centre goes back to the graph.
        if (repoTab.writeMovedHead)
            page.closeDiff()
    }

    // The centre shows the graph or a file diff. Kind and path are kept apart: a path may contain colons.
    property bool diffShown: false
    onDiffShownChanged: page.foldForDiff(page.diffShown)
    property string diffKey: ""
    property string diffKind: ""
    property string diffPath: ""
    property string diffOrigPath: ""
    // Whether the shown diff is a worktree file (stageable).
    property bool diffFromWorktree: false
    readonly property bool diffStaged: page.diffKind === "staged"
    /// The two stage letters git reports for the shown file, read once at open: on a conflict git prints no patch, so
    /// they are all there is to say. They hold until the file leaves the conflicts, which reopens it
    /// (`followEmptySide`).
    property string diffChange: ""
    /// Each conflict side's colour: the graph's lane colour for that branch, where it has one. `finishCount` is read
    /// only to depend on it — every graph property shares one notify, and the graph arrives in two passes (chips after
    /// rows) and rebuilds when refs move.
    readonly property var sideColors: {
        graphModel.finishCount
        return graphModel.sideColors(worktree.sideOurs, worktree.sideTheirs)
    }
    readonly property int sideColorOurs: page.sideColors.ours
    readonly property int sideColorTheirs: page.sideColors.theirs
    function toggleDiff(kind, path, origPath) {
        if (page.diffShown && page.diffKey === GitFacts.fileKey(kind, path)) {
            page.closeDiff()
            return
        }
        page.openDiff(kind, path, origPath)
    }
    /// Reads one file, whoever asked — a row that was clicked, or the pane moving itself off a side that ran out
    /// (`followEmptySide`).
    function openDiff(kind, path, origPath) {
        // Held off while a plan has the centre (`centreStack`): a file opened now would still be up when the plan is
        // put away (after `Discard`, the old diff would sit in the centre). The only place `diffShown` rises, so this
        // one line holds the fold, neighbour and read; the file rows stay live (規約 §フル interactive rebase「右の詳細
        // ペインがそのまま生きる」 — only this and the parent hash in `jumpToRef` freeze, the two that reach outside).
        if (page.planShown)
            return
        // A held scroll is the last file's: another file starts at the top and the left edge (`DiffCodeScroll.file`).
        diffPane.dropScroll()
        page.standDiff(kind, path, origPath)
        if (kind === "commit") {
            page.askCommitDiff(path, origPath)
        } else if (page.carriedPath !== "") {
            // Read-only, aimed at the worktree the rows came from (`RepoSession::load_carried_diff`).
            diffModel.requestCarried(page.carriedPath, kind, path, origPath)
        } else {
            page.noteDiffAlone()
            diffModel.requestWorktree(kind, path, origPath)
        }
        page.diffShown = true
        page.diffNeighbour = ""
        page.noteDiffNeighbour()
    }
    /// Names the file the pane stands on, and lights its row; what the pane draws of it is the caller's — a read
    /// (`openDiff`), or the rows a press moved there or a wait (`landAhead`). Any file stood on anew is no
    /// landing ahead of git (`diffLeftFrom`).
    function standDiff(kind, path, origPath) {
        page.diffLeftFrom = ""
        page.diffKey = GitFacts.fileKey(kind, path)
        page.diffKind = kind
        page.diffPath = path
        page.diffOrigPath = origPath
        page.diffFromWorktree = kind !== "commit"
        page.diffChange = kind === "conflicts" ? page.wipUnstaged.changeOf(path) : ""
        // The list's light follows the pane when it moves itself. Only this window's own tree: a carried worktree's
        // path picked here would aim this tree's presses at it (`WipPane.readOne`).
        if (kind !== "commit" && page.wipWritable)
            wipPane.readOne(kind, path)
    }
    /// The patch behind a commit-list row, in the list's three shapes (デザイン規約 §複数のコミットを選ぶ): one commit's
    /// change, the difference between exactly two, or each chosen commit's own block in turn — a diff of the two ends
    /// would carry the unchosen commits between them.
    function askCommitDiff(path, origPath) {
        if (detailsModel.selectionCount > 2) {
            diffModel.requestChoiceFile(page.chosenIds, path, origPath)
            return
        }
        if (detailsModel.comparing) {
            diffModel.requestRangeFile(detailsModel.compareFrom, detailsModel.compareTo, path, origPath)
            return
        }
        diffModel.requestCommitFile(detailsModel.shaHex, detailsModel.parentHex, path, origPath)
    }
    // Stages (or unstages) one hunk, or one line of it, by indices into the diff on screen.
    function stageSelection(hunk, line) {
        // The seats these come from are not offered on another worktree's file (`DiffPane.partial`); this is the belt.
        if (!page.wipWritable)
            return
        // The fingerprint rides along: the write refuses to apply indices to drifted bytes. `diffAwaits` is set from
        // the slot's answer (`diffSettling`). The part leaves the rows at the press (`restage`), and the file row
        // with it where it took every change (`takesAll`).
        page.diffAwaits = repoTab.stageSelection(page.diffKind, page.diffPath, page.diffOrigPath, hunk, line,
                                                 diffModel.fingerprint, diffModel.takesAll(hunk, line))
    }
    /// Discards one hunk, unasked: the hold on its own heading's button is the consent (`DiffRowDelegate`'s
    /// `Discard hunk`).
    function discardHunkNow(hunk) {
        if (!page.wipWritable)
            return
        // Armed by the answer (`diffSettling`); the hunk leaves at the press, as a staged one does.
        page.diffAwaits = repoTab.discardSelection(page.diffKind, page.diffPath, page.diffOrigPath, hunk, -1,
                                                   diffModel.fingerprint, diffModel.takesAll(hunk, -1))
    }

    // ---- what the reader lands on when a side runs out ---------------
    /// The open file's neighbour under the same heading, as `<bucket>:<path>` (`NavSectionModel.besidePath`, which
    /// passes over the files a stage press still reaches). Noted while the file is still there (on every `changed` of
    /// the list and every move of the presses, `restage`, only while `stillHere` says so): once the file has gone, the
    /// place it left is gone from the list. **Asked of the model** (`besidePath` / `holdsPath` / `bucketOf`), never by
    /// walking the view: a `ListView` holds no off-screen rows, and the neighbour would come back null.
    property string diffNeighbour: ""
    function noteDiffNeighbour() {
        if (!page.diffShown || page.diffKind === "commit" || !page.stillHere(page.diffKind, page.diffPath))
            return
        page.diffNeighbour = page.wipUnstaged.besidePath(page.diffKind, page.diffPath)
    }
    /// Whether the open file's change was on its own side alone — no other bucket listed it — as the list stood when
    /// the read on screen was asked for: that read then holds the whole of the file's change, and where presses move
    /// the file to another side, that side's diff is this read line for line (`landAhead`). Noted per read, not per
    /// change of the list: a part pressed on the read puts the file on the other side of the list too, ahead of git,
    /// while the read stays the whole change — and a read asked after git did the part holds what is left of it.
    property bool diffAlone: false
    function noteDiffAlone() {
        page.diffAlone = !page.diffSides.some(side => side !== page.diffKind
                                              && page.wipUnstaged.holdsPath(side, page.diffPath))
    }
    /// The buckets a worktree file's row can stand in, in the order a landing looks through them (`sideBesides`).
    readonly property var diffSides: ["staged", "unstaged", "untracked", "conflicts"]
    /// The side other than `from` that holds `path` for the pane's purposes (`stillHere`), or `""`.
    function sideBesides(from, path) {
        for (const side of page.diffSides) {
            if (side !== from && page.stillHere(side, path))
                return side
        }
        return ""
    }
    /// Whether `bucket` still holds `path` for the pane's purposes: on the list, and reached by no stage press standing
    /// — a row the press could not move ahead of git (`ops::stage_ahead`'s `moved`: a file on both sides, a staged
    /// rename, a conflict, the rows `Stage all` leaves to the status) leaves its side with the status after it, so
    /// the pane walks on as one would have at that status. A gitlink may stay where it is (its own changes stay
    /// unstaged), which `StageAhead::reaches` handles: the pane waits on it for the status. Read off `repoTab`, not a
    /// binding: `restage` runs in `onStagingSeqChanged`, ahead of the bindings the same notify moves
    /// (rules-refs/app-ui.md — the `onXChanged` line).
    function stillHere(bucket, path) {
        return page.wipUnstaged.holdsPath(bucket, path) && !(page.wipWritable && repoTab.pressReaches(bucket, path))
    }
    /// The open file's side has run out (staged, unstaged, discarded, committed), so the pane moves
    /// (デザイン規約 §diff の中のステージ; user decision — ask before changing): the side's next file under the same
    /// heading (the one before where it was the last), else the same file on the side it went to (all of it is there,
    /// so that is what to read), else closed. Kept standing it would be an empty frame with a live `Stage file`, its
    /// reason to stand gone. Both ways (staged → unstaged too). "Next" is the right panel's order, untracked inside
    /// the unstaged heading (`NavItem.group`), so an untracked file can be the neighbour.
    ///
    /// Driven by the file list's `changed`, once the file is no longer there (the guard below; there is no
    /// close-when-empty path), not by the re-read coming back empty: that read runs beside the status (`load_diff` /
    /// `publish_status`) and can land before the list has moved — asking the list from before the write for the next
    /// file, it opens the emptied side again and stops — and some sides never read empty (an untracked file staged
    /// whole, a picture). A file empty from the start stays in the list, so opening it closes nothing.
    /// Headless: `diff-follow`.
    ///
    /// **A press takes the file off its side at the press** (デザイン規約 §バケツごとの一覧と、ペインの分け方), ahead
    /// of git — moved there, or reached and left to the status (`stillHere`) — and the pane lands on that turn where
    /// the status after the press will have it: the neighbour, which no press reaches, else the file's own new side
    /// (`landAhead`), else closed. A press git refuses puts the rows back: on the neighbour the pane stays, from its
    /// own new side it goes back with them (`followLanding`). Headless: `diff-walk`, `stage-ahead`.
    function followEmptySide() {
        if (!page.diffShown || page.diffKind === "commit")
            return
        if (page.diffLeftFrom !== "" && page.followLanding())
            return
        if (page.stillHere(page.diffKind, page.diffPath))
            return
        const beside = GitFacts.fileOfKey(page.diffNeighbour)
        if (beside !== undefined && beside.bucket !== "") {
            if (page.stillHere(beside.bucket, beside.path)) {
                page.openDiff(beside.bucket, beside.path, page.wipUnstaged.origOf(beside.path))
                return
            }
        }
        if (page.wipWritable && repoTab.pressReaches(page.diffKind, page.diffPath)) {
            page.landAhead()
            return
        }
        const moved = page.wipUnstaged.bucketOf(page.diffPath)
        if (moved !== "") {
            page.openDiff(moved, page.diffPath, page.wipUnstaged.origOf(page.diffPath))
            return
        }
        page.closeDiff()
    }
    /// The side the open file was taken off at a press with no neighbour to walk to, while the pane stands where that
    /// press lands it ahead of git (`landAhead`); `""` otherwise.
    property string diffLeftFrom: ""
    /// The press took the open file off its side and no neighbour is left: the pane lands on the press's turn where
    /// the status after it will have the file (user decision — ask before changing), never on the side it left, which
    /// now draws empty. On the side the file went to, which the list already shows it on: a change that was on one
    /// side alone (`diffAlone`) is that side's diff line for line — the stage before is the unstage after — so its
    /// rows go there as they are (`DiffModel.relabelTo`); a file on both sides holds there what git's comparison of
    /// contents says, so the pane waits blank (`DiffModel.waitFor`). Nowhere, the file gone from the list (a discard),
    /// it closes. A file the list leaves to the status, with no side shown yet (a conflict marked resolved, a staged
    /// rename unstaged), waits blank in place for that status. Either wait reads nothing meanwhile (`diffHeld`).
    function landAhead() {
        const from = page.diffKind
        const path = page.diffPath
        const to = page.sideBesides(from, path)
        if (to === "" && !page.wipUnstaged.holdsPath(from, path)) {
            page.closeDiff()
            return
        }
        const alone = page.diffAlone
        if (to === "") {
            diffModel.waitFor(from, path, page.diffOrigPath)
            page.diffLeftFrom = from
            return
        }
        const orig = page.wipUnstaged.origOf(path)
        // The same rows stay where the reader had them; a wait starts a file of its own at the top.
        if (!alone)
            diffPane.dropScroll()
        page.standDiff(to, path, orig)
        page.diffLeftFrom = from
        if (alone)
            diffModel.relabelTo(to, path, orig)
        else
            diffModel.waitFor(to, path, orig)
        page.diffNeighbour = ""
        page.noteDiffNeighbour()
    }
    /// The presses moved while the pane stands where one landed it (`landAhead`). The rows back on the side it left —
    /// git refused, or the file changed there since — take it back there. The status after the presses, drawn, hands
    /// the pane to git: in place, it then lands where that status put the file (`followEmptySide`). Answers whether
    /// the pane stays where it is.
    function followLanding() {
        const from = page.diffLeftFrom
        const path = page.diffPath
        if (page.stillHere(from, path)) {
            page.openDiff(from, path, page.wipUnstaged.origOf(path))
            return true
        }
        if (repoTab.pressReaches(from, path))
            return page.diffKind === from
        page.diffLeftFrom = ""
        return false
    }
    /// Whether the pane, landed ahead of git (`landAhead`), holds off reading: on the file's new side until git has
    /// answered the presses that took it there — read before, the side shows itself as it was before the write — and
    /// in place until the status after them is drawn, which says where the file went. A read once the rows are back
    /// on the side it left is that side's, where the pane goes too (`followLanding`).
    function diffHeld() {
        const from = page.diffLeftFrom
        if (from === "" || page.stillHere(from, page.diffPath))
            return false
        if (page.diffKind === from)
            return repoTab.pressReaches(from, page.diffPath)
        return repoTab.pressOut(from, page.diffPath)
    }
    /// Whether the diff on screen is still catching up with a write — a press out, the queue busy, a read out, rows
    /// relabelled from the other side, or the pane waiting for where its file went: the paced re-read holds off
    /// meanwhile (`pollDiff`), and automation reads the pane as unsettled. **Every part ends by itself** — holding on
    /// "the tree has not been read yet" instead wedges for good on a write that moves no row (the second line of a
    /// file on both sides). `busyCount` alone is not enough: it rises when the queue starts the write, not at the
    /// press. `diffAwaits` rises on the press's own answer that it was queued (`stageSelection`; set before it, a press
    /// turned away at the door would wedge the same way) and falls on the write's answer (`absorbWriteResult`). The
    /// marks wait on less (`diffPressable`).
    property bool diffAwaits: false
    readonly property bool diffSettling:
        page.diffAwaits || repoTab.busyCount > 0 || diffModel.loading || diffModel.relabelled || page.diffMoving
    /// Whether the pane's marks take a press (`DiffPane.busy`) — core refuses (`Rejected`) a press made on rows that
    /// have since moved. **A part pressed here is already in the drawing** (`RepoTab.diffWritesOut` leaves it out),
    /// and a press on the rows it left is placed in that drawing (`StageAhead::pressed_on`), so the next hunk or line
    /// takes a press while the one before is still queued or running — and joins its burst, one read behind them all
    /// (デザイン規約 §diff の中のステージ). What the drawing does not hold still holds them: any other write out (it
    /// moves rows the drawing does not know of), a part of a shape not drawn ahead (`DiffModel.laidExact` — core
    /// places nothing behind it), and rows relabelled as the file's new side at a press (`landAhead`), whose places are
    /// in the other side's diff.
    readonly property bool diffPressable:
        !repoTab.diffWritesOut && !diffModel.loading && diffModel.laidExact && !diffModel.relabelled
        && !page.diffMoving
    /// The open file has left the side it was read on at a stage press (`stillHere`) and the pane has not moved off it:
    /// it waits there blank for the status after that press, which says where the file went (`landAhead`). Read again
    /// as the presses move (`stagingSeq`): the models' answers raise no notify.
    readonly property bool diffMoving:
        page.stagingSeq >= 0 && page.diffShown && page.diffKind !== "commit" && page.wipWritable
        && !page.stillHere(page.diffKind, page.diffPath)
    /// The tree moved, so the open file is stale — whoever moved it. With only the diff's own writes asking for a
    /// read, a line staged and then taken back with the file row's `−` would stay gone from both sides; moved by
    /// another window or a terminal, the old rows' `+` would be refused by git (the fingerprint). The callers already
    /// know the tree moved.
    function reloadDiff() {
        // Nothing this window writes moves another worktree's file; that worktree's own paced reads keep it current
        // (`pollCarried`).
        if (!page.diffShown || page.diffKind === "commit" || !page.wipWritable || page.diffHeld())
            return
        page.noteDiffAlone()
        diffModel.requestWorktree(page.diffKind, page.diffPath, page.diffOrigPath)
    }
    /// Asks, after each paced read of this tree, whether the file on screen still reads the same: the counts and the
    /// file list miss an edit that moves neither (a conflict resolved elsewhere keeps its stage letters until added).
    /// Returns whether a read went out — the only edge automation can latch on (`diff-tick`), since a quiet file gets
    /// no answer.
    function pollDiff() {
        if (!page.diffShown || page.diffKind === "commit" || page.diffSettling)
            return false
        // A worktree's file is re-read with that worktree's reads, aimed at the worktree — through here it would read
        // *this* window's file of that name into a pane showing somebody else's (`pollCarried`).
        if (!page.wipWritable)
            return false
        return diffModel.refreshWorktree(page.diffKind, page.diffPath, page.diffOrigPath)
    }
    /// The open file of the worktree being read: after each paced read of that worktree, which handed the pane its
    /// list as it read the row (`RepoSession::read_carried_status`), and when the window comes back. Returns whether it
    /// asked, as `pollDiff`.
    function pollCarried() {
        if (page.carriedPath === "")
            return false
        if (page.diffShown && page.diffKind !== "commit" && !diffModel.loading)
            diffModel.refreshCarried(page.carriedPath, page.diffKind, page.diffPath, page.diffOrigPath)
        return true
    }

    /// Escape or the header's mark: puts the diff away and hands the arrows to the graph (user decision — ask before
    /// changing): the two ways out aimed at the diff itself leave the keyboard nowhere — what was pressed goes, or
    /// there was no press at all. The product comes here from `escapePressed`, `DiffPane.onCloseRequested` and
    /// `showChosen` only. Not the file row's second click, a press that lands in a list still on screen with its
    /// row lit, which keeps the keyboard (`FileRowWalk.qml`); nor the closes the reader did not make — `closeDiff`
    /// itself hands nothing on, as the graph need not be what is left (handed the focus under the plan, a pane not
    /// on screen holds the arrows).
    /// Headless: `diff-escape`'s `graph=true` — one active focus, so it is also the list letting go.
    function closeDiffToGraph() {
        page.closeDiff()
        graphPane.takeKeyboard()
    }

    function closeDiff() {
        page.diffShown = false
        page.diffLeftFrom = ""
        page.diffKey = ""
        page.diffKind = ""
        page.diffPath = ""
        page.diffOrigPath = ""
        page.diffFromWorktree = false
        page.diffChange = ""
        diffModel.clear()
    }

    // Exposed for the window toolbar (acts on the active tab).
    readonly property var pageTab: repoTab
    readonly property var pageWorktree: worktree
    /// For the band's one reading: whether this branch's upstream is gone (`headUpstreamGone`, git's `[gone]`).
    readonly property var pageBranches: branchesModel
    /// For `TopBar`'s WORKTREE rows — the left menu's WORKTREES listing, so both name the same places in the same
    /// order.
    readonly property var pageWorktrees: worktreesModel
    /// The name the band's Stash button gives the entry — the commit box is on this page, the button is not
    /// (デザイン規約 §変更を退避する); the file row's `stash` reads the same one.
    readonly property string pageStashName: wipPane.stashName
    readonly property var pageCommands: commandsModel
    /// Automation only: `tab-carry` writes into one tab's boxes and reads another's — a two-page question, so the
    /// window's (`WindowAutoActDriver`).
    readonly property alias pageWip: wipPane
    /// For the settings card's avatar entry, which offers the authors of the repository being looked at.
    readonly property var pageGraph: graphModel
    /// Automation only, like `pageWip`: `carried-open` asks this page for a row and reads the landing in the window.
    readonly property alias pageGraphPane: graphPane
    /// Automation only, the same way (`worktree-stand`: a WORKTREES row stands the tab in that worktree, and the
    /// landing is the window's).
    readonly property alias pageSidebar: sidebarPane
    /// Automation only: the pane whose read `PageSettled` waits for (`diff`); the lists are found by walking the page.
    readonly property alias pageDiff: diffPane
    /// Whether the refs listing has landed — the read that also settles how many remotes there are, and so what the
    /// band's fetch button may be (`fetch-tip`).
    readonly property bool pageRefsLoaded: branchesModel.refsLoaded
    /// Whether the right pane has the selected commit's own read — the page's last (the graph pass and refs land first
    /// and pick the row, `trySelectDefault`). A page on the worktree, or with no commit to select, waits for none.
    readonly property bool pageDetailsSettled: page.wipShown || page.selectedOid === ""
                                               || !detailsModel.loading
    /// Whether the right pane holds a file list of the other worktree it stands on — the one `standOnWorktree` asks
    /// for, or one read when it stood there before. A pane on this window's own tree waits for none: its list is the
    /// worktree's own read.
    readonly property bool pageCarriedSettled: page.carriedPath === "" || carriedModel.carriedAt === page.carriedPath
    /// Whether the page still owes a landing it has decided on (`pendingHeadSelect` / `pendingWipSelect`). Read by
    /// `PageSettled`: until it lands, every other reading is of the repository as the write found it.
    readonly property bool pageLanding: page.pendingHeadSelect || page.pendingWipSelect
    /// Automation only: the window's band, handed in by `Main`, so this page's verbs press the real Stash button
    /// (デザイン規約 §変更を退避する) instead of calling what it calls. `var` because `TopBar` is above this file, not beside it.
    property var pageBand: null

    /// Whether the command log is up (デザイン規約 §git が言ったことを読む場所).
    property bool commandsOpen: false
    // The mark is the panel's (`commandsAttention`) and goes down with it, whichever way the panel went — here, not in
    // `notePress` alone: Escape, the panel's fourth way out, comes with no press.
    onCommandsOpenChanged: if (!page.commandsOpen) page.commandsAttention = false
    readonly property CommandsOwner commandsOwner: CommandsOwner {
        id: commandsOwner
    }
    function toggleCommands() {
        commandsOwner.readerTakes()
        page.commandsOpen = !page.commandsOpen
    }
    /// The panel was sent for from elsewhere and says so when the reader arrives
    /// (デザイン規約 §hover のツールチップ「送った先は名乗る」). The reader's own hand, so `readerTakes()` — a landing fetch
    /// may not take it away. Raised even if the panel was already up: the reader was still sent.
    property bool commandsAttention: false
    function raiseCommands() {
        commandsOwner.readerTakes()
        page.commandsOpen = true
        page.commandsAttention = true
    }
    /// The one link tooltips, the notice bar's line and the diff pane's unread line carry (`Words.commandsHref`); other
    /// hrefs are ignored — a tip is not a browser.
    function tipLinkAsked(href) {
        if (href === Words.commandsHref)
            page.raiseCommands()
    }
    /// The reader putting the panel away — its `Clear` and closing mark, and Escape.
    function shutCommands() {
        commandsOwner.readerTakes()
        page.commandsOpen = false
    }

    // ---- the record of what was thrown away (`platitude_core::discards`) ----
    DiscardModel {
        id: discardModel
    }
    /// What a press on a ref weighs, read as the ref's menu reads it (`switchToRef`) — the menus carry their own.
    readonly property RefMenuFacts refFacts: RefMenuFacts {
        repoTab: repoTab
        graphModel: graphModel
        worktreesModel: worktreesModel
        branchesModel: branchesModel
        remotesModel: remotesModel
        tagsModel: tagsModel
    }
    /// The discard log's words (`RecoverEntries`) — its times included, which the band over the graph writes too.
    readonly property RecoverEntries recoverWords: RecoverEntries {
        id: recoverWords
        rows: discardModel.rows
        calledOf: path => worktreesModel.worktreeName(path)
        homeBranch: worktreesModel.homeBranch
    }
    /// What the discard log draws, worded.
    readonly property var recoverEntries: recoverWords.entries
    /// The moment the log's times are read against. Re-read at half the finest step a time there moves by
    /// (`RecoverEntries.stepMs`), so a count shown is never more than half a step behind.
    property real recoverNow: Math.floor(Date.now() / 1000)
    Timer {
        interval: recoverWords.stepMs / 2
        running: page.recoverOpen
        repeat: true
        onTriggered: page.recoverNow = Math.floor(Date.now() / 1000)
    }
    /// Whether the entries are an answer — an empty list before the first read is not "nothing was thrown away".
    readonly property bool recoverAnswered: discardModel.state === "ready"
    /// The reflogs are being read; the list says so while it waits (`RecoverPane`).
    readonly property bool recoverReading: discardModel.state === "reading"
    /// The read failed: the list says so in git's words, with no rows left from an earlier read to pick.
    readonly property bool recoverFailed: discardModel.state === "error"
    readonly property string recoverError: discardModel.error
    /// The reflogs and the record, read each time the log is opened (`discards::read_repo`): to the graph's window, or
    /// whole when the window holds the whole history.
    function readDiscards() {
        discardModel.look(repoTab.repoPath, graphModel.truncated ? graphModel.rowTotal : 0)
    }
    /// The entry picked, as the list words it; null for none.
    readonly property var recoverPicked:
        page.recoverPick >= 0 && page.recoverPick < page.recoverEntries.length
            ? page.recoverEntries[page.recoverPick] : null
    /// Whether the discard log is up over the left menu.
    property bool recoverOpen: false
    /// The entry picked in it, -1 for none. One at a time.
    property int recoverPick: -1
    /// The seat lit: something was just thrown away, and this says where it went. Every write that can leave something
    /// only the log reaches (`RepoTab.takenSeq` — a restore is not one) blinks it twice and leaves it out
    /// (`blinkRecover`) wherever the list is not on screen; opening the log puts it out.
    property bool recoverLit: false
    readonly property bool recoverBlinking: recoverBlink.running
    readonly property int recoverTaken: repoTab.takenSeq
    onRecoverTakenChanged: {
        if (!page.recoverOpen || page.listFolded)
            page.blinkRecover()
    }
    /// What the log lists changed — a take, a restore, a name taken here (`RepoTab.discardSeq`): an open log reads
    /// again. The entry picked stays picked through it, found again by what it is (`DiscardModel.keyAt`): the rows
    /// move as entries come and go, and one the read lists no more — brought back — is put down.
    readonly property int recoverDiscards: repoTab.discardSeq
    onRecoverDiscardsChanged: {
        if (!page.recoverOpen)
            return
        if (page.recoverPick >= 0 && page.recoverHeld === "")
            page.recoverHeld = discardModel.keyAt(page.recoverPick)
        page.readDiscards()
    }
    /// The entry picked, held across a read again (`DiscardModel.keyAt`); "" for none.
    property string recoverHeld: ""
    onRecoverAnsweredChanged: {
        if (!page.recoverAnswered)
            return
        const held = page.recoverHeld
        page.recoverHeld = ""
        // Put down meanwhile — the list closed, the search opened — it stays down.
        if (held === "" || page.recoverPick < 0)
            return
        const at = discardModel.indexOfKey(held)
        if (at < 0) {
            page.recoverPick = -1
            page.showRecoverPick()
            return
        }
        page.recoverPick = at
        page.readRecoverReach()
    }
    // A failed read lists nothing: the entry it held goes down with its rows.
    onRecoverFailedChanged: {
        if (!page.recoverFailed)
            return
        page.recoverHeld = ""
        if (page.recoverPick >= 0) {
            page.recoverPick = -1
            page.showRecoverPick()
        }
    }
    function blinkRecover() {
        recoverBlink.restart()
    }
    // Twice on and off, a `blinkMs` each, as a flash is, then out (user decision, provisional: the count is to be
    // settled by watching it in use — P3-確認事項 §破棄記録と復元).
    SequentialAnimation {
        id: recoverBlink
        PropertyAction { target: page; property: "recoverLit"; value: true }
        PauseAnimation { duration: Metrics.blinkMs }
        PropertyAction { target: page; property: "recoverLit"; value: false }
        PauseAnimation { duration: Metrics.blinkMs }
        PropertyAction { target: page; property: "recoverLit"; value: true }
        PauseAnimation { duration: Metrics.blinkMs }
        PropertyAction { target: page; property: "recoverLit"; value: false }
    }
    function toggleRecover() {
        // Folded (a diff took the screen, or the window has no room for the list), an open list is not on it: the rail's
        // press brings it back, as a section's way back does, rather than closing what cannot be seen.
        if (page.recoverOpen && page.listFolded) {
            page.foldByHand(false)
            return
        }
        page.recoverOpen = !page.recoverOpen
        if (page.recoverOpen) {
            recoverBlink.stop()
            page.recoverLit = false
            page.recoverNow = Math.floor(Date.now() / 1000)
            page.readDiscards()
            // The list takes the menu's place, which the rail has no room for.
            if (page.listFolded)
                page.foldByHand(false)
        } else if (page.recoverPick >= 0) {
            // The graph goes back to what the repository holds with the list.
            page.recoverPick = -1
            page.showRecoverPick()
        }
    }
    /// A press on an entry picks it, or puts a picked one down. Picking ends a search as its `✕` does, and takes down a
    /// name box a half-typed name kept standing: the graph turns to what the entry would bring back, and renames
    /// nothing under it. Not while the log reads again: the rows up are the last read's, and the answer would deal the
    /// number picked to another entry. Putting down deals no number (`onRecoverAnsweredChanged` leaves it down), so the
    /// band's `✕` and Escape go through under a read.
    function pickRecover(index) {
        if (page.recoverReading && page.recoverPick !== index)
            return
        page.recoverPick = page.recoverPick === index ? -1 : index
        if (page.recoverPick >= 0) {
            // A card never raised has no search to end (`GraphPane.findCard`).
            if (graphPane.findCard !== null)
                graphPane.findCard.dismiss()
            graphPane.stopNaming()
        }
        page.showRecoverPick()
    }
    /// The picked entry on the graph, or none: what each part would bring back walks in dashed — a copy of thrown-away
    /// work as the uncommitted row it was, a dropped stash as the stash it was (`GraphModel.provisionalWipAt` /
    /// `provisionalStashAt`) — every other row dims as a search's misses do, and the view keeps to its span
    /// (`GraphModel.showDiscard`).
    function showRecoverPick() {
        const tips = page.recoverPick < 0 ? [] : discardModel.tipsAt(page.recoverPick)
        page.recoverTipPending = tips.length > 0 ? tips[0] : ""
        page.readRecoverReach()
        if (tips.length === 0) {
            graphModel.hideDiscard()
            return
        }
        graphModel.showDiscard(tips, discardModel.looksAt(page.recoverPick), discardModel.lostAt(page.recoverPick),
                               discardModel.standsAt(page.recoverPick))
        page.landRecoverTip()
    }
    /// The picked entry's first tip, until a pass has drawn its row — then it is selected and shown, as a jump to a ref
    /// is (`jumpToRef`), so the right pane reads what the entry would bring back.
    property string recoverTipPending: ""
    function landRecoverTip() {
        const row = graphModel.provisionalTipRow
        if (page.recoverTipPending === "" || row < 0 || graphModel.oidAt(row) !== page.recoverTipPending)
            return
        page.recoverTipPending = ""
        // A diff opened while the walk was out is the reader's own doing; landing would close it.
        if (page.diffShown)
            return
        page.jumpToRef(graphModel.oidAt(row))
    }
    /// Per part of the picked entry, whether what it would bring back is on the graph: its tip a row the provisional
    /// walk (the window's own `--max-count`) has drawn, or work put back into a worktree — and the stash entry it may
    /// land as, dated at the restore — which no window holds back. Nothing whose first commit the graph's window
    /// does not draw is restored: the band offers `Load N more commits` until it does. Empty until the walk
    /// that took the entry has landed (`GraphModel.provisionalWalked`) — before it, an undrawn tip is not yet walked,
    /// not out of reach — and while a stream brings the rows back from nothing (`GraphModel.loading`, a Reload's): a
    /// row not back yet is no further away than it was. Read again on each `statsChanged` as the graph's rows move — a
    /// press on `Load more` walks toward it; not a binding, since `graphModel.rowOf` is a slot and no binding follows a
    /// slot. Headless: `recover-escape`'s `notLoaded=`.
    property var recoverReach: []
    function readRecoverReach() {
        const picked = page.recoverPicked
        if (picked === null || !graphModel.provisionalWalked || graphModel.loading) {
            page.recoverReach = []
            return
        }
        const tips = discardModel.partTipsAt(page.recoverPick)
        // A part whose commit this repository does not have is drawn nowhere and waits for no walk: its restore is
        // git's to refuse.
        page.recoverReach = picked.parts.map((part, i) => part.look === "uncommitted" || part.look === "absent"
                                                          || graphModel.rowOf(tips[i]) >= 0)
    }
    Connections {
        target: graphModel
        function onStatsChanged() {
            page.landRecoverTip()
            page.readRecoverReach()
        }
    }
    // The search and the entry on the graph each dim the rows and hold the view: as picking ends a search
    // (`pickRecover`), opening one puts the entry down.
    Connections {
        target: graphPane.findCard
        function onOpenChanged() {
            if (graphPane.findCard.open && page.recoverPick >= 0)
                page.pickRecover(page.recoverPick)
        }
    }
    /// The band's press: the picked entry's part `part` brought back, or every part (-1) — one write on the session
    /// (`RepoSession::restore_discard`). The first thing it stands on the graph is selected once it has. Answers
    /// whether the session took it.
    function restoreRecover(part) {
        if (page.recoverPick < 0)
            return false
        const tips = discardModel.partTipsAt(page.recoverPick)
        const looks = page.recoverPicked.parts.map(p => p.look)
        const first = part >= 0 ? part : looks.findIndex(look => look !== "uncommitted")
        page.recoverLanding = first >= 0 && looks[first] !== "uncommitted" ? tips[first] : ""
        const asked = discardModel.restoreAt(page.recoverPick, part)
        // Counted out from here, as the tab's own writes are: a menu raised while it waits its turn waits for it.
        if (asked !== 0)
            repoTab.countWrite(asked)
        page.recoverAwaits = asked !== 0
        return page.recoverAwaits
    }
    property string recoverLanding: ""
    /// A restore taken and not answered yet: the band's buttons wait for the answer, not for the write to start — one
    /// waiting its turn in another worktree's order has not started, and a second press would bring the same part back
    /// twice. Set by the press's answer, as `diffAwaits` is.
    property bool recoverAwaits: false
    /// A restore came back: what it stood is selected, and work that did not come back as it was thrown away says where
    /// it went (`discards::restore`) — the report a write that did not do all it was asked gives (デザイン規約 §答えの要らない報せ).
    /// Nothing showier (user decision): the graph's focus on what came back, and the entry gone from the list.
    readonly property int recoverRestores: repoTab.restoreSeq
    onRecoverRestoresChanged: {
        if (page.recoverLanding !== "")
            page.jumpToRef(page.recoverLanding)
        page.recoverLanding = ""
        if (repoTab.restoreHow === "stash")
            page.showNotice(qsTr("The changes went to the stashes"),
                            qsTr("They would have conflicted with what is here now, so they wait as a stash."),
                            "warning")
        else if (repoTab.restoreHow === "unstaged")
            page.showNotice(qsTr("The changes came back unstaged"),
                            qsTr("What had been staged would not stage again over what is staged now."),
                            "warning")
    }
    /// A fetch reached the remote again, so the panel its failure raised goes down
    /// (デザイン規約 §git が言ったことを読む場所「取り込めるようになったら自分で閉じる」). Read on every drain: the failure
    /// run returning to zero is the recovery.
    function absorbFetchRecovery() {
        if (commandsOwner.landingTakesItDown(repoTab.fetchFailures, repoTab.lastError !== ""))
            page.commandsOpen = false
    }
    /// One mark for a failed command and for this tab's error line both. The page's rule, since the mark moves seats.
    readonly property bool commandsWrong: commandsModel.failed || repoTab.lastError !== ""
    /// The panel, or null while the log is down (built in `commandsSeat` only while raised) — read it through here.
    readonly property var commandsPane: commandsSeat.item
    /// Automation: the colour the `>_` painted — the log's own band while it is up, the pane's foot while it is down.
    readonly property color commandsMarkColor:
        page.commandsPane !== null ? page.commandsPane.markColor : sidebarPane.commandsMarkColor
    /// Whether the panel is really on screen, not only asked for (`commandsOpen`) — automation reads this one, so a cut
    /// binding fails.
    readonly property bool commandsShown: page.commandsPane !== null && page.commandsPane.visible
    /// Automation reads the laid-out width before persisting a state round trip.
    readonly property real stateDetailsWidth: rightPane.width

    RepoTab {
        id: repoTab
        // A turn later: this arrives mid-drain filling this page, and standing the tab elsewhere takes the page down
        // (rules/app-ui.md §Qt Bridges・QML の不変条件 — 二重借用). `Qt.callLater` also folds a repeat ask into one.
        onStandHomeAsked: Qt.callLater(page.standHomeRequested)
    }
    CommandsModel {
        id: commandsModel
        // The UTC offset before the first row: the panel re-says it when raised (`CommandsPane.tellTheZone`), but
        // rows are stamped as they arrive, before any panel is built.
        Component.onCompleted: commandsModel.setZoneMinutes(new Date().getTimezoneOffset())
    }
    GraphModel { id: graphModel }
    WorktreeModel { id: worktree }
    DetailsModel { id: detailsModel }
    DiffModel { id: diffModel }
    RebasePlanModel { id: planModel }
    NavSectionModel { id: branchesModel }
    NavSectionModel { id: remotesModel }
    // One letter apart: the three `*Model`s below are the WIP pane's buckets (uncommitted files); `worktreesModel`
    // lists git worktrees (the sidebar's WORKTREES). One per bucket (`WipPane`), each holding the whole status, so a
    // question about a file (which bucket, what is beside it, where it came from) goes to one of them, `unstagedModel`
    // (through `wipUnstaged`, which is `carriedModel` while the pane is about another worktree).
    NavSectionModel { id: conflictsModel }
    NavSectionModel { id: unstagedModel }
    NavSectionModel { id: stagedModel }
    // Another worktree's changes (`page.carriedPath`), kept apart: this window's own status arrives on its own
    // tick and would take the worktree's rows, and the reader's place, with it. One list, not three: the split is the
    // index's, and nothing here moves that worktree's index (`CarriedPane`).
    NavSectionModel { id: carriedModel }
    NavSectionModel { id: worktreesModel }
    NavSectionModel { id: stashesModel }
    NavSectionModel { id: tagsModel }

    /// True on the page for the tab in front — the only page there is: `RepoPageStack` builds it for that tab and
    /// destroys it when the tab leaves. Set once at construction and acted on in `Component.onCompleted` (a restored
    /// tab's first page reads it there too).
    property bool pageCurrent: false

    /// What this page owes on its way off the front, in order (`TabsModel::leaving_tab`), called while the page is
    /// still whole. The layout first — one set for the application, and the next tab is laid out at it
    /// (`PageLayout.reportLayout`): the page coming in reads the hub's value right after, so a later report would dress
    /// it one generation old. Then the editor's words, which no repository can give back; then the repository.
    function leaveFront() {
        pageLayout.reportLayout()
        if (page.blank)
            return
        repoTab.holdDraft(wipPane.subjectText, wipPane.bodyText, page.amending)
        repoTab.release()
    }

    /// What this page owes on its way into another worktree (`TabsModel::leaving_worktree`). The page stays — graph,
    /// refs and panes are the repository's, shared by linked worktrees
    /// (デザイン規約 §タブの所作「同じリポジトリのタブは 1 枚」); what goes is what the left worktree owned. Called while
    /// that worktree is still behind the tab, so the hub files the draft under it (`Hub::hold_draft`).
    function leaveWorktree() {
        if (page.blank)
            return
        repoTab.holdDraft(wipPane.subjectText, wipPane.bodyText, page.amending)
        // Words left standing would be read as the arrived worktree's (`restoreDraft` puts back only what it finds).
        page.amending = false
        wipPane.setAmendChecked(false)
        wipPane.clearMessage()
        // A plan is composed against one worktree and would be replayed in it.
        if (page.planShown)
            planModel.cancelPlan()
        // The pane goes back to this tab's own tree — which is about to be the worktree it was reading.
        page.dropCarried()
        // A worktree diff goes with the tree. A commit's stays (linked worktrees share the rows), but a read still
        // out died with the session: noted before the close below clears what was being read, and asked again
        // (`standSettled`).
        page.owedDiff = page.diffShown && page.diffKind === "commit" && diffModel.loading
        if (page.diffShown && page.diffKind !== "commit")
            page.closeDiff()
        // The same for the details read, which would otherwise wait in silence (`standSettled`).
        page.owedDetails = detailsModel.loading
        // Every answer still awaited was that session's and will not come (`Hub::let_go_of_session`). Left armed each
        // is a wait nothing ends — and one armed by number (`pendingPopId`) would take the next session's write,
        // counted from the start again, for its own. The landings go too: they are about the tree left.
        page.pendingPopLabel = ""
        page.pendingPopId = 0
        page.forgetRenameCarry()
        page.diffAwaits = false
        page.recoverAwaits = false
        page.moveLanding = ""
        page.pendingHeadSelect = false
        page.pendingHeadAsked = false
        page.pendingHeadFromSeq = 0
        page.pendingWipSelect = false
        page.rewordRow = -1
        page.planRunOut = false
        // A standing question was asked of the tree left — the leave question would read the arriving one's tree and
        // say what never happened in it — and the tab walks away from it.
        page.stopRowAsk()
        // The discard log's entry was walked by that session (`GraphModel.restand` takes it off the rows), and an open
        // list is that worktree's reading: it reads again once the worktree arrived at has settled (`standSettled`).
        page.recoverPick = -1
        page.recoverTipPending = ""
        page.recoverLanding = ""
        page.recoverReach = []
        page.owedDiscards = page.recoverOpen
        // What the models hold of that worktree, each by its own rule (the `restand` slots).
        repoTab.restand()
        graphModel.restand()
        worktree.restand()
        conflictsModel.restand()
        unstagedModel.restand()
        stagedModel.restand()
        carriedModel.restand()
        // The command log is not reset: its rows name their session (`CommandMsg`), and a write the left worktree is
        // still running ends in its own row (`BridgeSink::retire_reads`).
    }

    /// The tab now stands in the asked-for worktree and its session is opening (`TabsModel::stood_worktree`). The words
    /// only: other reads wait for `standSettled` — asked earlier they reach a session with no repository open and get
    /// nothing (`RepoSession::load_details`).
    function standInWorktree() {
        if (page.blank)
            return
        page.restoreDraft()
        // The log's switch is per session (`Recording`) and a new session opens with it off while the panel still
        // shows it on, so it is said again.
        if (commandsModel.backgroundReads)
            commandsModel.setBackgroundReads(true)
    }

    /// Reads that died with the closed session (`leaveWorktree`): the details, the open commit file, and the open
    /// discard log, which was that worktree's reading.
    property bool owedDetails: false
    property bool owedDiff: false
    property bool owedDiscards: false

    /// The new session has said where it is (`RepoTab.standing` going down), so the reads that died with the last are
    /// asked again — only those: a commit is the same in every linked worktree (the rule `activateRow` keeps for the
    /// commit already open). The details were noted on leaving; the signature shows it by not being the selection's.
    function standSettled() {
        if (page.owedDiscards) {
            page.owedDiscards = false
            if (page.recoverOpen)
                page.readDiscards()
        }
        if (page.owedDetails) {
            page.owedDetails = false
            if (page.chosenCount > 1)
                detailsModel.requestSelection(page.chosenIds, page.chosenCount === 2)
            else if (page.selectedOid !== "")
                detailsModel.request(page.selectedOid)
        }
        if (page.selectedOid !== "" && repoTab.signatureOid !== page.selectedOid)
            page.askSignature(page.selectedOid)
        // The open commit file, asked as before (`askCommitDiff` reads the unmoved header).
        if (page.owedDiff) {
            page.owedDiff = false
            if (page.diffShown && page.diffKind === "commit")
                page.askCommitDiff(page.diffPath, page.diffOrigPath)
        }
    }

    /// Puts the last page's draft back into the boxes — before the repository says anything, so a stopped merge's
    /// message (`absorbOpMessage`) and a popped stash's name (`absorbPopLabel`) find them filled and leave them alone.
    function restoreDraft() {
        // The flag as well as the words: an amend's message under a plain commit button would make a second commit.
        const draft = repoTab.draft()
        if (draft.amending) {
            page.amending = true
            wipPane.setAmendChecked(true)
        }
        if (draft.subject === "" && draft.body === "")
            return
        // Show the sentence the reader left mid-way; this also holds the default selection off (`trySelectDefault`
        // leaves a worktree page alone).
        page.showWip()
        wipPane.setMessage(draft.subject, draft.body)
        // `pendingWipSelect` stays down: a dirty tree's row is already at the top, and on a clean tree the flag would
        // stand until the tree got dirty, then jump the reader's view (rules-refs/app-ui.md — the
        // `ListView.highlightFollowsCurrentItem` line).
    }

    // ---- what this page is laid out at ------------------------------
    readonly property PageLayout pageLayout: PageLayout {
        id: pageLayout
        page: page
        sidebarPane: sidebarPane
        rightPane: rightPane
        commandsSeat: commandsSeat
        pageSplit: pageSplit
        failedSeat: failedSeat
        graphPane: graphPane
        noticeBar: noticeBar
        recoverBand: recoverBand
        wipPane: wipPane
        detailsPane: detailsPane
        repoTab: repoTab
        unstagedModel: unstagedModel
        detailsModel: detailsModel
        diffModel: diffModel
    }

    /// What the window reads off the page it shows: the floor (`Main.floorWidth` / `floorHeight`) and the two panes'
    /// own overflow, which the floor's verb asks about.
    readonly property real floorWidth: pageLayout.floorWidth
    /// …and the floor with the list open either way (`PageLayout.openFloorWidth`).
    readonly property real openFloorWidth: pageLayout.openFloorWidth
    readonly property real floorHeight: pageLayout.floorHeight
    readonly property bool wipBlockScrolls: pageLayout.wipBlockScrolls
    readonly property real detailsOverHeight: pageLayout.detailsOverHeight

    /// …and what it calls: the layout is pulled on the window's timer; the setters are the headless checks' (a person
    /// drags).
    function reportLayout() {
        pageLayout.reportLayout()
    }
    function setDetailsWidth(w) {
        pageLayout.setDetailsWidth(w)
    }
    function setGraphColumns(labels, lanes) {
        pageLayout.setGraphColumns(labels, lanes)
    }
    function setSidebarWidth(w) {
        pageLayout.setSidebarWidth(w)
    }

    Component.onCompleted: {
        page.earlyBars.forEach(b => splitWatch.holdSplitBar(b, false))
        page.earlyBars = []
        pageLayout.applySavedLayout()
        if (page.blank)
            return // no session to attach to; every model stays empty
        repoTab.attach(page.tab_id)
        if (page.pageCurrent)
            repoTab.activate()
        commandsModel.attach(page.tab_id)
        graphModel.attach(page.tab_id)
        worktree.attach(page.tab_id)
        detailsModel.attach(page.tab_id)
        diffModel.attach(page.tab_id)
        planModel.attach(page.tab_id)
        discardModel.attach(page.tab_id)
        branchesModel.attachSection(page.tab_id, "branches")
        remotesModel.attachSection(page.tab_id, "remotes")
        conflictsModel.attachWorktree(page.tab_id, "conflicts")
        unstagedModel.attachWorktree(page.tab_id, "unstaged")
        stagedModel.attachWorktree(page.tab_id, "staged")
        carriedModel.attachCarried(page.tab_id)
        worktreesModel.attachSection(page.tab_id, "worktrees")
        stashesModel.attachSection(page.tab_id, "stashes")
        tagsModel.attachSection(page.tab_id, "tags")
        page.restoreDraft()
        // `onPacedChanged` hears only a change: a tab whose repository is already open starts out paced.
        repoTab.setPaced(page.paced)
        const acts = harness.ask()
        if (acts !== null)
            acts.begin()
    }

    /// The window's focus epoch (bumped when the window regains focus) triggers a quick refresh of the visible page.
    property int focusEpoch: 0
    onFocusEpochChanged: {
        if (page.visible && repoTab.state === "open") {
            // The window coming back is when an outside change is most likely waiting: this tree now, with its stashes
            // (the file on screen follows that read — `absorbPacedReads`), and of the other worktrees only the one the
            // pane stands on, its row and its list at once rather than at its turn (`RepoSession::poll_now`). The rest
            // keep their pace. That worktree's open file is asked now; its list follows the worktree's read.
            repoTab.pollNow()
            page.pollCarried()
        }
    }

    /// True while the window is on screen (see Main.qml): the page shown there has its repository read on the
    /// session's pace, so a commit made in a terminal or by an agent turns up on its own.
    property bool onScreen: false
    /// What the session is told (`RepoSession::set_paced`): this tree and the other worktrees are read at its pace
    /// while this holds. Only the tab in front — the others are let go of and read again when switched to. Not while
    /// the tab stands in another worktree: the session behind it is being replaced, and the new one is told as it
    /// opens.
    readonly property bool paced: page.onScreen && page.visible && repoTab.state === "open" && !repoTab.standing
    onPacedChanged: repoTab.setPaced(page.paced)
    /// Paced reads that ended, as the tab counts them (`RepoTab.pacedSeq` / `pacedWorktrees`).
    property int seenPacedSeq: 0
    /// Each paced read of this tree is followed by the file on screen — a status that did not move says nothing about
    /// a file whose bytes did (`pollDiff`); a read of the worktree the pane stands on by that worktree's open file —
    /// the read handed the pane its list already (`pollCarried`). A turn later: this runs inside the tab's notify, and
    /// both reach back into the tab.
    function absorbPacedReads() {
        if (repoTab.pacedSeq !== page.seenPacedSeq) {
            page.seenPacedSeq = repoTab.pacedSeq
            pollDiffLater.restart()
        }
        if (page.carriedPath !== "" && repoTab.pacedWorktrees.indexOf(page.carriedPath) >= 0)
            pollCarriedLater.restart()
    }
    readonly property TurnLater pollDiffLater: TurnLater {
        id: pollDiffLater
        onTriggered: page.pollDiff()
    }
    readonly property TurnLater pollCarriedLater: TurnLater {
        id: pollCarriedLater
        onTriggered: page.pollCarried()
    }
    // The badge counting a running replay out, on its own tick: two file reads, no process (デザイン規約
    // §進行中・回数押しの定数). Not on the page's reads, seconds apart: a badge that says it counts would sit still.
    // `rebase-merge/msgnum` / `end` are numbers git writes to disk, not its human progress output, so this is the one
    // progress shown past the ring. Only while the write is out — the status tick says when the operation ended;
    // `replayRunning` covers the gap a handed-over plan leaves before the queue starts it.
    Timer {
        interval: Metrics.opProgressMs
        repeat: true
        running: page.visible && repoTab.state === "open" && page.replayRunning
        onTriggered: repoTab.refreshOpProgress()
    }
    // Where the selection stands, kept so a commit that disappears from under it can be followed to whatever took its
    // place.
    property int selectedRow: -1

    // ---- the commits being held (デザイン規約 §複数のコミットを選ぶ) ----
    /// The choice, as a set of commit ids, and how many are in it. **Ids**: a background pass rewrites the rows under
    /// the hand, and everything this page holds is re-resolved by id when one lands (`onStatsChanged`). Empty only
    /// where the worktree's row is what is shown — that row is not a commit.
    property var chosenOids: ({})
    property int chosenCount: 0
    /// The commit the last press landed on, which a Shift click measures its range from — an id for the same reason:
    /// a row number kept across a pass would measure from whatever moved into it. Empty before the first press.
    property string chosenAnchorOid: ""
    /// The choice as the models want it: the ids in walk order, and the rows that name them. Settled together, so the
    /// list on the right and the files under it can never be of different commits.
    property var chosenIds: []
    property var chosenRecords: []
    /// Makes one commit the whole of the choice. Every way a single row becomes what is being read comes through here
    /// — a plain click, an arrow key, a landing, the find bar.
    function chooseOnly(oidHex) {
        const only = ({})
        // A row of uncommitted work names no commit (its id is empty), so nothing is held for it.
        if (oidHex !== "")
            only[oidHex] = true
        page.chosenOids = only
        page.chosenCount = Object.keys(only).length
        page.chosenIds = []
        page.chosenRecords = []
    }
    /// A left click on a row, as this page answers one. **The choice and what is being read are two answers**: a plain
    /// click makes one commit both, a modified one moves only the choice (デザイン規約 §複数のコミットを選ぶ).
    function pickRow(oidHex, atRow, modifiers) {
        const mods = modifiers === undefined ? Qt.NoModifier : modifiers
        // A row of uncommitted work is not a commit (its id is empty): a modifier on it is the plain click it would be
        // without one.
        if (mods === Qt.NoModifier || oidHex === "") {
            page.activateRow(oidHex, atRow)
            return
        }
        if (mods & Qt.ShiftModifier) {
            // An anchor the graph no longer draws leaves the range to measure from the row being read.
            const anchorRow = page.chosenAnchorOid === "" ? -1 : graphModel.rowOf(page.chosenAnchorOid)
            page.chooseRange(anchorRow >= 0 ? anchorRow : page.selectedRow, atRow)
            return
        }
        // A press on a row of the graph is where the next Shift click measures from — the one press that moves the
        // anchor without moving what is read.
        page.chosenAnchorOid = oidHex
        page.chooseAlso(oidHex)
    }
    /// Puts a commit into the choice, or takes it back out — a Ctrl click. The anchor is left where it was: which
    /// presses move it is the callers' to say.
    ///
    /// **One always stays in.** An empty choice is the worktree's own state, and reaching it from a commit would
    /// leave the right pane describing something no row is drawn as holding.
    function chooseAlso(oidHex) {
        // A fresh object: assigning the same one back notifies nothing, and the rows follow this property.
        const next = ({})
        for (const held in page.chosenOids)
            next[held] = true
        if (next[oidHex] === true) {
            if (page.chosenCount <= 1)
                return
            delete next[oidHex]
        } else {
            next[oidHex] = true
        }
        page.settleChoice(next)
    }
    /// The same Ctrl click, made in the list of what is held (`ChosenCommitRow`) — it only ever takes a commit out.
    ///
    /// **The anchor stays where the graph left it**: this press was not on a graph row, so a Shift click after it
    /// measures from the last row a hand actually landed on.
    function dropFromChoice(oidHex) {
        page.chooseAlso(oidHex)
    }
    /// A plain click in that list: the graph brings the commit's row into the open (`GraphRowWalk.revealRow`). The
    /// choice, the row the arrows step from and the commit being read stay as they are — it finds a commit, it picks
    /// none. **An open diff closes first and the graph comes back** (user decision — ask before changing): a graph
    /// moved behind the diff would show the reader nothing until the diff went, as a parent's link closes it too
    /// (`jumpToRef`). Closed the way its `✕` closes it, so the keyboard goes back to the graph.
    function showChosen(oidHex) {
        if (page.diffShown)
            page.closeDiffToGraph()
        graphPane.revealRow(graphModel.rowOf(oidHex))
    }
    /// Reads the rows of the chosen commits again for the list on the right: what it says about them (a subject, a
    /// face) is read off the rows, so it is read again whenever the rows are.
    function refreshChosenRecords() {
        if (page.chosenIds.length > 0)
            page.chosenRecords = graphModel.chosenRows(page.chosenIds)
    }
    /// Every commit row between two places, ends included — what a Shift click reaches, **the rows scrolled past
    /// too**. Asked of the model in one crossing (`GraphModel.oidsBetween`): a call per row would put a walk of the
    /// history on a click (internal-docs/性能.md §設計規則).
    function chooseRange(from, to) {
        if (from < 0 || to < 0)
            return
        const next = ({})
        // The commits alone: a row of uncommitted work between the two is none to hold.
        for (const oidHex of graphModel.oidsBetween(from, to))
            next[oidHex] = true
        // A range that swept nothing but the worktree's row is not a choice; the anchor stays where it was.
        if (Object.keys(next).length === 0)
            return
        page.settleChoice(next)
    }
    /// The one place the set and its tally are written together, so a count and a highlight cannot disagree. **A
    /// choice back down to one commit is that commit being read** (`activateRow`, with `followsTree` as it is).
    function settleChoice(next, followsTree) {
        const ids = Object.keys(next)
        if (ids.length === 1) {
            page.activateRow(ids[0], -1, followsTree)
            return
        }
        page.chosenOids = next
        page.chosenCount = ids.length
        page.readChoice(ids)
    }
    /// Puts the choice to the graph for its order and the rows that name it, and asks the pane on the right for what
    /// these commits hold. **Two are read as what differs between them, three or more as what all of them changed**
    /// (デザイン規約 §複数のコミットを選ぶ).
    function readChoice(ids) {
        page.chosenIds = graphModel.presentOids(ids)
        page.chosenRecords = page.chosenIds.length === 0 ? [] : graphModel.chosenRows(page.chosenIds)
        if (page.chosenIds.length === 0)
            return
        // None of what a single commit's pane says applies to several: no stash sits under a choice, and the diff
        // that was open was of one file of one commit.
        page.selectedStashRef = ""
        page.closeDiff()
        detailsModel.requestSelection(page.chosenIds, page.chosenCount === 2)
    }
    /// Drops from the choice whatever the graph no longer stands on. Run when a pass lands: a rewrite takes commits
    /// away, and a choice that goes on counting them says a number no row on screen adds up to.
    function settleChoiceAfterPass() {
        if (page.chosenCount <= 1)
            return
        const kept = graphModel.presentOids(Object.keys(page.chosenOids))
        if (kept.length === page.chosenCount) {
            // The same commits. In the same order only what the list says about them is re-read (a reworded subject,
            // a face); in another order the choice is read again whole — the pane compares from the older of two.
            if (kept.every((oidHex, at) => oidHex === page.chosenIds[at]))
                page.refreshChosenRecords()
            else
                page.readChoice(kept)
            return
        }
        // Everything the choice named is gone. What is being read answers for it — that one is followed by name
        // (`followVanishedCommit`), and the choice comes back as it.
        if (kept.length === 0) {
            page.chooseOnly(page.selectedOid)
            return
        }
        const next = ({})
        for (const oidHex of kept)
            next[oidHex] = true
        page.settleChoice(next, true)
    }

    // The commit the viewport is measured against between passes, so the rows a reader is on can be put back under them
    // when new ones arrive above. The newest *real* commit: the WIP row comes and goes with the worktree.
    property string anchorOid: ""
    property int anchorRow: -1
    function rememberAnchor() {
        // Past however many worktree rows stand over it — ours and one per other worktree, none a commit — and the
        // stashes (`GraphModel.newestCommitRow` holds the rule).
        const row = graphModel.newestCommitRow()
        page.anchorOid = row < 0 ? "" : graphModel.oidAt(row)
        page.anchorRow = row
    }
    // How far the graph slid under the viewport: under the hand, by the row it is over — what the hand is on stays
    // where it is (デザイン規約 §hover のツールチップ「外乱では閉じない」); elsewhere by the newest commit. Zero when the
    // anchor is gone: a rewrite deep in the history moves rows by different amounts and there is no single answer, so
    // the view is left alone.
    function anchorShift() {
        const held = graphPane.handShift()
        if (held !== undefined)
            return held
        if (page.anchorRow < 0 || page.anchorOid === "")
            return 0
        const now = graphModel.rowOf(page.anchorOid)
        return now < 0 ? 0 : now - page.anchorRow
    }

    // A landing on HEAD this page still owes. Held over: the worktree's status, the refs and the walk arrive as
    // three separate messages, and the two that come first still describe the repository as it was — landing off them
    // picks the commit that was just replaced. Resolved once the graph holds where HEAD points.
    property bool pendingHeadSelect: false
    /// The first report of HEAD that may answer the landing (`WorktreeModel.headSeq`). A write's answer names it
    /// (`RepoTab.writeAnswerHeadSeq`): the report in hand at that arming may predate the write, with a stale HEAD that
    /// has a row. Never `headSeq + 1`: the status feed and the tab's are separate queued invokes, and on a clogged GUI
    /// the report after the write drains before the answer, so that number never comes. An arming read out of a report
    /// already in hand — a tree somebody else emptied, the selected commit vanishing — takes that very report; a tree
    /// this page's own press emptied arms on the write's fence (`takeStashLanding` / `takeCommitLanding`).
    property int pendingHeadFromSeq: 0
    // Whether the reader asked for the landing, in which case the viewport goes to it too — a commit they meant to make
    // is no answer off screen. The other ways it is owed happen *to* the window (a terminal commit, a rewrite under the
    // poll), and those may not move the reader's view: it is the reader's (why `AppListView` sets
    // `highlightFollowsCurrentItem: false`). A landing already on screen leaves the view be (`GraphPane.showRowSoon`).
    property bool pendingHeadAsked: false
    /// Whether the worktree the last status described is empty. **Every reader stands behind `worktree.loaded`**: the
    /// counts start at zero, which reads as a finished job before any status (rules/app-ui.md §UI 自動化).
    readonly property bool treeClean: worktree.stagedCount === 0 && worktree.unstagedCount === 0
                                   && worktree.untrackedCount === 0 && worktree.conflictCount === 0
    /// What operation the last status named, so that its going away can be read as an edge.
    property string seenOpText: ""
    /// The WIP face has nothing left to hold the reader with — the worktree emptied, or the operation went away
    /// (which takes the exit card off the face) — so land on the commit that now holds the changes. When someone else
    /// committed them, a message being written keeps the reader here: it is the one thing that cannot be read back off
    /// disk.
    ///
    /// **A stash pressed here lands past the message**: the press is the decision to empty the tree, and the entry took
    /// the words for its name. Left to the message, a stopped merge (whose box `absorbOpMessage` fills) never lands,
    /// and the stash row that press made inherits row 0's highlight the worktree row gave up. Headless: `stash-lands`,
    /// `rebase-edit-stop-out`, `op-exit-lands`.
    ///
    /// `edge` is what the status moved (the tree's counts, or the operation); every half is read off that one status,
    /// for two reasons. The sidebar lists say `changed()` only when their rows move (`nav/drain.rs` `arrived`), so a
    /// stop that ends on a clean tree (an `edit` stop let go) is 0 → 0 and fires nothing there. And `hub::sink` pushes
    /// one status as the three nav feeds, then the status headline, drained in that order (`Feed::wake` is a queued
    /// invoke): a list's handler reads `worktree.opText` one snapshot old, and the everyday conflict resolved →
    /// `--continue`, where the rows empty and the operation ends in one status, would be turned away by the `REBASING`
    /// before it. So emptiness is `treeClean`'s four counts too — mixing in `unstagedModel.total` brings the order
    /// back. Fired on the edge, not the level: on a clean tree, a message box emptied by hand would otherwise walk the
    /// reader off the face at the next poll. A landing of our own needs no edge.
    function leaveWipWhenDone(edge) {
        // `treeClean`'s guard. `loaded` latches on the first status, so this holds off only a page's opening moment.
        if (!worktree.loaded)
            return
        // **A pane about another worktree is that worktree's face** — its changes are still there, so nobody is walked
        // off it. The landing below stays armed for the reader coming back to their own row.
        if (!page.wipWritable)
            return
        // Whether a press of this window's own emptied it — the stash, or the editor's commit (`ops::TreeOut`) —
        // **asked from both sides of the pair**: the status and the answer arrive in no fixed order, and the first
        // asks and gets nothing. Answered with the write's fence, the first report of HEAD after it.
        const ours = Math.max(repoTab.takeStashLanding(), repoTab.takeCommitLanding())
        if (!page.treeClean)
            return
        // **A landing of our own is an edge in itself**: the press is what moved.
        if ((!edge && ours === 0) || !page.wipShown || worktree.opText !== "")
            return
        if (ours === 0 && (wipPane.subjectText !== "" || wipPane.bodyText !== ""))
            return
        page.readRow(undefined)
        page.pendingHeadSelect = true
        // Ours lands on HEAD as the write left it — the first report after the write, whatever report is in hand: an
        // amend of nothing leaves the counts as they were, and the report in hand may be of the commit it replaced. A
        // tree somebody else emptied lands on the report this status was read beside, already in hand (`hub::sink`
        // sends it ahead of the status).
        page.pendingHeadFromSeq = ours > 0 ? ours : worktree.headSeq
        // Only ours is a landing anybody asked for, so only ours takes the viewport along.
        page.pendingHeadAsked = ours > 0
    }
    function tryPendingHeadSelect() {
        if (!page.pendingHeadSelect || !worktree.headKnown)
            return
        // `headOid` is HEAD branch or not, so a detached landing is the same.
        if (worktree.headSeq < page.pendingHeadFromSeq)
            return
        const row = worktree.headOid !== "" ? graphModel.rowOf(worktree.headOid) : -1
        if (row < 0)
            return
        page.pendingHeadSelect = false
        const asked = page.pendingHeadAsked
        page.pendingHeadAsked = false
        graphPane.setCurrentRow(row)
        // A landing this window's write asked for answers a press; one a tree emptied elsewhere follows the tree.
        page.activateRow(graphModel.oidAt(row), row, !asked)
        if (asked)
            graphPane.showRowSoon(row)
    }

    // An operation stopped part-way and this page owes it a landing on the worktree, where the conflicts and the way
    // out are. Held like `pendingHeadSelect`: the status carrying those rows arrives after git's answer. Always asked
    // for — a stop only ever follows a press — so the viewport goes along. One wiring for every stop (merge,
    // cherry-pick, revert, rebase onto, and the replays of squash / drop / reword): the page looks only at "stopped",
    // which core says as `WriteStopped` and the answer carries as its own (`writeAnswerStopped`; its `error` is empty,
    // so no band and no log). The tip stays where it was. Work carried over a dirty tree stays in the stash through a
    // stop (`session::build::carry_across`): the STASHES count and the graph's stash row are what say where it went.
    // Paid in the graph's finish pass: the status lands before the walk, so `unstagedModel.total` would come too early
    // and land on the old tip.
    // Headless: `merge-stops` / `cherry-pick-stops` / `revert-stops` / `rebase-stops` / `drop-stops`.
    property bool pendingWipSelect: false
    function tryPendingWipSelect() {
        if (!page.pendingWipSelect)
            return
        // Until the walk has prepended our row there is none to land on: ours is asked for by its key, not as "a row of
        // uncommitted work on top" — a neighbour worktree's row is one too.
        const row = graphModel.rowOfKey(GitFacts.hereKey())
        if (row < 0)
            return
        page.pendingWipSelect = false
        graphPane.setCurrentRow(row)
        page.showWip()
        graphPane.showRowSoon(row)
    }

    // The selected commit is gone from the graph and no landing of this page's accounts for it: an amend or a rebase in
    // a terminal, or a rebase / squash / drop (a reword has `followRewrittenCommit`). Whatever now stands where it
    // stood (its row number) is the closest thing to what was being read; failing that, where HEAD stands. Not the
    // tip: that loses the place being read. Not the first-parent line either (user decision): a sibling branched off
    // the same parent can be the date-order neighbour (P3-確認事項 §参照していた行が消えた時の着地). A landing owed
    // first wins: while a stopped operation borrows the WIP row (`pendingWipSelect`), this does not follow.
    function followVanishedCommit() {
        // A row of uncommitted work names no commit (its id is empty), and is no commit to stand in for this one.
        const oidHex = graphModel.oidAt(page.selectedRow)
        if (oidHex !== "") {
            graphPane.setCurrentRow(page.selectedRow)
            page.activateRow(oidHex, page.selectedRow, true)
            return
        }
        page.readRow(undefined)
        page.pendingHeadSelect = true
        // The graph that let the commit go was rebuilt behind a read that had already reported where HEAD went, so
        // the report in hand is the one to land on.
        page.pendingHeadFromSeq = worktree.headSeq
    }

    // A reworded commit came back under a different hash: the one now standing where it stood is it, since only the
    // message changed.
    function followRewrittenCommit() {
        const row = page.rewordRow
        const oidHex = graphModel.oidAt(row)
        // Nothing there, or a worktree's row moved under it — neither names a commit.
        if (oidHex === "") {
            page.rewordRow = -1
            return
        }
        graphPane.setCurrentRow(row)
        page.activateRow(oidHex)
    }

    /// `atRow`: the row, for callers that already know it (a click does); -1 asks the model (`GraphModel.rowOf`, a
    /// binary search of its index), which a landing named only by its commit — the arrows, the search — has to do.
    /// `followsTree`: the landing follows the tree — it emptied, the commit being read went, a pass took commits off
    /// the choice — rather than anything pressed in this window, so a standing question stays up and says the tree as
    /// it now is (`LeaveFlow`).
    function activateRow(oidHex, atRow, followsTree) {
        // The history moved — by the arrows, the search, a menu or a click: a rename's second click is not one across
        // it (`Hand.acted`; a click's own landing is taken by its target). `jumpToRef` is the other road.
        Hand.acted()
        page.rewordRow = -1
        page.pendingHeadSelect = false
        page.pendingHeadAsked = false
        page.pendingWipSelect = false
        // Clicking anywhere is the way out of the name box and of a standing row question.
        graphPane.stopNaming()
        if (followsTree !== true)
            page.stopRowAsk()
        // The right pane's mark is about the commit it was raised over, and every road to another commit comes through
        // here, keyboard ones included — the one road that keeps it raises it again (`onMessageAsked`).
        detailsPane.dropAttention()
        // **A plain landing is one commit** (デザイン規約 §複数のコミットを選ぶ). Whether it was several is carried past the early
        // return below: a click on the commit already open still has to run everything if the pane is showing a choice.
        const wasChoice = page.chosenCount > 1
        page.chooseOnly(oidHex)
        const row = atRow !== undefined && atRow >= 0 ? atRow : graphModel.rowOf(oidHex)
        if (row >= 0) {
            graphPane.setCurrentRow(row)
            page.selectedRow = row
            page.chosenAnchorOid = oidHex
        }
        // **A row of uncommitted work is not a hash**: nothing can be skipped for it, it shows whatever the tree is
        // now. Which tree is the row's to say (`GraphModel.keyAt`, `openWipFor`).
        const key = row >= 0 ? graphModel.keyAt(row) : GitFacts.commitKey(oidHex)
        if (key !== undefined && key.kind !== "commit") {
            page.openWipFor(row)
            return
        }
        // The commit already open: its details and signature answer a hash, which cannot change, so nothing is asked
        // again (`git show`, `merge-base`, gpg) — the rename gesture's second click is exactly this click
        // (デザイン規約 §グラフ行のダブルクリック).
        if (!page.wipShown && !wasChoice && page.selectedOid === oidHex)
            return
        // A commit is nobody's worktree, so the pane stops being about one (`readRow`).
        page.readRow(GitFacts.commitKey(oidHex))
        page.selectedStashRef = graphModel.stashRefOf(oidHex)
        detailsModel.request(oidHex)
        page.askSignature(oidHex)
        page.closeDiff()
    }

    /// Something other than the reader is picking the row this page stands on, so the default below stays out of its
    /// way — it would land first and be photographed instead (`PageAutoStart`). False in every window a person opens.
    property bool rowPickedElsewhere: false

    // The opening landing (user decision — ask before changing): the worktree's own row where the tree has one, else
    // HEAD's commit (detached too: the commit HEAD stands on), else the newest commit — so the right pane always shows
    // something.
    function trySelectDefault() {
        if (page.selectedOid !== "" || page.wipShown || page.pendingHeadSelect || page.rowPickedElsewhere
                || graphModel.rowTotal === 0)
            return
        // Both reads must have answered: before the first status `wipRowStands` is false only because nothing was
        // asked, and a landing on a commit then is never taken back (the first line above holds every later call off).
        if (!worktree.headKnown || !worktree.loaded)
            return
        let row = -1
        if (worktree.wipRowStands) {
            // **Uncommitted work is the landing, branch or not**: the work standing there is where the reader came back
            // to — a dirty tree, or one where an operation with an exit card stands. Asked of the status
            // (`graph::wip_row_stands`) — whether the opening walk carries the row races the first status — and of the
            // graph by its key, for the reason `tryPendingWipSelect` gives. Every pass calls this again.
            row = graphModel.rowOfKey(GitFacts.hereKey())
            if (row < 0)
                return
        } else {
            // Where HEAD stands, branch or not (`WorktreeModel.headOid` — what `tryPendingHeadSelect` lands on too).
            row = worktree.headOid !== "" ? graphModel.rowOf(worktree.headOid) : -1
            if (row < 0) {
                if (graphModel.loading)
                    return // the head row may still be streaming in
                // HEAD outside the window: the newest commit, not the newest row where stashes stand over it
                // (`GraphModel.newestCommitRow`). A repository whose rows are all stashes has no commit to open on.
                row = graphModel.newestCommitRow()
                if (row < 0)
                    return
            }
        }
        graphPane.setCurrentRow(row)
        graphPane.anchorSoon()
        // Nobody pressed for it: the opening, or the tree gone from under what was being read.
        page.activateRow(graphModel.oidAt(row), row, true)
    }
    // Each finished pass bumps finishCount: re-resolve the selection by oid, since row numbers may have shifted. Only a
    // streaming restart bumps resetCount — the one case where the viewport lost its position and needs re-anchoring;
    // in-place replacements keep it, and re-centering would yank the view around.
    property int seenFinishCount: 0
    property int seenResetCount: 0
    Connections {
        target: graphModel
        function onStatsChanged() {
            if (graphModel.finishCount !== page.seenFinishCount) {
                page.seenFinishCount = graphModel.finishCount
                const resetHappened = graphModel.resetCount !== page.seenResetCount
                page.seenResetCount = graphModel.resetCount
                // A reset starts the viewport over anyway; only in-place replacements leave it pointing at rows that
                // moved — and what hangs off a commit standing on a row that shows another now (`RowHoverHost.reseat`).
                if (!resetHappened) {
                    graphPane.shiftRows(page.anchorShift())
                    rowHost.reseat()
                }
                graphPane.noteHand()
                page.rememberAnchor()
                // A stopped operation lands on the worktree's own row, which this pass is what puts there.
                page.tryPendingWipSelect()
                page.settleCarriedAfterPass()
                if (page.pendingHeadSelect) {
                    page.tryPendingHeadSelect()
                } else if (page.selectedOid !== "") {
                    const row = graphModel.rowOf(page.selectedOid)
                    if (row >= 0) {
                        page.selectedRow = row
                        graphPane.setCurrentRow(row)
                        if (resetHappened)
                            graphPane.anchorSoon()
                    } else if (page.pendingWipSelect) {
                        // **A landing the reader asked for outranks the two below.** A stopped operation owes them
                        // the worktree's row (デザイン規約 §進行中の操作から出る), which this pass may not have yet;
                        // any other landing now clears the owed one (`activateRow`).
                    } else if (page.rewordRow >= 0) {
                        page.followRewrittenCommit()
                    } else if (page.selectedRow >= 0) {
                        // Only a selection that had a row: one never on screen (a details jump past the walk window)
                        // has not vanished, only gone unwalked — the reader stays on it.
                        page.followVanishedCommit()
                    }
                }
                // After the one being read has been placed: the two follows above settle a choice of their own, and
                // this drops whatever else the pass took away.
                page.settleChoiceAfterPass()
            }
            page.trySelectDefault()
        }
    }
    // Where HEAD stands rides the worktree model, so the landings and the default selection are paid there
    // (`worktree.onChanged`), not on the refs.
    Connections {
        target: branchesModel
        function onRefsSettled() {
            // The listing is the half a move onto a branch that was not there waits on: the status behind the write
            // already has HEAD on it, and until this arrives the screen still says no such branch (`moveLanding`).
            page.absorbMoveLanding()
            page.listingDrawn()
        }
    }
    /// A list has drawn what it was handed, and a delete may be waiting on that list's own row — so **all five say
    /// it** — one left out leaves its rows to nobody — and **which list is not passed on**
    /// (`ops_delete::note_listing_drawn`, `ops::StandIn`). Withheld while a run holds the in-between open for a picture
    /// (`holdGoneRows`; headless `delete-gone` — the put-back side is `delete-branch-refused`'s picture).
    function listingDrawn() {
        if (!page.holdGoneRows)
            repoTab.listingDrawn()
    }
    Connections {
        target: remotesModel
        function onRefsSettled() {
            page.listingDrawn()
        }
    }
    Connections {
        target: tagsModel
        function onRefsSettled() {
            page.listingDrawn()
        }
    }
    Connections {
        target: diffModel
        // The rows are about to be laid out again as one column or two (`DiffModel.relay_rows`), a whole new list: the
        // place is held as the swap begins (`DiffScrollPlace`). A read of the same file moves only the rows that
        // changed and keeps the place by itself (`DiffPane.rowKeep`). Held here, not where a read is asked for
        // (`reloadDiff` / `pollDiff`): a tick that finds the file unmoved answers nothing, and swaps nothing.
        function onRowsReplacing() {
            diffPane.holdScroll()
        }
    }
    // The stashes arrive on a word of their own, after the refs — read off the refs' arrival, a dropped stash would be
    // back on screen for the graph rebuild between the two (`session::write`).
    Connections {
        target: stashesModel
        function onStashesSettled() {
            page.listingDrawn()
        }
    }
    // The worktrees on a word of their own too: their listing follows the write apart from the refs', and a
    // removed worktree's row answers only to it.
    Connections {
        target: worktreesModel
        function onWorktreesSettled() {
            page.listingDrawn()
        }
    }
    /// The `WorktreeModel.treeRevision` the open diff was last read against — bumped when the four buckets' counts
    /// move. Per file: a second hunk staged out of a file on both sides moves none, and is re-read at the write's
    /// answer (this window) or by `pollDiff` (anyone else).
    property int seenTreeRev: -1
    // **The tree was read** — heard from the worktree model, not the list, which says `changed` only when its rows
    // differ: a status that moved no row is exactly the one this has to hear about. Also said where the counts were
    // drawn again with the stage presses in them (`restage`): no read, so `treeRevision` does not move.
    Connections {
        target: worktree
        function onChanged() {
            const moved = worktree.treeRevision !== page.seenTreeRev
            page.seenTreeRev = worktree.treeRevision
            // The operation that was standing is not standing any more — the other edge the WIP face's exit turns on,
            // and the only one a clean stop ever moves (`leaveWipWhenDone`).
            const opGone = page.seenOpText !== "" && worktree.opText === ""
            page.seenOpText = worktree.opText
            // **Written down before anything asks what it means**, waited on or not: a write's answer and the status it
            // published are drained apart, and this may be the half that arrives first.
            if (worktree.loaded)
                repoTab.noteTreeRead(worktree.statusSeq, page.treeClean)
            // Whether this is the status our own write published, whose file was already re-read at the answer
            // (`ops::DiffReread`, `readDiffForAnswer`).
            const ours = repoTab.takeDiffRead()
            // Something outside this window moved the tree: the rows on screen, and the fingerprint the next `+` is
            // written against, describe the file as it was, and git would refuse the press.
            if (moved && !ours)
                page.reloadDiff()
            page.absorbOpMessage()
            // ...and the other half of a move's landing: this is what carries the branch HEAD ended up on.
            page.absorbMoveLanding()
            // Whether the WIP face still has anything to stand for. **After `absorbOpMessage`**: an abort takes the
            // words a stopped merge put in the box back out — asked before it, the box reads as a message being typed.
            page.leaveWipWhenDone(moved || opGone)
            // HEAD's report rides this model (`WorktreeModel.headSeq`), so an owed landing is paid here — even for a
            // write that recorded nothing: the first read after a write always sends one.
            page.tryPendingHeadSelect()
            page.trySelectDefault()
        }
    }
    Connections {
        // Whichever list the pane is showing: the file the diff is on belongs to the worktree those rows came from, and
        // so does the side the pane would move to when it runs out (`followEmptySide`).
        target: page.wipUnstaged
        function onChanged() {
            // Neighbour first, while the diff's file is still where it was (`diffNeighbour`).
            page.noteDiffNeighbour()
            page.followEmptySide()
            // **The face's own exit is not asked here** but off the status (`leaveWipWhenDone`): half the ways out of
            // it move no row in this list.
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // The items' minimums come down with the room in a window under its floor (`PageLayout.panesHeldHeight`).
        SplitView {
            id: pageSplit
            Layout.fillWidth: true
            Layout.fillHeight: true
            orientation: Qt.Vertical
            handle: SplitHandleBar {
                onHandChanged: (which, held) => page.holdSplitBar(which, held)
            }

            // ---- open failed ------------------------------------------
            // Built while the tab's open has failed and taken down with it (rules-refs/app-ui.md — the
            // `WindowDialogSeat` line): all it shows is the tab's. The seat is what stands in the split, so the split's
            // own properties are its.
            Loader {
                id: failedSeat
                visible: page.openFailed
                active: page.openFailed
                SplitView.fillHeight: true
                SplitView.minimumHeight: pageLayout.failedHeldHeight
                sourceComponent: OpenFailedScreen {
                    kind: repoTab.errorKind
                    path: repoTab.errorPath
                    message: repoTab.error
                    pageWidth: page.width
                    commandsPage: page
                    onCloseRequested: page.closeTabRequested()
                }
            }

            // ---- three-pane layout --------------------------------------
            SplitView {
                visible: !page.openFailed
                SplitView.fillHeight: true
                SplitView.minimumHeight: pageLayout.panesHeldHeight
                orientation: Qt.Horizontal
                handle: SplitHandleBar {
                    onHandChanged: (which, held) => page.holdSplitBar(which, held)
                }

                SidebarPane {
                    id: sidebarPane
                    keepBuilt: page.keepBuilt
                    // The plan's freeze takes the pane whole (the mode's own restriction); a replay behind the screen
                    // holds only the doors — a switch, a delete, a name box, the `+` — and reading goes on
                    // (`SidebarPane.frozen` / `doorsHeld`).
                    frozen: page.sidebarFrozen
                    doorsHeld: page.doorsHeldWhy !== ""
                    // Over the pane beside it while a name box stands: the box reaches past this pane's edge when its
                    // text does not fit, and the graph is laid out after this one (`NavItemDelegate`). Only then — the
                    // rest of the time this pane's edge would be drawn over the splitter.
                    z: sidebarPane.editKey !== "" ? 1 : 0
                    repoTab: repoTab
                    worktree: worktree
                    branchesModel: branchesModel
                    remotesModel: remotesModel
                    worktreesModel: worktreesModel
                    stashesModel: stashesModel
                    tagsModel: tagsModel
                    collapsed: page.listFolded
                    // Null on the blank page: nothing has run there (the band's `>_` answered the same way up above).
                    commandsPage: page.blank ? null : page
                    recoverPage: page.blank ? null : page
                    // The menus the rows raise are the page's, so only the page can say where the standing one came
                    // from — the folded list's open section stays under its own rows' menu alone.
                    menuDoor: page.menuDoor
                    onFoldRequested: collapse => page.foldByHand(collapse)
                    onRefActivated: oidHex => page.jumpToRef(oidHex)
                    onRefMenuRequested: (kind, name, full, oidHex, aim, area) =>
                        page.openRefMenu(kind, name, full, oidHex, area, aim)
                    onRemoteMenuRequested: (name, area) => page.openRemoteMenu(name, area)
                    onWorktreeActivated: path => page.openRepositoryPathRequested(path)
                    onRefSwitchRequested: (kind, name) => page.switchToRef(kind, name)
                    onBranchAtRequested: (oidHex, name) => {
                        if (name !== "")
                            repoTab.createBranch(name, oidHex, true)
                    }
                    onTagAtRequested: (oidHex, name) => {
                        if (name !== "")
                            repoTab.createTag(name, oidHex)
                    }
                    onWorktreeAtRequested: (oidHex, name) => page.worktreeFromBox(oidHex, name)
                    onRenameSubmitted: (kind, id, oid, name) => page.renameRow(kind, id, oid, name)
                    onAddRemoteRequested: publishFlow.startAddRemote()
                }

                // Center: the report on a row of its own, then the commit graph ⇄ file diff. A write is answered
                // wherever the reader stands (デザイン規約 §答えの要らない報せ; user decision — ask before changing): inside the graph
                // it would be silent while a diff is open, and over both it would cover what it is about. The bar
                // pushes the middle down, never covers it. **One bar over the middle at a time: the report, then the
                // graph's question (`GraphPane`'s `AskBar`), then the discard band** — news of a write that is over
                // first, then what waits on an answer, then the state that stands as long as an entry is picked. A
                // question asked under a report waits until the report is gone, and one standing steps aside for a
                // report and comes back with its words and what was typed into it (`AskBar.held`); the band steps aside
                // for either and comes back once both are gone. **One hands the middle to the next in place**
                // (`BarTravel`, user decision): the rows under them move by the difference in height only.
                //
                // Whatever stands over the graph leaves it its rows (`PageLayout.graphKeptHeight`), giving way inside
                // itself down to its least (デザイン規約 §窓の床).
                ColumnLayout {
                    id: centreColumn
                    SplitView.fillWidth: true
                    // The graph's columns once both have given all they can, held up to a side pane's width so the
                    // middle never reads as the thinnest of the three (`PageLayout.centreMinWidth`) — in a window at
                    // least as wide as its floor; narrower, the middle gives way (`PageLayout.centreHeldWidth`).
                    SplitView.minimumWidth: pageLayout.centreHeldWidth
                    spacing: 0

                    NoticeBar {
                        id: noticeBar
                        Layout.fillWidth: true
                        // Alone over the graph (`centreColumn`): reading what waits behind it would loop through
                        // `graphPane.askHeld` and the band's `visible`.
                        room: centreColumn.height - pageLayout.graphKeptHeight
                        givesWay: graphPane.asking || page.recoverBandWanted
                        onAcknowledged: page.hideNotice()
                        // The place its line names, gone to as a tooltip's is.
                        onLinkAsked: href => page.tipLinkAsked(href)
                    }
                    // Over the graph alone, which is what it describes (`RecoverBand`), and only once the report and
                    // the question are gone (`centreColumn`).
                    RecoverBand {
                        id: recoverBand
                        Layout.fillWidth: true
                        visible: page.recoverBandWanted && noticeBar.shut && !graphPane.asking && graphPane.askShut
                        entry: page.recoverPicked
                        when: page.recoverPicked === null
                              ? "" : recoverWords.whenWords(page.recoverPicked.at, page.recoverNow)
                        reach: page.recoverReach
                        canWalkFurther: graphModel.truncated && !graphModel.growing
                        step: graphModel.windowStep
                        busy: repoTab.busyCount > 0 || page.recoverAwaits
                        room: centreColumn.height - pageLayout.graphKeptHeight
                        onRestoreAsked: part => page.restoreRecover(part)
                        onDismissed: page.pickRecover(page.recoverPick)
                        onFurtherAsked: graphModel.growWindow()
                    }

                    StackLayout {
                        id: centreStack
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        // The plan stands over both from the press that asked for it, before its rows exist
                        // (`planShown`; an open diff closes on the way in, `onPlanShownChanged`), and the graph comes
                        // back as it was.
                        currentIndex: page.planShown ? 2 : page.diffShown ? 1 : 0

                        GraphPane {
                            id: graphPane
                            keepBuilt: page.keepBuilt
                            askRoom: centreColumn.height - pageLayout.graphKeptHeight
                            // Until the report is gone (`centreColumn`).
                            askHeld: !noticeBar.shut
                            askGivesWay: page.recoverBandWanted || page.noticeWaits
                            graphModel: graphModel
                            worktree: worktree
                            blank: page.blank
                            listKey: rowHost.listKey
                            cardKey: rowHost.cardKey
                            chosenOids: page.chosenOids
                            chosenCount: page.chosenCount
                            onRowActivated: (oidHex, atRow, modifiers) => page.pickRow(oidHex, atRow, modifiers)
                            onKeyReached: page.dropWithheld()
                            // The bar moves between matches.
                            onFindLanded: oidHex => {
                                // Only where the match moved to another row: a selection brings three spawns.
                                if (oidHex !== "" && oidHex !== page.selectedOid)
                                    page.activateRow(oidHex)
                            }
                            onRowMenuOpenRequested: (oidHex, chip) => page.openRowMenu(oidHex, chip)
                            onChoiceMenuOpenRequested: (oidHex, atRow, chip) => page.openChoiceMenu(oidHex, atRow, chip)
                            onRowSwitchRequested: (oidHex, chip) => page.rowDoubleClicked(oidHex, chip)
                            // The same door the WORKTREES row opens: the tab stands in that worktree, where its
                            // changes can be staged.
                            onCarriedOpenRequested: path => page.openRepositoryPathRequested(path)
                            onCarriedMenuRequested: (path, name) => page.openCarriedMenu(path, name)
                            onRowRenameRequested: (oidHex, chip) => page.startRename(oidHex, chip)
                            onRenameSubmitted: (kind, id, oid, name) => page.renameRow(kind, id, oid, name)
                            namingRefused: graphNameVerdict.refused
                            namingRefusedWhy: graphNameVerdict.refusedWhy
                            namingRefusedMark: graphNameVerdict.refusedMark
                            namingRefusedMarks: graphNameVerdict.refusedMarks
                            onChipExpandRequested: (oidHex, atRow, records, anchor) =>
                                rowHost.pointChip(oidHex, atRow, records, anchor)
                            onChipCollapseRequested: (oidHex, anchor) => rowHost.unpointChip(oidHex, anchor)
                            // One card between the graph's rows and the right pane's under a choice (デザイン規約
                            // §複数のコミットを選ぶ), told apart by area.
                            onRowHoverRequested: (row, inside) => rowHost.pointRow(row, inside, "graph")
                            onRowRewritten: row => rowHost.rowGone(row)
                            onChipChanged: (oidHex, anchor) => rowHost.chipChanged(oidHex, anchor)
                            onCreateBranchRequested: (oidHex, name) => repoTab.createBranch(name, oidHex, true)
                            // Nothing moves: a tag is left on the commit and the tree stays where it is.
                            onCreateTagRequested: (oidHex, name) => repoTab.createTag(name, oidHex)
                            onCreateWorktreeRequested: (oidHex, name) => page.worktreeFromBox(oidHex, name)
                            onOpenRepositoryRequested: page.openRepositoryPicker()
                            onAskConfirmed: page.answerRowAsk()
                            onAskCancelled: page.stopRowAsk()
                            onAskHandingOver: height => page.askHandsOver(height)
                            onAskAnsweringChanged: if (!graphPane.askAnswering && page.noticeWaits) page.raiseNotice()
                        }

                        DiffPane {
                            id: diffPane
                            keepBuilt: page.keepBuilt
                            diffModel: diffModel
                            fromWorktree: page.diffFromWorktree
                            staged: page.diffStaged
                            conflicted: page.diffKind === "conflicts"
                            conflictChange: page.diffChange
                            // Read where the file is, and staged only where this window is the one holding it.
                            writable: page.wipWritable
                            worktreeName: page.carriedName
                            worktreeHome: page.carriedHome
                            // The two swap over during a rebase; the model is where that is already answered. What
                            // *this* window is in the middle of, so a worktree's conflicted file falls back to git's
                            // own two words the way its rows do (`WipBucketPane`).
                            sideOurs: page.wipWritable ? worktree.sideOurs : ""
                            sideTheirs: page.wipWritable ? worktree.sideTheirs : ""
                            sideOursCurrent: page.wipWritable && worktree.sideOursCurrent
                            sideTheirsCurrent: page.wipWritable && worktree.sideTheirsCurrent
                            sideColorOurs: page.sideColorOurs
                            sideColorTheirs: page.sideColorTheirs
                            busy: !page.diffPressable
                            discardUnrecorded: GitFacts.discardUnrecorded(worktree.unborn, worktree.notCopied,
                                                                          [page.diffPath])
                            menuStanding: page.menuStanding
                            onCloseRequested: page.closeDiffToGraph()
                            onCopyRequested: text => clipboard.copy(text)
                            onCodeMenuRequested: page.openCodeMenu()
                            onLinkAsked: href => page.tipLinkAsked(href)
                            onDiscardHunkRequested: hunk => page.discardHunkNow(hunk)
                            onStageFileRequested: {
                                if (page.diffStaged)
                                    repoTab.unstagePath(page.diffPath)
                                else
                                    repoTab.stagePath(page.diffPath)
                            }
                            onStageSelectionRequested: (hunk, line) => page.stageSelection(hunk, line)
                            onSplitChosen: split => page.setDiffSplit(split)
                        }

                        // The plan's face is built when it takes the seat and taken down with it
                        // (rules-refs/app-ui.md — the `WindowDialogSeat` line); all it shows is the model's. The seat
                        // stands in the stack either way, so the index above still names it.
                        Loader {
                            id: planSeat
                            active: page.planShown
                            sourceComponent: RebasePlanPane {
                                planModel: planModel
                                selectedOid: page.selectedOid
                                // The pane has nothing of its own yet — or the run is holding the face it had then.
                                // A read that *replaces* a standing plan is not this: the pane keeps those rows until
                                // the newer answer lands (`RebasePlanModel::open`).
                                waiting: (planModel.loading && !planModel.active) || page.planLoadHeld
                                discards: page.planDiscards
                                onRowPicked: oidHex => page.activateRow(oidHex)
                            }
                        }
                    }
                }

                // Right side: worktree ⇄ commit details
                Rectangle {
                    id: rightPane
                    // Where it starts; `applySavedLayout` assigns over this with the reader's saved width
                    // (`AppBackend.startDetailsWidth`), which a narrow window squeezes without overwriting.
                    SplitView.preferredWidth: 400
                    SplitView.minimumWidth: pageLayout.rightFloorWidth
                    color: Theme.bgSurface

                    GitVersionCorner {
                        id: gitCorner
                        // The run button takes the pane's foot while a plan stands, and that foot is this corner's
                        // seat (デザイン規約 §コミットメッセージの 2 つの枠「ペインの底は常に埋まる」). Said through the
                        // corner's own property — a `visible` here replaces the one it draws itself by.
                        offered: !page.planActive
                        // One of the three panes is on screen, and each measures its own file list. **The carried
                        // worktree's is asked for its own room**: the tree's pane never lends the corner (its foot is
                        // the commit button's), so read through it a reader of another worktree would get no version
                        // at all.
                        roomAcross: rightPane.width
                        roomLeft: !page.wipShown ? detailsPane.bottomRoom
                                : page.wipWritable ? wipPane.bottomRoom
                                : carriedSeat.item !== null ? carriedSeat.item.bottomRoom : 0
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.rightMargin: Theme.spaceXs
                        anchors.bottomMargin: Theme.spaceXs
                        // Declared before the panes it hangs over, so z holds it in front of whatever they draw here.
                        z: 1
                    }

                    // Another worktree's changes (`CarriedPane`), built the first time it stands and kept from then on
                    // (rules-refs/app-ui.md — the `WindowDialogSeat` line): most pages read no other worktree, and one
                    // read again finds its list where it left it.
                    Loader {
                        id: carriedSeat
                        anchors.fill: parent
                        visible: page.carriedShown
                        active: false
                        sourceComponent: CarriedPane {
                            worktreeName: page.carriedName
                            worktreeHome: page.carriedHome
                            files: carriedModel
                            readBucket: page.diffFromWorktree ? page.diffKind : ""
                            readPath: page.diffFromWorktree ? page.diffPath : ""
                            onTreeViewChosen: tree => page.setWipTreeView(tree)
                            onFileActivated: (bucket, path, origPath) => page.toggleDiff(bucket, path, origPath)
                            onFileWalked: (bucket, path, origPath) => page.openDiff(bucket, path, origPath)
                        }
                    }

                    WipPane {
                        id: wipPane
                        anchors.fill: parent
                        visible: page.wipShown && page.wipWritable
                        repoTab: repoTab
                        worktree: worktree
                        unstagedModel: unstagedModel
                        conflictsModel: conflictsModel
                        stagedModel: stagedModel
                        onTreeViewChosen: tree => page.setWipTreeView(tree)
                        amending: page.amending
                        headPublished: page.headPublished
                        menuStanding: page.menuStanding
                        keepBuilt: page.keepBuilt
                        onAmendToggled: on => page.amendToggled(on)
                        onCommitClicked: page.commitNow()
                        readBucket: page.diffFromWorktree ? page.diffKind : ""
                        readPath: page.diffFromWorktree ? page.diffPath : ""
                        onFileActivated: (bucket, path, origPath) => page.toggleDiff(bucket, path, origPath)
                        onFileWalked: (bucket, path, origPath) => page.openDiff(bucket, path, origPath)
                        onFileMenuRequested: (bucket, path) => page.openFileMenu(bucket, path)
                    }

                    DetailsPane {
                        id: detailsPane
                        anchors.fill: parent
                        // The run button's seat is carved off the pane's foot while a plan stands — the pane's own
                        // press-things-here edge moves up with it (デザイン規約 §コミットメッセージの 2 つの枠「押す物はペインの底」).
                        anchors.bottomMargin: planRunSeat.visible ? planRunSeat.height : 0
                        visible: !page.wipShown
                        details: detailsModel
                        chosenCommits: page.chosenRecords
                        cardKey: rowHost.cardKey
                        onRowHoverRequested: (row, inside) => rowHost.pointRow(row, inside, "chosen")
                        onRowRewritten: row => rowHost.rowGone(row)
                        onCommitDropRequested: oidHex => page.dropFromChoice(oidHex)
                        onCommitShowRequested: oidHex => page.showChosen(oidHex)
                        stashRef: page.selectedStashRef
                        menuStanding: page.menuStanding
                        keepBuilt: page.keepBuilt
                        // The one commit an amend reaches, and the line the box gives when this is not it
                        // (`page.messageEdit`; the rule is core's). `not-head` is silent — the reader can see the row
                        // is not the top. While a plan row carries `reword` the same boxes are its plan input (one
                        // place in the app to type a message); a plain amend is held for the plan's whole stay — a
                        // queued rewrite of the history the plan is composed on, which the freeze exists to stop.
                        editable: (page.messageEdit === "amend" && !page.planActive) || page.planReword
                        intoPlan: page.planReword
                        planDraftOid: page.planActive ? planModel.selectedOid : ""
                        planDraftSubject: planModel.selectedMsgSubject
                        planDraftBody: planModel.selectedMsgBody
                        editBlocked: page.planActive && !page.planReword && page.messageEdit !== ""
                            ? qsTr("Mark the row reword to retype its message")
                            : page.messageEdit === "stash"
                              ? qsTr("Rename it in the list on the left")
                              : page.messageEdit === "standing"
                                ? Words.finishStoppedFirst
                                : ""
                        busy: repoTab.busyCount > 0
                        published: page.selectedPublished
                        // Only HEAD's own commit is saved, and only over something staged can that empty it.
                        amendEmpties: page.messageEdit === "amend" && worktree.stagedCount > 0
                                      && repoTab.amendEmpties
                        amendWaits: page.messageEdit === "amend" && repoTab.stagingUnseen
                        signatureKind: page.selectedSignatureKind
                        signatureCode: page.selectedSignatureCode
                        signatureSigner: page.selectedSignatureSigner
                        // Whom a rewrite would be attributed to: git keeps the author and puts the reader in as
                        // committer, so the save button wears the reader's face.
                        committerFace: repoTab.authorAvatar
                        committerFaceUrl: repoTab.authorAvatarUrl
                        signsCommits: repoTab.signsCommits
                        signingTip: repoTab.signingFormat === "ssh"
                                    ? qsTr("Signed with your ssh key")
                                    : repoTab.signingFormat === "x509"
                                      ? qsTr("Signed with your x509 certificate")
                                      : qsTr("Signed with your gpg key")
                        readPath: page.diffKind === "commit" ? page.diffPath : ""
                        onMessageSubmitted: (oidHex, subject, body) => {
                            if (page.planReword) {
                                planModel.setMessage(planModel.selectedRow, subject, body)
                                // Taken synchronously — the typed text is the resting text now
                                // (the write path hears it from `writeReworded`).
                                detailsPane.noteMessageSaved()
                            } else if (!page.planActive) {
                                page.saveMessage(oidHex, subject, body)
                            }
                        }
                        onFileActivated: (path, origPath) => page.toggleDiff("commit", path, origPath)
                        onFileWalked: (path, origPath) => page.openDiff("commit", path, origPath)
                        onFileMenuRequested: (path, origPath, change) =>
                            page.openCommitFileMenu(path, origPath, change)
                        onParentClicked: oidHex => page.jumpToRef(oidHex)
                        // The badge's press goes to the window, which owns the settings card.
                        onAvatarEditRequested: (name, email) => page.avatarSettingsRequested(name, email)
                        onCopyRequested: text => clipboard.copy(text)
                        onApplyStashRequested: selector => repoTab.applyStash(selector)
                        onPopStashRequested: selector => page.popStash(selector)
                    }

                    // Built the first time a plan stands (`onPlanActiveChanged`) and kept from then on
                    // (rules-refs/app-ui.md — the `WindowDialogSeat` line): a held run's press lands after the plan it
                    // ran has gone (`RebasePlanRunBar.runTicket`), so the bar outlives each plan.
                    Loader {
                        id: planRunSeat
                        // The plan holds the details face up (`onPlanActiveChanged`); the WIP guard is the belt —
                        // were that face ever up, this bar would sit over the commit button.
                        visible: page.planActive && !page.wipShown
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.leftMargin: Theme.spaceMd
                        anchors.rightMargin: Theme.spaceMd
                        active: false
                        sourceComponent: RebasePlanRunBar {
                            plan: planModel
                            pushedCount: page.planPushed
                            tipHeldElsewhere: worktree.headReachedElsewhere
                            busy: repoTab.busyCount > 0
                        }
                    }
                }
            }

            // ---- command log ------------------------------------------
            // **Built when raised, taken down when shut** (rules-refs/app-ui.md — the `WindowDialogSeat` line): the
            // rows and the selection are the model's, and the panel comes up on its newest row (`CommandsPane.cameUp`).
            // The seat is what stands in the split, so the split's own properties are its.
            Loader {
                id: commandsSeat
                visible: page.commandsOpen
                active: page.commandsOpen
                SplitView.preferredHeight: 280
                SplitView.minimumHeight: pageLayout.commandsHeldHeight
                sourceComponent: CommandsPane {
                    curPage: page
                    commandsModel: commandsModel
                    errorText: repoTab.lastError
                    errorWhy: repoTab.lastErrorWhy
                    // Read off the page: the press that raises the mark often builds this seat, and a panel told
                    // afterwards would come up dark and light a frame later.
                    attention: page.commandsAttention
                    onCloseRequested: page.shutCommands()
                    onErrorCleared: repoTab.clearLastError()
                    onCopyRequested: text => clipboard.copy(text)
                }
            }
        }
    }

    // A command the user asked for failed. **The panel is not raised here, nor its owner decided** (デザイン規約
    // §git が言ったことを読む場所「コマンド 1 本の終了コードでは下さない」): the write's answer does both —
    // `tellRefusal`, `absorbLeftoverAnswer`, `onFetchFirstFailed`. **Raising it here as well races that answer**: the
    // two travel separate queues (`hub::Feeds`) in no fixed order, and either order leaves the fetch's panel with the
    // wrong owner, so a recovered fetch never takes it down.
    //
    // Only the mark is this row's own: one a report already answered goes quiet (`answeredFailures`).
    Connections {
        target: commandsModel
        function onFailure() {
            if (page.answeredFailures > 0) {
                page.answeredFailures--
                commandsModel.noteAnswered()
            }
        }
    }

    // The split bars' refusal lives in the watcher; the source chain is the page's (`refusalSource`).
    SplitBarWatch {
        id: splitWatch
        refusalSource: page.refusalSource
        graphPane: graphPane
    }

    /// Bars announce themselves once, at their own creation (`SplitHandleBar.Component.onCompleted`) — before the
    /// watcher above exists, so the page holds the early ones until its `Component.onCompleted` hands them over.
    property var earlyBars: []
    function holdSplitBar(bar, held) {
        if (!splitWatch) {
            page.earlyBars.push(bar)
            return
        }
        splitWatch.holdSplitBar(bar, held)
        pageLayout.holdSplitBar(bar, held)
    }

    /// Automation: the drag past whichever boundary `which` names, and whether that boundary is still drawn
    /// (`_PGG_AUTO_ACT=divider-refuse`; AutoActDriver calls through the page).
    function dragDividerPast(which) {
        splitWatch.dragPast(which)
    }
    function refusalLineShown(which) {
        return splitWatch.lineShown(which)
    }

    /// Whichever boundary is refusing, or null. One pointer, so the order only decides which answers in the frame where
    /// two could — and two cannot, since a hand is on one boundary at a time.
    readonly property var refusalSource:
        graphPane.refused ? graphPane.refusedAt
        : detailsPane.descRefuses ? detailsPane.descPoint
        : wipPane.descRefuses ? wipPane.descPoint
        : splitWatch.refuses ? splitWatch.at
        : null

    /// What is drawn — the one badge's own `shown` (`RefusalBadge`, on why not its `visible`).
    readonly property bool refusalShown: splitWatch.shown

    function jumpToRef(oidHex) {
        // The history moved, as in `activateRow` (`Hand.acted`).
        Hand.acted()
        // Under a plan the selection has two halves meant to be one commit — this page's and the model's row — so a
        // commit the plan holds is taken as the row click takes it, model first, and anything past the base is refused
        // in silence. Moving this page's selection alone would part the row's highlight (`selectedOid`) from the
        // model's row, and `planReword`'s routing and the reword's landing would be about another commit. Headless:
        // `plan-details-held`.
        if (page.planShown) {
            const planRow = page.planActive ? planModel.rowOf(oidHex) : -1
            if (planRow < 0)
                return
            planModel.selectRow(planRow)
            page.activateRow(oidHex)
            return
        }
        // A commit outside the graph's window has no row to bring into view; its details are read all the same.
        const row = graphModel.rowOf(oidHex)
        if (row >= 0)
            graphPane.jumpToRow(row)
        // A jump is one commit, whatever was being held (デザイン規約 §複数のコミットを選ぶ).
        page.chooseOnly(oidHex)
        page.rewordRow = -1
        // A jump off the uncommitted changes puts a standing question down; one from a commit to another leaves it up
        // (a row's click puts it down either way — `activateRow`).
        if (page.wipShown)
            page.stopRowAsk()
        // The row travels with the oid: `followVanishedCommit` reads `selectedRow`, and a stale one lands on whatever
        // now stands at the previous selection's row. A worktree being read is let go of here as on every road to a
        // commit (`readRow`).
        page.selectedRow = row
        page.readRow(GitFacts.commitKey(oidHex))
        page.selectedStashRef = graphModel.stashRefOf(oidHex)
        detailsModel.request(oidHex)
        page.askSignature(oidHex)
        page.closeDiff()
    }
}
