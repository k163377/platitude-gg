pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The whole of one tab's harness in one part, which the page's own QML loads by name and nothing else names
/// (`HarnessSeat`). Everything the verbs and the measurements act on is handed over here — a file of its own cannot see
/// `RepoPage.qml`'s ids.
///
/// **The menus arrive whole**, not row by row: what a verb needs off `RefRowMenu` is which row it presses, and picking
/// that row out is harness wiring rather than something the page has to know it is holding.
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
    required property var planPane
    required property var gitCorner
    /// The menus and the flows a verb enters, each whole.
    required property var refRowMenu
    required property var fileRowMenu
    required property var diffRowMenu
    required property var commitRowMenu
    required property var remoteRowMenu
    required property var rowHost
    required property var clipboard
    required property var commitMenuState
    required property var publishFlow
    required property var upstreamFlow

    /// The page is up: the verbs may start.
    function begin() {
        if (actsLoader.item)
            actsLoader.item.begin()
    }

    // Built only when a verb was given, so a run that is only being measured carries none of the verbs.
    Loader {
        id: actsLoader
        active: AppBackend.autoAct !== ""
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
            planPane: harness.planPane
            gitCorner: harness.gitCorner
            refMenu: harness.refRowMenu.menu
            refBranchCard: harness.refRowMenu.branchCard
            refTagCard: harness.refRowMenu.tagCard
            refDeleteItem: harness.refRowMenu.deleteItem
            refUpstreamItem: harness.refRowMenu.upstreamItem
            refStashDropItem: harness.refRowMenu.stashDropItem
            refSwitchItem: harness.refRowMenu.switchItem
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
            hardResetItem: harness.commitRowMenu.hardResetRow
            publishFlow: harness.publishFlow
            upstreamFlow: harness.upstreamFlow
            remoteDialog: harness.publishFlow.dialog
            remoteMenu: harness.remoteRowMenu.menu
            refList: harness.rowHost.listPopup
            rowCard: harness.rowHost.hoverCard
        }
    }

    Loader {
        active: AppBackend.autoPerf && !harness.page.blank
        sourceComponent: PagePerfDriver {
            page: harness.page
            repoTab: harness.repoTab
            graphModel: harness.graphModel
            worktreeModel: harness.workTree
            branchesModel: harness.branchesModel
            detailsModel: harness.detailsModel
            diffModel: harness.diffModel
            graphPane: harness.graphPane
        }
    }
}
