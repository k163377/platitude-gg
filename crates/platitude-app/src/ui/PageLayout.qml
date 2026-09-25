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
    /// both.
    required property Item page

    required property SidebarPane sidebarPane
    /// The seat the working-tree and details panes share — `SplitView` measures the seat.
    required property Item rightPane
    /// The log's seat in the vertical split. The height is the seat's even while the log is down.
    required property Item commandsSeat
    required property GraphPane graphPane
    required property WipPane wipPane
    required property DetailsPane detailsPane

    required property RepoTab repoTab
    required property NavSectionModel worktreeModel
    required property DetailsModel detailsModel
    required property DiffModel diffModel

    /// Applied once — from here on the splitters own these, and a binding would fight the drag
    /// (rules-refs/app-ui.md「`SplitView` の min / max / preferred は代入で固定」). Whether the log is up is held in memory
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
        // Through the page: the choice also holds for another working copy's lists, which the page hands the pane.
        layout.page.setWipTreeView(AppBackend.startWipTree())
        layout.detailsModel.setTreeView(AppBackend.startDetailsTree())
        layout.page.setDiffSplit(AppBackend.startDiffSplit())
    }

    // ---- how narrow and how short this page may be laid out ------------
    // The window is held to these (`Main.floorWidth` / `floorHeight`; 規約 §窓の床): past an item's minimum `SplitView`
    // lays the rest out beyond its edge, where nothing scrolls. The numbers are §レイアウト初期値's.
    readonly property int rightMinWidth: 300
    readonly property int panesMinHeight: 200
    readonly property int commandsMinHeight: 120
    /// Whether the working-tree pane scrolls past its bottom — its blocks keep their heights (`window-floor wip`).
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
    /// The same floor with the list open, whether or not it is — what `TopBar.actionCap` schedules the band's words
    /// off, so folding the list does not bring them back (規約 §窓の床「誰も頼んでいないものが画面上で動く」).
    readonly property real openFloorWidth:
        layout.sidebarPane.minOpenWidth
        + Theme.splitterWidth + layout.centreMinWidth + Theme.splitterWidth + layout.rightMinWidth
    readonly property real floorHeight:
        layout.panesMinHeight
        + (layout.page.commandsOpen ? Theme.splitterWidth + layout.commandsMinHeight : 0)

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
    /// writing is where it settled.
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
                                   layout.worktreeModel.treeView,
                                   layout.detailsModel.treeView,
                                   layout.diffModel.split)
        AppBackend.saveSections(layout.sidebarPane.expBranches,
                                layout.sidebarPane.expRemotes,
                                layout.sidebarPane.expWorktree,
                                layout.sidebarPane.expStashes,
                                layout.sidebarPane.expTags)
    }
}
