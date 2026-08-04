pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// One repository page: owns the tab's models and every piece of page
// state (selection, WIP mode, pending diff, context menus), and
// composes the three panes. Panes report intents through signals; this
// page decides what they mean. Anything that crosses the page (the
// irreversible-action confirmation, opening another repository as a
// tab) goes up to the window as a signal.
Item {
    id: page
    required property int index
    required property int tab_id
    // No repository behind this page (tab_id -1): the chrome renders
    // with empty models and the graph column offers the way in.
    readonly property bool blank: tab_id < 0

    /// Ask the window's confirmation dialog (heading, detail, accept
    /// wording, what to run on yes).
    signal confirmRequested(string heading, string detail, string acceptText, var action)
    /// The blank page's "Open repository…" button (folder picker).
    signal openRepositoryPicker()
    /// A worktree row was clicked: open that path as a new tab.
    signal openRepositoryPathRequested(string path)
    /// PG_AUTO_ACT=settings wants the window's settings dialog open
    /// for the screenshot.
    signal settingsDialogRequested()

    property string selectedOid: ""

    // Right pane switches to the working-tree (WIP) view.
    property bool wipShown: false
    // Selected stash row's reflog selector ("" = not a stash).
    property string selectedStashRef: ""
    function showWip() {
        page.wipShown = true
        page.selectedOid = ""
        page.selectedStashRef = ""
        page.closeDiff()
        page.refreshHeadPublished()
    }

    // ---- commit editor -------------------------------------------
    // `amending` mirrors the checkbox so the page can act on it
    // without reaching into the pane.
    property bool amending: false
    // Whether HEAD is already on a remote. Amending it rewrites
    // something other people may have, so the editor says so.
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
            page.absorbHeadMessage()
            page.absorbMoveBlock()
            page.absorbWriteResult()
        }
    }

    // Turning amend on starts the editor from HEAD's message; turning
    // it off empties it again, since the text belonged to that commit.
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
    // Uncommitted changes are asked about only when they get in the way:
    // the move goes ahead carrying them, and git refuses — changing
    // nothing — when it cannot. That refusal comes back as `moveBlock`
    // and raises the dialog, which re-runs the same move a different way.
    // What the pending move is: kind is "branch" / "remote" / "commit".
    property string moveKind: ""
    property string moveTarget: ""
    property string moveLabel: ""
    // Set while the last move sits refused, so the panes are left as they
    // were: nothing moved, so nothing about them is stale.
    property bool moveRefused: false

    function switchTo(kind, target, label) {
        page.moveKind = kind
        page.moveTarget = target
        page.moveLabel = label
        page.runSwitch("carry")
    }
    function runSwitch(carry) {
        page.moveRefused = false
        if (page.moveKind === "branch")
            repoTab.checkoutBranch(page.moveTarget, carry)
        else if (page.moveKind === "remote")
            repoTab.checkoutRemote(page.moveTarget,
                                   repoTab.localNameFor(page.moveTarget), carry)
        else if (page.moveKind === "commit")
            repoTab.checkoutDetached(page.moveTarget, carry)
    }

    // A refusal arrives on its own counter: the same answer can be needed
    // twice in a row, and only a fresh one may raise the dialog.
    property int seenMoveBlockSeq: 0
    function absorbMoveBlock() {
        if (repoTab.moveBlockSeq === page.seenMoveBlockSeq)
            return
        page.seenMoveBlockSeq = repoTab.moveBlockSeq
        page.moveRefused = true
        dirtySwitchDialog.blockKind = repoTab.moveBlock
        dirtySwitchDialog.open()
        // A dialog is invisible to a screenshot (popups draw in the
        // window overlay), so say so where the smoke run can read it.
        if (AppBackend.autoAct !== "")
            AppBackend.report("move_blocked kind=" + repoTab.moveBlock)
    }

    DirtySwitchDialog {
        id: dirtySwitchDialog
        moveLabel: page.moveLabel
        stayLabel: workTree.detached ? qsTr("this commit") : workTree.branch
        onResolved: carry => page.runSwitch(carry)
    }

    // ---- push ------------------------------------------------------
    readonly property string pushTargetLabel:
        workTree.upstream !== "" ? workTree.upstream
                                 : repoTab.defaultRemote + "/" + workTree.branch
    readonly property bool canPush: repoTab.state === "open"
                                    && !workTree.detached
                                    && workTree.branch !== ""
                                    && repoTab.remoteCount > 0
                                    && repoTab.busyCount === 0
    function pushNow() {
        repoTab.pushCurrent("", "")
    }
    function forcePushNow() {
        page.confirmRequested(
            qsTr("Overwrite %1 with this branch?").arg(page.pushTargetLabel),
            qsTr("A force push replaces the remote branch's history with "
                 + "yours. Commits only the remote has are lost, and anyone "
                 + "who already pulled them keeps a history that no longer "
                 + "matches.\n\nThe push is refused if the remote moved since "
                 + "this window last saw it."),
            qsTr("Force push"),
            // A lease pinned to the commit actually on screen: a
            // background fetch must not turn this into a plain force.
            function () { repoTab.pushCurrent("lease", page.upstreamOid()) })
    }
    /// Commit the remote-tracking branch points at, as shown here.
    function upstreamOid() {
        return workTree.upstream !== ""
               ? remotesModel.oidOfName(workTree.upstream) : ""
    }

    // ---- context menu on a branch row ------------------------------
    property string menuRefName: ""
    property string menuRefOid: ""
    property bool menuRefRemote: false
    function openRefMenu(name, oidHex, isRemote) {
        page.menuRefName = name
        page.menuRefOid = oidHex
        page.menuRefRemote = isRemote
        refMenu.popup()
    }
    AppMenu {
        id: refMenu
        AppMenuItem {
            text: qsTr("Switch to %1").arg(page.menuRefName)
            enabled: page.menuRefName !== workTree.branch
            onTriggered: page.switchTo(page.menuRefRemote ? "remote" : "branch",
                                       page.menuRefName, page.menuRefName)
        }
        AppMenuSeparator {}
        AppMenuItem {
            text: qsTr("Copy commit hash")
            onTriggered: clipboard.copy(page.menuRefOid)
        }
    }

    // ---- putting changes away --------------------------------------
    // A stash takes nothing away — every change is in the entry, and the
    // sidebar shows it — so neither the whole tree nor one file is asked
    // about beyond the dialog that chooses what goes.
    StashDialog {
        id: stashDialog
        partiallyStaged: workTree.partiallyStagedCount
        busy: repoTab.busyCount > 0
        onSubmitted: (message, untracked, keepIndex, stagedOnly) =>
            repoTab.pushStash(message, untracked, keepIndex, stagedOnly)
    }

    // ---- context menu on a working-tree file row --------------------
    property string menuFilePath: ""
    property string menuFileBucket: ""
    function openFileMenu(bucket, path) {
        page.menuFileBucket = bucket
        page.menuFilePath = path
        fileMenu.popup()
    }
    AppMenu {
        id: fileMenu
        AppMenuItem {
            text: qsTr("Stash this file")
            // git will not stash a tree with unresolved conflicts in it.
            enabled: repoTab.busyCount === 0
                     && page.menuFileBucket !== "conflicts"
            onTriggered: repoTab.stashPath(page.menuFilePath, "")
        }
        AppMenuSeparator {}
        AppMenuItem {
            text: qsTr("Copy path")
            onTriggered: clipboard.copy(page.menuFilePath)
        }
    }

    // ---- context menu on a commit row ------------------------------
    property string menuOid: ""
    readonly property string menuShort: page.menuOid.substring(0, 8)
    function openCommitMenu(oidHex) {
        page.menuOid = oidHex
        commitMenu.popup()
    }

    AppMenu {
        id: commitMenu
        AppMenuItem {
            text: qsTr("Copy this commit onto the current branch")
            enabled: repoTab.busyCount === 0
            onTriggered: repoTab.cherryPick(page.menuOid)
        }
        AppMenuItem {
            text: qsTr("Switch to this commit")
            enabled: repoTab.busyCount === 0
            onTriggered: page.switchTo("commit", page.menuOid, page.menuShort)
        }
        AppMenuSeparator {}
        // The click that opened this menu selected the row too, so the
        // message is already in the details pane's boxes: this just
        // puts the caret there.
        AppMenuItem {
            text: qsTr("Edit message")
            enabled: repoTab.busyCount === 0 && page.selectedStashRef === ""
                     && page.menuOid === detailsModel.shaHex
                     && page.selectedInHistory
            onTriggered: detailsPane.focusMessage()
        }
        AppMenuItem {
            text: qsTr("Fold into the commit before it")
            enabled: repoTab.busyCount === 0
            onTriggered: page.squashCommit(page.menuOid)
        }
        // Three ways to take the branch back to this commit, told apart
        // by what becomes of the work they skip over rather than by
        // git's own words for the modes. A submenu keeps the choice out
        // of the way until it is asked for; disabling the submenu itself
        // greys the row that opens it (its items are never reachable
        // while it is off).
        AppMenu {
            id: resetMenu
            // "here" rather than "to this commit": a Fusion menu is 200px
            // wide whatever is in it, and the arrow of a submenu row eats
            // another 26 — the longer wording elides, and what elides
            // first should be the branch name, not the operation.
            title: workTree.branch !== ""
                   ? qsTr("Move %1 here").arg(workTree.branch)
                   : qsTr("Move the branch here")
            enabled: page.canMoveBranchHere
            AppMenuItem {
                text: qsTr("Keep everything, staged")
                onTriggered: page.moveBranchHere("soft")
            }
            AppMenuItem {
                text: qsTr("Keep everything, unstaged")
                onTriggered: page.moveBranchHere("mixed")
            }
            AppMenuItem {
                text: qsTr("Discard everything after it")
                onTriggered: page.moveBranchHere("hard")
            }
        }
        AppMenuSeparator {}
        AppMenuItem {
            text: qsTr("Copy commit hash")
            onTriggered: clipboard.copy(page.menuOid)
        }
    }

    ClipboardHelper {
        id: clipboard
    }

    // ---- rewriting one commit --------------------------------------
    // Not confirmed even for a commit a remote already has: nothing
    // here leaves the machine, and the push that would spread it is
    // asked about on its own.
    function squashCommit(oidHex) {
        repoTab.squashIntoParent(oidHex)
    }

    // ---- taking the branch back to an earlier commit ----------------
    // git calls this a reset; the menu says what it does, which is move
    // the branch. Offered only where there is a branch to move and
    // somewhere to move it to: detached HEAD has none, a stash sits on
    // no branch's history, an operation in progress is left through
    // Continue / Abort instead (mid-merge a soft reset refuses outright
    // and the other two abandon the merge without a word), and the
    // commit the branch already stands on is not a move at all.
    readonly property bool canMoveBranchHere:
        repoTab.state === "open" && repoTab.busyCount === 0
        && !workTree.detached && workTree.branch !== ""
        && workTree.opText === ""
        && page.menuOid !== "" && page.menuOid !== workTree.headOid
        && graphModel.stashRefOf(page.menuOid) === ""

    // Only the discarding one is asked about. Keeping the work staged or
    // unstaged leaves every byte where it is — the branch moves, and a
    // commit puts back what it skipped — while discarding is the one
    // that leaves nothing to put back.
    function moveBranchHere(mode) {
        // Held apart from the menu's own oid: the question outlives the
        // menu, and the answer must still act on the row it was asked
        // about.
        const oid = page.menuOid
        if (mode !== "hard") {
            repoTab.resetTo(oid, mode)
            return
        }
        page.confirmRequested(
            qsTr("Discard everything after %1?").arg(page.menuShort),
            qsTr("%1 goes back to this commit, and the working tree with "
                 + "it: changes to tracked files are thrown away whether "
                 + "they are staged or not, and nothing here keeps a copy "
                 + "of them.\n\nUntracked files are left alone. The "
                 + "commits after this one stay in the repository until "
                 + "git next cleans up, but nothing in this window points "
                 + "at them any more.").arg(workTree.branch),
            qsTr("Discard changes"),
            function () { repoTab.resetTo(oid, "hard") })
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
        }
    }

    // ---- smoke hook ------------------------------------------------
    // PG_AUTO_ACT runs one write operation through exactly the code
    // path a click takes, so the wiring can be proven headlessly. The
    // dispatch is equality on a bare verb; nothing here parses.
    Timer {
        id: autoActTimer
        interval: 1200
        onTriggered: page.runAutoAct()
    }
    // HEAD's author has to arrive before the offer to take it over can
    // be there to tick.
    Timer {
        id: resetAuthorTimer
        interval: 800
        onTriggered: {
            AppBackend.report("head_author differs="
                              + repoTab.headAuthorDiffers
                              + " name=" + repoTab.headAuthorName)
            wipPane.setResetAuthorChecked(true)
            wipPane.setMessage(AppBackend.autoActArg, "")
            page.commitNow()
        }
    }
    // The diff has to arrive before a row of it can be staged.
    Timer {
        id: stageRowTimer
        interval: 800
        onTriggered: page.stageSelection(
            0, AppBackend.autoAct === "stage-line" ? 0 : -1)
    }
    // The message has to arrive before it can be typed over, and the
    // "is this commit ours to rewrite?" answer before it may be saved.
    Timer {
        id: rewordTimer
        interval: 800
        onTriggered: {
            detailsPane.setMessageText(AppBackend.autoActArg, "")
            // "edit-message" stops here, with the save row on screen.
            if (AppBackend.autoAct === "reword")
                detailsPane.submitMessage()
            // "edit-message-leave" walks away from the unsaved text,
            // which is what raises the question about dropping it;
            // "-discard" then answers it, which lets the move through.
            else if (AppBackend.autoAct === "edit-message-leave"
                     || AppBackend.autoAct === "edit-message-discard") {
                page.activateRow(graphModel.oidAt(graphModel.rowOf(page.selectedOid) + 1))
                if (AppBackend.autoAct === "edit-message-discard")
                    detailsPane.leaveResolved(true)
            }
        }
    }
    function runAutoAct() {
        const act = AppBackend.autoAct
        const arg = AppBackend.autoActArg
        if (act === "commit") {
            repoTab.stageAll()
            wipPane.setMessage(arg, "")
            page.commitNow()
        } else if (act === "amend") {
            // The message is supplied, so skip the prefill request
            // that would otherwise land on top of it.
            wipPane.setAmendChecked(true)
            page.amending = true
            wipPane.setMessage(arg, "")
            page.commitNow()
        } else if (act === "amend-reset-author") {
            // Whether authorship is HEAD's to take over is only known
            // once HEAD has been read, so this one goes the long way
            // round: turn amend on and wait for the answer.
            wipPane.setAmendChecked(true)
            page.amendToggled(true)
            resetAuthorTimer.start()
        } else if (act === "stash" || act === "stash-staged") {
            // Through the dialog, like the button: it is what decides
            // which options the stash is made with.
            stashDialog.open()
            if (act === "stash-staged")
                stashDialog.setOptions(false, false, true)
            stashDialog.apply()
        } else if (act === "stash-file") {
            page.openFileMenu("unstaged", arg)
            repoTab.stashPath(arg, "")
        } else if (act === "stash-dialog") {
            // Opened and left standing, for a look at it.
            stashDialog.open()
        } else if (act === "file-menu") {
            page.openFileMenu("unstaged", arg)
        } else if (act === "amend-author") {
            // The amend editor with authorship on offer, left standing.
            wipPane.setAmendChecked(true)
            page.amendToggled(true)
        } else if (act === "switch") {
            page.switchTo("branch", arg, arg)
        } else if (act === "switch-leave" || act === "switch-merge") {
            // What each button of the in-the-way dialog does, reached
            // the way the dialog reaches it.
            page.moveKind = "branch"
            page.moveTarget = arg
            page.runSwitch(act === "switch-leave" ? "stash" : "merge")
        } else if (act === "switch-remote") {
            page.switchTo("remote", arg, arg)
        } else if (act === "squash") {
            page.openCommitMenu(branchesModel.headOid)
            page.squashCommit(branchesModel.headOid)
        } else if (act === "reword" || act === "edit-message"
                   || act === "edit-message-leave"
                   || act === "edit-message-discard") {
            // Through the pane, like typing: selecting the commit puts
            // its message in the boxes, and the boxes are what saves.
            // "edit-message" leaves it unsaved, for the editing state.
            // Detached there is no branch tip to name, so the newest
            // row stands in — which is a commit other than HEAD.
            page.jumpToRef(branchesModel.headOid !== ""
                           ? branchesModel.headOid : graphModel.oidAt(0))
            rewordTimer.start()
        } else if (act === "cherry-pick") {
            repoTab.cherryPick(arg)
        } else if (act === "reset-soft" || act === "reset-mixed"
                   || act === "reset-hard-confirm") {
            // Through the menu, like clicking it: the row the menu was
            // opened on is where the branch lands. "-confirm" stops at
            // the question, so nothing should have moved until it is
            // answered.
            page.openCommitMenu(arg)
            page.moveBranchHere(act === "reset-soft" ? "soft"
                                : act === "reset-mixed" ? "mixed" : "hard")
        } else if (act === "reset-hard") {
            // Past the question, for the discarding write itself.
            repoTab.resetTo(arg, "hard")
        } else if (act === "commit-menu" || act === "reset-menu") {
            // Nothing written: the menu is left standing so its wording
            // can be photographed from outside (a popup draws in the
            // window overlay, where grabToImage cannot reach it). Which
            // rows are offered is said in words as well — a greyed row
            // is not something a screenshot can be trusted on.
            page.openCommitMenu(arg)
            if (act === "reset-menu")
                resetMenu.popup()
            AppBackend.report("commit_menu can_move=" + page.canMoveBranchHere)
        } else if (act === "stage-hunk" || act === "stage-line") {
            page.toggleDiff("unstaged", arg, "")
            stageRowTimer.start()
        } else if (act === "push") {
            page.pushNow()
        } else if (act === "force-push") {
            repoTab.pushCurrent("lease", page.upstreamOid())
        } else if (act === "force-push-confirm") {
            // Goes through the confirmation, so nothing should be
            // pushed until someone answers it.
            page.forcePushNow()
        } else if (act === "fetch") {
            repoTab.fetch("")
        } else if (act === "preview") {
            page.toggleDiff("untracked", arg, "")
        } else if (act === "preview-unstaged") {
            page.toggleDiff("unstaged", arg, "")
        } else if (act === "preview-staged") {
            page.toggleDiff("staged", arg, "")
        } else if (act === "settings") {
            page.settingsDialogRequested()
            AppBackend.setAutoFetchMinutes(Number(arg))
        }
        AppBackend.report("auto_act ran=" + act)
    }

    // A finished write the editor asked for: clear it only once git
    // says the commit landed, so a rejected one keeps its text.
    property int seenWriteSeq: 0
    function absorbWriteResult() {
        if (repoTab.writeSeq === page.seenWriteSeq)
            return
        page.seenWriteSeq = repoTab.writeSeq
        if (repoTab.lastWriteError !== "")
            return
        if (repoTab.lastWriteOp === "commit") {
            page.clearCommitEditor()
            wipPane.setAmendChecked(false)
            page.amending = false
        }
        if (repoTab.lastWriteOp === "stage" || repoTab.lastWriteOp === "unstage")
            page.reloadDiff()
        // The message landed: the editor stops offering to save it, and
        // keeps what was written until the selection catches up with
        // the commit that now carries it.
        if (repoTab.lastWriteOp === "reword")
            detailsPane.noteMessageSaved()
        // Moving HEAD rewrites the working tree under the diff pane:
        // the file it holds may not even exist where the move landed,
        // so the center goes back to the graph that was moved through.
        // A refused move rewrote nothing, so it keeps its view. Taking
        // the branch back does the same to the file, and to which side
        // of the index it sits on.
        if ((repoTab.lastWriteOp === "checkout" && !page.moveRefused)
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
    property string diffKey: ""
    property string diffKind: ""
    property string diffPath: ""
    property string diffOrigPath: ""
    // Whether the shown diff is a working-tree file (stageable).
    property bool diffFromWt: false
    readonly property bool diffStaged: page.diffKind === "staged"
    function toggleDiff(kind, path, origPath) {
        const key = kind + ":" + path
        if (page.diffShown && page.diffKey === key) {
            page.closeDiff()
            return
        }
        page.diffKey = key
        page.diffKind = kind
        page.diffPath = path
        page.diffOrigPath = origPath
        page.diffFromWt = kind !== "commit"
        if (kind === "commit")
            diffModel.requestCommitFile(detailsModel.shaHex, detailsModel.parentHex,
                                        path, origPath)
        else
            diffModel.requestWorkTree(kind, path, origPath)
        page.diffShown = true
    }
    // Stages (or unstages) one hunk, or one line of it. The indices
    // address the diff currently on screen, so the pane is reloaded
    // afterwards: once the patch is applied the rows have moved.
    function stageSelection(hunk, line) {
        repoTab.stageSelection(page.diffKind, page.diffPath, page.diffOrigPath, hunk, line)
        page.pendingDiffReload = true
    }
    property bool pendingDiffReload: false
    function reloadDiff() {
        if (!page.pendingDiffReload || !page.diffShown)
            return
        page.pendingDiffReload = false
        diffModel.requestWorkTree(page.diffKind, page.diffPath, page.diffOrigPath)
    }

    function closeDiff() {
        page.diffShown = false
        page.diffKey = ""
        page.diffKind = ""
        page.diffPath = ""
        page.diffOrigPath = ""
        page.diffFromWt = false
        diffModel.clear()
    }

    // Exposed for the window toolbar (acts on the active tab).
    readonly property var pageTab: repoTab
    readonly property var pageWt: workTree

    RepoTab { id: repoTab }
    GraphModel { id: graphModel }
    WorkTreeModel { id: workTree }
    DetailsModel { id: detailsModel }
    DiffModel { id: diffModel }
    NavSectionModel { id: branchesModel }
    NavSectionModel { id: remotesModel }
    NavSectionModel { id: worktreeModel }
    NavSectionModel { id: worktreesModel }
    NavSectionModel { id: stashesModel }
    NavSectionModel { id: tagsModel }

    Component.onCompleted: {
        if (page.blank)
            return // no session to attach to; every model stays empty
        repoTab.attach(page.tab_id)
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
        if (AppBackend.autoAct !== "")
            autoActTimer.start()
    }

    /// The window's focus epoch (bumped when the window regains focus)
    /// triggers a quick refresh of the visible page.
    property int focusEpoch: 0
    onFocusEpochChanged: {
        if (page.visible && repoTab.state === "open")
            repoTab.refreshQuick()
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
        // Any new selection settles where the last rewrite left off.
        page.rewordRow = -1
        // A click moves the highlight itself, but one held back by the
        // unsaved-message question does not: the question put it back
        // where it was, so the answer has to move it again.
        const row = graphModel.rowOf(oidHex)
        if (row >= 0)
            graphPane.setCurrentRow(row)
        if (oidHex !== "" && !/[^0]/.test(oidHex)) {
            page.showWip()
            return
        }
        page.wipShown = false
        page.selectedOid = oidHex
        page.selectedStashRef = graphModel.stashRefOf(oidHex)
        detailsModel.request(oidHex)
        page.askInHistory(oidHex)
        page.closeDiff()
    }

    // Selection policy: restore across the tag-swap reset, and default
    // to the current branch's newest commit on first load so the
    // details pane always shows something.
    function trySelectDefault() {
        if (page.selectedOid !== "" || page.wipShown || AppBackend.autoSelect
                || AppBackend.autoWip || graphModel.rowTotal === 0)
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
                if (page.selectedOid !== "") {
                    const row = graphModel.rowOf(page.selectedOid)
                    if (row >= 0) {
                        graphPane.setCurrentRow(row)
                        if (resetHappened)
                            graphPane.anchorSoon()
                    } else if (page.rewordRow >= 0) {
                        page.followRewrittenCommit()
                    }
                }
            }
            page.trySelectDefault()
        }
    }
    Connections {
        target: branchesModel
        function onChanged() { page.trySelectDefault() }
    }
    Connections {
        target: worktreeModel
        function onChanged() {
            if (worktreeModel.total === 0 && page.wipShown)
                page.wipShown = false
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
    Timer {
        id: scrollToTimer
        interval: 600
        onTriggered: {
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
        enabled: AppBackend.scrollTo !== ""
        function onStatsChanged() {
            if (graphModel.finishCount > 0)
                scrollToTimer.restart()
        }
    }

    // Scroll benchmark (PG_AUTO_SCROLL=1): after the stream finishes,
    // animate 3000 rows over 12s and report the measured fps. Frame
    // counting rides on the window's frameCounter property.
    property bool benchStarted: false
    Connections {
        target: graphModel
        enabled: AppBackend.autoScroll
        function onStatsChanged() {
            if (!page.benchStarted && !graphModel.loading && graphModel.rowTotal > 0) {
                page.benchStarted = true
                benchPrep.start()
            }
        }
    }
    Timer {
        id: benchPrep
        interval: 800
        onTriggered: {
            page.benchT0 = Date.now()
            page.benchFrames0 = page.Window.window.frameCounter
            benchAnim.start()
        }
    }
    property real benchT0: 0
    property int benchFrames0: 0
    NumberAnimation {
        id: benchAnim
        target: graphPane.view
        property: "contentY"
        from: 0
        to: Math.min(3000, graphModel.rowTotal - 40) * Theme.graphRowHeight
        duration: 12000
        onStopped: {
            const secs = (Date.now() - page.benchT0) / 1000
            const frames = page.Window.window.frameCounter - page.benchFrames0
            AppBackend.report("scroll_bench fps=" + (frames / secs).toFixed(1)
                              + " rows=" + graphModel.rowTotal)
        }
    }

    // Automation (PG_AUTO_SELECT=1): select the newest row, then open
    // the first changed file's diff — exercises the full pipeline for
    // screenshot-based smoke tests.
    property bool autoSelected: false
    Connections {
        target: graphModel
        enabled: AppBackend.autoSelect
        function onStatsChanged() {
            if (!page.autoSelected && graphModel.rowTotal > 0) {
                page.autoSelected = true
                graphPane.setCurrentRow(0)
                page.activateRow(graphModel.oidAt(0))
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

    // Open failed: show git's own message
    Column {
        anchors.centerIn: parent
        visible: repoTab.state === "error"
        spacing: Theme.spaceMd
        width: Math.min(700, page.width - 2 * Theme.spaceXl)
        Label {
            text: qsTr("Could not open this folder as a git repository")
            font.pixelSize: Theme.fontLg
            font.weight: Font.DemiBold
            anchors.horizontalCenter: parent.horizontalCenter
        }
        Label {
            text: repoTab.error
            color: Theme.danger
            wrapMode: Text.Wrap
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
        }
        HoverButton {
            text: qsTr("Close tab")
            anchors.horizontalCenter: parent.horizontalCenter
            onClicked: page.closeTabRequested()
        }
    }
    /// The failed-open screen's "Close tab" button.
    signal closeTabRequested()

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        visible: repoTab.state !== "error"

        // ---- three-pane layout --------------------------------------
        // (repository state / search / reload live in the window
        // toolbar, next to the tabs)
        SplitView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            orientation: Qt.Horizontal
            handle: Rectangle {
                implicitWidth: Theme.splitterWidth
                implicitHeight: Theme.splitterWidth
                color: Theme.borderSubtle
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
                onRefActivated: oidHex => page.jumpToRef(oidHex)
                onRefMenuRequested: (name, oidHex, isRemote) =>
                    page.openRefMenu(name, oidHex, isRemote)
                onWorktreeActivated: path => page.openRepositoryPathRequested(path)
            }

            // Center: commit graph ⇄ file diff
            StackLayout {
                SplitView.fillWidth: true
                SplitView.minimumWidth: 420
                currentIndex: page.diffShown ? 1 : 0

                GraphPane {
                    id: graphPane
                    graphModel: graphModel
                    worktreeModel: worktreeModel
                    blank: page.blank
                    onRowActivated: oidHex => page.activateRow(oidHex)
                    onCommitMenuRequested: oidHex => page.openCommitMenu(oidHex)
                    onOpenRepositoryRequested: page.openRepositoryPicker()
                }

                DiffPane {
                    diffModel: diffModel
                    fromWorkTree: page.diffFromWt
                    staged: page.diffStaged
                    busy: repoTab.busyCount > 0
                    onCloseRequested: page.closeDiff()
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
                SplitView.preferredWidth: 400
                SplitView.minimumWidth: 300
                color: Theme.bgSurface

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
                    onStashRequested: stashDialog.open()
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
                        ? qsTr("A stash keeps the message it was made with.")
                        : (detailsModel.shaHex !== "" && !page.selectedInHistory
                           ? qsTr("This commit is not in the history you are on. "
                                  + "Switch to a branch that has it to edit its "
                                  + "message.")
                           : "")
                    busy: repoTab.busyCount > 0
                    asking: page.pendingMove !== null
                    // HEAD's own commit, not the current branch's tip:
                    // detached, there is no branch to ask.
                    headOid: workTree.headOid
                    onLeaveResolved: discard => page.resolveLeave(discard)
                    onMessageSubmitted: (oidHex, subject, body) =>
                        page.saveMessage(oidHex, subject, body)
                    onFileActivated: (path, origPath) =>
                        page.toggleDiff("commit", path, origPath)
                    onParentClicked: oidHex => page.jumpToRef(oidHex)
                    onCopyRequested: text => clipboard.copy(text)
                    onApplyStashRequested: selector => repoTab.applyStash(selector)
                    onPopStashRequested: selector => {
                        repoTab.popStash(selector)
                        page.selectedStashRef = ""
                    }
                }
            }
        }
    }

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
            page.closeDiff()
        })
    }
}
