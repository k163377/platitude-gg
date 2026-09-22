pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

// ---- the repository pages: one row per tab, and a page for the tab in front ----
//
// **A tab that leaves the front is taken down with everything it read.** The loader below is what does it: its item is
// built when its tab comes to the front and destroyed when it stops being there, which takes the models, their rows
// and the page's own state with it in one move — and the session behind them is let go of on the way out
// (`RepoPage.leaveFront`). Coming back reads the repository again, which is the price of a memory budget a second open
// repository would otherwise spend (CLAUDE.md §性能予算).
//
// What does *not* go with it is the way the window is laid out: that is one set for the whole application
// (`LayoutState`), reported by the page being left and applied by the one arriving, so a switch changes the repository
// under the panes and nothing about the panes.
//
// The loader keeps its row's place whether or not it has a page, so the strip and everything counting tabs still see
// one item per tab.
//
// The root is the layout itself: what stands in the window's column is a
// `StackLayout`, and its `Layout.*` stay where they are read (rules-refs/structure.md §分割の各論).
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

    /// The page in front — what the window's toolbar acts on, and null while no tab is.
    ///
    /// Reached through that tab's loader, and `item` is *read* live, so the binding follows the
    /// loader in and out: a switch moves the strip and builds the new page in two separate steps, and only a
    /// dependency on `item` sees the second one.
    readonly property var curPage: stack.pageAt(stack.tabsModel.currentIndex)
    /// The rows themselves, one per tab whether or not it has a page. The harness counts tabs with it
    /// (`WindowAutoActDriver`).
    readonly property alias seats: pageRepeater

    /// The page built for the strip's row at `index`, or null — the row is not there, or its tab is not in front.
    function pageAt(index) {
        if (index < 0 || index >= pageRepeater.count)
            return null
        const holder = pageRepeater.itemAt(index)
        return holder === null ? null : holder.item
    }

    currentIndex: Math.max(0, stack.tabsModel.currentIndex)

    // Handing over, before the strip moves: what the tab being left was laid out at goes to the next one, and what its
    // commit editor is holding goes to the hub — its page and everything it read are taken down behind this
    // (`TabsModel::leaving_tab`).
    Connections {
        target: stack.tabsModel
        function onLeavingTab(index) {
            const leaving = stack.pageAt(index)
            if (leaving !== null)
                leaving.leaveFront()
        }
        // …and the switch that keeps its page: the tab stands in another working copy of the repository it is
        // already showing, so the page stays and drops what that copy owned between these two
        // (`RepoPage.leaveCopy` / `standInCopy`). Its loader is not touched — the tab id does not move.
        function onLeavingCopy(index) {
            const leaving = stack.pageAt(index)
            if (leaving !== null)
                leaving.leaveCopy()
        }
        function onStoodCopy(index) {
            const stood = stack.pageAt(index)
            if (stood !== null)
                stood.standInCopy()
        }
    }

    Repeater {
        id: pageRepeater
        model: stack.tabsModel
        Loader {
            id: seat
            // Taken from the strip's rows: `pragma ComponentBehavior: Bound` keeps the component below out of the
            // delegate's context, so what it needs is read off this loader by id.
            required property int index
            required property int tab_id
            // Which *tab* is in front: a row closed to the left of this one, or a tab carried past it,
            // renumbers `index` before `currentIndex` catches up, and a page taken down and rebuilt in that gap comes
            // back to a session that is already open and will not read itself again (`TabsModel::current_tab_id`).
            active: seat.tab_id === stack.tabsModel.currentTabId
            sourceComponent: RepoPage {
                index: seat.index
                tab_id: seat.tab_id
                focusEpoch: stack.focusEpoch
                onScreen: stack.onScreen
                // Built only for the tab in front, so there is no other answer to give.
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
