import QtQuick
import platitude.ui

// The graph column's right edge: lanes, faces and badges sink into the ground together over `spaceSm`, from the column's
// edge to where the message tick stands (user decision — ask before changing; `GraphColumnMetrics`). One place and
// one depth for all of them: cut apart, a narrowing column drops the line first and leaves the face afloat. The ground
// is the row's own, selection and pointer washes included, and the edge stays with the column while the lanes are
// sent sideways. The HEAD pin sinks at the same edge; a lane fading out of the pin also sinks here, and the corner
// where both fades meet takes the product of the two, so neither edge shows a step. At the floor the badge's tip sinks
// too and no ink reaches the tick; with every lane in the column, nothing reaches the strip. Laid by the ink's owner at
// that edge, over the ink, and as tall as one ground: two over the same ink would steepen the fade.
//
// Painted over, not erased in the canvas: a row's canvas slides with the lanes sent sideways and is not repainted
// (rules-refs/app-ui.md「`Canvas` は「見える幅」で建てる」), so an edge drawn into it would travel with the lanes. Laid
// over as `TabTitleFade` is; not a `ShaderEffect` mask, which draws nothing under software rendering. The ink has three
// owners, and each lays its own: the row's body (`GraphLaneCell`, the HEAD pin's too), the pin going out
// (`GraphHeadPin`, one per ground in the going-out box: the row's box, then the hold) and the window's cut
// (`GraphTailFooter`).
//
// Checked off a picture: the strip where no ink stands has to match the ground — compare its pixel column with the
// ground's (`System.Drawing`); ±1 is the gradient's rounding.
Rectangle {
    id: dissolve

    /// Exactly what lies under the ink: the ground with every wash over it folded in (`Qt.tint`) — anything else leaves
    /// a wrong-coloured strip at the column's edge. The row's is `GraphRowDelegate.laneGround`, the pin's
    /// `GraphHeadPin.rowGround`: a ground added to the row goes into both.
    required property color ground

    width: Theme.spaceSm
    gradient: Gradient {
        orientation: Gradient.Horizontal
        // The ground at zero alpha: `transparent` is black and would fade through a colour the ground has not got.
        GradientStop { position: 0; color: Qt.rgba(dissolve.ground.r, dissolve.ground.g, dissolve.ground.b, 0) }
        GradientStop { position: 1; color: dissolve.ground }
    }
}
