pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The hands on the two boundaries between the graph's three columns (labels | graph | message): what a drag asks its
// column for, and whether the ask runs past what the column allows. Widths are `GraphColumnMetrics`'s; the hover lines
// are the pane's (rules-refs/structure.md「手を持つ部品と、その手が上げる線は同じファイルに入らない」).
Item {
    id: dividers

    /// `GraphColumnMetrics`; a drag writes its two manual widths.
    required property var columns
    /// No repository behind the pane.
    required property bool blank

    /// Whether the pointer is on each divider, and where — written by real hover and the automation hooks alike.
    property bool dividerPointed: false
    property point dividerPoint: Qt.point(0, 0)
    property bool labelPointed: false
    property point labelPoint: Qt.point(0, 0)
    /// What each divider's last drag asked for, before the clamp — the only thing that tells a refusal from a rest.
    property real dividerAsked: 0
    property real labelAsked: 0
    /// A real press or the hooks' stand-in, so each refusal is one expression for both roads.
    readonly property bool dividerDragging: graphDivider.pressed || dividers.dividerHeld
    readonly property bool labelDragging: labelDivider.pressed || dividers.labelHeld
    /// Automation only: stand-ins for the presses the hooks cannot make.
    property bool dividerHeld: false
    property bool labelHeld: false
    /// Whether a drag asks a column for a width it cannot have, at either end. Only while dragging, not on hover: at a
    /// bound the column still moves the other way. Bindings, so letting go ends the ask with no teardown to miss.
    readonly property bool dividerRefused: dividers.dividerDragging
        && dividers.outOfRange(dividers.dividerAsked, dividers.columns.graphColWMin, dividers.columns.graphColWMax)
    readonly property bool labelRefused: dividers.labelDragging
        && dividers.outOfRange(dividers.labelAsked, dividers.columns.labelColWMin, dividers.columns.labelColWMax)
    /// Whether either divider is refusing, and where the hand is (scene coordinates). A column with no drag left either
    /// way refuses on hover; one at a bound only under the drag that tried. The badge is the page's — a drag carries
    /// the hand out past this pane.
    readonly property bool refused: dividers.dividerRefused || dividers.labelRefused
        || (dividers.columns.graphColWFixed && dividers.dividerPointed)
    readonly property point refusedAt: dividers.labelRefused ? dividers.labelPoint : dividers.dividerPoint
    /// Where each boundary stands, and whether its hover line is wanted — the pane draws the lines.
    readonly property alias labelX: labelDivider.x
    readonly property alias graphDividerX: graphDivider.x
    readonly property bool labelLineWanted: dividers.labelPointed || labelDivider.pressed
    /// No line on a column with one width — the cursor speaks instead (規約 §グラフ列は最も広い所のレーンまで).
    readonly property bool graphLineWanted: !dividers.columns.graphColWFixed
        && (dividers.dividerPointed || graphDivider.pressed)
    /// The divider as drawn, so the hook cannot pass a fixed column that still offers a drag.
    readonly property alias graphDividerShown: graphDivider.visible

    /// A drag on the graph divider to `px`; the handler and the hook both come through here.
    function dragDividerTo(px) {
        if (dividers.columns.graphColWFixed)
            return
        dividers.dividerAsked = px - Theme.spaceSm - Theme.borderWidth - dividers.columns.labelW
        dividers.columns.graphColWManual = Math.max(dividers.columns.graphColWMin,
            Math.min(dividers.dividerAsked, dividers.columns.graphColWMax))
    }
    /// The same for the chip column, whose ask is the pointer's x — that column starts at the pane's edge.
    function dragLabelTo(px) {
        dividers.labelAsked = px
        dividers.columns.labelWManual = Math.max(dividers.columns.labelColWMin,
            Math.min(px, dividers.columns.labelColWMax))
    }
    /// Whether an ask runs past either end by more than the divider's width — a bare grab at a bound lands up to half
    /// that width out and must not trip it. Binding-safe: everything that varies arrives as an argument.
    function outOfRange(asked, floor, ceiling) {
        return asked > ceiling + Theme.splitterWidth || asked < floor - Theme.splitterWidth
    }
    /// Automation: the pointer resting on the graph divider's middle (`PGG_AUTO_ACT=graph-divider`).
    function restDividerPointer(inside) {
        dividers.dividerPointed = inside
        if (inside)
            dividers.dividerPoint = graphDivider.mapToItem(null, graphDivider.width / 2, dividers.height / 2)
    }
    /// Automation: a drag carried past one of the four bounds (`PGG_AUTO_ACT=divider-refuse`, whose argument names
    /// which), walking the road the handler walks.
    function dragDividerPast(which) {
        const over = 2 * Theme.splitterWidth
        if (which === "label-min" || which === "label-max") {
            const lx = which === "label-max" ? dividers.columns.labelColWMax + over
                                             : dividers.columns.labelColWMin - over
            dividers.labelPointed = true
            dividers.labelHeld = true
            dividers.labelPoint = dividers.mapToItem(null, lx, dividers.height / 2)
            dividers.dragLabelTo(lx)
            return
        }
        const px = dividers.columns.labelW + Theme.spaceSm + Theme.borderWidth
                   + (which === "graph-min" ? dividers.columns.graphColWMin - over
                                            : dividers.columns.graphColWMax + over)
        dividers.dividerPointed = true
        dividers.dividerHeld = true
        dividers.dividerPoint = dividers.mapToItem(null, px, dividers.height / 2)
        dividers.dragDividerTo(px)
    }

    ColumnDivider {
        id: labelDivider
        frame: dividers
        x: dividers.columns.labelW - Theme.splitterWidth / 2
        visible: !dividers.blank
        onPointedInto: inside => dividers.labelPointed = inside
        onPointedAt: at => dividers.labelPoint = at
        onDragged: x => dividers.dragLabelTo(x)
    }
    ColumnDivider {
        id: graphDivider
        frame: dividers
        // Centred on the message tick, so the hover line lies over it.
        x: dividers.columns.labelW + dividers.columns.graphColW + Theme.spaceSm
           + Theme.borderWidth - Theme.splitterWidth / 2
        visible: !dividers.blank
        onPointedInto: inside => dividers.dividerPointed = inside
        onPointedAt: at => dividers.dividerPoint = at
        onDragged: x => dividers.dragDividerTo(x)
    }
}
