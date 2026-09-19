pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The whole of one tab's harness in one part, which the page's own QML loads by name and nothing else names
/// (`HarnessSeat`). Everything the verbs and the measurements act on is handed over here — a file of its own cannot see
/// `RepoPage.qml`'s ids.
///
/// **The menus arrive whole**: what a verb needs off `RefRowMenu` is which row it presses, and picking
/// that row out is harness wiring. **And they arrive as
/// their seats**: the page builds a menu the first time it is raised, so what the page can hand over is the seat, and
/// asking for the menus up front is this file's to do (`RepoPage.keepBuilt`) — a verb reads a menu's rows before
/// opening it, and a null there is a dead run.
Item {
    id: harness

    anchors.fill: parent

    /// The page and the models it owns. An automation-only exposure, the same one `GraphPane.view` is (app-ui.md).
    required property var page
    required property var repoTab
    required property var workTree
    required property var graphModel
    required property var detailsModel
    required property var diffModel
    required property var branchesModel
    required property var remotesModel
    required property var worktreeModel
    required property var stashesModel
    required property var tagsModel
    /// The panes.
    required property var graphPane
    required property var sidebarPane
    required property var detailsPane
    required property var diffPane
    required property var wipPane
    required property var carriedPane
    required property var gitCorner
    /// The seat the plan's face is built in while a plan stands. Built on demand: the plan verbs raise the
    /// face the way a hand does and read it only once it is up, so the driver follows the seat's `item`.
    required property var planSeat
    /// The seats the five menus are built in, and the flows a verb enters, each whole.
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

    /// The menus have been asked for, so the verbs that read them may be built. **Two
    /// phases**: the driver's properties are read off the menus as it is built, and this whole part is built inside
    /// the page's own completion, so both phases are over before the page reaches its next line (`HarnessSeat`) — the
    /// same two the window's harness keeps (`WindowHarness.screensUp`).
    property bool seatsUp: false
    readonly property var refRowMenu: harness.refMenuSeat.item
    readonly property var fileRowMenu: harness.fileMenuSeat.item
    readonly property var diffRowMenu: harness.diffMenuSeat.item
    readonly property var commitRowMenu: harness.commitMenuSeat.item
    readonly property var remoteRowMenu: harness.remoteMenuSeat.item
    readonly property var planPane: harness.planSeat.item

    Component.onCompleted: {
        // Only where a verb is going to want one: five menus held ready are working set every measured run would
        // otherwise carry (`RepoPage.keepBuilt`).
        harness.page.keepBuilt = Harness.autoAct !== ""
        harness.seatsUp = true
    }

    /// The page is up: the verbs may start.
    function begin() {
        if (actsLoader.item)
            actsLoader.item.begin()
    }

    // What a run can be told to do to the page without giving it a verb at all.
    PageAutoStart {
        page: harness.page
        graphModel: harness.graphModel
        detailsModel: harness.detailsModel
        diffModel: harness.diffModel
        worktreeModel: harness.worktreeModel
        graphPane: harness.graphPane
    }

    // Built only when a verb was given, so a run that is only being measured carries none of the verbs — and only
    // once the menus it reads are standing.
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
