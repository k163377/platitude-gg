pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// The right-click on a working-tree file row. Every row here acts on the pane's chosen files, not on the one that was
// clicked — the click chose it if nothing else was — so the pane is asked what they are rather than a second answer
// being kept here.
//
// What the menu offers, and what the discard row would cost, are worked out once as it opens: the choice cannot change
// while the menu is up, so the words the row says and the writes it runs are read off the same plan (デザイン規約 §メニュー).
//
// An `Item` because a menu measures the window through the item it was declared under (`AppMenu.ownerItem`); it draws
// nothing itself.
Item {
    id: fileRowMenu

    required property RepoTab repoTab
    required property WorkTreeModel workTree
    /// Which rows are chosen, and what each of them is: the pane counts them the way the writes do (`chosenRows()`
    /// skips folder rows, which nothing in this menu acts on).
    required property WipPane wipPane

    /// What this menu is standing on and what it offers, decided as it opens and held while it stands: the conditions
    /// are live (a timer fetch alone moves `busyCount`), and a row that appears or vanishes under the pointer is a row
    /// clicked by accident.
    property string bucket: ""
    property string path: ""
    property bool canWrite: false
    /// How many files the rows here would touch, counted the way the writes count them.
    property int count: 0
    /// What the discard row would do, worked out once as the menu opens.
    property var plan: null

    /// The tool is not configured yet, so the row is the door to the setting instead (規約 §conflict を外部ツールへ渡す).
    signal mergeToolWanted()
    signal copyRequested(string text)

    /// The card is on screen. A plain property rather than an alias — `visible` read from another file comes back
    /// stale (`RefusalBadge`) — and what the page reads to know that hover is behind a menu now (デザイン規約 §メニュー).
    readonly property bool showing: fileMenu.visible

    /// The automation's handles into these rows, an automation-only exposure the same as `GraphPane.view` is
    /// (app-ui.md).
    readonly property alias menu: fileMenu
    readonly property alias discardItem: fileDiscardItem

    anchors.fill: parent

    /// Opens on a row of that bucket, deciding there and then what the rows may do and what the discard would take.
    function offer(bucket, path) {
        fileRowMenu.bucket = bucket
        fileRowMenu.path = path
        fileRowMenu.canWrite = fileRowMenu.repoTab.busyCount === 0
        fileRowMenu.plan = fileRowMenu.planDiscard()
        // Not `chosenCount` — that counts chosen keys, so a folder in the choice would put a file on the tag that no
        // command is going to reach.
        fileRowMenu.count = fileRowMenu.wipPane.chosenRows().length
        fileMenu.offer()
        fileRowMenu.offered(bucket)
    }

    /// The card was asked for on a row of `bucket`. An automation-only exposure, the same one `GraphPane.view` is
    /// (app-ui.md) — how many rows it came up with is on [`menu`] already.
    signal offered(string bucket)

    /// What the chosen rows' discard would cost, asked across the bridge: the rows go over keyed `<bucket>:<path>` —
    /// which row was chosen is the choice's to say, since a file changed on both sides has a row in each bucket — and
    /// the sorting into commands, a staged rename's second name and the conflicted rows' exemption are all Rust's
    /// (`RepoTab.planDiscard` / `discardRows`, 規約 §その他の操作). The keys are kept on the plan so the words the
    /// row says and the write it runs read off the same choice.
    function planDiscard() {
        const rows = fileRowMenu.wipPane.chosenRows()
        const keys = []
        for (let i = 0; i < rows.length; i++)
            keys.push(rows[i].bucket + ":" + rows[i].fullName)
        fileRowMenu.sendPaths(keys)
        fileRowMenu.repoTab.planDiscard()
        return {
            keys: keys,
            count: fileRowMenu.repoTab.discardCount,
            only: fileRowMenu.repoTab.discardOnly
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
    /// Held, not asked (デザイン規約 §長押し).
    function discardChosenNow(plan) {
        if (!plan || plan.count === 0)
            return
        fileRowMenu.sendPaths(plan.keys)
        fileRowMenu.repoTab.discardRows()
    }
    /// Hands a set of paths to the bridge for the write that follows.
    function sendPaths(paths) {
        fileRowMenu.repoTab.beginPaths()
        for (let i = 0; i < paths.length; i++)
            fileRowMenu.repoTab.addPath(paths[i])
    }
    /// The conflicted rows among those chosen — the only ones a side can be taken on.
    function chosenConflicts() {
        const rows = fileRowMenu.wipPane.chosenRows()
        const paths = []
        for (let i = 0; i < rows.length; i++)
            if (rows[i].bucket === "conflicts")
                paths.push(rows[i].fullName)
        return paths
    }
    /// Takes one side of every conflicted row that is highlighted, in one git command (デザイン規約 §その他の操作).
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
        // Named by branch rather than `--ours` / `--theirs` — during a rebase those two swap over (デザイン規約 §conflict の
        // ours / theirs). Plain clicks: a conflicted file has no settled version to lose.
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
        // With nothing configured the row becomes the door to the setting (規約 §conflict を外部ツールへ渡す).
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
            // git will not stash a tree with unresolved conflicts in it.
            offered: fileRowMenu.bucket !== "conflicts" && fileRowMenu.canWrite
            onTriggered: {
                const rows = fileRowMenu.wipPane.chosenRows()
                const paths = []
                for (let i = 0; i < rows.length; i++)
                    paths.push(rows[i].fullName)
                fileRowMenu.sendPaths(paths)
                // Named out of the commit box like the band's button, and by the same pane (デザイン規約 §変更を退避する).
                fileRowMenu.repoTab.stashPaths(fileRowMenu.wipPane.stashName)
            }
        }
        // Held, not asked (デザイン規約 §長押し). On a file changed on both sides the two rows are the choice itself: the
        // unstaged one keeps what is staged, the staged one takes the lot.
        AppMenuItem {
            id: fileDiscardItem
            text: fileRowMenu.discardWords(fileRowMenu.plan)
            note: fileRowMenu.discardNote(fileRowMenu.plan)
            offered: fileRowMenu.bucket !== "conflicts" && fileRowMenu.canWrite
            holdMs: Metrics.holdMs
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
