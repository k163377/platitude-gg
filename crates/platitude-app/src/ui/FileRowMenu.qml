pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// The right-click on a worktree file row. Every row acts on the pane's chosen files (the click chose the row if
// nothing else was).
//
// Do not add a row that deletes a tracked file from disk (other tools offer `delete file`): a deletion is a new change,
// made by deleting the file and staging that.
//
// An `Item` because a menu measures the window through the item it was declared under (`AppMenu.ownerItem`).
Item {
    id: fileRowMenu

    required property RepoTab repoTab
    required property WorktreeModel worktree
    /// The worktree as it stood when the command the menus wait for was asked (`RepoPage.treeAtAsk`); null while
    /// nothing is out.
    property var treeAtAsk: null
    /// What the rows are read off, set by the door that stands them: the worktree — or, for a menu raised under a
    /// command, `treeAtAsk`.
    property var tree: fileRowMenu.worktree
    /// Which rows are chosen (`chosenRows()` skips folder rows, which nothing here acts on).
    required property WipPane wipPane

    /// What the rows say while they wait for a git command this window started (`RepoPage.runningWhy`): a menu opened
    /// under one keeps its rows and greys them until it ends (`AppMenu.runningWhy`).
    property string runningWhy: ""

    /// Decided as it opens and held while it stands (デザイン規約 §メニュー).
    property string bucket: ""
    property string path: ""
    /// Whether git could stash at all — **a question about the tree**: one unmerged path anywhere and git refuses the
    /// whole command, named paths or not (read off the row's bucket, the row would stand on every file during a
    /// conflict and spend a write on the refusal), and an unborn repository has nowhere to put one (`BandStashButton`,
    /// `platitude_core::stash::standing`). Both of those grey the row in the toolbar's words; `clean` takes it away,
    /// a file row having nothing to set aside.
    property string stashStanding: ""
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
    readonly property alias stashItem: fileStashItem
    readonly property alias discardItem: fileDiscardItem

    anchors.fill: parent

    function offer(bucket, path) {
        fileMenu.releaseShape()
        fileRowMenu.bucket = bucket
        fileRowMenu.path = path
        fileRowMenu.tree = fileRowMenu.treeAtAsk !== null ? fileRowMenu.treeAtAsk : fileRowMenu.worktree
        fileRowMenu.standOn()
        fileMenu.offer()
        fileRowMenu.offered(bucket)
    }
    /// Reads and freezes what the rows stand on, off the pane's choice as it is now.
    function standOn() {
        fileRowMenu.stashStanding = fileRowMenu.tree.stashStanding
        fileRowMenu.plan = fileRowMenu.planDiscard()
        // Rows, not `chosenCount`: a folder in the choice is a chosen key no command here reaches.
        fileRowMenu.count = fileRowMenu.wipPane.chosenRows().length
    }
    /// The command the menu waited for has ended with the menu up: the rows stand on what it left, the menu keeping its
    /// shape (`AppMenu.holdShape`) — what a discard would now take, whether a stash could now be made. Files the
    /// command took out of their place in the list leave the menu nothing to act on.
    function standAgain() {
        if (!fileRowMenu.showing)
            return
        if (fileRowMenu.wipPane.chosenRows().length === 0) {
            fileMenu.dismiss()
            return
        }
        fileMenu.holdShape()
        fileRowMenu.tree = fileRowMenu.worktree
        fileRowMenu.standOn()
    }

    /// Automation: the card was asked for on a row of `bucket`.
    signal offered(string bucket)

    /// What the chosen rows' discard would cost, asked of Rust (`RepoTab.planDiscard` / `discardRows`,
    /// 規約 §その他の操作). Keyed `<bucket>:<path>`: a file changed on both sides has a row in each bucket, and which one
    /// was chosen lives only in the choice. The sorting — bucket to command, a rename's other half, conflict rows left
    /// out, the count — is Rust's; the menu only hands the keys on. The keys stay on the plan so the words and the
    /// write read off the same choice.
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
            // Answered before the row is shown, as the colour has to be (デザイン規約 §回数押し).
            unrecorded: GitFacts.discardUnrecorded(fileRowMenu.tree.unborn, fileRowMenu.tree.notCopied,
                                                   paths)
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
        if (fileRowMenu.worktree.mergeTool === "") {
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
        runningWhy: fileRowMenu.runningWhy
        onWaitEnded: fileRowMenu.standAgain()
        // Named by branch — during a rebase `--ours` / `--theirs` swap over (デザイン規約 §conflict の ours / theirs).
        // Plain clicks: a conflicted file has no settled version to lose.
        // Each side's branch behind its mark (デザイン規約 §ref の種別「名前の印」), plain — the branch stood on in its
        // own (§ref の種別「現在のブランチは、名前が出る場所すべてで」).
        AppMenuItem {
            refSentence: fileRowMenu.worktree.sideOurs !== "" ? qsTr("Keep %1's version") : ""
            refName: fileRowMenu.worktree.sideOurs
            refTint: fileRowMenu.worktree.sideOurs === fileRowMenu.worktree.branch ? Theme.textLink : Theme.textPrimary
            refMark: "branch"
            text: fileRowMenu.worktree.sideOurs !== ""
                  ? qsTr("Keep %1's version").arg(fileRowMenu.worktree.sideOurs)
                  : qsTr("Keep this branch's version")
            offered: fileRowMenu.bucket === "conflicts"
            onTriggered: fileRowMenu.takeSideNow("ours")
        }
        AppMenuItem {
            refSentence: fileRowMenu.worktree.sideTheirs !== "" ? qsTr("Take %1's version") : ""
            refName: fileRowMenu.worktree.sideTheirs
            refTint: fileRowMenu.worktree.sideTheirs === fileRowMenu.worktree.branch ? Theme.textLink
                                                                                  : Theme.textPrimary
            refMark: "branch"
            text: fileRowMenu.worktree.sideTheirs !== ""
                  ? qsTr("Take %1's version").arg(fileRowMenu.worktree.sideTheirs)
                  : qsTr("Take the incoming version")
            offered: fileRowMenu.bucket === "conflicts"
            onTriggered: fileRowMenu.takeSideNow("theirs")
        }
        // The third way to settle this conflict, above the divider with the two. Unset, the same sentence with the slot
        // empty: `…` keeps its "pressing asks" sense (§回数押し), the angle brackets say what goes there, and the words
        // match the label pressed through to (`Merge editor`). `Edit`, not `Open`: the two above say what becomes of
        // the file, `Open` would make the window the subject. The tool's name plain, not a chip — `stash` here wears
        // a chip as a command, and a config value in the same dress would split the mark's sense (§git 用語のコード表記).
        // Bright while unset too: `textMuted` says "cannot press", and unset is when a press is wanted (§無効).
        // Changing the tool is the settings' job, not a row's: a menu shows nothing until opened, and a rarely used
        // row costs every reader (§メニュー). The day a second thing to set wants a row here, make a row-end side
        // action and move this into it too. No `Mark resolved` row: a tool's resolution is staged by git itself, and
        // an editor's by the row's `+` — the bucket's `Mark all resolved` is a separate matter.
        AppMenuItem {
            text: fileRowMenu.worktree.mergeTool !== ""
                  ? qsTr("Edit in %1").arg(fileRowMenu.worktree.mergeTool)
                  : qsTr("Edit in <merge editor>…")
            offered: fileRowMenu.bucket === "conflicts"
            onTriggered: fileRowMenu.openInMergeTool()
        }
        AppMenuSeparator {}
        AppMenuItem {
            id: fileStashItem
            code: "stash"
            note: fileRowMenu.count > 1 ? qsTr("%n files", "", fileRowMenu.count) : ""
            offered: fileRowMenu.stashStanding !== "" && fileRowMenu.stashStanding !== "clean"
            blockedReason: fileRowMenu.stashStanding === "unborn"
                           ? qsTr("No commits yet — git cannot stash before the first one")
                           : fileRowMenu.stashStanding === "conflicts"
                             ? qsTr("Settle the conflicts first — git will not stash an unmerged file")
                             : ""
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
        // Held (デザイン規約 §回数押し) — the consent is the row's own presses; its words and tag say what goes. **One
        // discard row on every row but a conflicted one, in the same words**: the reader always means "undo what
        // happened to this file", and an untracked file's change is its existence, so it going is the right undo. Only
        // the tag at the row's end changes (`discardNote`): unstaged none — the disk side only, what is staged stays;
        // untracked `the file goes`; staged `both sides` — both back to HEAD, an added file deleted, a rename back
        // to its old name. On a file changed on both sides the two rows are the split: the unstaged row keeps what is
        // staged, the staged row takes the lot; the row says which file. Several chosen, the tag counts (`3 files`) the
        // files the write touches — not a folder row, which no row here acts on. Not on a conflicted row: git refuses
        // to restore an unmerged path. `warning`: what goes comes back from the discard record's copy — `danger`
        // where it would not (before the first commit, a file the copy cannot take; デザイン規約 §回数押し の色の表).
        AppMenuItem {
            id: fileDiscardItem
            text: fileRowMenu.discardWords(fileRowMenu.plan)
            note: fileRowMenu.discardNote(fileRowMenu.plan)
            offered: fileRowMenu.bucket !== "conflicts"
            holdPresses: Metrics.holdPressesWarned
            holdTone: fileRowMenu.plan && fileRowMenu.plan.unrecorded ? Theme.danger : Theme.warning
            onHeld: {
                fileDiscardItem.closeMenu()
                fileRowMenu.discardChosenNow(fileRowMenu.plan)
            }
        }
        AppMenuSeparator {}
        AppMenuItem {
            text: fileRowMenu.count > 1 ? qsTr("Copy paths") : qsTr("Copy path")
            // Reads the screen only, so a running command does not hold it.
            startsCommand: false
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
