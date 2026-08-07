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

    /// The blank page's "Open repository…" button (folder picker).
    signal openRepositoryPicker()
    /// A worktree row was clicked: open that path as a new tab.
    signal openRepositoryPathRequested(string path)
    /// PG_AUTO_ACT=settings wants the window's settings dialog open
    /// for the screenshot.
    signal settingsDialogRequested()

    property string selectedOid: ""

    // Left pane folded down to its icons. Held here rather than in the
    // sidebar because two things decide it: the control on its own band,
    // and a diff opening in the middle — reading a file wants every
    // column and every line the window can give it, and the list of refs
    // is the one thing on screen that has nothing to say about the file.
    property bool sidebarCollapsed: false
    /// The fold the diff put on. Closing the diff takes back exactly
    /// that and nothing anybody did by hand: somebody who opens the list
    /// while reading a file has said they want it, and somebody who folds
    /// it themselves has said they want it folded after.
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
        // A hand on it takes it over from the diff, whichever way it
        // moved it.
        page.foldedByDiff = false
    }

    // Right pane switches to the working-tree (WIP) view.
    property bool wipShown: false
    // Leaving the file list takes its question with it: the bar goes off
    // screen with the pane, and nothing off screen may be answered.
    onWipShownChanged: if (!page.wipShown) page.stopRowAsk()
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
            page.absorbMoveAsk()
            page.absorbWriteResult()
            page.runFetchFailures()
        }
        // The first fetch of a run to fail opens the log, the way a
        // refusal the user asked for does. Only the first: the ones after
        // it are the same news, and a machine that is offline would put
        // the panel back up every interval (デザイン規約 §git が言ったことを
        // 読む場所).
        function onFetchFirstFailed() {
            page.commandsOpen = true
            commandsPane.showLatest()
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
    // What the pending move is: kind is "branch" / "remote" / "force"
    // (a local branch moved to `moveStart` before landing on it). Every
    // one of them lands on a branch — nothing here moves onto a bare
    // commit (デザイン規約 §ブランチ・コミットへの移動).
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
        else if (page.moveKind === "force")
            repoTab.checkoutForceCreate(page.moveTarget, page.moveStart, carry)
    }

    // ---- what a chip leads to --------------------------------------
    // One dispatcher for every way of asking to move to a named ref: the
    // graph's chips, the list the stacked ones open into, and the
    // sidebar's menu. `record` is the chip as it is drawn — kind letter,
    // four flags, then the name (see encode.rs).
    function activateRecord(record) {
        if (record !== "")
            page.switchToRef(record[0], record.substring(5).split("\u001E")[0])
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
        // onto one can only leave HEAD on no branch, which nothing here
        // does. A tag's row offers a branch at that commit instead
        // (`startNaming`), which is what the move was after.
        if (kind !== "R")
            return
        const local = repoTab.localNameFor(name)
        // With no local branch of that name, landing on the remote one
        // means making it: one branch, tracking, nothing to lose, so
        // nothing to ask about.
        if (branchesModel.oidOfName(local) === "")
            page.switchTo("remote", name, local, "")
        else
            // Whether this is worth asking about is git's to answer: a
            // branch that has only fallen behind loses nothing by moving,
            // and the question's own words ("commits only X has") would be
            // describing something that does not exist. It comes back as
            // `moveAskSeq` when there really is something to lose.
            repoTab.checkoutMovingBranch(local, name)
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
            qsTr("Move"),
            function () { page.switchTo("force", local, local, remoteRef) },
            true)
        // Say it in words too: a smoke run asserts on the report line
        // without having to look at the shot.
        if (AppBackend.autoAct !== "")
            AppBackend.report("move_branch_asked local=" + local)
    }

    // ---- the standing question --------------------------------------
    // One bar, over the graph, with the row it is about marked (デザイン規約
    // §可否・警告の出し場所). The run waits here; Escape, another ask or a
    // click anywhere else walks away from it. A row outside the loaded
    // window simply goes unmarked, so nothing needs a dialog to fall back
    // to.
    //
    // Only questions about a ref reach it now. Everything that takes one
    // named thing away — a file's changes, a hunk, a stash, a branch on a
    // remote, `reset --hard` — is held down where the hand already is, on
    // the row or button that names it (デザイン規約 §長押し).
    property var rowAskRun: null
    function startRowAsk(oidHex, label, detail, danger, acceptText, run,
                         hold = false, tip = "") {
        page.rowAskRun = run
        graphPane.startAsking(oidHex, label, detail, acceptText, danger,
                              hold, tip)
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
    // The same shape for the branch move git says is worth asking about:
    // its own counter, because the same move can be asked about twice.
    property int seenMoveAskSeq: 0
    function absorbMoveAsk() {
        if (repoTab.moveAskSeq === page.seenMoveAskSeq)
            return
        page.seenMoveAskSeq = repoTab.moveAskSeq
        page.askMoveBranchOnto(repoTab.moveAskLocal, repoTab.moveAskStart)
    }

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
    /// The branch git has just refused to delete, if the menu that asked
    /// is still standing. Set only while its row is on screen to carry the
    /// answer; a fresh menu starts with nothing refused.
    property string forceDeleteBranch: ""
    function openRefMenu(kind, name, full, oidHex) {
        page.menuRefKind = kind
        page.menuRefName = name
        page.menuRefId = full
        page.menuRefOid = oidHex
        page.forceDeleteBranch = ""
        refMenu.popup()
    }
    AppMenu {
        id: refMenu
        // Walking away from a refused delete takes the offer with it.
        onClosed: page.forceDeleteBranch = ""
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
        // Nothing here repeats a gesture or a button: renaming and
        // creating a branch on a tag are a click away on the row itself,
        // and the commit's hash is on the pane the same click fills in.
        // Every row this menu keeps costs the ones that have nowhere else
        // to go (デザイン規約 §メニュー).
        AppMenuItem {
            id: refDeleteItem
            // A stash is held down here instead of raising a bar over the
            // graph: it is one row, kept nowhere else, and the question
            // has nothing to add that the words on the row do not already
            // say (デザイン規約 §長押し). The ellipsis goes with it — the
            // row no longer promises a question.
            //
            // A branch on a remote is held for the same reason, and the
            // chip is what tells the two apart: this one runs a command
            // that leaves this machine, and says so in git's own spelling
            // (デザイン規約 §git 用語のコード表記).
            readonly property bool stashRow: page.menuRefKind === "stash"
            readonly property bool remoteRow: page.menuRefKind === "remote"
            readonly property bool tagRow: page.menuRefKind === "tag"
            // A branch git already refused to delete. Until then a branch
            // is the one row here that is not held: `-d` takes nothing
            // away that git would not refuse over, so making the everyday
            // tidy-up cost a hold would spend the gesture where there is
            // nothing to lose (デザイン規約 §左メニューの所作).
            readonly property bool refusedRow:
                page.menuRefKind === "branch"
                && page.forceDeleteBranch === page.menuRefId
            readonly property bool heldRow: stashRow || remoteRow || tagRow
                                            || refusedRow
            code: remoteRow ? "push --delete" : ""
            // The verb and nothing else: the gesture is the mark ahead of
            // it, and the ellipsis is what tells a row that asks first
            // from one that is held (デザイン規約 §長押し).
            text: refusedRow ? qsTr("Delete anyway")
                : heldRow ? qsTr("Delete") : qsTr("Delete…")
            // What git said, in the row rather than on a bar: the branch
            // holds commits its reference point does not (§左メニューの所作).
            note: refusedRow ? qsTr("not merged") : ""
            visible: page.menuRefKind === "branch" || heldRow
            // The branch under the working tree cannot be deleted at all,
            // and git says so rather than doing something else.
            enabled: repoTab.busyCount === 0
                     && !(page.menuRefKind === "branch"
                          && page.menuRefId === workTree.branch)
            holdMs: heldRow ? Metrics.holdMs : 0
            // A branch's plain delete keeps the menu up: git's answer has
            // nowhere to land otherwise, and this row is where it lands.
            staysOpen: page.menuRefKind === "branch"
            // Reaching past this machine is the warning tone; throwing
            // away what is in hand is danger (デザイン規約 §状態).
            holdTone: remoteRow ? Theme.warning : Theme.danger
            onPicked: page.deleteRow(page.menuRefKind, page.menuRefId,
                                     page.menuRefName, page.menuRefOid)
            onHeld: {
                refMenu.close()
                if (stashRow)
                    page.dropStashNow(page.menuRefId)
                else if (remoteRow)
                    page.deleteRemoteNow(page.menuRefId)
                else if (tagRow)
                    repoTab.deleteTag(page.menuRefId)
                else
                    repoTab.deleteBranch(page.menuRefId, true)
            }
        }
    }

    // ---- what the sidebar's rows ask for ---------------------------
    /// A row was renamed in place. Nothing is confirmed: a name is not
    /// history, and the one it replaces is a switch away (a tag and a
    /// stash are re-made under the new name by core, which is the only
    /// rename git has for them).
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
            qsTr("Hold to rename. git has no rename on a remote: %1 is "
                 + "pushed, then %2 is deleted. Anything the old name "
                 + "carried — an open pull request, a running check — "
                 + "does not follow it.").arg(remote + "/" + name).arg(remoteRef))
        if (AppBackend.autoAct !== "")
            AppBackend.report("rename_remote_asked from=" + remoteRef + " to=" + name)
    }

    /// Deleting a row from the left menu.
    ///
    /// A branch goes the way git deletes one — `-d`, which refuses while
    /// the branch holds commits nothing else does. That refusal is the
    /// question worth asking, so it is asked when it arrives rather than
    /// guessed at beforehand (デザイン規約 §左メニューの所作). A tag and a stash have no
    /// such refusal in git, and both can take something with them, so they
    /// are asked about up front. A branch on a remote never arrives here:
    /// its row is held down instead (`deleteRemoteNow`).
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
        page.pendingDeleteBranch = id
        page.expectedRefusals++
        repoTab.deleteBranch(id, false)
    }
    /// A branch on a remote, deleted with no question in front of it: the
    /// menu row that reaches this was held down, which is the whole of the
    /// asking (デザイン規約 §リモートブランチを消す). git refuses nothing
    /// here — the branch is on the far side, so no `-d` can weigh what it
    /// holds — and the hold is what stands in for that refusal.
    function deleteRemoteNow(remoteRef) {
        const cut = remoteRef.indexOf("/")
        if (cut < 0)
            return
        repoTab.deleteRemoteBranch(remoteRef.substring(0, cut),
                                   remoteRef.substring(cut + 1))
    }
    /// A stash dropped with no question in front of it: the menu row that
    /// reaches this was held down, which is the whole of the asking
    /// (デザイン規約 §長押し).
    function dropStashNow(ref) {
        repoTab.dropStash(ref)
        if (page.selectedStashRef === ref)
            page.selectedStashRef = ""
    }
    /// git refused the plain delete. The row that asked is still standing
    /// — the click that ran it left the menu up for exactly this — so the
    /// answer lands there, on the row the hand is already on, and turns it
    /// into a held one (デザイン規約 §左メニューの所作).
    ///
    /// The row says what git said and no more. A refusal does not mean the
    /// commits stop being reachable: git measures the branch against its
    /// upstream when it has one, so a branch merged into HEAD but not yet
    /// pushed is refused while nothing at all would be lost (実測).
    function noteForceDelete(name) {
        page.forceDeleteBranch = name
        if (AppBackend.autoAct !== "")
            AppBackend.report("force_delete_offered branch=" + name)
    }

    // ---- context menu on a working-tree file row --------------------
    property string menuFilePath: ""
    property string menuFileBucket: ""
    /// Where a staged rename came from ("" for every other row).
    property string menuFileOrig: ""
    function openFileMenu(bucket, path, origPath) {
        // A right-click is a click: it walks away from a question that
        // was standing, which may well be about another row.
        page.stopRowAsk()
        page.menuFileBucket = bucket
        page.menuFilePath = path
        page.menuFileOrig = origPath === undefined ? "" : origPath
        // What the discard row would do, worked out once here: the choice
        // cannot change while the menu is up, so the words the row says
        // and the writes it runs are read off the same plan.
        page.discardPlan = page.planDiscard()
        fileMenu.popup()
        // Which rows the menu offers follows from the bucket, and a
        // greyed or absent row is not something a screenshot can be
        // trusted on.
        if (AppBackend.autoAct !== "")
            AppBackend.report("file_menu bucket=" + bucket)
    }
    /// Undoing what happened to one file: what the chosen rows would cost
    /// and which git command each of them goes to.
    ///
    /// One way in, because the reader's intent is the same whatever git
    /// knows about the file; the words apart, because what it costs is not
    /// (デザイン規約 §その他の操作). Which row it was opened on is the
    /// whole of the difference:
    ///
    /// - unstaged — the edits on disk go, and what is staged stays
    /// - untracked — the file goes; there the file *is* the change
    /// - staged — both sides go, back to HEAD, and a rename takes the
    ///   name it came from with it or leaves half of itself staged
    ///
    /// A conflicted path gets here from nowhere: git refuses to restore
    /// one until it has been told how it was resolved, so a conflicted
    /// row rides along with none of it and is not counted either.
    property var discardPlan: null
    function planDiscard() {
        // Whatever is highlighted, in list order. A right-click outside
        // the choice has already made its row the whole of it.
        const rows = wipPane.chosenRows()
        const plan = { count: 0, unstaged: [], untracked: [], staged: [] }
        for (let i = 0; i < rows.length; i++) {
            const row = rows[i]
            if (row.bucket === "conflicts")
                continue
            plan.count++
            const bag = row.bucket === "untracked" ? plan.untracked
                      : row.bucket === "staged" ? plan.staged : plan.unstaged
            bag.push(row.fullName)
            // A rename is undone by both of its names at once.
            if (row.bucket === "staged" && row.orig_path !== "")
                bag.push(row.orig_path)
        }
        return plan
    }
    /// What the row does, said the way every held row says it: the verb
    /// and nothing else, with the gesture left to the mark ahead of it
    /// (デザイン規約 §長押し). Nothing to take is nothing to say.
    function discardWords(plan) {
        return !plan || plan.count === 0 ? "" : qsTr("Discard")
    }
    /// What that costs, when it is more than the verb implies: one wording
    /// for every bucket and the difference on the tag, since the reader's
    /// intent is the same whatever git knows about the file
    /// (デザイン規約 §その他の操作).
    function discardNote(plan) {
        if (!plan || plan.count === 0)
            return ""
        if (plan.count > 1)
            return qsTr("%n files", "", plan.count)
        if (plan.untracked.length > 0)
            return qsTr("the file goes")
        if (plan.staged.length > 0)
            return qsTr("both sides")
        return ""
    }
    /// Thrown away with no question in front of it: the menu row that
    /// reaches this was held down, which is the whole of the asking
    /// (デザイン規約 §長押し).
    function discardChosenNow(plan) {
        if (!plan || plan.count === 0)
            return
        // One git command per bucket, however many rows were chosen: the
        // paths cross the bridge one at a time and the write takes the
        // whole set (デザイン規約 §その他の操作).
        if (plan.unstaged.length > 0) {
            page.sendPaths(plan.unstaged)
            repoTab.discardPaths()
        }
        if (plan.untracked.length > 0) {
            page.sendPaths(plan.untracked)
            repoTab.removeUntrackedPaths()
        }
        if (plan.staged.length > 0) {
            page.sendPaths(plan.staged)
            repoTab.discardPathsToHead()
        }
    }
    /// Hands a set of paths to the bridge for the write that follows.
    function sendPaths(paths) {
        repoTab.beginPaths()
        for (let i = 0; i < paths.length; i++)
            repoTab.addPath(paths[i])
    }
    /// The conflicted rows among those chosen — the only ones a side can
    /// be taken on. A right-click on a conflicted row has already made it
    /// the whole of the choice unless several were picked on purpose.
    function chosenConflicts() {
        const rows = wipPane.chosenRows()
        const paths = []
        for (let i = 0; i < rows.length; i++)
            if (rows[i].bucket === "conflicts")
                paths.push(rows[i].fullName)
        return paths
    }
    /// Takes one side of every conflicted row that is highlighted, in one
    /// git command (デザイン規約 §その他の操作).
    function takeSideNow(side) {
        const paths = page.chosenConflicts()
        if (paths.length === 0)
            return
        page.sendPaths(paths)
        repoTab.takeSidePaths(side)
    }
    AppMenu {
        id: fileMenu
        // Which side to keep, named by the branch each side is rather
        // than by `--ours` / `--theirs` — during a rebase those two swap
        // over, and a name that is simply what the side *is* does not ask
        // anyone to hold that in their head (デザイン規約 §conflict の
        // ours / theirs). Plain clicks: the file is conflicted, so there
        // is no settled version of it to lose, and the other side is one
        // click away until the operation is continued.
        AppMenuItem {
            text: wipPane.workTree.sideOurs !== ""
                  ? qsTr("Keep %1's version").arg(wipPane.workTree.sideOurs)
                  : qsTr("Keep this branch's version")
            visible: page.menuFileBucket === "conflicts"
            enabled: repoTab.busyCount === 0
            onTriggered: page.takeSideNow("ours")
        }
        AppMenuItem {
            text: wipPane.workTree.sideTheirs !== ""
                  ? qsTr("Take %1's version").arg(wipPane.workTree.sideTheirs)
                  : qsTr("Take the incoming version")
            visible: page.menuFileBucket === "conflicts"
            enabled: repoTab.busyCount === 0
            onTriggered: page.takeSideNow("theirs")
        }
        AppMenuSeparator { visible: page.menuFileBucket === "conflicts" }
        AppMenuItem {
            code: "stash"
            //: Follows the `stash` chip: "stash this file".
            text: wipPane.chosenCount > 1 ? qsTr("these files")
                                          : qsTr("this file")
            // git will not stash a tree with unresolved conflicts in it.
            enabled: repoTab.busyCount === 0
                     && page.menuFileBucket !== "conflicts"
            onTriggered: {
                const rows = wipPane.chosenRows()
                const paths = []
                for (let i = 0; i < rows.length; i++)
                    paths.push(rows[i].fullName)
                page.sendPaths(paths)
                repoTab.stashPaths("")
            }
        }
        // One row on every kind of file, and the row says what it takes
        // away — held rather than asked about, the way the stash and the
        // remote branch above it are (デザイン規約 §長押し). On a file
        // changed on both sides the two rows are the choice itself: the
        // unstaged one keeps what is staged, the staged one takes the lot.
        AppMenuItem {
            id: fileDiscardItem
            text: page.discardWords(page.discardPlan)
            note: page.discardNote(page.discardPlan)
            visible: page.menuFileBucket !== "conflicts"
            enabled: repoTab.busyCount === 0
            holdMs: Metrics.holdMs
            onHeld: {
                fileMenu.close()
                page.discardChosenNow(page.discardPlan)
            }
        }
        AppMenuSeparator {}
        AppMenuItem {
            text: wipPane.chosenCount > 1 ? qsTr("Copy paths") : qsTr("Copy path")
            onTriggered: {
                const rows = wipPane.chosenRows()
                const paths = []
                for (let i = 0; i < rows.length; i++)
                    paths.push(rows[i].fullName)
                clipboard.copy(paths.join("\n"))
            }
        }
    }

    // ---- context menu on a graph row -------------------------------
    property string menuOid: ""
    // The stash the menu was opened on, by the selector git answers to
    // ("" on an ordinary commit). A stash is a commit git keeps off to
    // one side of every branch, so none of the commit menu's rows land on
    // it and it gets its own (デザイン規約 §グラフ行の右クリック).
    property string menuStashRef: ""
    // Whether a remote already has the menu's commit. Rewriting it is
    // not asked about — nothing here leaves the machine — but 要望.md
    // wants it said, so the squash row carries a tag the way the amend
    // editor does. The answer lands a frame after the menu opens.
    property bool menuPublished: false
    function openRowMenu(oidHex) {
        page.menuOid = oidHex
        page.menuStashRef = graphModel.stashRefOf(oidHex)
        if (page.menuStashRef !== "") {
            stashMenu.popup()
            return
        }
        page.menuPublished = false
        if (repoTab.state === "open")
            repoTab.checkPublish(oidHex + "^!")
        commitMenu.popup()
    }

    // The one thing a stash row has nowhere else: the click that opens
    // this menu also selects the row, and the details pane it fills in
    // carries Apply and Pop (デザイン規約 §メニュー).
    AppMenu {
        id: stashMenu
        AppMenuItem {
            text: qsTr("Apply")
            enabled: repoTab.busyCount === 0
            onTriggered: repoTab.applyStash(page.menuStashRef)
        }
        AppMenuItem {
            text: qsTr("Pop")
            enabled: repoTab.busyCount === 0
            onTriggered: {
                repoTab.popStash(page.menuStashRef)
                page.selectedStashRef = ""
            }
        }
        AppMenuSeparator {}
        // Held, not asked about: a bar coming down over the graph to ask
        // about one row of it is more machinery than one stash is worth,
        // and the hold says the same thing in the place the hand already
        // is (デザイン規約 §長押し).
        AppMenuItem {
            id: stashDeleteItem
            text: qsTr("Delete")
            enabled: repoTab.busyCount === 0
            holdMs: Metrics.holdMs
            onHeld: {
                stashMenu.close()
                page.dropStashNow(page.menuStashRef)
            }
        }
    }

    AppMenu {
        id: commitMenu
        AppMenuItem {
            code: "cherry-pick"
            //: Follows the `cherry-pick` chip: "cherry-pick this commit".
            text: qsTr("this commit")
            enabled: repoTab.busyCount === 0
            onTriggered: repoTab.cherryPick(page.menuOid)
        }
        // No row for landing on the commit itself: doing so leaves HEAD
        // on no branch, which is a state to be got out of rather than one
        // to offer (デザイン規約 §ブランチ・コミットへの移動). What that
        // row was reached for is a branch at this commit, which the
        // double-click already opens the box for.
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
        // by what becomes of the work they skip over. The command is
        // said once, as the title row's own verb (`reset` main here, the
        // way `stash` this file reads), and each row leads with just its
        // flag as a code chip (デザイン規約 §git 用語のコード表記) — the
        // hand that knows `reset --soft` finds its row at a glance and
        // the eye that does not reads the sentence alone. A submenu
        // keeps the choice out of the way until it is asked for;
        // disabling the submenu itself greys the row that opens it (its
        // items are never reachable while it is off).
        AppMenu {
            id: resetMenu
            titleCode: "reset"
            // "here" rather than "to this commit": the commit it means
            // is the row this menu was opened on, and the row already
            // says which one that is.
            //: Follows the `reset` chip: "reset main here".
            title: workTree.branch !== ""
                   ? qsTr("%1 here").arg(workTree.branch)
                   : qsTr("the branch here")
            enabled: page.canMoveBranchHere
            AppMenuItem {
                code: "--soft"
                text: qsTr("Keep everything, staged")
                onTriggered: page.moveBranchHere("soft")
            }
            AppMenuItem {
                code: "--mixed"
                text: qsTr("Keep everything, unstaged")
                onTriggered: page.moveBranchHere("mixed")
            }
            // Held, not asked about — the stash delete's judgement: a
            // bar coming down over the whole graph is too much machinery
            // for one row of it, and the hold says the same thing in the
            // place the hand already is (デザイン規約 §長押し).
            AppMenuItem {
                id: hardResetItem
                code: "--hard"
                text: qsTr("Discard everything after it")
                holdMs: Metrics.holdMs
                onHeld: {
                    resetMenu.close()
                    commitMenu.close()
                    page.moveBranchHere("hard")
                }
            }
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
        && page.menuStashRef === ""

    // Only the discarding one is held, and it is held on its own row —
    // no bar. Keeping the work staged or unstaged leaves every byte
    // where it is — the branch moves, and a commit puts back what it
    // skipped — while discarding is the one that leaves nothing to put
    // back.
    function moveBranchHere(mode) {
        repoTab.resetTo(page.menuOid, mode)
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
    // The diff has to arrive before a row of it can be staged — or
    // thrown away, which stops at the question the pill answers.
    Timer {
        id: stageRowTimer
        interval: 800
        onTriggered: {
            const act = AppBackend.autoAct
            // Nothing to do but be looked at: the diff is the shot.
            if (act === "diff-file")
                return
            // Which line the line-level verbs mean. Not 0: a hunk numbers
            // its lines through the context it carries, and the context is
            // not part of the change (see `firstChangedLine`).
            const line = diffPane.firstChangedLine(0)
            // The squares a line only puts out under the pointer, named
            // rather than hovered (hover cannot be injected on Windows).
            if (act === "line-tools") {
                diffPane.showLineTools(0, line)
                return
            }
            if (act === "stage-hunk" || act === "stage-line") {
                page.stageSelection(0, act === "stage-line" ? line : -1)
                return
            }
            // "discard-hunk" leaves the held button on screen for the shot;
            // "-go" holds it to its end. There is no line-level discard to
            // enter — a hunk is the smallest piece that can be thrown away.
            if (act === "discard-hunk-go")
                diffPane.completeHold()
        }
    }
    // git's refusal has to come back before the row it turns into a held
    // one can be held — or photographed.
    Timer {
        id: forceDeleteTimer
        interval: 800
        onTriggered: {
            AppBackend.report("ref_menu delete=" + refDeleteItem.text
                              + " note=" + refDeleteItem.note)
            refDeleteItem.completeHold()
        }
    }
    // The splitter has to have handed the pane its new width before the
    // width can be reported — the fold sets it, the layout takes it.
    Timer {
        id: navRailTimer
        interval: 400
        onTriggered: AppBackend.report(
            "nav_rail collapsed=" + page.sidebarCollapsed
            + " width=" + Math.round(sidebarPane.width)
            + " peek=" + sidebarPane.peekKind
            + " editing=" + sidebarPane.editKey)
    }
    Timer {
        id: refusedRowTimer
        interval: 800
        onTriggered: AppBackend.report("ref_menu delete=" + refDeleteItem.text
                                       + " note=" + refDeleteItem.note)
    }
    // The fetch has to land, and its answer reach the chips, before the
    // stacked ones are worth unstacking.
    Timer {
        id: fetchedRefListTimer
        interval: 1500
        onTriggered: {
            const stacked = graphPane.view.itemAtIndex(
                Number(AppBackend.autoActArg))
            if (stacked)
                graphPane.view.chipExpandRequested(
                    stacked.chipItem.records, stacked.chipItem)
        }
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
    // gpg / ssh-keygen have to finish before the mark they decide can be
    // on screen, so the shot and the report both wait for them.
    Timer {
        id: signatureTimer
        interval: 800
        onTriggered: AppBackend.report(
            "signature kind=" + page.selectedSignatureKind
            + " code=" + page.selectedSignatureCode
            + " signer=" + page.selectedSignatureSigner)
    }
    /// Automation: how long a run of failed fetches the verb asked for,
    /// and whether to hold the button that resumes once it is there.
    property int fetchFailRuns: 0
    property bool fetchResumeAfter: false
    /// The count this already answered. `changed` fires on every message
    /// the tab drains, and without this every one of them would queue
    /// another fetch behind the one still running.
    property int fetchFailSeen: -1
    function runFetchFailures() {
        if (page.fetchFailRuns <= 0 || repoTab.fetchFailures === page.fetchFailSeen)
            return
        page.fetchFailSeen = repoTab.fetchFailures
        if (repoTab.fetchFailures < page.fetchFailRuns) {
            repoTab.fetch("")
            return
        }
        page.fetchFailRuns = 0
        if (page.fetchResumeAfter) {
            page.fetchResumeAfter = false
            repoTab.resumeAutoFetch()
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
            wipPane.chooseOnly("unstaged", arg)
            page.openFileMenu("unstaged", arg, "")
            page.sendPaths([arg])
            repoTab.stashPaths("")
        } else if (act === "discard-many" || act === "discard-many-go") {
            // Two rows chosen the way clicks choose them — the first row
            // plainly, the argument's row with Ctrl — and then the menu's
            // one row over both.
            page.showWip()
            const first = wipPane.rowAt(0)
            if (first)
                wipPane.chooseOnly(first.bucket, first.fullName)
            const other = wipPane.rowFor(arg)
            if (other)
                wipPane.applyClick(other.bucket, other.fullName, Qt.ControlModifier)
            AppBackend.report("chosen count=" + wipPane.chosenCount)
            page.openFileMenu(other ? other.bucket : "unstaged", arg, "")
            AppBackend.report("discard_row " + fileDiscardItem.text)
            if (act.endsWith("-go"))
                fileDiscardItem.completeHold()
        } else if (act === "stash-dialog") {
            // Opened and left standing, for a look at it. With the
            // argument "staged-only" the staged-only box is clicked
            // first, so the shot shows what that click clears.
            page.showWip()
            wipPane.openStashPanel()
            if (arg === "staged-only")
                wipPane.stashClickStagedOnly()
        } else if (act === "file-menu" || act === "file-menu-untracked"
                   || act === "file-menu-staged" || act === "file-menu-conflict") {
            // Which rows a file row offers follows from its bucket, so
            // each bucket has its own way in here. The row is chosen
            // first, the way a right-click on an unchosen row chooses it
            // — the menu acts on what is highlighted, and its discard row
            // says what that choice costs.
            const menuBucket = act === "file-menu" ? "unstaged"
                             : act === "file-menu-staged" ? "staged"
                             : act === "file-menu-conflict" ? "conflicts"
                             : "untracked"
            page.showWip()
            wipPane.chooseOnly(menuBucket, arg)
            page.openFileMenu(menuBucket, arg, "")
            if (menuBucket === "conflicts") {
                // The two sides are named after branches that swap over
                // during a rebase, so a shot has to be able to say which
                // words the rows actually got.
                const row = wipPane.rowFor(arg)
                AppBackend.report("conflict_kind " + (row ? row.conflictWords() : "-"))
            } else {
                AppBackend.report("discard_row " + fileDiscardItem.text)
            }
        } else if (act === "take-side-ours" || act === "take-side-theirs") {
            // Through the same menu a right-click opens, on the row that
            // is highlighted — the write goes to every conflicted row in
            // the choice, not just the one named here.
            page.showWip()
            wipPane.chooseOnly("conflicts", arg)
            page.openFileMenu("conflicts", arg, "")
            fileMenu.close()
            page.takeSideNow(act === "take-side-ours" ? "ours" : "theirs")
        } else if (act === "discard-file" || act === "discard-file-go"
                   || act === "delete-file" || act === "delete-file-go"
                   || act === "discard-staged" || act === "discard-staged-go") {
            // Through the file menu, where a right-click enters it. The
            // one row says what it takes, which follows the row it was
            // opened on: "delete-file" an untracked one, "discard-staged"
            // the staged side, otherwise the unstaged one. That row is
            // held rather than asked about, so there is no question to
            // stop at: the plain verb leaves the menu standing for the
            // shot, and "-go" runs the hold to its end. What the row says
            // goes into words as well — the wording is the whole of the
            // difference between the three, and a shot alone proves little.
            page.showWip()
            const bucket = act.startsWith("delete-file") ? "untracked"
                         : act.startsWith("discard-staged") ? "staged"
                         : "unstaged"
            // The row carries where a rename came from, as it does for a
            // right-click, and the right-click makes it the whole choice.
            const row = wipPane.rowFor(arg)
            wipPane.chooseOnly(bucket, arg)
            page.openFileMenu(bucket, arg, row ? row.orig_path : "")
            AppBackend.report("discard_row " + fileDiscardItem.text)
            if (act.endsWith("-go"))
                fileDiscardItem.completeHold()
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
        } else if (act === "nav-fold" || act === "nav-peek"
                   || act === "nav-unfold" || act === "nav-peek-rename") {
            // The left menu folded to its icons, and one of them rested
            // on. The resting cannot be injected (hover never can), so
            // the section is named the way the diff's line tools are.
            // "nav-unfold" walks the whole way back, which is the one
            // thing folding has to be able to do.
            // "nav-fold no-tags" takes the tags off the graph first, so
            // the mark the rail wears for that can be photographed.
            if (arg === "no-tags")
                repoTab.setTagsShown(false)
            page.foldByHand(true)
            if (act === "nav-peek")
                sidebarPane.peekAt(arg)
            else if (act === "nav-unfold")
                page.foldByHand(false)
            else if (act === "nav-peek-rename") {
                // Typing a name into a peeked row: the list has to come
                // back on its own and the box land on the same row in it
                // with the keyboard (SidebarPane.startEdit).
                sidebarPane.peekAt("branch")
                sidebarPane.beginRename("branch", workTree.branch,
                                        workTree.branch)
            }
            navRailTimer.start()
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
        } else if (act === "rename-remote" || act === "rename-remote-box"
                   || act === "rename-remote-go") {
            // A remote branch renamed from its own row, named outright
            // (`origin/billing:billing-v2`) because the remote's rows are
            // behind a fold. The box carries the branch without the
            // remote it is on; "-box" leaves it standing for the shot,
            // the plain act stops at the question, and "-go" holds the
            // pill down to the end.
            const parts = arg.split(":")
            const ref = parts[0]
            const was = ref.substring(ref.indexOf("/") + 1)
            // "-box" opens with the argument already in it, so a name the
            // remote already carries can be photographed being refused —
            // and the remote's fold has to come open for the row to be
            // there at all (a remote root starts closed).
            if (act === "rename-remote-box")
                remotesModel.toggleFolder(ref.substring(0, ref.indexOf("/")))
            sidebarPane.beginRename("remote", ref,
                                    act === "rename-remote-box" ? parts[1] : was)
            if (act === "rename-remote-box")
                return
            sidebarPane.submitEdit(parts[1])
            if (act === "rename-remote-go")
                graphPane.completeHold()
        } else if (act === "rename-local-upstream") {
            // The whole of the local flow: the branch takes the new name
            // here, and the question about carrying it over comes back
            // when git says that landed (so the shot is taken later).
            const local = workTree.branch
            sidebarPane.beginRename("branch", local, local)
            sidebarPane.submitEdit(arg)
        } else if (act === "delete-branch" || act === "delete-branch-go") {
            // Through the menu's own row, which stays standing over the
            // plain `-d` so git's answer has somewhere to land. On a merged
            // branch it lands and the menu closes; on one git refuses, the
            // row turns into the held `Delete anyway`, which "-go"
            // then runs to its end. What the row says goes into words too,
            // since that is the whole of the difference.
            page.openRefMenu("branch", arg, arg, branchesModel.oidOfName(arg))
            page.deleteRow("branch", arg, arg, branchesModel.oidOfName(arg))
            if (act === "delete-branch-go")
                forceDeleteTimer.start()
        } else if (act === "delete-tag" || act === "delete-tag-go") {
            // The tag's row is held rather than asked about: git refuses
            // nothing here, so the hold is the whole of the asking.
            page.openRefMenu("tag", arg, arg, tagsModel.oidOfName(arg))
            if (act === "delete-tag-go")
                refDeleteItem.completeHold()
        } else if (act === "delete-stash" || act === "delete-stash-go") {
            // The left menu's row for the first stash. That row is held
            // rather than asked about, so there is no question to stop at:
            // the plain verb leaves the menu standing for the shot, and
            // "-go" runs the hold to its end.
            page.openRefMenu("stash", stashesModel.nameAt(0),
                             stashesModel.fullAt(0),
                             stashesModel.oidOfName(stashesModel.nameAt(0)))
            if (act === "delete-stash-go")
                refDeleteItem.completeHold()
        } else if (act === "delete-remote" || act === "delete-remote-go") {
            // The left menu's row for a branch on a remote, named outright
            // (`origin/feature/x`) because those rows sit behind a fold —
            // which is opened here so the row is under the menu it raises.
            // That row is held rather than asked about, so there is no
            // question to stop at: the plain verb leaves the menu standing
            // for the shot, and "-go" runs the hold to its end. Which rows
            // it offers is said in words too, since a greyed or absent row
            // is not something a screenshot can be trusted on.
            remotesModel.toggleFolder(arg.substring(0, arg.indexOf("/")))
            page.openRefMenu("remote", arg, arg, remotesModel.oidOfName(arg))
            AppBackend.report("ref_menu kind=remote delete=" + refDeleteItem.code
                              + " " + refDeleteItem.text)
            if (act === "delete-remote-go")
                refDeleteItem.completeHold()
        } else if (act === "delete-force") {
            repoTab.deleteBranch(arg, true)
        } else if (act === "delete-branch-refused") {
            // The refused branch's row, left standing for the shot: the
            // plain delete runs and the refusal turns the row into a held
            // one. Same entry as delete-branch; this one just waits for
            // git's answer rather than acting on it.
            page.openRefMenu("branch", arg, arg, branchesModel.oidOfName(arg))
            page.deleteRow("branch", arg, arg, branchesModel.oidOfName(arg))
            refusedRowTimer.start()
        } else if (act === "stash-menu" || act === "delete-stash-row") {
            // The graph's way to the same three rows, entered where a
            // right-click enters it: the row menu on the first stash's
            // row. Which menu opened is said in words too — the difference
            // is the whole point, and a shot alone proves little. The
            // argument "go" holds the delete row down.
            page.openRowMenu(stashesModel.oidOfName(stashesModel.nameAt(0)))
            AppBackend.report("row_menu stash=" + page.menuStashRef)
            if (act === "delete-stash-row" && arg === "go")
                stashDeleteItem.completeHold()
        } else if (act === "stash-apply-row" || act === "stash-pop-row") {
            // Apply and Pop from the graph's own stash row.
            page.openRowMenu(stashesModel.oidOfName(stashesModel.nameAt(0)))
            if (act === "stash-apply-row")
                repoTab.applyStash(page.menuStashRef)
            else
                repoTab.popStash(page.menuStashRef)
        } else if (act === "branch-at-tag") {
            // The box a double-click puts on a tag row, accepted with the
            // argument as the new branch's name.
            sidebarPane.beginBranchAt(tagsModel.nameAt(0),
                                      tagsModel.oidOfName(tagsModel.nameAt(0)))
            sidebarPane.submitEdit(arg)
        } else if (act === "dbl-local" || act === "dbl-remote") {
            // What a double-click on a chip does, entered where the
            // delegate enters it: the record is the chip as it is drawn
            // (kind letter, four flags, name — see encode.rs). The last
            // flag is where the ref is, and it is the one thing the two
            // differ on here.
            page.activateRecord(
                (act === "dbl-local" ? "L0001" : "R0000") + arg)
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
        } else if (act === "signature") {
            // Selecting a row is all the operating there is; the mark
            // appears when the verify comes back, so the report waits
            // for it. The argument is the row.
            page.activateRow(graphModel.oidAt(Number(arg)))
            signatureTimer.start()
        } else if (act === "name-box") {
            // Opened and left standing, for a look at it. The argument is
            // the row, since the box only belongs on one with no chips.
            graphPane.startNaming(graphModel.oidAt(Number(arg)))
        } else if (act === "squash") {
            page.openRowMenu(branchesModel.headOid)
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
        } else if (act === "reset-soft" || act === "reset-mixed") {
            // Through the menu, like clicking it: the row the menu was
            // opened on is where the branch lands.
            page.openRowMenu(arg)
            page.moveBranchHere(act === "reset-soft" ? "soft" : "mixed")
        } else if (act === "reset-hard" || act === "reset-hard-confirm") {
            // Both stand where the click path stands: menu open, submenu
            // up, the held row on screen. "-confirm" stops there —
            // nothing has moved until the hold runs — and "reset-hard"
            // runs the hold to its end for the discarding write itself.
            page.openRowMenu(arg)
            resetMenu.popup()
            if (act === "reset-hard")
                hardResetItem.completeHold()
        } else if (act === "commit-menu" || act === "reset-menu") {
            // Nothing written: the menu is left standing for the overlay
            // shot. Which rows are offered is said in words as well — a
            // greyed row is not something a screenshot can be trusted on.
            page.openRowMenu(arg)
            if (act === "reset-menu")
                resetMenu.popup()
            AppBackend.report("commit_menu can_move=" + page.canMoveBranchHere)
        } else if (act === "wip") {
            // The working tree, as the row above the newest commit opens
            // it: the file list this pane's every other verb starts from.
            page.showWip()
        } else if (act === "op-exit" || act === "op-exit-go") {
            // The ways out of a stopped operation, which stand in the pane
            // under the commit button rather than dropping from a click.
            // "-go" runs the held row the argument names to its end.
            page.showWip()
            if (act === "op-exit-go")
                AppBackend.report("op_exit_held " + wipPane.completeOpExit(arg))
        } else if (act === "stage-hunk" || act === "stage-line"
                   || act === "discard-hunk" || act === "discard-hunk-go"
                   || act === "diff-file" || act === "line-tools") {
            // All of them enter through the diff of one unstaged file and
            // act on its first hunk: "diff-file" only opens it, and
            // "line-tools" puts out the square a line shows under the
            // pointer. "discard-hunk" leaves the held button standing for
            // the shot and "-go" holds it to its end. The working tree
            // comes up first, since its file list is where a diff is
            // reached from.
            page.showWip()
            page.toggleDiff("unstaged", arg, "")
            stageRowTimer.start()
        } else if (act === "diff-fold" || act === "diff-unfold"
                   || act === "diff-fold-by-hand"
                   || act === "diff-keep-folded") {
            // What opening a file does to the left menu on its own, and
            // what closing it puts back — which is what the diff did and
            // nothing else. "-by-hand" opens the list again while the
            // diff is still up, and "-keep-folded" had it folded before
            // the diff arrived: in both, the hand's answer is the one
            // that survives closing it.
            page.showWip()
            if (act === "diff-keep-folded")
                page.foldByHand(true)
            page.toggleDiff("unstaged", arg, "")
            if (act === "diff-fold-by-hand")
                page.foldByHand(false)
            if (act !== "diff-fold")
                page.closeDiff()
            navRailTimer.start()
        } else if (act === "push") {
            page.pushNow()
        } else if (act === "force-push") {
            page.forcePush()
        } else if (act === "fetch") {
            repoTab.fetch("")
        } else if (act === "fetch-ref-list") {
            // What a tag says about the remote only exists after a fetch
            // (`ls-remote --tags` is what carries it), and the names it
            // changes are inside the stacked chips — so the two steps are
            // one verb. The wait is the fetch's; a `file://` remote in a
            // demo repository answers in a fraction of it.
            repoTab.fetch("")
            fetchedRefListTimer.start()
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
        } else if (act === "fetch-fail") {
            // Against a remote that is not there, every fetch comes back
            // non-zero. The argument is how many to run, so one verb
            // reaches the warning shape and the stopped one alike.
            page.fetchFailRuns = Math.max(1, Number(arg))
            AppBackend.setAutoFetchMinutes(0)
            AppBackend.setAutoFetchMinutes(5)
            repoTab.fetch("")
        } else if (act === "fetch-resume") {
            // Stopped, then the hold that starts it again.
            page.fetchFailRuns = 3
            page.fetchResumeAfter = true
            AppBackend.setAutoFetchMinutes(0)
            AppBackend.setAutoFetchMinutes(5)
            repoTab.fetch("")
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
                page.noteForceDelete(refused)
            }
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
            refMenu.close()
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
        if (repoTab.lastWriteOp === "stage" || repoTab.lastWriteOp === "unstage"
                || repoTab.lastWriteOp === "discard")
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
    onDiffShownChanged: page.foldForDiff(page.diffShown)
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
        // The shown diff's fingerprint rides along: the write refuses to
        // apply the indices to bytes that drifted since this was read.
        repoTab.stageSelection(page.diffKind, page.diffPath, page.diffOrigPath,
                               hunk, line, diffModel.fingerprint)
        page.pendingDiffReload = true
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
                    collapsed: page.sidebarCollapsed
                    // The menus the rows raise are the page's, so only the
                    // page can say one is standing over the folded list.
                    menuOpen: refMenu.visible
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
                    SplitView.minimumWidth: 420
                    currentIndex: page.diffShown ? 1 : 0

                    GraphPane {
                        id: graphPane
                        graphModel: graphModel
                        worktreeModel: worktreeModel
                        blank: page.blank
                        onRowActivated: oidHex => page.activateRow(oidHex)
                        onRowMenuOpenRequested: oidHex => page.openRowMenu(oidHex)
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
                        id: diffPane
                        diffModel: diffModel
                        fromWorkTree: page.diffFromWt
                        staged: page.diffStaged
                        busy: repoTab.busyCount > 0
                        onCloseRequested: page.closeDiff()
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
            page.askSignature(oidHex)
            page.closeDiff()
        })
    }
}
