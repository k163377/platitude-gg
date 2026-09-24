import QtQuick
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

// ---- what a page is laid out at, and what the next launch gets back --
// The three sizes a reader can drag, the sections they can fold, and the floor under all of it. Held apart from the
// page because they answer one question between them — what this tab looks like when it is opened again — and because
// the window asks the page for the floor.
//
// Nothing is drawn here, so this is a `QtObject`: it hosts no item that would want a size (rules-refs/structure.md
// §描かないホスト).
QtObject {
    id: layout

    /// The page these belong to. Two of its properties are laid out from here — the fold and the log — because the
    /// saved state says what they were left at, and the floor is built on both.
    required property Item page

    required property SidebarPane sidebarPane
    /// The right-hand seat: the working-tree pane and the commit details share it, and `SplitView` measures the
    /// seat.
    required property Item rightPane
    /// The log's seat in the vertical split: the panel is built into it only while the log is up, and
    /// the height the split is asked for belongs to the seat either way (`RepoPage.commandsSeat`).
    required property Item commandsSeat
    required property GraphPane graphPane
    required property WipPane wipPane
    required property DetailsPane detailsPane

    required property RepoTab repoTab
    required property NavSectionModel worktreeModel
    required property DetailsModel detailsModel
    required property DiffModel diffModel

    /// The layout this page starts with: what the window is set to now, which after a restart is what the last session
    /// left. Read once — from here on the splitters own these, and a binding would fight the drag
    /// (規約 §左メニューを畳む). Whether the log is up is the one of them the file does not hold: it follows the reader
    /// from tab to tab like the rest, and a new launch finds it down (`AppBackend::commands_shown`).
    function applySavedLayout() {
        // The width first: while the list has never been folded, the splitter still reads it through a binding, so this
        // is what puts an unfolded list at the width it was left. Folding after is what pins the rail (`applyFold`,
        // through the sidebar's own binding on this property — calling it here as well would only run it twice).
        layout.sidebarPane.openWidth = AppBackend.startSidebarWidth()
        layout.page.sidebarCollapsed = AppBackend.startSidebarCollapsed()
        layout.page.commandsOpen = AppBackend.startCommandsShown()
        layout.rightPane.SplitView.preferredWidth = AppBackend.startDetailsWidth()
        layout.commandsSeat.SplitView.preferredHeight = AppBackend.startCommandsHeight()
        // -1 travels through unchanged: the pane reads it as "follow the default lane count", which is what a divider
        // nobody has dragged has always done.
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
        // The page's: the choice holds for the lists of another working copy as well, and those are
        // the page's to hand the pane (`RepoPage.setWipTreeView`).
        layout.page.setWipTreeView(AppBackend.startWipTree())
        layout.detailsModel.setTreeView(AppBackend.startDetailsTree())
        layout.page.setDiffSplit(AppBackend.startDiffSplit())
    }

    // ---- how narrow and how short this page may be laid out ------------
    // Under these the panes stop giving: `SplitView` does not shrink an item past its minimum, it lays the rest out
    // beyond its own edge, and nothing in this window scrolls to reach what went over (measured, a 640px
    // window left 32px of the right pane on screen and no way to the other 268). So the window is held to them instead
    // (`Main.floorWidth` / `floorHeight`).
    //
    // Every number here is one §レイアウト初期値 already carries; naming them is what lets the item that obeys one and the
    // floor that is built on it read the same value.
    readonly property int rightMinWidth: 300
    readonly property int panesMinHeight: 200
    readonly property int commandsMinHeight: 120
    /// Whether the working-tree pane has anything below its own fold — the editor, the commit button and a stopped
    /// operation's exit card keep their heights by construction, so in a short pane they are reached by scrolling
    /// (`window-floor wip`).
    readonly property bool wipBlockScrolls: layout.wipPane.blockScrolls
    /// …and how far the commit-details pane runs past its own bottom, which is the same question asked of the other
    /// half of this seat.
    readonly property real detailsOverHeight: layout.detailsPane.contentOverHeight

    /// The middle column's floor. The graph gives up its own columns first — the chips, then the lanes
    /// (`GraphPane.contentMinW`) — and stops where all three of them would stop saying anything. At least what a
    /// side pane may be, so that three columns still read as three at the floor.
    readonly property real centreMinWidth: Math.max(layout.graphPane.contentMinW, layout.sidebarPane.minOpenWidth)
    /// What the folded list costs is the rail, so folding lowers this and unfolding raises it — and a window standing
    /// at the old floor is grown by the new one (Main).
    readonly property real floorWidth:
        (layout.page.sidebarCollapsed ? Theme.railWidth : layout.sidebarPane.minOpenWidth)
        + Theme.splitterWidth + layout.centreMinWidth + Theme.splitterWidth + layout.rightMinWidth
    /// The same floor with the list open, whether or not it is. The band's actions give up their words over the last
    /// stretch before this width (`TopBar.actionCap`), and that stretch may not move when somebody folds the list:
    /// a schedule read off the floor of the moment would put the words back on screen as the rail took the list's
    /// place, which is a thing nobody asked to see move (規約 §窓の床「誰も頼んでいないものが画面上で動く」).
    readonly property real openFloorWidth:
        layout.sidebarPane.minOpenWidth
        + Theme.splitterWidth + layout.centreMinWidth + Theme.splitterWidth + layout.rightMinWidth
    /// The log is a second row when it is open, and it brings its own floor with it — so opening it raises this the
    /// same way.
    readonly property real floorHeight:
        layout.panesMinHeight
        + (layout.page.commandsOpen ? Theme.splitterWidth + layout.commandsMinHeight : 0)

    /// Assigned — a drag writes the same attached property and a binding would be gone after the first one (規約
    /// §左メニューを畳む).
    function setDetailsWidth(w) {
        layout.rightPane.SplitView.preferredWidth = w
    }
    /// The same, for the left menu: what a hand dragging its bar would leave. An ask narrower than the list's own
    /// floor comes back at the floor by the splitter's arithmetic. Only the headless
    /// state check calls this; a person drags.
    function setSidebarWidth(w) {
        layout.sidebarPane.SplitView.preferredWidth = w
    }

    /// What dragging the two dividers inside the graph would leave. Only the headless state check calls this; a person
    /// drags.
    function setGraphColumns(labels, lanes) {
        layout.graphPane.labelWManual = labels
        layout.graphPane.graphColWManual = lanes
    }

    /// Hands the window's layout over to be remembered. Pulled on a timer by the window: a splitter drag moves a
    /// width on every frame, and the point is to write what it settled on.
    function reportLayout() {
        AppBackend.saveLayoutSizes(
            // While the list is folded its width is the rail's; the width it goes back to is the one worth keeping.
            layout.page.sidebarCollapsed ? layout.sidebarPane.openWidth : layout.sidebarPane.width,
            layout.rightPane.width,
            // The height it asks for, open or closed. A drag writes this same
            // property, so what a hand set is here; what a short window squeezed it to is not (the same rule the folded
            // list keeps: only a chosen size is one to come back to. Measured with no floor under the window:
            // opening the log in a 420px window writes 168 over the 280 that was asked for, and every launch after
            // comes back to the smaller one).
            layout.commandsSeat.SplitView.preferredHeight,
            // The dragged values: a column that nobody has moved reports -1 and goes on
            // following the default.
            layout.graphPane.labelWManual, layout.graphPane.graphColWManual)
        // A page with no repository has no choices to report: its models stand at their defaults, and reporting
        // those would write them over what the last real page chose (the split view, tags, the tree views).
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
