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
    // something other people may have, so that gets confirmed.
    property bool headPublished: false
    readonly property string headRange: "HEAD^!"
    function refreshHeadPublished() {
        if (repoTab.state === "open")
            repoTab.checkPublish(page.headRange)
    }
    Connections {
        target: repoTab
        function onChanged() {
            // One shared answer slot, so each consumer only reads the
            // reply to the range it asked about.
            if (repoTab.publishRange === page.headRange) {
                page.headPublished = repoTab.publishPublished > 0
            } else if (page.menuOid !== ""
                       && repoTab.publishRange === page.menuOid + "^!") {
                page.menuPublished = repoTab.publishPublished > 0
                page.menuPublishKnown = true
            }
            page.absorbHeadMessage()
            page.absorbWriteResult()
        }
    }

    // Turning amend on starts the editor from HEAD's message; turning
    // it off empties it again, since the text belonged to that commit.
    property int seenHeadMessageSeq: 0
    property bool wantHeadMessage: false
    function amendToggled(on) {
        page.amending = on
        if (on) {
            page.wantHeadMessage = true
            repoTab.requestHeadMessage()
        } else {
            page.clearCommitEditor()
        }
    }
    function absorbHeadMessage() {
        if (repoTab.headMessageSeq === page.seenHeadMessageSeq)
            return
        page.seenHeadMessageSeq = repoTab.headMessageSeq
        if (!page.wantHeadMessage)
            return
        page.wantHeadMessage = false
        wipPane.setMessage(repoTab.headSubject, repoTab.headBody)
    }
    function clearCommitEditor() {
        wipPane.clearMessage()
    }

    function commitNow() {
        if (page.amending && page.headPublished) {
            page.confirmRequested(
                qsTr("Rewrite a commit that is already on a remote?"),
                qsTr("The last commit has been pushed. Amending replaces it "
                     + "with a different one, so anyone who already has it "
                     + "will be out of step until they reset."),
                qsTr("Amend anyway"), page.doCommit)
            return
        }
        page.doCommit()
    }
    function doCommit() {
        repoTab.commit(wipPane.subjectText, wipPane.bodyText, page.amending)
    }

    // ---- moving between branches and commits ----------------------
    // Terminology is deliberate: git runs `switch` / `restore`, and
    // the UI says "Switch to" (デザイン規約 §用語).
    readonly property bool treeDirty: workTree.stagedCount > 0
                                      || workTree.unstagedCount > 0
                                      || workTree.untrackedCount > 0
    // What the pending move is: kind is "branch" / "remote" / "commit".
    property string moveKind: ""
    property string moveTarget: ""
    property string moveLabel: ""

    function switchTo(kind, target, label) {
        page.moveKind = kind
        page.moveTarget = target
        page.moveLabel = label
        if (page.treeDirty)
            dirtySwitchDialog.open()
        else
            page.runSwitch(false)
    }
    function runSwitch(stashFirst) {
        if (page.moveKind === "branch")
            repoTab.checkoutBranch(page.moveTarget, stashFirst)
        else if (page.moveKind === "remote")
            repoTab.checkoutRemote(page.moveTarget,
                                   repoTab.localNameFor(page.moveTarget), stashFirst)
        else if (page.moveKind === "commit")
            repoTab.checkoutDetached(page.moveTarget, stashFirst)
        page.moveKind = ""
    }

    DirtySwitchDialog {
        id: dirtySwitchDialog
        moveLabel: page.moveLabel
        stayLabel: workTree.detached ? qsTr("this commit") : workTree.branch
        onResolved: stashFirst => page.runSwitch(stashFirst)
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
    Menu {
        id: refMenu
        MenuItem {
            text: qsTr("Switch to %1").arg(page.menuRefName)
            enabled: page.menuRefName !== workTree.branch
            onTriggered: page.switchTo(page.menuRefRemote ? "remote" : "branch",
                                       page.menuRefName, page.menuRefName)
        }
        MenuSeparator {}
        MenuItem {
            text: qsTr("Copy commit hash")
            onTriggered: clipboard.copy(page.menuRefOid)
        }
    }

    // ---- context menu on a commit row ------------------------------
    property string menuOid: ""
    readonly property string menuShort: page.menuOid.substring(0, 8)
    function openCommitMenu(oidHex) {
        page.menuOid = oidHex
        // Asked as the menu opens so the rewrite warnings inside it
        // know whether this commit has already left the machine. The
        // rewriting entries stay disabled until the answer lands —
        // one `rev-list --count`, so within a frame or two.
        page.menuPublished = false
        page.menuPublishKnown = false
        repoTab.checkPublish(oidHex + "^!")
        commitMenu.popup()
    }
    // Whether the commit the menu is about is already on a remote.
    property bool menuPublished: false
    property bool menuPublishKnown: false

    Menu {
        id: commitMenu
        MenuItem {
            text: qsTr("Copy this commit onto the current branch")
            enabled: repoTab.busyCount === 0
            onTriggered: repoTab.cherryPick(page.menuOid)
        }
        MenuItem {
            text: qsTr("Switch to this commit")
            enabled: repoTab.busyCount === 0
            onTriggered: page.switchTo("commit", page.menuOid, page.menuShort)
        }
        MenuSeparator {}
        MenuItem {
            text: qsTr("Edit message…")
            enabled: repoTab.busyCount === 0 && page.menuPublishKnown
            onTriggered: page.editMessage(page.menuOid)
        }
        MenuItem {
            text: qsTr("Fold into the commit before it")
            enabled: repoTab.busyCount === 0 && page.menuPublishKnown
            onTriggered: page.squashCommit(page.menuOid)
        }
        MenuSeparator {}
        MenuItem {
            text: qsTr("Copy commit hash")
            onTriggered: clipboard.copy(page.menuOid)
        }
    }

    ClipboardHelper {
        id: clipboard
    }

    // ---- rewriting one commit --------------------------------------
    // Both of these replay history when the commit is not the newest
    // one, so both warn once the commit has been pushed.
    function rewriteWarning(action, run) {
        if (!page.menuPublished) {
            run()
            return
        }
        page.confirmRequested(
            qsTr("Rewrite a commit that is already on a remote?"),
            qsTr("%1 has been pushed. %2 replaces it, and every commit after "
                 + "it, with different ones — anyone who already has them will "
                 + "be out of step until they reset.")
                .arg(page.menuShort).arg(action),
            qsTr("Rewrite anyway"), run)
    }
    function squashCommit(oidHex) {
        page.rewriteWarning(qsTr("Folding it in"),
                            function () { repoTab.squashIntoParent(oidHex) })
    }
    function editMessage(oidHex) {
        messageDialog.oid = oidHex
        messageDialog.published = page.menuPublished
        messageDialog.open()
    }

    RewordDialog {
        id: messageDialog
        details: detailsModel
        onSubmitted: (oid, subject, body, published) => {
            const run = function () {
                repoTab.rewordCommit(oid, subject, body)
            }
            if (published)
                page.rewriteWarning(qsTr("Changing its message"), run)
            else
                run()
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
    // The diff has to arrive before a row of it can be staged.
    Timer {
        id: stageRowTimer
        interval: 800
        onTriggered: page.stageSelection(
            0, AppBackend.autoAct === "stage-line" ? 0 : -1)
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
        } else if (act === "switch") {
            page.switchTo("branch", arg, arg)
        } else if (act === "switch-leave") {
            // What the dirty-tree dialog's first button does.
            page.moveKind = "branch"
            page.moveTarget = arg
            page.runSwitch(true)
        } else if (act === "switch-remote") {
            page.switchTo("remote", arg, arg)
        } else if (act === "squash") {
            page.openCommitMenu(branchesModel.headOid)
            page.squashCommit(branchesModel.headOid)
        } else if (act === "reword") {
            page.openCommitMenu(branchesModel.headOid)
            repoTab.rewordCommit(branchesModel.headOid, arg, "")
        } else if (act === "cherry-pick") {
            repoTab.cherryPick(arg)
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
        // Moving HEAD rewrites the working tree under the diff pane:
        // the file it holds may not even exist where the move landed,
        // so the center goes back to the graph that was moved through.
        if (repoTab.lastWriteOp === "checkout")
            page.closeDiff()
        page.refreshHeadPublished()
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

    // What a row click means: the synthetic WIP row (all-zero id)
    // opens the working-tree view, anything else selects the commit.
    function activateRow(oidHex) {
        if (oidHex !== "" && !/[^0]/.test(oidHex)) {
            page.showWip()
            return
        }
        page.wipShown = false
        page.selectedOid = oidHex
        page.selectedStashRef = graphModel.stashRefOf(oidHex)
        detailsModel.request(oidHex)
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
                }

                DetailsPane {
                    anchors.fill: parent
                    visible: !page.wipShown
                    details: detailsModel
                    stashRef: page.selectedStashRef
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
        const row = graphModel.rowOf(oidHex)
        if (row >= 0)
            graphPane.jumpToRow(row)
        // Details resolve even outside the window.
        page.wipShown = false
        page.selectedOid = oidHex
        page.selectedStashRef = graphModel.stashRefOf(oidHex)
        detailsModel.request(oidHex)
        page.closeDiff()
    }
}
