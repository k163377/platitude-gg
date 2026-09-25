pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// What the page's own wiring says it did, written out as report lines.
///
/// Each is an edge a later read of the page cannot recover, so the page raises a signal apiece and the line is
/// written here, where the whole of it stays out of a build. The held delete is the exception: git's refusal is an
/// answer on the tab, read here the way the card reads it.
// An `Item` only because that is what the driver's children are; it is a sizeless holder.
Item {
    id: reports

    /// `var` because naming the driver's type here would be a circle: it is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var fileRowMenu: driver.fileRowMenu
    readonly property var diffRowMenu: driver.diffRowMenu
    /// The branch whose plain delete git just refused (`RefBranchMenu` turns its row on it). Cleared as the next plain
    /// delete is asked, so each refusal is its own edge.
    readonly property string refusedDelete: reports.repoTab.branchDeleteRefused

    onRefusedDeleteChanged: {
        if (reports.refusedDelete !== "")
            Harness.report("force_delete_offered branch=" + reports.refusedDelete)
    }

    /// The row stood in for while git deletes it, per list — read off what the window actually draws without
    /// (`ops::StandIn`), so the line says a row left the screen. A delete touches at most one per list, so each row is
    /// named once.
    readonly property string goneBranch: reports.repoTab.goneBranch
    readonly property string goneRemote: reports.repoTab.goneRemote
    readonly property string goneTag: reports.repoTab.goneTag
    readonly property string goneStash: reports.repoTab.goneStash

    function noteGone(kind, id) {
        if (id !== "")
            Harness.report("gone_shown kind=" + kind + " id=" + id)
    }

    onGoneBranchChanged: reports.noteGone("branch", reports.goneBranch)
    onGoneRemoteChanged: reports.noteGone("remote", reports.goneRemote)
    onGoneTagChanged: reports.noteGone("tag", reports.goneTag)
    onGoneStashChanged: reports.noteGone("stash", reports.goneStash)

    Connections {
        target: reports.page

        function onMoveBranchAsked(local) {
            Harness.report("move_branch_asked local=" + local)
        }
        function onReplaceRemoteAsked(from, to) {
            Harness.report("replace_remote_asked from=" + from + " to=" + to)
        }
        // What the write was about rides the signal: one drain can bring several answers
        // (`RepoTab.writeAnswerReportKind`).
        function onWriteReported(kind, remote, name) {
            Harness.report("write_reported kind=" + kind + " ref=" + remote + "/" + name)
        }
    }

    Connections {
        target: reports.fileRowMenu
        function onOffered(bucket) {
            Harness.report("file_menu bucket=" + bucket
                              + " rows=" + reports.fileRowMenu.menu.offeredRows)
        }
    }

    Connections {
        target: reports.diffRowMenu
        // `shown` is the card's own answer: one that refused to open for want of rows photographs like one nobody
        // asked for.
        function onOffered(shown) {
            Harness.report("diff_menu open=" + shown
                              + " rows=" + reports.diffRowMenu.menu.offeredRows
                              + " copy=" + reports.diffRowMenu.canCopy
                              + " removed=" + reports.diffRowMenu.removed)
        }
    }
}
