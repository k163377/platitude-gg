import QtQuick
import platitude
import platitude.ui

// A split bar that has run out of room. Unlike the graph's own dividers, a `SplitView` bar cannot know what the hand
// asked for (rules-refs/app-ui.md「`SplitView` のドラッグは QML から一切覗けない」); what is knowable is where the
// pointer went and where the bar stopped, and past a clamp those part company.
Item {
    id: watch

    /// Whichever boundary is refusing, or null — the page decides, since only it can see all four sources.
    required property var refusalSource

    /// The graph pane: its own dividers answer their own drags, and the automation report reads their widths.
    required property Item graphPane

    /// What is drawn — the one badge's own `shown` (`RefusalBadge`, on why not its `visible`).
    readonly property alias shown: overlay.shown

    /// The split bars' own refusal and its point, for the page's `refusalSource` chain.
    readonly property alias refuses: overlay.refuses
    readonly property alias at: overlay.at

    /// Every split bar in this page, collected as they report — one `handle:` Component builds them all, so there is
    /// no id to hang on — and the one in hand.
    property var splitBars: []
    property Item heldSplit: null

    // The watcher hosts an overlay that measures the window — a sizeless host would place and clip it at 0
    // (rules-refs/structure.md「描かないホスト」).
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

    /// Settles the refusal for the hand's scene point; the `PointHandler` and the automation hook both come through
    /// here. Pushed on each point, not bound: `mapToItem` is a method, so a binding over it would freeze
    /// (rules/app-ui.md「メソッドはバインディングが依存を取らない」).
    function settleSplitRefusal(sceneX, sceneY) {
        const bar = watch.heldSplit
        if (!bar) {
            overlay.refuses = false
            return
        }
        const mid = bar.mapToItem(null, bar.width / 2, bar.height / 2)
        const gap = bar.sideways ? sceneX - mid.x : sceneY - mid.y
        // Scene coordinates (`SplitRefusalOverlay.at`): the overlay maps into itself in `pointFor`, so mapping here
        // too would put the badge a TopBar's height above the hand.
        overlay.at = Qt.point(sceneX, sceneY)
        // Past the bar by more than its width: the pointer sits somewhere inside the bar it grabbed, and SplitView
        // carries the bar at that offset for as long as the layout lets it.
        overlay.refuses = Math.abs(gap) > Theme.splitterWidth
    }

    /// Automation (`PGG_AUTO_ACT=divider-refuse`, `sidebar-min` / `details-min` / `log-min`): puts the bar in hand as a
    /// press would and walks the pointer's road.
    function dragSplitPast(which) {
        // Only drawn bars: SplitView also builds handles beside invisible items (the open-failed screen).
        const wanted = watch.splitBars.filter(b => b.visible && (which === "log-min" ? !b.sideways : b.sideways))
        if (wanted.length === 0)
            return false
        // Left to right: the sidebar's bar first, the details pane's last.
        wanted.sort((a, b) => a.mapToItem(null, 0, 0).x - b.mapToItem(null, 0, 0).x)
        const bar = which === "details-min" ? wanted[wanted.length - 1] : wanted[0]
        watch.heldSplit = bar
        const mid = bar.mapToItem(null, bar.width / 2, bar.height / 2)
        const over = 4 * Theme.splitterWidth
        // Into the pane that has no more to give: the sidebar and the log from their own side, the details pane from
        // the left.
        if (which === "details-min")
            watch.settleSplitRefusal(mid.x + over, mid.y)
        else if (which === "sidebar-min")
            watch.settleSplitRefusal(mid.x - over, mid.y)
        else
            watch.settleSplitRefusal(mid.x, mid.y + over)
        return true
    }

    /// Whether `which` names one of this page's split bars rather than a divider inside the graph.
    function isSplitBar(which) {
        return which === "sidebar-min" || which === "details-min" || which === "log-min"
    }

    /// The drag past whichever boundary `which` names, for the hand that cannot be injected.
    function dragPast(which) {
        if (watch.isSplitBar(which))
            watch.dragSplitPast(which)
        else
            watch.graphPane.dragDividerPast(which)
    }

    /// The boundary itself, still drawn and still promising the drag the other way. For a split bar this also answers
    /// a hook that never found one to put in hand.
    function lineShown(which) {
        if (!watch.isSplitBar(which))
            return watch.graphPane.refusedLineShown
        return watch.heldSplit ? watch.heldSplit.visible : false
    }

    SplitRefusalOverlay {
        id: overlay
        refusalSource: watch.refusalSource
        onPointMoved: (sceneX, sceneY) => watch.settleSplitRefusal(sceneX, sceneY)
    }
}
