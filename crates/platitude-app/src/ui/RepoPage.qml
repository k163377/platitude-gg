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
    /// The settings card, opened from an avatar and carrying whom it was opened on.
    signal avatarSettingsRequested(string name, string email)
    /// PG_AUTO_PERF completion after every requested measurement output.
    signal perfFinished()

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
    /// A press landed away from whatever held the keyboard (Main's `FocusRelease`), at `scenePos` — `null` for a press
    /// with no place of its own (the headless run's door). The left menu's name box goes with it — nothing is asked,
    /// what it costs is the typing (デザイン規約 §左メニューの所作) — and so does anything over the graph that is standing on an
    /// empty box, where there is not even that to lose (§コミットを探す).
    function releasePressedAway(scenePos) {
        sidebarPane.stopEdit()
        graphPane.dropEmptyBoxes(scenePos)
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
        page.closeDiff()
        page.refreshHeadPublished()
    }

    // ---- commit editor -------------------------------------------
    property bool amending: false
    property bool headPublished: false
    readonly property string headRange: "HEAD^!"
    function refreshHeadPublished() {
        if (repoTab.state === "open")
            repoTab.checkPublish(page.headRange)
    }
    Connections {
        target: repoTab
        function onChanged() {
            // One shared answer slot, so only the reply to the range this page asked about is read.
            if (repoTab.publishRange === page.headRange)
                page.headPublished = repoTab.publishPublished > 0
            if (commitMenuState.menuOid !== "" && repoTab.publishRange === commitMenuState.menuOid + "^!")
                commitMenuState.menuPublished = repoTab.publishPublished > 0
            if (page.selectedOid !== "" && repoTab.publishRange === page.selectedOid + "^!")
                page.selectedPublished = repoTab.publishPublished > 0
            if (refRowMenu.rebaseRange !== "" && repoTab.publishRange === refRowMenu.rebaseRange)
                refRowMenu.rebasePublished = repoTab.publishPublished > 0
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
    // read, so the panel would come up empty (実測).
    readonly property bool openFailed: !page.blank && repoTab.state === "error"

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
    /// down to the tree, the two parents and the hooks it runs (実測 2.55) — so the message it will use belongs on
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
    /// (デザイン規約 §変更を退避する. 2026-08-22 ユーザー指示).
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
    // commands, so it is the same question with one clause fewer (2026-08-22 ユーザー判断).
    //
    // **Asked before anything is sent.** The screen already knows what is standing, so letting git refuse would put a
    // red line in the command log where a question belongs — and leave the reader where they were with nothing to
    // press (2026-08-22 ユーザー報告).
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
    // was pressed** (デザイン規約 §進行中の操作から出る, 2026-08-22 ユーザー判断).
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
    function switchToRef(kind, name, leaving) {
        if (repoTab.state !== "open" || repoTab.busyCount > 0)
            return
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
            return
        }
        // Ahead of the branches below rather than inside them: the one that lands on an existing local branch asks git
        // what the move would cost before it moves, and that read is worth nothing while an operation is standing.
        if (page.standsInTheWay(leaving)) {
            page.askLeaveOperation(function () { page.switchToRef(kind, name, true) })
            return
        }
        if (action === "switch")
            page.switchTo("branch", name, name, "", leaving)
        else if (action === "materialize")
            page.switchTo("remote", name, local, "", leaving)
        else if (action === "move")
            // Whether to ask is git's to answer — a branch that only fell behind loses nothing by moving. The question
            // comes back as `moveAskSeq` when something would be lost, and the operation is left standing for the
            // answer to that one to undo (core asks before it aborts, never the other way round).
            repoTab.checkoutMovingBranch(local, name, leaving === true)
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
        if (AppBackend.autoAct !== "")
            AppBackend.report("move_branch_asked local=" + local)
    }

    // ---- the standing question --------------------------------------
    // One bar over the graph (デザイン規約 §可否・警告の出し場所); Escape, another ask or a click anywhere else walks away from it.
    // Only questions about a ref reach it: everything that takes one named thing away is held down on the row or button
    // that names it (デザイン規約 §長押し).
    /// Ctrl+F: the find bar belongs to the graph.
    function startFind() {
        graphPane.startFind()
    }

    property var rowAskRun: null
    function startRowAsk(oidHex, label, detail, danger, acceptText, run,
                         hold = false, tip = "", form = null, code = "") {
        // Whoever was dressing the bar lets go first: a question can be raised over one still standing (a ref clicked
        // in the left menu while the first push asks where it goes), and the flow left behind would go on calling
        // itself the one standing — which is what keeps its check timer firing round trips at a remote nobody is
        // asking about. Not for the dress: `startAsking`'s assignments beat a live `Binding` outright (実測
        // 2026-08-27). The flow raising this one turns its own back on after this returns.
        publishFlow.publishAsking = false
        upstreamFlow.asking = false
        page.rowAskRun = run
        graphPane.startAsking(oidHex, label, detail, acceptText, danger, hold, tip, form, code)
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
        onAskRequested: (label, run, form, code) =>
            page.startRowAsk("", label, "", false, "", run, false, "", form, code)
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
    /// drawn over the very rows the hand is reading (2026-08-17 ユーザー報告). The menus are all the page's, so this is
    /// the one place that can see all of them.
    readonly property bool menuStanding:
        refRowMenu.showing || commitRowMenu.showing || fileRowMenu.showing || remoteRowMenu.showing
        || diffRowMenu.showing

    // ---- context menu on a sidebar row ------------------------------
    RefRowMenu {
        id: refRowMenu
        repoTab: repoTab
        workTree: workTree
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
    // What a remote itself offers. Its own menu rather than rows added to the one above: a remote is repository
    // configuration, and the ref menu is about refs (デザイン規約 §左メニューの所作).
    RemoteRowMenu {
        id: remoteRowMenu
        repoTab: repoTab
        onUrlRequested: name => publishFlow.startEditRemote(name)
        onDismissed: rowHost.settleRefList()
    }
    /// The one door into that menu. Says whether it opened.
    function openRemoteMenu(name) {
        return remoteRowMenu.offerOn(name)
    }

    /// Which surface raised the standing ref menu. Its branch row opens a box to type a name in, and that box belongs
    /// on the row the hand is already on — the same reason the box is in the chip column and not over the window
    /// (デザイン規約 §可否・警告の出し場所).
    property bool refMenuInSidebar: false
    /// The one door into that menu: the sidebar's rows, a chip, the stacked list and the automation all come through
    /// here. Says whether it opened.
    function openRefMenu(kind, name, full, oidHex, inSidebar) {
        page.refMenuInSidebar = inSidebar === true
        return refRowMenu.offerOn(kind, name, full, oidHex)
    }
    /// A new branch on a commit, asked for from a menu: the name box opens where that menu was raised. Nothing is
    /// created until it is submitted — walking away costs the typing and nothing else.
    function startBranchAt(oidHex) {
        if (oidHex === "")
            return
        if (page.refMenuInSidebar)
            sidebarPane.beginBranchAt(refRowMenu.kind, refRowMenu.refId, oidHex)
        else
            graphPane.startNaming(oidHex)
    }
    /// And a tag on that commit, through the same two doors and on the same terms.
    function startTagAt(oidHex) {
        if (oidHex === "")
            return
        if (page.refMenuInSidebar)
            sidebarPane.beginTagAt(refRowMenu.kind, refRowMenu.refId, oidHex)
        else
            graphPane.startTagging(oidHex)
    }

    /// A chip's right-click: the ref menu for the name the chip shows. A chip that names nothing to act on — the
    /// detached-HEAD marker, a stash, the current branch (whose ref menu has no rows) — falls back to the row's commit
    /// menu. The stacked list's rows pass no `oidHex` and have no row to fall back to (デザイン規約 §メニュー).
    function openRecordMenu(record, oidHex) {
        const kind = GitFacts.recordKind(record)
        if (kind === "") {
            if (oidHex !== "")
                page.openRowMenu(oidHex)
            return
        }
        const name = GitFacts.recordName(record)
        const oid = kind === "branch" ? branchesModel.oidOfName(name)
                  : kind === "remote" ? remotesModel.oidOfName(name)
                  : tagsModel.oidOfName(name)
        if (!page.openRefMenu(kind, name, name, oid, false) && oidHex !== "")
            page.openRowMenu(oidHex)
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
            page.askRenameRemote(id, name)
        }
    }

    /// The remote branch a just-renamed local one spoke for, and the name it took — the question about carrying the
    /// name over waits until git says the local rename landed.
    property string pendingRenameRemote: ""
    property string pendingRenameTo: ""

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
        if (AppBackend.autoAct !== "")
            AppBackend.report("rename_remote_asked from=" + remoteRef + " to=" + name)
    }

    /// A branch is deleted with `-d`, and git's refusal is the question — asked when it arrives rather than guessed at
    /// beforehand (デザイン規約 §左メニューの所作). A tag and a stash have no such refusal in git, so they are asked about up front.
    /// A branch on a remote never arrives here (`deleteRemoteNow`).
    property string pendingDeleteBranch: ""
    /// Refusals this page already has an answer for. The question bar explains them, so the command log stays where it
    /// was rather than raising itself over the same news (デザイン規約 §git が言ったことを読む場所). Counted rather than flagged
    /// because the refusal and the write result arrive on separate paths, in no fixed order.
    property int expectedRefusals: 0

    // ---- what the window is already showing as gone ------------------
    //
    // **A delete takes the row away at the press, and git is asked behind it** (デザイン規約 §消す操作は先に画面から消す).
    // The write itself is the short half: `git branch -d` is one process, and everything the reader is actually
    // waiting on comes after it — the refs read, then the walk that rebuilds the graph (kotlin 級で 54ms のあとに
    // 1.3s、`busyCount` はその先頭しか覆わない — 2026-08-22 実測). Left to those, the row sits there through all of it
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
    /// repository answers in tens of milliseconds, which is over before the picture is grabbed. Latched at the press
    /// itself, which is the real edge rather than a stand-in for one (verify-ui スキル §壊れない動詞の実装と反復).
    property bool goneHeldForShot: false

    /// Takes a row away before git has answered for it. `id` is what git is being asked to delete, which is also what
    /// the row is keyed by — so a refusal puts back exactly what was taken.
    function showGone(kind, id) {
        page.goneAtSeq = repoTab.writeSeq
        if (AppBackend.autoAct === "delete-gone")
            page.goneHeldForShot = true
        if (kind === "branch")
            page.goneBranch = id
        else if (kind === "remote")
            page.goneRemote = id
        else if (kind === "tag")
            page.goneTag = id
        else if (kind === "stash")
            page.goneStash = id
        if (AppBackend.autoAct !== "")
            AppBackend.report("gone_shown kind=" + kind + " id=" + id)
    }
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
        page.goneAtSeq >= 0 && repoTab.writeSeq > page.goneAtSeq && !page.goneHeldForShot
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

    function deleteRow(kind, id, name, oidHex) {
        if (kind !== "branch")
            return
        // The one row in any menu that outlives its own write (it stays open for git's answer), so it is also the one
        // that can be clicked twice — the second click is the same request again.
        if (repoTab.busyCount > 0)
            return
        page.pendingDeleteBranch = id
        page.expectedRefusals++
        page.showGone("branch", id)
        repoTab.deleteBranch(id, false)
    }
    /// Held, not asked (デザイン規約 §長押し).
    function dropStashNow(ref) {
        page.showGone("stash", ref)
        repoTab.dropStash(ref)
        if (page.selectedStashRef === ref)
            page.selectedStashRef = ""
    }
    /// git refused the plain delete: the answer lands on the menu row that asked, turning it into a held one (デザイン規約
    /// §左メニューの所作). A refusal does not mean the commits stop being reachable: git measures the branch against its
    /// upstream when it has one, so a branch merged into HEAD but not yet pushed is refused while nothing at all would
    /// be lost (実測).
    function noteForceDelete(name) {
        // Whichever card asked — the ref menu's or the graph row's — is the one the answer belongs on. They are the
        // same component, and only one of them is ever standing.
        refRowMenu.branchCard.noteRefused(name)
        commitRowMenu.branchCard.noteRefused(name)
        if (AppBackend.autoAct !== "")
            AppBackend.report("force_delete_offered branch=" + name)
    }

    // ---- context menu on a working-tree file row --------------------
    function openFileMenu(bucket, path) {
        // A right-click is a click: it walks away from a question that was standing, which may well be about another
        // row.
        page.stopRowAsk()
        fileRowMenu.offer(bucket, path)
    }
    FileRowMenu {
        id: fileRowMenu
        repoTab: repoTab
        workTree: workTree
        wipPane: wipPane
        onMergeToolWanted: page.settingsDialogRequested()
        onCopyRequested: text => clipboard.copy(text)
    }

    // ---- context menu on the diff's own text ------------------------
    /// The one door into that menu. What it will act on is the selection, and the hand settled that before asking
    /// (`DiffTextSelect.askMenu`), so nothing about the row travels here.
    function openCodeMenu() {
        page.stopRowAsk()
        diffRowMenu.offer()
    }
    DiffRowMenu {
        id: diffRowMenu
        diffModel: diffModel
        onCopyRequested: text => clipboard.copy(text)
    }

    // ---- context menu on a graph row -------------------------------
    /// The one door into that menu: the graph's rows, a chip that names nothing to act on, and the automation all come
    /// through here.
    /// The branch a graph row carries, in the words the chip records use — the first one the row shows, which is the
    /// one a double-click on it already goes to (`GraphRowDelegate.primaryRecord`). The rows already taken off the
    /// screen ahead of git's answer are filtered out the same way the delegate filters them, so a chip that is gone
    /// does not put a card up over a branch that is not there any more (デザイン規約 §消す操作は先に画面から消す).
    function branchRecordAt(oidHex) {
        const row = graphModel.rowOf(oidHex)
        if (row < 0)
            return ""
        const shown = GitFacts.labelsShown(graphModel.labelsAt(row), graphModel.goneChips)
        if (shown === "")
            return ""
        const records = shown.split(String.fromCharCode(31))
        for (let i = 0; i < records.length; i++) {
            const kind = GitFacts.recordKind(records[i])
            if (kind === "branch" || kind === "remote")
                return records[i]
        }
        return ""
    }

    function openRowMenu(oidHex) {
        // The row's own branch, so the card at the foot of its menu is the card that branch's chip opens — the two
        // are ways at the same thing (2026-08-26 ユーザー判断).
        const record = page.branchRecordAt(oidHex)
        commitRowMenu.rowBranchKind = GitFacts.recordKind(record)
        commitRowMenu.rowBranch = record === "" ? "" : GitFacts.recordName(record)
        commitMenuState.openRowMenu(oidHex)
    }

    CommitMenuState {
        id: commitMenuState
        repoTab: repoTab
        workTree: workTree
        graphModel: graphModel
        menu: commitRowMenu
    }

    CommitRowMenu {
        id: commitRowMenu
        repoTab: repoTab
        workTree: workTree
        branchesModel: branchesModel
        worktreesModel: worktreesModel
        branch: workTree.branch
        oid: commitMenuState.menuOid
        stashRef: commitMenuState.menuStashRef
        published: commitMenuState.menuPublished
        canSequence: commitMenuState.menuCanSequence
        canIntegrate: commitMenuState.menuCanIntegrate
        canEditHistory: commitMenuState.menuCanEditHistory
        canMoveBranch: commitMenuState.menuCanMoveBranch
        canBranchHere: commitMenuState.menuCanBranchHere
        stashCanWrite: commitMenuState.menuStashCanWrite
        // Straight to the graph row: this menu is only ever raised on one.
        onBranchHereRequested: oidHex => graphPane.startNaming(oidHex)
        onTagHereRequested: oidHex => graphPane.startTagging(oidHex)
        onSquashRequested: oidHex => page.squashCommit(oidHex)
        onDropRequested: oidHex => page.dropCommit(oidHex)
        onResetRequested: mode => page.moveBranchHere(mode)
        onApplyStashRequested: selector => repoTab.applyStash(selector)
        onPopStashRequested: selector => page.popStash(selector)
        onDropStashRequested: selector => page.dropStashNow(selector)
        // The branch card's own three, answered exactly where the ref menu's are.
        onDeleteRequested: (kind, id, name, oidHex) => page.deleteRow(kind, id, name, oidHex)
        onDeleting: (kind, id) => page.showGone(kind, id)
        onUpstreamRequested: (branch, counterpart) => page.startUpstreamAsk(branch, counterpart)
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
        const typed = graphPane.namingText
        if (typed.trim() === "")
            return qsTr("A name is needed")
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
        if (repoTab.busyCount === 0)
            graphPane.startNaming(oidHex)
    }

    // What a resting pointer opens on a graph row: the row's own card, and the refs one chip had to stack. Owned here
    // because rows are recycled out from under both of them.
    RowHoverHost {
        id: rowHost
        graphPane: graphPane
        currentBranch: workTree.branch
        menuStanding: refRowMenu.opened
        hoverBlocked: page.menuStanding
        onRecordActivated: record => page.activateRecord(record)
        // A click in the card is a click on the row it is standing on: every name in it is on that one commit.
        onRecordChosen: (oidHex, atRow) => page.activateRow(oidHex, atRow)
        onRecordMenuAsked: record => page.openRecordMenu(record, "")
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

    // Whether the selected commit is one HEAD was built on. Only those can be amended or replayed from here, so the
    // boxes stay read-only until this comes back for the commit on screen.
    readonly property bool selectedInHistory:
        detailsModel.shaHex !== "" && repoTab.historyOid === detailsModel.shaHex && repoTab.historyIn
    function askInHistory(oidHex) {
        if (oidHex !== "" && repoTab.state === "open")
            repoTab.checkInHistory(oidHex)
    }

    // What git makes of the selected commit's signature. Asked on every selection, like the history question beside it,
    // and read only when the answer names the commit now on screen — verifying runs gpg or ssh-keygen, so the answer
    // arrives well after the details do. A signature only changes when the commit does, and a changed commit is a
    // different hash, so nothing has to ask twice.
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

    // Whether a remote already has the selected commit — what the save row's warning rests on. Asked only once its
    // message is touched: that is the first moment the answer can matter, and it spares a rev-list on every selection
    // click.
    property bool selectedPublished: false
    function askSelectedPublished() {
        if (page.selectedOid !== "" && repoTab.state === "open")
            repoTab.checkPublish(page.selectedOid + "^!")
    }
    onSelectedOidChanged: page.selectedPublished = false

    // **Moving off a half-written message drops it, and nothing asks** (デザイン規約 §コミットメッセージの 2 つの枠).
    // A message that has not been saved is a draft of somebody else's commit; the way to keep it is to press the
    // button, and the two ways to drop it — Escape, and reading another commit — are both things the reader did on
    // purpose. So nothing stands between a click in the graph and the commit it lands on.
    Connections {
        target: detailsPane
        // The first keystroke is the first moment "is this on a remote" can matter, and it spares a rev-list on every
        // selection click.
        function onMessageDirtyChanged() {
            if (detailsPane.messageDirty)
                page.askSelectedPublished()
        }
    }

    // An assignment is not something git knows about, so nothing here is waiting for a refresh to bring it: the rows
    // and the card re-read the store themselves.
    Connections {
        target: AppBackend
        function onAvatarsChanged() {
            graphModel.refreshAvatars()
            detailsModel.refreshAvatar()
            // The commit editor wears the identity's own face, which is reached by the same badge.
            repoTab.refreshAvatar()
        }
    }

    // ---- smoke hook ------------------------------------------------
    // The whole of this page's PG_AUTO_ACT harness, built only when a verb was given so an ordinary run carries none of
    // it. A file of its own cannot see this one's ids, so everything the verbs act on is named here — an
    // automation-only exposure, the same one `GraphPane.view` is (app-ui.md).
    Loader {
        id: autoActLoader
        active: AppBackend.autoAct !== ""
        sourceComponent: AutoActDriver {
            page: page
            repoTab: repoTab
            workTree: workTree
            graphModel: graphModel
            detailsModel: detailsModel
            branchesModel: branchesModel
            remotesModel: remotesModel
            worktreeModel: worktreeModel
            stashesModel: stashesModel
            tagsModel: tagsModel
            graphPane: graphPane
            sidebarPane: sidebarPane
            detailsPane: detailsPane
            diffPane: diffPane
            wipPane: wipPane
            gitCorner: gitCorner
            refMenu: refRowMenu.menu
            refBranchCard: refRowMenu.branchCard
            refTagCard: refRowMenu.tagCard
            refDeleteItem: refRowMenu.deleteItem
            refUpstreamItem: refRowMenu.upstreamItem
            refStashDropItem: refRowMenu.stashDropItem
            refSwitchItem: refRowMenu.switchItem
            refPushTagItem: refRowMenu.pushTagItem
            refTagDeleteItem: refRowMenu.deleteTagItem
            refTagHereItem: refRowMenu.tagHereItem
            refRemoteTagDeleteItem: refRowMenu.deleteRemoteTagItem
            refTagBothDeleteItem: refRowMenu.deleteTagBothItem
            fileRowMenu: fileRowMenu
            fileMenu: fileRowMenu.menu
            fileDiscardItem: fileRowMenu.discardItem
            diffRowMenu: diffRowMenu
            clipboard: clipboard
            commitMenuState: commitMenuState
            commitMenu: commitRowMenu.menu
            dropCommitItem: commitRowMenu.dropItem
            tagHereCommitItem: commitRowMenu.tagHereItem
            stashDeleteItem: commitRowMenu.stashDropItem
            resetMenu: commitRowMenu.resetSubmenu
            commitBranchCard: commitRowMenu.branchCard
            commitTagCard: commitRowMenu.tagCard
            hardResetItem: commitRowMenu.hardResetRow
            publishFlow: publishFlow
            upstreamFlow: upstreamFlow
            remoteDialog: publishFlow.dialog
            remoteMenu: remoteRowMenu.menu
            refList: rowHost.listPopup
            rowCard: rowHost.hoverCard
        }
    }

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
            // The one refusal this page has a second move for: a branch delete git would not do on its own.
            if (repoTab.writeBranchOp && page.pendingDeleteBranch !== "") {
                const refused = page.pendingDeleteBranch
                page.pendingDeleteBranch = ""
                page.noteForceDelete(refused)
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
        // Landed: no refusal is coming for it after all, so the menu left standing to catch one has nothing left to
        // say.
        if (page.pendingDeleteBranch !== "") {
            page.pendingDeleteBranch = ""
            page.expectedRefusals = Math.max(0, page.expectedRefusals - 1)
            refRowMenu.close()
        }
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
        // git stopped part-way and left the operation standing, so there is no commit at the tip to land on and the
        // answer to the press is the working tree: the conflicted rows, and the way out under them (デザイン規約
        // §進行中の操作から出る). Armed rather than done on the spot, for the reason the head landing below is: the
        // status that will carry those rows has not arrived yet (2026-08-22 ユーザー要望).
        if (repoTab.lastWriteStopped)
            page.pendingWipSelect = true
        // The answer is a commit at the tip, and that commit is what was asked for here, not the row or the ref that
        // was clicked. The selection goes to it and the viewport follows: what was clicked can be anywhere in the
        // history, while the answer is always at the top.
        else if (repoTab.writeAtTip) {
            page.pendingHeadSelect = true
            page.pendingHeadSeenSeq = workTree.statusSeq
            page.pendingHeadAsked = true
        }
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
        page.refreshHeadPublished()
        // HEAD may have moved: what the selected commit is to it — and so whether its message is ours to rewrite — is
        // asked again.
        page.askInHistory(page.selectedOid)
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
            diffModel.requestCommitFile(detailsModel.shaHex, detailsModel.parentHex, path, origPath)
        else
            diffModel.requestWorkTree(kind, path, origPath)
        page.diffShown = true
        page.diffNeighbour = ""
        page.noteDiffNeighbour()
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
    /// replaced and came back with a refusal in the log (2026-08-17 ユーザー報告).
    ///
    /// Three parts, in the order they happen, and **every one of them ends by itself**: the press is out and no answer
    /// has come (`diffAwaits`, armed only when the tab says a write went out, put down by the write's own answer), git
    /// is running (`busyCount`, which the session balances), the file is being read again (`loading`, put down by the
    /// rows arriving).
    ///
    /// **Nothing here waits on a signal that may not come.** Held on "the tree has not been read yet" instead, it
    /// wedged for good the first time a write moved no rows — staging a second line of a file already on both sides —
    /// because the file list only says `changed` when its rows differ, and then no `+` anywhere would go in again
    /// (2026-08-17 ユーザー報告).
    property bool diffAwaits: false
    readonly property bool diffSettling:
        page.diffAwaits || repoTab.busyCount > 0 || diffModel.loading
    /// A write on the working tree has landed, so the open diff is a picture of what the file used to be.
    ///
    /// **Whoever wrote it.** This was once asked for by the writes made inside the diff itself, and the file list's own
    /// `+` and `−` moved the same file out from under the pane without a word: a line staged here and then unstaged
    /// there left the line missing from both sides on screen (2026-08-17 ユーザー報告). The caller already knows the write
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
    /// conflict it was opened on (2026-08-22 ユーザー報告). Core answers this with silence unless the bytes moved
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
    /// Automation: the colour the `>_` painted — the log's own band while it is up, the pane's foot while it is down.
    readonly property color commandsMarkColor: page.commandsOpen ? commandsPane.markColor : sidebarPane.commandsMarkColor
    /// What the panel is doing rather than what was asked of it — automation reads this one, so a cut binding fails.
    readonly property bool commandsShown: commandsPane.visible
    /// Automation reads the laid-out width, not the preferred width it requested, before persisting a state round trip.
    readonly property real stateDetailsWidth: rightPane.width
    /// Automation only: the header's `Clear`, pressed from outside the panel (`PG_AUTO_ACT=commands-clear`).
    function clearCommandLog() {
        commandsPane.clearPanel()
    }

    RepoTab { id: repoTab }
    CommandsModel { id: commandsModel }
    GraphModel { id: graphModel }
    WorkTreeModel { id: workTree }
    DetailsModel { id: detailsModel }
    DiffModel { id: diffModel }
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
        commandsPane: commandsPane
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
        branchesModel.attachSection(page.tab_id, "branches")
        remotesModel.attachSection(page.tab_id, "remotes")
        conflictsModel.attachWorktree(page.tab_id, "conflicts")
        worktreeModel.attachWorktree(page.tab_id, "unstaged")
        stagedModel.attachWorktree(page.tab_id, "staged")
        worktreesModel.attachSection(page.tab_id, "worktrees")
        stashesModel.attachSection(page.tab_id, "stashes")
        tagsModel.attachSection(page.tab_id, "tags")
        page.restoreDraft()
        if (autoActLoader.item)
            autoActLoader.item.begin()
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
    /// One tick: the repository, and the file the diff pane is holding. The two are separate reads because they answer
    /// different questions — refs and status say what the tree is, the diff says what the file says.
    function pollRepo() {
        repoTab.refreshPoll()
        page.pollDiff()
    }

    // Where the selection stands, kept so a commit that disappears from under it can be followed to whatever took its
    // place.
    property int selectedRow: -1

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
    /// The `WorkTreeModel.statusSeq` the arming saw. The detached fallback below may only read a status that arrived
    /// after it — the messages in flight at the arming still describe the repository as it was, and a stale HEAD
    /// still has a row to land on.
    property int pendingHeadSeenSeq: -1
    // Whether the landing is one the person here asked for, in which case the viewport goes to it as well: a commit
    // they meant to make is not an answer if it lands off screen. The other two ways this is set happen *to* the window
    // — a commit made in a terminal, a rewrite that swept the selected commit away while the poll was watching — and a
    // background pass that moves rows under a reader may not also move their view
    // (§ListView.highlightFollowsCurrentItem).
    property bool pendingHeadAsked: false
    /// Whether the last write this window sent was a stash that landed — read where the working tree turns out to be
    /// empty, which is the moment that says the entry took all of it (`worktreeModel.onChanged`).
    property bool stashLanded: false
    function tryPendingHeadSelect() {
        if (!page.pendingHeadSelect || !branchesModel.refsLoaded)
            return
        // The branches section answers only for a branch; detached, HEAD is still somewhere, and the status model is
        // the one that says where (`WorkTreeModel.headOid` — "branch or not"). Without the fallback a landing owed
        // after a write made detached never resolves — but only a status that arrived after the arming may answer
        // (`statusSeq`): the one in flight still describes the repository as it was, and its stale HEAD has a row.
        const statusFresh = workTree.statusSeq !== page.pendingHeadSeenSeq
        const head = branchesModel.headOid !== "" ? branchesModel.headOid
                   : statusFresh ? workTree.headOid : ""
        const row = head !== "" ? graphModel.rowOf(head) : -1
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
        page.pendingHeadSeenSeq = workTree.statusSeq
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
        const row = atRow !== undefined && atRow >= 0 ? atRow : graphModel.rowOf(oidHex)
        if (row >= 0) {
            graphPane.setCurrentRow(row)
            page.selectedRow = row
        }
        // **The working tree's row is not a hash**, so nothing below can be skipped for it the way it can for a
        // commit: what it shows is whatever the tree is now.
        if (GitFacts.wipOid(oidHex)) {
            page.showWip()
            return
        }
        // The commit already open. **Everything below is of this commit and has been asked once**: the details, the
        // history question and the signature are answers to a hash, and a hash cannot have changed under the same row
        // — asking again spends a `git show`, a `merge-base` and a gpg run per click for an answer already on screen
        // (the find bar says the same of landing twice on one row). The second click of the rename gesture is exactly
        // this click, so the wait it opens would be spent on work nobody is waiting for (デザイン規約 §グラフ行のダブルクリック).
        if (!page.wipShown && page.selectedOid === oidHex)
            return
        page.wipShown = false
        page.selectedOid = oidHex
        page.selectedStashRef = graphModel.stashRefOf(oidHex)
        detailsModel.request(oidHex)
        page.askInHistory(oidHex)
        page.askSignature(oidHex)
        page.closeDiff()
    }

    // Selection policy: restore across the tag-swap reset, and default to the current branch's newest commit on first
    // load so the details pane always shows something.
    function trySelectDefault() {
        if (page.selectedOid !== "" || page.wipShown || page.pendingHeadSelect
                || AppBackend.autoSelect || AppBackend.autoWip
                || graphModel.rowTotal === 0)
            return
        // Refs decide which commit is "current" — wait for them instead of guessing the newest row too early.
        if (!branchesModel.refsLoaded)
            return
        let row = branchesModel.headOid !== "" ? graphModel.rowOf(branchesModel.headOid) : -1
        if (row < 0) {
            if (graphModel.loading)
                return // the head row may still be streaming in
            row = 0 // detached / head outside the window: newest commit
        }
        graphPane.setCurrentRow(row)
        graphPane.anchorSoon()
        page.activateRow(graphModel.oidAt(row))
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
            }
            page.trySelectDefault()
        }
    }
    Connections {
        target: branchesModel
        function onChanged() {
            // Refs can be the half that was missing, when the walk had already delivered the commit they now point at.
            page.tryPendingHeadSelect()
            page.trySelectDefault()
        }
        // Refs that settled without moving say nothing through `changed` (identical rows are deliberately quiet — see
        // the model), and a write that recorded nothing leaves them exactly so: a cherry-pick of a commit this branch
        // already has owes the same landing as one that wrote a commit, and this is the only word that it can be paid.
        function onRefsSettled() {
            page.tryPendingHeadSelect()
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
            // The status that follows this window's own write: the file was read when the write answered.
            const ours = page.diffReadAt === repoTab.writeSeq
            page.diffReadAt = -1
            // Something outside this window moved the tree, so the rows on screen — and the fingerprint the next `+`
            // would be written against — are a picture of the file as it was. Pressing one then came back with git's
            // refusal (2026-08-17 ユーザー報告).
            if (moved && !ours)
                page.reloadDiff()
            page.absorbOpMessage()
            // The status can be the half a detached landing was waiting on (`tryPendingHeadSelect`'s fallback reads
            // this model, and only a status younger than the arming may answer). **Only the detached half**: with a
            // branch name standing, resolution stays with the refs/graph events — a status that arrives first would
            // otherwise hand the still-stale `branchesModel.headOid` a row to land on, which is the exact stale read
            // the pending flag exists to wait out.
            if (branchesModel.headOid === "")
                page.tryPendingHeadSelect()
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
            // The working tree emptied. After a commit of our own that is the end of the editor's job; when someone
            // else committed these changes it happens with no warning, so a message being written stays on screen with
            // its text — it is the one thing here that cannot be read back off disk. Otherwise land on the commit that
            // now holds the changes rather than on nothing.
            //
            // **A stash pressed here is not held by that.** The press *is* the decision to empty the tree, so the box
            // is not a reason to stay: the words are still in it when the working tree comes back (the pane is hidden,
            // not unloaded) and the entry took them for its own name on the way out. Left to the message alone, the
            // one press that always has words in front of it — a stopped merge fills the box itself
            // (`absorbOpMessage`) — is the one that never lands, and the pane stands over a tree it no longer
            // describes while the highlight the working-tree row left behind is inherited by whatever slid into its
            // place, which after this press is the entry it just made (2026-08-22 ユーザー報告).
            const ourStash = page.stashLanded
            if (worktreeModel.total === 0 && page.wipShown
                    && (ourStash || (wipPane.subjectText === "" && wipPane.bodyText === ""))) {
                page.stashLanded = false
                page.wipShown = false
                page.pendingHeadSelect = true
                page.pendingHeadSeenSeq = workTree.statusSeq
                // Only ours is a landing anybody asked for, so only ours takes the viewport along.
                page.pendingHeadAsked = ourStash
            }
            // Smoke hook (PG_AUTO_WIP=1): open the WIP view once uncommitted changes are known.
            if (AppBackend.autoWip && worktreeModel.total > 0 && !page.wipShown) {
                graphPane.setCurrentRow(0)
                page.showWip()
            }
        }
    }

    // Smoke hook (PG_SCROLL_TO=top|bottom|nav-bottom): jump the graph — or the sidebar's branch list — after the final
    // pass settles, using the same clamped math as the wheel. Parks the view once and then stays out of the way:
    // re-running on every pass would drag a background refresh back to the edge, which is the one thing a scrolled view
    // must not do on its own.
    property bool scrolledTo: false
    Timer {
        id: scrollToTimer
        interval: 600
        onTriggered: {
            page.scrolledTo = true
            if (AppBackend.scrollTo === "nav-bottom") {
                sidebarPane.scrollBranchesToEnd()
                return
            }
            graphPane.view.contentY = graphPane.view.clampY(AppBackend.scrollTo === "bottom" ? 1e12 : -1e12)
        }
    }
    Connections {
        target: graphModel
        enabled: AppBackend.scrollTo !== "" && !page.scrolledTo
        function onStatsChanged() {
            if (graphModel.finishCount > 0)
                scrollToTimer.restart()
        }
    }

    Loader {
        active: AppBackend.autoPerf && !page.blank
        sourceComponent: PagePerfDriver {
            page: page
            repoTab: repoTab
            graphModel: graphModel
            worktreeModel: workTree
            branchesModel: branchesModel
            detailsModel: detailsModel
            diffModel: diffModel
            graphPane: graphPane
        }
    }

    // Automation (PG_AUTO_SELECT=1): select the newest commit, then open the first changed file's diff — exercises the
    // full pipeline for screenshot-based smoke tests.
    property bool autoSelected: false
    Connections {
        target: graphModel
        enabled: AppBackend.autoSelect
        function onStatsChanged() {
            if (page.autoSelected || graphModel.rowTotal === 0)
                return
            // **The newest commit, not the newest row.** A dirty working tree puts the WIP row on top, and selecting
            // that one shows the pending changes instead of a commit — no details are asked for, so a measurement that
            // reads the interaction budget off this hook measures nothing and says so (`xtask perf`'s `missing`). Every
            // demo repository is dirty.
            for (let row = 0; row < graphModel.rowTotal; row++) {
                const oid = graphModel.oidAt(row)
                if (oid !== "" && !GitFacts.wipOid(oid)) {
                    page.autoSelected = true
                    graphPane.setCurrentRow(row)
                    page.activateRow(oid)
                    return
                }
            }
        }
    }
    Connections {
        target: detailsModel
        enabled: AppBackend.autoSelect
        function onChanged() {
            if (detailsModel.shaHex !== "" && detailsModel.fileTotal > 0 && diffModel.title === "")
                page.toggleDiff("commit", detailsModel.filePathAt(0), detailsModel.fileOrigPathAt(0))
        }
    }

    /// The failed-open screen's "Close tab" button.
    signal closeTabRequested()

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

                // Center: commit graph ⇄ file diff
                StackLayout {
                    SplitView.fillWidth: true
                    // Not a number of its own: what the graph's own columns come to once they have both given
                    // everything they can, held up to a side pane's width so the middle never reads as the thinnest of
                    // the three (`PageLayout.centreMinWidth`).
                    SplitView.minimumWidth: pageLayout.centreMinWidth
                    currentIndex: page.diffShown ? 1 : 0

                    GraphPane {
                        id: graphPane
                        graphModel: graphModel
                        workTree: workTree
                        blank: page.blank
                        chipListAnchor: rowHost.refListAnchor
                        rowCardOid: rowHost.rowCardOid
                        onRowActivated: (oidHex, atRow) => page.activateRow(oidHex, atRow)
                        // The bar moves between matches, not between commits — landing on the same row twice changes
                        // nothing and costs no git.
                        onFindLanded: oidHex => {
                            if (oidHex !== "" && oidHex !== page.selectedOid)
                                page.activateRow(oidHex)
                        }
                        onRowMenuOpenRequested: oidHex => page.openRowMenu(oidHex)
                        onChipMenuOpenRequested: (oidHex, record) => page.openRecordMenu(record, oidHex)
                        onRowSwitchRequested: (oidHex, record) => page.rowDoubleClicked(oidHex, record)
                        onRowRenameRequested: (oidHex, record) => page.startRename(oidHex, record)
                        onRenameSubmitted: (kind, id, name) => page.renameRow(kind, id, name)
                        namingRefused: page.graphNameRefusedWhy !== ""
                        namingRefusedWhy: page.graphNameRefusedWhy
                        onChipExpandRequested: (oidHex, atRow, records, anchor) =>
                            rowHost.openRefList(oidHex, atRow, records, anchor)
                        onChipCollapseRequested: rowHost.closeRefListUnlessEntered()
                        onRowHoverRequested: (row, inside) => {
                            rowHost.rowCardWanted = inside
                            if (inside)
                                rowHost.openRowCard(row)
                            else
                                rowHost.settleRowCard()
                        }
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
                        visible: !page.wipShown
                        details: detailsModel
                        stashRef: page.selectedStashRef
                        menuStanding: page.menuStanding
                        // Only what the working tree stands on: a commit off this line cannot be amended or replayed
                        // from here, and a stash is a commit but never one of them.
                        editable: !page.blank && repoTab.state === "open"
                                  && page.selectedStashRef === ""
                                  && page.selectedInHistory
                        editBlocked: page.selectedStashRef !== ""
                            ? qsTr("Rename it in the list on the left")
                            : (detailsModel.shaHex !== "" && !page.selectedInHistory
                               ? qsTr("Not in the current history — switch to a branch that has it")
                               : "")
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
                        // HEAD's own commit, not the current branch's tip: detached, there is no branch to ask.
                        headOid: workTree.headOid
                        readPath: page.diffKind === "commit" ? page.diffPath : ""
                        onMessageSubmitted: (oidHex, subject, body) => page.saveMessage(oidHex, subject, body)
                        onFileActivated: (path, origPath) => page.toggleDiff("commit", path, origPath)
                        onFileWalked: (path, origPath) => page.openDiff("commit", path, origPath)
                        onParentClicked: oidHex => page.jumpToRef(oidHex)
                        // The badge's press goes to the window, which owns the settings card.
                        onAvatarEditRequested: (name, email) => page.avatarSettingsRequested(name, email)
                        onCopyRequested: text => clipboard.copy(text)
                        onApplyStashRequested: selector => repoTab.applyStash(selector)
                        onPopStashRequested: selector => page.popStash(selector)
                    }
                }
            }

            // ---- command log ------------------------------------------
            // Hidden until asked for, and raised by a failure. Its band is the left menu's last row, carried across.
            CommandsPane {
                id: commandsPane
                visible: page.commandsOpen
                curPage: page
                commandsModel: commandsModel
                errorText: repoTab.lastError
                SplitView.preferredHeight: 280
                SplitView.minimumHeight: pageLayout.commandsMinHeight
                onCloseRequested: page.commandsOpen = false
                onErrorCleared: repoTab.clearLastError()
                onCopyRequested: text => clipboard.copy(text)
            }
        }
    }

    // A command the user asked for failed. Nothing else on screen says what git said, so the log comes up by itself and
    // stays up — closing it is the reader's call, not the next success's. Unless this page asked for the refusal and
    // turned it into a question: then the bar is already saying it, and the log would say it twice while pushing the
    // graph out of the way.
    Connections {
        target: commandsModel
        function onFailure() {
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

    /// Automation: the drag and its answer for every boundary in the window (`PG_AUTO_ACT=divider-refuse`;
    /// AutoActDriver calls through the page).
    function reportDividerRefusal(which) {
        splitWatch.reportDividerRefusal(which)
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
        const row = graphModel.rowOf(oidHex)
        if (row >= 0)
            graphPane.jumpToRow(row)
        // Details resolve even outside the window.
        page.rewordRow = -1
        page.wipShown = false
        // The row travels with the oid: a landing owed later (`followVanishedCommit`) reads `selectedRow`, and a
        // stale one lands the selection on whatever now stands at the previous selection's row.
        page.selectedRow = row
        page.selectedOid = oidHex
        page.selectedStashRef = graphModel.stashRefOf(oidHex)
        detailsModel.request(oidHex)
        page.askInHistory(oidHex)
        page.askSignature(oidHex)
        page.closeDiff()
    }
}
