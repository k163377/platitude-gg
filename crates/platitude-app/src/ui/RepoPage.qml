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
    /// The settings card, opened from an avatar and carrying whom it was
    /// opened on.
    signal avatarSettingsRequested(string name, string email)

    property string selectedOid: ""

    // Left pane folded down to its icons. Held here rather than in the
    // sidebar because two things decide it: the control on its own band,
    // and a diff opening in the middle — reading a file wants every
    // column and every line the window can give it, and the list of refs
    // is the one thing on screen that has nothing to say about the file.
    property bool sidebarCollapsed: false
    /// The fold the diff put on. Closing the diff takes back exactly
    /// that and nothing anybody did by hand: somebody who folded the list
    /// themselves has said they want it folded afterwards too. There is
    /// no "opened it while reading" to remember — that is the one move
    /// that closes the file (`foldByHand`).
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
        // The list and a file never hold the window at once: asking for
        // the list back is how somebody says they are done reading, so
        // the centre goes back to the graph the list speaks about. Every
        // way the list comes back arrives here — the block on the band,
        // a cell on the rail, and the rename box that needs the keyboard
        // (SidebarPane.startEdit) — so the rule has no exceptions to
        // remember.
        if (!collapse && page.diffShown)
            page.closeDiff()
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
            if (page.rebaseRange !== ""
                    && repoTab.publishRange === page.rebaseRange)
                page.rebasePublished = repoTab.publishPublished > 0
            page.absorbHeadMessage()
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
    // Uncommitted changes come along and nothing is asked about them: the
    // move goes ahead carrying them, and where git refuses — changing
    // nothing — core goes round the same way a person would, through a
    // stash (デザイン規約 §未コミット変更がある状態での移動). What lands
    // is whatever that sequence lands: a settled tree, or a conflict to
    // work through, which the file list shows like any other.
    //
    // What the pending move is: kind is "branch" / "remote" / "force"
    // (a local branch moved to `moveStart` before landing on it). Every
    // one of them lands on a branch — nothing here moves onto a bare
    // commit (デザイン規約 §ブランチ・コミットへの移動).
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
                         hold = false, tip = "", form = null) {
        page.rowAskRun = run
        graphPane.startAsking(oidHex, label, detail, acceptText, danger,
                              hold, tip, form)
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

    // The branch move git says is worth asking about arrives on its own
    // counter, because the same move can be asked about twice in a row and
    // only a fresh one may raise the question.
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
    /// Whether there is anything of ours to put on the remote by force.
    /// Nothing to add means nothing to overwrite with.
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
    /// The button's press. A branch that has been somewhere before goes
    /// back there without a word; one that has not raises the question of
    /// where it should go (デザイン規約 §はじめてリモートへ送る) — and the
    /// send that question ends in marks the branch itself, so a refusal
    /// coming back finds the same seat waiting for it.
    function pushNow() {
        if (page.pushState === "publish") {
            page.startPublishAsk()
            return
        }
        page.pushSentBranch = workTree.branch
        repoTab.pushCurrent("", "")
    }
    /// Replace what the remote holds with this branch.
    ///
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
    // Nothing here records a target yet, so this is a question for
    // information rather than for consent (デザイン規約 §はじめてリモートへ送る).
    // It stands in the same bar every question stands in, because three
    // things raise it — this button, a graph row, a REMOTES row — and a
    // question that moves house by who asked it cannot be read.
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
    /// The last row of the chooser: not a remote, but the way to one this
    /// repository does not have yet. It opens the dialog that writes a
    /// remote down — the question stays standing behind it and picks the
    /// new remote up when it lands.
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
    /// Enough of an answer to send anything at all.
    readonly property bool publishFilled:
        page.publishBranch !== "" && page.publishRemote !== ""
    /// The remote has answered for exactly what is typed now.
    readonly property bool publishChecked:
        repoTab.remoteBranchAsked === page.publishRemote
        + String.fromCharCode(31) + page.publishBranch
    /// …and the answer was yes: this name is already over there, so
    /// sending advances a branch somebody else made instead of making one.
    /// git does not refuse that when it fast-forwards (実測), so the hold
    /// is what stands in for the refusal.
    readonly property bool publishTaken:
        page.publishChecked && repoTab.remoteBranchTaken
    /// The remote never answered — unreachable URL, credentials that are
    /// not there, no network. Sending is still allowed (only a push finds
    /// out what a remote really holds) but it takes the hold: what could
    /// not be ruled out is that the name is already over there.
    readonly property bool publishUnsure:
        page.publishChecked && !repoTab.remoteBranchReached

    /// The remote is asked once the typing settles, not per keystroke: it
    /// is a round trip to the network, and the person pressed a button
    /// that reaches it, not one that reaches it per letter.
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
        value: page.publishTaken || page.publishUnsure
        when: page.publishAsking
    }
    Binding {
        target: graphPane
        property: "askNeutral"
        value: !page.publishTaken && !page.publishUnsure
        when: page.publishAsking
    }
    Binding {
        target: graphPane
        property: "askDetail"
        value: !page.publishFilled ? qsTr("Pick where it goes.")
               : !page.publishChecked
                 ? qsTr("Asking %1 what it has…").arg(page.publishRemote)
                 : page.publishUnsure
                   ? qsTr("%1 did not answer — it may already have that branch.")
                     .arg(page.publishRemote)
                   : page.publishTaken
                     ? qsTr("%1 already exists — your commits go on top of it.")
                       .arg(page.publishTarget)
                     : qsTr("%1 does not exist yet; this makes it.")
                       .arg(page.publishTarget)
        when: page.publishAsking
    }
    Binding {
        target: graphPane
        property: "askTip"
        value: page.publishTaken
               ? qsTr("Nobody is asked over there: a branch that can be "
                      + "fast-forwarded simply moves.")
               : page.publishUnsure
                 ? qsTr("The push will say what went wrong; the command log "
                        + "has what was run.")
                 : ""
        when: page.publishAsking
    }

    function startPublishAsk() {
        page.publishRemote = repoTab.defaultRemote
        page.publishBranch = workTree.branch
        page.startRowAsk("", qsTr("Send %1 where?").arg(workTree.branch), "",
                         false, qsTr("Send"), page.answerPublish, false, "",
                         publishForm)
        // After the bar is up, never before: raising it resets the three
        // things the bindings below own, and a binding whose value has not
        // changed does not push back.
        page.publishAsking = true
        page.refreshPublishCheck()
        // No remote at all: the chooser holds nothing but its last row,
        // so the dialog that row opens comes up unasked — the question
        // keeps standing behind it and picks the new remote up when it
        // lands, the same as when the row is clicked
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
    /// Automation: the dialog's own button, once it is up.
    Timer {
        id: publishAddTimer
        interval: 600
        onTriggered: {
            remoteDialog.submit()
            publishAnswerTimer.start()
        }
    }
    /// Automation: the chooser's list, which no injected click can reach.
    function openPublishRemotes() {
        const form = graphPane.askForm
        if (form && form.remotePick)
            form.remotePick.popup.open()
    }
    /// Automation: the answer, given after the remote has had time to say
    /// what it has — the pill is dead until it has.
    Timer {
        id: publishAnswerTimer
        interval: 1200
        onTriggered: {
            AppBackend.report("publish answering taken=" + page.publishTaken
                              + " unsure=" + page.publishUnsure
                              + " answerable=" + graphPane.askAnswerable)
            // The same gesture a person is given: a hold cannot be
            // answered by a click here either.
            if (page.publishTaken || page.publishUnsure)
                graphPane.completeHold()
            else
                page.answerRowAsk()
        }
    }

    function answerPublish() {
        // The same slot a plain push fills: a first push git turns down is
        // still this button's news, and the mark it wears afterwards is
        // the same one (デザイン規約 §リモートへ送る).
        page.pushSentBranch = workTree.branch
        repoTab.publishCurrent(page.publishRemote, page.publishBranch)
    }

    // Writing a remote down, and correcting one. The question that sent us
    // here keeps standing: the new remote lands in the chooser and is
    // picked, so the answer carries on where it left off.
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

    /// The controls the question needs: which remote, and what the branch
    /// is called over there. One row — the words above it say what is
    /// being asked, so the boxes need no labels of their own.
    Component {
        id: publishForm
        ColumnLayout {
            spacing: Theme.spaceXs
            /// Automation only: the list cannot be opened by an injected
            /// click on the offscreen platform.
            property alias remotePick: remotePick
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            // Every remote this repository has, with the way to one it
            // does not on the end of the same list. A list rather than a
            // row of chips: a fork-and-upstream working copy has several,
            // and chips grow sideways until they push the name out of the
            // pane, while a list stays one control wide however many there
            // are (デザイン規約 §はじめてリモートへ送る).
            AppCombo {
                id: remotePick
                pickOnly: true
                // The fixed-input width every boxed field shares
                // (デザイン規約 §レイアウト初期値 160) — not a width of
                // its own.
                Layout.preferredWidth: 160
                model: page.publishChoices
                wanted: page.publishRemote
                onActivated: index => page.choosePublishRemote(index)
            }
            // Never a placeholder: the branch's own name is the answer
            // unless somebody changes it, and an empty box would read as
            // though there were nothing to send.
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
            // The controls sit together at the left; the slack belongs to
            // the row, not between the chips and the name.
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
    /// The branch git has just refused to delete, if the menu that asked
    /// is still standing. Set only while its row is on screen to carry the
    /// answer; a fresh menu starts with nothing refused.
    property string forceDeleteBranch: ""
    /// What this menu offers, worked out as it opens and left alone while
    /// it stands — the same way the discard row's plan is (`discardPlan`).
    /// The conditions behind them are live: a fetch on the timer alone
    /// moves `busyCount`, and a refresh can land while the card is up. A
    /// row that appears or vanishes under the pointer is a row clicked by
    /// accident (デザイン規約 §メニュー).
    property bool menuCanSwitch: false
    property bool menuCanIntegrateFrom: false
    property bool menuCanDelete: false
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
        // The branch the working tree is on cannot be deleted at all, and
        // git says so rather than doing something else.
        page.menuCanDelete = repoTab.busyCount === 0
                             && !(kind === "branch" && full === workTree.branch)
        if (repoTab.state === "open" && page.rebaseRange !== "")
            repoTab.checkPublish(page.rebaseRange)
        refMenu.offer()
    }

    // ---- bringing two lines of history together --------------------
    /// Whether the current branch can take a merge or a rebase from the
    /// row the menu is on: a branch to land on, nothing already stepping,
    /// and somewhere other than itself to come from.
    readonly property bool canIntegrateFrom:
        repoTab.state === "open" && repoTab.busyCount === 0
        && !workTree.detached && workTree.branch !== ""
        && workTree.opText === ""
        && page.menuRefId !== "" && page.menuRefId !== workTree.branch
    /// The commits a rebase onto this row would rewrite. Asked about as
    /// the menu opens, because the answer is a whole git call away and
    /// the row wants to say it the moment it is read.
    readonly property string rebaseRange:
        (page.menuRefKind === "branch" || page.menuRefKind === "remote")
        && page.menuRefId !== "" ? page.menuRefId + "..HEAD" : ""
    property bool rebasePublished: false
    AppMenu {
        id: refMenu
        // Walking away from a refused delete takes the offer with it.
        onClosed: page.forceDeleteBranch = ""
        AppMenuItem {
            // The verb alone: this menu was opened on the row it means,
            // and the row is already showing that name (デザイン規約
            // §メニュー). `Delete` below it has always read this way.
            text: qsTr("Switch")
            offered: page.menuCanSwitch
            // Through the same dispatcher the graph's chips use: a remote
            // branch whose local one already exists cannot simply be
            // created, and that answer belongs in one place.
            onTriggered: page.switchToRef(page.menuRefKind === "remote" ? "R" : "L",
                                          page.menuRefId)
        }
        // Bringing this row's line of history together with the one the
        // working tree is on. The current branch is the subject of both
        // sentences — it is what changes — and the row is where the
        // commits come from or land on (デザイン規約 §履歴を合流させる).
        //
        // `merge` and `rebase` are git's own words taken as they are, so
        // they are said in git's spelling on a chip, the way `cherry-pick`
        // and the reset flags are (§git 用語のコード表記).
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
            // Onto a tag as well would be a rebase onto a fixed point,
            // which is a thing to do — but the row that says it belongs
            // with the tag's own gestures, not squeezed in here.
            offered: (page.menuRefKind === "branch"
                      || page.menuRefKind === "remote")
                     && page.menuCanIntegrateFrom
            // Said, not asked (要望: rewriting a pushed commit shows a
            // warning): the rebase goes ahead, and this tag is the
            // warning. The count is the answer to this row's own range —
            // published means reachable from a remote-tracking ref, which
            // is only ever as fresh as the last fetch.
            note: page.rebasePublished ? qsTr("rewrites pushed commits") : ""
            onTriggered: repoTab.rebase(page.menuRefId, "", true, true)
        }
        AppMenuSeparator {}
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
            offered: (page.menuRefKind === "branch" || heldRow)
                     && page.menuCanDelete
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
        // The one row in any menu that outlives its own write (it stays
        // open for git's answer), so it is also the one that can be
        // clicked twice — the second click is the same request again.
        if (repoTab.busyCount > 0)
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
    /// Whether this menu's writing rows were on offer as it opened, held
    /// still for as long as it stands (`menuCanSwitch` and the rest).
    property bool menuFileCanWrite: false
    /// How many files the rows here would touch, counted the way the
    /// writes count them (`chosenRows()` skips folder rows, which nothing
    /// in this menu acts on). Not `chosenCount` — that counts chosen
    /// keys, so a folder in the choice would put a file on the tag that
    /// no command is going to reach.
    property int menuFileCount: 0
    function openFileMenu(bucket, path, origPath) {
        // A right-click is a click: it walks away from a question that
        // was standing, which may well be about another row.
        page.stopRowAsk()
        page.menuFileBucket = bucket
        page.menuFilePath = path
        page.menuFileOrig = origPath === undefined ? "" : origPath
        page.menuFileCanWrite = repoTab.busyCount === 0
        // What the discard row would do, worked out once here: the choice
        // cannot change while the menu is up, so the words the row says
        // and the writes it runs are read off the same plan.
        page.discardPlan = page.planDiscard()
        page.menuFileCount = wipPane.chosenRows().length
        fileMenu.offer()
        // Which rows the menu offers follows from the bucket, and an
        // absent row is not something a screenshot can be trusted on —
        // least of all now that a row nobody can choose leaves no trace
        // at all.
        if (AppBackend.autoAct !== "")
            AppBackend.report("file_menu bucket=" + bucket
                              + " rows=" + fileMenu.offeredRows)
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
    /// Opens the highlighted conflicted rows in the configured tool, or
    /// goes to settings when there is no tool to open them with.
    ///
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
        // `Edit in`, not `Open in`: the two rows above say what the file
        // ends up as, so this one keeps the same mood rather than
        // switching to what a window does — and `Open` is already this
        // app's word for bringing in a repository or a tab.
        //
        // The external tool shares its seat with the way to pick one:
        // with nothing configured there is nothing to open, so the row
        // becomes the door to the setting instead (`…` = a question
        // stands). Changing the tool afterwards lives in settings rather
        // than in a second row here — the menu is the one surface where
        // an extra row costs every reader (P3-確認事項 §B).
        //
        // The name is left in plain type. A code chip would be the second
        // in this menu, and `stash` two rows below wears one to say "this
        // is a git command" — a tool name is a config value, and one mark
        // cannot carry both (規約 §git 用語のコード表記).
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
            // The rows this menu was opened on are the ones lit in the
            // list, so how many there are is the only thing left to say —
            // and `Discard` below already says it on the tag rather than
            // in the words (デザイン規約 §メニュー).
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
        // One row on every kind of file, and the row says what it takes
        // away — held rather than asked about, the way the stash and the
        // remote branch above it are (デザイン規約 §長押し). On a file
        // changed on both sides the two rows are the choice itself: the
        // unstaged one keeps what is staged, the staged one takes the lot.
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
            // Plural by what will land on the clipboard, not by how many
            // rows are lit (`menuFileCount`).
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
    // not asked about — nothing here leaves the machine — but 要望.md
    // wants it said, so the squash row carries a tag the way the amend
    // editor does. The answer lands a frame after the menu opens.
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

    // The one thing a stash row has nowhere else: the click that opens
    // this menu also selects the row, and the details pane it fills in
    // carries Apply and Pop (デザイン規約 §メニュー).
    AppMenu {
        id: stashMenu
        AppMenuItem {
            text: qsTr("Apply")
            offered: page.menuStashCanWrite
            onTriggered: repoTab.applyStash(page.menuStashRef)
        }
        AppMenuItem {
            text: qsTr("Pop")
            offered: page.menuStashCanWrite
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
        // The other half of the same pair: one copies the commit here,
        // the other undoes it here. Both add a commit rather than
        // rewriting one, so neither is asked about or held.
        AppMenuItem {
            code: "revert"
            offered: page.menuCanSequence
            onTriggered: repoTab.revert(page.menuOid)
        }
        AppMenuSeparator {}
        // The same two the sidebar's rows carry, reaching a commit that
        // may have no name at all (デザイン規約 §履歴を合流させる). The
        // words say "here" rather than naming the row, the way the reset
        // submenu does — the row is what was clicked.
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
            onTriggered: repoTab.rebase(page.menuOid, "", true, true)
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
            code: "squash"
            // Which commit is folded is not said, the way the reset
            // submenu says "here": the row this menu was opened on is
            // the one that was pointed at. What has to be named is the
            // other end, and the details pane the same click fills in
            // already calls it the parent (its `←` link).
            //: Follows the `squash` chip: "squash into parent".
            text: qsTr("into parent")
            // Said, not asked (要望: rewriting a pushed commit shows a
            // warning): the squash goes ahead, and this tag is the warning.
            note: page.menuPublished ? qsTr("already pushed") : ""
            offered: page.menuCanEditHistory
            onTriggered: page.squashCommit(page.menuOid)
        }
        // The commit stops being part of the history, and what came after
        // it is replayed over the gap. Every other row here either adds a
        // commit or moves one; this is the only one that takes a commit
        // away, so it is the only one that can leave the old chain with
        // nothing but the reflog reaching it — and the mark follows that
        // rather than the row (デザイン規約 §長押し). Held while this
        // branch is the only thing holding its tip; a plain click once
        // something else does, because then the replaced commits stay
        // drawn and a cherry-pick brings any of them back.
        //
        // The answer is a property of the branch, not of the row, so it
        // is already in hand when the menu opens: a mark that appeared a
        // moment later would re-indent every row in the menu
        // (`AppMenu.holdColW`) with the hand already on its way.
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
        // Three ways to take the branch back to this commit, told apart
        // by what becomes of the work they skip over. The command is
        // said once, as the title row's own verb (`reset` main here, the
        // way `stash` this file reads), and each row leads with just its
        // flag as a code chip (デザイン規約 §git 用語のコード表記) — the
        // hand that knows `reset --soft` finds its row at a glance and
        // the eye that does not reads the sentence alone. A submenu
        // keeps the choice out of the way until it is asked for; where
        // there is no branch to move, the whole submenu goes and takes
        // the row that opens it with it (`AppMenu.applies`).
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

    /// Leaves the commit out of the history. One place for both ways in:
    /// the row is a hold or a click depending on whether anything else
    /// still holds the branch tip, and what it runs must not depend on
    /// which of the two the reader got (デザイン規約 §履歴を合流させる).
    function dropCommit(oidHex) {
        repoTab.dropCommit(oidHex)
    }

    // ---- taking the branch back to an earlier commit ----------------
    // git calls this a reset; the menu says what it does, which is move
    // the branch. Offered only where there is a branch to move and
    // somewhere to move it to: detached HEAD has none, a stash sits on
    // no branch's history, an operation in progress is left through
    // Continue / Abort instead (mid-merge a soft reset refuses outright
    // and the other two abandon the merge without a word), and the
    // commit the branch already stands on is not a move at all.
    /// Rewriting this commit's place in the history. Unlike the two
    /// above it, the newest commit is fair game — that is the one a fold
    /// or a drop most often means.
    readonly property bool canEditHistoryHere:
        repoTab.state === "open" && repoTab.busyCount === 0
        && !workTree.detached && workTree.branch !== ""
        && workTree.opText === ""
        && page.menuOid !== "" && page.menuStashRef === ""
    /// The commit-menu twin of `canIntegrateFrom`: a branch to land on,
    /// nothing already stepping, and a commit other than the one the
    /// working tree is already sitting on.
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
            // The hunk heading with the pointer on it: its two words carry
            // their own colours only there (デザイン規約 §diff の中の
            // ステージ), and hover cannot be injected, so the row is named
            // instead. A heading's own row is line -1 (`flatten_patches`).
            if (act === "hunk-tools") {
                diffPane.showLineTools(0, -1)
                return
            }
            // Lines picked by hand, and the write that takes the lot. The
            // argument says how many to pick (two by default), which is
            // what makes the heading name a count.
            // Reading part way down a long diff and then writing: the
            // rebuild has to come back to the same place, or a file with
            // any length to it throws the reader to the top on every
            // partial stage. `diff_place` is reported by the restore.
            if (act === "keep-place") {
                diffPane.scrollTo(400)
                page.stageSelection(0, diffPane.firstChangedLine(0))
                return
            }
            if (act === "pick-lines" || act === "stage-lines") {
                // The path came in as the argument, so how many lines to
                // pick is not something this verb can be told: two is what
                // makes a heading say a count rather than a hunk.
                const want = 2
                AppBackend.report("picked_lines "
                                  + diffPane.chooseLines(0, want))
                if (act === "stage-lines")
                    page.stageChosenLines()
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
            // Where the open section stands. `cell` is the top edge of the
            // mark that opened it and `top` where the panel begins — they
            // are the same number or the list has walked away from its own
            // cell, which is what a section too tall for the pane used to
            // do. `end` against `pane` is the other half: it grows down
            // into the pane and stops at the foot of it.
            + " top=" + Math.round(sidebarPane.peekY)
            + " cell=" + Math.round(sidebarPane.peekTop)
            + " end=" + Math.round(sidebarPane.peekBottom)
            + " pane=" + Math.round(sidebarPane.height)
            + " editing=" + sidebarPane.editKey
            // What the centre holds: the list coming back closes a file,
            // so the two are read together or not at all.
            + " diff=" + page.diffShown)
    }
    // The column has to be laid out again before the header that was
    // closed can say where it ended up.
    Timer {
        id: navSectionTimer
        interval: 400
        onTriggered: AppBackend.report(
            "nav_section closed=" + AppBackend.autoActArg
            + " header=" + Math.round(
                sidebarPane.headerTopOf(AppBackend.autoActArg))
            + " ground=" + Math.round(sidebarPane.groundTop)
            + " pane=" + Math.round(sidebarPane.height))
    }
    Timer {
        id: refusedRowTimer
        interval: 800
        onTriggered: AppBackend.report("ref_menu delete=" + refDeleteItem.text
                                       + " note=" + refDeleteItem.note)
    }
    // The lane column has to have taken its narrower width before there
    // is anywhere to pan to, or a bar worth wanting.
    Timer {
        id: graphPanTimer
        interval: 400
        onTriggered: {
            // Where the pointer is, which is the whole of what puts the
            // bar on screen. `-away` walks it back out again: a bar that
            // comes when the pointer does proves nothing on its own
            // unless it also goes when the pointer goes.
            graphPane.restPointer(true)
            if (AppBackend.autoAct !== "middle-scroll") {
                if (AppBackend.autoAct === "graph-bar-away")
                    graphPane.restPointer(false)
                AppBackend.report(
                    "graph_bar shown=" + graphPane.laneBarShown
                    + " overflow=" + Math.round(graphPane.graphXMax))
                return
            }
            // The middle click, then the pointer drifting sideways off
            // it. The argument says which column the click landed in,
            // which is the whole question — only the lanes take the
            // sideways drift (デザイン規約 §グラフを横へ送る).
            const y = graphPane.height / 2
            const x = AppBackend.autoActArg === "message"
                    ? graphPane.labelW + graphPane.graphColW + Theme.spaceXl
                    : graphPane.labelW + Theme.spaceSm
            graphPane.startAutoScroll(x, y)
            graphPane.driftPointer(x + graphPane.width, y)
            middleScrollTimer.start()
        }
    }
    // Long enough for the 16ms ticker to have taken the lanes as far as
    // they go: a pan that ran and a pan that was refused must not read
    // alike in the report.
    Timer {
        id: middleScrollTimer
        interval: 400
        onTriggered: AppBackend.report(
            "middle_scroll lanes=" + graphPane.autoPanning
            + " x=" + Math.round(graphPane.graphX)
            + " max=" + Math.round(graphPane.graphXMax))
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
    // The refusal has to be back and on the button before the second go
    // is sent, and the report is what says it ever got there — the mark
    // is gone again by the time the screenshot is taken.
    Timer {
        id: pushRetryTimer
        interval: 1800
        onTriggered: {
            AppBackend.report("push_retry refused=" + page.pushFailed
                              + " branch=" + page.pushFailBranch)
            page.forcePush()
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
    // Automation: the details have to land before the author card can be
    // worked, since it is that author the picture is filed against.
    Timer {
        id: avatarAssignTimer
        interval: 400
        onTriggered: {
            AppBackend.assignAvatar(detailsModel.authorEmail,
                                    detailsModel.authorName,
                                    AppBackend.autoActArg)
            avatarReportTimer.start()
        }
    }
    Timer {
        id: avatarBadgeTimer
        interval: 400
        onTriggered: detailsPane.avatarClicked()
    }
    // The store starts empty in every run, so the card's own verbs put a
    // picture in it before opening on it.
    Timer {
        id: avatarSeedTimer
        interval: 400
        onTriggered: {
            AppBackend.assignAvatar(detailsModel.authorEmail,
                                    detailsModel.authorName,
                                    AppBackend.autoActArg)
            page.settingsDialogRequested()
        }
    }
    // The picture is read off disk asynchronously, so what the shot wants
    // is a beat after the write rather than the instant it returns.
    Timer {
        id: avatarReportTimer
        interval: 600
        onTriggered: AppBackend.report(
            "avatar email=" + detailsModel.authorEmail
            + " details=" + (detailsModel.avatarUrl !== "")
            + " rows=" + graphModel.avatarRowCount()
            + " error=" + AppBackend.avatarError)
    }
    // The details have to arrive before the credit line they carry can
    // be opened or counted.
    Timer {
        id: coAuthorTimer
        interval: 800
        onTriggered: {
            if (AppBackend.autoAct === "co-authors-open")
                detailsPane.showCoAuthors(true)
            // `open` is the card's own visibility, not the input that
            // asked for it: reporting the input would go green with the
            // binding cut.
            AppBackend.report(
                "co_authors count=" + detailsPane.coAuthorRecords.length
                + " first=" + detailsPane.coAuthorName(0)
                + " open=" + detailsPane.matesCardOpen)
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
        if (act === "publish" || act === "publish-taken"
                || act === "publish-add" || act === "publish-go"
                || act === "publish-new-go" || act === "publish-remotes") {
            // The button's own path, so the state machine in front of the
            // question is exercised too, not just the question.
            page.pushNow()
            if (act === "publish-taken")
                page.setPublishBranch(arg === "" ? "taken" : arg)
            else if (act === "publish-add" || act === "publish-new-go")
                page.startPublishAddRemote(arg)
            else if (arg !== "")
                page.setPublishBranch(arg)
            if (act === "publish-new-go")
                publishAddTimer.start()
            else if (act === "publish-go")
                publishAnswerTimer.start()
            else if (act === "publish-remotes")
                page.openPublishRemotes()
            // `dialog=` / `name=` say whether the remote dialog stands and
            // what its name box holds — the no-remote push opens it by
            // itself, and only this line can say so headless.
            AppBackend.report("publish state=" + page.pushState
                              + " remote=" + page.publishRemote
                              + " branch=" + page.publishBranch
                              + " dialog=" + remoteDialog.visible
                              + " name=" + remoteDialog.wantedName)
        } else if (act === "commit") {
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
        } else if (act === "stage-many" || act === "stage-many-go") {
            // The same two rows the discard verbs choose, and then the
            // pointer put on the first one's own mark: the marks of every
            // row that would move with it come out together. "-go" presses
            // it, so the shot after is what one press moved.
            page.showWip()
            const head = wipPane.rowAt(0)
            if (head)
                wipPane.chooseOnly(head.bucket, head.fullName)
            const mate = wipPane.rowFor(arg)
            if (mate)
                wipPane.applyClick(mate.bucket, mate.fullName, Qt.ControlModifier)
            AppBackend.report("chosen count=" + wipPane.chosenCount)
            if (head) {
                wipPane.showStageTools(head.bucket, head.fullName)
                if (act.endsWith("-go")) {
                    const row = wipPane.rowAt(0)
                    if (row)
                        row.stageClicked(head.bucket, head.fullName)
                }
            }
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
        } else if (act === "open-mergetool") {
            // Same route as the row above. With a tool configured this
            // holds the write queue until it exits, so a demo tool that
            // blocks is what leaves the pane's wait on screen.
            page.showWip()
            wipPane.chooseOnly("conflicts", arg)
            page.openFileMenu("conflicts", arg, "")
            fileMenu.close()
            page.openInMergeTool()
            AppBackend.report("merge_tool " + wipPane.workTree.mergeTool)
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
                   || act === "nav-unfold" || act === "nav-peek-rename"
                   || act === "nav-peek-away" || act === "nav-peek-into"
                   || act === "nav-peek-out" || act === "nav-peek-shut") {
            // The left menu folded to its icons, and one of them rested
            // on. The resting cannot be injected (hover never can), so
            // the section is named the way the diff's line tools are.
            // "nav-unfold" walks the whole way back, which is the one
            // thing folding has to be able to do.
            // "nav-fold no-tags" takes the tags off the graph first, so
            // the mark the rail wears for that can be photographed.
            // "-away" walks the pointer off the cell, "-into" walks it off
            // the cell down into the list it opened, "-out" carries on out
            // of the list the other way (the exit no cell can see) and
            // "-shut" clicks the cell. Only "-into" leaves the section
            // standing; the other three close it, and the click has to
            // leave the list folded (`collapsed=`). "nav-peek" on a
            // section with nothing in it must not open at all — the same
            // verb answers both, because the cell decides
            // (NavRail.enterAt), so point it at an empty section
            // (`--preset empty`) to read that side.
            if (arg === "no-tags")
                repoTab.setTagsShown(false)
            page.foldByHand(true)
            if (act === "nav-peek")
                sidebarPane.peekAt(arg)
            else if (act === "nav-peek-away") {
                sidebarPane.peekAt(arg)
                sidebarPane.peekAway(arg)
            } else if (act === "nav-peek-into" || act === "nav-peek-out") {
                sidebarPane.peekAt(arg)
                sidebarPane.peekInto(arg)
                if (act === "nav-peek-out")
                    sidebarPane.peekOut()
            } else if (act === "nav-peek-shut") {
                sidebarPane.peekAt(arg)
                sidebarPane.peekTap(arg)
            } else if (act === "nav-unfold")
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
        } else if (act === "nav-close") {
            // One section closed from its header band. The pane keeps
            // the sections packed against the top whichever of them are
            // closed, so what is read afterwards is where the header of
            // the closed one came to rest — at the foot of the pane is
            // the failure this watches for.
            sidebarPane.closeSection(arg)
            navSectionTimer.start()
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
            // it offers is said in words too, since a row that cannot be
            // chosen leaves no trace in the picture at all.
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
        } else if (act === "co-authors" || act === "co-authors-open") {
            // The credit line on the date row, and the card the pointer
            // opens under it. Hover cannot be injected, so `-open` writes
            // the same property the handler writes; the two are read as a
            // pair, because "the card stayed shut" only means something
            // next to a run where it opened. The argument is the row.
            page.activateRow(graphModel.oidAt(Number(arg)))
            coAuthorTimer.start()
        } else if (act === "name-box") {
            // Opened and left standing, for a look at it. The argument is
            // the row, since the box only belongs on one with no chips.
            graphPane.startNaming(graphModel.oidAt(Number(arg)))
        } else if (act === "graph-bar" || act === "graph-bar-away"
                   || act === "middle-scroll") {
            // Both want lanes that do not fit their column, and no demo
            // repository has that many (the column starts wide enough for
            // `graphDefaultLanes`). Pulling the divider in is what a
            // person does, so the state these read is a real one.
            page.setGraphColumns(graphPane.labelWManual,
                                 Metrics.laneInset + 2 * Metrics.laneW)
            graphPanTimer.start()
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
            resetMenu.offer()
            if (act === "reset-hard")
                hardResetItem.completeHold()
        } else if (act === "commit-menu" || act === "reset-menu") {
            // Nothing written: the menu is left standing for the overlay
            // shot. Which rows are offered is said in words as well — a
            // row that cannot be chosen is not in the picture at all.
            //
            // With no row named, the row under HEAD's: most of this menu
            // is about a commit the branch is *not* already standing on,
            // so HEAD's own row would leave half of it out. Counted from
            // where HEAD actually sits rather than from the top — the
            // rows above it belong to whatever else the graph is showing
            // (in the demo repository, a remote that is ahead).
            let menuOid = arg
            if (menuOid === "")
                menuOid = graphModel.oidAt(
                    graphModel.rowOf(workTree.headOid) + 1)
            page.openRowMenu(menuOid)
            if (act === "reset-menu")
                resetMenu.offer()
            AppBackend.report("commit_menu rows=" + commitMenu.offeredRows
                              + " can_move=" + page.menuCanMoveBranch)
        } else if (act === "wip") {
            // The working tree, as the row above the newest commit opens
            // it: the file list this pane's every other verb starts from.
            page.showWip()
        } else if (act === "drop-commit" || act === "drop-commit-go") {
            // Through the graph row's menu, where the row is held or
            // clicked depending on what still holds the branch tip: the
            // plain verb leaves the menu standing for the shot and "-go"
            // takes whichever of the two the row is offering. The plan is
            // built by object name, the way a graph row hands one over,
            // so a symbolic name is not what this takes.
            page.openRowMenu(arg !== "" ? arg : branchesModel.headOid)
            AppBackend.report("drop_row " + dropCommitItem.code
                              + " " + dropCommitItem.text
                              + " hold=" + (dropCommitItem.holdMs > 0)
                              + " reached=" + repoTab.headReachedElsewhere)
            if (act === "drop-commit-go") {
                if (dropCommitItem.holdMs > 0)
                    dropCommitItem.completeHold()
                else
                    page.dropCommit(page.menuOid)
            }
        } else if (act === "merge-branch" || act === "rebase-onto"
                   || act === "revert-commit" || act === "integrate-menu") {
            // Through the menus a right-click opens, so the rows' own
            // gating decides whether anything runs. "integrate-menu"
            // leaves the ref menu standing for a shot instead.
            if (act === "revert-commit") {
                page.openRowMenu(arg)
                repoTab.revert(arg)
            } else {
                page.openRefMenu("branch", arg, arg,
                                 branchesModel.oidOfName(arg))
                if (act === "merge-branch")
                    repoTab.merge(arg, false, false, "")
                else if (act === "rebase-onto")
                    repoTab.rebase(arg, "", true, true)
            }
        } else if (act === "op-exit" || act === "op-exit-go") {
            // The ways out of a stopped operation, which stand in the pane
            // under the commit button rather than dropping from a click.
            // "-go" runs the held row the argument names to its end.
            page.showWip()
            if (act === "op-exit-go")
                AppBackend.report("op_exit_held " + wipPane.completeOpExit(arg))
        } else if (act === "stage-hunk" || act === "stage-line"
                   || act === "discard-hunk" || act === "discard-hunk-go"
                   || act === "diff-file" || act === "line-tools"
                   || act === "hunk-tools" || act === "pick-lines"
                   || act === "stage-lines" || act === "keep-place") {
            // All of them enter through the diff of one file and act on
            // its first hunk: "diff-file" only opens it, "line-tools" puts
            // out the square a line shows under the pointer, and
            // "hunk-tools" wakes the heading's two words. "discard-hunk"
            // leaves the held button standing for the shot and "-go" holds
            // it to its end. The working tree comes up first, since its
            // file list is where a diff is reached from.
            //
            // The bucket rides in front of the path (`<bucket>:<path>`, the
            // form `nav-dbl` uses) when it is not the usual unstaged one: a
            // file the repository has never seen has no unstaged diff at
            // all — it is read from `untracked`, and a file git stopped on
            // from `conflicts` (which is where its words differ). Only the
            // bucket names count as one, so a path carrying a colon still
            // opens.
            const cut = arg.indexOf(":")
            const head = cut > 0 ? arg.substring(0, cut) : ""
            const named = head === "staged" || head === "unstaged"
                          || head === "untracked" || head === "conflicts"
            page.showWip()
            page.toggleDiff(named ? head : "unstaged",
                            named ? arg.substring(cut + 1) : arg, "")
            stageRowTimer.start()
        } else if (act === "diff-fold" || act === "diff-unfold"
                   || act === "diff-fold-by-hand"
                   || act === "diff-fold-by-rename"
                   || act === "diff-keep-folded") {
            // What opening a file does to the left menu on its own, and
            // what each way back does to the file. "-by-hand" asks for
            // the list back with the block on the band; "-by-rename" gets
            // it back as the side effect of typing a name into a peeked
            // row — both take the diff down with them. "-keep-folded" had
            // it folded before the diff arrived, so closing the diff
            // leaves it folded.
            page.showWip()
            if (act === "diff-keep-folded")
                page.foldByHand(true)
            page.toggleDiff("unstaged", arg, "")
            if (act === "diff-fold-by-hand")
                page.foldByHand(false)
            else if (act === "diff-fold-by-rename") {
                sidebarPane.peekAt("branch")
                sidebarPane.beginRename("branch", workTree.branch,
                                        workTree.branch)
            } else if (act !== "diff-fold")
                page.closeDiff()
            navRailTimer.start()
        } else if (act === "push") {
            page.pushNow()
        } else if (act === "force-push") {
            page.forcePush()
        } else if (act === "push-retry") {
            // One that cannot land, then one that can. What the picture
            // cannot hold is the second half — a mark coming off is the
            // absence of a thing — so the timer reports the state the
            // refusal left before sending the go that clears it.
            page.pushNow()
            pushRetryTimer.start()
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
        } else if (act === "open-picker") {
            // The window owns the dialog, so this asks for it the way the
            // toolbar row does; the folder it opens at is reported there.
            page.openRepositoryPicker()
        } else if (act === "settings" || act === "settings-tools") {
            // `-tools` goes on to open the candidate list from inside the
            // dialog, and leaves the fetch interval where it was.
            page.settingsDialogRequested()
            if (act === "settings")
                AppBackend.setAutoFetchMinutes(Number(arg))
        } else if (act === "avatar-rest" || act === "avatar-hover"
                   || act === "avatar-assign" || act === "avatar-badge") {
            // Select the first ordinary commit (row 0 is WIP), then work
            // its author card. `PG_AUTO_ACT_ARG` is the picture to file
            // where one is being filed.
            page.activateRow(graphModel.oidAt(1))
            if (act === "avatar-hover" || act === "avatar-assign")
                detailsPane.avatarPointedAt = true
            if (act === "avatar-assign")
                avatarAssignTimer.start()
            if (act === "avatar-badge")
                avatarBadgeTimer.start()
        } else if (act === "avatar-settings" || act === "avatar-combo"
                   || act === "avatar-row-lit" || act === "avatar-remove") {
            // The card on its own, rather than reached from a face. Each
            // run starts with an empty store, so a picture to look at has
            // to be filed first — the argument is the one to file.
            page.activateRow(graphModel.oidAt(1))
            if (arg !== "")
                avatarSeedTimer.start()
            else
                page.settingsDialogRequested()
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
            page.commandsOpen ? commandsPane.height
                              : commandsPane.SplitView.preferredHeight,
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
