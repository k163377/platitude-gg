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
    /// PG_AUTO_ACT=settings wants the window's settings dialog open
    /// for the screenshot.
    signal settingsDialogRequested()
    /// The settings card, opened from an avatar and carrying whom it was
    /// opened on.
    signal avatarSettingsRequested(string name, string email)
    /// PG_AUTO_PERF completion after every requested measurement output.
    signal perfFinished()

    property string selectedOid: ""

    property bool sidebarCollapsed: false
    /// The fold the diff put on: closing the diff takes back only that,
    /// never a fold made by hand.
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
    function foldByHand(collapse) {
        page.sidebarCollapsed = collapse
        page.foldedByDiff = false
        // Every way the list comes back arrives here — the band's block,
        // a rail cell, the rename box (SidebarPane.startEdit) — so
        // "unfolding closes the diff" has no exceptions elsewhere.
        if (!collapse && page.diffShown)
            page.closeDiff()
    }

    property bool wipShown: false
    // The ask bar goes off screen with the pane, and nothing off screen
    // may be answered.
    onWipShownChanged: if (!page.wipShown) page.stopRowAsk()
    // Selected stash row's reflog selector ("" = not a stash).
    property string selectedStashRef: ""
    function showWip() {
        page.wipShown = true
        page.pendingHeadSelect = false
        page.pendingHeadAsked = false
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
            // One shared answer slot, so only the reply to the range
            // this page asked about is read.
            if (repoTab.publishRange === page.headRange)
                page.headPublished = repoTab.publishPublished > 0
            if (commitMenuState.menuOid !== ""
                    && repoTab.publishRange === commitMenuState.menuOid + "^!")
                commitMenuState.menuPublished = repoTab.publishPublished > 0
            if (page.selectedOid !== ""
                    && repoTab.publishRange === page.selectedOid + "^!")
                page.selectedPublished = repoTab.publishPublished > 0
            if (refRowMenu.rebaseRange !== ""
                    && repoTab.publishRange === refRowMenu.rebaseRange)
                refRowMenu.rebasePublished = repoTab.publishPublished > 0
            page.absorbHeadMessage()
            page.absorbMoveAsk()
            page.absorbWriteResult()
            if (autoActLoader.item)
                autoActLoader.item.runFetchFailures()
        }
        // Only the first failure of a run: an offline machine would
        // otherwise re-raise the panel every interval (デザイン規約
        // §git が言ったことを読む場所).
        function onFetchFirstFailed() {
            page.commandsOpen = true
            commandsPane.showLatest()
        }
    }

    // A tab whose repository would not open. The screen sits where the
    // panes do, not over the whole page, so the log stays reachable under
    // it. The log is not raised on its own: the failed command is a
    // background read, so the panel would come up empty (実測).
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
    }

    // Not confirmed even when HEAD is already on a remote: amending
    // rewrites nothing that a switch or a reset cannot bring back, and
    // the push that would spread it is asked about on its own.
    function commitNow() {
        repoTab.commit(wipPane.subjectText, wipPane.bodyText,
                       page.amending, wipPane.resetAuthor)
    }

    // ---- moving between branches and commits ----------------------
    // Terminology is deliberate: git runs `switch` / `restore`, and
    // the UI says "Switch to" (デザイン規約 §用語).
    //
    // Uncommitted changes come along unasked; where git refuses, core
    // goes round through a stash (デザイン規約 §未コミット変更がある
    // 状態での移動).
    //
    // kind is "branch" / "remote" / "force" (a local branch moved to
    // `moveStart` before landing on it). Every one lands on a branch —
    // nothing here moves onto a bare commit (デザイン規約
    // §ブランチ・コミットへの移動).
    property string moveKind: ""
    property string moveTarget: ""
    property string moveLocal: ""
    property string moveStart: ""

    function switchTo(kind, target, local, start) {
        page.moveKind = kind
        page.moveTarget = target
        page.moveLocal = local
        page.moveStart = start === undefined ? "" : start
        page.runSwitch()
    }
    function runSwitch() {
        if (page.moveKind === "branch")
            repoTab.checkoutBranch(page.moveTarget)
        else if (page.moveKind === "remote")
            repoTab.checkoutRemote(page.moveTarget, page.moveLocal)
        else if (page.moveKind === "force")
            repoTab.checkoutForceCreate(page.moveTarget, page.moveStart)
    }

    // ---- what a chip leads to --------------------------------------
    // `record` is the chip as it is drawn — kind letter, four flags,
    // then the name (see encode.rs).
    function activateRecord(record) {
        if (record !== "")
            page.switchToRef(record[0], record.substring(5).split("\u001E")[0])
    }
    function switchToRef(kind, name) {
        if (repoTab.state !== "open" || repoTab.busyCount > 0)
            return
        if (kind === "L") {
            if (name !== workTree.branch)
                page.switchTo("branch", name, name, "")
            return
        }
        // The detached-HEAD marker names no branch, and moving onto a
        // tag could only detach HEAD — a tag's row offers a branch at
        // that commit instead (`startNaming`).
        if (kind !== "R")
            return
        const local = remotesModel.localNameFor(name)
        if (branchesModel.oidOfName(local) === "")
            page.switchTo("remote", name, local, "")
        else
            // Whether to ask is git's to answer — a branch that only
            // fell behind loses nothing by moving. The question comes
            // back as `moveAskSeq` when something would be lost.
            repoTab.checkoutMovingBranch(local, name)
    }

    // Landing on the remote branch moves the existing local one onto it
    // — the one move here that can leave commits unreachable.
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
    // One bar over the graph (デザイン規約 §可否・警告の出し場所);
    // Escape, another ask or a click anywhere else walks away from it.
    // Only questions about a ref reach it: everything that takes one
    // named thing away is held down on the row or button that names it
    // (デザイン規約 §長押し).
    /// Ctrl+F: the find bar belongs to the graph.
    function startFind() {
        graphPane.startFind()
    }

    property var rowAskRun: null
    function startRowAsk(oidHex, label, detail, danger, acceptText, run,
                         hold = false, tip = "", form = null, code = "") {
        page.rowAskRun = run
        graphPane.startAsking(oidHex, label, detail, acceptText, danger,
                              hold, tip, form, code)
    }
    function stopRowAsk() {
        page.rowAskRun = null
        publishFlow.publishAsking = false
        graphPane.stopAsking()
    }
    function answerRowAsk() {
        const run = page.rowAskRun
        page.stopRowAsk()
        if (run)
            run()
    }

    // On its own counter: the same move can be asked about twice in a
    // row, and only a fresh answer may raise the question.
    property int seenMoveAskSeq: 0
    function absorbMoveAsk() {
        if (repoTab.moveAskSeq === page.seenMoveAskSeq)
            return
        page.seenMoveAskSeq = repoTab.moveAskSeq
        page.askMoveBranchOnto(repoTab.moveAskLocal, repoTab.moveAskStart)
    }

    // ---- sending the branch to its remote ---------------------------
    PublishFlow {
        id: publishFlow
        repoTab: repoTab
        workTree: workTree
        remotesModel: remotesModel
        graphPane: graphPane
        // The first push asks where the branch goes, and it asks in the
        // one bar every other question stands in.
        onAskRequested: (label, run, form, code) =>
            page.startRowAsk("", label, "", false, "", run, false, "",
                             form, code)
    }
    /// What the window's toolbar reads off the page it is showing: the
    /// button lives up there, and the state machine behind it down here
    /// (`TopBar`).
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

    // ---- context menu on a sidebar row ------------------------------
    RefRowMenu {
        id: refRowMenu
        repoTab: repoTab
        workTree: workTree
        branchesModel: branchesModel
        onSwitchRequested: (kindLetter, name) => page.switchToRef(kindLetter, name)
        onDeleteRequested: (kind, id, name, oidHex) =>
            page.deleteRow(kind, id, name, oidHex)
        onDropStashRequested: selector => page.dropStashNow(selector)
        // The settle re-run is for a menu that stood on the stacked
        // list's row: the list stayed up under it, and whether it stays
        // now is the pointer's to answer again.
        onDismissed: rowHost.settleRefList()
    }
    /// The one door into that menu: the sidebar's rows, a chip, the
    /// stacked list and the automation all come through here. Says
    /// whether it opened.
    function openRefMenu(kind, name, full, oidHex) {
        return refRowMenu.offerOn(kind, name, full, oidHex)
    }

    /// A chip's right-click: the ref menu for the name the chip shows. A
    /// chip that names nothing to act on — the detached-HEAD marker, a
    /// stash, the current branch (whose ref menu has no rows) — falls
    /// back to the row's commit menu. The stacked list's rows pass no
    /// `oidHex` and have no row to fall back to (デザイン規約 §メニュー).
    function openRecordMenu(record, oidHex) {
        const kind = record === "" ? ""
                   : record[0] === "L" ? "branch"
                   : record[0] === "R" ? "remote"
                   : record[0] === "T" ? "tag" : ""
        if (kind === "") {
            if (oidHex !== "")
                page.openRowMenu(oidHex)
            return
        }
        const name = record.substring(5).split(String.fromCharCode(30))[0]
        const oid = kind === "branch" ? branchesModel.oidOfName(name)
                  : kind === "remote" ? remotesModel.oidOfName(name)
                  : tagsModel.oidOfName(name)
        if (!page.openRefMenu(kind, name, name, oid) && oidHex !== "")
            page.openRowMenu(oidHex)
    }

    // ---- what the sidebar's rows ask for ---------------------------
    /// Unconfirmed: a name is not history. A tag and a stash are re-made
    /// under the new name by core — the only rename git has for them.
    function renameRow(kind, id, name) {
        if (kind === "branch") {
            // Which remote this branch speaks for has to be read before
            // the rename: afterwards the row answers to the new name.
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

    /// The remote branch a just-renamed local one spoke for, and the name
    /// it took — the question about carrying the name over waits until
    /// git says the local rename landed.
    property string pendingRenameRemote: ""
    property string pendingRenameTo: ""

    /// Renaming a branch on a remote, which git has no command for: core
    /// pushes the new name and deletes the old, so the question is asked
    /// first and its answer is held down rather than clicked — this is
    /// the one write here that another machine keeps (デザイン規約 §長押し).
    function askRenameRemote(remoteRef, name) {
        const cut = remoteRef.indexOf("/")
        if (cut < 0)
            return
        const remote = remoteRef.substring(0, cut)
        const from = remoteRef.substring(cut + 1)
        // A name already over there is not offered: a plain push to one
        // that exists fast-forwards it and reports success, so somebody
        // else's branch would move instead of ours being renamed. The
        // box refuses it too; this catches the way in that has no box.
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

    /// A branch is deleted with `-d`, and git's refusal is the question —
    /// asked when it arrives rather than guessed at beforehand
    /// (デザイン規約 §左メニューの所作). A tag and a stash have no such
    /// refusal in git, so they are asked about up front. A branch on a
    /// remote never arrives here (`deleteRemoteNow`).
    property string pendingDeleteBranch: ""
    /// Refusals this page already has an answer for. The question bar
    /// explains them, so the command log stays where it was rather than
    /// raising itself over the same news (デザイン規約 §git が言ったこと
    /// を読む場所). Counted rather than flagged because the refusal and
    /// the write result arrive on separate paths, in no fixed order.
    property int expectedRefusals: 0
    function deleteRow(kind, id, name, oidHex) {
        if (kind !== "branch")
            return
        // The one row in any menu that outlives its own write (it stays
        // open for git's answer), so it is also the one that can be
        // clicked twice — the second click is the same request again.
        if (repoTab.busyCount > 0)
            return
        page.pendingDeleteBranch = id
        page.expectedRefusals++
        repoTab.deleteBranch(id, false)
    }
    /// Held, not asked (デザイン規約 §長押し).
    function dropStashNow(ref) {
        repoTab.dropStash(ref)
        if (page.selectedStashRef === ref)
            page.selectedStashRef = ""
    }
    /// git refused the plain delete: the answer lands on the menu row that
    /// asked, turning it into a held one (デザイン規約 §左メニューの所作).
    /// A refusal does not mean the commits stop being reachable: git
    /// measures the branch against its upstream when it has one, so a
    /// branch merged into HEAD but not yet pushed is refused while nothing
    /// at all would be lost (実測).
    function noteForceDelete(name) {
        refRowMenu.forceDeleteBranch = name
        if (AppBackend.autoAct !== "")
            AppBackend.report("force_delete_offered branch=" + name)
    }

    // ---- context menu on a working-tree file row --------------------
    function openFileMenu(bucket, path) {
        // A right-click is a click: it walks away from a question that
        // was standing, which may well be about another row.
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

    // ---- context menu on a graph row -------------------------------
    /// The one door into that menu: the graph's rows, a chip that names
    /// nothing to act on, and the automation all come through here.
    function openRowMenu(oidHex) {
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
        branch: workTree.branch
        oid: commitMenuState.menuOid
        stashRef: commitMenuState.menuStashRef
        published: commitMenuState.menuPublished
        canSequence: commitMenuState.menuCanSequence
        canIntegrate: commitMenuState.menuCanIntegrate
        canEditHistory: commitMenuState.menuCanEditHistory
        canMoveBranch: commitMenuState.menuCanMoveBranch
        stashCanWrite: commitMenuState.menuStashCanWrite
        onSquashRequested: oidHex => page.squashCommit(oidHex)
        onDropRequested: oidHex => page.dropCommit(oidHex)
        onResetRequested: mode => page.moveBranchHere(mode)
        onApplyStashRequested: selector => repoTab.applyStash(selector)
        onPopStashRequested: selector => {
            repoTab.popStash(selector)
            page.selectedStashRef = ""
        }
        onDropStashRequested: selector => page.dropStashNow(selector)
    }

    ClipboardHelper {
        id: clipboard
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

    // What a resting pointer opens on a graph row: the row's own card, and
    // the refs one chip had to stack. Owned here because rows are recycled
    // out from under both of them.
    RowHoverHost {
        id: rowHost
        graphPane: graphPane
        currentBranch: workTree.branch
        menuStanding: refRowMenu.opened
        onRecordActivated: record => page.activateRecord(record)
        onRecordMenuAsked: record => page.openRecordMenu(record, "")
    }

    // ---- rewriting one commit --------------------------------------
    // Not confirmed even for a commit a remote already has: nothing
    // here leaves the machine, and the push that would spread it is
    // asked about on its own.
    function squashCommit(oidHex) {
        repoTab.squashIntoParent(oidHex)
    }

    /// Leaves the commit out of the history. One place for both ways in:
    /// the row is a hold or a click depending on whether anything else
    /// still holds the branch tip, and what it runs must not depend on
    /// which of the two the reader got (デザイン規約 §履歴を合流させる).
    function dropCommit(oidHex) {
        repoTab.dropCommit(oidHex)
    }

    // ---- taking the branch back to an earlier commit ----------------
    function moveBranchHere(mode) {
        repoTab.resetTo(commitMenuState.menuOid, mode)
    }

    // ---- editing the selected commit's message ---------------------
    // No confirmation, even for a commit a remote already has: this
    // rewrites nothing that a switch or a reset cannot bring back, and
    // the push that would spread it is asked about on its own.
    function saveMessage(oidHex, subject, body) {
        // Where the row sits now. A reword leaves the shape of the
        // history alone, so the rewritten commit lands on the same row
        // and the selection can follow it there.
        page.rewordRow = graphModel.rowOf(oidHex)
        repoTab.rewordCommit(oidHex, subject, body)
    }
    // Row to re-select once the rewritten graph arrives (-1 = none).
    property int rewordRow: -1

    // Whether the selected commit is one HEAD was built on. Only those
    // can be amended or replayed from here, so the boxes stay read-only
    // until this comes back for the commit on screen.
    readonly property bool selectedInHistory:
        detailsModel.shaHex !== ""
        && repoTab.historyOid === detailsModel.shaHex && repoTab.historyIn
    function askInHistory(oidHex) {
        if (oidHex !== "" && repoTab.state === "open")
            repoTab.checkInHistory(oidHex)
    }

    // What git makes of the selected commit's signature. Asked on every
    // selection, like the history question beside it, and read only when
    // the answer names the commit now on screen — verifying runs gpg or
    // ssh-keygen, so the answer arrives well after the details do.
    // A signature only changes when the commit does, and a changed commit
    // is a different hash, so nothing has to ask twice.
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

    // Whether a remote already has the selected commit — what the save
    // row's warning rests on. Asked only once its message is touched:
    // that is the first moment the answer can matter, and it spares a
    // rev-list on every selection click.
    property bool selectedPublished: false
    function askSelectedPublished() {
        if (page.selectedOid !== "" && repoTab.state === "open")
            repoTab.checkPublish(page.selectedOid + "^!")
    }
    onSelectedOidChanged: page.selectedPublished = false

    // Moving off a half-written message would drop it. Hold the move
    // and let the editor ask — the question is about the text, so it is
    // asked where the text is rather than over the whole window. The
    // selection stays where it is while it stands: nothing has moved.
    property var pendingMove: null
    function guardEdits(targetOid, proceed) {
        // Landing where it already is takes nothing away.
        if (!detailsPane.messageDirty || targetOid === page.selectedOid) {
            proceed()
            return
        }
        const back = graphModel.rowOf(page.selectedOid)
        if (back >= 0)
            graphPane.setCurrentRow(back)
        page.pendingMove = proceed
    }
    function resolveLeave(discard) {
        const go = page.pendingMove
        page.pendingMove = null
        if (discard && go)
            go()
    }
    Connections {
        target: detailsPane
        // Reverted or saved by hand while the question stood: there is
        // nothing left to lose, so it answers itself.
        function onMessageDirtyChanged() {
            if (!detailsPane.messageDirty)
                page.pendingMove = null
            else
                page.askSelectedPublished()
        }
    }

    // An assignment is not something git knows about, so nothing here is
    // waiting for a refresh to bring it: the rows and the card re-read
    // the store themselves.
    Connections {
        target: AppBackend
        function onAvatarsChanged() {
            graphModel.refreshAvatars()
            detailsModel.refreshAvatar()
        }
    }

    // ---- smoke hook ------------------------------------------------
    // The whole of this page's PG_AUTO_ACT harness, built only when a
    // verb was given so an ordinary run carries none of it. A file of its
    // own cannot see this one's ids, so everything the verbs act on is
    // named here — an automation-only exposure, the same one
    // `GraphPane.view` is (app-ui.md).
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
            refDeleteItem: refRowMenu.deleteItem
            fileRowMenu: fileRowMenu
            fileMenu: fileRowMenu.menu
            fileDiscardItem: fileRowMenu.discardItem
            commitMenuState: commitMenuState
            commitMenu: commitRowMenu.menu
            dropCommitItem: commitRowMenu.dropItem
            stashDeleteItem: commitRowMenu.stashDropItem
            resetMenu: commitRowMenu.resetSubmenu
            hardResetItem: commitRowMenu.hardResetRow
            publishFlow: publishFlow
            remoteDialog: publishFlow.dialog
            refList: rowHost.listPopup
            rowCard: rowHost.hoverCard
        }
    }

    // A finished write the editor asked for: clear it only once git
    // says the commit landed, so a rejected one keeps its text.
    property int seenWriteSeq: 0
    function absorbWriteResult() {
        if (repoTab.writeSeq === page.seenWriteSeq)
            return
        page.seenWriteSeq = repoTab.writeSeq
        // A push this button sent has come back; what it means for the
        // toolbar's button is the flow's to work out.
        publishFlow.noteWriteAnswer(repoTab.lastWriteOp, repoTab.lastWriteError)
        if (repoTab.lastWriteError !== "") {
            // The one refusal this page has a second move for: a branch
            // delete git would not do on its own.
            if (repoTab.lastWriteOp === "branch" && page.pendingDeleteBranch !== "") {
                const refused = page.pendingDeleteBranch
                page.pendingDeleteBranch = ""
                page.noteForceDelete(refused)
                page.pendingRenameRemote = ""
                page.pendingRenameTo = ""
                return
            }
            // Nothing else on screen says what git said, so the log comes
            // up (デザイン規約 §git が言ったことを読む場所). Raised from
            // the answer rather than from the commands, because the ones
            // that answer by their exit code do not raise it themselves —
            // and whether an operation built out of several of them failed
            // is a question only its own answer can settle.
            page.commandsOpen = true
            commandsPane.showLatest()
            // A rename that did not happen has nothing to carry over.
            page.pendingRenameRemote = ""
            page.pendingRenameTo = ""
            return
        }
        // Landed: no refusal is coming for it after all, so the menu left
        // standing to catch one has nothing left to say.
        if (page.pendingDeleteBranch !== "") {
            page.pendingDeleteBranch = ""
            page.expectedRefusals = Math.max(0, page.expectedRefusals - 1)
            refRowMenu.close()
        }
        // The branch took its new name here; the remote it speaks for is
        // still under the old one. Asked only now, and only because there
        // is a remote to ask about (デザイン規約 §左メニューの所作).
        if (repoTab.lastWriteOp === "branch" && page.pendingRenameRemote !== "") {
            const spokenFor = page.pendingRenameRemote
            const took = page.pendingRenameTo
            page.pendingRenameRemote = ""
            page.pendingRenameTo = ""
            page.askRenameRemote(spokenFor, took)
        }
        if (repoTab.lastWriteOp === "commit") {
            page.clearCommitEditor()
            wipPane.setAmendChecked(false)
            page.amending = false
        }
        // These three answer with a commit at the tip — the undo, the
        // copy, the merge — and that commit is what was asked for here,
        // not the row or the ref that was clicked. The selection goes to
        // it and the viewport follows: what was clicked can be anywhere in
        // the history, while the answer is always at the top.
        //
        // A merge of something the branch already holds lands there too,
        // and rightly: git says "Already up to date", and the tip is
        // exactly where that merge would have put anyone.
        if (repoTab.lastWriteOp === "revert" || repoTab.lastWriteOp === "cherry-pick"
                || repoTab.lastWriteOp === "merge") {
            page.pendingHeadSelect = true
            page.pendingHeadAsked = true
        }
        // Everything that moves what the two sides hold. A commit empties
        // the index and a stash empties both, so a diff left open on
        // either is a picture of a file as it was — the same staleness the
        // file list's own `+` used to leave behind.
        //
        // **Marked, not read.** This lands before the status the write
        // moved (`session::write` finishes the write and then publishes
        // status), so reading the file here would read it against a tree
        // the app has not caught up with — and then read it a second time
        // when it does. Saying the tally is unknown makes the one read
        // happen where the tree has settled, which is also where a change
        // nobody in this window made arrives.
        if (repoTab.lastWriteOp === "stage" || repoTab.lastWriteOp === "unstage"
                || repoTab.lastWriteOp === "discard"
                || repoTab.lastWriteOp === "commit"
                || repoTab.lastWriteOp === "stash")
            page.seenTreeTally = ""
        // The message landed: the editor stops offering to save it, and
        // keeps what was written until the selection catches up with
        // the commit that now carries it.
        if (repoTab.lastWriteOp === "reword")
            detailsPane.noteMessageSaved()
        // Moving HEAD rewrites the working tree under the diff pane:
        // the file it holds may not even exist where the move landed,
        // so the center goes back to the graph that was moved through.
        // Taking the branch back does the same to the file, and to which
        // side of the index it sits on.
        if (repoTab.lastWriteOp === "checkout"
                || repoTab.lastWriteOp === "reset")
            page.closeDiff()
        page.refreshHeadPublished()
        // HEAD may have moved: what the selected commit is to it — and
        // so whether its message is ours to rewrite — is asked again.
        page.askInHistory(page.selectedOid)
    }

    // Center area switches between the graph and a file diff. The
    // pieces are kept apart rather than parsed back out of the key:
    // a path may contain anything, colons included.
    property bool diffShown: false
    onDiffShownChanged: page.foldForDiff(page.diffShown)
    property string diffKey: ""
    property string diffKind: ""
    property string diffPath: ""
    property string diffOrigPath: ""
    // Whether the shown diff is a working-tree file (stageable).
    property bool diffFromWt: false
    readonly property bool diffStaged: page.diffKind === "staged"
    /// The two stage letters git reports for the file being shown, read
    /// once when it is opened: the pane is handed a path, not the row the
    /// path came from, and on a conflict git prints no patch for those
    /// letters are all there is to say. A file stops being conflicted only
    /// by a write, and every write reads the diff again.
    property string diffChange: ""
    /// The colour each side of a conflict is drawn in: the lane colour the
    /// graph gives that branch where it has one, so the diff borrows an
    /// answer rather than keeping a second set of its own.
    ///
    /// A binding, not a one-off read: the graph arrives in two passes (the
    /// chips land after the rows), and it is rebuilt whenever refs move.
    /// `finishCount` is read only to depend on it — every graph property
    /// shares one notify signal, so touching any of them is what makes
    /// this re-run when the rows change.
    readonly property int sideColorOurs:
        graphModel.finishCount >= 0
        ? graphModel.conflictColorOurs(workTree.sideOurs, workTree.sideTheirs)
        : -1
    readonly property int sideColorTheirs:
        graphModel.finishCount >= 0
        ? graphModel.conflictColorTheirs(workTree.sideOurs, workTree.sideTheirs)
        : -1
    function toggleDiff(kind, path, origPath) {
        if (page.diffShown && page.diffKey === kind + ":" + path) {
            page.closeDiff()
            return
        }
        page.openDiff(kind, path, origPath)
    }
    /// Reads one file, whoever asked — a row that was clicked, or the pane
    /// moving itself off a side that ran out (`followEmptySide`).
    function openDiff(kind, path, origPath) {
        page.diffKey = kind + ":" + path
        page.diffKind = kind
        page.diffPath = path
        page.diffOrigPath = origPath
        page.diffFromWt = kind !== "commit"
        page.diffChange = kind === "conflicts" ? worktreeModel.changeOf(path) : ""
        if (kind === "commit")
            diffModel.requestCommitFile(detailsModel.shaHex, detailsModel.parentHex,
                                        path, origPath)
        else
            diffModel.requestWorkTree(kind, path, origPath)
        page.diffShown = true
        page.diffNeighbour = ""
        page.noteDiffNeighbour()
    }
    // Stages (or unstages) one hunk, or one line of it. The indices
    // address the diff currently on screen, so the pane is reloaded
    // afterwards: once the patch is applied the rows have moved.
    function stageSelection(hunk, line) {
        // The shown diff's fingerprint rides along: the write refuses to
        // apply the indices to bytes that drifted since this was read.
        repoTab.stageSelection(page.diffKind, page.diffPath, page.diffOrigPath,
                               hunk, line, diffModel.fingerprint)
    }
    /// Throwing one hunk of the shown diff away, with no question in front
    /// of it: the button in that hunk's own heading was held down, which is
    /// the whole of the asking (デザイン規約 §その他の操作). A line cannot
    /// be thrown away on its own — the hunk is the smallest piece — though
    /// it can still be staged on its own, which loses nothing.
    function discardHunkNow(hunk) {
        repoTab.discardSelection(page.diffKind, page.diffPath,
                                 page.diffOrigPath, hunk, -1,
                                 diffModel.fingerprint)
    }
    // ---- what the reader lands on when a side runs out ---------------
    /// The row beside the open diff's file under the same heading, as
    /// `<bucket>:<path>` — the file after it, or the one before it where
    /// it is the last (`NavSectionModel.besidePath`).
    ///
    /// Noted while the file is still there, because by the time the side
    /// has run out the file has already moved to the other one and the
    /// place it left is not in the list to be read.
    property string diffNeighbour: ""
    function noteDiffNeighbour() {
        if (!page.diffShown || page.diffKind === "commit"
                || !worktreeModel.holdsPath(page.diffKind, page.diffPath))
            return
        page.diffNeighbour = worktreeModel.besidePath(page.diffKind, page.diffPath)
    }
    /// Everything that was on this side has gone over to the other one.
    /// The reader is left standing on an empty frame, so the pane moves
    /// rather than closing (デザイン規約 §diff の中のステージ):
    ///
    ///  - the next file of the side that ran out, if it still has one;
    ///  - otherwise the same file, read from wherever it went — the whole
    ///    of it is on the other side now, which is the thing to look at;
    ///  - and only with nothing uncommitted left does the pane close.
    function followEmptySide() {
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
    /// A press addresses the rows it was made on, and carries the
    /// fingerprint of the bytes they were read from; git refuses it
    /// against anything else. So from the moment a write goes out until
    /// the rows it changed are back, the pane must not take another one —
    /// pressed twice in a row, the second landed on the diff the first had
    /// already replaced and came back with a refusal in the log
    /// (2026-08-17 ユーザー報告).
    ///
    /// Three parts, in the order they happen: the write is running, the
    /// tree it moved has not been read yet (`seenTreeTally`), and the file
    /// is being read again.
    readonly property bool diffSettling:
        repoTab.busyCount > 0 || page.seenTreeTally === "" || diffModel.loading
    /// A write on the working tree has landed, so the open diff is a
    /// picture of what the file used to be.
    ///
    /// **Whoever wrote it.** This was once asked for by the writes made
    /// inside the diff itself, and the file list's own `+` and `−` moved
    /// the same file out from under the pane without a word: a line staged
    /// here and then unstaged there left the line missing from both sides
    /// on screen (2026-08-17 ユーザー報告). The caller already knows the
    /// write was one that moves the tree (`absorbWriteResult`), so being
    /// open is the whole of the condition.
    function reloadDiff() {
        if (!page.diffShown || page.diffKind === "commit")
            return
        // Held here rather than beside each write: the rows on screen do
        // not move until the answer lands, so this is still the place the
        // reader was at (`DiffScrollPlace`).
        diffPane.holdScroll()
        // If this write took the last of what was on this side, the pane
        // has nothing left to stand on and closes (デザイン規約 §diff の
        // 中のステージ).
        diffPane.closeWhenEmpty = true
        diffModel.requestWorkTree(page.diffKind, page.diffPath, page.diffOrigPath)
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
    readonly property var pageCommands: commandsModel
    /// For the settings card's avatar entry, which offers the authors of
    /// the repository being looked at.
    readonly property var pageGraph: graphModel

    /// Whether the command log is up. Closed is the resting state: the
    /// toolbar's `>_` opens it, and a failed command raises it.
    property bool commandsOpen: false
    function toggleCommands() {
        page.commandsOpen = !page.commandsOpen
        if (page.commandsOpen)
            commandsPane.showLatest()
    }
    /// What the panel is doing, rather than what was asked of it — the
    /// automation reads this one, so a cut binding cannot pass.
    readonly property bool commandsShown: commandsPane.visible
    /// Automation reads the laid-out width, not the preferred width it
    /// requested, before persisting a state round trip.
    readonly property real stateDetailsWidth: rightPane.width
    /// Automation only: the header's `Clear`, pressed from outside the
    /// pane. The answer to what it clears is on the band, which cannot
    /// reach in here (`PG_AUTO_ACT=commands-clear`).
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
    // One letter apart, two different things: `worktreeModel` feeds the
    // WIP pane (the "worktree" nav section = uncommitted files), while
    // `worktreesModel` lists git worktrees (the sidebar's WORKTREES).
    NavSectionModel { id: worktreeModel }
    NavSectionModel { id: worktreesModel }
    NavSectionModel { id: stashesModel }
    NavSectionModel { id: tagsModel }

    /// True on the one page the window is showing. A tab restored from the
    /// last session has no repository behind it until this turns true —
    /// the session is what costs, and it is not spent on pages nobody has
    /// looked at.
    property bool pageCurrent: false
    onPageCurrentChanged: {
        if (page.pageCurrent && !page.blank)
            repoTab.activate()
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

    /// What the window reads off the page it is showing: the floor it may
    /// not be laid out under (`Main.floorWidth` / `floorHeight`) and the
    /// two panes' own overflow, which the floor's own verb asks about.
    readonly property real floorWidth: pageLayout.floorWidth
    readonly property real floorHeight: pageLayout.floorHeight
    readonly property bool wipBlockScrolls: pageLayout.wipBlockScrolls
    readonly property real detailsOverHeight: pageLayout.detailsOverHeight

    /// …and what it calls: the layout is pulled on the window's timer, and
    /// the two setters are the headless state check's (a person drags).
    function reportLayout() {
        pageLayout.reportLayout()
    }
    function setDetailsWidth(w) {
        pageLayout.setDetailsWidth(w)
    }
    function setGraphColumns(labels, lanes) {
        pageLayout.setGraphColumns(labels, lanes)
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
        worktreeModel.attachSection(page.tab_id, "worktree")
        worktreesModel.attachSection(page.tab_id, "worktrees")
        stashesModel.attachSection(page.tab_id, "stashes")
        tagsModel.attachSection(page.tab_id, "tags")
        if (autoActLoader.item)
            autoActLoader.item.begin()
    }

    /// The window's focus epoch (bumped when the window regains focus)
    /// triggers a quick refresh of the visible page.
    property int focusEpoch: 0
    onFocusEpochChanged: {
        if (page.visible && repoTab.state === "open")
            repoTab.refreshQuick()
    }

    /// True while the window is on screen (see Main.qml): the page shown
    /// there re-reads its repository on a tick, so a commit made in a
    /// terminal or by an agent turns up on its own.
    property bool onScreen: false
    Timer {
        interval: Metrics.pollIntervalMs
        repeat: true
        // Only the tab in front — the others catch up when switched to,
        // and reading every open repository on every tick is what makes
        // polling expensive elsewhere.
        running: page.onScreen && page.visible && repoTab.state === "open"
        onTriggered: repoTab.refreshPoll()
    }

    // Where the selection stands, kept so a commit that disappears from
    // under it can be followed to whatever took its place.
    property int selectedRow: -1

    // The commit the viewport is measured against between passes, so the
    // rows a reader is on can be put back under them when new ones arrive
    // above. The WIP row is no use for that — it comes and goes with the
    // working tree — so the newest *real* commit carries the measurement.
    property string anchorOid: ""
    property int anchorRow: -1
    function rememberAnchor() {
        let row = 0
        let oidHex = graphModel.oidAt(0)
        if (oidHex !== "" && !/[^0]/.test(oidHex)) {
            row = 1
            oidHex = graphModel.oidAt(1)
        }
        page.anchorOid = oidHex
        page.anchorRow = oidHex === "" ? -1 : row
    }
    // How far the graph slid under the viewport. Zero when the anchor is
    // gone: a rewrite deep in the history moves rows by different amounts
    // and there is no single answer, so the view is left alone.
    function anchorShift() {
        if (page.anchorRow < 0 || page.anchorOid === "")
            return 0
        const now = graphModel.rowOf(page.anchorOid)
        return now < 0 ? 0 : now - page.anchorRow
    }

    // The row the selection stood on is gone and this page owes it a
    // landing. Deliberately not resolved on the spot: the status of the
    // working tree, the refs and the walk arrive as three separate
    // messages, and the two that come first still describe the repository
    // as it was — reading the branch out of them lands on the commit that
    // was just replaced. Resolved once the graph holds where the branch
    // points, which is only true of the refreshed pair.
    property bool pendingHeadSelect: false
    // Whether the landing is one the person here asked for, in which case
    // the viewport goes to it as well: a commit they meant to make is not
    // an answer if it lands off screen. The other two ways this is set
    // happen *to* the window — a commit made in a terminal, a rewrite that
    // swept the selected commit away while the poll was watching — and a
    // background pass that moves rows under a reader may not also move
    // their view (§ListView.highlightFollowsCurrentItem).
    property bool pendingHeadAsked: false
    function tryPendingHeadSelect() {
        if (!page.pendingHeadSelect || !branchesModel.refsLoaded)
            return
        const row = branchesModel.headOid !== ""
                    ? graphModel.rowOf(branchesModel.headOid) : -1
        if (row < 0)
            return
        page.pendingHeadSelect = false
        const asked = page.pendingHeadAsked
        page.pendingHeadAsked = false
        graphPane.setCurrentRow(row)
        page.activateRow(graphModel.oidAt(row))
        // Held back by an unsaved message: the question put the highlight
        // back where it was, so there is nowhere for the view to go yet.
        if (asked && page.pendingMove === null)
            graphPane.showRowSoon(row)
    }

    // The selected commit is gone from the graph and this page did not
    // rewrite it: an amend or a rebase run in a terminal replaced it while
    // the poll was watching. Whatever now stands where it stood is the
    // closest thing to what was being read; failing that, fall back to the
    // branch's own commit, which is never nothing.
    function followVanishedCommit() {
        const oidHex = graphModel.oidAt(page.selectedRow)
        if (oidHex !== "" && /[^0]/.test(oidHex)) {
            graphPane.setCurrentRow(page.selectedRow)
            page.activateRow(oidHex)
            return
        }
        page.selectedOid = ""
        page.pendingHeadSelect = true
    }

    // A reworded commit came back under a different hash: the one now
    // standing where it stood is it, since only the message changed.
    function followRewrittenCommit() {
        const row = page.rewordRow
        const oidHex = graphModel.oidAt(row)
        // Nothing there, or the working-tree row moved under it.
        if (oidHex === "" || !/[^0]/.test(oidHex)) {
            page.rewordRow = -1
            return
        }
        graphPane.setCurrentRow(row)
        page.activateRow(oidHex)
    }

    // What a row click means: the synthetic WIP row (all-zero id)
    // opens the working-tree view, anything else selects the commit.
    function activateRow(oidHex) {
        page.guardEdits(oidHex, function () { page.selectRow(oidHex) })
    }
    function selectRow(oidHex) {
        // Any new selection settles where the last rewrite left off, and
        // answers any landing this page still owed.
        page.rewordRow = -1
        page.pendingHeadSelect = false
        page.pendingHeadAsked = false
        // Clicking anywhere is the way out of the name box and of a
        // standing row question: both are offers, not work in progress.
        graphPane.stopNaming()
        page.stopRowAsk()
        // A click moves the highlight itself, but one held back by the
        // unsaved-message question does not: the question put it back
        // where it was, so the answer has to move it again.
        const row = graphModel.rowOf(oidHex)
        if (row >= 0) {
            graphPane.setCurrentRow(row)
            page.selectedRow = row
        }
        if (oidHex !== "" && !/[^0]/.test(oidHex)) {
            page.showWip()
            return
        }
        page.wipShown = false
        page.selectedOid = oidHex
        page.selectedStashRef = graphModel.stashRefOf(oidHex)
        detailsModel.request(oidHex)
        page.askInHistory(oidHex)
        page.askSignature(oidHex)
        page.closeDiff()
    }

    // Selection policy: restore across the tag-swap reset, and default
    // to the current branch's newest commit on first load so the
    // details pane always shows something.
    function trySelectDefault() {
        if (page.selectedOid !== "" || page.wipShown || page.pendingHeadSelect
                || AppBackend.autoSelect || AppBackend.autoWip
                || graphModel.rowTotal === 0)
            return
        // Refs decide which commit is "current" — wait for them
        // instead of guessing the newest row too early.
        if (!branchesModel.refsLoaded)
            return
        let row = branchesModel.headOid !== ""
                  ? graphModel.rowOf(branchesModel.headOid) : -1
        if (row < 0) {
            if (graphModel.loading)
                return // the head row may still be streaming in
            row = 0 // detached / head outside the window: newest commit
        }
        graphPane.setCurrentRow(row)
        graphPane.anchorSoon()
        page.activateRow(graphModel.oidAt(row))
    }
    // Each finished pass bumps finishCount: re-resolve the selection by
    // oid, since row numbers may have shifted. Only a streaming restart
    // bumps resetCount — that is the only case where the viewport lost
    // its scroll position and needs re-anchoring. In-place replacements
    // keep the position, and re-centering would yank the view around.
    property int seenFinishCount: 0
    property int seenResetCount: 0
    Connections {
        target: graphModel
        function onStatsChanged() {
            if (graphModel.finishCount !== page.seenFinishCount) {
                page.seenFinishCount = graphModel.finishCount
                const resetHappened = graphModel.resetCount !== page.seenResetCount
                page.seenResetCount = graphModel.resetCount
                // A reset starts the viewport over anyway; only in-place
                // replacements leave it pointing at rows that moved.
                if (!resetHappened)
                    graphPane.shiftRows(page.anchorShift())
                page.rememberAnchor()
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
                    } else {
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
            // Refs can be the half that was missing, when the walk had
            // already delivered the commit they now point at.
            page.tryPendingHeadSelect()
            page.trySelectDefault()
        }
        // Refs that settled without moving say nothing through `changed`
        // (identical rows are deliberately quiet — see the model), and a
        // write that recorded nothing leaves them exactly so: a
        // cherry-pick of a commit this branch already has owes the same
        // landing as one that wrote a commit, and this is the only word
        // that it can be paid.
        function onRefsSettled() {
            page.tryPendingHeadSelect()
        }
    }
    /// What the working tree looked like the last time the open diff was
    /// read against it. Not a diff of the file — the counts of the four
    /// buckets, which is what a stage or an unstage moves whoever made it
    /// (another window, a terminal, this pane's own `+`).
    property string seenTreeTally: ""
    function treeTally() {
        return workTree.stagedCount + "/" + workTree.unstagedCount + "/"
             + workTree.untrackedCount + "/" + workTree.conflictCount + "/"
             + worktreeModel.total
    }
    Connections {
        target: worktreeModel
        function onChanged() {
            // The file the diff is on is still where it was, so this is
            // the last moment its neighbour can be read (see
            // `noteDiffNeighbour`).
            page.noteDiffNeighbour()
            // The one place the open diff is read again, whatever moved
            // the tree — this pane's own `+`, the file list's, another
            // window, a terminal. Left to the writes it could see, the
            // rows on screen and the fingerprint the next `+` is written
            // against stayed a picture of the file as it was, and pressing
            // one came back with git's refusal (2026-08-17 ユーザー報告).
            //
            // Read against the tally rather than every tick: a refresh
            // that found the same tree has nothing to catch up with, and
            // a write says so by putting the tally beyond reach.
            const tally = page.treeTally()
            if (tally !== page.seenTreeTally) {
                page.seenTreeTally = tally
                page.reloadDiff()
            }
            // The working tree emptied. After a commit of our own that is
            // the end of the editor's job; when someone else committed
            // these changes it happens with no warning, so a message being
            // written stays on screen with its text — it is the one thing
            // here that cannot be read back off disk. Otherwise land on the
            // commit that now holds the changes rather than on nothing.
            if (worktreeModel.total === 0 && page.wipShown
                    && wipPane.subjectText === "" && wipPane.bodyText === "") {
                page.wipShown = false
                page.pendingHeadSelect = true
            }
            // Smoke hook (PG_AUTO_WIP=1): open the WIP view once
            // uncommitted changes are known.
            if (AppBackend.autoWip && worktreeModel.total > 0 && !page.wipShown) {
                graphPane.setCurrentRow(0)
                page.showWip()
            }
        }
    }

    // Smoke hook (PG_SCROLL_TO=top|bottom|nav-bottom): jump the graph —
    // or the sidebar's branch list — after the final pass settles, using
    // the same clamped math as the wheel.
    // Parks the view once and then stays out of the way: re-running on
    // every pass would drag a background refresh back to the edge, which
    // is the one thing a scrolled view must not do on its own.
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
            graphPane.view.contentY = graphPane.view.clampY(
                AppBackend.scrollTo === "bottom" ? 1e12 : -1e12)
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

    // Automation (PG_AUTO_SELECT=1): select the newest commit, then open
    // the first changed file's diff — exercises the full pipeline for
    // screenshot-based smoke tests.
    property bool autoSelected: false
    Connections {
        target: graphModel
        enabled: AppBackend.autoSelect
        function onStatsChanged() {
            if (page.autoSelected || graphModel.rowTotal === 0)
                return
            // **The newest commit, not the newest row.** A dirty working
            // tree puts the WIP row on top, and selecting that one shows
            // the pending changes instead of a commit — no details are
            // asked for, so a measurement that reads the interaction
            // budget off this hook measures nothing and says so
            // (`xtask perf`'s `missing`). Every demo repository is dirty.
            for (let row = 0; row < graphModel.rowTotal; row++) {
                const oid = graphModel.oidAt(row)
                if (oid !== "" && /[^0]/.test(oid)) {
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
            if (detailsModel.shaHex !== "" && detailsModel.fileTotal > 0
                    && diffModel.title === "")
                page.toggleDiff("commit", detailsModel.filePathAt(0),
                                detailsModel.fileOrigPathAt(0))
        }
    }

    /// The failed-open screen's "Close tab" button.
    signal closeTabRequested()

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // The panes sit above the command log, which is closed until it
        // is asked for. Splitting them vertically keeps the log's height
        // in the reader's hands and out of the panes' business.
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
                onCloseRequested: page.closeTabRequested()
            }

            // ---- three-pane layout --------------------------------------
            // (repository state / search / fetch / push live in the window
            // toolbar, next to the tabs; Reload is the app menu and F5)
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
                    repoTab: repoTab
                    workTree: workTree
                    branchesModel: branchesModel
                    remotesModel: remotesModel
                    worktreesModel: worktreesModel
                    stashesModel: stashesModel
                    tagsModel: tagsModel
                    collapsed: page.sidebarCollapsed
                    // The menus the rows raise are the page's, so only the
                    // page can say one is standing over the folded list.
                    menuOpen: refRowMenu.showing
                    onFoldRequested: collapse => page.foldByHand(collapse)
                    onRefActivated: oidHex => page.jumpToRef(oidHex)
                    onRefMenuRequested: (kind, name, full, oidHex) =>
                        page.openRefMenu(kind, name, full, oidHex)
                    onWorktreeActivated: path => page.openRepositoryPathRequested(path)
                    onRefSwitchRequested: (kind, name) => page.switchToRef(kind, name)
                    onBranchAtRequested: (oidHex, name) => {
                        if (name !== "")
                            repoTab.createBranch(name, oidHex, true)
                    }
                    onRenameSubmitted: (kind, id, name) => page.renameRow(kind, id, name)
                }

                // Center: commit graph ⇄ file diff
                StackLayout {
                    SplitView.fillWidth: true
                    // Not a number of its own: what the graph's own columns
                    // come to once they have both given everything they can,
                    // held up to a side pane's width so the middle never
                    // reads as the thinnest of the three
                    // (`PageLayout.centreMinWidth`).
                    SplitView.minimumWidth: pageLayout.centreMinWidth
                    currentIndex: page.diffShown ? 1 : 0

                    GraphPane {
                        id: graphPane
                        graphModel: graphModel
                        workTree: workTree
                        blank: page.blank
                        chipListAnchor: rowHost.refListAnchor
                        // A half-written message has put the move back
                        // and a question up: until it is answered the
                        // arrows stay still, or every press would be
                        // pushed back by `guardEdits` and the two would
                        // fight (規約 §矢印で履歴を辿る).
                        selectionHeld: page.pendingMove !== null
                        onRowActivated: oidHex => page.activateRow(oidHex)
                        // The bar moves between matches, not between
                        // commits — landing on the same row twice changes
                        // nothing and costs no git.
                        onFindLanded: oidHex => {
                            if (oidHex !== "" && oidHex !== page.selectedOid)
                                page.activateRow(oidHex)
                        }
                        onRowMenuOpenRequested: oidHex => page.openRowMenu(oidHex)
                        onChipMenuOpenRequested: (oidHex, record) =>
                            page.openRecordMenu(record, oidHex)
                        onRowSwitchRequested: (oidHex, record) =>
                            page.rowDoubleClicked(oidHex, record)
                        onChipExpandRequested: (records, anchor) =>
                            rowHost.openRefList(records, anchor)
                        onChipCollapseRequested: rowHost.closeRefListUnlessEntered()
                        onRowHoverRequested: (row, inside) => {
                            rowHost.rowCardWanted = inside
                            if (inside)
                                rowHost.openRowCard(row)
                            else
                                rowHost.settleRowCard()
                        }
                        onCreateBranchRequested: (oidHex, name) =>
                            repoTab.createBranch(name, oidHex, true)
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
                        // The two swap over during a rebase; the model is
                        // where that is already answered.
                        sideOurs: workTree.sideOurs
                        sideTheirs: workTree.sideTheirs
                        sideColorOurs: page.sideColorOurs
                        sideColorTheirs: page.sideColorTheirs
                        busy: page.diffSettling
                        onCloseRequested: page.closeDiff()
                        onNothingLeft: page.followEmptySide()
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
                    // Where it starts; `applySavedLayout` assigns over this
                    // with the width the window is set to.
                    SplitView.preferredWidth: 400
                    SplitView.minimumWidth: pageLayout.rightMinWidth
                    color: Theme.bgSurface

                    GitVersionCorner {
                        id: gitCorner
                        // Only one of the two panes is on screen at a
                        // time, and each measures its own file list.
                        roomLeft: page.wipShown ? wipPane.bottomRoom
                                                : detailsPane.bottomRoom
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.rightMargin: Theme.spaceSm
                        anchors.bottomMargin: Theme.spaceXs
                        // Declared before the panes it hangs over, so z
                        // holds it in front of whatever they draw here.
                        z: 1
                    }

                    WipPane {
                        id: wipPane
                        anchors.fill: parent
                        visible: page.wipShown
                        repoTab: repoTab
                        workTree: workTree
                        worktreeModel: worktreeModel
                        amending: page.amending
                        headPublished: page.headPublished
                        onAmendToggled: on => page.amendToggled(on)
                        onCommitClicked: page.commitNow()
                        onFileActivated: (bucket, path, origPath) =>
                            page.toggleDiff(bucket, path, origPath)
                        onStashSubmitted: (message, untracked, keepIndex, stagedOnly) =>
                            repoTab.pushStash(message, untracked, keepIndex, stagedOnly)
                        onFileMenuRequested: (bucket, path) =>
                            page.openFileMenu(bucket, path)
                    }

                    DetailsPane {
                        id: detailsPane
                        anchors.fill: parent
                        visible: !page.wipShown
                        details: detailsModel
                        stashRef: page.selectedStashRef
                        // Only what the working tree stands on: a commit off
                        // this line cannot be amended or replayed from here,
                        // and a stash is a commit but never one of them.
                        editable: !page.blank && repoTab.state === "open"
                                  && page.selectedStashRef === ""
                                  && page.selectedInHistory
                        editBlocked: page.selectedStashRef !== ""
                            ? qsTr("Rename it in the list on the left")
                            : (detailsModel.shaHex !== "" && !page.selectedInHistory
                               ? qsTr("Not in the current history — switch to a branch that has it")
                               : "")
                        busy: repoTab.busyCount > 0
                        asking: page.pendingMove !== null
                        published: page.selectedPublished
                        signatureKind: page.selectedSignatureKind
                        signatureCode: page.selectedSignatureCode
                        signatureSigner: page.selectedSignatureSigner
                        // HEAD's own commit, not the current branch's tip:
                        // detached, there is no branch to ask.
                        headOid: workTree.headOid
                        onLeaveResolved: discard => page.resolveLeave(discard)
                        onMessageSubmitted: (oidHex, subject, body) =>
                            page.saveMessage(oidHex, subject, body)
                        onFileActivated: (path, origPath) =>
                            page.toggleDiff("commit", path, origPath)
                        onParentClicked: oidHex => page.jumpToRef(oidHex)
                        // The badge's press goes to the window, which owns
                        // the settings card.
                        onAvatarEditRequested: (name, email) =>
                            page.avatarSettingsRequested(name, email)
                        onCopyRequested: text => clipboard.copy(text)
                        onApplyStashRequested: selector => repoTab.applyStash(selector)
                        onPopStashRequested: selector => {
                            repoTab.popStash(selector)
                            page.selectedStashRef = ""
                        }
                    }
                }
            }

            // ---- command log ------------------------------------------
            // Hidden until asked for, and raised by a failure.
            CommandsPane {
                id: commandsPane
                visible: page.commandsOpen
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

    // A command the user asked for failed. Nothing else on screen says
    // what git said, so the log comes up by itself and stays up — closing
    // it is the reader's call, not the next success's. Unless this page
    // asked for the refusal and turned it into a question: then the bar
    // is already saying it, and the log would say it twice while pushing
    // the graph out of the way.
    Connections {
        target: commandsModel
        function onFailure() {
            if (page.expectedRefusals > 0) {
                page.expectedRefusals--
                return
            }
            page.commandsOpen = true
            commandsPane.showLatest()
        }
    }

    // The split bars' refusal — collection, settling, overlay and the
    // divider-refuse report all live in the watcher (SplitBarWatch); the
    // page keeps the source chain, since only it sees all four sources.
    SplitBarWatch {
        id: splitWatch
        refusalSource: page.refusalSource
        graphPane: graphPane
    }

    /// Bars announce themselves once, at their own creation
    /// (`SplitHandleBar.Component.onCompleted`) — which is before the
    /// watcher above exists, so the page relays and holds the early
    /// ones; the page's `Component.onCompleted` hands them over.
    property var earlyBars: []
    function holdSplitBar(bar, held) {
        if (!splitWatch) {
            page.earlyBars.push(bar)
            return
        }
        splitWatch.holdSplitBar(bar, held)
    }

    /// Automation: the drag and its answer for every boundary in the
    /// window (`PG_AUTO_ACT=divider-refuse`; AutoActDriver calls through
    /// the page).
    function reportDividerRefusal(which) {
        splitWatch.reportDividerRefusal(which)
    }

    /// Whichever boundary is refusing, or null. One pointer, so the order
    /// only decides which answers in the frame where two could — and two
    /// cannot, since a hand is on one boundary at a time.
    readonly property var refusalSource:
        graphPane.refused ? graphPane.refusedAt
        : detailsPane.descRefuses ? detailsPane.descPoint
        : wipPane.descRefuses ? wipPane.descPoint
        : splitWatch.refuses ? splitWatch.at
        : null

    /// What is drawn, not what was asked for — the one badge's own
    /// `shown` (`RefusalBadge`, on why not its `visible`).
    readonly property bool refusalShown: splitWatch.shown

    function jumpToRef(oidHex) {
        page.guardEdits(oidHex, function () {
            const row = graphModel.rowOf(oidHex)
            if (row >= 0)
                graphPane.jumpToRow(row)
            // Details resolve even outside the window.
            page.rewordRow = -1
            page.wipShown = false
            page.selectedOid = oidHex
            page.selectedStashRef = graphModel.stashRefOf(oidHex)
            detailsModel.request(oidHex)
            page.askInHistory(oidHex)
            page.askSignature(oidHex)
            page.closeDiff()
        })
    }
}
