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
            if (page.menuOid !== ""
                    && repoTab.publishRange === page.menuOid + "^!")
                page.menuPublished = repoTab.publishPublished > 0
            if (page.selectedOid !== ""
                    && repoTab.publishRange === page.selectedOid + "^!")
                page.selectedPublished = repoTab.publishPublished > 0
            if (page.rebaseRange !== ""
                    && repoTab.publishRange === page.rebaseRange)
                page.rebasePublished = repoTab.publishPublished > 0
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
    property string moveLabel: ""
    property string moveStart: ""

    function switchTo(kind, target, label, start) {
        page.moveKind = kind
        page.moveTarget = target
        page.moveLabel = label
        page.moveStart = start === undefined ? "" : start
        page.runSwitch()
    }
    function runSwitch() {
        if (page.moveKind === "branch")
            repoTab.checkoutBranch(page.moveTarget)
        else if (page.moveKind === "remote")
            repoTab.checkoutRemote(page.moveTarget,
                                   repoTab.localNameFor(page.moveTarget))
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
        const local = repoTab.localNameFor(name)
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
        page.publishAsking = false
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

    // ---- push ------------------------------------------------------
    readonly property string pushTargetLabel:
        workTree.upstream !== "" ? workTree.upstream
                                 : repoTab.defaultRemote + "/" + workTree.branch
    /// What the branch can do with its remote, worked out before anything
    /// is sent (デザイン規約 §リモートへ送る):
    ///
    /// - `closed`   — there is no branch here to send
    /// - `publish`  — the branch has never been sent, so where it goes is
    ///                a question rather than something to look up. A
    ///                repository with no remote at all lands here too: the
    ///                question can make one
    /// - `ready`    — commits of ours to add, and nothing in the way
    /// - `clean`    — the remote already has them all
    /// - `behind`   — the remote moved on; we have nothing to add
    /// - `diverged` — both moved; only an overwrite can land
    ///
    /// The counts behind this come from the last fetch, so they prove the
    /// negative only: a push may still be refused when they say it fits.
    readonly property string pushState:
        repoTab.state !== "open" || workTree.detached
        || workTree.branch === "" ? "closed"
        : workTree.upstream === "" || !workTree.upstreamTracked ? "publish"
        : workTree.behind > 0 ? (workTree.ahead > 0 ? "diverged" : "behind")
        : workTree.ahead > 0 ? "ready" : "clean"
    readonly property bool canPush: (pushState === "publish"
                                     || pushState === "ready")
                                    && repoTab.busyCount === 0
    readonly property bool canForcePush: (pushState === "ready"
                                          || pushState === "diverged")
                                         && repoTab.busyCount === 0
    /// The branch a push of this button's is out for, from the send until
    /// the answer. What comes back names the operation and not what it was
    /// about, and `push` is also what a remote branch's rename and delete
    /// report as — so this says both which branch the answer belongs to and
    /// whether it belongs to this button at all.
    property string pushSentBranch: ""
    /// The branch whose last push git turned down, and what it said.
    /// Remembered per branch: most refusals are about that branch's
    /// standing with its remote and say nothing about the one beside it
    /// (デザイン規約 §リモートへ送る).
    property string pushFailBranch: ""
    property string pushFailReason: ""
    readonly property bool pushFailed:
        page.pushFailBranch !== "" && page.pushFailBranch === workTree.branch
    function pushNow() {
        if (page.pushState === "publish") {
            page.startPublishAsk()
            return
        }
        page.pushSentBranch = workTree.branch
        repoTab.pushCurrent("", "")
    }
    /// The lease is pinned to the commit this window has on screen rather
    /// than left to compare against the tracking ref: a background fetch
    /// must not turn this into a plain force. A remote that moved since is
    /// refused, and the refusal is answered by a fetch (core).
    function forcePush() {
        page.pushSentBranch = workTree.branch
        repoTab.pushCurrent("lease", page.upstreamOid())
    }
    /// Commit the remote-tracking branch points at, as shown here.
    function upstreamOid() {
        return workTree.upstream !== ""
               ? remotesModel.oidOfName(workTree.upstream) : ""
    }

    // ---- the first push: where does this branch go? -----------------
    // (デザイン規約 §はじめてリモートへ送る). Three things raise it —
    // this button, a graph row, a REMOTES row — all through the one bar.
    property bool publishAsking: false
    /// The remote the answer picks.
    property string publishRemote: ""
    /// What the branch is to be called over there — its own name unless
    /// the answer says otherwise.
    property string publishBranch: ""

    readonly property var publishRemotes: {
        const packed = repoTab.remoteNames
        return packed === "" ? [] : packed.split(String.fromCharCode(31))
    }
    /// The chooser's last row: opens the add-remote dialog. The question
    /// stays standing behind it and picks the new remote up when it lands.
    readonly property string publishAddChoice: qsTr("Add remote…")
    readonly property var publishChoices:
        page.publishRemotes.concat([page.publishAddChoice])
    function choosePublishRemote(index) {
        if (index >= page.publishRemotes.length) {
            remoteDialog.start("", "", page.publishRemotes)
            return
        }
        page.publishRemote = page.publishRemotes[index]
        page.refreshPublishCheck()
    }

    readonly property string publishTarget:
        page.publishRemote + "/" + page.publishBranch
    readonly property bool publishFilled:
        page.publishBranch !== "" && page.publishRemote !== ""
    /// The remote has answered for exactly what is typed now.
    readonly property bool publishChecked:
        repoTab.remoteBranchAsked === page.publishRemote
        + String.fromCharCode(31) + page.publishBranch
    /// …and what that answer says a push under this name would meet
    /// (`platitude_core::remote::RemoteBranchState`). Empty until the
    /// answer for exactly this name is in.
    readonly property string publishState:
        page.publishChecked ? repoTab.remoteBranchState : ""
    /// The far side cannot be settled from this end. Sending stays a
    /// click — a plain push can only fast-forward, so nothing over there
    /// can be lost by pressing — but the bar wears the frame and the mark
    /// (デザイン規約 §リモートへ送る).
    readonly property bool publishUnsure:
        page.publishState === "unknown" || page.publishState === "unreachable"
    /// The name is taken by commits this history does not have: a plain
    /// push cannot land at all (実測), only an overwrite can, so the pill
    /// becomes the diverged branch's `push -f` — held, warning-coloured
    /// (デザイン規約 §リモートへ送る).
    readonly property bool publishRefused: page.publishState === "refused"
    /// The commit the question showed on the far side, which is what an
    /// overwrite leases against: a remote that moved since is refused by
    /// git rather than flattened (§相手の履歴を置き換える).
    readonly property string publishLease:
        page.publishRefused ? repoTab.remoteBranchTip : ""

    /// Asked once the typing settles, not per keystroke: the check is a
    /// network round trip.
    Timer {
        id: publishCheckTimer
        interval: 350
        onTriggered: {
            if (page.publishAsking && page.publishRemote !== ""
                    && page.publishBranch !== "")
                repoTab.checkRemoteBranch(page.publishRemote, page.publishBranch)
        }
    }
    function refreshPublishCheck() {
        publishCheckTimer.restart()
    }

    // The bar's own state follows what has been typed into it, which is
    // why these are bindings rather than arguments: the question changes
    // what it is asking while it stands.
    Binding {
        target: graphPane
        property: "askAnswerable"
        value: page.publishFilled && page.publishChecked
        when: page.publishAsking
    }
    Binding {
        target: graphPane
        property: "askHold"
        value: page.publishRefused
        when: page.publishAsking
    }
    Binding {
        target: graphPane
        property: "askNeutral"
        value: !page.publishRefused
        when: page.publishAsking
    }
    Binding {
        target: graphPane
        property: "askCode"
        value: page.publishRefused ? "push -f" : "push"
        when: page.publishAsking
    }
    Binding {
        target: graphPane
        property: "askAlert"
        value: page.publishUnsure
        when: page.publishAsking
    }
    Binding {
        target: graphPane
        property: "askDetail"
        value: !page.publishFilled ? qsTr("Pick where it goes.")
               : !page.publishChecked
                 ? qsTr("Asking %1 what it has…").arg(page.publishRemote)
                 : page.publishState === "unreachable"
                   ? qsTr("%1 did not answer — it may already have that branch.")
                     .arg(page.publishRemote)
                   : page.publishState === "unknown"
                     ? qsTr("%1 already exists, and what it holds is not here.")
                       .arg(page.publishTarget)
                     : page.publishRefused
                       ? qsTr("Commits only %1 has stop being on it.")
                         .arg(page.publishTarget)
                       : page.publishState === "fast-forward"
                         ? qsTr("%1 already exists — your commits go on top of it.")
                           .arg(page.publishTarget)
                         : qsTr("%1 does not exist yet; this makes it.")
                           .arg(page.publishTarget)
        when: page.publishAsking
    }
    Binding {
        target: graphPane
        property: "askTip"
        value: page.publishRefused
               ? qsTr("Hold to overwrite %1, dropping %n commit(s) it has and yours does not. A remote that moved since is refused.",
                      "", repoTab.remoteBranchTheirs).arg(page.publishTarget)
               : page.publishState === "unknown"
                 ? qsTr("%1 was never fetched here, so what it holds cannot be read — the push can only fast-forward it.")
                   .arg(page.publishTarget)
                 : page.publishState === "unreachable"
                   ? qsTr("%1 did not answer; the push can only fast-forward, and its answer will say what went wrong.")
                     .arg(page.publishRemote)
                   : ""
        when: page.publishAsking
    }

    function startPublishAsk() {
        page.publishRemote = repoTab.defaultRemote
        page.publishBranch = workTree.branch
        // `push` goes untranslated — it is the command's spelling, not a
        // word for it.
        page.startRowAsk("", qsTr("%1 where?").arg(workTree.branch), "",
                         false, "", page.answerPublish, false, "",
                         publishForm, "push")
        // After the bar is up, never before: raising it resets the
        // properties the `publishAsking` bindings above own, and a binding
        // whose value has not changed does not push back.
        page.publishAsking = true
        page.refreshPublishCheck()
        // No remote at all: the chooser holds nothing but its last row,
        // so the dialog that row opens comes up unasked
        // (デザイン規約 §はじめてリモートへ送る).
        if (page.publishRemotes.length === 0)
            page.choosePublishRemote(page.publishRemotes.length)
    }
    /// Automation: typing into the name box, which no injected key can
    /// reach on the offscreen platform.
    function setPublishBranch(name) {
        page.publishBranch = name
        page.refreshPublishCheck()
    }
    /// Automation: the chooser's last row, and what gets typed into the
    /// dialog it opens (`<name>|<url>`, either half may be empty).
    function startPublishAddRemote(spec) {
        const parts = spec === "" ? [] : spec.split("|")
        page.choosePublishRemote(page.publishRemotes.length)
        remoteDialog.setFields(parts.length > 0 ? parts[0] : "",
                               parts.length > 1 ? parts[1] : "")
    }
    /// Automation: the chooser's list, which no injected click can reach.
    function openPublishRemotes() {
        const form = graphPane.askForm
        if (form && form.remotePick)
            form.remotePick.popup.open()
    }

    function answerPublish() {
        // The same slot a plain push fills: a refused first push is still
        // this button's news (デザイン規約 §リモートへ送る).
        page.pushSentBranch = workTree.branch
        repoTab.publishCurrent(page.publishRemote, page.publishBranch,
                               page.publishLease)
    }

    RemoteDialog {
        id: remoteDialog
        onSubmitted: (name, url) => {
            if (remoteDialog.editing === "")
                repoTab.addRemote(name, url)
            else
                repoTab.setRemoteUrl(name, url)
            if (page.publishAsking) {
                page.publishRemote = name
                page.refreshPublishCheck()
            }
        }
    }

    Component {
        id: publishForm
        ColumnLayout {
            spacing: Theme.spaceXs
            /// Automation only: the list cannot be opened by an injected
            /// click on the offscreen platform.
            property alias remotePick: remotePick
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            AppCombo {
                id: remotePick
                pickOnly: true
                lastRowActs: true
                // The fixed-input width every boxed field shares
                // (デザイン規約 §レイアウト初期値 160) — not a width of
                // its own.
                Layout.preferredWidth: 160
                model: page.publishChoices
                wanted: page.publishRemote
                onActivated: index => page.choosePublishRemote(index)
            }
            Label {
                Layout.alignment: Qt.AlignVCenter
                text: "/"
                color: Theme.textMuted
                font.pixelSize: Theme.fontMd
            }
            // Never a placeholder: an empty box would read as though
            // there were nothing to send.
            SlimField {
                id: publishBranchField
                Layout.preferredWidth: 160
                text: page.publishBranch
                onTextEdited: {
                    page.publishBranch = text
                    page.refreshPublishCheck()
                }
                Component.onCompleted: publishBranchField.forceActiveFocus()
            }
            Item {
                Layout.fillWidth: true
            }
        }
        }
    }

    // ---- context menu on a sidebar row ------------------------------
    // `menuRefName` is what the row shows, `menuRefId` what git knows it
    // by (they differ for a stash: a message and a selector).
    property string menuRefKind: ""
    property string menuRefName: ""
    property string menuRefId: ""
    property string menuRefOid: ""
    /// The branch git has just refused to delete, while the menu that
    /// asked is still standing.
    property string forceDeleteBranch: ""
    /// What this menu offers, decided as it opens and held while it
    /// stands: the conditions are live (a timer fetch alone moves
    /// `busyCount`), and a row that appears or vanishes under the pointer
    /// is a row clicked by accident (デザイン規約 §メニュー).
    property bool menuCanSwitch: false
    property bool menuCanIntegrateFrom: false
    property bool menuCanDelete: false
    /// The remote reading a branch row can also shed (`origin/main`),
    /// empty where it has none. What the remote-side delete rows name.
    property string menuRemoteCounterpart: ""
    property bool menuCanDeleteRemote: false
    /// Why the delete table's rows are out — decided as the menu opens,
    /// worn as the rows' `blockedReason` (デザイン規約 §無効).
    property bool menuOnCurrentBranch: false
    property bool menuWriteRunning: false
    readonly property string deleteBlockedOnCurrent:
        qsTr("Switch away first — this is the branch you are on")
    readonly property string deleteBlockedWhileBusy:
        qsTr("Another git command is still running")
    function openRefMenu(kind, name, full, oidHex) {
        page.menuRefKind = kind
        page.menuRefName = name
        page.menuRefId = full
        page.menuRefOid = oidHex
        page.forceDeleteBranch = ""
        page.rebasePublished = false
        page.menuCanSwitch = (kind === "branch" || kind === "remote")
                             && full !== workTree.branch
        page.menuCanIntegrateFrom = page.canIntegrateFrom
        // git refuses to delete the branch the working tree is on; its
        // remote reading can still be deleted.
        page.menuCanDelete = repoTab.busyCount === 0
                             && !(kind === "branch" && full === workTree.branch)
        page.menuRemoteCounterpart =
            kind === "branch" ? branchesModel.upstreamOf(full) : ""
        page.menuCanDeleteRemote = repoTab.busyCount === 0
                                   && page.menuRemoteCounterpart !== ""
        page.menuOnCurrentBranch = kind === "branch" && full === workTree.branch
        page.menuWriteRunning = repoTab.busyCount !== 0
        if (repoTab.state === "open" && page.rebaseRange !== "")
            repoTab.checkPublish(page.rebaseRange)
        // Whether the everyday delete would be refused, asked as the menu
        // opens: the unmerged answer usually lands before the pointer
        // does, and the delete row wears `-D` from the start instead of
        // only after a refused click (§左メニューの所作). The chip
        // column is settled at open, so the swap moves no other row.
        if (repoTab.state === "open" && kind === "branch" && page.menuCanDelete)
            repoTab.checkBranchDelete(full)
        return refMenu.offer()
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

    // ---- bringing two lines of history together --------------------
    readonly property bool canIntegrateFrom:
        repoTab.state === "open" && repoTab.busyCount === 0
        && !workTree.detached && workTree.branch !== ""
        && workTree.opText === ""
        && page.menuRefId !== "" && page.menuRefId !== workTree.branch
    /// The commits a rebase onto this row would rewrite. Asked as the
    /// menu opens — the answer is a whole git call away.
    readonly property string rebaseRange:
        (page.menuRefKind === "branch" || page.menuRefKind === "remote")
        && page.menuRefId !== "" ? page.menuRefId + "..HEAD" : ""
    property bool rebasePublished: false
    AppMenu {
        id: refMenu
        // Walking away from a refused delete takes the offer with it.
        // The settle re-run is for a menu that stood on the stacked
        // list's row: the list stayed up under it, and whether it stays
        // now is the pointer's to answer again.
        onClosed: {
            page.forceDeleteBranch = ""
            page.settleRefList()
        }
        AppMenuItem {
            code: "switch"
            offered: page.menuCanSwitch
            // Through the chips' dispatcher: a remote branch whose local
            // one already exists cannot simply be created.
            onTriggered: page.switchToRef(page.menuRefKind === "remote" ? "R" : "L",
                                          page.menuRefId)
        }
        AppMenuItem {
            code: "merge"
            //: Follows the `merge` chip: "merge into main".
            text: qsTr("into %1").arg(workTree.branch)
            offered: (page.menuRefKind === "branch"
                      || page.menuRefKind === "remote"
                      || page.menuRefKind === "tag")
                     && page.menuCanIntegrateFrom
            onTriggered: repoTab.merge(page.menuRefId, false, false, "")
        }
        AppMenuItem {
            code: "rebase"
            //: Follows the `rebase` chip: "rebase main onto it".
            text: qsTr("%1 onto it").arg(workTree.branch)
            // Deliberately not offered on a tag — that row belongs with
            // the tag's own gestures.
            offered: (page.menuRefKind === "branch"
                      || page.menuRefKind === "remote")
                     && page.menuCanIntegrateFrom
            // Said, not asked (要望: rewriting a pushed commit shows a
            // warning). Published = reachable from a remote-tracking ref,
            // only as fresh as the last fetch.
            note: page.rebasePublished ? qsTr("rewrites pushed commits") : ""
            onTriggered: repoTab.rebase(page.menuRefId, "", true)
        }
        AppMenuSeparator {}
        AppMenuItem {
            id: refDeleteItem
            // Alone in this menu the delete re-states its target: during
            // the hold the name of what is about to go has to be readable
            // on the row itself (デザイン規約 §メニュー 言い直さない、の例外).
            readonly property bool stashRow: page.menuRefKind === "stash"
            readonly property bool remoteRow: page.menuRefKind === "remote"
            readonly property bool tagRow: page.menuRefKind === "tag"
            readonly property bool branchRow: page.menuRefKind === "branch"
            // git already refused `--delete` while this menu stood — or
            // the check run at open came back unmerged, the same answer a
            // click ahead of time (§左メニューの所作).
            readonly property bool refusedRow:
                branchRow
                && (page.forceDeleteBranch === page.menuRefId
                    || (repoTab.branchDeleteAsked === page.menuRefId
                        && !repoTab.branchDeleteMerged))
            readonly property bool heldRow: stashRow || remoteRow || tagRow
                                            || refusedRow
            code: refusedRow ? "branch -D"
                : branchRow ? "branch --delete"
                : tagRow ? "tag --delete"
                : remoteRow ? "push --delete"
                : "drop"
            // The name is data, not sentence: never translated, and it
            // does not bid for the menu's width (`growsForText`).
            text: stashRow ? "" : page.menuRefId
            growsForText: false
            note: refusedRow ? qsTr("not merged") : ""
            // On a branch the three delete forms are a fixed table — rows
            // that cannot be chosen stay and grey out, the app-menu rule
            // rather than the assembled-menu one (デザイン規約 §メニュー、
            // 2026-08-11 ユーザー判断): the current branch keeps its rows,
            // saying why nothing here answers. The other kinds keep the
            // assembled rule.
            offered: branchRow || (heldRow && page.menuCanDelete)
            blockedReason: !branchRow || page.menuCanDelete ? ""
                         : page.menuOnCurrentBranch ? page.deleteBlockedOnCurrent
                                                    : page.deleteBlockedWhileBusy
            holdMs: heldRow ? Metrics.holdMs : 0
            // A branch's plain delete keeps the menu up: git's answer has
            // nowhere to land otherwise, and this row is where it lands.
            staysOpen: branchRow
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
        // The branch's remote reading, deleted without touching the local
        // one — on the current branch the one delete on offer at all
        // (デザイン規約 §左メニューの所作).
        AppMenuItem {
            id: refRemoteDeleteItem
            code: "push --delete"
            text: page.menuRemoteCounterpart
            growsForText: false
            // In the table only while the branch has a remote reading at
            // all: a row for a target that does not exist keeps no seat
            // (2026-08-11 ユーザー判断). Grey is for "not now" — busy —
            // not for "no such thing".
            offered: page.menuRefKind === "branch"
                     && page.menuRemoteCounterpart !== ""
            blockedReason: page.menuCanDeleteRemote
                           ? "" : page.deleteBlockedWhileBusy
            holdMs: Metrics.holdMs
            holdTone: Theme.warning
            onHeld: {
                refMenu.close()
                page.deleteRemoteNow(page.menuRemoteCounterpart)
            }
        }
        // A composite of two commands is no one command, so words rather
        // than a chip (§git 用語のコード表記 の 1:1 規則). The local half
        // runs first and a refusal stops the pair with nothing touched.
        AppMenuItem {
            id: refBothDeleteItem
            text: qsTr("Delete both")
            note: refDeleteItem.refusedRow ? qsTr("not merged") : ""
            offered: page.menuRefKind === "branch"
                     && page.menuRemoteCounterpart !== ""
            blockedReason: page.menuCanDelete && page.menuCanDeleteRemote ? ""
                         : page.menuOnCurrentBranch ? page.deleteBlockedOnCurrent
                                                    : page.deleteBlockedWhileBusy
            holdMs: Metrics.holdMs
            holdTone: Theme.warning
            onHeld: {
                refMenu.close()
                const c = page.menuRemoteCounterpart
                const cut = c.indexOf("/")
                if (cut < 0)
                    return
                repoTab.deleteBranchEverywhere(page.menuRefId,
                                               c.substring(0, cut),
                                               c.substring(cut + 1),
                                               refDeleteItem.refusedRow)
            }
        }
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
    /// Held, not asked: git refuses nothing here — the branch is on the
    /// far side, so no `-d` can weigh what it holds — and the hold stands
    /// in for that refusal (デザイン規約 §リモートブランチを消す).
    function deleteRemoteNow(remoteRef) {
        const cut = remoteRef.indexOf("/")
        if (cut < 0)
            return
        repoTab.deleteRemoteBranch(remoteRef.substring(0, cut),
                                   remoteRef.substring(cut + 1))
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
        page.forceDeleteBranch = name
        if (AppBackend.autoAct !== "")
            AppBackend.report("force_delete_offered branch=" + name)
    }

    // ---- context menu on a working-tree file row --------------------
    property string menuFilePath: ""
    property string menuFileBucket: ""
    /// Whether this menu's writing rows were on offer as it opened, held
    /// still for as long as it stands (`menuCanSwitch` and the rest).
    property bool menuFileCanWrite: false
    /// How many files the rows here would touch, counted the way the
    /// writes count them (`chosenRows()` skips folder rows, which nothing
    /// in this menu acts on). Not `chosenCount` — that counts chosen
    /// keys, so a folder in the choice would put a file on the tag that
    /// no command is going to reach.
    property int menuFileCount: 0
    function openFileMenu(bucket, path) {
        // A right-click is a click: it walks away from a question that
        // was standing, which may well be about another row.
        page.stopRowAsk()
        page.menuFileBucket = bucket
        page.menuFilePath = path
        page.menuFileCanWrite = repoTab.busyCount === 0
        // What the discard row would do, worked out once here: the choice
        // cannot change while the menu is up, so the words the row says
        // and the writes it runs are read off the same plan.
        page.discardPlan = page.planDiscard()
        page.menuFileCount = wipPane.chosenRows().length
        fileMenu.offer()
        if (AppBackend.autoAct !== "")
            AppBackend.report("file_menu bucket=" + bucket
                              + " rows=" + fileMenu.offeredRows)
    }
    /// What the chosen rows' discard costs, by the bucket the row was
    /// opened on (デザイン規約 §その他の操作):
    ///
    /// - unstaged — the edits on disk go, and what is staged stays
    /// - untracked — the file goes; there the file *is* the change
    /// - staged — both sides go, back to HEAD, and a rename takes the
    ///   name it came from with it or leaves half of itself staged
    ///
    /// git refuses to restore a conflicted path until told how it was
    /// resolved, so a conflicted row rides along untouched and uncounted.
    property var discardPlan: null
    function planDiscard() {
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
    function discardWords(plan) {
        return !plan || plan.count === 0 ? "" : qsTr("Discard")
    }
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
    /// Held, not asked (デザイン規約 §長押し).
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
    /// be taken on.
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
    /// The paths are always named: git walks a bare `mergetool` one file
    /// at a time and holds the write queue for the whole walk.
    function openInMergeTool() {
        if (wipPane.workTree.mergeTool === "") {
            page.settingsDialogRequested()
            return
        }
        const paths = page.chosenConflicts()
        if (paths.length === 0)
            return
        page.sendPaths(paths)
        repoTab.openMergetool()
    }
    AppMenu {
        id: fileMenu
        // Named by branch rather than `--ours` / `--theirs` — during a
        // rebase those two swap over (デザイン規約 §conflict の ours /
        // theirs). Plain clicks: a conflicted file has no settled version
        // to lose.
        AppMenuItem {
            text: wipPane.workTree.sideOurs !== ""
                  ? qsTr("Keep %1's version").arg(wipPane.workTree.sideOurs)
                  : qsTr("Keep this branch's version")
            offered: page.menuFileBucket === "conflicts"
                     && page.menuFileCanWrite
            onTriggered: page.takeSideNow("ours")
        }
        AppMenuItem {
            text: wipPane.workTree.sideTheirs !== ""
                  ? qsTr("Take %1's version").arg(wipPane.workTree.sideTheirs)
                  : qsTr("Take the incoming version")
            offered: page.menuFileBucket === "conflicts"
                     && page.menuFileCanWrite
            onTriggered: page.takeSideNow("theirs")
        }
        // With nothing configured the row becomes the door to the setting
        // (規約 §conflict を外部ツールへ渡す).
        AppMenuItem {
            text: wipPane.workTree.mergeTool !== ""
                  ? qsTr("Edit in %1").arg(wipPane.workTree.mergeTool)
                  : qsTr("Edit in <merge editor>…")
            offered: page.menuFileBucket === "conflicts"
                     && page.menuFileCanWrite
            onTriggered: page.openInMergeTool()
        }
        AppMenuSeparator {}
        AppMenuItem {
            code: "stash"
            note: page.menuFileCount > 1
                  ? qsTr("%n files", "", page.menuFileCount) : ""
            // git will not stash a tree with unresolved conflicts in it.
            offered: page.menuFileBucket !== "conflicts"
                     && page.menuFileCanWrite
            onTriggered: {
                const rows = wipPane.chosenRows()
                const paths = []
                for (let i = 0; i < rows.length; i++)
                    paths.push(rows[i].fullName)
                page.sendPaths(paths)
                repoTab.stashPaths("")
            }
        }
        // Held, not asked (デザイン規約 §長押し). On a file changed on
        // both sides the two rows are the choice itself: the unstaged one
        // keeps what is staged, the staged one takes the lot.
        AppMenuItem {
            id: fileDiscardItem
            text: page.discardWords(page.discardPlan)
            note: page.discardNote(page.discardPlan)
            offered: page.menuFileBucket !== "conflicts"
                     && page.menuFileCanWrite
            holdMs: Metrics.holdMs
            onHeld: {
                fileMenu.close()
                page.discardChosenNow(page.discardPlan)
            }
        }
        AppMenuSeparator {}
        AppMenuItem {
            text: page.menuFileCount > 1 ? qsTr("Copy paths") : qsTr("Copy path")
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
    // not asked about — nothing here leaves the machine — but デザイン規約
    // 「push 済みの範囲は尋ねずに言う」 wants it said, so the squash row
    // carries a tag the way the amend editor does. The answer lands a
    // frame after the menu opens.
    property bool menuPublished: false
    /// What these menus offer, held still for as long as they stand
    /// (`menuCanSwitch` and the rest). `menuCanSequence` is the pair that
    /// only add a commit, and so ask less of the repository than the rest.
    property bool menuCanSequence: false
    property bool menuCanIntegrate: false
    property bool menuCanEditHistory: false
    property bool menuCanMoveBranch: false
    property bool menuStashCanWrite: false
    function openRowMenu(oidHex) {
        page.menuOid = oidHex
        page.menuStashRef = graphModel.stashRefOf(oidHex)
        if (page.menuStashRef !== "") {
            page.menuStashCanWrite = repoTab.busyCount === 0
            stashMenu.offer()
            return
        }
        page.menuPublished = false
        page.menuCanSequence = repoTab.busyCount === 0
                               && workTree.opText === ""
        page.menuCanIntegrate = page.canIntegrateHere
        page.menuCanEditHistory = page.canEditHistoryHere
        page.menuCanMoveBranch = page.canMoveBranchHere
        if (repoTab.state === "open")
            repoTab.checkPublish(oidHex + "^!")
        commitMenu.offer()
    }

    AppMenu {
        id: stashMenu
        AppMenuItem {
            code: "apply"
            offered: page.menuStashCanWrite
            onTriggered: repoTab.applyStash(page.menuStashRef)
        }
        AppMenuItem {
            code: "pop"
            offered: page.menuStashCanWrite
            onTriggered: {
                repoTab.popStash(page.menuStashRef)
                page.selectedStashRef = ""
            }
        }
        AppMenuSeparator {}
        // Held, not asked (デザイン規約 §長押し).
        AppMenuItem {
            id: stashDeleteItem
            code: "drop"
            offered: page.menuStashCanWrite
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
            offered: page.menuCanSequence
            onTriggered: repoTab.cherryPick(page.menuOid)
        }
        // Both cherry-pick and revert only add a commit, so neither is
        // asked about or held.
        AppMenuItem {
            code: "revert"
            offered: page.menuCanSequence
            onTriggered: repoTab.revert(page.menuOid)
        }
        AppMenuSeparator {}
        AppMenuItem {
            code: "merge"
            //: Follows the `merge` chip: "merge into main".
            text: qsTr("into %1").arg(workTree.branch)
            offered: page.menuCanIntegrate
            onTriggered: repoTab.merge(page.menuOid, false, false, "")
        }
        AppMenuItem {
            code: "rebase"
            //: Follows the `rebase` chip: "rebase main onto it".
            text: qsTr("%1 onto it").arg(workTree.branch)
            note: page.menuPublished ? qsTr("rewrites pushed commits") : ""
            offered: page.menuCanIntegrate
            onTriggered: repoTab.rebase(page.menuOid, "", true)
        }
        // No row for landing on the commit itself: that leaves HEAD on no
        // branch (デザイン規約 §ブランチ・コミットへの移動) — the
        // double-click already offers a branch at this commit.
        AppMenuSeparator {}
        // No entry for editing the message: the click that opens this
        // menu already puts the message in the details pane's boxes.
        AppMenuItem {
            code: "squash"
            //: Follows the `squash` chip: "squash into parent".
            text: qsTr("into parent")
            // Said, not asked (要望: rewriting a pushed commit shows a
            // warning): the squash goes ahead, and this tag is the warning.
            note: page.menuPublished ? qsTr("already pushed") : ""
            offered: page.menuCanEditHistory
            onTriggered: page.squashCommit(page.menuOid)
        }
        // Held while this branch is the only thing holding its tip; a
        // plain click once something else does — then the replaced
        // commits stay drawn and a cherry-pick brings any of them back
        // (デザイン規約 §長押し). The answer is a property of the branch,
        // not of the row, so it is already in hand when the menu opens: a
        // mark appearing later would re-indent every row
        // (`AppMenu.holdIndent`) with the hand already on its way.
        AppMenuItem {
            id: dropCommitItem
            code: "drop"
            note: page.menuPublished ? qsTr("already pushed") : ""
            offered: page.menuCanEditHistory
            holdMs: repoTab.headReachedElsewhere ? 0 : Metrics.holdMs
            onTriggered: page.dropCommit(page.menuOid)
            onHeld: {
                commitMenu.close()
                page.dropCommit(page.menuOid)
            }
        }
        AppMenu {
            id: resetMenu
            titleCode: "reset"
            //: Follows the `reset` chip: "reset main here".
            title: workTree.branch !== ""
                   ? qsTr("%1 here").arg(workTree.branch)
                   : qsTr("the branch here")
            applies: page.menuCanMoveBranch
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
            // Held, not asked (デザイン規約 §長押し).
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

    // The refs one chip had to stack, unstacked under it. It opens and
    // closes with the pointer, and the pointer is over exactly one of the
    // two things that keep it up: the chip, or the list itself.
    RefListPopup {
        id: refList
        currentBranch: workTree.branch
        onPicked: record => page.activateRecord(record)
        // The stacked rows answer the same right-click the chip does,
        // with no row of the graph to fall back to.
        onMenuAsked: record => page.openRecordMenu(record, "")
        onClosed: {
            page.refListWanted = false
            page.refListAnchor = null
        }
        onPointerInsideChanged: page.settleRefList()
    }
    // ---- the row's own card -----------------------------------------
    // Opened by a row once the pointer has rested on it, closed when the
    // pointer leaves both it and the row. Owned here because rows are
    // recycled out from under it, the same reason the ref list is.
    CommitHoverCard {
        id: rowCard
        textWidth: graphPane.width / 2
        // A quarter each to the subject and the body, so the card can
        // never pass half the pane however long a message is.
        textHeight: graphPane.height / 4
        onPointerInsideChanged: page.settleRowCard()
    }
    property bool rowCardWanted: false
    /// The chip's list is up, or is about to be. Only one of the two is
    /// ever out, and the chip's is the more particular
    /// (デザイン規約 §hover のツールチップ).
    readonly property bool refListUp: page.refListWanted || refList.opened
    function openRowCard(row) {
        if (!row || page.refListUp)
            return
        rowCard.subject = row.subject
        rowCard.body = row.body
        rowCard.author = row.author
        rowCard.atime = row.atime
        rowCard.mates = row.co_authors
        // Under the pointer, not under the row: a row is as wide as the
        // pane, so its left edge is nowhere near the hand.
        const at = row.mapToItem(page, row.pointerX, row.height)
        rowCard.x = at.x
        rowCard.y = at.y
        page.rowCardWanted = true
        rowCard.open()
    }
    function settleRowCard() {
        rowCardSettle.restart()
    }
    /// Down now, not in a beat's time: what makes way for the chip's
    /// list has to be gone before it is drawn, or the two overlap for
    /// as long as the wait.
    function closeRowCard() {
        rowCard.close()
    }
    // Closing waits a beat rather than a turn of the event loop. Walking
    // from the row into the card it opened crosses a boundary where the
    // two hovers change in different frames, and `Qt.callLater` runs
    // between them — the card closed under the hand (2026-08-09 report).
    Timer {
        id: rowCardSettle
        interval: Metrics.hoverKeepMs
        onTriggered: {
            if (!rowCard.pointerInside && !page.rowCardWanted)
                rowCard.close()
        }
    }

    property bool refListWanted: false
    /// The chip the open list hangs off (null when none). The graph's
    /// rows read it back through `GraphPane.chipListAnchor`, so a hand
    /// that walked down into the list and comes back to that chip
    /// re-holds it instead of sitting out the opening rest again.
    property var refListAnchor: null
    function openRefList(records, anchor) {
        const at = anchor.mapToItem(page, 0, anchor.height)
        // The row's card opens under the pointer, which is on the chip
        // — it would be drawn over the list the chip is opening.
        page.closeRowCard()
        refList.records = records
        // Sized before it is shown, so it does not grow under the hand
        // that is walking into it — see the function.
        refList.layOutRows()
        refList.x = at.x
        refList.y = at.y
        page.refListWanted = true
        page.refListAnchor = anchor
        refList.open()
    }
    function closeRefListUnlessEntered() {
        page.refListWanted = false
        page.settleRefList()
    }
    // The list opens flush under the chip, so walking into it takes the
    // pointer off the chip on the way, and walking back out puts it on
    // again. The two hovers change in different frames and in no fixed
    // order, and `Qt.callLater` runs between them — the list shut under
    // the hand on the way in (2026-08-09 report). So the answer waits a
    // beat, the way the row's card does.
    function settleRefList() {
        refListSettle.restart()
    }
    Timer {
        id: refListSettle
        interval: Metrics.hoverKeepMs
        onTriggered: {
            // A ref menu standing on one of the list's rows keeps the
            // list up under it: the hand went into the menu, not away,
            // and closing the list would pull the ground out from what
            // it right-clicked. The menu's own close settles this again.
            if (!refList.pointerInside && !page.refListWanted
                    && !refMenu.opened)
                refList.close()
        }
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

    /// Leaves the commit out of the history. One place for both ways in:
    /// the row is a hold or a click depending on whether anything else
    /// still holds the branch tip, and what it runs must not depend on
    /// which of the two the reader got (デザイン規約 §履歴を合流させる).
    function dropCommit(oidHex) {
        repoTab.dropCommit(oidHex)
    }

    // ---- taking the branch back to an earlier commit ----------------
    // Not offered mid-operation: mid-merge a soft reset refuses outright
    // and the other two abandon the merge without a word.
    /// Rewriting this commit's place in the history. Unlike the two
    /// above it, the newest commit is fair game — that is the one a fold
    /// or a drop most often means.
    readonly property bool canEditHistoryHere:
        repoTab.state === "open" && repoTab.busyCount === 0
        && !workTree.detached && workTree.branch !== ""
        && workTree.opText === ""
        && page.menuOid !== "" && page.menuStashRef === ""
    readonly property bool canIntegrateHere:
        repoTab.state === "open" && repoTab.busyCount === 0
        && !workTree.detached && workTree.branch !== ""
        && workTree.opText === ""
        && page.menuOid !== "" && page.menuOid !== workTree.headOid
        && page.menuStashRef === ""
    readonly property bool canMoveBranchHere:
        repoTab.state === "open" && repoTab.busyCount === 0
        && !workTree.detached && workTree.branch !== ""
        && workTree.opText === ""
        && page.menuOid !== "" && page.menuOid !== workTree.headOid
        && page.menuStashRef === ""

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
            refMenu: refMenu
            refDeleteItem: refDeleteItem
            fileMenu: fileMenu
            fileDiscardItem: fileDiscardItem
            commitMenu: commitMenu
            dropCommitItem: dropCommitItem
            stashDeleteItem: stashDeleteItem
            resetMenu: resetMenu
            hardResetItem: hardResetItem
            remoteDialog: remoteDialog
            refList: refList
            rowCard: rowCard
        }
    }

    // A finished write the editor asked for: clear it only once git
    // says the commit landed, so a rejected one keeps its text.
    property int seenWriteSeq: 0
    function absorbWriteResult() {
        if (repoTab.writeSeq === page.seenWriteSeq)
            return
        page.seenWriteSeq = repoTab.writeSeq
        // A push this button sent has come back. Nothing in the answer
        // says which branch it was for, and a remote branch's rename and
        // delete report under the same name — the slot filled at the send
        // is what makes this the toolbar button's news
        // (デザイン規約 §リモートへ送る).
        if (repoTab.lastWriteOp === "push" && page.pushSentBranch !== "") {
            const sent = page.pushSentBranch
            page.pushSentBranch = ""
            if (repoTab.lastWriteError !== "") {
                page.pushFailBranch = sent
                page.pushFailReason = repoTab.lastWriteError
            } else if (page.pushFailBranch === sent) {
                // Landed. The button that went through must not go on
                // saying that the go before it did not.
                page.pushFailBranch = ""
                page.pushFailReason = ""
            }
        }
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
        page.diffChange = kind === "conflicts" ? worktreeModel.changeOf(path) : ""
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
        diffPane.holdScroll()
        repoTab.stageSelection(page.diffKind, page.diffPath, page.diffOrigPath,
                               hunk, line, diffModel.fingerprint)
        page.pendingDiffReload = true
    }
    /// The lines picked by hand, all of them in one write: the diff is
    /// read once and rebuilt once, however many were chosen
    /// (デザイン規約 §diff の中のステージ). The indices belong to the diff
    /// on screen, so the choice goes down with the rebuild that follows.
    function stageChosenLines() {
        const pairs = diffPane.chosenPairs()
        if (pairs.length === 0)
            return
        diffPane.holdScroll()
        repoTab.beginLines()
        for (let i = 0; i < pairs.length; i++)
            repoTab.addLine(pairs[i][0], pairs[i][1])
        repoTab.stageLines(page.diffKind, page.diffPath, page.diffOrigPath,
                           diffModel.fingerprint)
        diffPane.clearLines()
        page.pendingDiffReload = true
    }
    /// Throwing one hunk of the shown diff away, with no question in front
    /// of it: the button in that hunk's own heading was held down, which is
    /// the whole of the asking (デザイン規約 §その他の操作). A line cannot
    /// be thrown away on its own — the hunk is the smallest piece — though
    /// it can still be staged on its own, which loses nothing.
    function discardHunkNow(hunk) {
        diffPane.holdScroll()
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
        // Whatever was picked addressed the diff that is being replaced.
        diffPane.clearLines()
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

    /// The layout this page starts with: what the window is set to now,
    /// which after a restart is what the last session left. Read once
    /// rather than bound — from here on the splitters own these, and a
    /// binding would fight the drag (規約 §左メニューを畳む).
    function applySavedLayout() {
        // The width first: while the list has never been folded, the
        // splitter still reads it through a binding, so this is what puts
        // an unfolded list at the width it was left. Folding after is what
        // pins the rail (`applyFold`, through the sidebar's own binding on
        // this property — calling it here as well would only run it twice).
        sidebarPane.openWidth = AppBackend.startSidebarWidth()
        page.sidebarCollapsed = AppBackend.startSidebarCollapsed()
        page.commandsOpen = AppBackend.startCommandsShown()
        rightPane.SplitView.preferredWidth = AppBackend.startDetailsWidth()
        commandsPane.SplitView.preferredHeight = AppBackend.startCommandsHeight()
        // -1 travels through unchanged: the pane reads it as "follow the
        // default lane count", which is what a divider nobody has dragged
        // has always done.
        graphPane.labelWManual = AppBackend.startGraphLabelsWidth()
        graphPane.graphColWManual = AppBackend.startGraphLanesWidth()
        sidebarPane.expBranches = AppBackend.startSection("branches")
        sidebarPane.expRemotes = AppBackend.startSection("remotes")
        sidebarPane.expWorktree = AppBackend.startSection("worktree")
        sidebarPane.expStashes = AppBackend.startSection("stashes")
        sidebarPane.expTags = AppBackend.startSection("tags")
        if (page.blank)
            return
        repoTab.setTagsShown(AppBackend.startTagsShown())
        worktreeModel.setTreeView(AppBackend.startWipTree())
        detailsModel.setTreeView(AppBackend.startDetailsTree())
    }

    // ---- how narrow and how short this page may be laid out ------------
    // Under these the panes stop giving: `SplitView` does not shrink an
    // item past its minimum, it lays the rest out beyond its own edge, and
    // nothing in this window scrolls to reach what went over (measured
    // 2026-08-09: a 640px window left 32px of the right pane on screen and
    // no way to the other 268). So the window is held to them instead
    // (`Main.floorWidth` / `floorHeight`).
    //
    // Every number here is one §レイアウト初期値 already carries; naming
    // them is what lets the item that obeys one and the floor that is
    // built on it read the same value.
    readonly property int rightMinWidth: 300
    readonly property int panesMinHeight: 200
    readonly property int commandsMinHeight: 120
    /// Whether the working-tree pane has anything below its own fold —
    /// the editor, the commit button and a stopped operation's exit card
    /// keep their heights by construction, so in a short pane they are
    /// reached by scrolling rather than not at all (`window-floor wip`).
    readonly property bool wipBlockScrolls: wipPane.blockScrolls
    /// …and how far the commit-details pane runs past its own bottom,
    /// which is the same question asked of the other half of this seat.
    readonly property real detailsOverHeight: detailsPane.contentOverHeight

    /// The middle column's floor. The graph gives up its own columns
    /// first — the chips, then the lanes (`GraphPane.contentMinW`) — and
    /// stops where all three of them would stop saying anything. Never
    /// under what a side pane may be, so that three columns still read as
    /// three at the floor.
    readonly property real centreMinWidth:
        Math.max(graphPane.contentMinW, sidebarPane.minOpenWidth)
    /// What the folded list costs is the rail, so folding lowers this and
    /// unfolding raises it — and a window standing at the old floor is
    /// grown by the new one rather than cutting the list off (Main).
    readonly property real floorWidth:
        (page.sidebarCollapsed ? Theme.railWidth : sidebarPane.minOpenWidth)
        + Theme.splitterWidth + page.centreMinWidth
        + Theme.splitterWidth + page.rightMinWidth
    /// The log is a second row when it is open, and it brings its own
    /// floor with it — so opening it raises this the same way.
    readonly property real floorHeight:
        page.panesMinHeight
        + (page.commandsOpen
           ? Theme.splitterWidth + page.commandsMinHeight : 0)

    /// Assigned, not bound — a drag writes the same attached property and
    /// would be gone after the first one (規約 §左メニューを畳む).
    function setDetailsWidth(w) {
        rightPane.SplitView.preferredWidth = w
    }

    /// What dragging the two dividers inside the graph would leave. Only
    /// the headless state check calls this; a person drags.
    function setGraphColumns(labels, lanes) {
        graphPane.labelWManual = labels
        graphPane.graphColWManual = lanes
    }

    /// Hands the window's layout over to be remembered. Pulled on a timer
    /// by the window rather than pushed as each value changes: a splitter
    /// drag moves a width on every frame, and the point is to write what
    /// it settled on.
    function reportLayout() {
        AppBackend.saveLayoutSizes(
            // While the list is folded its width is the rail's; the width
            // it goes back to is the one worth keeping.
            page.sidebarCollapsed ? sidebarPane.openWidth : sidebarPane.width,
            rightPane.width,
            // The height it asks for, open or closed — not the one it was
            // laid out at. A drag writes this same property, so what a
            // hand set is here; what a short window squeezed it to is not
            // (the same rule the folded list keeps: a size nobody chose is
            // not a size to come back to. Measured before the window had a
            // floor: opening the log in a 420px window wrote 168 over the
            // 280 that had been asked for, and every launch after came
            // back to the smaller one).
            commandsPane.SplitView.preferredHeight,
            // The dragged values, not the widths on screen: a column that
            // nobody has moved reports -1 and goes on following the
            // default rather than freezing today's number into the file.
            graphPane.labelWManual, graphPane.graphColWManual)
        AppBackend.saveLayoutFlags(page.sidebarCollapsed, page.commandsOpen,
                                   repoTab.tagsShown, worktreeModel.treeView,
                                   detailsModel.treeView)
        AppBackend.saveSections(sidebarPane.expBranches, sidebarPane.expRemotes,
                                sidebarPane.expWorktree, sidebarPane.expStashes,
                                sidebarPane.expTags)
    }

    Component.onCompleted: {
        page.applySavedLayout()
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
            // The same three lines the picker's dialog says, out of the
            // same place (`Words.openFailure`): which of the three it was
            // is core's answer (`errorKind`), not something read off
            // git's wording. Which of the two screens a failure lands on
            // is decided by the road, not the kind: the picker's answers
            // go to the dialog, everything else — a restored tab, a
            // worktree row, PG_AUTO_OPEN — comes here.
            Item {
                visible: page.openFailed
                SplitView.fillHeight: true
                SplitView.minimumHeight: page.panesMinHeight
                Column {
                    anchors.centerIn: parent
                    spacing: Theme.spaceMd
                    width: Math.min(700, page.width - 2 * Theme.spaceXl)
                    Label {
                        text: Words.openFailure(repoTab.errorKind)
                        font.pixelSize: Theme.fontLg
                        font.weight: Font.DemiBold
                        anchors.horizontalCenter: parent.horizontalCenter
                    }
                    Label {
                        text: repoTab.errorPath
                        color: Theme.textSecondary
                        elide: Text.ElideMiddle
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                    }
                    // git's words, and the only red on this screen: the
                    // gate that reports a git it cannot use draws the
                    // line the same way (Main.qml).
                    Label {
                        visible: repoTab.errorKind === "other"
                        text: repoTab.error
                        color: Theme.danger
                        wrapMode: Text.Wrap
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                    }
                    // A plain frame (規約 §肯定側のボタン).
                    ActionButton {
                        implicitHeight: Theme.controlHeight
                        text: qsTr("Close tab")
                        frameColor: Theme.borderDefault
                        activeFocusOnTab: true
                        anchors.horizontalCenter: parent.horizontalCenter
                        onActivated: page.closeTabRequested()
                    }
                }
            }

            // ---- three-pane layout --------------------------------------
            // (repository state / search / fetch / push live in the window
            // toolbar, next to the tabs; Reload is the app menu and F5)
            SplitView {
                visible: !page.openFailed
                SplitView.fillHeight: true
                SplitView.minimumHeight: page.panesMinHeight
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
                    // Not a number of its own: what the graph's own columns
                    // come to once they have both given everything they can,
                    // held up to a side pane's width so the middle never
                    // reads as the thinnest of the three (page.centreMinWidth).
                    SplitView.minimumWidth: page.centreMinWidth
                    currentIndex: page.diffShown ? 1 : 0

                    GraphPane {
                        id: graphPane
                        graphModel: graphModel
                        workTree: workTree
                        blank: page.blank
                        chipListAnchor: page.refListAnchor
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
                            page.openRefList(records, anchor)
                        onChipCollapseRequested: page.closeRefListUnlessEntered()
                        onRowHoverRequested: (row, inside) => {
                            page.rowCardWanted = inside
                            if (inside)
                                page.openRowCard(row)
                            else
                                page.settleRowCard()
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
                        busy: repoTab.busyCount > 0
                        onCloseRequested: page.closeDiff()
                        onNothingLeft: page.closeDiff()
                        onDiscardHunkRequested: hunk => page.discardHunkNow(hunk)
                        onStageFileRequested: {
                            if (page.diffStaged)
                                repoTab.unstagePath(page.diffPath)
                            else
                                repoTab.stagePath(page.diffPath)
                        }
                        onStageSelectionRequested: (hunk, line) => page.stageSelection(hunk, line)
                        onStageChosenRequested: page.stageChosenLines()
                    }
                }

                // Right side: working tree ⇄ commit details
                Rectangle {
                    id: rightPane
                    // Where it starts; `applySavedLayout` assigns over this
                    // with the width the window is set to.
                    SplitView.preferredWidth: 400
                    SplitView.minimumWidth: page.rightMinWidth
                    color: Theme.bgSurface

                    // Which git is doing all this. It sits here rather
                    // than at the window's edge so the command log can
                    // open without landing on top of it. A build made in
                    // a worktree adds which one — parallel sessions'
                    // windows are otherwise identical (デザイン規約
                    // §アプリ名).
                    RowLayout {
                        id: gitCorner
                        spacing: Theme.spaceXs
                        /// Its own box and nothing more — no extra air
                        /// above: a list's rows are already spaced by
                        /// their own height (2026-08-10 ユーザー報告).
                        readonly property real roomNeeded:
                            gitCorner.implicitHeight + Theme.spaceXs
                        /// What the pane under it is leaving bare. Only one
                        /// of the two is on screen at a time, and each
                        /// measures its own file list.
                        readonly property real roomLeft:
                            page.wipShown ? wipPane.bottomRoom
                                          : detailsPane.bottomRoom
                        // Out of the way as soon as the list reaches this
                        // corner (2026-08-10 ユーザー報告). Hidden
                        // outright rather than held at zero opacity: a
                        // Label keeps its implicit height while
                        // `visible: false`, so the answer never eats what
                        // it read (qmltestrunner, 2026-08-10 実測).
                        visible: AppBackend.gitVersion !== ""
                                 && gitCorner.roomLeft >= gitCorner.roomNeeded
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        anchors.rightMargin: Theme.spaceSm
                        anchors.bottomMargin: Theme.spaceXs
                        // Declared before the panes it hangs over, so z
                        // holds it in front of whatever they draw here.
                        z: 1
                        Label {
                            text: qsTr("git %1").arg(AppBackend.gitVersion)
                            color: Theme.textMuted
                            font.pixelSize: Theme.fontSm
                        }
                        // Drawn rather than typed: as a glyph the spacing
                        // here was a full-width cell's leftover (規約 §余白).
                        DotMark {
                            visible: AppBackend.buildTree !== ""
                            Layout.alignment: Qt.AlignVCenter
                        }
                        Label {
                            visible: AppBackend.buildTree !== ""
                            text: AppBackend.buildTree
                            color: Theme.textMuted
                            font.pixelSize: Theme.fontSm
                        }
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
                SplitView.minimumHeight: page.commandsMinHeight
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

    // ---- a split bar that has run out of room ---------------------------
    // The graph's own dividers know what the hand asked for; a
    // `SplitView` bar cannot (measured 2026-08-11, qmltestrunner:
    // SplitView takes the press before anything inside the delegate sees
    // it, and no observer behind the view ever becomes active). What is
    // knowable is where the pointer went and where the bar stopped, and
    // past a clamp those part company — the same refusal read from the
    // other end.

    /// Every split bar in this page, and whichever one has the hand. One
    /// pointer, so at most one at a time. The list is collected rather
    /// than declared: one `handle:` Component builds every bar of its
    /// SplitView, so there is nothing to hang an id on.
    property var splitBars: []
    property Item heldSplit: null
    function holdSplitBar(bar, held) {
        if (page.splitBars.indexOf(bar) < 0)
            page.splitBars.push(bar)
        if (held)
            page.heldSplit = bar
        else if (page.heldSplit === bar)
            page.heldSplit = null
    }

    /// What the watcher settles on, given where the hand is in the scene.
    /// The `PointHandler` and the automation hook both come through here,
    /// so the refusal is one answer rather than two kept in step.
    ///
    /// Not a binding: `mapToItem` is a method, and a binding over it would
    /// take no dependency on the geometry it reads and freeze on its first
    /// answer (app-ui.md). A drag moves the bar every frame, so this is
    /// pushed on each point instead.
    function settleSplitRefusal(sceneX, sceneY) {
        const bar = page.heldSplit
        if (!bar) {
            splitRefusal.refuses = false
            return
        }
        const mid = bar.mapToItem(null, bar.width / 2, bar.height / 2)
        const gap = bar.sideways ? sceneX - mid.x : sceneY - mid.y
        splitRefusal.at = splitRefusal.mapFromItem(null, sceneX, sceneY)
        // Past the bar by more than the bar is wide, for the reason the
        // graph's dividers use the same slack: the pointer sits somewhere
        // inside the bar it grabbed, and SplitView carries the bar along
        // at that offset for as long as the layout lets it.
        splitRefusal.refuses = Math.abs(gap) > Theme.splitterWidth
    }

    /// Automation (`PG_AUTO_ACT=divider-refuse`, cases `sidebar-min` /
    /// `details-min` / `log-min`): a press is no more injectable than
    /// hover, so the hook puts the bar in hand the way a press does and
    /// walks the same road the pointer walks.
    function dragSplitPast(which) {
        // Only bars that are drawn. A SplitView builds a handle between
        // every pair of items including the ones standing invisible (the
        // open-failed screen sits beside the panes), and one that is not
        // on screen is not one a hand could have grabbed — nor one with a
        // width and height to tell its direction from.
        const wanted = page.splitBars.filter(b =>
            b.visible && (which === "log-min" ? !b.sideways : b.sideways))
        if (wanted.length === 0)
            return false
        // Left to right, so the sidebar's bar is the first of the two the
        // horizontal view builds and the details pane's is the last.
        wanted.sort((a, b) => a.mapToItem(null, 0, 0).x - b.mapToItem(null, 0, 0).x)
        const bar = which === "details-min" ? wanted[wanted.length - 1] : wanted[0]
        page.heldSplit = bar
        const mid = bar.mapToItem(null, bar.width / 2, bar.height / 2)
        const over = 4 * Theme.splitterWidth
        // Into the pane that has no more to give: the sidebar and the log
        // are squeezed from their own side, the details pane from the left.
        if (which === "details-min")
            page.settleSplitRefusal(mid.x + over, mid.y)
        else if (which === "sidebar-min")
            page.settleSplitRefusal(mid.x - over, mid.y)
        else
            page.settleSplitRefusal(mid.x, mid.y + over)
        return true
    }

    /// Automation: the drag and its answer for every boundary in the
    /// window (`PG_AUTO_ACT=divider-refuse`).
    function reportDividerRefusal(which) {
        // The graph's own dividers know what was asked; a split bar is
        // read from where the pointer went instead.
        const split = which === "sidebar-min" || which === "details-min"
                      || which === "log-min"
        if (split)
            page.dragSplitPast(which)
        else
            graphPane.dragDividerPast(which)
        AppBackend.report("divider_refuse refuses=" + page.refusalShown
                          // The boundary itself, still drawn and still
                          // promising the drag the other way. For a split
                          // bar this also catches a hook that never found
                          // one to put in hand.
                          + " line=" + (split
                                        ? (page.heldSplit ? page.heldSplit.visible
                                                          : false)
                                        : graphPane.refusedLineShown)
                          + " case=" + which
                          + " labelW=" + graphPane.labelW
                          + " graphW=" + graphPane.graphColW)
    }

    // Front-most over the page and drawing nothing but the badge. A
    // passive grab is the only thing that sees the pointer while SplitView
    // has the drag, and being passive it leaves SplitView the drag
    // (measured: the boundary still moved while this reported the hand out
    // past it).
    Item {
        id: splitRefusal
        anchors.fill: parent
        z: 50
        /// Where the split bar's hand is, in scene coordinates, and
        /// whether it is being refused. The other two sources keep their
        /// own; this one is the overlay's because only it can see them.
        property point at: Qt.point(0, 0)
        property bool refuses: false
        PointHandler {
            onPointChanged: page.settleSplitRefusal(point.scenePosition.x,
                                                    point.scenePosition.y)
            // Letting go ends the ask, the same way the graph's dividers
            // end theirs — nothing to remember to take back down.
            onActiveChanged: if (!active) splitRefusal.refuses = false
        }

        // ---- the one badge in the window ----------------------------
        // A refused drag has the hand *outside* the thing it was
        // dragging, so a badge parented to the divider's own pane lands
        // outside its parent and is composited under the page's panes
        // (2026-08-11 ユーザー報告); over the whole page it is above them
        // all. `mapFromItem` is a method and would not re-run on its own,
        // but the point it reads changes on every move of the drag that
        // raised it, which is the only time this is up (app-ui.md).
        RefusalBadge {
            id: refusalBadge
            at: page.refusalAt(page.refusalSource)
            shown: page.refusalSource !== null
        }
    }

    /// Whichever boundary is refusing, or null. One pointer, so the order
    /// only decides which answers in the frame where two could — and two
    /// cannot, since a hand is on one boundary at a time.
    readonly property var refusalSource:
        graphPane.refused ? graphPane.refusedAt
        : detailsPane.descRefuses ? detailsPane.descPoint
        : wipPane.descRefuses ? wipPane.descPoint
        : splitRefusal.refuses ? splitRefusal.at
        : null
    function refusalAt(scene) {
        return scene === null ? Qt.point(0, 0)
                              : splitRefusal.mapFromItem(null, scene.x, scene.y)
    }

    /// What is drawn, not what was asked for — the one badge's own
    /// `shown` (`RefusalBadge`, on why not its `visible`).
    readonly property alias refusalShown: refusalBadge.shown

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
