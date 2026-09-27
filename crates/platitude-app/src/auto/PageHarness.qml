pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// One tab's whole harness, loaded by name from the page's `HarnessSeat`. Everything the verbs and the measurements
/// act on is handed over here — a file of its own cannot see `RepoPage.qml`'s ids.
///
/// The menus arrive whole (picking out the row a verb presses is harness wiring) and as their seats: the page builds
/// a menu the first time it is raised, so this file asks for them up front (`RepoPage.keepBuilt`) — a verb reads a
/// menu's rows before opening it, and a null there is a dead run.
Item {
    id: harness

    anchors.fill: parent

    required property var page
    required property var repoTab
    required property var workTree
    required property var graphModel
    required property var detailsModel
    required property var diffModel
    required property var branchesModel
    required property var remotesModel
    required property var worktreeModel
    /// The working copies (WORKTREES), apart from `worktreeModel` — this tab's own changed files.
    required property var worktreesModel
    required property var stashesModel
    required property var tagsModel
    required property var graphPane
    required property var sidebarPane
    required property var detailsPane
    required property var diffPane
    required property var wipPane
    required property var carriedPane
    required property var gitCorner
    /// The seat the plan's face is built in on demand: the plan verbs raise it the way a hand does, so the driver
    /// follows the seat's `item`.
    required property var planSeat
    required property var refMenuSeat
    required property var fileMenuSeat
    required property var diffMenuSeat
    required property var commitMenuSeat
    required property var remoteMenuSeat
    required property var rowHost
    required property var clipboard
    required property var commitMenuState
    required property var publishFlow
    required property var upstreamFlow

    /// The menus have been asked for, so the driver, which reads its properties off them as it is built, may be
    /// built. Two phases, as the window's (`WindowHarness.screensUp`); both end inside the page's own completion
    /// (`HarnessSeat`).
    property bool seatsUp: false
    readonly property var refRowMenu: harness.refMenuSeat.item
    readonly property var fileRowMenu: harness.fileMenuSeat.item
    readonly property var diffRowMenu: harness.diffMenuSeat.item
    readonly property var commitRowMenu: harness.commitMenuSeat.item
    readonly property var remoteRowMenu: harness.remoteMenuSeat.item
    readonly property var planPane: harness.planSeat.item

    Component.onCompleted: {
        // Only when a verb will want them: five menus held ready are working set a measured run would carry.
        harness.page.keepBuilt = Harness.autoAct !== ""
        harness.seatsUp = true
    }

    /// The page is up: the verbs may start.
    function begin() {
        if (actsLoader.item)
            actsLoader.item.begin()
    }

    PageAutoStart {
        page: harness.page
        graphModel: harness.graphModel
        detailsModel: harness.detailsModel
        diffModel: harness.diffModel
        worktreeModel: harness.worktreeModel
        graphPane: harness.graphPane
    }

    // Only when a verb was given (a measured run carries none), and once the menus it reads are standing.
    Loader {
        id: actsLoader
        active: harness.seatsUp && Harness.autoAct !== ""
        sourceComponent: AutoActDriver {
            page: harness.page
            repoTab: harness.repoTab
            workTree: harness.workTree
            graphModel: harness.graphModel
            detailsModel: harness.detailsModel
            branchesModel: harness.branchesModel
            remotesModel: harness.remotesModel
            worktreeModel: harness.worktreeModel
            worktreesModel: harness.worktreesModel
            stashesModel: harness.stashesModel
            tagsModel: harness.tagsModel
            graphPane: harness.graphPane
            sidebarPane: harness.sidebarPane
            detailsPane: harness.detailsPane
            diffPane: harness.diffPane
            wipPane: harness.wipPane
            carriedPane: harness.carriedPane
            planPane: harness.planPane
            gitCorner: harness.gitCorner
            refMenu: harness.refRowMenu.menu
            refBranchCard: harness.refRowMenu.branchCard
            refTagCard: harness.refRowMenu.tagCard
            refDeleteItem: harness.refRowMenu.deleteItem
            refUpstreamItem: harness.refRowMenu.upstreamItem
            refStashDropItem: harness.refRowMenu.stashDropItem
            refSwitchItem: harness.refRowMenu.switchItem
            refPullItem: harness.refRowMenu.pullItem
            refRebaseItem: harness.refRowMenu.rebaseItem
            refPushTagItem: harness.refRowMenu.pushTagItem
            refTagDeleteItem: harness.refRowMenu.deleteTagItem
            refTagHereItem: harness.refRowMenu.tagHereItem
            refRemoteTagDeleteItem: harness.refRowMenu.deleteRemoteTagItem
            refTagBothDeleteItem: harness.refRowMenu.deleteTagBothItem
            fileRowMenu: harness.fileRowMenu
            fileMenu: harness.fileRowMenu.menu
            fileDiscardItem: harness.fileRowMenu.discardItem
            diffRowMenu: harness.diffRowMenu
            clipboard: harness.clipboard
            commitMenuState: harness.commitMenuState
            commitMenu: harness.commitRowMenu.menu
            dropCommitItem: harness.commitRowMenu.dropItem
            tagHereCommitItem: harness.commitRowMenu.tagHereItem
            stashDeleteItem: harness.commitRowMenu.stashDropItem
            resetMenu: harness.commitRowMenu.resetSubmenu
            commitBranchCard: harness.commitRowMenu.branchCard
            commitTagCard: harness.commitRowMenu.tagCard
            commitDeleteItem: harness.commitRowMenu.branchCard.deleteItem
            switchCommitItem: harness.commitRowMenu.switchItem
            pullCommitItem: harness.commitRowMenu.pullItem
            hardResetItem: harness.commitRowMenu.hardResetRow
            publishFlow: harness.publishFlow
            upstreamFlow: harness.upstreamFlow
            remoteDialog: harness.publishFlow.dialog
            remoteMenu: harness.remoteRowMenu.menu
            refCopyCard: harness.refRowMenu.copyCard
            commitCopyCard: harness.commitRowMenu.copyCard
            refList: harness.rowHost.listPopup
            rowCard: harness.rowHost.hoverCard
            rowHost: harness.rowHost
        }
    }

    Loader {
        active: Harness.autoPerf && !harness.page.blank
        sourceComponent: PagePerfDriver {
            page: harness.page
            repoTab: harness.repoTab
            graphModel: harness.graphModel
            worktreeModel: harness.workTree
            branchesModel: harness.branchesModel
            detailsModel: harness.detailsModel
            diffModel: harness.diffModel
            graphPane: harness.graphPane
            diffPane: harness.diffPane
        }
    }
}
