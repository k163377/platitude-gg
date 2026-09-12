pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// What the page's own wiring says it did, written out as report lines.
///
/// Each of these is an **edge**, not a state: the question bar carries no name to read the branch back off, the row
/// that is being stood in for looks the same as one nothing was asked about, and git's answer to a write is one
/// message wide. None of them can be recovered by reading the page a moment later, so the page says so with a
/// signal apiece and the line is written from here — where the whole of it can be left out of a build. The held
/// delete is the exception: git's refusal is an answer on the tab, read here the way the card reads it.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given: the same guard every one of these
/// lines carried when the page still wrote them itself.
// An `Item` only because that is what the driver's children are; it draws nothing and is never given a size.
Item {
    id: reports

    /// The driver these belong to. `var` because naming its type here would be a circle: it is the file that builds
    /// this one.
    required property var driver

    // The driver's own names, read once so the handlers below can name them bare.
    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var fileRowMenu: driver.fileRowMenu
    readonly property var diffRowMenu: driver.diffRowMenu
    /// The branch whose plain delete git has just refused — the answer the standing card turns its row on
    /// (`RefBranchMenu`), cleared as the next plain delete is asked, so each refusal is its own edge here.
    readonly property string refusedDelete: reports.repoTab.branchDeleteRefused

    onRefusedDeleteChanged: {
        if (reports.refusedDelete !== "")
            Harness.report("force_delete_offered branch=" + reports.refusedDelete)
    }

    /// A row is being stood in for while git is asked to delete it, and which one — read off what the window is
    /// actually drawing without (`ops::StandIn`, through `RepoTab`) rather than off the press, so the line says a row
    /// left the screen and not merely that something was asked for. One name per list, and a delete touches at most
    /// one of each, so the four edges never say the same row twice.
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
        function onRenameRemoteAsked(from, to) {
            Harness.report("rename_remote_asked from=" + from + " to=" + to)
        }
        // What the write was about is still on the tab when this goes out — the page clears its own standing
        // questions, not the tab's report of the answer.
        function onWriteReported() {
            Harness.report("write_reported kind=" + reports.repoTab.writeReportKind
                              + " ref=" + reports.repoTab.writeReportRemote
                              + "/" + reports.repoTab.writeReportName)
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
        // `shown` is the card's own answer, not the conditions behind it: a card that refused to open because it had
        // nothing to put in it is the one failure a picture of an empty overlay cannot tell from a card nobody asked
        // for.
        function onOffered(shown) {
            Harness.report("diff_menu open=" + shown
                              + " rows=" + reports.diffRowMenu.menu.offeredRows
                              + " copy=" + reports.diffRowMenu.canCopy
                              + " removed=" + reports.diffRowMenu.removed)
        }
    }
}
