pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// Where a press leaves the reader, once everything it set off has settled.
///
/// A file of their own because they are the one wait that is behind the write barrier rather than at it: core answers a
/// write before it publishes the status and the refs that write invalidated (`session::write::run_write`), so the
/// landing is a message or two later than the answer, and each of these has a report of its own to make about it. The
/// barriers on `AutoActDriver` say a write happened; these say where it put somebody.
///
/// Built by that driver, which `RepoPage` builds only when a verb was given. What they act on hangs off it.
// An `Item` only because `QtObject` has no default property to hold the timers below; it draws nothing and is never
// given a size.
Item {
    id: landings

    /// The driver these landings belong to. `var` because naming its type here would be a circle: it is the file that
    /// builds this one.
    required property var driver

    // The driver's own names, read once so the waits below can name them bare.
    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var workTree: driver.workTree
    readonly property var graphModel: driver.graphModel
    readonly property var branchesModel: driver.branchesModel
    readonly property var worktreeModel: driver.worktreeModel
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

    // Where the press left the reader, once the graph the row went out of has settled. The selection is the whole
    // subject, so it is waited for on the far side of the rebuild and read the way the reader would: the pane that was
    // describing the working tree is gone, the commit under it is the one the branch points at, the details pane is
    // showing that commit rather than the one before it, and the row is on screen.
    //
    // **The row being lit is not enough** — a highlight left on an index the working-tree row vacated lights whatever
    // slid into it, and in this run that is the entry the press just made. `follows=` is the identity the picture
    // cannot hold: two rows a couple of lines apart look alike at this width.
    //
    // **Waited out on the pane, not on the landing**, so a build that never lands still answers: the tree is empty and
    // whatever is on the right has caught up with the page — the working tree's own pane, which needs nothing fetched,
    // or a commit whose details have arrived. Both are states the application rests in, and the run says which one it
    // reached rather than waiting out its watchdog on the wrong one.
    SampleTimer {
        id: stashLandTimer
        onTriggered: {
            if (worktreeModel.total !== 0)
                return
            if (!page.wipShown && !driver.cardSettled)
                return
            stashLandTimer.stop()
            const row = graphModel.rowOf(branchesModel.headOid)
            AppBackend.report("stash_landed wip=" + page.wipShown
                              + " follows=" + (page.selectedOid === branchesModel.headOid)
                              + " onscreen=" + graphPane.rowOnScreen(row)
                              + " lit=" + (graphPane.view.currentIndex === row)
                              + " head=" + branchesModel.headOid.substring(0, 8)
                              + " selected=" + page.selectedOid.substring(0, 8)
                              + " row=" + row + " rows=" + graphModel.rowTotal
                              // What the entry ended up called, last because it is the one field with spaces in it.
                              // The other half of the same press: a box filled by the merge that was standing
                              // (`absorbOpMessage`) is not a name anybody gave these changes, and an entry wearing it
                              // would be promising a merge it does not hold (`WipPane.stashName`).
                              + " entry=" + stashesModel.nameAt(0))
            renderedBarrier.begin()
        }
    }
    /// Where putting a stopped operation down leaves the reader — the far end of every exit-card row, and of the same
    /// row pressed in a terminal.
    ///
    /// **The write is not what ends this.** Core answers the continuation before it publishes the status that says the
    /// operation is gone, and the page leaves the WIP face off that status (`RepoPage.leaveWipWhenDone`) — so a barrier
    /// on the write photographs the face still standing, which is what a build that never leaves it photographs too.
    ///
    /// Waited out on the states the application rests in rather than on the landing, for `stashLandTimer`'s reason: a
    /// build that keeps the reader on the face rests there just as firmly, and the run says which of the two it reached
    /// instead of spending its watchdog on the wrong one. The face it would be left on is an empty pane over a clean
    /// tree with no card on it — the same picture as an ordinary WIP face with nothing in it, which is why `wip=` is a
    /// report and not a photograph.
    SampleTimer {
        id: opExitLandTimer
        onTriggered: {
            if (repoTab.busyCount !== 0 || workTree.opText !== "")
                return
            if (!page.wipShown) {
                if (!driver.cardSettled)
                    return
                // The graph is rebuilt after the status that ends the operation, so the row the operation was holding
                // open — the working tree's, drawn over a clean tree because something was running — is still there
                // when the landing is decided. Waited out so the picture is of the history the reader is left reading
                // rather than of the one the stop left behind. **Only on a clean tree**: an abort that brings
                // uncommitted work back keeps that row for a reason of its own, and there is nothing to wait for.
                if (worktreeModel.total === 0 && driver.graphTopKind() === "wip")
                    return
            }
            opExitLandTimer.stop()
            const row = graphModel.rowOf(branchesModel.headOid)
            AppBackend.report("op_exit_landed wip=" + page.wipShown
                              + " op=" + workTree.opText
                              + " follows=" + (page.selectedOid === branchesModel.headOid)
                              + " onscreen=" + graphPane.rowOnScreen(row)
                              + " lit=" + (graphPane.view.currentIndex === row)
                              + " files=" + worktreeModel.total
                              + " head=" + branchesModel.headOid.substring(0, 8)
                              + " selected=" + page.selectedOid.substring(0, 8)
                              + " row=" + row)
            renderedBarrier.begin()
        }
    }
}
