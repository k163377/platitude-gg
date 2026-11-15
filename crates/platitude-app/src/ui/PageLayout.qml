import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// A page's layout: the sizes a reader drags, the folded sections, what the next launch restores, and the floor the
// window is held to.
//
// A `QtObject`: no child, and nothing that wants a size
// (rules-refs/structure.md「切り出した非表示のホストは `Item` にする」).
QtObject {
    id: layout

    /// The page whose fold and log (`sidebarCollapsed` / `commandsOpen`) are restored from here; the floor is built on
    /// both. Those two are all this writes on the page, and only in the restore (`applySavedLayout`), so it crosses
    /// none of the page's invariants — the condition for moving page writes out of `RepoPage` at all.
    required property Item page

    required property SidebarPane sidebarPane
    /// The seat the worktree and details panes share — `SplitView` measures the seat.
    required property Item rightPane
    /// The log's seat in the vertical split. The height is the seat's even while the log is down.
    required property Item commandsSeat
    /// The vertical split itself: its height is what the panes and the log share (`panesHeldHeight`).
    required property Item pageSplit
    /// The seat of the screen a tab that failed to open shows in the panes' place (`failedHeldHeight`).
    required property Loader failedSeat
    required property GraphPane graphPane
    /// The two bars over the graph besides its question, whose least the middle column keeps (`panesMinHeight`).
    required property NoticeBar noticeBar
    required property RecoverBand recoverBand
    required property WipPane wipPane
    required property DetailsPane detailsPane
    /// Its line is the right pane's other floor.
    required property GitVersionCorner gitCorner

    required property RepoTab repoTab
    required property NavSectionModel unstagedModel
    required property DetailsModel detailsModel
    required property DiffModel diffModel

    /// Applied once — from here on the splitters own these, and a binding would fight the drag
    /// (rules-refs/app-ui.md「`SplitView` の preferred は代入で固定」). Whether the log is up is held in memory
    /// only (`AppBackend::commands_shown`): a new launch finds it down.
    function applySavedLayout() {
        // The width before the fold: an unfolded list reads it through a binding, and the fold then pins the rail
        // (`applyFold`, through the sidebar's own binding — calling it here too would run it twice).
        layout.sidebarPane.openWidth = AppBackend.startSidebarWidth()
        layout.page.sidebarCollapsed = AppBackend.startSidebarCollapsed()
        layout.page.commandsOpen = AppBackend.startCommandsShown()
        layout.rightPane.SplitView.preferredWidth = AppBackend.startDetailsWidth()
        layout.commandsSeat.SplitView.preferredHeight = AppBackend.startCommandsHeight()
        // -1 = never dragged: follow the default.
        layout.graphPane.labelWManual = AppBackend.startGraphLabelsWidth()
        layout.graphPane.graphColWManual = AppBackend.startGraphLanesWidth()
        layout.sidebarPane.expBranches = AppBackend.startSection("branches")
        layout.sidebarPane.expRemotes = AppBackend.startSection("remotes")
        layout.sidebarPane.expWorktree = AppBackend.startSection("worktree")
        layout.sidebarPane.expStashes = AppBackend.startSection("stashes")
        layout.sidebarPane.expTags = AppBackend.startSection("tags")
        if (layout.page.blank)
            return
        layout.repoTab.setTagsShown(AppBackend.startTagsShown())
        // Through the page: the choice also holds for another worktree's lists, which the page hands the pane.
        layout.page.setWipTreeView(AppBackend.startWipTree())
        layout.detailsModel.setTreeView(AppBackend.startDetailsTree())
        layout.page.setDiffSplit(AppBackend.startDiffSplit())
    }

    // ---- how narrow and how short this page may be laid out ------------
    // The window is held to these (`Main.floorWidth` / `floorHeight`; 規約 §窓の床): past an item's minimum `SplitView`
    // lays the rest out beyond its edge, where nothing scrolls. The numbers are §レイアウト初期値's.
    /// Wide enough for the whole of the corner's line (`GitVersionCorner.floorWidth`): a build named by a long tree
    /// name would otherwise run out past the pane's left edge. A seat letter's build fits 300; only a build from a
    /// long-named tree lifts it.
    readonly property real rightMinWidth: Math.max(300, Math.ceil(layout.gitCorner.floorWidth))
    /// What the graph keeps under whatever stands over it: the bars give way before it does (`RepoPage`'s
    /// `centreColumn`).
    readonly property real graphKeptHeight: Theme.graphRowsKept * Theme.graphRowHeight
    /// The middle column at its least still shows those rows under the report, the discard log's band and a question
    /// with a form all standing at once, each down to its least (デザイン規約 §窓の床). Read off the bars themselves,
    /// so the face each platform sets them in decides it rather than a number taken in one face. The same whatever
    /// stands, as the floor's height is: each bar answers the least of its tallest form (`NoticeBar.floorHeight`,
    /// `RecoverBand.floorHeight`, `AskBar.floorHeight`). Whole pixels, as the window's floor is.
    readonly property int panesMinHeight: Math.ceil(layout.noticeBar.floorHeight + layout.recoverBand.floorHeight
                                                     + layout.graphPane.askFloorHeight + layout.graphKeptHeight)
    /// The three panes' floor: their own, and at least the left menu whole — the log dragged up as far as it goes
    /// stops at the menu's last cell, never over it.
    readonly property real panesFloorHeight: Math.max(layout.panesMinHeight, layout.sidebarPane.floorHeight)
    /// The log's band and one record with a line of output under it (`CommandRowDelegate.height`), and the list's
    /// `bottomMargin`: where a record's two lines can be read (user decision). The same floor with the log down —
    /// the foot that stands in for it is shorter.
    readonly property int commandsMinHeight:
        Theme.headerHeight + Theme.rowHeight + Theme.fontSmLine + 2 * Theme.spaceXs + Theme.spaceXs
    /// The least the log gives way to in a window under its floor: the band and a record's first line, so the list
    /// still shows a row and the rest of every record is reached by its scroll (デザイン規約 §窓の床).
    readonly property int commandsLeastHeight: Theme.headerHeight + Theme.rowHeight

    // ---- a window under its floor ----------------------------------------
    // The floor is a hint to the window manager: Windows' and KWin's maximise, Wayland compositors and tiling window
    // managers stand the window lower. Past the split's minimums `SplitView` lays its items out under its own bottom,
    // where nothing scrolls, so the minimums come down with the room instead (user decision — ask before changing):
    // **the log gives way first**, down to `commandsLeastHeight`, as it does just above the floor (the split holds its
    // fill item, the panes, at their minimum and shortens the log down to its own); then the item over it, down to its
    // least. Each pane takes what it is handed with the scroll it already has — the left menu's sections, the graph's
    // rows, the commit block. Above the floor all of these are the floors themselves.
    /// The least the panes give way to: the left menu whole (`SidebarPane.floorHeight` — the log never stands over its
    /// last cell) and the bars standing over the graph — the report and the discard band each at its least, a
    /// question at the least the floor keeps for any of its forms — so what gives way in the middle is the graph's
    /// rows. Past it the split lays the log out under the window again.
    readonly property real panesLeastHeight: Math.max(layout.sidebarPane.floorHeight,
        (layout.noticeBar.open ? layout.noticeBar.leastHeight : 0)
        + (layout.recoverBand.visible ? layout.recoverBand.leastHeight : 0)
        + (layout.graphPane.asking ? layout.graphPane.askFloorHeight : 0))
    /// The log's minimum: its floor while the split holds it under the item over it at that item's floor, else what
    /// is left, down to its least.
    readonly property real commandsHeldHeight: Math.max(layout.commandsLeastHeight, Math.min(
        layout.commandsMinHeight,
        layout.pageSplit.height - Theme.splitterWidth
        - (layout.page.openFailed ? layout.panesMinHeight : layout.panesFloorHeight)))
    /// What the split leaves the item over the log, the log at its minimum.
    readonly property real roomOverLog:
        layout.pageSplit.height - (layout.page.commandsOpen ? Theme.splitterWidth + layout.commandsHeldHeight : 0)
    /// The panes' minimum (`panesFloorHeight` above the floor).
    readonly property real panesHeldHeight:
        Math.max(layout.panesLeastHeight, Math.min(layout.panesFloorHeight, layout.roomOverLog))
    /// The same for the screen of a tab that failed to open: its floor (`panesMinHeight`) or the room, down to its
    /// words and its way out whole (`OpenFailedScreen.leastHeight` — the screen does not scroll).
    readonly property real failedHeldHeight: Math.max(
        layout.failedSeat.item !== null ? layout.failedSeat.item.leastHeight : 0,
        Math.min(layout.panesMinHeight, layout.roomOverLog))
    /// Whether the worktree pane scrolls past its bottom — its blocks keep their heights (`window-floor wip`).
    readonly property bool wipBlockScrolls: layout.wipPane.blockScrolls
    /// …and how far the commit-details pane runs past its bottom.
    readonly property real detailsOverHeight: layout.detailsPane.contentOverHeight

    /// The middle column's floor: the graph's own (`GraphPane.contentMinW`), and at least a side pane's, so three
    /// columns still read as three.
    readonly property real centreMinWidth: Math.max(layout.graphPane.contentMinW, layout.sidebarPane.minOpenWidth)
    /// Folding the list lowers this to the rail's; a window standing at the old floor is grown by the new one (Main).
    readonly property real floorWidth:
        (layout.page.sidebarCollapsed ? Theme.railWidth : layout.sidebarPane.minOpenWidth)
        + Theme.splitterWidth + layout.centreMinWidth + Theme.splitterWidth + layout.rightMinWidth
    /// The same floor with the list open, whether or not it is — reported beside the floor by the panel's runs
    /// (`openW=`).
    readonly property real openFloorWidth:
        layout.sidebarPane.minOpenWidth
        + Theme.splitterWidth + layout.centreMinWidth + Theme.splitterWidth + layout.rightMinWidth
    /// The panes over the taller of the menu's two faces, and the log at its floor under them, whether or not it is up:
    /// neither the fold nor the log moves it, so opening either on a window at its floor leaves the window's size alone
    /// (デザイン規約 §窓の床). The foot the log takes the place of is shorter than the log, so a window with the log
    /// down holds `panesFloorHeight` too.
    readonly property real floorHeight:
        Math.max(layout.panesMinHeight, layout.sidebarPane.menuHeight)
        + Theme.splitterWidth + layout.commandsMinHeight

    /// Assigned, not bound, as in `applySavedLayout`.
    function setDetailsWidth(w) {
        layout.rightPane.SplitView.preferredWidth = w
    }
    /// The same for the left menu; an ask under the list's floor comes back at the floor. Only the headless check
    /// calls this.
    function setSidebarWidth(w) {
        layout.sidebarPane.SplitView.preferredWidth = w
    }

    /// What dragging the graph's two dividers would leave. Only the headless check calls this.
    function setGraphColumns(labels, lanes) {
        layout.graphPane.labelWManual = labels
        layout.graphPane.graphColWManual = lanes
    }

    /// Hands the layout over to be remembered. Pulled on a timer: a drag moves a width every frame, and what is worth
    /// writing is where it settled. The saved layout is also how tabs hand it over: the leaving tab's page writes here
    /// and the coming one reads `applySavedLayout` (`RepoPageStack.onLeavingTab`), so a field taken out of
    /// `LayoutState` loses both the next launch and the carry between tabs (headless `tab-carry`: `folded=` / `log=`).
    /// A value held for the run only goes on a plain field of `AppBackend` (`commands_shown`, the log up or down).
    function reportLayout() {
        AppBackend.saveLayoutSizes(
            // Folded, the width is the rail's; keep the one it goes back to.
            layout.page.sidebarCollapsed ? layout.sidebarPane.openWidth : layout.sidebarPane.width,
            layout.rightPane.width,
            // `preferredHeight`, not `height`: what a hand set, not what a short window squeezed it to.
            layout.commandsSeat.SplitView.preferredHeight,
            layout.graphPane.labelWManual, layout.graphPane.graphColWManual)
        // A blank page's models stand at their defaults; reporting them would overwrite what the last real page chose.
        if (layout.page.blank)
            return
        AppBackend.saveLayoutFlags(layout.page.sidebarCollapsed,
                                   layout.page.commandsOpen,
                                   layout.repoTab.tagsShown,
                                   layout.unstagedModel.treeView,
                                   layout.detailsModel.treeView,
                                   layout.diffModel.split)
        AppBackend.saveSections(layout.sidebarPane.expBranches,
                                layout.sidebarPane.expRemotes,
                                layout.sidebarPane.expWorktree,
                                layout.sidebarPane.expStashes,
                                layout.sidebarPane.expTags)
    }
}
