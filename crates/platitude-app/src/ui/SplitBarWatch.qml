import QtQuick
import platitude
import platitude.ui

// ---- a split bar that has run out of room ---------------------------
// The graph's own dividers know what the hand asked for; a `SplitView`
// bar cannot (measured 2026-08-11, qmltestrunner: SplitView takes the
// press before anything inside the delegate sees it, and no observer
// behind the view ever becomes active). What is knowable is where the
// pointer went and where the bar stopped, and past a clamp those part
// company — the same refusal read from the other end.
Item {
    id: watch

    /// Whichever boundary is refusing, or null — the page decides, since
    /// only it can see all four sources.
    required property var refusalSource

    /// The graph pane: its own dividers answer their own drags, and the
    /// automation report reads their widths.
    required property Item graphPane

    /// What is drawn, not what was asked for — the one badge's own
    /// `shown` (`RefusalBadge`, on why not its `visible`).
    readonly property alias shown: overlay.shown

    /// The split bars' own refusal and its point, for the page's
    /// `refusalSource` chain (the other three sources keep their own).
    readonly property alias refuses: overlay.refuses
    readonly property alias at: overlay.at

    /// Every split bar in this page, and whichever one has the hand. One
    /// pointer, so at most one at a time. The list is collected rather
    /// than declared: one `handle:` Component builds every bar of its
    /// SplitView, so there is nothing to hang an id on.
    property var splitBars: []
    property Item heldSplit: null

    // The watcher hosts an overlay that measures the window — a sizeless
    // host would place and clip it at 0 (rules-refs/structure.md
    // §描かないホスト).
    anchors.fill: parent
    z: 50

    function holdSplitBar(bar, held) {
        if (watch.splitBars.indexOf(bar) < 0)
            watch.splitBars.push(bar)
        if (held)
            watch.heldSplit = bar
        else if (watch.heldSplit === bar)
            watch.heldSplit = null
    }

    /// What the watcher settles on, given where the hand is in the scene.
    /// The `PointHandler` and the automation hook both come through here,
    /// so the refusal is one answer rather than two kept in step.
    ///
    /// Not a binding: `mapToItem` is a method, and a binding over it would
    /// take no dependency on the geometry it reads and freeze on its first
    /// answer (app-ui.md). A drag moves the bar every frame, so this is
    /// pushed on each point instead.
    function settleSplitRefusal(sceneX, sceneY) {
        const bar = watch.heldSplit
        if (!bar) {
            overlay.refuses = false
            return
        }
        const mid = bar.mapToItem(null, bar.width / 2, bar.height / 2)
        const gap = bar.sideways ? sceneX - mid.x : sceneY - mid.y
        // Scene coordinates, as `SplitRefusalOverlay.at` documents: the
        // overlay maps every source into itself once, in `pointFor` —
        // mapping here too put the badge a TopBar's height above the hand.
        overlay.at = Qt.point(sceneX, sceneY)
        // Past the bar by more than the bar is wide, for the reason the
        // graph's dividers use the same slack: the pointer sits somewhere
        // inside the bar it grabbed, and SplitView carries the bar along
        // at that offset for as long as the layout lets it.
        overlay.refuses = Math.abs(gap) > Theme.splitterWidth
    }

    /// Automation (`PG_AUTO_ACT=divider-refuse`, cases `sidebar-min` /
    /// `details-min` / `log-min`): a press is no more injectable than
    /// hover, so the hook puts the bar in hand the way a press does and
    /// walks the same road the pointer walks.
    function dragSplitPast(which) {
        // Only bars that are drawn. A SplitView builds a handle between
        // every pair of items including the ones standing invisible (the
        // open-failed screen sits beside the panes), and one that is not
        // on screen is not one a hand could have grabbed — nor one with a
        // width and height to tell its direction from.
        const wanted = watch.splitBars.filter(b =>
            b.visible && (which === "log-min" ? !b.sideways : b.sideways))
        if (wanted.length === 0)
            return false
        // Left to right, so the sidebar's bar is the first of the two the
        // horizontal view builds and the details pane's is the last.
        wanted.sort((a, b) => a.mapToItem(null, 0, 0).x - b.mapToItem(null, 0, 0).x)
        const bar = which === "details-min" ? wanted[wanted.length - 1] : wanted[0]
        watch.heldSplit = bar
        const mid = bar.mapToItem(null, bar.width / 2, bar.height / 2)
        const over = 4 * Theme.splitterWidth
        // Into the pane that has no more to give: the sidebar and the log
        // are squeezed from their own side, the details pane from the left.
        if (which === "details-min")
            watch.settleSplitRefusal(mid.x + over, mid.y)
        else if (which === "sidebar-min")
            watch.settleSplitRefusal(mid.x - over, mid.y)
        else
            watch.settleSplitRefusal(mid.x, mid.y + over)
        return true
    }

    /// Automation: the drag and its answer for every boundary in the
    /// window (`PG_AUTO_ACT=divider-refuse`).
    function reportDividerRefusal(which) {
        // The graph's own dividers know what was asked; a split bar is
        // read from where the pointer went instead.
        const split = which === "sidebar-min" || which === "details-min"
                      || which === "log-min"
        if (split)
            watch.dragSplitPast(which)
        else
            watch.graphPane.dragDividerPast(which)
        AppBackend.report("divider_refuse refuses=" + watch.shown
                          // The boundary itself, still drawn and still
                          // promising the drag the other way. For a split
                          // bar this also catches a hook that never found
                          // one to put in hand.
                          + " line=" + (split
                                        ? (watch.heldSplit ? watch.heldSplit.visible
                                                           : false)
                                        : watch.graphPane.refusedLineShown)
                          + " case=" + which
                          + " labelW=" + watch.graphPane.labelW
                          + " graphW=" + watch.graphPane.graphColW)
    }

    SplitRefusalOverlay {
        id: overlay
        refusalSource: watch.refusalSource
        onPointMoved: (sceneX, sceneY) => watch.settleSplitRefusal(sceneX, sceneY)
    }
}
