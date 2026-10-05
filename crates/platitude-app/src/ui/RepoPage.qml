pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// **A focus scope**, so Escape stays answerable: a pane swapped off the screen lets the keyboard go
// (規約 §矢印で履歴を辿る), and in a plain item focus then falls out to the window's content item, off this page's key
// path — Escape goes quiet until the next press (`tests/qml/tst_escape.qml`).
FocusScope {
    id: page
    required property int index
    required property int tab_id
    readonly property bool blank: tab_id < 0

    /// The blank page's "Open repository…" button (folder picker).
    signal openRepositoryPicker()
    /// A working copy asked for by path (a WORKTREES row, `openHolder`): a copy of this repository stands this tab in
    /// it (デザイン規約 §タブの所作「同じリポジトリのタブは 1 枚」).
    signal openRepositoryPathRequested(string path)
    /// PGG_AUTO_ACT=settings wants the window's settings dialog open for the screenshot.
    signal settingsDialogRequested()
    /// A conflicted file has nowhere to be opened: the git settings screen is where the merge editor is named.
    signal gitSettingsRequested()
    /// The settings card, opened from an avatar and carrying whom it was opened on.
    signal avatarSettingsRequested(string name, string email)
    /// PGG_AUTO_PERF completion after every requested measurement output.
    signal perfFinished()
    /// The failed-open screen's "Close tab" button.
    signal closeTabRequested()
    /// The working copy this tab is standing in would not open, and the repository has its own one to stand in
    /// instead (`RepoTab.standHomeAsked`). The strip is what moves the tab.
    signal standHomeRequested()

    property string selectedOid: ""

    property bool sidebarCollapsed: false
    /// The fold the diff put on: closing the diff takes back only that; a fold by hand stays.
    property bool foldedByDiff: false
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
    /// The same for the plan (規約 §フル interactive rebase). **The list comes back when the plan goes**, run included —
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
    /// Automation (`AutoActNavVerbs`, PGG_AUTO_ACT=doors-held): a replay this run really started, held until its
    /// picture is taken (rules-refs/app-ui.md「一瞬だけ立つ状態は signal で観測して latch する」).
    property bool autoReplayHeld: false
    /// **The left pane is out two different ways** (デザイン規約 §フル interactive rebase). This one is the plan's: the
    /// whole pane but the `>_` band goes to the disabled step, from the press (`SidebarPane.frozen`).
    readonly property bool sidebarFrozen: page.planShown
    /// The other, from where the plan ends: a replay is running (`RepoTab.replaying`) or its run is out and not
    /// started. Nothing freezes; each door is held and says this line until git answers (`SidebarPane.doorsHeld` /
    /// `AppMenu.heldReason`) — one line, so the pane and the menus cannot disagree.
    readonly property string doorsHeldWhy:
        page.replayRunning || page.planRunOut ? Words.otherCommandRunning : ""
    /// A press landed away from whatever held the keyboard (Main's `FocusRelease`), at `scenePos` — `null` for a press
    /// with no place of its own (the headless run's door). The left menu's name box goes unasked
    /// (デザイン規約 §左メニューの所作), and so does whatever over the graph stands on an empty box (§コミットを探す).
    function releasePressedAway(scenePos) {
        sidebarPane.stopEdit()
        graphPane.dropEmptyBoxes(scenePos)
    }
    /// A press landed anywhere in the window (Main's `FocusRelease`): the first press after arriving takes down the
    /// mark that sent the reader here (デザイン規約 §hover のツールチップ).
    function notePress() {
        detailsPane.dropAttention()
        // And the commands mark (`raiseCommands`). A link answers on the release, after this press, so the mark it
        // raises stands.
        page.commandsAttention = false
    }
    /// And Escape, which every standing thing in this window answers (デザイン規約 §hover のツールチップ).
    /// **A key handler, not a shortcut**: this is reached only when no bar, popup or box took the key first, and two
    /// enabled `StandardKey.Cancel` shortcuts fire neither (`noticeBar.yieldsEscape`, `tests/qml/tst_escape.qml`) —
    /// anything else that wants Escape joins as a key handler. **Topmost first, the marks last**; accepted only when
    /// something was taken.
    function escapePressed() {
        // First: a middle-click autoscroll holds the pointer and the view, on whichever surface (`MiddleHand`).
        if (MiddleHand.stop())
            return true
        // The plan's own way out — **only while that door is a press**. With something composed to lose it is a
        // hold (規約 §長押し — `RebasePlanPane.discards`), and one key down must not throw away what the hold guards.
        if (page.planShown) {
            if (page.planDiscards)
                return false
            planModel.cancelPlan()
            return true
        }
        // The same press as the diff's `✕`, arrows back to the graph (`closeDiffToGraph`).
        if (page.diffShown) {
            page.closeDiffToGraph()
            return true
        }
        // The log (デザイン規約 §git が言ったことを読む場所) — unlike the left menu, a failed command raises it on
        // its own. By `shutCommands`: a panel the reader put away is not one the next landing fetch may take over.
        if (page.commandsOpen) {
            page.shutCommands()
            return true
        }
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
        page.foldedByDiff = false
        // Every way the list comes back (the band's block) arrives here, so "unfolding closes the diff" has no
        // exceptions elsewhere; a rail cell's peek and a rename box (`SidebarRowGestures.startEdit`) keep the fold.
        if (!collapse && page.diffShown)
            page.closeDiff()
    }

    property bool wipShown: false
    // The ask bar goes off screen with the pane, and nothing off screen may be answered.
    onWipShownChanged: if (!page.wipShown) page.stopRowAsk()
    // Selected stash row's reflog selector ("" = not a stash).
    property string selectedStashRef: ""
    function showWip() {
        // This window's tree: every road here is about it, and `openWipFor` alone puts a copy back afterwards.
        page.dropCarried()
        page.wipShown = true
        page.pendingHeadSelect = false
        page.pendingHeadAsked = false
        page.pendingWipSelect = false
        page.selectedOid = ""
        page.selectedStashRef = ""
        // The working tree's row is not a commit, so nothing is held; the rows light the current one
        // (`GraphRowDelegate.selected`).
        page.chooseOnly("")
        page.closeDiff()
    }

    // ---- which working copy the WIP pane is about ---------------------
    //
    // **Every uncommitted row carries git's all-zero id**, ours and other copies' alike
    // (P3-確認事項 §別 worktree の未コミット行), so the copy is named by its path. **Empty means this window's own
    // tree, and only then can anything here be written.**
    property string carriedPath: ""
    property string carriedName: ""
    /// Whether the pane may write what it shows — the one property every write control reads (デザイン規約 §無効).
    readonly property bool wipWritable: page.carriedPath === ""
    /// The list the right pane shows: our unstaged run, or another copy's one folded list (`CarriedPane`). Every
    /// question about the files on screen goes here; both answer alike, being built from one whole status
    /// (`models::nav::Source::files`).
    readonly property var wipUnstaged: page.wipWritable ? worktreeModel : carriedModel
    /// The diff in two columns or one — the band's toggle, and the saved choice as the page opens
    /// (`PageLayout.applySavedLayout`, デザイン規約 §diff を 2 列で読む).
    function setDiffSplit(split) {
        diffModel.setSplit(split)
    }
    /// Tree or flat paths, for our three lists and the copy's one at once: the choice is the pane's, so stepping onto
    /// another copy and back keeps it, and one answer is saved at close.
    function setWipTreeView(tree) {
        conflictsModel.setTreeView(tree)
        worktreeModel.setTreeView(tree)
        stagedModel.setTreeView(tree)
        carriedModel.setTreeView(tree)
    }
    /// Opens the WIP pane on whichever working copy row `row` is — this window's own, or another copy's.
    function openWipFor(row) {
        const at = row >= 0 ? graphModel.carriedPath(row) : ""
        // Already the copy being read: clearing the pane would close the file opened from it under the clicking hand.
        if (at !== "" && at === page.carriedPath) {
            page.wipShown = true
            return
        }
        // The list's light names a file of the copy being left, which the arriving copy may also have. Our own row
        // to itself is not a step.
        if (at !== page.carriedPath)
            wipPane.clearChoice()
        page.showWip()
        if (at !== "")
            page.standOnCopy(at, graphModel.carriedName(row))
    }
    /// Points the pane at one copy and reads its files; the copies' tick keeps them current while it stands
    /// (`pollCarried`).
    function standOnCopy(at, name) {
        page.carriedPath = at
        page.carriedName = name
        repoTab.readCarriedStatus(at, name)
    }
    /// The pane is about this window's own tree again — every road to a commit and to our own row comes through here.
    function dropCarried() {
        page.carriedPath = ""
        page.carriedName = ""
    }
    /// Follows the copy being read across a graph pass, and lets go where the copy has gone clean and taken its row.
    /// **Addressed by the copy**: rows move under a rebuild, and all of them wear the all-zero id.
    function settleCarriedAfterPass() {
        if (page.carriedPath === "")
            return
        const row = graphModel.carriedRowOf(page.carriedPath)
        if (row >= 0) {
            page.selectedRow = row
            graphPane.setCurrentRow(row)
            return
        }
        // The copy committed or stashed: **land where that copy now stands** — the row named a copy, not a branch
        // (`NavSectionModel.headOfCopy`). A listing a tick behind lands on its previous HEAD, still its own history.
        const head = worktreesModel.headOfCopy(page.carriedPath)
        const headRow = head === "" ? -1 : graphModel.rowOf(head)
        page.dropCarried()
        page.wipShown = false
        page.selectedOid = ""
        if (headRow >= 0) {
            graphPane.setCurrentRow(headRow)
            page.activateRow(graphModel.oidAt(headRow), headRow)
            return
        }
        // The copy is gone from the listing (removed, pruned, or bare): fall back to the opening default.
        page.trySelectDefault()
    }

    // ---- commit editor -------------------------------------------
    property bool amending: false
    // Whether a remote already has HEAD's commit — the amend's `already pushed`. Kept beside HEAD by the session, so a
    // HEAD that moved wears no answer about the commit it left.
    readonly property bool headPublished: workTree.headPublished
    /// Whether the tab was standing in another working copy at the last drain (`RepoTab.standing`). Every property of
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

    // A tab whose repository would not open: the screen takes the panes' seat, so the log stays reachable. Not raised
    // — the failed command is a background read, and the panel would come up empty.
    readonly property bool openFailed: !page.blank && repoTab.state === "error"
    /// Automation (`PGG_AUTO_ACT=open-fail-sweep`): that screen's hand, which its three lines are dragged over from.
    readonly property alias openFailedHand: failedScreen.pad

    property int seenHeadCommitSeq: 0
    property bool wantHeadMessage: false
    function amendToggled(on) {
        page.amending = on
        if (on) {
            page.wantHeadMessage = true
            repoTab.requestHeadCommit()
        } else {
            page.clearCommitEditor()
        }
    }
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
    /// would (デザイン規約 §進行中の操作から出る). **Once per message, only into empty boxes**: typed text is the one
    /// thing here that cannot be read back off disk. **Taken back out when the merge goes**, but only while untouched
    /// (the pair below).
    property string seenOpMessage: ""
    property string opFilledSubject: ""
    property string opFilledBody: ""
    function absorbOpMessage() {
        const subject = workTree.opMerging ? workTree.opSubject : ""
        const body = workTree.opMerging ? workTree.opBody : ""
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
    /// place (デザイン規約 §変更を退避する).
    function popStash(selector) {
        page.pendingPopLabel = GitFacts.stashLabel(stashesModel.nameOfFull(selector))
        page.pendingPopId = repoTab.popStash(selector)
        page.selectedStashRef = ""
    }
    /// The answer to that pop, found by id among this notify's (`RepoTab.writeAnswerIndex`); a notify without it
    /// leaves the wait standing. Read ahead of `absorbLeftoverAnswer`, so a refusal reaches here too. The words go in
    /// only where the pop landed, and **only into empty boxes** (`absorbOpMessage`) — both: a description with no
    /// summary is not an empty editor.
    function absorbPopLabel() {
        if (page.pendingPopId === 0)
            return
        const answer = repoTab.writeAnswerIndex(page.pendingPopId)
        if (answer < 0)
            return
        const carried = page.pendingPopLabel
        const landed = !repoTab.writeAnswerFailed(answer)
        page.pendingPopLabel = ""
        page.pendingPopId = 0
        if (landed && carried !== "" && wipPane.subjectText === "" && wipPane.bodyText === "")
            wipPane.setMessage(carried, "")
    }

    // An amend runs straight even when HEAD is pushed (デザイン規約 §長押し「ローカルの履歴書き換えはクリック」). The
    // slot records the queue's id itself (`ops::Press`).
    function commitNow() {
        // Another copy's changes show in `CarriedPane`, which has no commit button; this is the belt under it.
        if (!page.wipWritable)
            return
        repoTab.commit(wipPane.outgoingSubject, wipPane.outgoingBody, page.amending, wipPane.resetAuthor)
    }
    /// The answer to that commit — **its own answer** among those this notify carried (`RepoTab.commitAnswer`, -1
    /// for none): one drain can bring a fetch's too, and read off a shared property a landed commit would keep its
    /// message and a hook's refusal would go untold. Everything the page does with a commit is here;
    /// `absorbLeftoverAnswer` never reaches it.
    function absorbCommitAnswer() {
        const answer = repoTab.commitAnswer
        if (answer < 0)
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
    /// The answer to the stash press that emptied the tree — its own answer (`RepoTab.stashAnswer`, -1 for none). The
    /// emptied tree is a second wait (`leaveWipWhenDone`); this half is over as soon as git speaks.
    function absorbStashAnswer() {
        const answer = repoTab.stashAnswer
        if (answer < 0)
            return
        if (repoTab.writeAnswerFailed(answer)) {
            page.tellRefusal(answer)
            return
        }
        page.readDiffForAnswer()
    }
    /// The answer to a working copy a menu asked for — its own answer (`RepoTab.copyAnswer`, -1 for none). Made, the
    /// tab goes and stands in it, as the box and the row said it would: the copy is where the reader went to work
    /// (デザイン規約 §作業コピーを作る). Refused, the report comes down over the graph, the log left shut.
    function absorbCopyAnswer() {
        const answer = repoTab.copyAnswer
        if (answer < 0)
            return
        if (repoTab.writeAnswerFailed(answer)) {
            page.tellRefusal(answer)
            return
        }
        page.openRepositoryPathRequested(repoTab.copyAnswerPath)
    }
    /// The answer to the toolbar's push — its own answer (`RepoTab.pushAnswer`, -1 for none). Only a refusal is the
    /// page's; the button's mark is `PublishFlow.noteWriteAnswer`'s.
    function absorbPushAnswer() {
        const answer = repoTab.pushAnswer
        if (answer < 0 || !repoTab.writeAnswerFailed(answer))
            return
        page.tellRefusal(answer)
    }
    /// The answer to a push a **ref row** sent — the `push --delete`s, the replace, the pairs that push after a local
    /// step (`RepoTab.refPushAnswer`, -1 for none). **Its own answer**: read by word, a fetch in the same drain would
    /// swallow the refusal. The rows a delete took come back through the delete's own owner; this carries only why.
    function absorbRefPushAnswer() {
        const answer = repoTab.refPushAnswer
        if (answer < 0 || !repoTab.writeAnswerFailed(answer))
            return
        page.tellRefusal(answer)
    }
    /// A refusal somebody outside made (a hook, a remote newer than our picture) comes down as a report
    /// (デザイン規約 §答えの要らない報せ). Returns whether there was one; otherwise each caller does its own thing.
    /// Read off the answer: one drain can carry several reports (`RepoTab.writeAnswerReportKind`).
    function reportRefusal(answer) {
        const kind = repoTab.writeAnswerReportKind(answer)
        if (kind === "")
            return false
        const remote = repoTab.writeAnswerReportRemote(answer)
        const name = repoTab.writeAnswerReportName(answer)
        page.showReport(kind, remote, name, repoTab.writeAnswerReportReason(answer))
        // The `>_` mark goes quiet with the report (`answeredFailures`).
        if (!commandsModel.failed)
            page.answeredFailures++
        commandsModel.noteAnswered()
        page.writeReported(kind, remote, name)
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
    // Uncommitted changes come along (デザイン規約 §未コミット変更がある状態での移動). `moveKind` is "branch" /
    // "remote" / "force" (a local branch moved to `moveStart` before landing on it); every one lands on a branch
    // (デザイン規約 §ブランチ・コミットへの移動).
    property string moveKind: ""
    property string moveTarget: ""
    property string moveLocal: ""
    property string moveStart: ""

    // ---- the same chip, pressed again ------------------------------
    /// The branch a move already sent is putting HEAD on; `""` while none is on its way. Meanwhile `switchToRef` sends
    /// nothing: the screen takes a move in long after the write answered (refs are published after the answer,
    /// `session::write`), so a second press on a remote branch would send the same `switch --create` and be refused.
    /// **Ends at the move's own landing** — HEAD on the branch and the branch listed (`offers::move_landed`; not a
    /// seq, which moves for reads nobody asked for) — or at a refusal or the move's question (`absorbMoveAsk`).
    property string moveLanding: ""
    function absorbMoveLanding() {
        if (page.moveLanding !== ""
                && GitFacts.moveLanded(page.moveLanding, workTree.branch,
                                       branchesModel.oidOfName(page.moveLanding)))
            page.moveLanding = ""
    }

    function switchTo(kind, target, local, start, leaving) {
        page.moveKind = kind
        page.moveTarget = target
        page.moveLocal = local
        page.moveStart = start === undefined ? "" : start
        if (page.standsInTheWay(leaving)) {
            page.askLeaveOperation(function () { page.runSwitch(true) })
            return
        }
        page.runSwitch(leaving === true)
    }
    function runSwitch(leaving) {
        // Marked where the move goes out: the question a blocked move raises comes back through here, and a press
        // held behind a bar has sent nothing yet.
        page.moveLanding = page.moveLocal
        if (page.moveKind === "branch")
            repoTab.checkoutBranch(page.moveTarget, leaving === true)
        else if (page.moveKind === "remote")
            repoTab.checkoutRemote(page.moveTarget, page.moveLocal, leaving === true)
        else if (page.moveKind === "force")
            repoTab.checkoutForceCreate(page.moveTarget, page.moveStart, leaving === true)
    }

    // ---- moving out of what stands in a move's way --------------------
    // What blocks a move (an operation standing, unmerged paths) is core's rule (offers::moves_blocked), and clearing
    // it is one question for both (デザイン規約 §進行中の操作から出る). **Asked before anything is sent** — git's
    // refusal would put a red log line where a question belongs.
    function standsInTheWay(leaving) {
        return leaving !== true && workTree.movesBlocked
    }
    readonly property string leaveHeading:
        workTree.leaveUndoes ? qsTr("Undo it and go?") : qsTr("Put it aside and go?")
    readonly property string leaveDetail:
        workTree.leaveUndoes ? qsTr("Nothing it did since it started is kept.")
        // Nothing standing: only the files are in the way.
        : workTree.opCommand === ""
        ? qsTr("The files waiting on a decision go to the stash, markers and all.")
        //: %1 is the standing operation in git's own spelling, e.g. cherry-pick.
        : qsTr("The %1 stops; its files go to the stash, markers and all.").arg(workTree.opCommand)
    function askLeaveOperation(retry) {
        // Marked on HEAD's row — a rebase runs detached, so that is the only name the tree has for where it is
        // (デザイン規約 §立っている質問は 1 か所で聞く). Whether leaving undoes work in hand (`danger` and a hold —
        // §状態, §長押し) and the command on the question's line are core's (offers::leaving_undoes / leave_code).
        page.startRowAsk(workTree.headOid, page.leaveHeading, page.leaveDetail,
                         workTree.leaveUndoes, "", retry, workTree.leaveUndoes, "", null,
                         workTree.leaveCode)
    }

    // ---- what a chip leads to --------------------------------------
    // `chip` is the chip as it is drawn (`encode::Chip`); null is a row that draws none.
    function activateChip(chip) {
        if (chip)
            page.switchToRef(chip.kind, chip.name)
    }
    // A branch another working copy holds is the one refusal no stash or put-down can clear (offers::SwitchAction), so
    // **the press goes to that copy** (`openRepositoryPathRequested`). Unasked — it writes nothing; the worded rows
    // name the copy first (`RefRowMenu` の `Open`, デザイン規約 §進行中の操作から出る).
    function openHolder(local) {
        const held = worktreesModel.worktreeHolding(local)
        if (held !== "")
            page.openRepositoryPathRequested(held)
    }
    /// Answers whether the press did anything — a move sent, or a question raised in front of one. `false` is a press
    /// turned away, which the headless double press reads (動詞 `switch-remote-twice`): from outside, both presses
    /// look alike.
    function switchToRef(kind, name, leaving) {
        // A working copy on no branch, named by its path (the menus' `Open`, `RefRowMenu.offerOn`): the tab goes and
        // stands in it, as its WORKTREES row's double-click does — ahead of the gate below, since it writes nothing.
        if (kind === "worktree") {
            if (worktreesModel.copyFacts(name) === undefined)
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
        // the local branch it lands on.
        const local = kind === "remote" ? remotesModel.localNameFor(name) : name
        const action = GitFacts.switchAction(kind, local, workTree.branch,
                                             worktreesModel.worktreeHolding(local),
                                             branchesModel.oidOfName(local))
        // Before the leave question: no put-down clears the holder road, so undoing a rebase first would spend the
        // undo on a move that was never possible.
        if (action === "holder") {
            page.openHolder(local)
            return true
        }
        // Ahead of the branches below: the one that lands on an existing local branch asks git what the move would
        // cost before it moves, and that read is worth nothing while an operation is standing.
        if (page.standsInTheWay(leaving)) {
            page.askLeaveOperation(function () { page.switchToRef(kind, name, true) })
            return true
        }
        if (action === "switch")
            page.switchTo("branch", name, name, "", leaving)
        else if (action === "materialize")
            page.switchTo("remote", name, local, "", leaving)
        else if (action === "move") {
            // Whether to ask is git's to answer — a branch that only fell behind loses nothing. The question comes back
            // as `moveAskSeq`, the operation still standing for its answer to undo (core asks before it aborts).
            page.moveLanding = local
            repoTab.checkoutMovingBranch(local, name, leaving === true)
        }
        // A tag or the detached marker moves nothing and raised nothing (a tag's row offers a branch at its commit
        // instead, `startNaming`).
        return action !== ""
    }

    // Landing on the remote branch moves the existing local one onto it — the one move here that can leave commits
    // unreachable.
    function askMoveBranchOnto(local, remoteRef) {
        page.startRowAsk(
            remotesModel.oidOfName(remoteRef),
            qsTr("Move %1 here?").arg(local),
            qsTr("Commits only %1 has stop being reachable.").arg(local),
            false,
            qsTr("Move"),
            function () { page.switchTo("force", local, local, remoteRef) },
            true)
        page.moveBranchAsked(local)
    }
    /// Automation: the question above was raised, about `local` — the bar carries no name a run could read back.
    signal moveBranchAsked(string local)

    // ---- the standing question --------------------------------------
    // One bar over the graph (デザイン規約 §可否・警告の出し場所). Only questions about a ref reach it: taking one named
    // thing away is a hold (デザイン規約 §長押し).
    /// Ctrl+F is quietly refused while a plan covers the graph, as a standing question refuses it
    /// (`GraphPane.startFind`).
    function startFind() {
        if (page.planShown)
            return
        graphPane.startFind()
    }

    // ---- the interactive-rebase plan ---------------------------------
    // Composing runs nothing; the run is one queued write pinned to the tip the plan opened on. The page holds what
    // could move the history from inside this window (`planShown`); a terminal or another session is answered by the
    // model putting the plan away when the tip moves (`noteHead` below).
    /// A plan stands with its rows: **the one answer** for everything that needs a draft (the right pane's boxes, the
    /// run bar) — a second spelling would disagree while `planLoadHeld` holds the opening face.
    readonly property bool planActive: planModel.active && !page.planLoadHeld
    /// The plan's face has the graph's seat: its read is out, or its rows have arrived. **Taken at the press** — the
    /// read at the root of a real history is far past a press's acknowledgement (ci/baseline/code-costs-windows-x64.md;
    /// the pane shows `RebasePlanPane.waiting`). The mode's restrictions read this too (the left menu, `push`, `stash`,
    /// Ctrl+F); only what needs a draft waits for `planActive`.
    readonly property bool planShown: planModel.active || planModel.loading
    /// Automation (`PGG_AUTO_ACT=plan-loading`): the opening face, held from the real edge
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
    /// §長押し): the plan's edits, and a reword in the right pane's boxes not yet given to the plan. **Not a
    /// half-written amend from before the plan** — closing the plan leaves it (`DetailsPane.dropDraft`). **Read off
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
            page.foldForPlan(true)
        } else {
            page.foldForPlan(false)
        }
    }
    onPlanActiveChanged: {
        if (page.planActive) {
            // Land on the plan's newest row whatever face was up — the WIP face cannot stay under an open plan. The
            // plan's *own* selection is already there: set from here it would fire the model's one `changed()` inside
            // the binding delivering it (`RebasePlanModel::take`).
            page.activateRow(planModel.expectHead)
        } else {
            // However the plan went, a `reword` left in the boxes has no row to carry it, and the commit on screen did
            // not move (`DetailsPane.syncMessage` would keep the text for the plain amend) — its own message goes back.
            detailsPane.dropDraft()
        }
    }
    // The pushed count moves with the remote-tracking refs (fetch runs under the freeze, 規約 §フル interactive rebase),
    // so it is asked again on `refsMoved` — the every-tick `refsSettled` would spawn a rev-list at the status rate.
    Connections {
        target: branchesModel
        function onRefsMoved() {
            planModel.refreshPushed()
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
        // heading and `warning` are the plan's own (規約 §答えの要らない報せ).
        function onRefusedPlan(kind) {
            page.showNotice(Words.planRefused(kind), Words.rewriteRefusedWhy(kind), "warning")
        }
        function onStalePlan() {
            page.showNotice(qsTr("The branch tip moved"),
                            qsTr("The plan was put away; nothing has run."), "warning")
        }
        function onStandingOp() {
            page.showNotice(qsTr("An operation started here"),
                            qsTr("The plan was put away; nothing has run. Finish what is in progress first."),
                            "warning")
        }
    }
    // Tip and standing operation, fed on every snapshot while a plan stands: either moving puts the plan away (a merge
    // stopped on a conflict leaves the tip where it was).
    Connections {
        target: workTree
        function onChanged() {
            if (!page.planActive)
                return
            planModel.noteOp(workTree.opText)
            if (page.planActive)
                planModel.noteHead(workTree.headOid)
        }
    }

    property var rowAskRun: null
    function startRowAsk(oidHex, label, detail, danger, acceptText, run,
                         hold = false, tip = "", form = null, code = "", refName = "") {
        // Whoever was dressing the bar lets go first, or that flow would think it still stands and its check timer
        // go on firing round trips at a remote. `startAsking`'s assignments beat a live `Binding` outright; the flow
        // raising this one turns its own back on afterwards.
        publishFlow.publishAsking = false
        upstreamFlow.asking = false
        renameCarryFlow.asking = false
        page.rowAskRun = run
        graphPane.startAsking(oidHex, label, detail, acceptText, danger, hold, tip, form, code, refName)
    }
    function stopRowAsk() {
        page.rowAskRun = null
        publishFlow.publishAsking = false
        upstreamFlow.asking = false
        renameCarryFlow.asking = false
        graphPane.stopAsking()
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
        page.askMoveBranchOnto(repoTab.moveAskLocal, repoTab.moveAskStart)
    }

    // ---- what a branch is measured against --------------------------
    UpstreamFlow {
        id: upstreamFlow
        repoTab: repoTab
        remotesModel: remotesModel
        graphPane: graphPane
        onAskRequested: (oidHex, label, accept, run, form) =>
            page.startRowAsk(oidHex, label, "", false, accept, run, false, "", form, "")
    }
    /// The branch card's row, from either menu it is carried by; the question is marked on that branch's own row.
    function startUpstreamAsk(branch, counterpart) {
        upstreamFlow.startAsk(branch, branchesModel.oidOfName(branch), counterpart)
    }

    // ---- what a name changed here does to the remote ----------------
    RenameCarryFlow {
        id: renameCarryFlow
        repoTab: repoTab
        graphPane: graphPane
        onAskRequested: (oidHex, label, accept, run, form) =>
            page.startRowAsk(oidHex, label, "", false, accept, run, false, "", form, "")
        onCarryAsked: (kind, from, to) => page.renameCarryAsked(kind, from, to)
    }
    /// Automation: the question above was raised, and the two names it is between.
    signal renameCarryAsked(string kind, string from, string to)
    /// A branch took a new name here and the remote it was measured against still carries the old one. The cut is
    /// the configured remote name where one owns the ref (a remote's own name may contain `/`), the first slash
    /// otherwise (`GitFacts.remoteOfRef`).
    function carryBranchRenameOver(remoteRef, name) {
        const remote = GitFacts.remoteOfRef(remoteRef, repoTab.remoteNames)
        if (remote === "")
            return
        const from = GitFacts.branchOfRef(remoteRef, repoTab.remoteNames)
        // Not where the remote already carries the new name: both answers would push, which fast-forwards the branch
        // over there and reports success — somebody else's branch would move. The box refuses it too; this catches
        // the way in that has no box.
        if (name === from || remotesModel.oidOfName(remote + "/" + name) !== "")
            return
        renameCarryFlow.startAsk("branch", remote, from, name, remotesModel.oidOfName(remoteRef))
    }
    /// Automation: the chooser inside it, which no injected click can open.
    function openCarryChoices() { return renameCarryFlow.openChoices() }
    function pickCarryChoice(index) { return renameCarryFlow.pickChoice(index) }

    // ---- sending the branch to its remote ---------------------------
    PublishFlow {
        id: publishFlow
        repoTab: repoTab
        workTree: workTree
        remotesModel: remotesModel
        graphPane: graphPane
        // The first push asks where the branch goes, in the one question bar.
        onAskRequested: (label, run, form, code, refName) =>
            page.startRowAsk("", label, "", false, "", run, false, "", form, code, refName)
    }
    /// What the window's toolbar reads off the page it is showing: the button lives up there, and the state machine
    /// behind it down here (`TopBar`).
    readonly property alias pushTargetLabel: publishFlow.pushTargetLabel
    readonly property alias pushState: publishFlow.pushState
    readonly property alias pushAhead: publishFlow.pushAhead
    readonly property alias pushBehind: publishFlow.pushBehind
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

    /// Where the standing right-click menu was raised — `sidebar` (the left panel's rows), `refList` (the rows of a
    /// chip's stacked list), `graph`, `files`, `diff` — or "" while none stands. What hover opened stays under a menu
    /// raised on its own rows and goes behind any other (デザイン規約 §メニュー). One binding over the seats and the
    /// doors' marks, which are written before the menu shows: a reader that also read `menuStanding` could see the
    /// menu up before its origin.
    readonly property string menuRaisedOn:
        page.menuShowing(refMenuSeat) ? (page.refMenuInSidebar ? "sidebar" : "graph")
        : page.menuShowing(remoteMenuSeat) ? "sidebar"
        : page.menuShowing(commitMenuSeat) ? (page.rowMenuOnList ? "refList" : "graph")
        : page.menuShowing(fileMenuSeat) ? "files"
        : page.menuShowing(diffMenuSeat) ? "diff" : ""
    /// One of this page's right-click menus is standing: nothing behind a menu is being hovered, so no card or tip may
    /// come out over the rows the hand is reading. A menu nobody has raised yet is not standing.
    readonly property bool menuStanding: page.menuRaisedOn !== ""
    function menuShowing(seat) {
        return seat.item !== null && seat.item.showing
    }

    /// The five menu seats stand built from the start — written only by the harness (`PageHarness`), so its verbs can
    /// read a menu's rows. The plan's face and the command log are not in this: their verbs raise them as a hand does.
    property bool keepBuilt: false

    // ---- context menu on a sidebar row ------------------------------
    // Each menu is built when first raised (rules-refs/app-ui.md — the `WindowDialogSeat` line). The door activates
    // the seat, then calls in synchronously; the seat fills the page so the menu measures the window
    // (`AppMenu.ownerItem`).
    Loader {
        id: refMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: RefRowMenu {
            // Every row moves something, so the whole card is held while the doors are, raised from a graph chip too
            // (デザイン規約 §メニュー「入口が違っても同じ操作は同じ文」).
            heldReason: page.doorsHeldWhy
            repoTab: repoTab
            workTree: workTree
            graphModel: graphModel
            branchesModel: branchesModel
            worktreesModel: worktreesModel
            tagsModel: tagsModel
            onSwitchRequested: (kind, name) => page.switchToRef(kind, name)
            onBranchHereRequested: oidHex => page.startBranchAt(oidHex)
            onTagHereRequested: oidHex => page.startTagAt(oidHex)
            onCopyHereRequested: oidHex => page.startCopyAt(oidHex)
            onCopyAddRequested: (mode, branch, start, path, name) => page.addCopy(path, mode, branch, start, name)
            onDeleteRequested: (kind, id, name, oidHex) => page.deleteRow(kind, id, name, oidHex)
            onDropStashRequested: selector => page.dropStashNow(selector)
            onUpstreamRequested: (branch, counterpart) => page.startUpstreamAsk(branch, counterpart)
            onRemoveCopyRequested: (path, name) => repoTab.removeWorktree(path, name)
        }
    }
    // What a remote itself offers. Its own menu: a remote is repository configuration, and the ref menu is about
    // refs (デザイン規約 §左メニューの所作).
    Loader {
        id: remoteMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: RemoteRowMenu {
            heldReason: page.doorsHeldWhy
            repoTab: repoTab
            onUrlRequested: name => publishFlow.startEditRemote(name)
        }
    }
    /// The one door into that menu. Says whether it opened.
    function openRemoteMenu(name) {
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
    /// The one door into that menu: the sidebar's rows (`inSidebar`) and the automation, whose calls without it take
    /// the graph's side. Says whether it opened. A working copy's row names its copy (`full` is its path), which carries its own
    /// card (`RefRowMenu.offerOn`). `aim` is a remote the reader named on its own (a TAGS row's carrier line), which
    /// the menu then acts on.
    function openRefMenu(kind, name, full, oidHex, inSidebar, aim) {
        page.refMenuInSidebar = inSidebar === true
        page.refMenuRowKind = kind
        page.refMenuRowId = full
        refMenuSeat.active = true
        return kind === "worktree" ? refMenuSeat.item.offerOn(kind, name, full, oidHex, full)
                                   : refMenuSeat.item.offerOn(kind, name, full, oidHex, undefined,
                                                              aim === undefined ? "" : aim)
    }
    /// A new branch on a commit, asked for from a menu: the name box opens where that menu was raised. The sidebar's
    /// half opens on the row that raised the menu, standing whenever that half is taken (only its door sets
    /// `refMenuInSidebar`).
    function startBranchAt(oidHex) {
        if (oidHex === "")
            return
        if (page.refMenuInSidebar)
            sidebarPane.beginBranchAt(page.refMenuRowKind, page.refMenuRowId, oidHex)
        else
            graphPane.startNaming(oidHex)
    }
    /// And a tag on that commit, through the same two doors and on the same terms.
    function startTagAt(oidHex) {
        if (oidHex === "")
            return
        if (page.refMenuInSidebar)
            sidebarPane.beginTagAt(page.refMenuRowKind, page.refMenuRowId, oidHex)
        else
            graphPane.startTagging(oidHex)
    }
    /// And a branch on that commit out in a working copy of its own (`Create worktree here…`), the same way.
    function startCopyAt(oidHex) {
        if (oidHex === "")
            return
        if (page.refMenuInSidebar)
            sidebarPane.beginCopyAt(page.refMenuRowKind, page.refMenuRowId, oidHex)
        else
            graphPane.startCopying(oidHex)
    }
    /// What either box sends once a name is typed: the branch made on `oidHex`, out in a copy where `newCopyFor`
    /// puts it. The box has already turned down every name that cannot go there.
    function copyFromBox(oidHex, name) {
        const place = worktreesModel.newCopyFor(name)
        if (place !== undefined)
            page.addCopy(place.path, "new", name, oidHex, place.name)
    }
    /// The one door to `git worktree add` (both rows and both boxes). Its answer is waited for by name: landed, the
    /// tab goes and stands in the new copy (`absorbCopyAnswer`).
    function addCopy(path, mode, branch, start, name) {
        if (path === "" || repoTab.state !== "open" || page.doorsHeldWhy !== "")
            return
        repoTab.addWorktree(path, mode, branch, start, name)
    }

    // ---- what the sidebar's rows ask for ---------------------------
    /// Unconfirmed: a name is not history. A tag and a stash are re-made under the new name by core — the only rename
    /// git has for them.
    function renameRow(kind, id, name) {
        if (kind === "branch") {
            // Which remote this branch speaks for has to be read before the rename: afterwards the row answers to the
            // new name.
            page.pendingRenameRemote = branchesModel.upstreamOf(id)
            page.pendingRenameTo = name
            repoTab.renameBranch(id, name, false)
        } else if (kind === "tag") {
            // Which remote carries this name, read before the rename for the same reason
            // (デザイン規約 §手元の改名の後のリモート).
            page.armRenameTagRemote(id, name)
            repoTab.renameTag(id, name)
        } else if (kind === "stash") {
            repoTab.renameStash(id, name)
        } else if (kind === "remote") {
            // A remote branch's box leads to the replace over there, whose question carries both names
            // (`askReplaceRemote`) — so, unlike every rename, the box comes down now: left waiting, a dismissed
            // question would leave it standing with nothing coming (デザイン規約 §答えの要らない報せ).
            page.noteRenameLanded()
            page.askReplaceRemote(id, name)
        }
    }

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
        if (tagsModel.tagSides(from) !== "both" || tagsModel.remoteTagDrift(from, remote) !== "")
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

    /// A rename's box stays open until git answers; these are the two words back to it. **Both boxes are told** and
    /// only the one that was waiting acts — the left menu's row and the graph's chip know nothing of each other
    /// (デザイン規約 §答えの要らない報せ).
    function noteRenameLanded() {
        sidebarPane.noteRenameLanded()
        graphPane.renameLanded()
    }
    function noteRenameRefused(why) {
        sidebarPane.noteRenameRefused(why)
        graphPane.renameRefused(why)
    }

    /// Replacing a branch on a remote with one under a new name, which git has no command for: core pushes the new
    /// name and deletes the old, so the question is asked first and its answer is held down — this is the one write
    /// here that another machine keeps (デザイン規約 §長押し).
    function askReplaceRemote(remoteRef, name) {
        // The same cut as `carryBranchRenameOver`.
        const remote = GitFacts.remoteOfRef(remoteRef, repoTab.remoteNames)
        if (remote === "")
            return
        const from = GitFacts.branchOfRef(remoteRef, repoTab.remoteNames)
        // And the same stop at a name already over there (`carryBranchRenameOver`).
        if (name === from || remotesModel.oidOfName(remote + "/" + name) !== "")
            return
        // The commit the bar marks, taken as it is raised: the old name's delete is leased to it
        // (`remote::replace_remote_branch`), so a remote that moved while the question stood is refused.
        const shown = remotesModel.oidOfName(remoteRef)
        page.startRowAsk(
            shown,
            //: %1 and %2 are remote branches, e.g. origin/main. The old name goes and the new one is made.
            qsTr("Replace %1 with %2?").arg(remoteRef).arg(remote + "/" + name),
            qsTr("The old branch is deleted, not moved."),
            false,
            qsTr("Replace"),
            function () { repoTab.replaceRemoteBranch(remote, from, name, shown) },
            true,
            qsTr("Hold to replace. %1 goes up first, then %2 comes off — so a push the far side turns down leaves the old name where it is. Anything it carried — an open pull request, a running check — does not follow the new name.").arg(remote + "/" + name).arg(remoteRef))
        page.replaceRemoteAsked(remoteRef, name)
    }
    /// Automation: the question above was raised, and the two names it is between.
    signal replaceRemoteAsked(string from, string to)

    /// The failures a report has already answered — armed by the report when the row it is about has not reached the
    /// log yet (`absorbWriteResult`), spent by that row's own arrival. Counted, because the refusal and the write
    /// result arrive on separate paths, in no fixed order.
    property int answeredFailures: 0
    /// The answer to the plain delete a card stayed up for — its own answer (`RepoTab.branchDeleteAnswer`, -1 for
    /// none), which no count of answers could tell from the delete before it.
    function absorbBranchDelete() {
        const answer = repoTab.branchDeleteAnswer
        if (answer < 0)
            return
        // Landed: the card goes by itself (`RefBranchMenu`). Refused by git itself: the card turns its row into the
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
    /// The chips that go with the rows are the graph model's to key (`GraphModel.setGone` / `encode::gone_keys`). A
    /// dropped stash has no chip: it is a graph row, not a name on one, and a row only leaves with the walk.
    function syncGone() {
        graphModel.setGone(page.goneBranch, page.goneRemote, page.goneTag)
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
    // No chip: a working copy is marked on the graph by its checkout, which the listing that proves it gone redraws.
    onGoneWorktreeChanged: worktreesModel.setHidden(page.goneWorktree)

    /// Automation: holds a row taken away before git answers, for its picture — asked before the press, since a demo
    /// repository answers before the grab (verify-ui スキル §壊れない動詞の実装と反復). Held by withholding
    /// `listingDrawn`, the news the owner puts the rows down on.
    property bool holdGoneRows: false

    /// Automation only: whether the last delete asked of this page went out to git or was dropped — "did the input
    /// land" for both entrances (app-ui.md §UI 自動化), as `RefBranchMenu.deleteAsked` is for the card. A dropped
    /// request leaves nothing for a write barrier to wait on.
    property bool deleteRowAsked: false
    function deleteRow(kind, id, name, oidHex) {
        page.deleteRowAsked = false
        if (kind !== "branch")
            return
        // The card stays open for git's answer, so it can be clicked twice; the second click is the same request.
        if (repoTab.busyCount > 0)
            return
        // `-d`: git's refusal becomes the question (デザイン規約 §左メニューの所作). The tab keeps which branch the answer
        // is about and the card reads it there (`RefBranchMenu`); the same slot takes the row off screen
        // (`ops_delete`).
        repoTab.deleteBranch(id, false)
        page.deleteRowAsked = true
    }
    /// Held (デザイン規約 §長押し).
    function dropStashNow(ref) {
        repoTab.dropStash(ref)
        if (page.selectedStashRef === ref)
            page.selectedStashRef = ""
    }

    // ---- context menu on a working-tree file row --------------------
    function openFileMenu(bucket, path) {
        // A right-click is a click: it walks away from a standing question, which may be about another row.
        page.stopRowAsk()
        fileMenuSeat.active = true
        fileMenuSeat.item.offer(bucket, path)
    }
    Loader {
        id: fileMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: FileRowMenu {
            repoTab: repoTab
            workTree: workTree
            wipPane: wipPane
            onMergeToolWanted: page.gitSettingsRequested()
            onCopyRequested: text => clipboard.copy(text)
        }
    }

    // ---- context menu on the diff's own text ------------------------
    /// The menu acts on the selection, settled before asking (`DiffTextSelect.askMenu`), so no row travels here.
    function openCodeMenu() {
        page.stopRowAsk()
        diffMenuSeat.active = true
        diffMenuSeat.item.offer()
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
    /// The first chip a graph row draws (what a right-click hands over — `GraphRowDelegate.menuChip`), for callers with
    /// no row in hand; null where it draws neither a ref nor a working copy's folder. Chips already taken off screen
    /// are filtered as the delegate does, so a gone name puts no card up (デザイン規約 §消す操作は先に画面から消す).
    function rowChipAt(oidHex) {
        const row = graphModel.rowOf(oidHex)
        if (row < 0)
            return null
        const shown = GitFacts.chipsShown(graphModel.labelsAt(row), graphModel.goneChips)
        if (shown.length === 0)
            return null
        return GitFacts.menuKind(shown[0].kind) === "" ? null : shown[0]
    }

    /// Whether the standing row menu was raised on a row of the chip's stacked list, which it stands on rather than
    /// over (`menuRaisedOn`).
    property bool rowMenuOnList: false
    /// The one door into that menu: graph rows, the rows of a chip's stacked list (`onList`), and automation. `chip`
    /// is the name it is aimed at; null aims it at nothing, and left out it is asked of the model (`rowChipAt`). The
    /// first level is the same commit either way — the name only picks which card comes up
    /// (デザイン規約 §グラフ行の右クリック).
    function openRowMenu(oidHex, chip, onList) {
        page.rowMenuOnList = onList === true
        const named = chip === undefined ? page.rowChipAt(oidHex) : chip
        commitMenuSeat.active = true
        const menu = commitMenuSeat.item
        const kind = named ? GitFacts.menuKind(named.kind) : ""
        // A folder's chip carries only the folder: the copy is the one of that name standing on this commit.
        const target = kind === "" ? "" : kind === "worktree" ? worktreesModel.copyAt(named.name, oidHex) : named.name
        menu.targetKind = target === "" ? "" : kind
        menu.targetName = target
        commitMenuState.openRowMenu(oidHex)
    }

    CommitMenuState {
        id: commitMenuState
        repoTab: repoTab
        workTree: workTree
        graphModel: graphModel
        worktreesModel: worktreesModel
        branchesModel: branchesModel
        tagsModel: tagsModel
        // Null until the menu is first raised; the one function that reads it is the door that raises it.
        menu: commitMenuSeat.item
    }

    Loader {
        id: commitMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: CommitRowMenu {
            // Held on the same answer as the left pane's rows: a reset or drop mid-replay is the same accident a
            // switch would be.
            heldReason: page.doorsHeldWhy
            branch: workTree.branch
            oid: commitMenuState.menuOid
            stashRef: commitMenuState.menuStashRef
            published: commitMenuState.menuPublished
            canSwitch: commitMenuState.menuCanSwitch
            switchAsks: commitMenuState.menuSwitchAsks
            heldLeaf: commitMenuState.menuHeldLeaf
            canPull: commitMenuState.menuCanPull
            pullBlocked: commitMenuState.menuPullBlocked
            canSequence: commitMenuState.menuCanSequence
            canIntegrate: commitMenuState.menuCanIntegrate
            canEditHistory: commitMenuState.menuCanEditHistory
            canMoveBranch: commitMenuState.menuCanMoveBranch
            canBranchHere: commitMenuState.menuCanBranchHere
            stashCanWrite: commitMenuState.menuStashCanWrite
            hardResetTakes: commitMenuState.menuHardResetTakes
            hardResetNotCopied: commitMenuState.menuHardResetNotCopied
            tipHeldElsewhere: commitMenuState.menuTipHeldElsewhere
            // git's answers to the branch card's delete, live while the card stands (`RefBranchMenu`).
            refusedDelete: repoTab.branchDeleteRefused
            landedDelete: repoTab.branchDeleteLanded
            checkedBranch: repoTab.branchDeleteAsked
            checkedMerged: repoTab.branchDeleteMerged
            // The same road the row's double-click takes, held on the same answers (`switchToRef`).
            onSwitchRequested: (kind, name) => page.switchToRef(kind, name)
            // Straight to the graph row: this menu is only ever raised on one.
            onBranchHereRequested: oidHex => graphPane.startNaming(oidHex)
            onCopyHereRequested: oidHex => graphPane.startCopying(oidHex)
            onCopyAddRequested: (mode, branch, start, path, name) => page.addCopy(path, mode, branch, start, name)
            onTagHereRequested: oidHex => graphPane.startTagging(oidHex)
            onSquashRequested: oidHex => page.squashCommit(oidHex)
            onDropRequested: oidHex => page.dropCommit(oidHex)
            onPlanRequested: oidHex => page.startRebasePlan(oidHex)
            onResetRequested: mode => page.moveBranchHere(mode)
            onApplyStashRequested: selector => repoTab.applyStash(selector)
            onPopStashRequested: selector => page.popStash(selector)
            onDropStashRequested: selector => page.dropStashNow(selector)
            // The branch card's own rows, answered where the ref menu's are.
            onDeleteRequested: (kind, id, name, oidHex) => page.deleteRow(kind, id, name, oidHex)
            onUpstreamRequested: (branch, counterpart) => page.startUpstreamAsk(branch, counterpart)
            onRemoveCopyRequested: (path, name) => repoTab.removeWorktree(path, name)
            // The remote-ref deletes go through `CommitMenuState`, beside the lookups their cards read.
            onCherryPickRequested: oidHex => repoTab.cherryPick(oidHex)
            onRevertRequested: oidHex => repoTab.revert(oidHex)
            onMergeRequested: ref => repoTab.merge(ref, false, false, "")
            onRebaseRequested: ref => repoTab.rebase(ref, "", true)
            onPullRequested: repoTab.pull()
            onCheckDeleteRequested: branch => repoTab.checkBranchDelete(branch)
            onForceDeleteRequested: branch => repoTab.deleteBranch(branch, true)
            onDeleteRemoteRequested: (remoteRef, expect) => commitMenuState.deleteRemoteNow(remoteRef, expect)
            onDeleteEverywhereRequested: (branch, remoteRef, forced, expect) =>
                commitMenuState.deleteEverywhereNow(branch, remoteRef, forced, expect)
            onPushTagRequested: (remote, tag, lease) => repoTab.pushTag(remote, tag, lease)
            onDeleteTagRequested: tag => repoTab.deleteTag(tag)
            onDeleteRemoteTagRequested: (remote, tag, onlyThere, expect) =>
                repoTab.deleteRemoteTag(remote, tag, onlyThere, expect)
            onDeleteTagEverywhereRequested: (tag, remote, expect) => repoTab.deleteTagEverywhere(tag, remote, expect)
            // For a menu raised on a row of the stacked list: the list stayed up under it, and the pointer now decides
            // again whether it stays.
            onDismissed: rowHost.settleRefList()
        }
    }

    ClipboardHelper {
        id: clipboard
    }

    // ---- the second click, spaced, on a graph row -------------------
    /// The row chip's name goes into a box where the chip is (デザイン規約 §グラフ行のダブルクリック / §左メニューの所作),
    /// from the row or the card its chip unfolds into. The box holds the ref's own name — a remote branch without its
    /// remote, the shape `renameRow` takes.
    function startRename(oidHex, chip) {
        if (repoTab.state !== "open" || !chip)
            return
        const kind = GitFacts.refKind(chip.kind)
        // The detached-HEAD marker names no ref.
        if (kind === "")
            return
        const id = chip.name
        // The card the chip unfolds into is standing on the column the box opens in.
        rowHost.closeRefList()
        graphPane.startRenaming(oidHex, kind, id,
                                kind === "remote" ? GitFacts.branchOfRef(id, repoTab.remoteNames) : id)
        // デザイン規約 §左メニューの所作「開く行は必ず見える所へ送る」 — the gesture's wait is long enough to scroll away in.
        const row = graphModel.rowOf(oidHex)
        if (row >= 0)
            graphPane.showRowSoon(row)
    }
    /// Whether the graph's name box can be accepted, and the line that says why not (the pane only draws it). A rename
    /// is refused, and a new copy's name once one is typed — a box opened empty would come up turning down the
    /// reader's arrival (§可否・警告の出し場所, `SidebarRowGestures`).
    readonly property string graphRenameRemote: graphPane.namingKind !== "remote" ? ""
        : GitFacts.remoteOfRef(graphPane.namingId, repoTab.remoteNames)
    readonly property bool graphNameTaken: graphPane.namingMode === "rename" && page.graphRenameRemote !== ""
        && graphPane.namingText.trim() !== ""
        && remotesModel.oidOfName(page.graphRenameRemote + "/" + graphPane.namingText.trim()) !== ""
    /// Where the copy named in the graph's box would go (`NavSectionModel.newCopyFor`), asked per keystroke; undefined
    /// for every other box.
    readonly property var graphCopyPlace: graphPane.namingMode === "worktree"
        ? worktreesModel.newCopyFor(graphPane.namingText.trim()) : undefined
    /// The folder the box's tip stands the tree mark in front of, where the refusal names one (`Words.copyNameMark`).
    readonly property string graphNameRefusedMark: Words.copyNameMark(page.graphNameRefusedWhy, page.graphCopyPlace)
    readonly property string graphNameRefusedWhy: {
        if (graphPane.namingOid === "")
            return ""
        // A new working copy is warned of what stands in its way before the press — that is the box's whole answer.
        if (graphPane.namingMode === "worktree")
            return Words.copyNameRefused(graphPane.namingText,
                                         branchesModel.oidOfName(graphPane.namingText.trim()) !== "",
                                         page.graphCopyPlace)
        if (graphPane.namingMode !== "rename")
            return ""
        // git's answer on this very name; the rest is worked out before asking.
        if (graphPane.namingGitRefusal !== "")
            return graphPane.namingGitRefusal
        const typed = graphPane.namingText
        if (typed.trim() === "")
            return qsTr("A name is needed")
        // As `SidebarRowGestures.editCaseOnly`: a tag renamed to its own name in other case loses both names on a
        // case-insensitive disk.
        if (graphPane.namingKind === "tag" && typed.trim() !== graphPane.namingId
            && typed.trim().toLowerCase() === graphPane.namingId.toLowerCase())
            return qsTr("Only the letter case differs — on this disk that deletes both names")
        if (page.graphNameTaken)
            return qsTr("%1 already has a branch called that").arg(page.graphRenameRemote)
        return GitFacts.validRefName(typed) ? "" : Words.notAName
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

    // What a resting pointer opens on a graph row: the row's own card, and the refs one chip had to stack. Owned here
    // because rows are recycled out from under both of them.
    RowHoverHost {
        id: rowHost
        graphPane: graphPane
        currentBranch: workTree.branch
        branchesModel: branchesModel
        remotesModel: remotesModel
        tagsModel: tagsModel
        tagAgainst: repoTab.defaultRemote
        menuRaisedOn: page.menuRaisedOn
        onRecordActivated: chip => page.activateChip(chip)
        // A plain click in the card is a click on its row: every name in it is on that one commit.
        onRecordChosen: (oidHex, atRow) => page.activateRow(oidHex, atRow)
        // The note under a cut message: the row's click, then the pane holding the whole message names itself — the
        // card's sentence is gone by the time the reader arrives (デザイン規約 §hover のツールチップ「送った先は名乗る」).
        onMessageAsked: (oidHex, atRow) => {
            page.activateRow(oidHex, atRow)
            detailsPane.callAttention()
        }
        onRecordMenuAsked: (oidHex, chip) => page.openRowMenu(oidHex, chip, true)
        // The line under a card's name: the same jump its left-panel row makes.
        onMateFollowed: oidHex => page.jumpToRef(oidHex)
    }

    // ---- rewriting one commit --------------------------------------
    // Run straight even for a pushed commit (デザイン規約 §長押し「ローカルの履歴書き換えはクリック」).
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
    // where typing lands (HEAD's own commit only, デザイン規約 §コミットメッセージの 2 つの枠), `stash` / `not-head` /
    // `standing` where it does not, `""` with no message.
    readonly property string messageEdit: GitFacts.messageEdit(
        !page.blank && repoTab.state === "open", detailsModel.shaHex, workTree.headOid,
        page.selectedStashRef, workTree.opText, workTree.opEditing)

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
                                              && page.selectedOid === workTree.headOid && workTree.headPublished

    // Moving off a half-written message in the details pane drops it and nothing asks (`DetailsPane.syncMessage`,
    // デザイン規約 §コミットメッセージの 2 つの枠): the button keeps it, and Escape or reading another commit are on purpose.

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
            workTree: workTree,
            graphModel: graphModel,
            detailsModel: detailsModel,
            diffModel: diffModel,
            branchesModel: branchesModel,
            remotesModel: remotesModel,
            worktreeModel: worktreeModel,
            worktreesModel: worktreesModel,
            stashesModel: stashesModel,
            tagsModel: tagsModel,
            graphPane: graphPane,
            sidebarPane: sidebarPane,
            detailsPane: detailsPane,
            diffPane: diffPane,
            wipPane: wipPane,
            carriedPane: carriedPane,
            planSeat: planSeat,
            gitCorner: gitCorner,
            refMenuSeat: refMenuSeat,
            fileMenuSeat: fileMenuSeat,
            diffMenuSeat: diffMenuSeat,
            commitMenuSeat: commitMenuSeat,
            remoteMenuSeat: remoteMenuSeat,
            rowHost: rowHost,
            clipboard: clipboard,
            commitMenuState: commitMenuState,
            publishFlow: publishFlow,
            upstreamFlow: upstreamFlow
        })
    }

    // ---- a report with nothing to answer ----------------------------
    // A write that did not happen, and whoever said no (デザイン規約 §答えの要らない報せ).

    /// The one door, so a headless run handing it a kind enters the same body the answer does. `reason` is the
    /// refuser's words; empty where this end refused, and then the second line is ours (`Words.writeReportedWhy`).
    function showReport(kind, remote, name, reason) {
        page.showNotice(Words.writeReported(kind, remote, name),
                        reason !== "" ? reason : Words.writeReportedWhy(kind, remote, name),
                        Words.reportTone(kind),
                        Words.reportNamesCopy(kind) ? name : "")
    }
    /// Automation only: git's answer to a write has been taken all the way — bar raised, mark down, standing questions
    /// cleared. What it was about rides the signal: a drain can bring several reports, so the tab holds none to read
    /// back.
    signal writeReported(string kind, string remote, string name)
    /// `label` is what did not happen, `detail` the refuser's words, `tone` the state colour, if any; `markWord` the
    /// working copy's name in `label` that wears the tree mark, left out for none (`NoticeBar.markWord`).
    function showNotice(label, detail, tone, markWord) {
        // Dressed, then raised — so nothing is written on a bar the reader can see (`NoticeBar.open`).
        noticeBar.label = label
        noticeBar.detail = detail
        noticeBar.tone = tone
        noticeBar.markWord = markWord === undefined ? "" : markWord
        noticeBar.open = true
    }
    /// Only lowered: the words stay while the bar slides up.
    function hideNotice() {
        noticeBar.open = false
    }
    /// Automation only, like `GraphPane.askCard`: a run reads `settled` / `shut` / `label` and presses `dismiss()`.
    readonly property alias noticeCard: noticeBar
    /// Automation: whether the centre stands under the report, not behind it — a picture cannot tell. In page
    /// coordinates, so it holds for graph and diff alike.
    readonly property bool noticeClears:
        centreStack.mapToItem(page, 0, 0).y >= noticeBar.mapToItem(page, 0, noticeBar.height).y

    property int seenWriteSeq: 0
    /// Whether the open file was already re-read for this notify's answers: a drain can bring several that moved it,
    /// and one read covers them all.
    property bool diffReadForAnswers: false
    /// Re-reads the open file because a write answered, and tells the tab — which keeps the status published behind
    /// that write from reading the same file as news (`ops::DiffReread`). The one door, so the count lives here.
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
        page.absorbCopyAnswer()
        // The status and this answer drain apart: the tree a stash emptied may already be read, with nothing left to
        // ask. No edge of its own — the landing is the edge, and with nothing standing it is a poll it ignores.
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
            // Unless the name itself was refused, the box has nothing to answer and closes as on a landing (a
            // half-finished rename too: its row is what could not be found).
            if (repoTab.writeReportKind !== "rename")
                page.noteRenameLanded()
            // Refused from outside: a report, as `reportRefusal`.
            if (repoTab.writeReportKind !== "") {
                // A refused name goes into the still-open box as well as the bar (デザイン規約 §答えの要らない報せ).
                if (repoTab.writeReportKind === "rename")
                    page.noteRenameRefused(repoTab.writeReportReason)
                page.showReport(repoTab.writeReportKind,
                                repoTab.writeReportRemote,
                                repoTab.writeReportName,
                                repoTab.writeReportReason)
                // The `>_` mark goes quiet with the report (`answeredFailures`).
                if (!commandsModel.failed)
                    page.answeredFailures++
                commandsModel.noteAnswered()
                page.pendingRenameRemote = ""
                page.pendingRenameTo = ""
                page.forgetRenameTagRemote()
                page.writeReported(repoTab.writeReportKind,
                                   repoTab.writeReportRemote,
                                   repoTab.writeReportName)
                return
            }
            // Otherwise the log comes up, as `tellRefusal`. A fetch here is not other news: the tab already raised the
            // panel for it, and taking the panel over would pin a failure that later heals (`CommandsOwner`).
            if (!repoTab.writeFetched)
                commandsOwner.newsTakes()
            page.commandsOpen = true
            // A rename that did not happen has nothing to carry over.
            page.pendingRenameRemote = ""
            page.pendingRenameTo = ""
            page.forgetRenameTagRemote()
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
            renameCarryFlow.startAsk("tag", on, was, now, at)
        }
        // Where the answer sends the reader, read per answer this notify carried (`RepoTab::write_answers`): a write
        // starting in the same drain (the fetch after a run) would already have lowered a single stop flag. The two
        // landings are exclusive, and the later answer wins.
        for (let i = 0; i < repoTab.writeAnswerCount(); i++) {
            // git stopped part-way: the answer is the working tree — the conflicted rows and the way out
            // (デザイン規約 §進行中の操作から出る). Armed, since the status carrying those rows has not arrived yet.
            if (repoTab.writeAnswerStopped(i)) {
                page.pendingWipSelect = true
                page.pendingHeadSelect = false
                page.pendingHeadAsked = false
            }
            // A commit at the tip: the selection goes there and the view follows.
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
        // An open diff is stale: re-read here on the answer, not on the status that follows
        // (rules-refs/app-ui.md「diff の読み直しは書き込みの答えの所で撃つ」).
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
    // Whether the shown diff is a working-tree file (stageable).
    property bool diffFromWt: false
    readonly property bool diffStaged: page.diffKind === "staged"
    /// The two stage letters git reports for the shown file, read once at open: on a conflict git prints no patch, so
    /// they are all there is to say. They hold until the file leaves the conflicts, which reopens it
    /// (`followEmptySide`).
    property string diffChange: ""
    /// Each conflict side's colour: the graph's lane colour for that branch, where it has one. `finishCount` is read
    /// only to depend on it — every graph property shares one notify, and the graph arrives in two passes (chips after
    /// rows) and rebuilds when refs move.
    readonly property int sideColorOurs:
        graphModel.finishCount >= 0 ? graphModel.conflictColorOurs(workTree.sideOurs, workTree.sideTheirs) : -1
    readonly property int sideColorTheirs:
        graphModel.finishCount >= 0 ? graphModel.conflictColorTheirs(workTree.sideOurs, workTree.sideTheirs) : -1
    function toggleDiff(kind, path, origPath) {
        if (page.diffShown && page.diffKey === kind + ":" + path) {
            page.closeDiff()
            return
        }
        page.openDiff(kind, path, origPath)
    }
    /// Reads one file, whoever asked — a row that was clicked, or the pane moving itself off a side that ran out
    /// (`followEmptySide`).
    function openDiff(kind, path, origPath) {
        // Held off while a plan has the centre (`centreStack`): a file opened now would still be up when the plan is
        // put away. The only place the diff is raised, so this one line holds the fold, neighbour and read; the file
        // rows stay live (規約 §フル interactive rebase「右の詳細ペインがそのまま生きる」).
        if (page.planShown)
            return
        // A held scroll is the last file's (規約 §diff を横へ送る「別のファイルは左端から」).
        diffPane.dropScroll()
        page.diffKey = kind + ":" + path
        page.diffKind = kind
        page.diffPath = path
        page.diffOrigPath = origPath
        page.diffFromWt = kind !== "commit"
        page.diffChange = kind === "conflicts" ? page.wipUnstaged.changeOf(path) : ""
        // The list's light follows the pane when it moves itself. Only this window's own tree: a carried copy's path
        // picked here would aim this tree's presses at it (`WipPane.readOne`).
        if (kind !== "commit" && page.wipWritable)
            wipPane.readOne(kind, path)
        if (kind === "commit")
            page.askCommitDiff(path, origPath)
        else if (page.carriedPath !== "")
            // Read-only, aimed at the copy the rows came from (`RepoSession::load_carried_diff`).
            diffModel.requestCarried(page.carriedPath, kind, path, origPath)
        else
            diffModel.requestWorkTree(kind, path, origPath)
        page.diffShown = true
        page.diffNeighbour = ""
        page.noteDiffNeighbour()
    }
    /// A pointer came to rest on a commit, or left one. Two lists reach this — the graph's rows and the right pane's
    /// under a choice (デザイン規約 §複数のコミットを選ぶ) — with one card between them (`RowHoverHost.openRowCard`).
    function restOnCommit(row, inside) {
        rowHost.rowCardWanted = inside
        if (inside)
            rowHost.openRowCard(row)
        else
            rowHost.settleRowCard()
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
        // The seats these come from are not offered on another copy's file (`DiffPane.partial`); this is the belt.
        if (!page.wipWritable)
            return
        // The fingerprint rides along: the write refuses to apply indices to drifted bytes. `diffAwaits` is set here,
        // from the slot's answer — `busyCount` rises too late, and a request turned away has no answer coming
        // (rules-refs/app-ui.md「書き込みが行に届くまで印は無効」).
        page.diffAwaits = repoTab.stageSelection(page.diffKind, page.diffPath, page.diffOrigPath, hunk, line,
                                                 diffModel.fingerprint)
    }
    /// Discards one hunk, unasked: the hold on its own heading's button is the consent (デザイン規約
    /// §バケツごとの一覧と、ペインの分け方).
    function discardHunkNow(hunk) {
        if (!page.wipWritable)
            return
        // Armed by the answer, for the reason `stageSelection` gives.
        page.diffAwaits = repoTab.discardSelection(page.diffKind, page.diffPath, page.diffOrigPath, hunk, -1,
                                                   diffModel.fingerprint)
    }
    // ---- what the reader lands on when a side runs out ---------------
    /// The open file's neighbour under the same heading, as `<bucket>:<path>` (`NavSectionModel.besidePath`). Noted
    /// while the file is still there: once the side runs out, the place it left is gone from the list.
    property string diffNeighbour: ""
    function noteDiffNeighbour() {
        if (!page.diffShown || page.diffKind === "commit" || !page.wipUnstaged.holdsPath(page.diffKind, page.diffPath))
            return
        page.diffNeighbour = page.wipUnstaged.besidePath(page.diffKind, page.diffPath)
    }
    /// The open file's side has run out (staged, unstaged, discarded, committed), so the pane moves
    /// (デザイン規約 §diff の中のステージ): the side's next file, else the same file from where it went, else closed.
    ///
    /// Driven by the file list's `changed`, not by the re-read coming back empty: that read runs beside the status
    /// (`load_diff` / `publish_status`) and can land before the list has moved, and some sides never read empty (an
    /// untracked file staged whole, a picture).
    function followEmptySide() {
        if (!page.diffShown || page.diffKind === "commit" || page.wipUnstaged.holdsPath(page.diffKind, page.diffPath))
            return
        const cut = page.diffNeighbour.indexOf(":")
        if (cut > 0) {
            const bucket = page.diffNeighbour.substring(0, cut)
            const path = page.diffNeighbour.substring(cut + 1)
            if (page.wipUnstaged.holdsPath(bucket, path)) {
                page.openDiff(bucket, path, page.wipUnstaged.origOf(path))
                return
            }
        }
        const moved = page.wipUnstaged.bucketOf(page.diffPath)
        if (moved !== "") {
            page.openDiff(moved, page.diffPath, page.wipUnstaged.origOf(page.diffPath))
            return
        }
        page.closeDiff()
    }
    /// Whether the diff on screen is still catching up with a write: git refuses a press made on rows that have since
    /// moved. Every part ends by itself — holding on "the tree has not been read yet" instead wedges for good
    /// (rules-refs/app-ui.md「書き込みが行に届くまで印は無効」).
    property bool diffAwaits: false
    readonly property bool diffSettling:
        page.diffAwaits || repoTab.busyCount > 0 || diffModel.loading
    /// The tree moved, so the open file is stale — whoever moved it (デザイン規約 §diff の中のステージ
    /// 「作業ツリーが動いたら diff を読み直す」). The callers already know the tree moved.
    function reloadDiff() {
        // Nothing this window writes moves another copy's file; its own tick keeps it current (`pollCarried`).
        if (!page.diffShown || page.diffKind === "commit" || !page.wipWritable)
            return
        diffModel.requestWorkTree(page.diffKind, page.diffPath, page.diffOrigPath)
    }
    /// Asks, on the page's tick, whether the file on screen still reads the same: the counts and the file list miss an
    /// edit that moves neither (a conflict resolved elsewhere keeps its stage letters until added). Returns whether a
    /// read went out — the only edge automation can latch on (`diff-tick`), since a quiet file gets no answer.
    function pollDiff() {
        if (!page.diffShown || page.diffKind === "commit" || page.diffSettling)
            return false
        // A copy's file is re-read on the copies' tick, aimed at the copy — through here it would read *this* window's
        // file of that name into a pane showing somebody else's (`pollCarried`).
        if (!page.wipWritable)
            return false
        return diffModel.refreshWorkTree(page.diffKind, page.diffPath, page.diffOrigPath)
    }
    /// The copies' tick over the copy being read: its file list and its open file. Off the page's tick — a `status` of
    /// another tree is too dear for it (`RepoSession::refresh_carried`). Returns whether it asked, as `pollDiff`.
    function pollCarried() {
        if (page.carriedPath === "")
            return false
        repoTab.readCarriedStatus(page.carriedPath, page.carriedName)
        if (page.diffShown && page.diffKind !== "commit" && !diffModel.loading)
            diffModel.refreshCarried(page.carriedPath, page.diffKind, page.diffPath, page.diffOrigPath)
        return true
    }

    /// Escape or the header's mark: puts the diff away and hands the arrows to the graph. Not the file row's second
    /// click, nor the closes the reader did not make (rules-refs/app-ui.md「diff を仕舞う 2 つの出口は矢印をグラフへ返す」).
    function closeDiffToGraph() {
        page.closeDiff()
        graphPane.takeKeyboard()
    }

    function closeDiff() {
        page.diffShown = false
        page.diffKey = ""
        page.diffKind = ""
        page.diffPath = ""
        page.diffOrigPath = ""
        page.diffFromWt = false
        page.diffChange = ""
        diffModel.clear()
    }

    // Exposed for the window toolbar (acts on the active tab).
    readonly property var pageTab: repoTab
    readonly property var pageWt: workTree
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
    /// Automation only, the same way (`worktree-stand`: a WORKTREES row stands the tab in that copy, and the landing
    /// is the window's).
    readonly property alias pageSidebar: sidebarPane
    /// Whether the refs listing has landed — the read that also settles how many remotes there are, and so what the
    /// band's fetch button may be (`fetch-tip`).
    readonly property bool pageRefsLoaded: branchesModel.refsLoaded
    /// Whether the right pane has the selected commit's own read — the page's last (the graph pass and refs land first
    /// and pick the row, `trySelectDefault`). A page on the working tree, or with no commit to select, waits for none.
    readonly property bool pageDetailsSettled: page.wipShown || page.selectedOid === ""
                                               || !detailsModel.loading
    /// Whether the page still owes a landing it has decided on (`pendingHeadSelect` / `pendingWipSelect`). Read by
    /// `PageSettled`: until it lands, every other reading is of the repository as the write found it.
    readonly property bool pageLanding: page.pendingHeadSelect || page.pendingWipSelect
    /// Automation only: the window's band, handed in by `Main`, so this page's verbs press the real Stash button
    /// (デザイン規約 §変更を退避する) instead of calling what it calls. `var` because `TopBar` is above this file, not
    /// beside it.
    property var pageBand: null

    /// Whether the command log is up (デザイン規約 §git が言ったことを読む場所).
    property bool commandsOpen: false
    // The mark is the panel's (`commandsAttention`) and goes down with it, whichever way the panel went.
    onCommandsOpenChanged: if (!page.commandsOpen) page.commandsAttention = false
    // Who owns the standing panel — the one rule about the order failures arrive in, so a component of its own with
    // every order walked (`CommandsOwner` / `tst_commandsowner.qml`).
    CommandsOwner {
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
    /// The one link tooltips carry (`Words.commandsHref`); other hrefs are ignored — a tip is not a browser.
    function tipLinkAsked(href) {
        if (href === Words.commandsHref)
            page.raiseCommands()
    }
    /// The reader putting the panel away — its `Clear` and closing mark, and Escape.
    function shutCommands() {
        commandsOwner.readerTakes()
        page.commandsOpen = false
    }

    // ---- the record of what was thrown away (破棄記録仕様.md) ----
    DiscardModel {
        id: discardModel
    }
    RecoverEntries {
        id: recoverShown
        rows: discardModel.rows
    }
    /// What the discard log draws, worded (`RecoverEntries`), and the words themselves — its times included, which
    /// the band over the graph writes too.
    readonly property var recoverEntries: recoverShown.entries
    readonly property alias recoverWords: recoverShown
    /// The moment the log's times are read against; a minute's step is the finest a time there says.
    property real recoverNow: Math.floor(Date.now() / 1000)
    Timer {
        interval: 30000
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
    /// The reflogs and the record, read each time the log is opened (破棄記録仕様.md §3): to the graph's window, or
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
    /// The seat is blinking.
    readonly property bool recoverBlinking: recoverBlink.running
    readonly property int recoverTaken: repoTab.takenSeq
    onRecoverTakenChanged: {
        if (!page.recoverOpen || page.sidebarCollapsed)
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
    // Twice on and off, a `blinkMs` each, as a flash is (P3-確認事項 §破棄記録と復元).
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
        // Folded (a diff took the screen), an open list is not on it: the rail's press brings it back, as a section's
        // way back does, rather than closing what cannot be seen.
        if (page.recoverOpen && page.sidebarCollapsed) {
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
            if (page.sidebarCollapsed)
                page.foldByHand(false)
        } else if (page.recoverPick >= 0) {
            // The graph goes back to what the repository holds with the list.
            page.recoverPick = -1
            page.showRecoverPick()
        }
    }
    /// A press on an entry picks it, or puts a picked one down. Picking ends a search as its `✕` does: the graph turns
    /// to what the entry would bring back. Not while the log reads again: the rows up are the last read's, and the
    /// answer would deal the number picked to another entry.
    function pickRecover(index) {
        if (page.recoverReading)
            return
        page.recoverPick = page.recoverPick === index ? -1 : index
        if (page.recoverPick >= 0)
            graphPane.findCard.dismiss()
        page.showRecoverPick()
    }
    /// The picked entry on the graph, or none (破棄記録仕様.md): what each part would bring back walks in dashed — a
    /// copy of thrown-away work as the uncommitted row it was, a dropped stash as the stash it was
    /// (`GraphModel.provisionalWipAt` / `provisionalStashAt`) — every other row dims as a search's misses do, and the
    /// view keeps to its span (`GraphModel.showDiscard`).
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
    /// Per part of the picked entry, whether what it would bring back is on the graph (破棄記録仕様.md §4): its tip a
    /// row the walk has drawn, or work put back into a working copy, which no window holds back. Empty until the walk
    /// that took the entry has landed (`GraphModel.provisionalWalked`) — before it, an undrawn tip is not yet walked,
    /// not out of reach. Read again as the graph's rows move — a press on `Load more` walks toward it.
    property var recoverReach: []
    function readRecoverReach() {
        const picked = page.recoverPicked
        if (picked === null || !graphModel.provisionalWalked) {
            page.recoverReach = []
            return
        }
        const tips = discardModel.partTipsAt(page.recoverPick)
        // A part whose commit this repository does not have is drawn nowhere and waits for no walk: its restore is
        // git's to refuse (破棄記録仕様.md §4).
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
        page.recoverAwaits = discardModel.restoreAt(page.recoverPick, part)
        return page.recoverAwaits
    }
    property string recoverLanding: ""
    /// A restore taken and not answered yet: the band's buttons wait for the answer, not for the write to start — one
    /// waiting its turn in another copy's order (破棄記録仕様.md §4) has not started, and a second press would bring
    /// the same part back twice. Set by the press's answer, as `diffAwaits` is.
    property bool recoverAwaits: false
    /// A restore came back: what it stood is selected, and work that did not come back as it was thrown away says
    /// where it went (破棄記録仕様.md §4) — the report a write that did not do all it was asked gives
    /// (デザイン規約 §答えの要らない報せ).
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
    /// Automation only: the header's `Clear`, pressed from outside the panel (`PGG_AUTO_ACT=commands-clear`).
    function clearCommandLog() {
        if (page.commandsPane !== null)
            page.commandsPane.clearPanel()
    }
    /// Automation only: the hand that drags over the log, and the key that takes what it picked
    /// (`PGG_AUTO_ACT=commands-select` / `commands-copy`).
    function pickCommandText(fromRow, fromAt, toRow, toAt) {
        if (page.commandsPane !== null)
            page.commandsPane.pickText(fromRow, fromAt, toRow, toAt)
    }
    /// ...and the same hand started on the ground under the last row (`PGG_AUTO_ACT=commands-sweep`).
    function sweepCommandGround(fx, fy) {
        return page.commandsPane !== null && page.commandsPane.sweepGround(fx, fy)
    }
    /// Automation: how much of the log is actually wearing the wash the drag left (`CommandsPane.washTally`) — the
    /// half of `commands-select` the picture cannot be judged on.
    function commandWashTally() {
        return page.commandsPane === null ? "worn=false rows=0 washed=0" : page.commandsPane.washTally()
    }
    readonly property bool commandsHasGround: page.commandsPane !== null && page.commandsPane.hasGround
    readonly property real commandsGroundTop: page.commandsPane !== null ? page.commandsPane.groundTop : 0
    function copyCommandText() {
        return page.commandsPane !== null && page.commandsPane.copySelection()
    }

    RepoTab {
        id: repoTab
        // A turn later: this arrives mid-drain filling this page, and standing the tab elsewhere takes the page down
        // (rules/app-ui.md §Qt Bridges・QML の不変条件 — 再入 borrow). `Qt.callLater` also folds a repeat ask into one.
        onStandHomeAsked: Qt.callLater(page.standHomeRequested)
    }
    CommandsModel {
        id: commandsModel
        // The UTC offset before the first row: the panel re-says it when raised (`CommandsPane.tellTheZone`), but
        // rows are stamped as they arrive, before any panel is built.
        Component.onCompleted: commandsModel.setZoneMinutes(new Date().getTimezoneOffset())
    }
    GraphModel { id: graphModel }
    WorkTreeModel { id: workTree }
    DetailsModel { id: detailsModel }
    DiffModel { id: diffModel }
    RebasePlanModel { id: planModel }
    NavSectionModel { id: branchesModel }
    NavSectionModel { id: remotesModel }
    // One letter apart: the three `*Model`s below are the WIP pane's buckets (uncommitted files); `worktreesModel`
    // lists git worktrees (the sidebar's WORKTREES). One per bucket (`WipPane`), each holding the whole status, so a
    // question about a file (which bucket, what is beside it, where it came from) goes to `worktreeModel`.
    NavSectionModel { id: conflictsModel }
    NavSectionModel { id: worktreeModel }
    NavSectionModel { id: stagedModel }
    // Another working copy's changes (`page.carriedPath`), kept apart: this window's own status arrives on its own
    // tick and would take the copy's rows, and the reader's place, with it. One list, not three: the split is the
    // index's, and nothing here moves that copy's index (`CarriedPane`).
    NavSectionModel { id: carriedModel }
    NavSectionModel { id: worktreesModel }
    NavSectionModel { id: stashesModel }
    NavSectionModel { id: tagsModel }

    /// True on the page for the tab in front — the only page there is: `Main.qml` builds it for that tab and destroys
    /// it when the tab leaves. Set once at construction and acted on in `Component.onCompleted`.
    property bool pageCurrent: false

    /// What this page owes on its way off the front, in order (`TabsModel::leaving_tab`), called while the page is
    /// still whole. The layout first — one set for the application, and the next tab is laid out at it
    /// (`PageLayout.reportLayout`); then the editor's words, which no repository can give back; then the repository.
    function leaveFront() {
        pageLayout.reportLayout()
        if (page.blank)
            return
        repoTab.holdDraft(wipPane.subjectText, wipPane.bodyText, page.amending)
        repoTab.release()
    }

    /// What this page owes on its way into another working copy (`TabsModel::leaving_copy`). The page stays — graph,
    /// refs and panes are the repository's, shared by linked copies (デザイン規約 §タブの所作「同じリポジトリのタブは 1 枚」);
    /// what goes is what the left copy owned. Called while that copy is still behind the tab, so the hub files the
    /// draft under it (`Hub::hold_draft`).
    function leaveCopy() {
        if (page.blank)
            return
        repoTab.holdDraft(wipPane.subjectText, wipPane.bodyText, page.amending)
        // Words left standing would be read as the arrived copy's (`restoreDraft` puts back only what it finds).
        page.amending = false
        wipPane.setAmendChecked(false)
        wipPane.clearMessage()
        // A plan is composed against one working tree and would be replayed in it.
        if (page.planShown)
            planModel.cancelPlan()
        // The pane goes back to this tab's own tree — which is about to be the copy it was reading.
        page.dropCarried()
        // A working-tree diff goes with the tree. A commit's stays (linked copies share the rows), but a read still
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
        page.pendingRenameRemote = ""
        page.pendingRenameTo = ""
        page.diffAwaits = false
        page.recoverAwaits = false
        page.moveLanding = ""
        page.pendingHeadSelect = false
        page.pendingHeadAsked = false
        page.pendingHeadFromSeq = 0
        page.pendingWipSelect = false
        page.rewordRow = -1
        page.planRunOut = false
        // The discard log's entry was walked by that session (`GraphModel.restand` takes it off the rows), and an open
        // list is that copy's reading: it reads again once the copy arrived at has settled (`standSettled`).
        page.recoverPick = -1
        page.recoverTipPending = ""
        page.recoverLanding = ""
        page.recoverReach = []
        page.owedDiscards = page.recoverOpen
        // What the models hold of that copy, each by its own rule (the `restand` slots).
        repoTab.restand()
        graphModel.restand()
        workTree.restand()
        conflictsModel.restand()
        worktreeModel.restand()
        stagedModel.restand()
        carriedModel.restand()
        // The command log is not reset: its rows name their session (`CommandMsg`), and a write the left copy is still
        // running ends in its own row (`BridgeSink::retire_reads`).
    }

    /// The tab now stands in the asked-for copy and its session is opening (`TabsModel::stood_copy`). The words only:
    /// other reads wait for `standSettled` — asked earlier they reach a session with no repository open and get
    /// nothing (`RepoSession::load_details`).
    function standInCopy() {
        if (page.blank)
            return
        page.restoreDraft()
        // The log's switch is per session (`Recording`) and a new session opens with it off while the panel still
        // shows it on, so it is said again.
        if (commandsModel.backgroundReads)
            commandsModel.setBackgroundReads(true)
    }

    /// Reads that died with the closed session (`leaveCopy`): the details, the open commit file, and the open discard
    /// log, which was that copy's reading.
    property bool owedDetails: false
    property bool owedDiff: false
    property bool owedDiscards: false

    /// The new session has said where it is (`RepoTab.standing` going down), so the reads that died with the last are
    /// asked again — only those: a commit is the same in every linked copy (the rule `activateRow` keeps for the
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
        if (repoTab.draftAmending()) {
            page.amending = true
            wipPane.setAmendChecked(true)
        }
        const subject = repoTab.draftSubject()
        const body = repoTab.draftBody()
        if (subject === "" && body === "")
            return
        // Show the sentence the reader left mid-way; this also holds the default selection off (`trySelectDefault`
        // leaves a working-tree page alone).
        page.showWip()
        wipPane.setMessage(subject, body)
        // `pendingWipSelect` stays down: a dirty tree's row is already at the top, and on a clean tree the flag would
        // stand until the tree got dirty, then jump the reader's view (rules-refs/app-ui.md の
        // `ListView.highlightFollowsCurrentItem` の行).
    }

    // ---- what this page is laid out at ------------------------------

    PageLayout {
        id: pageLayout
        page: page
        sidebarPane: sidebarPane
        rightPane: rightPane
        commandsSeat: commandsSeat
        graphPane: graphPane
        wipPane: wipPane
        detailsPane: detailsPane
        gitCorner: gitCorner
        repoTab: repoTab
        worktreeModel: worktreeModel
        detailsModel: detailsModel
        diffModel: diffModel
    }

    /// What the window reads off the page it shows: the floor (`Main.floorWidth` / `floorHeight`) and the two panes'
    /// own overflow, which the floor's verb asks about.
    readonly property real floorWidth: pageLayout.floorWidth
    /// …and the floor with the list open either way — what the band's actions time their giving way against
    /// (`PageLayout.openFloorWidth`).
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
        workTree.attach(page.tab_id)
        detailsModel.attach(page.tab_id)
        diffModel.attach(page.tab_id)
        planModel.attach(page.tab_id)
        discardModel.attach(page.tab_id)
        branchesModel.attachSection(page.tab_id, "branches")
        remotesModel.attachSection(page.tab_id, "remotes")
        conflictsModel.attachWorktree(page.tab_id, "conflicts")
        worktreeModel.attachWorktree(page.tab_id, "unstaged")
        stagedModel.attachWorktree(page.tab_id, "staged")
        carriedModel.attachCarried(page.tab_id)
        worktreesModel.attachSection(page.tab_id, "worktrees")
        stashesModel.attachSection(page.tab_id, "stashes")
        tagsModel.attachSection(page.tab_id, "tags")
        page.restoreDraft()
        const acts = harness.ask()
        if (acts !== null)
            acts.begin()
    }

    /// The window's focus epoch (bumped when the window regains focus) triggers a quick refresh of the visible page.
    property int focusEpoch: 0
    onFocusEpochChanged: {
        if (page.visible && repoTab.state === "open") {
            repoTab.refreshQuick()
            // The window coming back is when an outside change is most likely waiting, so the file on screen is asked
            // too — and the copy the pane stands on (`pollCarried` reads that one only; the rest are the slow tick's).
            page.pollDiff()
            page.pollCarried()
        }
    }

    /// True while the window is on screen (see Main.qml): the page shown there re-reads its repository on a tick, so a
    /// commit made in a terminal or by an agent turns up on its own.
    property bool onScreen: false
    Timer {
        interval: Metrics.pollIntervalMs
        repeat: true
        // Only the tab in front — the others catch up when switched to, so a tick reads one repository however many
        // are open.
        running: page.onScreen && page.visible && repoTab.state === "open"
        onTriggered: page.pollRepo()
    }
    // What the other working copies are carrying, on a tick of its own because it costs a `status` per copy — the
    // interval is the reader's setting (`AppBackend.copiesIntervalMs`, zero for never).
    Timer {
        interval: AppBackend.copiesIntervalMs
        repeat: true
        running: page.onScreen && page.visible && repoTab.state === "open" && AppBackend.copiesIntervalMs > 0
        onTriggered: {
            repoTab.refreshCarried()
            // …and the copy being read, file by file: the rows need only the tallies, the pane needs the list
            // (`pollCarried`).
            page.pollCarried()
        }
    }
    // The badge counting a running replay out, on its own tick: two file reads, no process (デザイン規約
    // §進行中・長押しの定数). Only while the write is out — the status tick says when the operation ended;
    // `replayRunning` covers the gap a handed-over plan leaves before the queue starts it.
    Timer {
        interval: Metrics.opProgressMs
        repeat: true
        running: page.visible && repoTab.state === "open" && page.replayRunning
        onTriggered: repoTab.refreshOpProgress()
    }
    /// One tick: the repository, and the file the diff pane holds (`pollDiff`).
    function pollRepo() {
        repoTab.refreshPoll()
        // The worktree listing rides this tick (one process): without it the WORKTREES rows and the mark saying a
        // branch is another copy's freeze until the window is clicked.
        repoTab.refreshWorktrees()
        page.pollDiff()
    }

    // Where the selection stands, kept so a commit that disappears from under it can be followed to whatever took its
    // place.
    property int selectedRow: -1

    // ---- the commits being held (デザイン規約 §複数のコミットを選ぶ) ----
    /// The choice, as a set of commit ids, and how many are in it. **Ids**: a background pass rewrites the rows under
    /// the hand, and everything this page holds is re-resolved by id when one lands (`onStatsChanged`).
    /// Empty only where the working tree's row is what is shown — that row is not a commit.
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
        if (oidHex !== "" && !GitFacts.wipOid(oidHex))
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
        // The working tree's row is not a commit: a modifier on it is the plain click it would be without one.
        if (mods === Qt.NoModifier || oidHex === "" || GitFacts.wipOid(oidHex)) {
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
    /// **One always stays in.** An empty choice is the working tree's own state, and reaching it from a commit would
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
    /// Reads the rows of the chosen commits again for the list on the right: what it says about them (a subject, a
    /// face) is read off the rows, so it is read again whenever the rows are.
    function refreshChosenRecords() {
        if (page.chosenIds.length > 0)
            page.chosenRecords = graphModel.chosenRows(page.chosenIds)
    }
    /// Every commit row between two places, ends included — what a Shift click reaches, **the rows scrolled past
    /// too**. Asked of the model in one crossing (`GraphModel.oidsBetween`): a call per row would put a walk of the
    /// history on a click (CLAUDE.md §性能予算).
    function chooseRange(from, to) {
        if (from < 0 || to < 0)
            return
        const next = ({})
        for (const oidHex of graphModel.oidsBetween(from, to)) {
            if (oidHex !== "" && !GitFacts.wipOid(oidHex))
                next[oidHex] = true
        }
        // A range that swept nothing but the working tree's row is not a choice; the anchor stays where it was.
        if (Object.keys(next).length === 0)
            return
        page.settleChoice(next)
    }
    /// The one place the set and its tally are written together, so a count and a highlight cannot disagree. **A
    /// choice back down to one commit is that commit being read** (`activateRow`).
    function settleChoice(next) {
        const ids = Object.keys(next)
        if (ids.length === 1) {
            page.activateRow(ids[0])
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
        page.settleChoice(next)
    }

    // The commit the viewport is measured against between passes, so the rows a reader is on can be put back under them
    // when new ones arrive above. The newest *real* commit: the WIP row comes and goes with the working tree.
    property string anchorOid: ""
    property int anchorRow: -1
    function rememberAnchor() {
        // Past however many working-tree rows stand over it — ours and one per other copy, all with the all-zero id,
        // so skipping one leaves the anchor on another (`GraphModel.newestCommitRow` holds the rule).
        const row = graphModel.newestCommitRow()
        page.anchorOid = row < 0 ? "" : graphModel.oidAt(row)
        page.anchorRow = row
    }
    // How far the graph slid under the viewport. Zero when the anchor is gone: a rewrite deep in the history moves rows
    // by different amounts and there is no single answer, so the view is left alone.
    function anchorShift() {
        if (page.anchorRow < 0 || page.anchorOid === "")
            return 0
        const now = graphModel.rowOf(page.anchorOid)
        return now < 0 ? 0 : now - page.anchorRow
    }

    // A landing on HEAD this page still owes. Held over: the working tree's status, the refs and the walk arrive as
    // three separate messages, and the two that come first still describe the repository as it was — landing off them
    // picks the commit that was just replaced. Resolved once the graph holds where HEAD points.
    property bool pendingHeadSelect: false
    /// The first report of HEAD that may answer the landing (`WorkTreeModel.headSeq`). A write's answer names it
    /// (`RepoTab.writeAnswerHeadSeq`): the report in hand at that arming may predate the write, with a stale HEAD that
    /// has a row. An arming read out of a report already in hand (the tree emptying, the selected commit vanishing)
    /// takes that very report.
    property int pendingHeadFromSeq: 0
    // Whether the reader asked for the landing, in which case the viewport goes to it too — a commit they meant to make
    // is no answer off screen. The other ways it is owed happen *to* the window (a terminal commit, a rewrite under the
    // poll), and those may not move the reader's view (rules-refs/app-ui.md — the `pendingHeadAsked` line).
    property bool pendingHeadAsked: false
    /// Whether the working tree the last status described is empty. **Every reader stands behind `workTree.loaded`**:
    /// the counts start at zero, which reads as a finished job before any status (rules/app-ui.md §UI 自動化).
    readonly property bool treeClean: workTree.stagedCount === 0 && workTree.unstagedCount === 0
                                   && workTree.untrackedCount === 0 && workTree.conflictCount === 0
    /// What operation the last status named, so that its going away can be read as an edge.
    property string seenOpText: ""
    /// The WIP face has nothing left to hold the reader with — the working tree emptied, or the operation went away
    /// (which takes the exit card off the face) — so land on the commit that now holds the changes. When someone else
    /// committed them, a message being written keeps the reader here: it is the one thing that cannot be read back off
    /// disk.
    ///
    /// **A stash pressed here lands past the message**: the press is the decision to empty the tree, and the entry took
    /// the words for its name. Left to the message, a stopped merge (whose box `absorbOpMessage` fills) never lands.
    ///
    /// `edge` is what the status moved (the tree's counts, or the operation); every half is read off that one status
    /// (rules-refs/app-ui.md — the `RepoPage.leaveWipWhenDone` line).
    function leaveWipWhenDone(edge) {
        // `treeClean`'s guard. `loaded` latches on the first status, so this holds off only a page's opening moment.
        if (!workTree.loaded)
            return
        // **A pane about another working copy is that copy's face** — its changes are still there, so nobody is walked
        // off it. The landing below stays armed for the reader coming back to their own row.
        if (!page.wipWritable)
            return
        // Whether this window's own stash emptied it (`ops::StashOut`), **asked from both sides of the pair**: the
        // status and the answer arrive in no fixed order, and the first asks and gets nothing.
        const ourStash = repoTab.takeStashLanding()
        if (!page.treeClean)
            return
        // **A landing of our own is an edge in itself**: the press is what moved.
        if ((!edge && !ourStash) || !page.wipShown || workTree.opText !== "")
            return
        if (!ourStash && (wipPane.subjectText !== "" || wipPane.bodyText !== ""))
            return
        page.wipShown = false
        page.pendingHeadSelect = true
        // The status this is read out of **is** the one that answers: its HEAD report is already in hand (`hub::sink`
        // sends it ahead of the status).
        page.pendingHeadFromSeq = workTree.headSeq
        // Only ours is a landing anybody asked for, so only ours takes the viewport along.
        page.pendingHeadAsked = ourStash
    }
    function tryPendingHeadSelect() {
        if (!page.pendingHeadSelect || !workTree.headKnown)
            return
        // `headOid` is HEAD branch or not, so a detached landing is the same.
        if (workTree.headSeq < page.pendingHeadFromSeq)
            return
        const row = workTree.headOid !== "" ? graphModel.rowOf(workTree.headOid) : -1
        if (row < 0)
            return
        page.pendingHeadSelect = false
        const asked = page.pendingHeadAsked
        page.pendingHeadAsked = false
        graphPane.setCurrentRow(row)
        page.activateRow(graphModel.oidAt(row))
        if (asked)
            graphPane.showRowSoon(row)
    }

    // An operation stopped part-way and this page owes it a landing on the working tree, where the conflicts and the
    // way out are. Held like `pendingHeadSelect`: the status carrying those rows arrives after git's answer. Always
    // asked for — a stop only ever follows a press — so the viewport goes along.
    property bool pendingWipSelect: false
    function tryPendingWipSelect() {
        if (!page.pendingWipSelect)
            return
        // Until the walk has prepended our row, row 0 is the commit that was on top or a neighbour copy's row with the
        // same all-zero id — so ask the graph (`GraphModel.wipRow`, the one place ours is told from theirs).
        if (!graphModel.wipRow)
            return
        page.pendingWipSelect = false
        graphPane.setCurrentRow(0)
        page.showWip()
        graphPane.showRowSoon(0)
    }

    // The selected commit is gone from the graph and this page did not rewrite it (an amend or a rebase in a terminal).
    // Whatever now stands where it stood is the closest thing to what was being read; failing that, where HEAD stands.
    function followVanishedCommit() {
        const oidHex = graphModel.oidAt(page.selectedRow)
        if (oidHex !== "" && !GitFacts.wipOid(oidHex)) {
            graphPane.setCurrentRow(page.selectedRow)
            page.activateRow(oidHex)
            return
        }
        page.selectedOid = ""
        page.pendingHeadSelect = true
        // The graph that let the commit go was rebuilt behind a read that had already reported where HEAD went, so
        // the report in hand is the one to land on.
        page.pendingHeadFromSeq = workTree.headSeq
    }

    // A reworded commit came back under a different hash: the one now standing where it stood is it, since only the
    // message changed.
    function followRewrittenCommit() {
        const row = page.rewordRow
        const oidHex = graphModel.oidAt(row)
        // Nothing there, or the working-tree row moved under it.
        if (oidHex === "" || GitFacts.wipOid(oidHex)) {
            page.rewordRow = -1
            return
        }
        graphPane.setCurrentRow(row)
        page.activateRow(oidHex)
    }

    /// `atRow`: the row, for callers that already know it (a click does); -1 asks the model (`GraphModel.rowOf`),
    /// which a landing named only by its commit has to do.
    function activateRow(oidHex, atRow) {
        page.rewordRow = -1
        page.pendingHeadSelect = false
        page.pendingHeadAsked = false
        page.pendingWipSelect = false
        // Clicking anywhere is the way out of the name box and of a standing row question.
        graphPane.stopNaming()
        page.stopRowAsk()
        // The right pane's mark is about the commit it was raised over, and every road to another commit comes through
        // here, keyboard ones included — the one road that keeps it raises it again (`onMessageAsked`).
        detailsPane.dropAttention()
        // **A plain landing is one commit** (デザイン規約 §複数のコミットを選ぶ). Whether it was several is carried past
        // the early return below: a click on the commit already open still has to run everything if the pane is
        // showing a choice.
        const wasChoice = page.chosenCount > 1
        page.chooseOnly(oidHex)
        const row = atRow !== undefined && atRow >= 0 ? atRow : graphModel.rowOf(oidHex)
        if (row >= 0) {
            graphPane.setCurrentRow(row)
            page.selectedRow = row
            page.chosenAnchorOid = oidHex
        }
        // **The working tree's row is not a hash**: nothing can be skipped for it, it shows whatever the tree is now.
        // Which tree is the row's answer — several copies' rows wear the same all-zero id (`openWipFor`).
        if (GitFacts.wipOid(oidHex)) {
            page.openWipFor(row)
            return
        }
        // A commit is nobody's working copy, so the pane stops being about one.
        page.dropCarried()
        // The commit already open: its details and signature answer a hash, which cannot change, so nothing is asked
        // again — the rename gesture's second click is exactly this click (デザイン規約 §グラフ行のダブルクリック).
        if (!page.wipShown && !wasChoice && page.selectedOid === oidHex)
            return
        page.wipShown = false
        page.selectedOid = oidHex
        page.selectedStashRef = graphModel.stashRefOf(oidHex)
        detailsModel.request(oidHex)
        page.askSignature(oidHex)
        page.closeDiff()
    }

    /// Something other than the reader is picking the row this page stands on, so the default below stays out of its
    /// way — it would land first and be photographed instead (`PageAutoStart`). False in every window a person opens.
    property bool rowPickedElsewhere: false

    // The opening landing (rules-refs/app-ui.md「ページの既定の着地」): the working tree's own row where the tree has one,
    // else HEAD's commit, so the right pane always shows something.
    function trySelectDefault() {
        if (page.selectedOid !== "" || page.wipShown || page.pendingHeadSelect || page.rowPickedElsewhere
                || graphModel.rowTotal === 0)
            return
        // Both reads must have answered: before the first status `wipRowStands` is false only because nothing was
        // asked, and a landing on a commit then is never taken back (the first line above holds every later call off).
        if (!workTree.headKnown || !workTree.loaded)
            return
        let row = -1
        if (workTree.wipRowStands) {
            // **Uncommitted work is the landing, branch or not** (デザイン規約 §未コミット行が名乗るもの). Asked of the
            // status (`graph::wip_row_stands`) — whether the opening walk carries the row races the first status — and
            // of the graph (`GraphModel.wipRow`), for the reason `tryPendingWipSelect` gives. Every pass calls this
            // again.
            if (!graphModel.wipRow)
                return
            row = 0
        } else {
            // Where HEAD stands, branch or not (`WorkTreeModel.headOid` — what `tryPendingHeadSelect` lands on too).
            row = workTree.headOid !== "" ? graphModel.rowOf(workTree.headOid) : -1
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
        page.activateRow(graphModel.oidAt(row), row)
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
                // moved.
                if (!resetHappened)
                    graphPane.shiftRows(page.anchorShift())
                page.rememberAnchor()
                // A stopped operation lands on the working tree's own row, which this pass is what puts there.
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
                        // the working tree's row (デザイン規約 §進行中の操作から出る), which this pass may not have yet;
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
    // Where HEAD stands rides the working-tree model, so the landings and the default selection are paid there
    // (`workTree.onChanged`), not on the refs.
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
    /// it**, and **which list is not passed on** (`ops_delete::note_listing_drawn`, rules-refs/app-ui.md
    /// 「消した行を戻す合図は 3 つ」). Withheld while a run holds the in-between open for a picture (`holdGoneRows`).
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
        // The rows are about to be swapped for a re-read of the same file (a write of ours, or an outside edit the
        // tick caught). Held here, not by the askers: the tick asks on files that turn out unchanged, and a place held
        // for a swap that never comes puts the next one back where the reader has since left (`DiffScrollPlace`).
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
    // The working copies on a word of their own too: their listing follows the write apart from the refs', and a
    // removed copy's row answers only to it.
    Connections {
        target: worktreesModel
        function onWorktreesSettled() {
            page.listingDrawn()
        }
    }
    /// The `WorkTreeModel.treeRevision` the open diff was last read against — bumped when the four buckets' counts
    /// move. Per file: a second hunk staged out of a file on both sides moves none, and is re-read at the write's
    /// answer (this window) or by `pollDiff` (anyone else).
    property int seenTreeRev: -1
    // **The tree was read** — heard from the working-tree model, not the list, which says `changed` only when its rows
    // differ: a status that moved no row is exactly the one this has to hear about.
    Connections {
        target: workTree
        function onChanged() {
            const moved = workTree.treeRevision !== page.seenTreeRev
            page.seenTreeRev = workTree.treeRevision
            // The operation that was standing is not standing any more — the other edge the WIP face's exit turns on,
            // and the only one a clean stop ever moves (`leaveWipWhenDone`).
            const opGone = page.seenOpText !== "" && workTree.opText === ""
            page.seenOpText = workTree.opText
            // **Written down before anything asks what it means**, waited on or not: a write's answer and the status it
            // published are drained apart, and this may be the half that arrives first.
            if (workTree.loaded)
                repoTab.noteTreeRead(workTree.statusSeq, page.treeClean)
            // Whether this is the status our own write published, whose file was already re-read at the answer
            // (`ops::DiffReread`, rules-refs/app-ui.md「diff の読み直しは書き込みの答えの所で撃つ」).
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
            // HEAD's report rides this model (`WorkTreeModel.headSeq`), so an owed landing is paid here — even for a
            // write that recorded nothing: the first read after a write always sends one.
            page.tryPendingHeadSelect()
            page.trySelectDefault()
        }
    }
    Connections {
        // Whichever list the pane is showing: the file the diff is on belongs to the copy those rows came from, and
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

        SplitView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            orientation: Qt.Vertical
            handle: SplitHandleBar {
                onHandChanged: (which, held) => page.holdSplitBar(which, held)
            }

            // ---- open failed ------------------------------------------
            OpenFailedScreen {
                id: failedScreen
                visible: page.openFailed
                SplitView.fillHeight: true
                SplitView.minimumHeight: pageLayout.panesMinHeight
                kind: repoTab.errorKind
                path: repoTab.errorPath
                message: repoTab.error
                pageWidth: page.width
                commandsPage: page
                onCloseRequested: page.closeTabRequested()
            }

            // ---- three-pane layout --------------------------------------
            SplitView {
                visible: !page.openFailed
                SplitView.fillHeight: true
                SplitView.minimumHeight: pageLayout.panesFloorHeight
                orientation: Qt.Horizontal
                handle: SplitHandleBar {
                    onHandChanged: (which, held) => page.holdSplitBar(which, held)
                }

                SidebarPane {
                    id: sidebarPane
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
                    workTree: workTree
                    branchesModel: branchesModel
                    remotesModel: remotesModel
                    worktreesModel: worktreesModel
                    stashesModel: stashesModel
                    tagsModel: tagsModel
                    collapsed: page.sidebarCollapsed
                    // Null on the blank page: nothing has run there (the band's `>_` answered the same way up above).
                    commandsPage: page.blank ? null : page
                    recoverPage: page.blank ? null : page
                    // The menus the rows raise are the page's, so only the page can say where the standing one came
                    // from — the folded list's open section stays under its own rows' menu alone.
                    menuRaisedOn: page.menuRaisedOn
                    onFoldRequested: collapse => page.foldByHand(collapse)
                    onRefActivated: oidHex => page.jumpToRef(oidHex)
                    onRefMenuRequested: (kind, name, full, oidHex, aim) =>
                        page.openRefMenu(kind, name, full, oidHex, true, aim)
                    onRemoteMenuRequested: name => page.openRemoteMenu(name)
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
                    onCopyAtRequested: (oidHex, name) => page.copyFromBox(oidHex, name)
                    onRenameSubmitted: (kind, id, name) => page.renameRow(kind, id, name)
                    onAddRemoteRequested: publishFlow.startAddRemote()
                }

                // Center: the report on a row of its own, then the commit graph ⇄ file diff. A write is answered
                // wherever the reader stands (デザイン規約 §答えの要らない報せ): inside either pane the report would be
                // silent in the other, and over them it would cover what it is about.
                ColumnLayout {
                    SplitView.fillWidth: true
                    // The graph's columns once both have given all they can, held up to a side pane's width so the
                    // middle never reads as the thinnest of the three (`PageLayout.centreMinWidth`).
                    SplitView.minimumWidth: pageLayout.centreMinWidth
                    spacing: 0

                    NoticeBar {
                        id: noticeBar
                        Layout.fillWidth: true
                        // **The order Escape is handed out in, and the one place it is written** (rules-refs/app-ui.md
                        // 「Esc は窓に 1 本しか生きられない」). **The question keeps it**: it holds the keyboard (its pill
                        // takes the focus as it opens) and stands until answered — a report is only read.
                        yieldsEscape: graphPane.asking
                        onAcknowledged: page.hideNotice()
                    }
                    // The discard log's entry on the graph, named over it with what it would bring back and the press
                    // that does — over the graph alone, which is what it describes (P3-確認事項 §破棄記録と復元).
                    RecoverBand {
                        Layout.fillWidth: true
                        visible: page.recoverPicked !== null && !page.planShown && !page.diffShown
                        entry: page.recoverPicked
                        when: page.recoverPicked === null
                              ? "" : recoverShown.whenWords(page.recoverPicked.at, page.recoverNow)
                        reach: page.recoverReach
                        canWalkFurther: graphModel.truncated && !graphModel.growing
                        step: graphModel.windowStep
                        busy: repoTab.busyCount > 0 || page.recoverAwaits
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
                            graphModel: graphModel
                            workTree: workTree
                            blank: page.blank
                            chipListAnchor: rowHost.refListAnchor
                            rowCardOid: rowHost.rowCardOid
                            chosenOids: page.chosenOids
                            chosenCount: page.chosenCount
                            onRowActivated: (oidHex, atRow, modifiers) => page.pickRow(oidHex, atRow, modifiers)
                            // The bar moves between matches — landing on the same row twice changes nothing and
                            // costs no git.
                            onFindLanded: oidHex => {
                                if (oidHex !== "" && oidHex !== page.selectedOid)
                                    page.activateRow(oidHex)
                            }
                            onRowMenuOpenRequested: (oidHex, chip) => page.openRowMenu(oidHex, chip)
                            onRowSwitchRequested: (oidHex, chip) => page.rowDoubleClicked(oidHex, chip)
                            // The same door the WORKTREES row opens: the tab stands in that copy, where its changes
                            // can be staged.
                            onCarriedOpenRequested: path => page.openRepositoryPathRequested(path)
                            onRowRenameRequested: (oidHex, chip) => page.startRename(oidHex, chip)
                            onRenameSubmitted: (kind, id, name) => page.renameRow(kind, id, name)
                            namingRefused: page.graphNameRefusedWhy !== ""
                            namingRefusedWhy: page.graphNameRefusedWhy
                            namingRefusedMark: page.graphNameRefusedMark
                            onChipExpandRequested: (oidHex, atRow, records, anchor) =>
                                rowHost.openRefList(oidHex, atRow, records, anchor)
                            onChipCollapseRequested: anchor => rowHost.closeRefListUnlessEntered(anchor)
                            onRowHoverRequested: (row, inside) => page.restOnCommit(row, inside)
                            onCreateBranchRequested: (oidHex, name) => repoTab.createBranch(name, oidHex, true)
                            // Nothing moves: a tag is left on the commit and the tree stays where it is.
                            onCreateTagRequested: (oidHex, name) => repoTab.createTag(name, oidHex)
                            onCreateCopyRequested: (oidHex, name) => page.copyFromBox(oidHex, name)
                            onOpenRepositoryRequested: page.openRepositoryPicker()
                            onAskConfirmed: page.answerRowAsk()
                            onAskCancelled: page.stopRowAsk()
                        }

                        DiffPane {
                            id: diffPane
                            diffModel: diffModel
                            fromWorkTree: page.diffFromWt
                            staged: page.diffStaged
                            conflicted: page.diffKind === "conflicts"
                            conflictChange: page.diffChange
                            // Read where the file is, and staged only where this window is the one holding it.
                            writable: page.wipWritable
                            copyName: page.carriedName
                            // The two swap over during a rebase; the model is where that is already answered. What
                            // *this* window is in the middle of, so a copy's conflicted file falls back to git's own
                            // two words the way its rows do (`WipBucketPane`).
                            sideOurs: page.wipWritable ? workTree.sideOurs : ""
                            sideTheirs: page.wipWritable ? workTree.sideTheirs : ""
                            sideColorOurs: page.sideColorOurs
                            sideColorTheirs: page.sideColorTheirs
                            busy: page.diffSettling
                            discardUnrecorded: GitFacts.discardUnrecorded(workTree.unborn, workTree.notCopied,
                                                                          [page.diffPath])
                            menuStanding: page.menuStanding
                            onCloseRequested: page.closeDiffToGraph()
                            onCopyRequested: text => clipboard.copy(text)
                            onCodeMenuRequested: page.openCodeMenu()
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

                // Right side: working tree ⇄ commit details
                Rectangle {
                    id: rightPane
                    // Where it starts; `applySavedLayout` assigns over this with the width the window is set to.
                    SplitView.preferredWidth: 400
                    SplitView.minimumWidth: pageLayout.rightMinWidth
                    color: Theme.bgSurface

                    GitVersionCorner {
                        id: gitCorner
                        // The run button takes the pane's foot while a plan stands, and that foot is this corner's
                        // seat (デザイン規約 §コミットメッセージの 2 つの枠「ペインの底は常に埋まる」). Said through the
                        // corner's own property — a `visible` here replaces the one it draws itself by.
                        offered: !page.planActive
                        // One of the three panes is on screen, and each measures its own file list. **The carried
                        // copy's is asked for its own room**: the tree's pane never lends the corner (its foot is the
                        // commit button's), so read through it a reader of another copy would get no version at all.
                        roomLeft: !page.wipShown ? detailsPane.bottomRoom
                                : page.wipWritable ? wipPane.bottomRoom : carriedPane.bottomRoom
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.rightMargin: Theme.spaceXs
                        anchors.bottomMargin: Theme.spaceXs
                        // Declared before the panes it hangs over, so z holds it in front of whatever they draw here.
                        z: 1
                    }

                    // Another working copy's changes are a pane of their own (デザイン規約 §別の作業コピーを読む): they
                    // can only be read, which is what the commit pane is shaped for.
                    CarriedPane {
                        id: carriedPane
                        anchors.fill: parent
                        visible: page.wipShown && !page.wipWritable
                        copyName: page.carriedName
                        files: carriedModel
                        readBucket: page.diffFromWt ? page.diffKind : ""
                        readPath: page.diffFromWt ? page.diffPath : ""
                        menuStanding: page.menuStanding
                        onTreeViewChosen: tree => page.setWipTreeView(tree)
                        onFileActivated: (bucket, path, origPath) => page.toggleDiff(bucket, path, origPath)
                        onFileWalked: (bucket, path, origPath) => page.openDiff(bucket, path, origPath)
                    }

                    WipPane {
                        id: wipPane
                        anchors.fill: parent
                        visible: page.wipShown && page.wipWritable
                        repoTab: repoTab
                        workTree: workTree
                        worktreeModel: worktreeModel
                        conflictsModel: conflictsModel
                        stagedModel: stagedModel
                        onTreeViewChosen: tree => page.setWipTreeView(tree)
                        amending: page.amending
                        headPublished: page.headPublished
                        menuStanding: page.menuStanding
                        onAmendToggled: on => page.amendToggled(on)
                        onCommitClicked: page.commitNow()
                        readBucket: page.diffFromWt ? page.diffKind : ""
                        readPath: page.diffFromWt ? page.diffPath : ""
                        onFileActivated: (bucket, path, origPath) => page.toggleDiff(bucket, path, origPath)
                        onFileWalked: (bucket, path, origPath) => page.openDiff(bucket, path, origPath)
                        onFileMenuRequested: (bucket, path) => page.openFileMenu(bucket, path)
                    }

                    DetailsPane {
                        id: detailsPane
                        anchors.fill: parent
                        // The run button's seat is carved off the pane's foot while a plan stands — the pane's own
                        // press-things-here edge moves up with it (デザイン規約 §コミットメッセージの 2 つの枠
                        // 「押す物はペインの底」).
                        anchors.bottomMargin: planRunBar.visible ? planRunBar.height : 0
                        visible: !page.wipShown
                        details: detailsModel
                        chosenCommits: page.chosenRecords
                        rowCardOid: rowHost.rowCardOid
                        onRowHoverRequested: (row, inside) => page.restOnCommit(row, inside)
                        onCommitDropRequested: oidHex => page.dropFromChoice(oidHex)
                        stashRef: page.selectedStashRef
                        menuStanding: page.menuStanding
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
                                ? qsTr("Finish the stopped operation first")
                                : ""
                        busy: repoTab.busyCount > 0
                        published: page.selectedPublished
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
                        onParentClicked: oidHex => page.jumpToRef(oidHex)
                        // The badge's press goes to the window, which owns the settings card.
                        onAvatarEditRequested: (name, email) => page.avatarSettingsRequested(name, email)
                        onCopyRequested: text => clipboard.copy(text)
                        onApplyStashRequested: selector => repoTab.applyStash(selector)
                        onPopStashRequested: selector => page.popStash(selector)
                    }

                    RebasePlanRunBar {
                        id: planRunBar
                        // The plan holds the details face up (`onPlanActiveChanged`); the WIP guard is the belt —
                        // were that face ever up, this bar would sit over the commit button.
                        visible: page.planActive && !page.wipShown
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.leftMargin: Theme.spaceMd
                        anchors.rightMargin: Theme.spaceMd
                        plan: planModel
                        pushedCount: page.planPushed
                        tipHeldElsewhere: workTree.headReachedElsewhere
                        busy: repoTab.busyCount > 0
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
                SplitView.minimumHeight: pageLayout.commandsMinHeight
                sourceComponent: CommandsPane {
                    curPage: page
                    commandsModel: commandsModel
                    errorText: repoTab.lastError
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

    // The split bars' refusal lives in the watcher; the page keeps the source chain, since only it sees all four
    // sources.
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
    }

    /// Automation: the drag past whichever boundary `which` names, and whether that boundary is still drawn
    /// (`PGG_AUTO_ACT=divider-refuse`; AutoActDriver calls through the page).
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
        // Under a plan the selection has two halves meant to be one commit — this page's and the model's row — so a
        // commit the plan holds is taken as the row click takes it, model first, and anything past the base is refused
        // in silence (rules-refs/app-ui.md — the `RepoPage.jumpToRef` line).
        if (page.planShown) {
            const planRow = page.planActive ? planModel.rowOf(oidHex) : -1
            if (planRow < 0)
                return
            planModel.selectRow(planRow)
            page.activateRow(oidHex)
            return
        }
        const row = graphModel.rowOf(oidHex)
        if (row >= 0)
            graphPane.jumpToRow(row)
        // A jump is one commit, whatever was being held (デザイン規約 §複数のコミットを選ぶ).
        page.chooseOnly(oidHex)
        // Details resolve even outside the window.
        page.rewordRow = -1
        page.wipShown = false
        // The row travels with the oid: `followVanishedCommit` reads `selectedRow`, and a stale one lands on whatever
        // now stands at the previous selection's row.
        page.selectedRow = row
        page.selectedOid = oidHex
        page.selectedStashRef = graphModel.stashRefOf(oidHex)
        detailsModel.request(oidHex)
        page.askSignature(oidHex)
        page.closeDiff()
    }
}
