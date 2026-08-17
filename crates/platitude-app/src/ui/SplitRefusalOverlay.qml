import QtQuick
import platitude.ui

// Front-most over the page and drawing nothing but the badge. A passive
// grab is the only thing that sees the pointer while SplitView has the
// drag, and being passive it leaves SplitView the drag (measured: the
// boundary still moved while this reported the hand out past it).
Item {
    id: overlay

    /// Where the split bar's hand is, in scene coordinates, and whether it
    /// is being refused. The other two sources keep their own; this one is
    /// the overlay's because only it can see them. The page pushes both on
    /// every point (`RepoPage.settleSplitRefusal`).
    property point at: Qt.point(0, 0)
    property bool refuses: false

    /// Whichever boundary is refusing, or null — the page decides, since
    /// only it can see all four of them.
    required property var refusalSource

    /// What is drawn, not what was asked for — the one badge's own `shown`
    /// (`RefusalBadge`, on why not its `visible`).
    readonly property alias shown: refusalBadge.shown

    /// The hand, every time it moves. The page settles what it means: the
    /// `PointHandler` and the automation hook both go through that one
    /// answer rather than two kept in step.
    signal pointMoved(real sceneX, real sceneY)

    anchors.fill: parent
    z: 50

    /// Where the badge goes, given the refusing point: out of the scene
    /// and into this sheet, which is what `RefusalBadge.at` is in.
    function pointFor(scene) {
        return scene === null ? Qt.point(0, 0) : overlay.mapFromItem(null, scene.x, scene.y)
    }

    PointHandler {
        onPointChanged: overlay.pointMoved(point.scenePosition.x, point.scenePosition.y)
        // Letting go ends the ask, the same way the graph's dividers end
        // theirs — nothing to remember to take back down.
        onActiveChanged: if (!active) overlay.refuses = false
    }

    // ---- the one badge in the window ----------------------------
    // A refused drag has the hand *outside* the thing it was dragging, so
    // a badge parented to the divider's own pane lands outside its parent
    // and is composited under the page's panes (2026-08-11 ユーザー報告);
    // over the whole page it is above them all. `mapFromItem` is a method
    // and would not re-run on its own, but the point it reads changes on
    // every move of the drag that raised it, which is the only time this
    // is up (app-ui.md).
    RefusalBadge {
        id: refusalBadge
        at: overlay.pointFor(overlay.refusalSource)
        shown: overlay.refusalSource !== null
    }
}
