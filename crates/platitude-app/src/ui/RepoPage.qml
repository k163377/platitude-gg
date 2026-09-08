pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

Item {
    id: page
    required property int index
    required property int tab_id
    readonly property bool blank: tab_id < 0

    /// The blank page's "Open repository…" button (folder picker).
    signal openRepositoryPicker()
    /// A worktree row was clicked: open that path as a new tab.
    signal openRepositoryPathRequested(string path)
    /// PG_AUTO_ACT=settings wants the window's settings dialog open for the screenshot.
    signal settingsDialogRequested()
    /// A conflicted file has nowhere to be opened: the git settings screen is where the merge editor is named.
    signal gitSettingsRequested()
    /// The settings card, opened from an avatar and carrying whom it was opened on.
    signal avatarSettingsRequested(string name, string email)
    /// PG_AUTO_PERF completion after every requested measurement output.
    signal perfFinished()
    /// The failed-open screen's "Close tab" button.
    signal closeTabRequested()

    property string selectedOid: ""

    property bool sidebarCollapsed: false
    /// The fold the diff put on: closing the diff takes back only that, never a fold made by hand.
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
    /// The same for the plan: the mode takes the graph pane whole, and the list beside it is a column of doors into a
    /// history the plan is composed over — so it goes down to the rail with the rest of the window's other business
    /// (規約 §フル interactive rebase). Its own flag, so putting the plan away takes back only the fold the plan made.
    ///
    /// **The fold comes back when the plan does, not when the replay ends.** From the run onwards the list has nothing
    /// left to say about a screen that is no longer standing over it, and the reader is back on the graph watching the
    /// rewrite land — the hold on the doors carries on without it (`doorsHeldWhy`).
    ///
    /// Unlike the diff's, there is no way back by hand while the plan stands: the rail's own fold control is inside
    /// the pane the plan freezes (`sidebarFrozen`).
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
    /// A plan has been handed over and its replay has not started yet, so the hold can begin at the press:
    /// `replaying` rises when the queue *starts* the write, not when the button was let go.
    ///
    /// **The gap is not a fixed length and no sequence number spans it.** The run is queued, and the queue can be
    /// carrying something else — the timer's fetch travels it too — so the wait is however long that takes; and
    /// `writeSeq` counts *every* write's answer, so the fetch landing in between would move it past anything armed
    /// here and let the pane back to life with the rebase still to come. Let go of the moment a replay is under way,
    /// which is exactly where `replayRunning` takes over.
    property bool planRunOut: false
    readonly property bool replayRunning: repoTab.replaying || page.autoReplayHeld
    onReplayRunningChanged: if (page.replayRunning) page.planRunOut = false
    /// Automation: a replay this run really started, kept standing until its picture has been taken
    /// (`AutoActNavVerbs`, PG_AUTO_ACT=doors-held). The rise is caught at the signal rather than sampled — a demo
    /// repository's rebase is over inside one beat of the sampler — and what it holds up is the state the picture is
    /// of, not a stand-in for it (app-ui.md §UI 自動化の因果性: 一瞬だけ立つ状態は signal で観測して latch する).
    property bool autoReplayHeld: false
    /// **The two ways the left pane is out are two different things, and they are not shown the same way.**
    ///
    /// This one is the plan's: while a rebase is being composed the only way into a write is the run button, so the
    /// whole pane goes to the disabled step and stays there — the restriction belongs to the mode rather than to a
    /// command, and lasts as long as the mode does, so it is said plainly (デザイン規約 §フル interactive rebase). Only
    /// the `>_` band is left out of it (`SidebarPane.frozen`).
    ///
    /// **From the press, not from the rows**: the mode is entered when the face takes the graph's seat, and a pane
    /// that stays live under it for the length of the read would be saying the reader may still write — which is
    /// the one thing this mode is for taking away.
    readonly property bool sidebarFrozen: page.planShown
    /// The other one, which begins where the plan ends: a write that replays a range a commit at a time is running
    /// (`RepoTab.replaying` — the meaning core puts on the op name), or its run is out and has not started yet. A
    /// rebase is measured in seconds once the range is deep, and a switch or a delete let go into the middle of one is
    /// the exit nobody meant — but the answer here is "wait for it", not "this screen is another mode".
    ///
    /// **So nothing freezes and nothing leaves**: the doors are held one at a time and each says this line
    /// (`SidebarPane.doorsHeld` / `AppMenu.heldReason`), and the lock comes off the moment git answers. One line for
    /// both halves of that, so the pane and the menus cannot disagree about whether a rewrite is under way.
    readonly property string doorsHeldWhy:
        page.replayRunning || page.planRunOut ? Words.otherCommandRunning : ""
    /// A press landed away from whatever held the keyboard (Main's `FocusRelease`), at `scenePos` — `null` for a press
    /// with no place of its own (the headless run's door). The left menu's name box goes with it — nothing is asked,
    /// what it costs is the typing (デザイン規約 §左メニューの所作) — and so does anything over the graph that is standing on an
    /// empty box, where there is not even that to lose (§コミットを探す).
    function releasePressedAway(scenePos) {
        sidebarPane.stopEdit()
        graphPane.dropEmptyBoxes(scenePos)
    }
    /// A press landed anywhere in the window (Main's `FocusRelease` again, this one on every press rather than only
    /// the ones that take a caret away). The right pane's mark says where a reader was just sent, and the first thing
    /// they do after arriving is the answer that they arrived (デザイン規約 §hover のツールチップ).
    function notePress() {
        detailsPane.dropAttention()
    }
    /// And Escape, which every other standing thing in this window answers (デザイン規約 §hover のツールチップ).
    ///
    /// **A key handler rather than a `Shortcut`**, and that is what keeps it out of everyone else's way: a shortcut is
    /// matched before the key is delivered at all, so a bar, a popup or a box that wants Escape takes it first and
    /// this is never reached — which is the rule itself, since the mark is the last thing left to dismiss. **Two
    /// enabled `StandardKey.Cancel` shortcuts in one window fire neither**, so a third one here would have taken
    /// whichever bar was standing down with it (`tests/qml/tst_escape.qml` holds all of this). The two bars keep one
    /// live between them by an order written where they meet (`noticeBar.yieldsEscape`); anything else that comes to
    /// want Escape is added the way this one is, as a key handler under all of them.
    ///
    /// Accepted only when there was a mark to take: an Escape this page did nothing with is not this page's.
    function escapePressed() {
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
        // Every way the list comes back arrives here — the band's block, a rail cell, the rename box
        // (SidebarPane.startEdit) — so "unfolding closes the diff" has no exceptions elsewhere.
        if (!collapse && page.diffShown)
            page.closeDiff()
    }

    property bool wipShown: false
    // The ask bar goes off screen with the pane, and nothing off screen may be answered.
    onWipShownChanged: if (!page.wipShown) page.stopRowAsk()
    // Selected stash row's reflog selector ("" = not a stash).
    property string selectedStashRef: ""
    function showWip() {
        page.wipShown = true
        page.pendingHeadSelect = false
        page.pendingHeadAsked = false
        page.pendingWipSelect = false
        page.selectedOid = ""
        page.selectedStashRef = ""
        // The working tree's row is not a commit, so nothing is held while it is what is shown — and the rows fall
        // back to the current one for the highlight there (`GraphRowDelegate.selected`).
        page.chooseOnly("")
        page.closeDiff()
    }

    // ---- commit editor -------------------------------------------
    property bool amending: false
    // Whether a remote already has the commit HEAD is on — the amend's `already pushed`. Answered by the session off
    // the walk's own marks and kept beside HEAD (`WorkTreeModel.headPublished`), so nothing here asks git for it and
    // a HEAD that moved wears no answer about the commit it left.
    readonly property bool headPublished: workTree.headPublished
    Connections {
        target: repoTab
        function onChanged() {
            page.absorbHeadMessage()
            page.absorbMoveAsk()
            page.absorbWriteResult()
        }
        // Only the first failure of a run: an offline machine would otherwise re-raise the panel every interval (デザイン規約
        // §git が言ったことを読む場所).
        function onFetchFirstFailed() {
            page.commandsOpen = true
        }
    }

    // A tab whose repository would not open. The screen sits where the panes do, not over the whole page, so the log's
    // seat and the panel both stay reachable. The log is not raised on its own: the failed command is a background
    // read, so the panel would come up empty (measured).
    readonly property bool openFailed: !page.blank && repoTab.state === "error"
    /// Automation: that screen's hand — the one its three lines are dragged over from the air around them
    /// (`PG_AUTO_ACT=open-fail-sweep`). An automation-only exposure, the same one `GraphPane.view` is (app-ui.md).
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

    /// A stopped merge opens the box already holding what it is about to record.
    ///
    /// The button under it is what finishes a merge — `git commit` there writes the very commit `--continue` would,
    /// down to the tree, the two parents and the hooks it runs (measured, 2.55) — so the message it will use belongs on
    /// screen before the press rather than in the log after it (デザイン規約 §進行中の操作から出る).
    ///
    /// **Answered once per message, and never written over a draft.** Text in the boxes is the one thing here that
    /// cannot be read back off disk, so a merge arriving under it leaves it where it is; the merge's own words are
    /// still the placeholder underneath, and emptying the boxes commits them.
    ///
    /// **Taken back out when the merge goes** — but only where it is still standing untouched, which is what the pair
    /// below is for: an abort leaves a box holding a message for a merge that no longer exists, and one word typed
    /// into it makes it the reader's.
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

    /// The name the entry a pop is bringing back was carrying, held until git says the pop landed.
    ///
    /// **Read at the press**: by the time the answer comes the entry is off the list, and nothing else holds the
    /// words. Empty for one git named itself (`WIP on …`) — that line names the commit the work was standing on,
    /// not the work.
    property string pendingPopLabel: ""
    /// What `writeSeq` stood at when it was armed, so the answer it is waiting for can be told from any other.
    ///
    /// **Every stash operation answers under the same word** (`writeStashed` cannot say whose), so the answer alone
    /// does not say it was the pop's: the details pane's band leaves its buttons live, and an `apply` pressed just
    /// before a pop would take the pop's name with it — and drop it if that apply were refused. Only the very next
    /// answer is this one's, and anything else disarms it: a name put back off the wrong write is worse than one not
    /// put back at all.
    property int pendingPopSeq: -1
    /// Both ways in to a pop — the graph row's menu and the details pane's band — so the name comes back from one
    /// place (デザイン規約 §変更を退避する).
    function popStash(selector) {
        page.pendingPopLabel = GitFacts.stashLabel(stashesModel.nameOfFull(selector))
        page.pendingPopSeq = repoTab.writeSeq
        repoTab.popStash(selector)
        page.selectedStashRef = ""
    }
    /// The answer to that pop, whichever way it went. Read before the refusal branch below so both landings pass
    /// through here — and the words only go in where the pop is what answered, and it landed
    /// (デザイン規約 §変更を退避する. by design).
    ///
    /// **Never over what is already typed.** Text in these boxes is the one thing on this page that cannot be read
    /// back off disk (`absorbOpMessage`), and both are asked: a description with no summary is not an empty editor.
    function absorbPopLabel() {
        if (page.pendingPopSeq < 0 || repoTab.writeSeq <= page.pendingPopSeq)
            return
        const carried = page.pendingPopLabel
        const mine = repoTab.writeSeq === page.pendingPopSeq + 1 && repoTab.writeStashed
        page.pendingPopLabel = ""
        page.pendingPopSeq = -1
        if (mine && carried !== "" && wipPane.subjectText === "" && wipPane.bodyText === "")
            wipPane.setMessage(carried, "")
    }

    // Not confirmed even when HEAD is already on a remote: amending rewrites nothing that a switch or a reset cannot
    // bring back, and the push that would spread it is asked about on its own.
    function commitNow() {
        repoTab.commit(wipPane.outgoingSubject, wipPane.outgoingBody, page.amending, wipPane.resetAuthor)
    }

    // ---- moving between branches and commits ----------------------
    // Terminology is deliberate: git runs `switch` / `restore`, and the UI says "Switch to" (デザイン規約 §用語).
    //
    // Uncommitted changes come along unasked; where git refuses, core goes round through a stash (デザイン規約
    // §未コミット変更がある状態での移動).
    //
    // kind is "branch" / "remote" / "force" (a local branch moved to `moveStart` before landing on it). Every one lands
    // on a branch — nothing here moves onto a bare commit (デザイン規約 §ブランチ・コミットへの移動).
    property string moveKind: ""
    property string moveTarget: ""
    property string moveLocal: ""
    property string moveStart: ""

    // ---- the same chip, pressed again ------------------------------
    /// The branch a move already sent is putting HEAD on; `""` while none is on its way.
    ///
    /// **A press is decided from what the screen holds, and the screen takes a move in long after the write that made
    /// it has answered** — core answers a write and only then publishes the refs it moved (`session::write`), and a
    /// listing of fifty thousand refs is the slow half of that. Pressed twice on a remote branch with no local one,
    /// the second press is decided from the very picture the first was and sends the same `switch --create`, which
    /// git refuses because the first one made the branch.
    ///
    /// So while a move is on its way, the road a press arrives by (`switchToRef`) sends nothing. **What ends the wait
    /// is the move's own landing, not a counter**: HEAD on the branch and that branch in the listing
    /// (`offers::move_landed` — a seq would move for reads nobody asked for). Everything else that can become of a
    /// move puts it down where it happens, and each of those certainly comes: a refusal (nothing moved, so pressing
    /// again is the reader's to do) and the question a move can come back as (`absorbMoveAsk`).
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
        // Marked where the move goes out rather than where it was asked for: the question a blocked move raises comes
        // back through here, and a press held behind a bar has sent nothing yet.
        page.moveLanding = page.moveLocal
        if (page.moveKind === "branch")
            repoTab.checkoutBranch(page.moveTarget, leaving === true)
        else if (page.moveKind === "remote")
            repoTab.checkoutRemote(page.moveTarget, page.moveLocal, leaving === true)
        else if (page.moveKind === "force")
            repoTab.checkoutForceCreate(page.moveTarget, page.moveStart, leaving === true)
    }

    // ---- moving out of what stands in a move's way --------------------
    // What blocks a move — an operation standing or unmerged paths — is core's measured rule
    // (offers::moves_blocked → `workTree.movesBlocked`). The move has to clear the way first, and that is a question
    // rather than a side effect (デザイン規約 §進行中の操作から出る).
    //
    // **One question for both.** The second needs no operation put down, and everything after that is the same two
    // commands, so it is the same question with one clause fewer.
    //
    // **Asked before anything is sent.** The screen already knows what is standing, so letting git refuse would put a
    // red line in the command log where a question belongs — and leave the reader where they were with nothing to
    // press.
    function standsInTheWay(leaving) {
        return leaving !== true && workTree.movesBlocked
    }
    readonly property string leaveHeading:
        workTree.leaveUndoes ? qsTr("Undo it and go?") : qsTr("Put it aside and go?")
    readonly property string leaveDetail:
        workTree.leaveUndoes ? qsTr("Nothing it did since it started is kept.")
        // Nothing standing: the files are the whole of what is in the way, and there is no operation to name.
        : workTree.opCommand === ""
        ? qsTr("The files waiting on a decision go to the stash, markers and all.")
        //: %1 is the standing operation in git's own spelling, e.g. cherry-pick.
        : qsTr("The %1 stops; its files go to the stash, markers and all.").arg(workTree.opCommand)
    function askLeaveOperation(retry) {
        // Marked on the row HEAD stands on — a rebase runs detached, so that is the only name the tree has for where
        // it is (デザイン規約 §立っている質問は 1 か所で聞く). Whether leaving undoes anything — the rebase, whose abort throws away
        // work in hand: `danger` and a hold (§状態, §長押し) — and the command that opens the question's line are core's
        // answers (offers::leaving_undoes / leave_code → `workTree.leaveUndoes` / `leaveCode`).
        page.startRowAsk(workTree.headOid, page.leaveHeading, page.leaveDetail,
                         workTree.leaveUndoes, "", retry, workTree.leaveUndoes, "", null,
                         workTree.leaveCode)
    }

    // ---- what a chip leads to --------------------------------------
    // `record` is the chip as it is drawn — kind letter, the flag digits, then the name (see encode.rs, `FLAGS`).
    function activateRecord(record) {
        if (record !== "")
            page.switchToRef(record[0], GitFacts.recordName(record))
    }
    // A branch another working copy holds is the one refusal no stash gets past and no operation put down can clear
    // (offers::SwitchAction) — the branch is simply somewhere else, and the way to it is that copy. So the press
    // raises a bar like every other refusal does, and the pill goes there instead: the same road the WORKTREES row
    // takes (`openRepositoryPathRequested`). **The `!` after the word is what says the pill is not the switch that
    // was pressed** (デザイン規約 §進行中の操作から出る, by design).
    function askOpenHolder(local) {
        const held = worktreesModel.worktreeHolding(local)
        if (held === "")
            return
        const leaf = GitFacts.pathLeaf(held)
        page.startRowAsk(
            branchesModel.oidOfName(local),
            //: %1 is the folder of the working copy that has the branch checked out.
            qsTr("Open %1 instead?").arg(leaf),
            //: %1 is a branch name.
            qsTr("%1 is checked out there, so nothing here can move onto it.").arg(local),
            false,
            qsTr("Open"),
            function () { page.openRepositoryPathRequested(held) },
            false,
            "",
            null,
            "")
        graphPane.askAlert = true
    }
    /// Answers whether the press did anything — a move sent, or a question raised in front of one. `false` is a press
    /// this road turned away, which is what the headless double press reads (動詞 `switch-remote-twice`): the second
    /// of two presses in one turn has to be turned away, and the gate that turns it away is not something the run can
    /// see from outside (both presses look alike, and the window after them differs only by a line in the log).
    function switchToRef(kind, name, leaving) {
        // `busyCount` alone is not the gate: it rises when the queue starts the write, not when the press is made, and
        // it is back down while the screen is still catching up with what the write did (`moveLanding`).
        //
        // **The held doors are the third of them**, and the one that covers the beat between a plan's run being handed
        // over and the queue starting it: the count is still zero there, and a move let go into that gap lands ahead
        // of the rewrite it was made about. This road is where the graph's own doors end up — a row's double-click, a
        // chip's, a row of the list a chip had to stack — so holding it here holds all three (`doorsHeldWhy`).
        if (repoTab.state !== "open" || repoTab.busyCount > 0 || page.moveLanding !== ""
                || page.doorsHeldWhy !== "")
            return false
        // The models hold the lookups — the local branch a remote row lands on is the one another copy can be holding
        // — and which move they add up to is core's rule (offers::switch_action): a tag or the detached marker moves
        // nothing, a tag's row offering a branch at its commit instead (`startNaming`).
        const local = kind === "R" ? remotesModel.localNameFor(name) : name
        const action = GitFacts.switchAction(kind, local, workTree.branch,
                                             worktreesModel.worktreeHolding(local),
                                             branchesModel.oidOfName(local))
        // Before the leave question: the holder refusal is the one no
        // operation put down can clear (offers::SwitchAction), so asking
        // to undo a rebase first would spend the undo on a move that was
        // never possible.
        if (action === "holder") {
            page.askOpenHolder(local)
            return true
        }
        // Ahead of the branches below rather than inside them: the one that lands on an existing local branch asks git
        // what the move would cost before it moves, and that read is worth nothing while an operation is standing.
        if (page.standsInTheWay(leaving)) {
            page.askLeaveOperation(function () { page.switchToRef(kind, name, true) })
            return true
        }
        if (action === "switch")
            page.switchTo("branch", name, name, "", leaving)
        else if (action === "materialize")
            page.switchTo("remote", name, local, "", leaving)
        else if (action === "move") {
            // Whether to ask is git's to answer — a branch that only fell behind loses nothing by moving. The question
            // comes back as `moveAskSeq` when something would be lost, and the operation is left standing for the
            // answer to that one to undo (core asks before it aborts, never the other way round).
            page.moveLanding = local
            repoTab.checkoutMovingBranch(local, name, leaving === true)
        }
        // A tag or the detached marker is the one press left: it moves nothing, and it raised nothing either.
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
    /// Automation: the question above was raised, about `local`. An automation-only exposure, the same one
    /// `GraphPane.view` is (app-ui.md) — the bar itself carries no name a run could read the branch back off.
    signal moveBranchAsked(string local)

    // ---- the standing question --------------------------------------
    // One bar over the graph (デザイン規約 §可否・警告の出し場所); Escape, another ask or a click anywhere else walks away from it.
    // Only questions about a ref reach it: everything that takes one named thing away is held down on the row or button
    // that names it (デザイン規約 §長押し).
    /// Ctrl+F: the find bar belongs to the graph — which the plan is standing over while one is open, so the press
    /// is quietly refused there, the way a standing question already refuses it (`GraphPane.startFind`).
    function startFind() {
        if (page.planShown)
            return
        graphPane.startFind()
    }

    // ---- the interactive-rebase plan ---------------------------------
    // Composing runs nothing; the run is the one queued write, pinned by the model to the tip the plan opened on.
    // While the plan stands, the page holds down what could move the history under it from inside this window (the
    // sidebar, find, the toolbar's own writes — TopBar reads `planActive`); what it cannot hold — a terminal, another
    // session — the model answers by putting the plan away when the tip moves (`noteHead` below).
    /// A plan is standing: its rows are here and they are a draft somebody can compose.
    ///
    /// **The one answer**, read by everything on this page that a standing plan changes — the freezes, the right
    /// pane's boxes, the run bar, the corner it takes the seat of. Written twice they could disagree, and the run
    /// that holds the opening face (`planLoadHeld`) is exactly the moment a second spelling would.
    readonly property bool planActive: planModel.active && !page.planLoadHeld
    /// The plan's face has the graph's seat: the read that opens it is out, or its rows have arrived.
    ///
    /// **The seat is taken at the press, not at the answer**. The read walks the whole range — 27ms where the click
    /// was shallow, a second and more at the root of a real history (measured, JetBrains/kotlin) — and a screen that
    /// does not move for that long says the press was not heard. What the pane can say before the rows land is in
    /// the pane (`RebasePlanPane.waiting`).
    ///
    /// **The mode's restrictions are on this too** — the left menu, `push`, `stash` and Ctrl+F all go out from the
    /// press, because entering the mode is what taking the seat *is*. What waits for the rows is only what needs a
    /// draft to mean anything, and that reads `planActive`.
    readonly property bool planShown: planModel.active || planModel.loading
    /// Automation (`PG_AUTO_ACT=plan-loading`): the opening face, held from the real edge — the rows arrive through
    /// the feed and can land while the asynchronous grab is still out, and the picture would then be of the plan
    /// rather than of the wait for it (verify-ui スキル §中間状態は実 edge を latch する).
    ///
    /// It holds the *whole* face, not the middle of it: a run that held only the pane would photograph the run bar
    /// standing at the right pane's foot and the left menu already frozen, under a centre that says the plan has not
    /// arrived. That is a screen this app never puts up.
    property bool planLoadHeld: false
    /// The plan model itself — an automation-only exposure, the same one `GraphPane.view` is (app-ui.md).
    readonly property var rebasePlan: planModel
    /// The rewrite warning's count for the plan's own range — the plan's own answer, counted by the read that opens
    /// it and again when the refs move under it (`RebasePlanModel.pushedCount`); 0 while no plan stands.
    readonly property int planPushed: planModel.pushedCount
    /// Whether the details pane's boxes are, right now, a plan row's reword input: the plan stands, the row the
    /// selection sits on carries the verb, **and the pane is showing that very commit** — anything else that moves
    /// the selection (a shortcut, a landing) must not leave typing routed into a row whose message is not on
    /// screen. The model owns which row it is, so a reorder cannot detach the two.
    readonly property bool planReword: page.planActive && planModel.selectedAction === "reword"
                                       && planModel.selectedOid !== ""
                                       && planModel.selectedOid === detailsModel.shaHex
    /// What `Discard` is about to take away with the plan: the plan's own edits, and a reword standing in the right
    /// pane's boxes that the plan has not been given yet. Both are composed here and nowhere else — the screen after
    /// the press holds no copy of either — so the button is held rather than clicked (デザイン規約 §長押し).
    ///
    /// **A half-written amend from before the plan stood is not in this.** The plan closing leaves that text exactly
    /// where it is (`DetailsPane.dropDraft`), so there is nothing for a hold to guard, and arming one there would
    /// say the press costs something it does not.
    ///
    /// **Read off `boxMoved`, not `messageDirty`.** A row walked back out of `reword` locks the boxes with the
    /// typed text still standing in them, and the reader can neither save it nor be warned by a button that asks
    /// whether the boxes still take typing — but `Discard` takes that text all the same.
    readonly property bool planDiscards: page.planActive
        && (planModel.dirty || (detailsPane.boxFromPlan && detailsPane.boxMoved))
    function startRebasePlan(oidHex) {
        planModel.open(oidHex)
    }
    // What the pane's arrival and departure do, on the edge the *seat* changes rather than the one the rows do: the
    // graph is gone from the press, and a diff opened over it goes with it. Either way out of the read — rows, a
    // refusal, a failure, `Discard` pressed on the empty face — comes back through here, so the fold has no exceptions.
    onPlanShownChanged: {
        if (page.planShown) {
            page.closeDiff()
            // Down to the rail with the rest of the window's other business, and back up when the plan goes —
            // whichever way it goes (`foldForPlan`).
            page.foldForPlan(true)
        } else {
            page.foldForPlan(false)
        }
    }
    onPlanActiveChanged: {
        if (page.planActive) {
            // The graph lands on the plan's own newest row, whatever face was up before — the right pane
            // becomes that commit's, and the WIP face (whose commit button would sit under the run bar, and whose
            // own writes the freeze is for) cannot stay up under an open plan. `activateRow` also puts away a
            // standing row question and a name box, the way any deliberate click does. The plan's *own* selection
            // is already on that row: the model opens it there, because writing it from here would fire the
            // model's one `changed()` inside the very binding delivering it (`RebasePlanModel::take`).
            page.activateRow(planModel.expectHead)
        } else {
            // Discarded, stale, run, or put away under a standing operation — the plan is gone either way, and so is
            // the row that was going to carry whatever a `reword` left in the right pane's boxes. Nothing moved the
            // commit on screen, so the pane's own guard would sit still (`DetailsPane.syncMessage`) and leave that
            // text where the plain amend this hands back can reach it: on the newest commit that is a one-press
            // rewrite of the history the plan never ran, and on any other row it is one commit's message shown under
            // another's. The commit's own message goes back in.
            detailsPane.dropDraft()
        }
    }
    // What the range has already been sent of moves with the remote-tracking refs, and fetch is the one write the
    // freeze leaves running — so the count is asked again whenever the refs actually move (`refsMoved`, not the
    // every-tick `refsSettled` — that would spawn a rev-list at the status rate), and the run button's amber
    // follows the fetch instead of freezing at the plan's opening (規約 §フル interactive rebase). The plan asks for
    // itself and reads its own answer by range, so no other question can take the answer's place.
    Connections {
        target: branchesModel
        function onRefsMoved() {
            planModel.refreshPushed()
        }
    }
    Connections {
        target: planModel
        // Armed by the answer that a run actually went out, not by the button being pressed: `runPlan` turns away a
        // plan that asks for nothing, and a hold armed for a write that was never sent would never be let go of.
        function onPlanRan() {
            page.planRunOut = true
        }
        // Three ways a range cannot be replayed. Only the kind travels — the words are this end's, because git was
        // never run (app-ui.md「Rust に文言を置かない」). **The line under the heading is not this file's**: the row
        // menu turns down the same three histories and says them the same way (`Words.rewriteRefusedWhy`). What
        // stays here is what the two surfaces do not share — a heading about the range rather than about the one
        // commit that was pressed, and `warning` rather than `danger`, because the plan is a gesture still going
        // (規約 §答えの要らない報せ).
        function onRefusedPlan(kind) {
            const why = Words.rewriteRefusedWhy(kind)
            if (kind === "across-merge")
                page.showNotice(qsTr("A merge is in the way"), why, "warning")
            else if (kind === "unfetched-base")
                page.showNotice(qsTr("The history stops here"), why, "warning")
            else
                page.showNotice(qsTr("Not on this branch"), why, "warning")
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
    // The tip and the standing operation as the status reads them, fed on every snapshot: the model compares the tip
    // against the one the plan opened on and puts a stale draft away itself, and an operation starting under the plan
    // puts it away outright — a merge stopped on a conflict leaves the tip where it was, so the tip alone cannot see
    // it. Only while one stands — fed to a shut model they would spend a borrow per tick saying nothing.
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
        // Whoever was dressing the bar lets go first: a question can be raised over one still standing (a ref clicked
        // in the left menu while the first push asks where it goes), and the flow left behind would go on calling
        // itself the one standing — which is what keeps its check timer firing round trips at a remote nobody is
        // asking about. Not for the dress: `startAsking`'s assignments beat a live `Binding` outright
        // (measured). The flow raising this one turns its own back on after this returns.
        publishFlow.publishAsking = false
        upstreamFlow.asking = false
        page.rowAskRun = run
        graphPane.startAsking(oidHex, label, detail, acceptText, danger, hold, tip, form, code, refName)
    }
    function stopRowAsk() {
        page.rowAskRun = null
        publishFlow.publishAsking = false
        upstreamFlow.asking = false
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
        // The move came back as a question instead: nothing moved, so nothing is on its way to the screen and the next
        // press — the one that answers the bar, or the one that raises it again after it is walked away from — goes.
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
    /// The branch card's row, from either menu it is carried by. Marked on the row that branch stands on, so the
    /// question is beside the thing it is about.
    function startUpstreamAsk(branch, counterpart) {
        upstreamFlow.startAsk(branch, branchesModel.oidOfName(branch), counterpart)
    }

    // ---- sending the branch to its remote ---------------------------
    PublishFlow {
        id: publishFlow
        repoTab: repoTab
        workTree: workTree
        remotesModel: remotesModel
        graphPane: graphPane
        // The first push asks where the branch goes, and it asks in the one bar every other question stands in.
        onAskRequested: (label, run, form, code, refName) =>
            page.startRowAsk("", label, "", false, "", run, false, "", form, code, refName)
    }
    /// What the window's toolbar reads off the page it is showing: the button lives up there, and the state machine
    /// behind it down here (`TopBar`).
    readonly property alias pushTargetLabel: publishFlow.pushTargetLabel
    readonly property alias pushState: publishFlow.pushState
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

    /// One of this page's right-click menus is standing. What the pointer is over then is the menu; the row it was
    /// opened on is behind it, and nothing behind a menu is being hovered — a card or a tip that comes out now is
    /// drawn over the very rows the hand is reading. The menus are all the page's, so this is
    /// the one place that can see all of them — and a menu nobody has raised yet is not standing.
    readonly property bool menuStanding:
        page.menuShowing(refMenuSeat) || page.menuShowing(commitMenuSeat) || page.menuShowing(fileMenuSeat)
        || page.menuShowing(remoteMenuSeat) || page.menuShowing(diffMenuSeat)
    function menuShowing(seat) {
        return seat.item !== null && seat.item.showing
    }

    /// Every seat on this page that is built on being asked for — the five menus — stands built from the start
    /// instead. Written from outside: the harness asks for this before its verbs read a menu's rows, since a null
    /// there is nothing to read rather than a refusal (`PageHarness`), and false wherever nobody wrote it, which is
    /// every page a person opens. The plan's face and the command log are not in this: their verbs raise them the
    /// way a hand does, and read them only once they are up.
    property bool keepBuilt: false

    // ---- context menu on a sidebar row ------------------------------
    // Each menu is built the first time it is raised, not with the page: five cards of rows, held ready, are working
    // set a page pays for whether or not a hand ever comes (rules-refs/app-ui.md — the same rule the window's two
    // dialogs follow, `WindowDialogSeat`). The door activates the seat and then calls into it, which is synchronous;
    // the seat fills the page so the menu inside measures the window the way it always did (`AppMenu.ownerItem`).
    Loader {
        id: refMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: RefRowMenu {
            // Every row of this menu moves something — a switch, an integrate, a name taken away — so the whole card
            // is held while the doors are, wherever it was raised from. **The chip on a graph row is the same door**:
            // one menu answers for both entrances, and a `switch` that stayed live over there would be the very move
            // the left pane just closed (デザイン規約 §メニュー: 入口が違っても同じ操作は同じ文).
            heldReason: page.doorsHeldWhy
            repoTab: repoTab
            workTree: workTree
            graphModel: graphModel
            branchesModel: branchesModel
            worktreesModel: worktreesModel
            tagsModel: tagsModel
            onSwitchRequested: (kindLetter, name) => page.switchToRef(kindLetter, name)
            onBranchHereRequested: oidHex => page.startBranchAt(oidHex)
            onTagHereRequested: oidHex => page.startTagAt(oidHex)
            onDeleteRequested: (kind, id, name, oidHex) => page.deleteRow(kind, id, name, oidHex)
            onDropStashRequested: selector => page.dropStashNow(selector)
            onUpstreamRequested: (branch, counterpart) => page.startUpstreamAsk(branch, counterpart)
            onDeleting: (kind, id) => page.showGone(kind, id)
            // The settle re-run is for a menu that stood on the stacked list's row: the list stayed up under it, and
            // whether it stays now is the pointer's to answer again.
            onDismissed: rowHost.settleRefList()
        }
    }
    // What a remote itself offers. Its own menu rather than rows added to the one above: a remote is repository
    // configuration, and the ref menu is about refs (デザイン規約 §左メニューの所作).
    Loader {
        id: remoteMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: RemoteRowMenu {
            heldReason: page.doorsHeldWhy
            repoTab: repoTab
            onUrlRequested: name => publishFlow.startEditRemote(name)
            onDismissed: rowHost.settleRefList()
        }
    }
    /// The one door into that menu. Says whether it opened.
    function openRemoteMenu(name) {
        remoteMenuSeat.active = true
        return remoteMenuSeat.item.offerOn(name)
    }

    /// Which surface raised the standing ref menu. Its branch row opens a box to type a name in, and that box belongs
    /// on the row the hand is already on — the same reason the box is in the chip column and not over the window
    /// (デザイン規約 §可否・警告の出し場所).
    property bool refMenuInSidebar: false
    /// The one door into that menu: the sidebar's rows, a chip, the stacked list and the automation all come through
    /// here. Says whether it opened.
    function openRefMenu(kind, name, full, oidHex, inSidebar) {
        page.refMenuInSidebar = inSidebar === true
        refMenuSeat.active = true
        return refMenuSeat.item.offerOn(kind, name, full, oidHex)
    }
    /// A new branch on a commit, asked for from a menu: the name box opens where that menu was raised. Nothing is
    /// created until it is submitted — walking away costs the typing and nothing else. The sidebar's half reads the
    /// row off the ref menu, which is standing whenever that half is taken: only its door sets `refMenuInSidebar`.
    function startBranchAt(oidHex) {
        if (oidHex === "")
            return
        if (page.refMenuInSidebar)
            sidebarPane.beginBranchAt(refMenuSeat.item.kind, refMenuSeat.item.refId, oidHex)
        else
            graphPane.startNaming(oidHex)
    }
    /// And a tag on that commit, through the same two doors and on the same terms.
    function startTagAt(oidHex) {
        if (oidHex === "")
            return
        if (page.refMenuInSidebar)
            sidebarPane.beginTagAt(refMenuSeat.item.kind, refMenuSeat.item.refId, oidHex)
        else
            graphPane.startTagging(oidHex)
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
            repoTab.renameTag(id, name)
        } else if (kind === "stash") {
            repoTab.renameStash(id, name)
        } else if (kind === "remote") {
            // **No write goes out here** — this one is asked first, and the question carries both names from now on
            // (`askRenameRemote`). So the box that sent it has nothing to wait for and comes down, where every other
            // rename keeps it until git answers (デザイン規約 §答えの要らない報せ). Left waiting, a dismissed question would
            // leave it standing over the sidebar with nothing coming.
            page.noteRenameLanded()
            page.askRenameRemote(id, name)
        }
    }

    /// The remote branch a just-renamed local one spoke for, and the name it took — the question about carrying the
    /// name over waits until git says the local rename landed.
    property string pendingRenameRemote: ""
    property string pendingRenameTo: ""

    /// A rename's box stays open until git answers, and these are the two words back to it. **Both boxes are asked**
    /// and only the one that was waiting acts: a rename comes from the left menu's row or from the chip on the graph,
    /// and neither knows about the other (デザイン規約 §答えの要らない報せ).
    function noteRenameLanded() {
        sidebarPane.noteRenameLanded()
        graphPane.renameLanded()
    }
    function noteRenameRefused(why) {
        sidebarPane.noteRenameRefused(why)
        graphPane.renameRefused(why)
    }

    /// Renaming a branch on a remote, which git has no command for: core pushes the new name and deletes the old, so
    /// the question is asked first and its answer is held down rather than clicked — this is the one write here that
    /// another machine keeps (デザイン規約 §長押し).
    function askRenameRemote(remoteRef, name) {
        // The cut is the configured remote name where one owns the ref (a remote's own name may contain `/`), the
        // first slash otherwise — either way the question still fires (`GitFacts.remoteOfRef`).
        const remote = GitFacts.remoteOfRef(remoteRef, repoTab.remoteNames)
        if (remote === "")
            return
        const from = GitFacts.branchOfRef(remoteRef, repoTab.remoteNames)
        // A name already over there is not offered: a plain push to one that exists fast-forwards it and reports
        // success, so somebody else's branch would move instead of ours being renamed. The box refuses it too; this
        // catches the way in that has no box.
        if (name === from || remotesModel.oidOfName(remote + "/" + name) !== "")
            return
        page.startRowAsk(
            remotesModel.oidOfName(remoteRef),
            qsTr("Rename %1 to %2?").arg(remoteRef).arg(remote + "/" + name),
            qsTr("The old branch is deleted, not moved."),
            false,
            qsTr("Rename"),
            function () { repoTab.renameRemoteBranch(remote, from, name) },
            true,
            qsTr("Hold to rename. git has no rename on a remote: %1 is pushed, then %2 is deleted. Anything the old name carried — an open pull request, a running check — does not follow it.").arg(remote + "/" + name).arg(remoteRef))
        page.renameRemoteAsked(remoteRef, name)
    }
    /// Automation: the question above was raised, and the two names it is between. An automation-only exposure, the
    /// same one `GraphPane.view` is (app-ui.md).
    signal renameRemoteAsked(string from, string to)

    /// Refusals this page already has an answer for. The question bar explains them, so the command log stays where it
    /// was rather than raising itself over the same news (デザイン規約 §git が言ったことを読む場所). Counted rather than flagged
    /// because the refusal and the write result arrive on separate paths, in no fixed order.
    property int expectedRefusals: 0
    /// The plain delete's landing, read the way the card reads it (`RefBranchMenu`): the refusal armed above is not
    /// coming, so the credit goes back. An edge rather than a check on the answer in hand — a fetch answering in the
    /// same drain leaves the group saying fetch, and the landing would never be seen.
    readonly property string deleteLanded: repoTab.branchDeleteLanded
    onDeleteLandedChanged: {
        if (page.deleteLanded !== "")
            page.expectedRefusals = Math.max(0, page.expectedRefusals - 1)
    }
    /// The same, for the failures a report has already answered — armed by the report when the row it is about has
    /// not reached the log yet (`absorbWriteResult`), spent by that row's own arrival.
    property int answeredFailures: 0

    // ---- what the window is already showing as gone ------------------
    //
    // **A delete takes the row away at the press, and git is asked behind it** (デザイン規約 §消す操作は先に画面から消す).
    // The write itself is the short half: `git branch -d` is one process, and everything the reader is actually
    // waiting on comes after it — the refs read, then the walk that rebuilds the graph (kotlin 級で 54ms のあとに
    // 1.3s、`busyCount` はその先頭しか覆わない — measured). Left to those, the row sits there through all of it
    // with nothing to say whether the press even landed.
    //
    // One key per kind, because a delete touches at most one of each; `Delete both` is the one that touches two.
    // Empty means nothing is being shown as gone, which is also what a refusal goes back to.
    property string goneBranch: ""
    property string goneRemote: ""
    property string goneTag: ""
    property string goneStash: ""
    /// The write count the rows were taken away at. **The reading that puts them back has to be one that came after
    /// the write answered** — a listing already queued when the press landed says nothing about the delete, and read
    /// as though it did it would put the row straight back under the hand that had just taken it away.
    property int goneAtSeq: -1
    /// The chips that go with the rows are the graph model's to key and pack (`GraphModel.setGone` /
    /// `encode::gone_keys`): the names cross the bridge as they are. A dropped stash has no chip: it is a row of the
    /// graph rather than a name on one, and a row only leaves with the walk.
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

    /// Automation: the run that photographs a row already gone holds it there.
    ///
    /// What it photographs is the in-between — the row taken away, git not yet answered for it — and a demo
    /// repository answers in tens of milliseconds, which is over before the picture is grabbed. Asked for before the
    /// press rather than off it, so what holds the row up is a standing decision and not a second edge to get right
    /// (verify-ui スキル §壊れない動詞の実装と反復).
    property bool holdGoneRows: false

    /// Takes a row away before git has answered for it. `id` is what git is being asked to delete, which is also what
    /// the row is keyed by — so a refusal puts back exactly what was taken.
    function showGone(kind, id) {
        page.goneAtSeq = repoTab.writeSeq
        if (kind === "branch")
            page.goneBranch = id
        else if (kind === "remote")
            page.goneRemote = id
        else if (kind === "tag")
            page.goneTag = id
        else if (kind === "stash")
            page.goneStash = id
        page.goneShown(kind, id)
    }
    /// Automation: a row was stood in for, and which one. An automation-only exposure, the same one `GraphPane.view`
    /// is (app-ui.md) — the four `gone*` names below say what is standing in, not that this is the moment it started.
    signal goneShown(string kind, string id)
    /// Puts every one of them back: git refused, and what it refused is still there.
    ///
    /// **Only for the answer to the write that took them away** — `goneAtSeq` is read before the write goes out, so
    /// its own answer is the next one. The queue is serial across the refreshes as well (`session::write` holds
    /// `write_busy` around the whole request), so the only thing that can still be standing when a later write
    /// answers is the stash: its listing is asked for after the request returns rather than awaited inside it, and a
    /// refusal read as this one's would put a dropped stash back while its own listing was still in flight.
    ///
    /// **What it cannot tell apart is the composite.** `Delete both` deletes locally and then pushes, and a remote
    /// half that failed answers with the same one error as a local half git would not do — so a landed local delete
    /// is put back here too, until the refs read takes it away again (rules-refs/app-ui.md).
    function showBack() {
        if (repoTab.writeSeq !== page.goneAtSeq + 1)
            return
        page.goneBranch = ""
        page.goneRemote = ""
        page.goneTag = ""
        page.goneStash = ""
    }
    /// Whether a listing arriving now is one that can answer for the delete.
    readonly property bool goneAnswered:
        page.goneAtSeq >= 0 && repoTab.writeSeq > page.goneAtSeq && !page.holdGoneRows
    /// The refs the delete moved have arrived, so what the sidebar and the chips now hold is the truth — whichever
    /// way it went, nothing is being stood in for any more. **The stash is not one of them**: its listing is asked
    /// for after the graph is rebuilt rather than beside the refs (`session::write`), so it has a word of its own.
    function refsProvedGone() {
        if (!page.goneAnswered)
            return
        page.goneBranch = ""
        page.goneRemote = ""
        page.goneTag = ""
    }
    function stashesProvedGone() {
        if (page.goneAnswered)
            page.goneStash = ""
    }

    /// Whether the delete this page was last asked for went out to git, or was dropped before it did. **Both
    /// entrances end here** — the left row's card and the graph row's — and a signal carries no answer back to
    /// either, so the answer stands here. The automation reads it as "did the input land"
    /// (app-ui.md §UI 自動化の因果性), the same question `RefBranchMenu.deleteAsked` answers for the card: a request
    /// this page dropped leaves nothing for a write barrier to wait on, and waiting on it is the watchdog's whole
    /// ceiling in silence. Nothing on screen needs it.
    property bool deleteRowAsked: false
    function deleteRow(kind, id, name, oidHex) {
        page.deleteRowAsked = false
        if (kind !== "branch")
            return
        // The one row in any menu that outlives its own write (it stays open for git's answer), so it is also the one
        // that can be clicked twice — the second click is the same request again.
        if (repoTab.busyCount > 0)
            return
        // A branch is deleted with `-d`, and git's refusal is the question — asked when it arrives rather than
        // guessed at beforehand (デザイン規約 §左メニューの所作). Which branch the answer is about is the tab's to keep,
        // and the card that asked reads it there (`RefBranchMenu`).
        page.expectedRefusals++
        page.showGone("branch", id)
        repoTab.deleteBranch(id, false)
        page.deleteRowAsked = true
    }
    /// Held, not asked (デザイン規約 §長押し).
    function dropStashNow(ref) {
        page.showGone("stash", ref)
        repoTab.dropStash(ref)
        if (page.selectedStashRef === ref)
            page.selectedStashRef = ""
    }

    // ---- context menu on a working-tree file row --------------------
    function openFileMenu(bucket, path) {
        // A right-click is a click: it walks away from a question that was standing, which may well be about another
        // row.
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
    /// The one door into that menu. What it will act on is the selection, and the hand settled that before asking
    /// (`DiffTextSelect.askMenu`), so nothing about the row travels here.
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
    /// The name a graph row draws, in the words the chip records use — its first one, whatever kind it is, which is
    /// what the row hands over when a hand presses it (`GraphRowDelegate.renameRecord`). Asked of the model for the
    /// callers that have no row in hand. The records already taken off the screen ahead of git's answer are filtered
    /// out the same way the delegate filters them, so a name that is gone does not put a card up over a ref that is
    /// not there any more (デザイン規約 §消す操作は先に画面から消す).
    function rowRecordAt(oidHex) {
        const row = graphModel.rowOf(oidHex)
        if (row < 0)
            return ""
        const shown = GitFacts.labelsShown(graphModel.labelsAt(row), graphModel.goneChips)
        if (shown === "")
            return ""
        const first = shown.split(String.fromCharCode(31))[0]
        return GitFacts.recordKind(first) === "" ? "" : first
    }

    /// The one door into that menu: the graph's rows wherever they are pressed, the rows of the stacked list a chip
    /// unfolds into, and the automation all come through here. `record` is the name the menu is aimed at — the one
    /// the chip draws, or the one pressed in the list — and empty aims it at nothing, which is a row that draws no
    /// name at all. **The first level does not move with it**: it is the same commit either way, so what a naming in
    /// the list changes is which card comes up (デザイン規約 §グラフ行の右クリック). Left out, the row's own name is
    /// asked of the model, which is what the callers with no row in hand do.
    function openRowMenu(oidHex, record) {
        const named = record === undefined ? page.rowRecordAt(oidHex) : record
        commitMenuSeat.active = true
        const menu = commitMenuSeat.item
        menu.targetKind = GitFacts.recordKind(named)
        menu.targetName = menu.targetKind === "" ? "" : GitFacts.recordName(named)
        commitMenuState.openRowMenu(oidHex)
    }

    CommitMenuState {
        id: commitMenuState
        repoTab: repoTab
        workTree: workTree
        graphModel: graphModel
        worktreesModel: worktreesModel
        // Null until the menu is first raised; the one function that reads it is the door that raises it.
        menu: commitMenuSeat.item
    }

    Loader {
        id: commitMenuSeat
        anchors.fill: parent
        active: page.keepBuilt
        sourceComponent: CommitRowMenu {
            // The graph's rows are doors onto the same history the left pane's are, so they are held on the same
            // answer: a reset or a drop let go into the middle of a replay is the same accident a switch would be.
            heldReason: page.doorsHeldWhy
            repoTab: repoTab
            workTree: workTree
            graphModel: graphModel
            branchesModel: branchesModel
            worktreesModel: worktreesModel
            tagsModel: tagsModel
            branch: workTree.branch
            oid: commitMenuState.menuOid
            stashRef: commitMenuState.menuStashRef
            published: commitMenuState.menuPublished
            canSwitch: commitMenuState.menuCanSwitch
            switchAsks: commitMenuState.menuSwitchAsks
            canSequence: commitMenuState.menuCanSequence
            canIntegrate: commitMenuState.menuCanIntegrate
            canEditHistory: commitMenuState.menuCanEditHistory
            canMoveBranch: commitMenuState.menuCanMoveBranch
            canBranchHere: commitMenuState.menuCanBranchHere
            stashCanWrite: commitMenuState.menuStashCanWrite
            hardResetTakes: commitMenuState.menuHardResetTakes
            // The same road the row's double-click takes, held on the same answers (`switchToRef`).
            onSwitchRequested: (kindLetter, name) => page.switchToRef(kindLetter, name)
            // Straight to the graph row: this menu is only ever raised on one.
            onBranchHereRequested: oidHex => graphPane.startNaming(oidHex)
            onTagHereRequested: oidHex => graphPane.startTagging(oidHex)
            onSquashRequested: oidHex => page.squashCommit(oidHex)
            onDropRequested: oidHex => page.dropCommit(oidHex)
            onPlanRequested: oidHex => page.startRebasePlan(oidHex)
            onResetRequested: mode => page.moveBranchHere(mode)
            onApplyStashRequested: selector => repoTab.applyStash(selector)
            onPopStashRequested: selector => page.popStash(selector)
            onDropStashRequested: selector => page.dropStashNow(selector)
            // The branch card's own three, answered exactly where the ref menu's are.
            onDeleteRequested: (kind, id, name, oidHex) => page.deleteRow(kind, id, name, oidHex)
            onDeleting: (kind, id) => page.showGone(kind, id)
            onUpstreamRequested: (branch, counterpart) => page.startUpstreamAsk(branch, counterpart)
            // The settle re-run is for a menu that stood on the stacked list's row: the list stayed up under it, and
            // whether it stays now is the pointer's to answer again.
            onDismissed: rowHost.settleRefList()
        }
    }

    ClipboardHelper {
        id: clipboard
    }

    // ---- the second click, spaced, on a graph row -------------------
    /// The name on the row's chip goes into a box where the chip is (デザイン規約 §グラフ行のダブルクリック / §左メニューの所作 — the
    /// gesture and the box are the sidebar's, and this is the other place a ref name is on screen). **One way in for
    /// both halves of the row**: the click that lands on the row itself, and the one that lands on the card the chip
    /// unfolds into, which is standing on that same chip.
    ///
    /// What is typed is the ref's own name — a remote branch without the remote it is on, the shape `renameRow` and
    /// the sidebar's box both take it in.
    function startRename(oidHex, record) {
        if (repoTab.state !== "open" || record === "")
            return
        const kind = GitFacts.recordKind(record)
        // The detached-HEAD marker names no ref, so there is nothing here to be renamed.
        if (kind === "")
            return
        const id = GitFacts.recordName(record)
        // The card the chip unfolds into is standing on the column the box opens in.
        rowHost.closeRefList()
        graphPane.startRenaming(oidHex, kind, id,
                                kind === "remote" ? GitFacts.branchOfRef(id, repoTab.remoteNames) : id)
        // **A box always opens where it can be seen** (デザイン規約 §左メニューの所作「開く行は必ず見える所へ送る」): the wait this
        // gesture opens is long enough to scroll away in, and a name changing itself off screen is a name nobody
        // agreed to. A row the walk no longer has is simply not sent anywhere.
        const row = graphModel.rowOf(oidHex)
        if (row >= 0)
            graphPane.showRowSoon(row)
    }
    /// Whether what is in the graph's name box can be accepted, and the one line that says why not. The rules are
    /// git's own, asked of core, and the models that answer "is that name taken" are here — the pane only draws it.
    /// **Only a rename is refused**: a box that opened empty to make a name has not been answered yet, and a frame
    /// that comes up already turned down is turning down the reader's arrival (§可否・警告の出し場所, `SidebarRowGestures`).
    readonly property string graphRenameRemote: graphPane.namingKind !== "remote" ? ""
        : GitFacts.remoteOfRef(graphPane.namingId, repoTab.remoteNames)
    readonly property bool graphNameTaken: graphPane.namingMode === "rename" && page.graphRenameRemote !== ""
        && graphPane.namingText.trim() !== ""
        && remotesModel.oidOfName(page.graphRenameRemote + "/" + graphPane.namingText.trim()) !== ""
    readonly property string graphNameRefusedWhy: {
        if (graphPane.namingOid === "" || graphPane.namingMode !== "rename")
            return ""
        // git was asked about this very name and answered; everything below is what this end works out before asking.
        if (graphPane.namingGitRefusal !== "")
            return graphPane.namingGitRefusal
        const typed = graphPane.namingText
        if (typed.trim() === "")
            return qsTr("A name is needed")
        // The same rule the left menu's box asks (`SidebarRowGestures.editCaseOnly`): a tag renamed to its own name in
        // other letters takes both names with it on a case-insensitive disk.
        if (graphPane.namingKind === "tag" && typed.trim() !== graphPane.namingId
            && typed.trim().toLowerCase() === graphPane.namingId.toLowerCase())
            return qsTr("Only the letter case differs — on this disk that deletes both names")
        if (page.graphNameTaken)
            return qsTr("%1 already has a branch called that").arg(page.graphRenameRemote)
        return GitFacts.validRefName(typed) ? "" : qsTr("git will not take this as a name")
    }

    // ---- double-click on a graph row -------------------------------
    // A row with no chip is offered one instead of doing nothing.
    function rowDoubleClicked(oidHex, record) {
        if (repoTab.state !== "open")
            return
        if (record !== "") {
            page.activateRecord(record)
            return
        }
        // The other half of the same door, and held on the same answer as the switch above it (`switchToRef`): the box
        // is a branch about to be written, and the run of a plan is out before the count has anything to say.
        if (repoTab.busyCount === 0 && page.doorsHeldWhy === "")
            graphPane.startNaming(oidHex)
    }

    // What a resting pointer opens on a graph row: the row's own card, and the refs one chip had to stack. Owned here
    // because rows are recycled out from under both of them.
    RowHoverHost {
        id: rowHost
        graphPane: graphPane
        currentBranch: workTree.branch
        menuStanding: commitMenuSeat.item !== null && commitMenuSeat.item.opened
        hoverBlocked: page.menuStanding
        onRecordActivated: record => page.activateRecord(record)
        // A plain click in the card is a click on the row it is standing on: every name in it is on that one commit
        // (a held one never comes this way — the card hands it to the graph's row itself).
        onRecordChosen: (oidHex, atRow) => page.activateRow(oidHex, atRow)
        // The note under a message the card had to cut: the row's own click, made from inside the card, and then the
        // pane that holds the whole of it says so — the reader was sent somewhere and the sentence they read is gone
        // by the time they arrive (デザイン規約 §hover のツールチップ).
        onMessageAsked: (oidHex, atRow) => {
            page.activateRow(oidHex, atRow)
            detailsPane.callAttention()
        }
        // A right-click on one of the card's rows raises the row's own menu, aimed at the name that was pressed.
        onRecordMenuAsked: (oidHex, record) => page.openRowMenu(oidHex, record)
    }

    // ---- rewriting one commit --------------------------------------
    // Not confirmed even for a commit a remote already has: nothing here leaves the machine, and the push that would
    // spread it is asked about on its own.
    function squashCommit(oidHex) {
        repoTab.squashIntoParent(oidHex)
    }

    /// Leaves the commit out of the history. One place for both ways in: the row is a hold or a click depending on
    /// whether anything else still holds the branch tip, and what it runs must not depend on which of the two the
    /// reader got (デザイン規約 §履歴を合流させる).
    function dropCommit(oidHex) {
        repoTab.dropCommit(oidHex)
    }

    // ---- taking the branch back to an earlier commit ----------------
    function moveBranchHere(mode) {
        repoTab.resetTo(commitMenuState.menuOid, mode)
    }

    // ---- editing the selected commit's message ---------------------
    // No confirmation, even for a commit a remote already has: this rewrites nothing that a switch or a reset cannot
    // bring back, and the push that would spread it is asked about on its own.
    function saveMessage(oidHex, subject, body) {
        // Where the row sits now. A reword leaves the shape of the history alone, so the rewritten commit lands on the
        // same row and the selection can follow it there.
        page.rewordRow = graphModel.rowOf(oidHex)
        repoTab.rewordCommit(oidHex, subject, body)
    }
    // Row to re-select once the rewritten graph arrives (-1 = none).
    property int rewordRow: -1

    // What the details pane's boxes do with the commit on screen, in core's own word (`offers::message_edit`):
    // `amend` where typing lands, `stash` / `not-head` / `standing` where it does not, `""` where there is no message.
    // **Only HEAD's own commit takes it** — everything else would be replayed, and this is a text box a click can land
    // a caret in (デザイン規約 §コミットメッセージの 2 つの枠). A binding rather than an answer asked per selection: the
    // boxes stand open while the repository moves under them, so a commit that stops being HEAD's stops taking typing.
    readonly property string messageEdit: GitFacts.messageEdit(
        !page.blank && repoTab.state === "open", detailsModel.shaHex, workTree.headOid,
        page.selectedStashRef, workTree.opText, workTree.opEditing)

    // What git makes of the selected commit's signature. Asked on every selection, and read only when the answer names
    // the commit now on screen — verifying runs gpg or ssh-keygen, so the answer arrives well after the details do.
    // A signature only changes when the commit does, and a changed commit is a different hash, so nothing has to ask
    // twice.
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

    // Whether a remote already has the selected commit — what the save row's warning rests on. Only HEAD's own commit
    // takes typing (`messageEdit`), so the answer is HEAD's own (`WorkTreeModel.headPublished`), read while the
    // selection stands on it; under a plan the reword chip carries the plan's own warning instead. Nothing is asked
    // on the keystroke: the answer rides beside HEAD, and a plan standing over the boxes cannot have spent it.
    readonly property bool selectedPublished: !page.planActive && page.selectedOid !== ""
                                              && page.selectedOid === workTree.headOid && workTree.headPublished

    // **Moving off a half-written message drops it, and nothing asks** (デザイン規約 §コミットメッセージの 2 つの枠).
    // A message that has not been saved is a draft of somebody else's commit; the way to keep it is to press the
    // button, and the two ways to drop it — Escape, and reading another commit — are both things the reader did on
    // purpose. So nothing stands between a click in the graph and the commit it lands on (`DetailsPane.syncMessage`).

    // An assignment is not something git knows about, so nothing here is waiting for a refresh to bring it: the rows
    // and the card re-read the store themselves.
    Connections {
        target: AppBackend
        function onAvatarsChanged() {
            graphModel.refreshAvatars()
            detailsModel.refreshAvatar()
            // The commit editor wears the identity's own face, which is reached by the same badge.
            repoTab.refreshAvatar()
            // And the list of what is held, whose faces are packed off the rows.
            page.refreshChosenRecords()
        }
    }

    // ---- the harness -----------------------------------------------
    // The whole of this tab's verification harness, which a shipped build does not carry (`HarnessSeat`). Everything
    // the verbs and the measurements act on is handed over here — a module of its own cannot see this one's ids — and
    // naming them is an automation-only exposure, the same one `GraphPane.view` is (app-ui.md). The menus and the
    // plan's face go over as the seats they are built in rather than as themselves: none exists until it is asked
    // for, and asking is the harness's to do (`keepBuilt`).
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
            stashesModel: stashesModel,
            tagsModel: tagsModel,
            graphPane: graphPane,
            sidebarPane: sidebarPane,
            detailsPane: detailsPane,
            diffPane: diffPane,
            wipPane: wipPane,
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
    // A write that did not happen, and whoever said no. It comes down over the middle of the page — over whichever of
    // the graph and the diff is showing — and goes back up on the one word it offers (デザイン規約 §答えの要らない報せ).

    /// Raises the report for one write that did not happen — **the one door**, so a headless run that hands it a kind
    /// enters the same body the answer does (verify-ui). `reason` is what whoever said no wrote; empty where this end
    /// refused the write itself, and then the second line is ours (`Words.writeReportedWhy`).
    function showReport(kind, remote, name, reason) {
        page.showNotice(Words.writeReported(kind, remote, name),
                        reason !== "" ? reason : Words.writeReportedWhy(kind),
                        Words.reportTone(kind))
    }
    /// Automation: git's answer to a write has been taken all the way — the bar raised, the mark taken down, the
    /// standing questions cleared. An automation-only exposure, the same one `GraphPane.view` is (app-ui.md); what the
    /// write was about is still on `repoTab.writeReport*` when this goes out.
    signal writeReported()
    /// Raises the report. `label` is what did not happen, `detail` whoever said no in their own words, `tone` the
    /// state it is in if it is in one at all.
    function showNotice(label, detail, tone) {
        // Dressed, then raised — so nothing on a bar the reader can see is ever written (`NoticeBar.open`).
        noticeBar.label = label
        noticeBar.detail = detail
        noticeBar.tone = tone
        noticeBar.open = true
    }
    /// Only lowered: the words stay where they are for the 200ms it spends going up.
    function hideNotice() {
        noticeBar.open = false
    }
    /// The bar itself — automation-only exposure, like `GraphPane.askCard` (app-ui.md). A headless run reads
    /// `settled` / `shut` / `label` off it and presses its one control through `dismiss()`.
    readonly property alias noticeCard: noticeBar
    /// Automation: whether the middle of the page is standing **under** the report rather than behind it — the whole
    /// of what moving the bar out of the graph was for, and the one thing about it a picture cannot settle (a bar
    /// drawn over a pane frames exactly like a bar the pane was moved down for, since what it covers is the pane's
    /// own top edge either way). Read in page coordinates, so it holds whichever of the two panes is showing.
    readonly property bool noticeClears:
        centreStack.mapToItem(page, 0, 0).y >= noticeBar.mapToItem(page, 0, noticeBar.height).y

    // A finished write the editor asked for: clear it only once git says the commit landed, so a rejected one keeps its
    // text.
    property int seenWriteSeq: 0
    // What an answer *means* is settled on the tab, where the op names are known (`drain::settle_write`); this
    // function only sequences the screen off those classified properties — reads, landings, menus.
    function absorbWriteResult() {
        if (repoTab.writeSeq === page.seenWriteSeq)
            return
        page.seenWriteSeq = repoTab.writeSeq
        // The press has its answer. What is left of the wait is the read, which says so itself (`diffSettling`).
        page.diffAwaits = false
        // A push this button sent has come back; what it means for the toolbar's button is the flow's to work out.
        publishFlow.noteWriteAnswer()
        // A pop that did not happen leaves its entry, and its name, where they were — so this is read on both
        // landings, above the refusal branch and its early returns.
        page.absorbPopLabel()
        // Whether the working tree emptying next is this window's own doing — only a stash can empty a tree, so the
        // count arriving at zero is what says it was one, and this only says whose. Read off every answer rather than
        // armed and cleared, so nothing can be left standing for a later write to trip over; a refusal writes `false`
        // the same way.
        page.stashLanded = repoTab.writeStashed
        if (repoTab.writeRefused) {
            // Nothing moved, so nothing is coming to the screen for a move to be recognised by: pressing again is the
            // reader's to do, and this is the one put-down `moveLanding` cannot wait for a landing for.
            page.moveLanding = ""
            // Whatever the window took away for this write is still there — git would not do it, or could not reach
            // the far side to. Put back before anything below answers for the refusal, so the row the question is
            // about is on screen when the question is (デザイン規約 §消す操作は先に画面から消す).
            page.showBack()
            // The rows on screen are not the file any more — drifted bytes are the one thing the fingerprint refuses
            // on, and the tally watch below cannot always catch the drift that caused it (an outside change that moves
            // no bucket count moves no tally), so left alone the same press would be refused again for as long as the
            // reader cared to try. The refusal's answer is the fresh file.
            if (repoTab.writeStaleDiff) {
                page.diffReadAt = repoTab.writeSeq
                page.reloadDiff()
            }
            // The write did not happen and something outside this application said so — a protected branch, a hook
            // over there or here, a remote this end had only an older picture of. Nothing here could have known
            // beforehand and nothing here can answer it, so what it said comes down as a report and the log stays
            // where the reader left it (デザイン規約 §答えの要らない報せ). Ahead of the branch delete's own second move:
            // `Delete both` is a branch write whose remote half is what the far side refused, and the row it would
            // morph is about the half that landed.
            // Anything but the name itself being turned down leaves the box nothing to answer, so it comes down the
            // way a landing takes it down — a half-finished rename most of all, where the row it was on is exactly
            // what could not be found.
            if (repoTab.writeReportKind !== "rename")
                page.noteRenameLanded()
            if (repoTab.writeReportKind !== "") {
                // A name git would not take goes back into the box it was typed into as well as into the bar: the
                // box is still open, holding it (デザイン規約 §答えの要らない報せ). Whichever of the two boxes was waiting
                // answers; the other is not open and says nothing.
                if (repoTab.writeReportKind === "rename")
                    page.noteRenameRefused(repoTab.writeReportReason)
                page.showReport(repoTab.writeReportKind,
                                repoTab.writeReportRemote,
                                repoTab.writeReportName,
                                repoTab.writeReportReason)
                // …and the mark in the corner goes quiet with it: it is there to fetch somebody to a failure nothing
                // else has said, and the bar has just said this one (デザイン規約 §git が言ったことを読む場所). The row keeps
                // git's words under its red edge — that is the record, and the record is what the panel is for.
                //
                // **The row it is about may not have reached the log yet.** The write's answer and the command's own
                // end travel separate feeds, in no fixed order (`expectedRefusals` is counted for the same reason):
                // where the row has landed already, taking the mark down is the whole of it; where it has not, it puts
                // the mark back up when it does — and raises the log with it, over the news this bar is already giving
                // (measured, 1 Linux run in 5, and none of 5 on Windows — the picture is identical either way).
                if (!commandsModel.failed)
                    page.answeredFailures++
                commandsModel.noteAnswered()
                page.pendingRenameRemote = ""
                page.pendingRenameTo = ""
                page.writeReported()
                return
            }
            // The one refusal this page has a second move for — a branch delete git would not do on its own — is
            // answered where it was asked: the card that stayed up for it reads its own name off the tab and turns
            // its row into the held `-D` (`RefBranchMenu`). A refusal does not mean the commits stop being
            // reachable: git measures the branch against its upstream when it has one, so a branch merged into HEAD
            // but not yet pushed is refused while nothing at all would be lost (measured). Nothing to raise here —
            // the row is the answer. **This answer's, by its seq**: the name stands until the next delete is
            // asked, and a refusal of something else that comes later must still raise the log.
            if (repoTab.branchDeleteRefused !== "" && repoTab.branchDeleteSeq === repoTab.writeSeq) {
                page.pendingRenameRemote = ""
                page.pendingRenameTo = ""
                return
            }
            // Nothing else on screen says what git said, so the log comes up (デザイン規約 §git が言ったことを読む場所). Raised from the
            // answer rather than from the commands, because the ones that answer by their exit code do not raise it
            // themselves — and whether an operation built out of several of them failed is a question only its own
            // answer can settle.
            page.commandsOpen = true
            // A rename that did not happen has nothing to carry over.
            page.pendingRenameRemote = ""
            page.pendingRenameTo = ""
            return
        }
        // A plain delete that landed is not answered here: the card that stayed up for it goes by itself, and the
        // credit armed for its refusal goes back off the same edge (`deleteLanded`).
        //
        // The name went in, so the box that was holding it has done its job and comes down (デザイン規約 §答えの要らない報せ:
        // a rename keeps its box until git answers).
        //
        // **Only once the queue has drained.** The writes are serialised but a fetch queued ahead of the rename
        // answers first, and this branch cannot tell whose answer it is holding — closing the box on that one would
        // take it down before its own refusal arrived, leaving git's words nowhere to go. Nothing else can be in
        // flight when the rename's own answer lands, so `busyCount` is the gate.
        if (repoTab.busyCount === 0)
            page.noteRenameLanded()
        // The branch took its new name here; the remote it speaks for is still under the old one. Asked only now, and
        // only because there is a remote to ask about (デザイン規約 §左メニューの所作).
        if (repoTab.writeBranchOp && page.pendingRenameRemote !== "") {
            const spokenFor = page.pendingRenameRemote
            const took = page.pendingRenameTo
            page.pendingRenameRemote = ""
            page.pendingRenameTo = ""
            page.askRenameRemote(spokenFor, took)
        }
        if (repoTab.writeCommitted) {
            page.clearCommitEditor()
            wipPane.setAmendChecked(false)
            page.amending = false
        }
        // Where the answer sends the reader — **read out of the answers this notify carried, not off the group they
        // leave behind**. One drain empties the whole queue and notifies once (`RepoTab::write_answers`), so a write
        // *starting* in the same batch — the fetch that follows a run — takes the stop's flag back down before this
        // line is reached, and the reader is left on the graph where the conflicted rows should have been.
        //
        // Walked in order with the later answer winning: the two landings are exclusive — a stop leaves no commit at
        // the tip to go to — so the one armed is the one the last answer asked for.
        for (let i = 0; i < repoTab.writeAnswerCount(); i++) {
            // git stopped part-way and left the operation standing, so there is no commit at the tip to land on and
            // the answer to the press is the working tree: the conflicted rows, and the way out under them
            // (デザイン規約 §進行中の操作から出る). Armed rather than done on the spot, for the reason the head landing
            // below is: the status that will carry those rows has not arrived yet (by design).
            if (repoTab.writeAnswerStopped(i)) {
                page.pendingWipSelect = true
                page.pendingHeadSelect = false
                page.pendingHeadAsked = false
            }
            // The answer is a commit at the tip, and that commit is what was asked for here, not the row or the ref
            // that was clicked. The selection goes to it and the viewport follows: what was clicked can be anywhere
            // in the history, while the answer is always at the top.
            else if (repoTab.writeAnswerAtTip(i)) {
                page.pendingWipSelect = false
                page.pendingHeadSelect = true
                // The answer names the first report that may answer it (`RepoTab.writeAnswerHeadSeq`): the report
                // in hand may be the one before the write or already the one after it — the two feeds are drained
                // in no fixed order — and the number tells them apart where a count of reports could not.
                page.pendingHeadFromSeq = repoTab.writeAnswerHeadSeq(i)
                page.pendingHeadAsked = true
            }
        }
        // The report that answers the landing may already be in hand (above).
        page.tryPendingHeadSelect()
        // The write moved what the two sides hold, so a diff left open on either is a picture of a file as it was —
        // the same staleness the file list's own `+` used to leave behind.
        //
        // **Read here, where the answer is.** git has already moved the index by the time it answers, so the file's
        // diff is the new one — what has not caught up yet is the *file list*, and that is a different question
        // (`followEmptySide` asks it later). The status that follows would be the other place to read from, but it is
        // published only when it has rows to change, so a second line staged out of the same file would never be read
        // at all.
        //
        // Which write this was is remembered, so the status that follows does not read the same file over again.
        if (repoTab.writeStaleDiff) {
            page.diffReadAt = repoTab.writeSeq
            page.reloadDiff()
        }
        // The message landed: the editor stops offering to save it, and keeps what was written until the selection
        // catches up with the commit that now carries it.
        if (repoTab.writeReworded)
            detailsPane.noteMessageSaved()
        // Moving HEAD rewrites the working tree under the diff pane: the file it holds may not even exist where the
        // move landed, so the center goes back to the graph that was moved through. Taking the branch back does the
        // same to the file, and to which side of the index it sits on.
        if (repoTab.writeMovedHead)
            page.closeDiff()
    }

    // Center area switches between the graph and a file diff. The pieces are kept apart rather than parsed back out of
    // the key: a path may contain anything, colons included.
    property bool diffShown: false
    onDiffShownChanged: page.foldForDiff(page.diffShown)
    property string diffKey: ""
    property string diffKind: ""
    property string diffPath: ""
    property string diffOrigPath: ""
    // Whether the shown diff is a working-tree file (stageable).
    property bool diffFromWt: false
    readonly property bool diffStaged: page.diffKind === "staged"
    /// The two stage letters git reports for the file being shown, read once when it is opened: the pane is handed a
    /// path, not the row the path came from, and on a conflict git prints no patch for those letters are all there is
    /// to say. A file stops being conflicted only by a write, and every write reads the diff again.
    property string diffChange: ""
    /// The colour each side of a conflict is drawn in: the lane colour the graph gives that branch where it has one, so
    /// the diff borrows an answer rather than keeping a second set of its own.
    ///
    /// A binding, not a one-off read: the graph arrives in two passes (the chips land after the rows), and it is
    /// rebuilt whenever refs move. `finishCount` is read only to depend on it — every graph property shares one notify
    /// signal, so touching any of them is what makes this re-run when the rows change.
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
        // Not while a plan stands. The plan has the centre for the whole of its stay (`centreStack`), so a file read
        // here goes into a pane nobody can see — and it is still open when the plan is put away, which lands the
        // reader who came back for the graph on a file instead. **This is the only place the diff can be raised**, so
        // the fold, the neighbour and the read are all held by the one line.
        //
        // The file rows themselves stay live, because the right pane does (規約 §フル interactive rebase「右の詳細
        // ペインがそのまま生きる」): what is held is the one thing a press on them reaches past the pane to do. Silent,
        // like the mode's other refusals — the arrows are already inert here for a reason of their own (`readPath` is
        // empty with no diff open, and `FileRowWalk.stepFile` will not step from nowhere).
        if (page.planShown)
            return
        // Whatever place a rebuild of the last diff was keeping is the last diff's: restored here it would put the new
        // rows at the old file's scroll (規約 §diff を横へ送る「別のファイルは左端から」).
        diffPane.dropScroll()
        page.diffKey = kind + ":" + path
        page.diffKind = kind
        page.diffPath = path
        page.diffOrigPath = origPath
        page.diffFromWt = kind !== "commit"
        page.diffChange = kind === "conflicts" ? worktreeModel.changeOf(path) : ""
        // The list's light names the file being read. A click had already made this row the whole of the choice, so
        // what this catches is the pane moving itself — the commit's list needs no such line, its light *is* the path
        // being read (`DetailsPane.readPath`).
        if (kind !== "commit")
            wipPane.readOne(kind, path)
        if (kind === "commit")
            page.askCommitDiff(path, origPath)
        else
            diffModel.requestWorkTree(kind, path, origPath)
        page.diffShown = true
        page.diffNeighbour = ""
        page.noteDiffNeighbour()
    }
    /// A pointer came to rest on a commit, or left one. **Two lists reach this**: the graph's rows and the ones the
    /// right pane lists under a choice (デザイン規約 §複数のコミットを選ぶ). One card between them — what it holds back
    /// depends on what the row it came off had already shown (`RowHoverHost.openRowCard`).
    function restOnCommit(row, inside) {
        rowHost.rowCardWanted = inside
        if (inside)
            rowHost.openRowCard(row)
        else
            rowHost.settleRowCard()
    }
    /// The patch behind a row of the commit pane's list, in the three shapes that list comes in
    /// (デザイン規約 §複数のコミットを選ぶ): one commit's own change to the file, what differs between exactly two, or —
    /// for a merged list — what **each** of the chosen commits did to it, one block after another.
    ///
    /// **The last is not a comparison across the span.** The list above it is what these commits did, so the patches
    /// behind a row of it are theirs: a diff of the two ends would carry whatever unchosen commits stand in between.
    function askCommitDiff(path, origPath) {
        if (detailsModel.selectionCount > 2) {
            diffModel.requestChoiceFile(page.chosenPacked, path, origPath)
            return
        }
        if (detailsModel.comparing) {
            diffModel.requestRangeFile(detailsModel.compareFrom, detailsModel.compareTo, path, origPath)
            return
        }
        diffModel.requestCommitFile(detailsModel.shaHex, detailsModel.parentHex, path, origPath)
    }
    // Stages (or unstages) one hunk, or one line of it. The indices address the diff currently on screen, so the pane
    // is reloaded afterwards: once the patch is applied the rows have moved.
    function stageSelection(hunk, line) {
        // The shown diff's fingerprint rides along: the write refuses to apply the indices to bytes that drifted since
        // this was read.
        //
        // Said here rather than left to `busyCount`, which rises when the queue starts the write rather than when the
        // press is made: two presses in a row both went out before the first had begun.
        //
        // Armed by the answer, not by the asking: the slot says whether a write actually went out, and a request it
        // turned away — nothing selected, no fingerprint to address — has no answer coming, so a wait armed for it
        // would hold the marks for good (the same wedge the tree-read wait had, through the request's own door).
        page.diffAwaits = repoTab.stageSelection(page.diffKind, page.diffPath, page.diffOrigPath, hunk, line,
                                                 diffModel.fingerprint)
    }
    /// Throwing one hunk of the shown diff away, with no question in front of it: the button in that hunk's own heading
    /// was held down, which is the whole of the asking (デザイン規約 §その他の操作). A line cannot be thrown away on its own — the
    /// hunk is the smallest piece — though it can still be staged on its own, which loses nothing.
    function discardHunkNow(hunk) {
        // Armed by the answer, for the reason `stageSelection` gives.
        page.diffAwaits = repoTab.discardSelection(page.diffKind, page.diffPath, page.diffOrigPath, hunk, -1,
                                                   diffModel.fingerprint)
    }
    // ---- what the reader lands on when a side runs out ---------------
    /// The row beside the open diff's file under the same heading, as `<bucket>:<path>` — the file after it, or the one
    /// before it where it is the last (`NavSectionModel.besidePath`).
    ///
    /// Noted while the file is still there, because by the time the side has run out the file has already moved to the
    /// other one and the place it left is not in the list to be read.
    property string diffNeighbour: ""
    function noteDiffNeighbour() {
        if (!page.diffShown || page.diffKind === "commit" || !worktreeModel.holdsPath(page.diffKind, page.diffPath))
            return
        page.diffNeighbour = worktreeModel.besidePath(page.diffKind, page.diffPath)
    }
    /// Everything the open diff's file had on the side being read has gone over — staged, unstaged, thrown away,
    /// committed. The reader is left standing on it, so the pane moves rather than closing (デザイン規約 §diff の中のステージ):
    ///
    ///  - the next file of the side that ran out, if it still has one;
    ///  - otherwise the same file, read from wherever it went — the whole
    ///    of it is on the other side now, which is the thing to look at;
    ///  - and only with nothing uncommitted left does the pane close.
    ///
    /// **The file list is what says so** — this runs on its `changed` and stands down while it still holds the file on
    /// the side being read. The re-read's own emptiness cannot say it: the read runs beside the status rather than
    /// after it (`load_diff` / `publish_status`), so an empty answer could land first and ask a list that still held
    /// the pre-write rows for a neighbour — and some sides never read empty at all (an untracked file staged whole
    /// still renders as its whole content, a picture has no rows either way). Asked here, the answers below are read
    /// from the very change that said the file moved.
    function followEmptySide() {
        if (!page.diffShown || page.diffKind === "commit" || worktreeModel.holdsPath(page.diffKind, page.diffPath))
            return
        const cut = page.diffNeighbour.indexOf(":")
        if (cut > 0) {
            const bucket = page.diffNeighbour.substring(0, cut)
            const path = page.diffNeighbour.substring(cut + 1)
            if (worktreeModel.holdsPath(bucket, path)) {
                page.openDiff(bucket, path, worktreeModel.origOf(path))
                return
            }
        }
        const moved = worktreeModel.bucketOf(page.diffPath)
        if (moved !== "") {
            page.openDiff(moved, page.diffPath, worktreeModel.origOf(page.diffPath))
            return
        }
        page.closeDiff()
    }
    /// Whether the diff on screen is still catching up with a write.
    ///
    /// A press addresses the rows it was made on, and carries the fingerprint of the bytes they were read from; git
    /// refuses it against anything else. So from the moment a press goes out until the rows it changed are back, the
    /// pane must not take another one — pressed twice in a row, the second landed on the diff the first had already
    /// replaced and came back with a refusal in the log.
    ///
    /// Three parts, in the order they happen, and **every one of them ends by itself**: the press is out and no answer
    /// has come (`diffAwaits`, armed only when the tab says a write went out, put down by the write's own answer), git
    /// is running (`busyCount`, which the session balances), the file is being read again (`loading`, put down by the
    /// rows arriving).
    ///
    /// **Nothing here waits on a signal that may not come.** Held on "the tree has not been read yet" instead, it
    /// wedged for good the first time a write moved no rows — staging a second line of a file already on both sides —
    /// because the file list only says `changed` when its rows differ, and then no `+` anywhere would go in again.
    property bool diffAwaits: false
    readonly property bool diffSettling:
        page.diffAwaits || repoTab.busyCount > 0 || diffModel.loading
    /// A write on the working tree has landed, so the open diff is a picture of what the file used to be.
    ///
    /// **Whoever wrote it.** This was once asked for by the writes made inside the diff itself, and the file list's own
    /// `+` and `−` moved the same file out from under the pane without a word: a line staged here and then unstaged
    /// there left the line missing from both sides on screen. The caller already knows the write
    /// was one that moves the tree (`absorbWriteResult`), so being open is the whole of the condition.
    function reloadDiff() {
        if (!page.diffShown || page.diffKind === "commit")
            return
        diffModel.requestWorkTree(page.diffKind, page.diffPath, page.diffOrigPath)
    }
    /// Asks whether the file on screen still reads the way it did. Run on the page's tick, beside the repository's own
    /// re-read.
    ///
    /// **The tree moving is not the only way the file moves.** The counts below say a stage or an unstage happened, and
    /// the file list says a file changed state; neither hears an edit that leaves both where they were — a conflict
    /// resolved in another window keeps its two stage letters until it is added, so the pane went on drawing the
    /// conflict it was opened on. Core answers this with silence unless the bytes moved
    /// (`RepoSession::refresh_diff`), so a quiet file costs one read and no repaint.
    ///
    /// A commit's diff is not asked: what a commit holds is settled. Nor is one still catching up with a write — that
    /// answer is already on its way.
    ///
    /// Answers whether a read went out, for the automation to latch on (`diff-tick`): the read itself is answered with
    /// silence on a file nobody touched, so the ask is the only edge this side of it has.
    function pollDiff() {
        if (!page.diffShown || page.diffKind === "commit" || page.diffSettling)
            return false
        return diffModel.refreshWorkTree(page.diffKind, page.diffPath, page.diffOrigPath)
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
    /// What the band's Stash button would name the entry — the commit box is on this page, the button is not
    /// (デザイン規約 §変更を退避する). Decided by the pane that owns the box, so the file row's `stash` reads the same one.
    readonly property string pageStashName: wipPane.stashName
    readonly property var pageCommands: commandsModel
    /// The commit editor itself. Automation only, and only for the one verb that has to reach it from outside a page:
    /// `tab-carry` writes words into one tab's boxes and reads them out of another's, which is a question about two
    /// pages and so belongs to the window (`WindowAutoActDriver`). Everything a person does to these boxes goes
    /// through the pane's own signals.
    readonly property alias pageWip: wipPane
    /// For the settings card's avatar entry, which offers the authors of the repository being looked at.
    readonly property var pageGraph: graphModel
    /// Whether the refs listing has landed — the read that also settles how many remotes this repository has, and so
    /// what the band's fetch button is allowed to be (`fetch-tip`).
    readonly property bool pageRefsLoaded: branchesModel.refsLoaded
    /// Whether the right pane is still waiting on the selected commit's own read — the last of the page's reads, and
    /// the one the changed-file list is laid out from. **The graph's pass and the refs are both in before it is even
    /// asked for**: they are what decide the row this page opens on, and the request goes out from that landing
    /// (`trySelectDefault`). **Settled, not loaded**: a page showing the working tree is waiting for no such answer,
    /// and neither is a repository with no commit to select, so both answer yes holding no details at all.
    readonly property bool pageDetailsSettled: page.wipShown || page.selectedOid === ""
                                               || !detailsModel.loading
    /// The window's own band, handed back in by `Main`. The Stash button stands there rather than on this page
    /// (デザイン規約 §変更を退避する), and this page's verbs press the real one through here — a verb that called what the
    /// button calls would be answering for a second way in rather than for the band's wiring. An automation-only
    /// exposure, the same one `GraphPane.view` is (app-ui.md). `var` because `TopBar` is above this file, not beside it.
    property var pageBand: null

    /// Whether the command log is up. Closed is the resting state: the `>_` at the foot of the left menu opens it, and
    /// a failed command raises it.
    property bool commandsOpen: false
    function toggleCommands() {
        page.commandsOpen = !page.commandsOpen
    }
    /// One mark for a failed command and for this tab's error line both. The page's rule, since the mark moves seats.
    readonly property bool commandsWrong: commandsModel.failed || repoTab.lastError !== ""
    /// The panel itself, or null: it is built into its seat when the log is raised and taken down with it
    /// (`commandsSeat`), so everything below that reads the panel reads it through here and answers for a log that
    /// is down with nothing.
    readonly property var commandsPane: commandsSeat.item
    /// Automation: the colour the `>_` painted — the log's own band while it is up, the pane's foot while it is down.
    readonly property color commandsMarkColor:
        page.commandsPane !== null ? page.commandsPane.markColor : sidebarPane.commandsMarkColor
    /// What the panel is doing rather than what was asked of it — automation reads this one, so a cut binding fails.
    readonly property bool commandsShown: page.commandsPane !== null && page.commandsPane.visible
    /// Automation reads the laid-out width, not the preferred width it requested, before persisting a state round trip.
    readonly property real stateDetailsWidth: rightPane.width
    /// Automation only: the header's `Clear`, pressed from outside the panel (`PG_AUTO_ACT=commands-clear`).
    function clearCommandLog() {
        if (page.commandsPane !== null)
            page.commandsPane.clearPanel()
    }
    /// Automation only: the hand that drags over the log, and the key that takes what it picked
    /// (`PG_AUTO_ACT=commands-select` / `commands-copy`). Both enter the panel's own functions.
    function pickCommandText(fromRow, fromAt, toRow, toAt) {
        if (page.commandsPane !== null)
            page.commandsPane.pickText(fromRow, fromAt, toRow, toAt)
    }
    /// ...and the same hand started on the ground under the last row (`PG_AUTO_ACT=commands-sweep`).
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

    RepoTab { id: repoTab }
    CommandsModel {
        id: commandsModel
        // The machine's offset from UTC, said before the first row arrives: the panel says it again each time it is
        // raised (`CommandsPane.tellTheZone`), but the panel is built only when it is raised, and the rows that
        // arrive before that are stamped as they arrive.
        Component.onCompleted: commandsModel.setZoneMinutes(new Date().getTimezoneOffset())
    }
    GraphModel { id: graphModel }
    WorkTreeModel { id: workTree }
    DetailsModel { id: detailsModel }
    DiffModel { id: diffModel }
    RebasePlanModel { id: planModel }
    NavSectionModel { id: branchesModel }
    NavSectionModel { id: remotesModel }
    // One letter apart, two different things: the three `*Model`s below feed the WIP pane (the "worktree" nav section =
    // uncommitted files), while `worktreesModel` lists git worktrees (the sidebar's WORKTREES).
    //
    // **One per bucket**, because the pane is a list per bucket (`WipPane`). Each shows its own run and holds the whole
    // status, so a question about a file — which bucket has it, what is beside it, where it came from — is asked of
    // `worktreeModel` whichever bucket the file is in.
    NavSectionModel { id: conflictsModel }
    NavSectionModel { id: worktreeModel }
    NavSectionModel { id: stagedModel }
    NavSectionModel { id: worktreesModel }
    NavSectionModel { id: stashesModel }
    NavSectionModel { id: tagsModel }

    /// True on the one page the window is showing, which is the only page there is: the window builds one for the tab
    /// in front and takes it down when that tab stops being in front (`Main.qml`). The blank page is the exception —
    /// it stands in for no tab at all.
    ///
    /// So this is set once, at construction, and `Component.onCompleted` is what acts on it. Nothing turns it over
    /// afterwards; a page that would have to be told it is no longer current is a page that has already been
    /// destroyed.
    property bool pageCurrent: false

    /// Everything this page owes on its way off the front, in the order it is owed (`TabsModel::leaving_tab`).
    ///
    /// **Called while the page is still whole**, which is the only moment any of it can be read: the strip has not
    /// moved yet, so nothing here has been taken down.
    ///
    /// The layout goes first because it is what the *next* tab is laid out at — one set for the whole application, so
    /// leaving is the moment it is worth writing down (`PageLayout.reportLayout`). Then the words in the commit
    /// editor, which are the one thing on this page no repository can be asked for again. Then the repository itself,
    /// which can.
    function leaveFront() {
        pageLayout.reportLayout()
        if (page.blank)
            return
        repoTab.holdDraft(wipPane.subjectText, wipPane.bodyText, page.amending)
        repoTab.release()
    }

    /// …and the other half: what the last page on this tab was holding, put back into the boxes.
    ///
    /// **Before the repository says anything**, so that the two paths that fill these boxes on their own — a stopped
    /// merge's message (`absorbOpMessage`) and a popped stash's name (`absorbPopLabel`) — find them already written
    /// in and leave them alone, which is the rule those two keep anyway.
    function restoreDraft() {
        // The flag as well as the words: a message written for an amend, put back under a plain commit button, would
        // make a second commit instead of replacing the first.
        if (repoTab.draftAmending()) {
            page.amending = true
            wipPane.setAmendChecked(true)
        }
        const subject = repoTab.draftSubject()
        const body = repoTab.draftBody()
        if (subject === "" && body === "")
            return
        // Words put back into a pane nobody is looking at are only half of them being kept: the reader left this tab
        // mid-sentence, and what they come back to has to be the sentence. This is also what holds the selection off —
        // `trySelectDefault` leaves a page showing the working tree alone.
        page.showWip()
        wipPane.setMessage(subject, body)
        // **Not `pendingWipSelect`.** A dirty tree already puts its own row at the top of a graph nobody has picked a
        // row on, so the highlight lands there without asking; a clean tree has no such row, and the flag would then
        // stand until the tree got dirty and jump the view to the top from wherever the reader had scrolled to
        // (規約 §ListView.highlightFollowsCurrentItem — a background pass may not move a reader's view).
    }

    // ---- what this page is laid out at ------------------------------
    // The saved sizes, the sections, and the floor the window is held to.

    PageLayout {
        id: pageLayout
        page: page
        sidebarPane: sidebarPane
        rightPane: rightPane
        commandsSeat: commandsSeat
        graphPane: graphPane
        wipPane: wipPane
        detailsPane: detailsPane
        repoTab: repoTab
        worktreeModel: worktreeModel
        detailsModel: detailsModel
    }

    /// What the window reads off the page it is showing: the floor it may not be laid out under (`Main.floorWidth` /
    /// `floorHeight`) and the two panes' own overflow, which the floor's own verb asks about.
    readonly property real floorWidth: pageLayout.floorWidth
    /// …and the same floor with the list open whether or not it is, which is the one the band's actions time their
    /// giving way against (`PageLayout.openFloorWidth`).
    readonly property real openFloorWidth: pageLayout.openFloorWidth
    readonly property real floorHeight: pageLayout.floorHeight
    readonly property bool wipBlockScrolls: pageLayout.wipBlockScrolls
    readonly property real detailsOverHeight: pageLayout.detailsOverHeight

    /// …and what it calls: the layout is pulled on the window's timer, and the two setters are the headless state
    /// check's (a person drags).
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
        // The bars that announced themselves before the watcher existed.
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
        branchesModel.attachSection(page.tab_id, "branches")
        remotesModel.attachSection(page.tab_id, "remotes")
        conflictsModel.attachWorktree(page.tab_id, "conflicts")
        worktreeModel.attachWorktree(page.tab_id, "unstaged")
        stagedModel.attachWorktree(page.tab_id, "staged")
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
            // The window coming back is the moment an outside change is most likely to be waiting, so the file on
            // screen is asked as well rather than waiting out the rest of the tick.
            page.pollDiff()
        }
    }

    /// True while the window is on screen (see Main.qml): the page shown there re-reads its repository on a tick, so a
    /// commit made in a terminal or by an agent turns up on its own.
    property bool onScreen: false
    Timer {
        interval: Metrics.pollIntervalMs
        repeat: true
        // Only the tab in front — the others catch up when switched to, and reading every open repository on every tick
        // is what makes polling expensive elsewhere.
        running: page.onScreen && page.visible && repoTab.state === "open"
        onTriggered: page.pollRepo()
    }
    // The badge counting a running replay out. Its own tick because it asks its own question: two file reads off the
    // git directory, no process, so it can run at a rate a number is worth watching at — where the tick above carries
    // a whole `git status` and is ten seconds apart for it (デザイン規約 §進行中・長押しの定数, and the measured cost
    // in `ci/baseline/poll-cost-windows-x64.md`).
    //
    // While the write is out and no longer: with nothing replaying there is no number, and the status tick is what
    // says the operation ended. `replayRunning` covers the gap a handed-over plan leaves before the queue starts it.
    Timer {
        interval: Metrics.opProgressMs
        repeat: true
        running: page.visible && repoTab.state === "open" && page.replayRunning
        onTriggered: repoTab.refreshOpProgress()
    }
    /// One tick: the repository, and the file the diff pane is holding. The two are separate reads because they answer
    /// different questions — refs and status say what the tree is, the diff says what the file says.
    function pollRepo() {
        repoTab.refreshPoll()
        page.pollDiff()
    }

    // Where the selection stands, kept so a commit that disappears from under it can be followed to whatever took its
    // place.
    property int selectedRow: -1

    // ---- the commits being held (デザイン規約 §複数のコミットを選ぶ) ----
    /// The choice, as a set of commit ids, and how many are in it. **Ids, not rows**: a background pass rewrites the
    /// rows under the hand, and everything this page holds is re-resolved by id when one lands (`onStatsChanged`).
    /// Empty only where the working tree's row is what is shown — that row is not a commit.
    property var chosenOids: ({})
    property int chosenCount: 0
    /// The commit the last press landed on, which is what a Shift click measures its range from. **A commit, not a
    /// row**: a background pass rewrites the rows under the hand, and a row number kept across one would measure the
    /// next range from whatever commit had moved into it (デザイン規約 §複数のコミットを選ぶ). Empty before the first press.
    property string chosenAnchorOid: ""
    /// The choice as the models want it: the ids in walk order, and the rows that name them. Settled together, so the
    /// list on the right and the files under it can never be of different commits.
    property string chosenPacked: ""
    property string chosenRecords: ""
    /// Makes one commit the whole of the choice. Every way a single row becomes what is being read comes through here
    /// — a plain click, an arrow key, a landing, the find bar — so there is one place the choice is settled from.
    function chooseOnly(oidHex) {
        const only = ({})
        if (oidHex !== "" && !GitFacts.wipOid(oidHex))
            only[oidHex] = true
        page.chosenOids = only
        page.chosenCount = Object.keys(only).length
        page.chosenPacked = ""
        page.chosenRecords = ""
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
            // The anchor is found again by its id: the rows may have moved since the press that set it, and an
            // anchor the graph no longer draws leaves the range to measure from the row being read.
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
    /// **The last one cannot be taken out.** An empty choice is the working tree's own state, and reaching it from a
    /// commit would leave the pane on the right describing something no row is drawn as holding.
    function chooseAlso(oidHex) {
        // A fresh object every time: the rows follow this property, and assigning the same one back changes nothing
        // for them to follow.
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
    /// The same Ctrl click, made in the list of what is held rather than in the graph (`ChosenCommitRow`). It takes a
    /// commit out and never puts one in — everything in that list is in the choice already.
    ///
    /// **The anchor stays where the graph left it**: it is a commit of the graph, and this press was not on one. A
    /// Shift click after this one measures from the last row a hand actually landed on, which is what it would
    /// measure from if the drop had been made in the graph.
    function dropFromChoice(oidHex) {
        page.chooseAlso(oidHex)
    }
    /// Packs the rows of the chosen commits again for the list on the right: what it says about them (a subject, a
    /// face) is read off the rows, so it is read again whenever the rows are.
    function refreshChosenRecords() {
        if (page.chosenPacked !== "")
            page.chosenRecords = graphModel.chosenRows(page.chosenPacked)
    }
    /// Every commit row between two places, ends included — what a Shift click reaches. **The ones scrolled past are
    /// in it too**: the anchor and the click are on screen by definition, and what lies between them usually is not.
    ///
    /// Asked of the model in one crossing (`GraphModel.oidsBetween`): a range is as long as the hand dragged it, and a
    /// call per row would put a walk of the history on a click (CLAUDE.md §性能予算). The working tree's row is dropped
    /// wherever a range sweeps over it.
    function chooseRange(from, to) {
        if (from < 0 || to < 0)
            return
        const packed = graphModel.oidsBetween(from, to)
        const next = ({})
        const ids = packed === "" ? [] : packed.split(String.fromCharCode(31))
        for (const oidHex of ids) {
            if (oidHex !== "" && !GitFacts.wipOid(oidHex))
                next[oidHex] = true
        }
        // A range that swept nothing but the working tree's row is not a choice; the anchor stays where it was.
        if (Object.keys(next).length === 0)
            return
        page.settleChoice(next)
    }
    /// The one place the set and its tally are written together, so a count and a highlight cannot disagree.
    ///
    /// **A choice that has come back down to one commit is not a choice**: it is that commit being read, which is what
    /// every other way of landing on one row does (`activateRow`).
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
        page.chosenPacked = graphModel.presentOids(ids.join(String.fromCharCode(31)))
        page.chosenRecords = page.chosenPacked === "" ? "" : graphModel.chosenRows(page.chosenPacked)
        if (page.chosenPacked === "")
            return
        // None of what a single commit's pane says applies to several: no stash sits under a choice, and the diff
        // that was open was of one file of one commit.
        page.selectedStashRef = ""
        page.closeDiff()
        detailsModel.requestSelection(page.chosenPacked, page.chosenCount === 2)
    }
    /// Drops from the choice whatever the graph no longer stands on. Run when a pass lands: a rewrite takes commits
    /// away, and a choice that goes on counting them says a number no row on screen adds up to.
    function settleChoiceAfterPass() {
        if (page.chosenCount <= 1)
            return
        const packed = graphModel.presentOids(Object.keys(page.chosenOids).join(String.fromCharCode(31)))
        const kept = packed === "" ? [] : packed.split(String.fromCharCode(31))
        if (kept.length === page.chosenCount) {
            // The same commits, drawn again. In the same order, what the list on the right says about them is read
            // again off the rows the pass brought (a subject reworded, a face assigned); in another order — a rewrite
            // moved one past another — the choice is read again whole, since which of two commits is the older is
            // what the pane compares from.
            if (packed === page.chosenPacked)
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
    // when new ones arrive above. The WIP row is no use for that — it comes and goes with the working tree — so the
    // newest *real* commit carries the measurement.
    property string anchorOid: ""
    property int anchorRow: -1
    function rememberAnchor() {
        let row = 0
        let oidHex = graphModel.oidAt(0)
        if (GitFacts.wipOid(oidHex)) {
            row = 1
            oidHex = graphModel.oidAt(1)
        }
        page.anchorOid = oidHex
        page.anchorRow = oidHex === "" ? -1 : row
    }
    // How far the graph slid under the viewport. Zero when the anchor is gone: a rewrite deep in the history moves rows
    // by different amounts and there is no single answer, so the view is left alone.
    function anchorShift() {
        if (page.anchorRow < 0 || page.anchorOid === "")
            return 0
        const now = graphModel.rowOf(page.anchorOid)
        return now < 0 ? 0 : now - page.anchorRow
    }

    // The row the selection stood on is gone and this page owes it a landing. Deliberately not resolved on the spot:
    // the status of the working tree, the refs and the walk arrive as three separate messages, and the two that come
    // first still describe the repository as it was — reading the branch out of them lands on the commit that was just
    // replaced. Resolved once the graph holds where the branch points, which is only true of the refreshed pair.
    property bool pendingHeadSelect: false
    /// The first report of HEAD that may answer the landing (`WorkTreeModel.headSeq`). A write's answer names it
    /// (`RepoTab.writeAnswerHeadSeq`): the report in hand at that arming may still describe the repository as it
    /// was, with a stale HEAD that has a row to land on, and only a report numbered at or above the answer's looked
    /// after the write. An arming read out of a report already in hand (the working tree emptying, the selected
    /// commit vanishing) takes that very report.
    property int pendingHeadFromSeq: 0
    // Whether the landing is one the person here asked for, in which case the viewport goes to it as well: a commit
    // they meant to make is not an answer if it lands off screen. The other two ways this is set happen *to* the window
    // — a commit made in a terminal, a rewrite that swept the selected commit away while the poll was watching — and a
    // background pass that moves rows under a reader may not also move their view
    // (§ListView.highlightFollowsCurrentItem).
    property bool pendingHeadAsked: false
    /// Whether the last write this window sent was a stash that landed — read where the working tree turns out to be
    /// empty, which is the moment that says the entry took all of it (`leaveWipWhenDone`).
    property bool stashLanded: false
    /// What operation the last status named, so that its going away can be read as an edge rather than as a state.
    property string seenOpText: ""
    /// The WIP face has nothing left to hold the reader with. The working tree emptied: after a commit of our own that
    /// is the end of the editor's job; when someone else committed these changes it happens with no warning, so a
    /// message being written stays on screen with its text — it is the one thing here that cannot be read back off
    /// disk. Or the operation went away, which is what takes the exit card off the face. Either way, land on the commit
    /// that now holds the changes rather than on nothing.
    ///
    /// **A stash pressed here is not held by the message.** The press *is* the decision to empty the tree, so the box
    /// is not a reason to stay: the words are still in it when the working tree comes back (the pane is hidden, not
    /// unloaded) and the entry took them for its own name on the way out. Left to the message alone, the one press that
    /// always has words in front of it — a stopped merge fills the box itself (`absorbOpMessage`) — is the one that
    /// never lands, and the pane stands over a tree it no longer describes while the highlight the working-tree row
    /// left behind is inherited by whatever slid into its place, which after this press is the entry it just made.
    ///
    /// **Every half is read off one status** (`WorkTreeModel`), never off the file list beside it, and each of the two
    /// reasons is a way out that was missing. The list says `changed` only when its rows differ, so a stop that ends on
    /// a clean tree — an `edit` stop put down by `--continue`, `--skip`, `--abort` or a terminal — moves no row and
    /// would never ask this question at all. And the list is drained before the headline is (`hub::sink` pushes the nav
    /// runs first), so a question asked from there reads an `opText` one status old and hears the operation that has
    /// just gone as though it were still standing — which is every ordinary conflict landing, where the rows empty and
    /// the operation ends in the same status.
    ///
    /// `edge` is what that status moved: the tree's own counts, or the operation. Standing alone, the condition would
    /// walk a reader off the face on a poll that changed nothing under them — the message box emptied by hand is the
    /// one that would do it.
    function leaveWipWhenDone(edge) {
        // **A tree nobody has read is not a tree with nothing in it.** The counts start at zero and `opText` starts
        // empty, which is this question's own picture of a job that is done — so asked before the first status it
        // answers yes about a repository it has never seen (規約 §UI 自動化の因果性). `loaded` latches on that first
        // status and never goes back (`WorkTreeModel`), so this stands in front of a page's opening moment alone.
        if (!workTree.loaded)
            return
        if (!edge || !page.wipShown || workTree.opText !== "")
            return
        const clean = workTree.stagedCount === 0 && workTree.unstagedCount === 0
                   && workTree.untrackedCount === 0 && workTree.conflictCount === 0
        if (!clean)
            return
        const ourStash = page.stashLanded
        if (!ourStash && (wipPane.subjectText !== "" || wipPane.bodyText !== ""))
            return
        page.stashLanded = false
        page.wipShown = false
        page.pendingHeadSelect = true
        // The status this is read out of **is** the one that answers: the report of HEAD that came with it is already
        // in hand (`hub::sink` sends it ahead of the status), and that is the commit the changes went into.
        page.pendingHeadFromSeq = workTree.headSeq
        // Only ours is a landing anybody asked for, so only ours takes the viewport along.
        page.pendingHeadAsked = ourStash
    }
    function tryPendingHeadSelect() {
        if (!page.pendingHeadSelect || !workTree.headKnown)
            return
        // Where HEAD stands is the one record's, branch or not (`WorkTreeModel.headOid`), so a write that left it
        // detached lands the same as any other. Only a report counted after the arming may answer
        // (`pendingHeadFromSeq`): the one in flight at a write's answer still describes the repository as it was,
        // and its stale HEAD has a row.
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
        // Held back by an unsaved message: the question put the highlight back where it was, so there is nowhere for
        // the view to go yet.
        if (asked)
            graphPane.showRowSoon(row)
    }

    // An operation stopped part-way and this page owes it a landing on the working tree, where the conflicts and the
    // way out are. Held for the same reason `pendingHeadSelect` is: git has already written the markers by the time it
    // answers, but the status carrying those rows arrives afterwards, and the row does not exist until it does. Always
    // asked for — a stop only ever follows a press — so the viewport goes along.
    property bool pendingWipSelect: false
    function tryPendingWipSelect() {
        if (!page.pendingWipSelect)
            return
        // Row 0 is where the working tree stands, and its all-zero id is the graph saying the row is there at all: the
        // walk prepends it only once it knows the tree is dirty, and until then row 0 is still the commit that was on
        // top. Landing on that one would take the press to the wrong place entirely.
        const oidHex = graphModel.oidAt(0)
        if (!GitFacts.wipOid(oidHex))
            return
        page.pendingWipSelect = false
        graphPane.setCurrentRow(0)
        page.showWip()
        graphPane.showRowSoon(0)
    }

    // The selected commit is gone from the graph and this page did not rewrite it: an amend or a rebase run in a
    // terminal replaced it while the poll was watching. Whatever now stands where it stood is the closest thing to what
    // was being read; failing that, fall back to the branch's own commit, which is never nothing.
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

    // What a row click means: the synthetic WIP row (all-zero id) opens the working-tree view, anything else selects
    // the commit.
    /// `atRow` is where the row is, for the callers that already know (a click knows: it came from that row). **The
    /// lookup is a walk over every loaded row** (`GraphModel::row_of`), so a path that has the number and asks for it
    /// again puts a walk of the whole history on a click (CLAUDE.md §性能予算「コミット数に比例する同期処理を UI 操作の経路に置かない」).
    /// -1 asks for the lookup, which is what a landing named only by its commit has to do.
    function activateRow(oidHex, atRow) {
        // Any new selection settles where the last rewrite left off, and answers any landing this page still owed.
        page.rewordRow = -1
        page.pendingHeadSelect = false
        page.pendingHeadAsked = false
        page.pendingWipSelect = false
        // Clicking anywhere is the way out of the name box and of a standing row question: both are offers, not work in
        // progress.
        graphPane.stopNaming()
        page.stopRowAsk()
        // And the right pane's mark is about the commit it was raised over. Every road to another commit comes through
        // here, keyboard ones included, so this is where it stops being true — the one road that keeps it raises it
        // again on the way out (`onMessageAsked`).
        detailsPane.dropAttention()
        // **A plain landing is one commit**, whatever was being held before it (デザイン規約 §複数のコミットを選ぶ).
        // Whether it was several is carried past the early return below: a click on the commit already open has
        // nothing left to ask git for *unless* the pane is showing a choice, and then everything below has to run.
        const wasChoice = page.chosenCount > 1
        page.chooseOnly(oidHex)
        const row = atRow !== undefined && atRow >= 0 ? atRow : graphModel.rowOf(oidHex)
        if (row >= 0) {
            graphPane.setCurrentRow(row)
            page.selectedRow = row
            page.chosenAnchorOid = oidHex
        }
        // **The working tree's row is not a hash**, so nothing below can be skipped for it the way it can for a
        // commit: what it shows is whatever the tree is now.
        if (GitFacts.wipOid(oidHex)) {
            page.showWip()
            return
        }
        // The commit already open. **Everything below is of this commit and has been asked once**: the details and the
        // signature are answers to a hash, and a hash cannot have changed under the same row — asking again spends a
        // `git show` and a gpg run per click for an answer already on screen (the find bar says the same of landing
        // twice on one row). The second click of the rename gesture is exactly this click, so the wait it opens would
        // be spent on work nobody is waiting for (デザイン規約 §グラフ行のダブルクリック).
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
    /// way — it would land first and be photographed instead (`PageAutoStart`). Written from outside and false
    /// wherever nobody wrote it, which is every window a person opens.
    property bool rowPickedElsewhere: false

    // Selection policy: restore across the tag-swap reset, and open on the working tree's own row where the tree has
    // one and on the commit HEAD stands on where it does not, so the right pane always shows something.
    function trySelectDefault() {
        if (page.selectedOid !== "" || page.wipShown || page.pendingHeadSelect || page.rowPickedElsewhere
                || graphModel.rowTotal === 0)
            return
        // Where HEAD stands is the commit to open on, and whether the tree has a row of its own decides whether a
        // commit is the landing at all — so both reads have to have answered. **Before the first status
        // `wipRowStands` is false because nothing has been asked yet** (`WorkTreeModel.loaded`), and a graph pass that
        // beat it would land on a commit and never come back: the first line above holds every later call off.
        if (!workTree.headKnown || !workTree.loaded)
            return
        let row = -1
        if (workTree.wipRowStands) {
            // **Uncommitted work is the landing, branch or no branch** (デザイン規約 §未コミット行が名乗るもの):
            // the row stands while the tree is dirty or an operation is (`graph::wip_row_stands`), and it is what
            // the reader came back to. Asked of the status rather than of row 0, because whether the opening walk
            // carries that row is a race between the first status and the walk's own start (`session::walk`) — the
            // two disagreeing is the graph saying it is one read behind, and until the pass that knows lands, row 0
            // is still the commit that was on top (`tryPendingWipSelect` waits on the same word). Every pass calls
            // this again.
            if (!GitFacts.wipOid(graphModel.oidAt(0)))
                return
            row = 0
        } else {
            // **Where HEAD stands, branch or not** (`WorkTreeModel.headOid`, the record's own report — the same one
            // a write's landing is paid off, `tryPendingHeadSelect`). A detached HEAD is still a commit somebody is
            // standing on, and that is the one to open on.
            row = workTree.headOid !== "" ? graphModel.rowOf(workTree.headOid) : -1
            if (row < 0) {
                if (graphModel.loading)
                    return // the head row may still be streaming in
                // HEAD outside the window: the newest commit, which is not the newest row where stashes stand over
                // it (`GraphModel.newestCommitRow` — the rule, and the two automation drivers that ask it too). A
                // repository whose rows are all stashes has no commit to open on.
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
    // streaming restart bumps resetCount — that is the only case where the viewport lost its scroll position and needs
    // re-anchoring. In-place replacements keep the position, and re-centering would yank the view around.
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
                if (page.pendingHeadSelect) {
                    page.tryPendingHeadSelect()
                } else if (page.selectedOid !== "") {
                    const row = graphModel.rowOf(page.selectedOid)
                    if (row >= 0) {
                        page.selectedRow = row
                        graphPane.setCurrentRow(row)
                        if (resetHappened)
                            graphPane.anchorSoon()
                    } else if (page.rewordRow >= 0) {
                        page.followRewrittenCommit()
                    } else if (page.selectedRow >= 0) {
                        // A selection that was never on screen — a details jump past the walk window — has not
                        // vanished: the commit is still there, only unwalked. Leave the reader on it.
                        page.followVanishedCommit()
                    }
                }
                // After the one being read has been placed: the two follows above settle a choice of their own, and
                // this drops whatever else the pass took away (`settleChoiceAfterPass`).
                page.settleChoiceAfterPass()
            }
            page.trySelectDefault()
        }
    }
    // Where HEAD stands is not the refs' to say — the record's report rides the working-tree model, and the landings
    // and the default selection are paid there (`workTree.onChanged`).
    Connections {
        target: branchesModel
        function onRefsSettled() {
            // The listing is the half a move onto a branch that was not there waits on: the status behind the write
            // already has HEAD on it, and until this arrives the screen still says no such branch (`moveLanding`).
            page.absorbMoveLanding()
            // The listing the delete was waiting on: the rows it took away are gone from the model itself now, so the
            // window stops standing in for it (デザイン規約 §消す操作は先に画面から消す).
            page.refsProvedGone()
        }
    }
    Connections {
        target: diffModel
        // The rows are about to be swapped for a re-read of the same file — a write of ours, or an edit made outside
        // this window that the tick caught. Held here rather than beside whoever asked, because this is the moment the
        // view is still standing where the reader left it, and the two askers cannot both be trusted to say so: the
        // tick asks on files that turn out not to have moved, and holding a place for a swap that never comes would
        // put the next one back somewhere the reader has since left (`DiffScrollPlace`).
        function onRowsReplacing() {
            diffPane.holdScroll()
        }
    }
    // The stashes arrive on a word of their own, later than the refs — read off the refs' arrival, a dropped stash
    // would be back on screen for the whole of the graph rebuild that sits between the two (`session::write`).
    Connections {
        target: stashesModel
        function onStashesSettled() {
            page.stashesProvedGone()
        }
    }
    /// The `WorkTreeModel.treeRevision` the open diff was last read against — the counts of the four buckets are what
    /// a stage or an unstage moves whoever made it, and the model bumps the revision when they do.
    property int seenTreeRev: -1
    /// The write whose answer already re-read the file, so that the status arriving behind it does not read the same
    /// file over again. -1 once that status has come and gone.
    property int diffReadAt: -1
    // **The tree was read.** Said by the working-tree model rather than by the file list beside it: the list says
    // `changed` only when its rows differ, and a status that moved no row is exactly the one this has to hear about (a
    // second line staged out of a file already on both sides moves nothing).
    Connections {
        target: workTree
        function onChanged() {
            const moved = workTree.treeRevision !== page.seenTreeRev
            page.seenTreeRev = workTree.treeRevision
            // The operation that was standing is not standing any more — the other edge the WIP face's exit turns on,
            // and the only one a clean stop ever moves (`leaveWipWhenDone`).
            const opGone = page.seenOpText !== "" && workTree.opText === ""
            page.seenOpText = workTree.opText
            // The status that follows this window's own write: the file was read when the write answered.
            const ours = page.diffReadAt === repoTab.writeSeq
            page.diffReadAt = -1
            // Something outside this window moved the tree, so the rows on screen — and the fingerprint the next `+`
            // would be written against — are a picture of the file as it was. Pressing one then came back with git's
            // refusal.
            if (moved && !ours)
                page.reloadDiff()
            page.absorbOpMessage()
            // ...and the other half of a move's landing: this is what carries the branch HEAD ended up on.
            page.absorbMoveLanding()
            // Whether the WIP face still has anything to stand for. **After `absorbOpMessage`**: the words a stopped
            // merge put in the box are that operation's, and an abort takes them back out — asked before it, the box it
            // has yet to empty reads as a message somebody is writing.
            page.leaveWipWhenDone(moved || opGone)
            // The report of where HEAD stands rides this model (`WorkTreeModel.headSeq`), so this is where a landing
            // owed to a write is paid — only a report counted after the arming answers (`tryPendingHeadSelect`), and
            // the first read after a write always sends one, moved or not, so a write that recorded nothing (a
            // cherry-pick of a commit the branch already has) pays the same landing as one that wrote a commit.
            page.tryPendingHeadSelect()
            page.trySelectDefault()
        }
    }
    Connections {
        target: worktreeModel
        function onChanged() {
            // The file the diff is on is still where it was, so this is the last moment its neighbour can be read (see
            // `noteDiffNeighbour`) — and the change that takes it off the side being read is the one that moves the
            // pane, with the neighbour noted by every change before it (`followEmptySide`).
            page.noteDiffNeighbour()
            page.followEmptySide()
            // **The face's own exit is not here** — it is asked of the status headline, one snapshot at a time
            // (`leaveWipWhenDone`), because half the ways out of it move no row in this list at all.
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // The panes sit above the command log, which is closed until it is asked for. Splitting them vertically keeps
        // the log's height in the reader's hands and out of the panes' business.
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
            // (repository state / search / fetch / push live in the window toolbar, next to the tabs; Reload is the app
            // menu and F5)
            SplitView {
                visible: !page.openFailed
                SplitView.fillHeight: true
                SplitView.minimumHeight: pageLayout.panesMinHeight
                orientation: Qt.Horizontal
                handle: SplitHandleBar {
                    onHandChanged: (which, held) => page.holdSplitBar(which, held)
                }

                SidebarPane {
                    id: sidebarPane
                    // The plan's freeze takes the pane whole and is meant to be read as the mode's own restriction;
                    // a replay running behind the screen holds only the doors — a switch, a delete, a name box, the
                    // `+` — and everything the pane is *read* with goes on working through it. Two states, two
                    // answers (`SidebarPane.frozen` / `doorsHeld`).
                    frozen: page.sidebarFrozen
                    doorsHeld: page.doorsHeldWhy !== ""
                    // Over the pane beside it while a name box is standing: the box reaches past this pane's edge when
                    // what is in it does not fit, and the graph is laid out after this one (`NavItemDelegate`). Only
                    // then — a pane that sat over its neighbour the rest of the time would draw its own edge over the
                    // splitter.
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
                    // The menus the rows raise are the page's, so only the page can say one is standing over the folded
                    // list.
                    menuOpen: page.menuStanding
                    onFoldRequested: collapse => page.foldByHand(collapse)
                    onRefActivated: oidHex => page.jumpToRef(oidHex)
                    onRefMenuRequested: (kind, name, full, oidHex) => page.openRefMenu(kind, name, full, oidHex, true)
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
                    onRenameSubmitted: (kind, id, name) => page.renameRow(kind, id, name)
                    onAddRemoteRequested: publishFlow.startAddRemote()
                }

                // Center: a report over the whole of it, and under that the commit graph ⇄ file diff.
                //
                // **The report is above the pair rather than inside either**: it answers a write, and a write is
                // answered wherever the reader happens to be standing — a push refused while a diff is open has the
                // same news to give (デザイン規約 §答えの要らない報せ). Put in one of them it would be silent in the other, and
                // put over them it would cover what it is about.
                //
                // **So the middle steps down for it.** The bar takes its own row of the column and the pair takes what
                // is left, which is the same thing the graph did for it when the bar lived inside that pane: nothing is
                // covered, and a closed bar has no height to give.
                ColumnLayout {
                    SplitView.fillWidth: true
                    // Not a number of its own: what the graph's own columns come to once they have both given
                    // everything they can, held up to a side pane's width so the middle never reads as the thinnest of
                    // the three (`PageLayout.centreMinWidth`).
                    SplitView.minimumWidth: pageLayout.centreMinWidth
                    spacing: 0

                    NoticeBar {
                        id: noticeBar
                        Layout.fillWidth: true
                        // **The order Escape is handed out in, and the one place it is written.** Both bars can be
                        // standing at once — raising a question does not lower a report (`startRowAsk`), and a write
                        // answered while one stands does not lower the question — and **two enabled
                        // `StandardKey.Cancel` shortcuts in one window fire neither** (`tests/qml/tst_escape.qml`),
                        // so one of them has to give way rather than both dying.
                        //
                        // **The question keeps it**: it is what is being asked of the reader, it holds the keyboard
                        // (its pill takes the focus as it opens), and it is the one thing here that stands until it
                        // is answered — a report is read and nothing follows from it. Under both is the arrival mark,
                        // last because it takes Escape as a key handler rather than as a shortcut
                        // (`page.escapePressed`).
                        //
                        // Written as this bar's own line rather than handed round from the page: what each bar does
                        // with Escape stays in its own declaration, and only the order between them is here.
                        yieldsEscape: graphPane.asking
                        onAcknowledged: page.hideNotice()
                    }

                    StackLayout {
                        id: centreStack
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        // The plan stands over both from the press that asked for it — before its rows exist, which is
                        // the whole of `planShown` (a diff opened earlier closes on the way in, `onPlanShownChanged`),
                        // and the graph comes back exactly as it was when the plan is put away.
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
                            // The bar moves between matches, not between commits — landing on the same row twice changes
                            // nothing and costs no git.
                            onFindLanded: oidHex => {
                                if (oidHex !== "" && oidHex !== page.selectedOid)
                                    page.activateRow(oidHex)
                            }
                            onRowMenuOpenRequested: (oidHex, record) => page.openRowMenu(oidHex, record)
                            onRowSwitchRequested: (oidHex, record) => page.rowDoubleClicked(oidHex, record)
                            onRowRenameRequested: (oidHex, record) => page.startRename(oidHex, record)
                            onRenameSubmitted: (kind, id, name) => page.renameRow(kind, id, name)
                            namingRefused: page.graphNameRefusedWhy !== ""
                            namingRefusedWhy: page.graphNameRefusedWhy
                            onChipExpandRequested: (oidHex, atRow, records, anchor) =>
                                rowHost.openRefList(oidHex, atRow, records, anchor)
                            onChipCollapseRequested: rowHost.closeRefListUnlessEntered()
                            onRowHoverRequested: (row, inside) => page.restOnCommit(row, inside)
                            onCreateBranchRequested: (oidHex, name) => repoTab.createBranch(name, oidHex, true)
                            // Nothing moves: a tag is left on the commit and the tree stays where it is.
                            onCreateTagRequested: (oidHex, name) => repoTab.createTag(name, oidHex)
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
                            // The two swap over during a rebase; the model is where that is already answered.
                            sideOurs: workTree.sideOurs
                            sideTheirs: workTree.sideTheirs
                            sideColorOurs: page.sideColorOurs
                            sideColorTheirs: page.sideColorTheirs
                            busy: page.diffSettling
                            menuStanding: page.menuStanding
                            onCloseRequested: page.closeDiff()
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
                        }

                        // The plan's face is built when it takes the seat and taken down with it: it is a whole pane
                        // of rows, a band and a menu that a page pays for otherwise, and everything it shows is the
                        // model's, so nothing is lost in the taking down (rules-refs/app-ui.md — the dialog-seat
                        // rule). The seat stands in the stack whether or not it holds a face, so the index above
                        // still names it.
                        Loader {
                            id: planSeat
                            active: page.planShown
                            sourceComponent: RebasePlanPane {
                                planModel: planModel
                                selectedOid: page.selectedOid
                                // The pane has nothing of its own yet — or the run is holding the face it had then.
                                // A read that *replaces* a standing plan is not this: those rows still answer a
                                // question somebody asked, and the pane keeps them until the newer answer lands,
                                // which is the model's own rule (`RebasePlanModel::open`).
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
                        // The run button takes the pane's foot while a plan stands, and the foot it takes is this
                        // corner's seat (§コミットメッセージの 2 つの枠「ペインの底は空かない」). Said through the
                        // corner's own property — a `visible` here replaces the one it draws itself by.
                        offered: !page.planActive
                        // Only one of the two panes is on screen at a time, and each measures its own file list.
                        roomLeft: page.wipShown ? wipPane.bottomRoom : detailsPane.bottomRoom
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.rightMargin: Theme.spaceXs
                        anchors.bottomMargin: Theme.spaceXs
                        // Declared before the panes it hangs over, so z holds it in front of whatever they draw here.
                        z: 1
                    }

                    WipPane {
                        id: wipPane
                        anchors.fill: parent
                        visible: page.wipShown
                        repoTab: repoTab
                        workTree: workTree
                        worktreeModel: worktreeModel
                        conflictsModel: conflictsModel
                        stagedModel: stagedModel
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
                        // press-things-here edge moves up with it (§コミットメッセージの 2 つの枠「押す物はペインの底」).
                        anchors.bottomMargin: planRunBar.visible ? planRunBar.height : 0
                        visible: !page.wipShown
                        details: detailsModel
                        chosenRecords: page.chosenRecords
                        rowCardOid: rowHost.rowCardOid
                        onRowHoverRequested: (row, inside) => page.restOnCommit(row, inside)
                        onCommitDropRequested: oidHex => page.dropFromChoice(oidHex)
                        stashRef: page.selectedStashRef
                        menuStanding: page.menuStanding
                        // The one commit an amend reaches, and the line the box gives when this is not it
                        // (`page.messageEdit`). The words are the page's; the rule is core's — and while a plan row
                        // carries `reword`, the same boxes are that row's plan input, because there is one place in
                        // this app to type a message. A plain amend is held down for the plan's whole stay — it is a
                        // queued rewrite of the very history the plan is composed on, which the freeze exists to
                        // stop; on a plan row the way to type is the row's own verb.
                        editable: (page.messageEdit === "amend" && !page.planActive) || page.planReword
                        intoPlan: page.planReword
                        planDraftOid: page.planActive ? planModel.selectedOid : ""
                        planDraftSubject: planModel.selectedMsgSubject
                        planDraftBody: planModel.selectedMsgBody
                        editBlocked: page.planActive && !page.planReword && page.messageEdit !== ""
                            ? qsTr("Mark the row reword to retype its message")
                            : page.messageEdit === "stash"
                              ? qsTr("Rename it in the list on the left")
                              : page.messageEdit === "not-head"
                                ? qsTr("Only the newest commit's message can be rewritten here")
                                : page.messageEdit === "standing"
                                  ? qsTr("Finish the stopped operation first")
                                  : ""
                        busy: repoTab.busyCount > 0
                        published: page.selectedPublished
                        signatureKind: page.selectedSignatureKind
                        signatureCode: page.selectedSignatureCode
                        signatureSigner: page.selectedSignatureSigner
                        // Whom a rewrite would be attributed to: git keeps the author and puts the reader in as
                        // committer, so the save button wears the reader's face rather than the row's.
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
                                // The model took it synchronously — the typed text is the
                                // resting text now (the write path hears this from
                                // `writeReworded` instead).
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
            // Raised when asked for, and by a failure. Its band is the left menu's last row, carried across.
            //
            // **Built when it is raised and taken down when it is shut**, not hidden: the panel is a list with its
            // own hand, a band and a ruler, and a page whose log is down would otherwise carry all of it
            // (rules-refs/app-ui.md — the dialog-seat rule). Nothing is lost in the taking down — the rows and the
            // selection are the model's — and the panel comes back the way it always came up, on its newest row
            // (`CommandsPane.cameUp`). The seat is what stands in the split, so the split's own properties are its.
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
                    onCloseRequested: page.commandsOpen = false
                    onErrorCleared: repoTab.clearLastError()
                    onCopyRequested: text => clipboard.copy(text)
                }
            }
        }
    }

    // A command the user asked for failed. Nothing else on screen says what git said, so the log comes up by itself and
    // stays up — closing it is the reader's call, not the next success's. Unless this page asked for the refusal and
    // turned it into a question: then the bar is already saying it, and the log would say it twice while pushing the
    // graph out of the way.
    //
    // And unless a write is in flight, whose own answer decides instead (デザイン規約 §git が言ったことを読む場所 — 開く判断は
    // 「操作」の答えで下し、コマンド 1 本の終了コードでは下さない). One operation is several commands, so a non-zero one part
    // way through is not a failure yet; and the answer is the only thing that knows whether the far side turned it
    // down with something to report rather than to raise (`absorbWriteResult`, which raises the log itself).
    Connections {
        target: commandsModel
        function onFailure() {
            if (repoTab.writeRunning)
                return
            // The report that answers this one is already standing; this is only its row arriving late. The mark goes
            // back down and the log stays where the reader left it (デザイン規約 §答えの要らない報せ).
            if (page.answeredFailures > 0) {
                page.answeredFailures--
                commandsModel.noteAnswered()
                return
            }
            if (page.expectedRefusals > 0) {
                page.expectedRefusals--
                return
            }
            page.commandsOpen = true
        }
    }

    // The split bars' refusal — collection, settling, overlay and the divider-refuse report all live in the watcher
    // (SplitBarWatch); the page keeps the source chain, since only it sees all four sources.
    SplitBarWatch {
        id: splitWatch
        refusalSource: page.refusalSource
        graphPane: graphPane
    }

    /// Bars announce themselves once, at their own creation (`SplitHandleBar.Component.onCompleted`) — which is before
    /// the watcher above exists, so the page relays and holds the early ones; the page's `Component.onCompleted` hands
    /// them over.
    property var earlyBars: []
    function holdSplitBar(bar, held) {
        if (!splitWatch) {
            page.earlyBars.push(bar)
            return
        }
        splitWatch.holdSplitBar(bar, held)
    }

    /// Automation: the drag past whichever boundary `which` names, and whether that boundary is still drawn
    /// (`PG_AUTO_ACT=divider-refuse`; AutoActDriver calls through the page).
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

    /// What is drawn, not what was asked for — the one badge's own `shown` (`RefusalBadge`, on why not its `visible`).
    readonly property bool refusalShown: splitWatch.shown

    function jumpToRef(oidHex) {
        // While a plan stands the selection has two halves, and they are meant to be the one commit: **this page's**,
        // which is what the plan pane lights a row from (`RebasePlanPane.selectedOid`), and **the model's row**, which
        // is what the boxes are routed by and where a typed reword lands (`planReword` / `planDraftOid`). A jump made
        // straight into this page's half lights the new row and leaves the routing on the old one, and the screen is
        // then about two commits at once.
        //
        // So a hash pressed under a plan is one of two things. **A commit the plan holds** is a walk down the plan,
        // and is taken the way the row click takes it — the model first, the page second (`RebasePlanPane`).
        // **Anything else** is past the base, where this screen has no row to put the reader on; it is refused in
        // silence, the way the mode's other refusals are (`startFind`), because nothing was lost and a hash is not a
        // press that has to be answered. The waiting face holds no rows at all, so every hash there goes the second
        // way.
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
        // A jump is one commit, whatever was being held (デザイン規約 §複数のコミットを選ぶ): the pane on the right is
        // about to describe this one, and rows lit for a choice that no longer stands would say otherwise.
        page.chooseOnly(oidHex)
        // Details resolve even outside the window.
        page.rewordRow = -1
        page.wipShown = false
        // The row travels with the oid: a landing owed later (`followVanishedCommit`) reads `selectedRow`, and a
        // stale one lands the selection on whatever now stands at the previous selection's row.
        page.selectedRow = row
        page.selectedOid = oidHex
        page.selectedStashRef = graphModel.stashRefOf(oidHex)
        detailsModel.request(oidHex)
        page.askSignature(oidHex)
        page.closeDiff()
    }
}
