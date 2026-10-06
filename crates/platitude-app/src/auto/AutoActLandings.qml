pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// Where a press leaves the reader, once everything it set off has settled. Core answers a write before it
/// publishes the status and refs that write invalidated (`session::write::run_write`), so these wait past the write
/// barriers on `AutoActDriver`, which builds this file.
// An `Item` only because `QtObject` has no default property to hold the timers below.
Item {
    id: landings

    /// `var` because naming its type would be a circle: the driver is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var workingTree: driver.workingTree
    readonly property var graphModel: driver.graphModel
    readonly property var branchesModel: driver.branchesModel
    readonly property var unstagedModel: driver.unstagedModel
    readonly property var stashesModel: driver.stashesModel
    readonly property var graphPane: driver.graphPane
    readonly property var renderedBarrier: driver.barrierRendered

    /// Arms the stash landing — called once the graph the row went out of has settled.
    function awaitStash() {
        stashLandTimer.start()
    }
    /// Arms the stopped-operation landing — called right after the press that puts the operation down.
    function awaitOpExit() {
        opExitLandTimer.start()
    }

    // Where the stash press left the reader. `follows=` is by oid: a highlight left on the index the working-tree row
    // vacated lights whatever slid into it (here, the new entry), and two rows a few lines apart look alike in the
    // picture. Waited out on the pane, so a build that never lands still answers: the tree is empty and the right side
    // has caught up — the working tree's pane or a settled commit card; `wip=` says which.
    SampleTimer {
        id: stashLandTimer
        onTriggered: {
            if (unstagedModel.total !== 0)
                return
            if (!page.wipShown && !driver.cardSettled)
                return
            stashLandTimer.stop()
            const row = graphModel.rowOf(workingTree.headOid)
            Harness.report("stash_landed wip=" + page.wipShown
                              + " follows=" + (page.selectedOid === workingTree.headOid)
                              + " onscreen=" + graphPane.rowOnScreen(row)
                              + " lit=" + (graphPane.view.currentIndex === row)
                              + " head=" + workingTree.headOid.substring(0, 8)
                              + " selected=" + page.selectedOid.substring(0, 8)
                              + " row=" + row + " rows=" + graphModel.rowTotal
                              // Last because it has spaces. A box filled by a standing merge (`absorbOpMessage`)
                              // holds the merge's words, which must not name the entry (`WipPane.stashName`).
                              + " entry=" + stashesModel.nameAt(0))
            renderedBarrier.begin()
        }
    }
    /// Where putting a stopped operation down leaves the reader — from every exit-card row, or the same command run in
    /// a terminal. Ended by the status, not the write: the page leaves the WIP face off the status that says the
    /// operation is gone (`RepoPage.leaveWipWhenDone`), published after the answer. Waited out on resting states, as
    /// `stashLandTimer` is; `wip=` is a report because the face left over a clean tree pictures the same as an
    /// ordinary empty WIP face. Behind the whole write barrier as well: the worktree listing is a read of its own,
    /// landing apart from that status, and until it lands the main copy's row names the HEAD the stop stood on — only
    /// the write's settle says it is in (`session::write::settle_after`).
    SampleTimer {
        id: opExitLandTimer
        onTriggered: {
            if (!driver.wroteAndSettled() || workingTree.opText !== "")
                return
            if (!page.wipShown) {
                if (!driver.cardSettled)
                    return
                // The graph rebuilds after the status, so the working-tree row the operation held open over a clean
                // tree is still drawn; waited out. Only on a clean tree: an abort that brings work back keeps that row.
                if (unstagedModel.total === 0 && driver.graphTopKind() === "wip")
                    return
            }
            opExitLandTimer.stop()
            const row = graphModel.rowOf(workingTree.headOid)
            Harness.report("op_exit_landed wip=" + page.wipShown
                              + " op=" + workingTree.opText
                              + " follows=" + (page.selectedOid === workingTree.headOid)
                              + " onscreen=" + graphPane.rowOnScreen(row)
                              // The main copy's WORKTREES row: the branch HEAD stands on once the operation is down,
                              // the folder where it is left on none (`quit`; デザイン規約 §左メニューの所作).
                              + " home=" + driver.navProbe.homeCopyName()
                              + " lit=" + (graphPane.view.currentIndex === row)
                              + " files=" + unstagedModel.total
                              + " head=" + workingTree.headOid.substring(0, 8)
                              + " selected=" + page.selectedOid.substring(0, 8)
                              + " row=" + row)
            renderedBarrier.begin()
        }
    }
}
