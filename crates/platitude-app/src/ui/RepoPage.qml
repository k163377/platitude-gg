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
        page.pendingHeadSelect = false
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
            if (page.menuOid !== ""
                    && repoTab.publishRange === page.menuOid + "^!")
                page.menuPublished = repoTab.publishPublished > 0
            if (page.selectedOid !== ""
                    && repoTab.publishRange === page.selectedOid + "^!")
                page.selectedPublished = repoTab.publishPublished > 0
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
    // What the pending move is: kind is "branch" / "remote" / "commit" /
    // "force" (a local branch moved to `moveStart` before landing on it).
    property string moveKind: ""
    property string moveTarget: ""
    property string moveLabel: ""
    property string moveStart: ""
    // Set while the last move sits refused, so the panes are left as they
    // were: nothing moved, so nothing about them is stale.
    property bool moveRefused: false

    function switchTo(kind, target, label, start) {
        page.moveKind = kind
        page.moveTarget = target
        page.moveLabel = label
        page.moveStart = start === undefined ? "" : start
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
        else if (page.moveKind === "force")
            repoTab.checkoutForceCreate(page.moveTarget, page.moveStart, carry)
    }

    // ---- what a chip leads to --------------------------------------
    // One dispatcher for every way of asking to move to a named ref: the
    // graph's chips, the list the stacked ones open into, and the
    // sidebar's menu. `record` is the chip as it is drawn — kind letter,
    // three flags, then the name (see encode.rs).
    function activateRecord(record) {
        if (record !== "")
            page.switchToRef(record[0], record.substring(4))
    }
    function switchToRef(kind, name) {
        if (repoTab.state !== "open" || repoTab.busyCount > 0)
            return
        if (kind === "L") {
            // Already standing there.
            if (name !== workTree.branch)
                page.switchTo("branch", name, name, "")
            return
        }
        // The detached-HEAD marker names no branch, and a tag is a
        // standing mark rather than somewhere to carry on from — moving
        // onto one can only detach, which stays an explicit choice
        // (the commit menu's "Switch to this commit").
        if (kind !== "R")
            return
        const local = repoTab.localNameFor(name)
        // With no local branch of that name, landing on the remote one
        // means making it: one branch, tracking, nothing to lose, so
        // nothing to ask about.
        if (branchesModel.oidOfName(local) === "")
            page.switchTo("remote", name, local, "")
        else
            page.askMoveBranchOnto(local, name)
    }

    // A local branch of that name exists, and it is not here — its own
    // row is elsewhere in the graph. Landing on the remote branch means
    // moving the local one onto it, which is the one thing here that can
    // leave work with nothing pointing at it.
    function askMoveBranchOnto(local, remoteRef) {
        page.startRowAsk(
            remotesModel.oidOfName(remoteRef),
            qsTr("Move %1 here?").arg(local),
            qsTr("Commits only %1 has stop being reachable.").arg(local),
            false,
            qsTr("Move %1 here").arg(local),
            function () { page.switchTo("force", local, local, remoteRef) })
        // Say it in words too: a smoke run asserts on the report line
        // without having to look at the shot.
        if (AppBackend.autoAct !== "")
            AppBackend.report("move_branch_asked local=" + local)
    }

    // ---- standing questions ----------------------------------------
    // One question at a time, asked on the bar that comes down over the
    // graph, with the row it is about marked (デザイン規約
    // §可否・警告の出し場所). The run waits here; Escape, another ask or
    // a click anywhere else walks away from it. The bar stands wherever
    // the question came from — a row outside the loaded window simply
    // goes unmarked — so nothing needs a dialog to fall back to.
    property var rowAskRun: null
    function startRowAsk(oidHex, label, detail, danger, acceptText, run) {
        page.rowAskRun = run
        graphPane.startAsking(oidHex, label, detail, acceptText, danger)
    }
    function stopRowAsk() {
        page.rowAskRun = null
        graphPane.stopAsking()
    }
    function answerRowAsk() {
        const run = page.rowAskRun
        page.stopRowAsk()
        if (run)
            run()
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
        // Say it in words too: a smoke run asserts on the report line
        // without having to look at the overlay shot.
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
    /// What the branch can do with its remote, worked out before anything
    /// is sent (デザイン規約 §リモートへ送る):
    ///
    /// - `closed`   — nothing here has a remote to go to
    /// - `publish`  — the branch is not on a remote yet
    /// - `ready`    — commits of ours to add, and nothing in the way
    /// - `clean`    — the remote already has them all
    /// - `behind`   — the remote moved on; we have nothing to add
    /// - `diverged` — both moved; only an overwrite can land
    ///
    /// The counts behind this come from the last fetch, so they prove the
    /// negative only: a push may still be refused when they say it fits.
    readonly property string pushState:
        repoTab.state !== "open" || workTree.detached
        || workTree.branch === "" || repoTab.remoteCount === 0 ? "closed"
        : workTree.upstream === "" || !workTree.upstreamTracked ? "publish"
        : workTree.behind > 0 ? (workTree.ahead > 0 ? "diverged" : "behind")
        : workTree.ahead > 0 ? "ready" : "clean"
    readonly property bool canPush: (pushState === "publish"
                                     || pushState === "ready")
                                    && repoTab.busyCount === 0
    /// Whether there is anything of ours to put on the remote by force.
    /// Nothing to add means nothing to overwrite with.
    readonly property bool canForcePush: (pushState === "ready"
                                          || pushState === "diverged")
                                         && repoTab.busyCount === 0
    function pushNow() {
        repoTab.pushCurrent("", "")
    }
    /// Replace what the remote holds with this branch.
    ///
    /// The lease is pinned to the commit this window has on screen rather
    /// than left to compare against the tracking ref: a background fetch
    /// must not turn this into a plain force. A remote that moved since is
    /// refused, and the refusal is answered by a fetch (core).
    function forcePush() {
        repoTab.pushCurrent("lease", page.upstreamOid())
    }
    /// The menu's way to the same thing, for anyone who cannot hold a
    /// button down. Here the asking is the dialog's job rather than the
    /// press's, and its one remaining job is to say what the overwrite
    /// costs: status's behind count is exactly the commits the remote
    /// loses, dated to the fetch it came from (デザイン規約 §相手の履歴を
    /// 置き換える).
    function forcePushNow() {
        const lost = workTree.behind > 0
            ? qsTr("%1 has %n commit(s) this branch does not, as seen at "
                   + "the last fetch. Overwriting removes them, and anyone "
                   + "who already pulled them keeps a history that no "
                   + "longer matches.", "", workTree.behind)
              .arg(page.pushTargetLabel)
            : qsTr("As seen at the last fetch, %1 has no commits of its "
                   + "own, so overwriting it removes nothing.")
              .arg(page.pushTargetLabel)
        page.confirmRequested(
            qsTr("Overwrite %1 with this branch?").arg(page.pushTargetLabel),
            lost + "\n\n"
            + qsTr("The push is refused if the remote moved since this "
                   + "window last saw it."),
            qsTr("Force push"),
            function () { page.forcePush() })
    }
    /// Commit the remote-tracking branch points at, as shown here.
    function upstreamOid() {
        return workTree.upstream !== ""
               ? remotesModel.oidOfName(workTree.upstream) : ""
    }

    // ---- context menu on a sidebar row ------------------------------
    // `menuRefName` is what the row shows, `menuRefId` what git knows it
    // by (they differ for a stash: a message and a selector).
    property string menuRefKind: ""
    property string menuRefName: ""
    property string menuRefId: ""
    property string menuRefOid: ""
    function openRefMenu(kind, name, full, oidHex) {
        page.menuRefKind = kind
        page.menuRefName = name
        page.menuRefId = full
        page.menuRefOid = oidHex
        refMenu.popup()
    }
    AppMenu {
        id: refMenu
        AppMenuItem {
            text: qsTr("Switch to %1").arg(page.menuRefId)
            visible: page.menuRefKind === "branch" || page.menuRefKind === "remote"
            enabled: page.menuRefId !== workTree.branch
            // Through the same dispatcher the graph's chips use: a remote
            // branch whose local one already exists cannot simply be
            // created, and that answer belongs in one place.
            onTriggered: page.switchToRef(page.menuRefKind === "remote" ? "R" : "L",
                                          page.menuRefId)
        }
        // Nothing here repeats a gesture: renaming and creating a branch
        // on a tag are a click away on the row itself, and every row this
        // menu keeps costs the ones that have nowhere else to go
        // (デザイン規約 §メニュー).
        AppMenuItem {
            text: qsTr("Delete…")
            visible: page.menuRefKind === "branch" || page.menuRefKind === "tag"
                     || page.menuRefKind === "stash"
            // The branch under the working tree cannot be deleted at all,
            // and git says so rather than doing something else.
            enabled: repoTab.busyCount === 0
                     && !(page.menuRefKind === "branch"
                          && page.menuRefId === workTree.branch)
            onTriggered: page.deleteRow(page.menuRefKind, page.menuRefId,
                                        page.menuRefName, page.menuRefOid)
        }
        AppMenuSeparator {}
        AppMenuItem {
            text: qsTr("Copy commit hash")
            onTriggered: clipboard.copy(page.menuRefOid)
        }
    }

    // ---- what the sidebar's rows ask for ---------------------------
    /// A row was renamed in place. Nothing is confirmed: a name is not
    /// history, and the one it replaces is a switch away (a tag and a
    /// stash are re-made under the new name by core, which is the only
    /// rename git has for them).
    function renameRow(kind, id, name) {
        if (kind === "branch")
            repoTab.renameBranch(id, name, false)
        else if (kind === "tag")
            repoTab.renameTag(id, name)
        else if (kind === "stash")
            repoTab.renameStash(id, name)
    }

    /// Deleting a row from the left menu.
    ///
    /// A branch goes the way git deletes one — `-d`, which refuses while
    /// the branch holds commits nothing else does. That refusal is the
    /// question worth asking, so it is asked when it arrives rather than
    /// guessed at beforehand (P2-確認事項 §B). A tag and a stash have no
    /// such refusal in git, and both can take something with them, so they
    /// are asked about up front.
    property string pendingDeleteBranch: ""
    /// Refusals this page already has an answer for. The question bar
    /// explains them, so the command log stays where it was rather than
    /// raising itself over the same news (デザイン規約 §git が言ったこと
    /// を読む場所). Counted rather than flagged because the refusal and
    /// the write result arrive on separate paths, in no fixed order.
    property int expectedRefusals: 0
    function deleteRow(kind, id, name, oidHex) {
        if (kind === "branch") {
            page.pendingDeleteBranch = id
            page.expectedRefusals++
            repoTab.deleteBranch(id, false)
        } else if (kind === "tag") {
            page.startRowAsk(
                oidHex,
                qsTr("Delete %1?").arg(id),
                qsTr("What only this tag reaches stops being reachable."),
                true,
                qsTr("Delete %1").arg(id),
                function () { repoTab.deleteTag(id) })
        } else if (kind === "stash") {
            page.startRowAsk(
                oidHex,
                qsTr("Delete this stash?"),
                qsTr("What it holds is kept nowhere else."),
                true,
                qsTr("Delete it"),
                function () { repoTab.dropStash(id) })
        }
    }
    /// git refused the plain delete: the branch has commits of its own.
    /// Forcing is the only way through, and it is the one thing here that
    /// leaves work with nothing pointing at it.
    function askForceDelete(name) {
        const oid = branchesModel.oidOfName(name)
        page.startRowAsk(
            oid,
            qsTr("Delete %1 anyway?").arg(name),
            qsTr("Commits only %1 has stop being reachable.").arg(name),
            true,
            qsTr("Delete %1").arg(name),
            function () { repoTab.deleteBranch(name, true) })
        if (AppBackend.autoAct !== "")
            AppBackend.report("force_delete_asked branch=" + name)
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
    // Whether a remote already has the menu's commit. Rewriting it is
    // not asked about — nothing here leaves the machine — but 要望.md
    // wants it said, so the squash row carries a tag the way the amend
    // editor does. The answer lands a frame after the menu opens.
    property bool menuPublished: false
    function openCommitMenu(oidHex) {
        page.menuOid = oidHex
        page.menuPublished = false
        if (repoTab.state === "open")
            repoTab.checkPublish(oidHex + "^!")
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
        // No entry for editing the message: the click that opens this
        // menu selects the row, which puts the message in the details
        // pane's own editable boxes. A second way in would say the same
        // thing twice, and every row here costs the ones still to come.
        AppMenuItem {
            text: qsTr("Fold into the commit before it")
            // Said, not asked (要望: rewriting a pushed commit shows a
            // warning): the fold goes ahead, and this tag is the warning.
            note: page.menuPublished ? qsTr("already pushed") : ""
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
            // "here" rather than "to this commit": the commit it means
            // is the row this menu was opened on, and the row already
            // says which one that is.
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

    // ---- double-click on a graph row -------------------------------
    // The row leads where its chip says, and a row with no chip is
    // offered one instead of doing nothing: the gesture always answers.
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

    // The refs one chip had to stack, unstacked under it. It opens and
    // closes with the pointer, and the pointer is over exactly one of the
    // two things that keep it up: the chip, or the list itself.
    RefListPopup {
        id: refList
        currentBranch: workTree.branch
        onPicked: record => page.activateRecord(record)
        onClosed: page.refListWanted = false
        onPointerInsideChanged: page.settleRefList()
    }
    property bool refListWanted: false
    function openRefList(records, anchor) {
        const at = anchor.mapToItem(page, 0, anchor.height)
        refList.records = records
        refList.x = at.x
        refList.y = at.y
        page.refListWanted = true
        refList.open()
    }
    function closeRefListUnlessEntered() {
        page.refListWanted = false
        page.settleRefList()
    }
    // The list opens flush under the chip, so walking into it takes the
    // pointer off the chip on the way, and walking back out puts it on
    // again. Both hovers change in the same frame and in no fixed order,
    // so the answer waits for the end of this round of events, by which
    // time whichever of the two now holds the pointer has said so.
    function settleRefList() {
        Qt.callLater(function () {
            if (!refList.pointerInside && !page.refListWanted)
                refList.close()
        })
    }
    Connections {
        // The row it hangs off is a delegate, and delegates travel: once
        // the graph moves under it the list is pointing at nothing.
        target: graphPane.view
        function onContentYChanged() { refList.close() }
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
        // "after it": the marked row is the very commit the branch would
        // go back to, so the question needs no other name for it.
        page.startRowAsk(
            oid,
            qsTr("Discard everything after it?"),
            qsTr("Changes to tracked files go too; untracked files stay."),
            true,
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

    // ---- smoke hook ------------------------------------------------
    // PG_AUTO_ACT runs one operation — a write, or a surface left
    // standing for the overlay shot — through exactly the code path a
    // click takes, so the wiring can be proven headlessly. The dispatch
    // is equality on a bare verb; the argument passes through as
    // whatever the verb needs (a name, an oid, a row number).
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
            // Through the pane's card, like the button: it is what
            // decides which options the stash is made with.
            // "stash-staged" clicks the staged-only box the way a
            // person would, so the run proves the ticks it clears
            // really clear.
            page.showWip()
            wipPane.openStashPanel()
            if (act === "stash-staged")
                wipPane.stashClickStagedOnly()
            wipPane.stashApply()
        } else if (act === "stash-file") {
            page.openFileMenu("unstaged", arg)
            repoTab.stashPath(arg, "")
        } else if (act === "stash-dialog") {
            // Opened and left standing, for a look at it. With the
            // argument "staged-only" the staged-only box is clicked
            // first, so the shot shows what that click clears.
            page.showWip()
            wipPane.openStashPanel()
            if (arg === "staged-only")
                wipPane.stashClickStagedOnly()
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
            page.switchTo("remote", arg, arg, "")
        } else if (act === "nav-dbl") {
            // A double-click in the left menu, entered where the row
            // enters it. The argument is `<section>:<name>`.
            const cut = arg.indexOf(":")
            const section = arg.substring(0, cut)
            const rowName = arg.substring(cut + 1)
            const model = section === "tag" ? tagsModel
                        : section === "remote" ? remotesModel : branchesModel
            sidebarPane.activateRow(section, rowName, rowName,
                                    model.oidOfName(rowName))
        } else if (act === "nav-rename" || act === "rename-branch"
                   || act === "rename-tag" || act === "rename-stash") {
            // The box the second click opens, entered at the same place.
            // "nav-rename" leaves it standing for the shot; the others
            // type the argument into it and accept. Which row: the
            // current branch, the first tag, the first stash.
            const kind = act === "rename-tag" ? "tag"
                       : act === "rename-stash" ? "stash" : "branch"
            const id = kind === "branch" ? workTree.branch
                     : kind === "tag" ? tagsModel.nameAt(0) : stashesModel.fullAt(0)
            const shown = kind === "stash" ? stashesModel.nameAt(0) : id
            sidebarPane.beginRename(kind, id, shown)
            if (act !== "nav-rename")
                sidebarPane.submitEdit(arg)
        } else if (act === "delete-branch" || act === "delete-tag"
                   || act === "delete-stash") {
            // Through the menu's own path: the branch one runs the plain
            // delete (and raises the question when git refuses), the
            // other two stop at the question.
            const kind = act.substring("delete-".length)
            const id = kind === "stash" ? stashesModel.fullAt(0) : arg
            const model = kind === "tag" ? tagsModel : branchesModel
            page.deleteRow(kind, id, id,
                           kind === "stash" ? stashesModel.oidOfName(stashesModel.nameAt(0))
                                            : model.oidOfName(id))
        } else if (act === "delete-force") {
            repoTab.deleteBranch(arg, true)
        } else if (act === "delete-tag-go") {
            repoTab.deleteTag(arg)
        } else if (act === "delete-stash-go") {
            repoTab.dropStash(stashesModel.fullAt(0))
        } else if (act === "branch-at-tag") {
            // The box a double-click puts on a tag row, accepted with the
            // argument as the new branch's name.
            sidebarPane.beginBranchAt(tagsModel.nameAt(0),
                                      tagsModel.oidOfName(tagsModel.nameAt(0)))
            sidebarPane.submitEdit(arg)
        } else if (act === "dbl-local" || act === "dbl-remote") {
            // What a double-click on a chip does, entered where the
            // delegate enters it: the record is the chip as it is drawn
            // (kind letter, three flags, name).
            page.activateRecord((act === "dbl-local" ? "L000" : "R000") + arg)
        } else if (act === "move-branch") {
            // Past the question, for the write it guards: the local
            // branch of that name is moved onto the remote one.
            page.switchTo("force", repoTab.localNameFor(arg),
                          repoTab.localNameFor(arg), arg)
        } else if (act === "name-branch") {
            // From the box's own accept onward — the page never sees the
            // typing, only a name and the row it belongs to.
            graphPane.view.namingSubmitted(graphModel.oidAt(0), arg)
        } else if (act === "ref-list") {
            // The unstacked chips, left standing. Hover cannot be
            // injected on Windows, so this enters where the hover timer
            // would; the argument is the row whose chip is stacked.
            const stacked = graphPane.view.itemAtIndex(Number(arg))
            if (stacked)
                graphPane.view.chipExpandRequested(
                    stacked.chipItem.records, stacked.chipItem)
        } else if (act === "name-box") {
            // Opened and left standing, for a look at it. The argument is
            // the row, since the box only belongs on one with no chips.
            graphPane.startNaming(graphModel.oidAt(Number(arg)))
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
            // Nothing written: the menu is left standing for the overlay
            // shot. Which rows are offered is said in words as well — a
            // greyed row is not something a screenshot can be trusted on.
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
            page.forcePush()
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
        } else if (act === "commands") {
            // Stage and unstage so the log has something in it, then
            // open it the way the toolbar does.
            repoTab.stageAll()
            repoTab.unstageAll()
            page.toggleCommands()
        } else if (act === "commands-fail") {
            // A move to a branch that is not there: a real refusal, in
            // git's own words, that raises the panel by itself.
            repoTab.checkoutBranch("pg-no-such-branch", "carry")
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
        if (repoTab.lastWriteError !== "") {
            // The one refusal this page has a second move for: a branch
            // delete git would not do on its own.
            if (repoTab.lastWriteOp === "branch" && page.pendingDeleteBranch !== "") {
                const refused = page.pendingDeleteBranch
                page.pendingDeleteBranch = ""
                page.askForceDelete(refused)
            }
            return
        }
        // Landed: no refusal is coming for it after all.
        if (page.pendingDeleteBranch !== "") {
            page.pendingDeleteBranch = ""
            page.expectedRefusals = Math.max(0, page.expectedRefusals - 1)
        }
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
    readonly property var pageCommands: commandsModel

    /// Whether the command log is up. Closed is the resting state: the
    /// toolbar's `>_` opens it, and a failed command raises it.
    property bool commandsOpen: false
    function toggleCommands() {
        page.commandsOpen = !page.commandsOpen
        if (page.commandsOpen)
            commandsPane.showLatest()
    }

    RepoTab { id: repoTab }
    CommandsModel { id: commandsModel }
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
    function tryPendingHeadSelect() {
        if (!page.pendingHeadSelect || !branchesModel.refsLoaded)
            return
        const row = branchesModel.headOid !== ""
                    ? graphModel.rowOf(branchesModel.headOid) : -1
        if (row < 0)
            return
        page.pendingHeadSelect = false
        graphPane.setCurrentRow(row)
        page.activateRow(graphModel.oidAt(row))
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
    }
    Connections {
        target: worktreeModel
        function onChanged() {
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

        // The panes sit above the command log, which is closed until it
        // is asked for. Splitting them vertically keeps the log's height
        // in the reader's hands and out of the panes' business.
        SplitView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            orientation: Qt.Vertical
            handle: Rectangle {
                implicitWidth: Theme.splitterWidth
                implicitHeight: Theme.splitterWidth
                color: Theme.borderSubtle
            }

            // ---- three-pane layout --------------------------------------
            // (repository state / search / fetch / push live in the window
            // toolbar, next to the tabs; Reload is the app menu and F5)
            SplitView {
                SplitView.fillHeight: true
                SplitView.minimumHeight: 200
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
                    SplitView.minimumWidth: 420
                    currentIndex: page.diffShown ? 1 : 0

                    GraphPane {
                        id: graphPane
                        graphModel: graphModel
                        worktreeModel: worktreeModel
                        blank: page.blank
                        onRowActivated: oidHex => page.activateRow(oidHex)
                        onCommitMenuRequested: oidHex => page.openCommitMenu(oidHex)
                        onRowSwitchRequested: (oidHex, record) =>
                            page.rowDoubleClicked(oidHex, record)
                        onChipExpandRequested: (records, anchor) =>
                            page.openRefList(records, anchor)
                        onChipCollapseRequested: page.closeRefListUnlessEntered()
                        onCreateBranchRequested: (oidHex, name) =>
                            repoTab.createBranch(name, oidHex, true)
                        onOpenRepositoryRequested: page.openRepositoryPicker()
                        onAskConfirmed: page.answerRowAsk()
                        onAskCancelled: page.stopRowAsk()
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

                    // Which git is doing all this, as faint bare text in
                    // the corner of the pane that has room to spare. It
                    // sits here rather than at the window's edge so the
                    // command log can open without landing on top of it.
                    Label {
                        visible: AppBackend.gitVersion !== ""
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.rightMargin: Theme.spaceSm
                        anchors.bottomMargin: Theme.spaceXs
                        text: qsTr("git %1").arg(AppBackend.gitVersion)
                        color: Theme.textMuted
                        font.pixelSize: Theme.fontSm
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
                            ? qsTr("Rename a stash in the list on the left")
                            : (detailsModel.shaHex !== "" && !page.selectedInHistory
                               ? qsTr("Not in the current history — switch to a "
                                      + "branch that has it")
                               : "")
                        busy: repoTab.busyCount > 0
                        asking: page.pendingMove !== null
                        published: page.selectedPublished
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

            // ---- command log ------------------------------------------
            // Hidden until asked for, and raised by a failure.
            CommandsPane {
                id: commandsPane
                visible: page.commandsOpen
                commandsModel: commandsModel
                errorText: repoTab.lastError
                SplitView.preferredHeight: 280
                SplitView.minimumHeight: 120
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
