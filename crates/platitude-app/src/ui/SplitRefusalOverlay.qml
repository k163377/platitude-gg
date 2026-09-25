import QtQuick
import platitude.ui

// Front-most over the page, drawing nothing but the badge. Its passive grab is the only thing that sees the pointer
// while SplitView has the drag, and being passive it leaves SplitView the drag.
Item {
    id: overlay

    /// Where the split bar's hand is, in scene coordinates, and whether it is refused. Held here because only this
    /// overlay sees the hand; pushed on every point (`SplitBarWatch.settleSplitRefusal`).
    property point at: Qt.point(0, 0)
    property bool refuses: false

    /// Whichever boundary is refusing, or null — the page decides, since only it can see all four of them.
    required property var refusalSource

    /// What is drawn — the one badge's own `shown` (`RefusalBadge`, on why not its `visible`).
    readonly property alias shown: refusalBadge.shown

    /// The hand, every time it moves; `SplitBarWatch.settleSplitRefusal` settles what it means.
    signal pointMoved(real sceneX, real sceneY)

    anchors.fill: parent
    z: 50

    /// Where the badge goes, given the refusing point: out of the scene and into this sheet (`RefusalBadge.at`).
    function pointFor(scene) {
        return scene === null ? Qt.point(0, 0) : overlay.mapFromItem(null, scene.x, scene.y)
    }

    PointHandler {
        onPointChanged: overlay.pointMoved(point.scenePosition.x, point.scenePosition.y)
        onActiveChanged: if (!active) overlay.refuses = false
    }

    // The one badge in the window, over the whole page: a refused drag has the hand outside the pane it was dragging,
    // where a badge parented to that pane is composited under the other panes. `mapFromItem` is a method and would not
    // re-run on its own, but its point changes on every move of the drag, the only time this is up
    // (rules/app-ui.md「メソッドはバインディングが依存を取らない」).
    RefusalBadge {
        id: refusalBadge
        at: overlay.pointFor(overlay.refusalSource)
        shown: overlay.refusalSource !== null
    }
}
