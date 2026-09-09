pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// The two boundaries between the graph's three columns (labels | graph | message): where each stands, what a drag on it
// asks its column for, and whether that ask has run out past what the column is allowed.
//
// The widths themselves are `GraphColumnMetrics`'s, and the hover lines are drawn by the pane: those sit under the list
// so the message ticks and the lane strokes stay in front of them, which is the one place a hand cannot also be. What
// lives here is the hand.
Item {
    id: dividers

    /// The three columns' arithmetic (`GraphColumnMetrics`): what each is allowed to be, what it is, and the two manual
    /// widths a drag writes.
    required property var columns
    /// No repository behind the pane: there are no columns to divide.
    required property bool blank

    /// Whether the pointer is on each divider, and where. Real hover and the automation hooks write the same pairs —
    /// hover cannot be injected (verify-ui), and both the lines and the badge have to be provable.
    property bool dividerPointed: false
    property point dividerPoint: Qt.point(0, 0)
    property bool labelPointed: false
    property point labelPoint: Qt.point(0, 0)
    /// What each divider's last drag asked its column for, clamped or not. The clamped answer is the column's width;
    /// this is what the hand wanted, which is the only thing that can tell a refusal from a rest.
    property real dividerAsked: 0
    property real labelAsked: 0
    /// Whether a drag is under way. The hooks' own flags ride beside the real presses because a press is no more
    /// injectable than hover is (verify-ui) — and they ride here rather than inside the refusals, so each refusal stays
    /// one expression that both roads reach.
    readonly property bool dividerDragging: graphDivider.pressed || dividers.dividerHeld
    readonly property bool labelDragging: labelDivider.pressed || dividers.labelHeld
    /// Automation only: stand in for the presses the hooks cannot make. Nothing a hand can reach writes these.
    property bool dividerHeld: false
    property bool labelHeld: false
    /// Whether a drag is asking a column for a width it cannot have. **Either end** — a hand that has run out has run
    /// out whichever way it was going, and answering only one way leaves the other reading as a divider that broke.
    ///
    /// Held apart from hover on purpose: at either bound the column still moves the other way, so a badge worn merely
    /// for standing there would say "this does not move" about a divider that does.
    ///
    /// Bindings, not something set and taken back down: letting go ends the ask, and a badge left on screen by a
    /// teardown nobody ran is exactly the failure a hand would see and a run could not.
    readonly property bool dividerRefused: dividers.dividerDragging
        && dividers.outOfRange(dividers.dividerAsked, dividers.columns.graphColWMin, dividers.columns.graphColWMax)
    readonly property bool labelRefused: dividers.labelDragging
        && dividers.outOfRange(dividers.labelAsked, dividers.columns.labelColWMin, dividers.columns.labelColWMax)
    /// Whether either divider is refusing, and where the hand is while it does (**scene coordinates**). One pointer, so
    /// one answer at a time: the two dividers cannot both have the hand.
    ///
    /// Two ways to it, and they are asked differently. A column squeezed until it has no drag left in either direction
    /// answers on hover — there is nothing to try. A column at a bound answers only the drag that tried, because it
    /// still moves the other way.
    ///
    /// The badge is not drawn here: a drag carries the hand out past the pane, and a badge parented to it would be
    /// composited among the page's panes rather than over them. The page owns the one badge.
    readonly property bool refused: dividers.dividerRefused || dividers.labelRefused
        || (dividers.columns.graphColWFixed && dividers.dividerPointed)
    readonly property point refusedAt: dividers.labelRefused ? dividers.labelPoint : dividers.dividerPoint
    /// Where each boundary stands, and whether its hover line is wanted — the pane draws those two lines and reports
    /// what they came to, since they belong under the list rather than in here.
    readonly property alias labelX: labelDivider.x
    readonly property alias graphDividerX: graphDivider.x
    /// Reads this item's own property rather than the MouseArea's hover, so the automation hook can raise the line too
    /// — hover cannot be injected, and a line nothing can prove is a line nothing checks (app-ui.md).
    readonly property bool labelLineWanted: dividers.labelPointed || labelDivider.pressed
    /// The line says "this moves". A column with one width does not, so it stays out and the cursor speaks instead (規約
    /// §グラフ列は最も広い所のレーンまで).
    readonly property bool graphLineWanted: !dividers.columns.graphColWFixed
        && (dividers.dividerPointed || graphDivider.pressed)
    /// What is drawn, not what was asked for: the automation hook reports the divider itself, so a column that cannot
    /// be resized but still promises a drag cannot pass.
    readonly property alias graphDividerShown: graphDivider.visible

    /// Where a drag on the graph divider leaves the column, and what it asked for on the way. The handler and the
    /// automation hook both come through here, so the clamp is one answer rather than two kept in step — and the
    /// refusal above reads the ask, so it is one too.
    function dragDividerTo(px) {
        if (dividers.columns.graphColWFixed)
            return
        dividers.dividerAsked = px - Theme.spaceSm - Theme.borderWidth - dividers.columns.labelW
        dividers.columns.graphColWManual = Math.max(dividers.columns.graphColWMin,
            Math.min(dividers.dividerAsked, dividers.columns.graphColWMax))
    }
    /// The same for the chip column's divider, whose asked width is simply where the pointer is — that column starts at
    /// the pane's edge.
    function dragLabelTo(px) {
        dividers.labelAsked = px
        dividers.columns.labelWManual = Math.max(dividers.columns.labelColWMin,
            Math.min(px, dividers.columns.labelColWMax))
    }
    /// Whether an ask has run out past either end of what it is allowed, by more than the divider is wide — so a press
    /// on its own cannot trip it: grabbing a line lands its column anywhere within half that width of where it already
    /// sat, and at a bound half of those grabs would be asking for more.
    ///
    /// Safe to call from a binding: everything that varies arrives as an argument, so the binding takes its
    /// dependencies from the call site (unlike a method that reads them itself — app-ui.md).
    function outOfRange(asked, floor, ceiling) {
        return asked > ceiling + Theme.splitterWidth || asked < floor - Theme.splitterWidth
    }
    /// Automation: the pointer resting on the graph divider, at its middle (`PGG_AUTO_ACT=graph-divider`). Where that is
    /// stays here rather than in the hook — one answer, not a second one to keep in step.
    function restDividerPointer(inside) {
        dividers.dividerPointed = inside
        if (inside)
            dividers.dividerPoint = graphDivider.mapToItem(null, graphDivider.width / 2, dividers.height / 2)
    }
    /// Automation: a drag carried out past one of the four bounds these two dividers have
    /// (`PGG_AUTO_ACT=divider-refuse`, whose argument names which). Presses are no more injectable than hover is
    /// (verify-ui), so each walks the road its own handler walks and takes the pointer where a hand would have carried
    /// it.
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
        // Sits behind the message tick column so the hover line overlaps the ticks.
        x: dividers.columns.labelW + dividers.columns.graphColW + Theme.spaceSm
           + Theme.borderWidth - Theme.splitterWidth / 2
        visible: !dividers.blank
        onPointedInto: inside => dividers.dividerPointed = inside
        onPointedAt: at => dividers.dividerPoint = at
        onDragged: x => dividers.dragDividerTo(x)
    }
}
