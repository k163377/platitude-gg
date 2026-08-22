pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Center pane: the commit graph.
Rectangle {
    id: graphArea

    required property var graphModel
    // The uncommitted row's tallies ride on the ListView for the delegate. They come off status rather than off the
    // file list — the same status the list itself is built from, counted by kind (`status::Kinds`).
    required property var workTree
    // No repository behind this pane: the empty-window call to action.
    property bool blank: false

    signal rowActivated(string oidHex)
    signal rowMenuOpenRequested(string oidHex)
    /// Right-click on the chip itself: the menu is the named ref's rather than the row's — and the page still falls
    /// back to the row's where the chip names nothing to act on. `record` is the chip as drawn (kind + flags + name).
    signal chipMenuOpenRequested(string oidHex, string record)
    /// A row was double-clicked. `record` is the chip it shows (kind + flags + name); empty when the row shows no
    /// branch at all.
    signal rowSwitchRequested(string oidHex, string record)
    /// The chip whose stacked list the page has out (null when none). Rows read it back through the view: a hand that
    /// walked down into the list and comes back to this chip is not opening anything, so it is not made to sit out the
    /// opening rest again.
    property var chipListAnchor: null
    /// The commit whose card the page has out (empty when none). Rows read it back through the view the same way: the
    /// row the card came out of stays lit under it, the card having taken the pointer off it.
    property string rowCardOid: ""
    /// A stacked chip was hovered: unstack it under the chip.
    signal chipExpandRequested(var records, var anchor)
    /// The pointer settled on a row (or left it): open the commit card under it. `row` is the delegate, which the page
    /// needs for its position and its fields — it must not hold on to it.
    signal rowHoverRequested(var row, bool inside)
    /// The pointer left that chip — put it back, unless it went into the list itself (only the owner can tell).
    signal chipCollapseRequested()
    signal createBranchRequested(string oidHex, string name)
    signal openRepositoryRequested()

    // ---- naming a branch on a row that has none --------------------
    /// Puts the chip column of one row into a branch-name box.
    function startNaming(oidHex) {
        graphList.askOid = ""
        graphList.namingText = ""
        graphList.namingOid = oidHex
    }
    function stopNaming() {
        graphList.namingOid = ""
        graphList.namingText = ""
    }

    // ---- looking for a commit --------------------------------------
    /// The card itself — automation-only exposure, the same one `view` and `headPin` are (app-ui.md). A headless run
    /// reads its `open` / `query` / `matches` / `atMatch` / `width` / `opacity` / `findClears` off it and types into
    /// its box; eight names on this pane said nothing the card does not, and the key that opens it cannot be pressed
    /// from there.
    readonly property alias findCard: findBar
    /// How far the graph has stepped down out from under the card. This one is the list's, not the card's.
    readonly property real findShift: graphList.anchors.topMargin
    signal findLanded(string oidHex)
    function startFind() { findBar.startFind() }
    function findNext() { findBar.findNext() }
    function findPrevious() { findBar.findPrevious() }

    // ---- a press that landed somewhere else -------------------------
    /// Both boxes this pane can be standing on are offers rather than work half done: the find card and the name box on
    /// a row. An empty one goes away with the press that landed elsewhere (`RepoPage.releasePressedAway`, `scenePos` =
    /// where it landed); one with something typed in it stays, because the typing is what there would be to lose.
    function dropEmptyBoxes(scenePos) {
        findBar.dropIfEmpty(scenePos)
        if (graphList.namingOid !== "" && graphList.namingText === "")
            graphArea.stopNaming()
    }
    /// Whether all of `row` is on screen — asked of the list, which is where the viewport arithmetic lives.
    function rowOnScreen(row) { return graphList.rowOnScreen(row) }

    // ---- a standing question ---------------------------------------
    // The bar comes down from the top of the pane and pushes the history down (デザイン規約 §可否・警告の出し場所); the row it
    // concerns is marked rather than worded, so the question is written exactly once.

    /// Raises the bar. `oidHex` is the row it is about ("" for none, and a row outside the loaded window simply goes
    /// unmarked — the bar stands either way). `hold` takes the answer as a press held down instead of a click, and
    /// `tip` is what the pill says on hover — the questions whose write leaves this machine ask that way (デザイン規約 §長押し).
    /// `form` is what the question needs in order to take an answer at all — a chooser, a name box. Most questions have
    /// none: they are answered by the pill and nothing else. `code` is the git command the question is about, said at
    /// the head of the question and on the pill both, where the act has one word of its own (デザイン規約 §git 用語のコード表記);
    /// `accept` carries the wording everywhere else.
    function startAsking(oidHex, label, detail, accept, danger, hold = false, tip = "", form = null, code = "") {
        graphList.namingOid = ""
        graphList.namingText = ""
        // Before the label, which is what opens the bar: the form has to exist by the time opening decides where the
        // focus goes.
        askBar.form = form
        askBar.answerable = true
        askBar.neutral = false
        askBar.alert = false
        askBar.label = label
        askBar.detail = detail
        askBar.accept = accept
        askBar.code = code
        askBar.danger = danger
        askBar.hold = hold
        askBar.tip = tip
        graphList.askDanger = danger
        graphList.askOid = oidHex === undefined ? "" : oidHex
    }
    /// Whether the standing question can be answered yet. A question with a form turns this off until the form has
    /// something to send.
    property alias askAnswerable: askBar.answerable
    /// Whether it is asking for information rather than for consent, how it is answered, and what it says while it
    /// stands — these change under a publish question as the remote answers what the typed name means. The wording of
    /// the pill is not among them: the command it names is settled when the question opens (デザイン規約 §はじめてリモートへ送る).
    property alias askNeutral: askBar.neutral
    property alias askHold: askBar.hold
    /// The command the pill answers with, and whether the far side could be read at all — both move under a publish
    /// question as the remote answers, because what would run depends on what is over there.
    property alias askCode: askBar.code
    property alias askAlert: askBar.alert
    property alias askDetail: askBar.detail
    property alias askTip: askBar.tip
    /// The live form, so its owner can read what was typed into it.
    readonly property alias askForm: askBar.formItem
    function stopAsking() {
        askBar.label = ""
        graphList.askOid = ""
    }
    /// Automation: answer a held question the way a person does, by keeping the pill down to the end.
    function completeHold() {
        askBar.completeHold()
    }
    signal askConfirmed()
    signal askCancelled()

    /// The list itself — for automation hooks (bench / scroll-to / screenshot flows) only; app code goes through the
    /// functions.
    readonly property alias view: graphList

    /// Moves the selection without scrolling.
    function setCurrentRow(row) {
        graphList.currentIndex = row
    }

    // ---- walking the history with the arrow keys, and where the view
    // stands for it (規約 §矢印で履歴を辿る) ---------------------------
    GraphRowWalk {
        id: rowWalk
        view: graphList
        graphModel: graphArea.graphModel
        asking: askBar.label !== ""
        onActivated: oidHex => graphArea.rowActivated(oidHex)
    }
    function stepRow(delta) { return rowWalk.stepRow(delta) }
    function stepLanding(row, wasY) { return rowWalk.stepLanding(row, wasY) }
    function jumpToRow(row) { rowWalk.jumpToRow(row) }
    function anchorSoon() { rowWalk.anchorSoon() }
    function shiftRows(rows) { rowWalk.shiftRows(rows) }
    function showRowSoon(row) { rowWalk.showRowSoon(row) }

    // The three columns' widths, and how far the lanes have been sent sideways. Worked out in GraphColumnMetrics; the
    // pane's own name for each answer is the alias below it, so the rows, the dividers, the find bar and the page all
    // keep reading them off this pane.
    GraphColumnMetrics {
        id: metrics
        paneW: graphArea.width
        maxLanes: graphArea.graphModel.maxLanes
    }
    property alias labelWManual: metrics.labelWManual
    property alias graphColWManual: metrics.graphColWManual
    readonly property alias labelW: metrics.labelW
    readonly property alias contentMinW: metrics.contentMinW
    readonly property alias graphFullW: metrics.graphFullW
    readonly property alias graphColWMin: metrics.graphColWMin
    readonly property alias graphColWMax: metrics.graphColWMax
    readonly property alias graphColW: metrics.graphColW
    property alias graphX: metrics.graphX
    readonly property alias graphXMax: metrics.graphXMax
    readonly property alias subjectTextX: metrics.subjectTextX

    color: Theme.bgSurface

    // Divider hover-lines live under the list so the message ticks and lane strokes stay in front.
    Rectangle {
        id: labelDividerLine
        x: columnDividers.labelX + Theme.borderWidth
        width: Theme.splitterWidth - 2 * Theme.borderWidth
        height: parent.height
        color: Theme.borderStrong
        visible: columnDividers.labelLineWanted
    }
    Rectangle {
        id: graphDividerLine
        x: columnDividers.graphDividerX + Theme.borderWidth
        width: Theme.splitterWidth - 2 * Theme.borderWidth
        height: parent.height
        color: Theme.borderStrong
        visible: columnDividers.graphLineWanted
    }
    // The list starts under the bar — the graph moves down rather than losing its top rows behind it.
    AskBar {
        id: askBar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        z: 3
        onConfirmed: graphArea.askConfirmed()
        onCancelled: graphArea.askCancelled()
    }
    // Hangs from the top-right corner, over the list rather than above it — declared here rather than inside the view
    // because a Flickable adopts what is declared in it and scrolls it away. Only one of the two is ever up: a question
    // already standing keeps the place, since it is one gesture from being over.
    GraphFind {
        id: findBar
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceLg
        z: 4
        graphModel: graphArea.graphModel
        view: graphList
        asking: askBar.label !== ""
        // How far left the card may reach: `spaceXs` past where a subject starts, which is about half of the first
        // character (§コミットを探す). Measured from the columns rather than from the pane, so the cap follows the dividers
        // when they are dragged — it is a distance from the tick the messages begin at, not a fraction of the window.
        maxWidth: graphArea.width - graphArea.subjectTextX - Theme.spaceXs - anchors.rightMargin
        onDismissed: {
            findBar.runFind()
            graphList.takeKeyboard()
        }
        onLanded: oidHex => graphArea.findLanded(oidHex)
    }
    GraphList {
        id: graphList
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.top: askBar.bottom
        // The card hangs over the top rows rather than pushing them down — but when the newest commit is itself one of
        // the answers, the graph steps down by the card's height so that answer is not the one thing the search covers.
        // It goes back the moment the top row stops matching, so the band is not a place the eye learns to expect (規約
        // §コミットを探す).
        anchors.topMargin: findBar.findClears ? findBar.height : 0
        Behavior on anchors.topMargin {
            NumberAnimation { duration: 200 }
        }
        graphModel: graphArea.graphModel
        workTree: graphArea.workTree
        columns: metrics
        findOn: findBar.findOn
        chipListAnchor: graphArea.chipListAnchor
        rowCardOid: graphArea.rowCardOid
        // The arrows are answered here rather than in the list, because it is this pane that walks the history with
        // them and the page that hears where they landed (規約 §矢印で履歴を辿る).
        Keys.onUpPressed: event => event.accepted = graphArea.stepRow(-1)
        Keys.onDownPressed: event => event.accepted = graphArea.stepRow(1)
        onRowMenuRequested: oidHex => graphArea.rowMenuOpenRequested(oidHex)
        onChipMenuRequested: (oidHex, record) => graphArea.chipMenuOpenRequested(oidHex, record)
        onRowSelected: oidHex => graphArea.rowActivated(oidHex)
        onRowSwitchRequested: (oidHex, record) => graphArea.rowSwitchRequested(oidHex, record)
        onChipExpandRequested: (records, anchor) => graphArea.chipExpandRequested(records, anchor)
        onChipCollapseRequested: graphArea.chipCollapseRequested()
        onRowHoverRequested: (row, inside) => graphArea.rowHoverRequested(row, inside)
        onNamingSubmitted: (oidHex, name) => {
            graphArea.stopNaming()
            // An empty box is the way out of the offer, not a branch called nothing.
            if (name !== "")
                graphArea.createBranchRequested(oidHex, name)
        }
        onNamingCancelled: graphArea.stopNaming()
        onWheelTaken: graphArea.autoScrolling = false
        onWheelPanned: delta => graphArea.graphX = Math.max(0,
            Math.min(graphArea.graphX - delta / 2, graphArea.graphXMax))
    }
    /// Whether a middle-click autoscroll is under way, and whether it carries the lanes sideways as well — read by the
    /// wheel, which ends the gesture, and by the automation hook.
    property alias autoScrolling: autoScroll.scrolling
    readonly property alias autoPanning: autoScroll.panning
    /// Starts autoscroll from a point in this pane's frame, and moves the pointer of one already under way. The presses
    /// and the automation hook both come through here.
    function startAutoScroll(x, y) {
        autoScroll.start(x, y)
    }
    function driftPointer(x, y) {
        autoScroll.drift(x, y)
    }
    MiddleAutoScroll {
        id: autoScroll
        anchors.fill: parent
        panFrom: graphArea.labelW
        panTo: graphArea.labelW + graphArea.graphColW
        canPan: graphArea.graphXMax > 0
        onDrifted: (dy, dx) => {
            graphList.contentY = graphList.clampY(graphList.contentY + dy)
            if (dx !== 0)
                graphArea.graphX = Math.max(0, Math.min(graphArea.graphX + dx, graphArea.graphXMax))
        }
    }
    GraphLanePan {
        columns: metrics
        view: graphList
        graphModel: graphArea.graphModel
    }
    // The current branch's stand-in, riding whichever edge its own row went out of. **Over the lane strip and under the
    // dividers**: the strip takes presses across the lane column, so a stand-in below it would answer its own lanes
    // with the row scrolling underneath — and the dividers stay on top, because a boundary that can be dragged is only
    // a few pixels wide wherever it crosses. It follows the list's frame instead of living in it (`view`), which is
    // what keeps it still while the rows go by.
    GraphHeadPin {
        id: headPin
        z: 1
        graphModel: graphArea.graphModel
        view: graphList
        labelWidth: graphArea.labelW
        graphColWidth: graphArea.graphColW
        graphFullWidth: graphArea.graphFullW
        graphXOffset: graphArea.graphX
        // The list's own bar, which this lies on top of. Read off the bar rather than off the token: the width is the
        // style's, and a guess would leave either a strip of trough taken or a strip of stand-in that answers nothing.
        barRoom: graphList.ScrollBar.vertical.visible ? graphList.ScrollBar.vertical.width : 0
        onActivated: row => {
            graphList.takeKeyboard()
            graphArea.jumpToRow(row)
            graphArea.rowActivated(graphArea.graphModel.oidAt(row))
        }
    }
    /// The stand-in itself — automation-only exposure, the same one `view` is (app-ui.md). A headless run reads what it
    /// drew (`visible` / `rowAbove` / `lit`), rests the pointer on it by writing the one property the pointer's own
    /// arrival writes, and presses it through its own signal: five names on this pane said nothing the item does not.
    readonly property alias headPin: headPin

    // Draggable column dividers (labels | graph | message). The hand is in there; the two lines it raises are drawn
    // above, under the list. Same seat in the stack as the two dividers had: over the lane pan, under the lane bar.
    GraphColumnDividers {
        id: columnDividers
        anchors.fill: parent
        z: 2
        columns: metrics
        blank: graphArea.blank
    }
    /// Whether either divider is refusing, and where the hand is while it does (**scene coordinates**) — the page draws
    /// the one badge, since a drag carries the hand out past this pane.
    readonly property alias refused: columnDividers.refused
    readonly property alias refusedAt: columnDividers.refusedAt
    /// Automation: the pointer resting on the graph divider, and a drag carried out past one of the four bounds these
    /// two dividers have.
    function restDividerPointer(inside) {
        columnDividers.restDividerPointer(inside)
    }
    function dragDividerPast(which) {
        columnDividers.dragDividerPast(which)
    }
    /// What is drawn, not what was asked for: the automation hook reports the line and the badge themselves, so a
    /// column that cannot be resized but still promises a drag cannot pass.
    readonly property alias graphDividerShown: columnDividers.graphDividerShown
    readonly property alias graphDividerLineShown: graphDividerLine.visible
    /// Whether the badge the page draws for this pane is up. The page owns it, so this is the pane's half of that
    /// answer.
    readonly property alias graphDividerRefuses: columnDividers.refused
    /// The line of whichever divider has the hand. A refused drag has to leave it drawn — the boundary still moves the
    /// other way — so this is the half of the picture the badge does not hold, and it is the pane that knows which of
    /// the two lines is being asked about.
    readonly property bool refusedLineShown: columnDividers.labelDragging ? labelDividerLine.visible
        : graphDividerLine.visible
    /// Whether the pointer is anywhere in this pane. A `HoverHandler` rather than a `MouseArea`: handlers are passive,
    /// so the rows', chips' and dividers' own hover does not take this one away. Real hover and the automation hook
    /// write the same property — hover cannot be injected (verify-ui).
    ///
    /// **It stays on the pane itself.** Moved into an item stacked over the list it took the hover away from every row
    /// under it: hover goes to the topmost item that accepts it, and an item carrying a handler accepts it for its
    /// whole area (2026-08-22 qmltestrunner で実測 — rows that would not light, cards that would not close).
    property bool pointerInside: false
    HoverHandler {
        onHoveredChanged: graphArea.pointerInside = hovered
    }
    /// Automation: the pointer resting in the pane, which is the only thing that puts the lane bar on screen
    /// (`PG_AUTO_ACT=graph-bar`).
    function restPointer(inside) {
        graphArea.pointerInside = inside
    }
    /// What is drawn, not what was asked for: the automation hook reports the bar itself so a broken binding cannot
    /// pass.
    readonly property alias laneBarShown: laneBar.visible
    // The lanes' horizontal bar (`GraphLaneBar`), on the pane's bottom edge. **Its own `z`, on the bar itself**: a QML
    // stack is the parent's one number, and at the default it would go under the lane strip and the dividers it has to
    // sit on top of.
    GraphLaneBar {
        id: laneBar
        anchors.bottom: parent.bottom
        z: 2
        columns: metrics
        pointerInside: graphArea.pointerInside
    }
    // What stands in the middle while there is no history to draw.
    GraphEmptyState {
        anchors.fill: parent
        graphModel: graphArea.graphModel
        blank: graphArea.blank
        onOpenRepositoryRequested: graphArea.openRepositoryRequested()
    }
}
