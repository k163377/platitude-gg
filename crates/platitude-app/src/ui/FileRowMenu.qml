pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// The right-click on a working-tree file row. Every row acts on the pane's chosen files (the click chose the row if
// nothing else was).
//
// An `Item` because a menu measures the window through the item it was declared under (`AppMenu.ownerItem`).
Item {
    id: fileRowMenu

    required property RepoTab repoTab
    required property WorkTreeModel workTree
    /// Which rows are chosen (`chosenRows()` skips folder rows, which nothing here acts on).
    required property WipPane wipPane

    /// Decided as it opens and held while it stands (デザイン規約 §メニュー).
    property string bucket: ""
    property string path: ""
    property bool canWrite: false
    /// Whether git could stash at all — **a question about the tree**: one unmerged path anywhere and git refuses the
    /// whole command, named paths or not, and an unborn repository has nowhere to put one (`BandStashButton`,
    /// `platitude_core::stash::standing`).
    property bool canStash: false
    /// How many files the rows here would touch, counted the way the writes count them.
    property int count: 0
    property var plan: null

    /// The tool is not configured yet, so the row is the door to the setting instead (規約 §conflict を外部ツールへ渡す).
    signal mergeToolWanted()
    signal copyRequested(string text)

    /// The card is on screen — what the page reads to know hover is behind a menu (デザイン規約 §メニュー). A plain
    /// property: `visible` read from another file comes back stale (`RefusalBadge`).
    readonly property bool showing: fileMenu.visible

    /// Automation: handles into these rows (the same exposure `GraphPane.view` is).
    readonly property alias menu: fileMenu
    readonly property alias discardItem: fileDiscardItem

    anchors.fill: parent

    function offer(bucket, path) {
        fileRowMenu.bucket = bucket
        fileRowMenu.path = path
        fileRowMenu.canWrite = fileRowMenu.repoTab.busyCount === 0
        fileRowMenu.canStash = fileRowMenu.canWrite && fileRowMenu.workTree.stashStanding === "ready"
        fileRowMenu.plan = fileRowMenu.planDiscard()
        // Rows, not `chosenCount`: a folder in the choice is a chosen key no command here reaches.
        fileRowMenu.count = fileRowMenu.wipPane.chosenRows().length
        fileMenu.offer()
        fileRowMenu.offered(bucket)
    }

    /// Automation: the card was asked for on a row of `bucket`.
    signal offered(string bucket)

    /// What the chosen rows' discard would cost, asked of Rust (`RepoTab.planDiscard` / `discardRows`,
    /// 規約 §その他の操作). Keyed `<bucket>:<path>`: a file changed on both sides has a row in each bucket. The keys stay
    /// on the plan so the words and the write read off the same choice.
    function planDiscard() {
        const rows = fileRowMenu.wipPane.chosenRows()
        const keys = []
        const paths = []
        for (let i = 0; i < rows.length; i++) {
            keys.push(rows[i].bucket + ":" + rows[i].fullName)
            paths.push(rows[i].fullName)
        }
        fileRowMenu.sendPaths(keys)
        fileRowMenu.repoTab.planDiscard()
        return {
            keys: keys,
            count: fileRowMenu.repoTab.discardCount,
            only: fileRowMenu.repoTab.discardOnly,
            // Answered before the row is shown, as the colour has to be (デザイン規約 §長押し).
            unrecorded: GitFacts.discardUnrecorded(fileRowMenu.workTree.unborn, fileRowMenu.workTree.notCopied, paths)
        }
    }
    function discardWords(plan) {
        return !plan || plan.count === 0 ? "" : qsTr("Discard")
    }
    function discardNote(plan) {
        if (!plan || plan.count === 0)
            return ""
        if (plan.count > 1)
            return qsTr("%n files", "", plan.count)
        if (plan.only === "untracked")
            return qsTr("the file goes")
        if (plan.only === "staged")
            return qsTr("both sides")
        return ""
    }
    function discardChosenNow(plan) {
        if (!plan || plan.count === 0)
            return
        fileRowMenu.sendPaths(plan.keys)
        fileRowMenu.repoTab.discardRows()
    }
    function sendPaths(paths) {
        fileRowMenu.repoTab.beginPaths()
        for (let i = 0; i < paths.length; i++)
            fileRowMenu.repoTab.addPath(paths[i])
    }
    function chosenConflicts() {
        const rows = fileRowMenu.wipPane.chosenRows()
        const paths = []
        for (let i = 0; i < rows.length; i++)
            if (rows[i].bucket === "conflicts")
                paths.push(rows[i].fullName)
        return paths
    }
    /// One git command for every chosen conflicted row (デザイン規約 §conflict の ours / theirs).
    function takeSideNow(side) {
        const paths = fileRowMenu.chosenConflicts()
        if (paths.length === 0)
            return
        fileRowMenu.sendPaths(paths)
        fileRowMenu.repoTab.takeSidePaths(side)
    }
    /// The paths are always named: git walks a bare `mergetool` one file at a time and holds the write queue for the
    /// whole walk.
    function openInMergeTool() {
        if (fileRowMenu.workTree.mergeTool === "") {
            fileRowMenu.mergeToolWanted()
            return
        }
        const paths = fileRowMenu.chosenConflicts()
        if (paths.length === 0)
            return
        fileRowMenu.sendPaths(paths)
        fileRowMenu.repoTab.openMergetool()
    }

    AppMenu {
        id: fileMenu
        // Named by branch — during a rebase `--ours` / `--theirs` swap over (デザイン規約 §conflict の ours / theirs).
        // Plain clicks: a conflicted file has no settled version to lose.
        AppMenuItem {
            text: fileRowMenu.workTree.sideOurs !== ""
                  ? qsTr("Keep %1's version").arg(fileRowMenu.workTree.sideOurs)
                  : qsTr("Keep this branch's version")
            offered: fileRowMenu.bucket === "conflicts" && fileRowMenu.canWrite
            onTriggered: fileRowMenu.takeSideNow("ours")
        }
        AppMenuItem {
            text: fileRowMenu.workTree.sideTheirs !== ""
                  ? qsTr("Take %1's version").arg(fileRowMenu.workTree.sideTheirs)
                  : qsTr("Take the incoming version")
            offered: fileRowMenu.bucket === "conflicts" && fileRowMenu.canWrite
            onTriggered: fileRowMenu.takeSideNow("theirs")
        }
        AppMenuItem {
            text: fileRowMenu.workTree.mergeTool !== ""
                  ? qsTr("Edit in %1").arg(fileRowMenu.workTree.mergeTool)
                  : qsTr("Edit in <merge editor>…")
            offered: fileRowMenu.bucket === "conflicts" && fileRowMenu.canWrite
            onTriggered: fileRowMenu.openInMergeTool()
        }
        AppMenuSeparator {}
        AppMenuItem {
            code: "stash"
            note: fileRowMenu.count > 1 ? qsTr("%n files", "", fileRowMenu.count) : ""
            offered: fileRowMenu.canStash
            onTriggered: {
                const rows = fileRowMenu.wipPane.chosenRows()
                const paths = []
                for (let i = 0; i < rows.length; i++)
                    paths.push(rows[i].fullName)
                fileRowMenu.sendPaths(paths)
                // Named out of the commit box, like the band's button (デザイン規約 §変更を退避する).
                fileRowMenu.repoTab.stashPaths(fileRowMenu.wipPane.stashName)
            }
        }
        // Held (デザイン規約 §長押し). On a file changed on both sides the unstaged row keeps what is staged, the staged
        // row takes the lot. `warning`: what goes comes back from the discard record's copy — `danger` where it would
        // not (before the first commit, a file the copy cannot take; デザイン規約 §長押し の色の表).
        AppMenuItem {
            id: fileDiscardItem
            text: fileRowMenu.discardWords(fileRowMenu.plan)
            note: fileRowMenu.discardNote(fileRowMenu.plan)
            offered: fileRowMenu.bucket !== "conflicts" && fileRowMenu.canWrite
            holdMs: Metrics.holdMs
            holdTone: fileRowMenu.plan && fileRowMenu.plan.unrecorded ? Theme.danger : Theme.warning
            onHeld: {
                fileMenu.close()
                fileRowMenu.discardChosenNow(fileRowMenu.plan)
            }
        }
        AppMenuSeparator {}
        AppMenuItem {
            text: fileRowMenu.count > 1 ? qsTr("Copy paths") : qsTr("Copy path")
            onTriggered: {
                const rows = fileRowMenu.wipPane.chosenRows()
                const paths = []
                for (let i = 0; i < rows.length; i++)
                    paths.push(rows[i].fullName)
                fileRowMenu.copyRequested(paths.join("\n"))
            }
        }
    }
}
