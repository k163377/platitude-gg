pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

// The repository pages: one row per tab, and a page only for the tab in front. A tab that leaves the front is taken
// down with its models and state, and its session let go of (`RepoPage.leaveFront`); coming back reads the repository
// again — the price of the memory budget (internal-docs/性能.md §予算). Taking the page down also means no part writes a
// reset of its own for a tab switch.
//
// Do not cheapen the switch back (user decision — ask before changing): no kept sessions for tabs out of front (an
// LRU), no lighter `RepoSession::refresh_quick` on a large repository, no opening in stages (the newest 64–100
// commits first) — what the time to the first picture waits on is the processes queued before the walk, not the walk
// (ci/baseline/code-costs-windows-x64.md). The other worktrees' pass stays right after the open
// (`RepoSession::refresh_opening`).
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

    /// The seat of the tab in front — put here by the seat itself as its row's flag rises (`settleFrontSeat`), as the
    /// strip's tabs name themselves (`TabStrip.frontTab`), never looked up by row: closing a tab to the left renumbers
    /// rows before `currentIndex` catches up, and a page looked up by row reads null in that gap. Null while no tab is
    /// (a front row being closed lets go of the front before it goes — `TabsModel::close`).
    property Item frontSeat: null
    /// The page in front, or null while no tab is. Read off the seat's `item` live: a switch builds the new page after
    /// its seat comes forward, and only a dependency on `item` sees it arrive.
    readonly property var curPage: stack.frontSeat === null ? null : stack.frontSeat.item
    /// One row per tab whether or not it has a page; the harness counts tabs with it (`WindowAutoActDriver`).
    readonly property alias seats: pageRepeater

    /// A seat's row flag moved (`TabItem::current`). Order-free: the seat losing the front clears it only if nobody has
    /// claimed it since.
    function settleFrontSeat(seat) {
        if (seat.current)
            stack.frontSeat = seat
        else if (stack.frontSeat === seat)
            stack.frontSeat = null
    }

    // The seat shown, by the model's row, not the front seat's `index`: a carry hands the seat its new `index` before
    // the `StackLayout`'s children are put in the new order, and the layout, shown the new number then, shows the seat
    // that stood there — an empty one (`tab-drag 3:1`, `tab-edge 0`). `currentIndex` is told once the rows have moved.
    // In the turn a close renumbers them it can name an empty seat, which no frame draws.
    currentIndex: Math.max(0, stack.tabsModel.currentIndex)

    // Handing over before the strip moves — the layout to the next tab, the commit editor's draft to the hub — since
    // the page is taken down behind this (`TabsModel::leaving_tab`). The signal makes the order (leaving page's
    // `reportLayout`, then the coming page's `applySavedLayout`); left to binding evaluation, which runs first is
    // undefined. Every one of them is about the tab in front, so it goes to the page in front.
    Connections {
        target: stack.tabsModel
        function onLeavingTab() {
            if (stack.curPage !== null)
                stack.curPage.leaveFront()
        }
        // A switch to another worktree of the same repository keeps the page (its row stays the tab in front); it
        // drops what the old worktree owned between these two (`RepoPage.leaveWorktree` / `standInWorktree`).
        function onLeavingWorktree() {
            if (stack.curPage !== null)
                stack.curPage.leaveWorktree()
        }
        function onStoodWorktree() {
            if (stack.curPage !== null)
                stack.curPage.standInWorktree()
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
            // By its row's own flag, not `index` against `currentIndex`: closing or moving a tab renumbers `index`
            // before `currentIndex` catches up, and a page rebuilt in that gap finds its session already open and never
            // reads (`TabItem::current`).
            required property bool current
            active: seat.current
            // Before `active` builds the page (a handler runs ahead of the bindings on the same property), so
            // `curPage` is this seat's page the moment it is built.
            onCurrentChanged: stack.settleFrontSeat(seat)
            // A seat born in front never announces `current` changing.
            Component.onCompleted: if (seat.current) stack.settleFrontSeat(seat)
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
                // Called later, as the stand home is: the page is still sequencing the answer that asked.
                onWorktreeFollowRequested: path =>
                    Qt.callLater(() => stack.tabsModel.followWorktree(seat.tab_id, path))
            }
        }
    }
}
