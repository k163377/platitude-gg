import QtQuick
import platitude.ui

// The graph column's right edge: lanes, faces and badges sink into the ground together over `spaceSm`, from the column's
// edge to where the message tick stands (デザイン規約 §グラフ列は最も広い所のレーンまで). Laid by the ink's owner at that
// edge, over the ink, and as tall as one ground: two over the same ink would steepen the fade.
//
// Painted over, not erased in the canvas: a row's canvas slides with the lanes sent sideways and is not repainted
// (rules-refs/app-ui.md「`Canvas` は「見える幅」で建てる」), so an edge drawn into it would travel with the lanes.
Rectangle {
    id: dissolve

    /// Exactly what lies under the ink: the ground with every wash over it folded in (`Qt.tint`) — anything else leaves
    /// a wrong-coloured strip at the column's edge.
    required property color ground

    width: Theme.spaceSm
    gradient: Gradient {
        orientation: Gradient.Horizontal
        // The ground at zero alpha: `transparent` is black and would fade through a colour the ground has not got.
        GradientStop { position: 0; color: Qt.rgba(dissolve.ground.r, dissolve.ground.g, dissolve.ground.b, 0) }
        GradientStop { position: 1; color: dissolve.ground }
    }
}
