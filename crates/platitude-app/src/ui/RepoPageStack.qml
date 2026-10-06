pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

// The repository pages: one row per tab, and a page only for the tab in front. A tab that leaves the front is taken
// down with its models and state, and its session let go of (`RepoPage.leaveFront`); coming back reads the repository
// again — the price of the memory budget (CLAUDE.md §性能予算).
//
// The root is the `StackLayout` itself, so its `Layout.*` stay where they are read (rules-refs/structure.md §分割の各論).
StackLayout {
    id: stack

    required property TabsModel tabsModel
    /// The band the pages' toolbar actions stand on (`RepoPage.pageBand`).
    required property Item pageBand
    /// Passed through to whichever page is built: the window regaining focus, and the window being on screen.
    property int focusEpoch: 0
    property bool onScreen: false

    signal openRepositoryPicker()
    signal settingsDialogRequested()
    signal gitSettingsRequested()
    signal avatarSettingsRequested(string name, string email)

    /// The page in front, or null while no tab is. `pageAt` reads the loader's `item` live: a switch moves the strip
    /// and builds the new page in two steps, and only a dependency on `item` sees the second.
    readonly property var curPage: stack.pageAt(stack.tabsModel.currentIndex)
    /// One row per tab whether or not it has a page; the harness counts tabs with it (`WindowAutoActDriver`).
    readonly property alias seats: pageRepeater

    /// The page built for the strip's row at `index`, or null — the row is not there, or its tab is not in front.
    function pageAt(index) {
        if (index < 0 || index >= pageRepeater.count)
            return null
        const holder = pageRepeater.itemAt(index)
        return holder === null ? null : holder.item
    }

    currentIndex: Math.max(0, stack.tabsModel.currentIndex)

    // Handing over before the strip moves — the layout to the next tab, the commit editor's draft to the hub — since
    // the page is taken down behind this (`TabsModel::leaving_tab`).
    Connections {
        target: stack.tabsModel
        function onLeavingTab(index) {
            const leaving = stack.pageAt(index)
            if (leaving !== null)
                leaving.leaveFront()
        }
        // A switch to another worktree of the same repository keeps the page (the tab id does not move); it drops
        // what the old worktree owned between these two (`RepoPage.leaveWorktree` / `standInWorktree`).
        function onLeavingWorktree(index) {
            const leaving = stack.pageAt(index)
            if (leaving !== null)
                leaving.leaveWorktree()
        }
        function onStoodWorktree(index) {
            const stood = stack.pageAt(index)
            if (stood !== null)
                stood.standInWorktree()
        }
    }

    Repeater {
        id: pageRepeater
        model: stack.tabsModel
        Loader {
            id: seat
            // `ComponentBehavior: Bound` keeps the component below out of the delegate's context, so it reads these
            // off this loader by id.
            required property int index
            required property int tab_id
            // By tab id, not `index`: closing or moving a tab renumbers `index` before `currentIndex` catches up, and a
            // page rebuilt in that gap finds its session already open and never reads (`TabsModel::current_tab_id`).
            active: seat.tab_id === stack.tabsModel.currentTabId
            sourceComponent: RepoPage {
                index: seat.index
                tab_id: seat.tab_id
                focusEpoch: stack.focusEpoch
                onScreen: stack.onScreen
                // Built only for the tab in front.
                pageCurrent: true
                pageBand: stack.pageBand
                onOpenRepositoryPicker: stack.openRepositoryPicker()
                onOpenRepositoryPathRequested: path => stack.tabsModel.openRepositoryPath(path)
                onSettingsDialogRequested: stack.settingsDialogRequested()
                onGitSettingsRequested: stack.gitSettingsRequested()
                onAvatarSettingsRequested: (name, email) => stack.avatarSettingsRequested(name, email)
                onCloseTabRequested: stack.tabsModel.closeTab(seat.tab_id)
                onStandHomeRequested: stack.tabsModel.standTabHome(seat.tab_id)
            }
        }
    }
}
