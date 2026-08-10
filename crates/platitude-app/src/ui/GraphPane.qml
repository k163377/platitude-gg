pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Center pane: the commit graph. Owns its column geometry (label /
// lane widths, horizontal pan) and every in-pane interaction (wheel,
// middle-click autoscroll, draggable dividers); row clicks go up as
// signals and the owner decides what selection means.
Rectangle {
    id: graphArea

    required property var graphModel
    // WIP row's file count rides on the ListView for the delegate.
    required property var worktreeModel
    // No repository behind this pane: the empty-window call to action.
    property bool blank: false

    /// A row was clicked (or programmatically activated).
    signal rowActivated(string oidHex)
    /// Right-click on a row. What the row is decides which menu opens,
    /// and that is the page's call.
    signal rowMenuOpenRequested(string oidHex)
    /// A row was double-clicked. `record` is the chip it shows (kind +
    /// flags + name); empty when the row shows no branch at all.
    signal rowSwitchRequested(string oidHex, string record)
    /// The chip whose stacked list the page has out (null when none).
    /// Rows read it back through the view: a hand that walked down into
    /// the list and comes back to this chip is not opening anything, so
    /// it is not made to sit out the opening rest again.
    property var chipListAnchor: null
    /// A stacked chip was hovered: unstack it under the chip.
    signal chipExpandRequested(var records, var anchor)
    /// The pointer settled on a row (or left it): open the commit card
    /// under it. `row` is the delegate, which the page needs for its
    /// position and its fields — it must not hold on to it.
    signal rowHoverRequested(var row, bool inside)
    /// The pointer left that chip — put it back, unless it went into the
    /// list itself (only the owner can tell).
    signal chipCollapseRequested()
    /// A name was typed into a row that had no branch on it.
    signal createBranchRequested(string oidHex, string name)
    /// The blank pane's "Open repository…" button.
    signal openRepositoryRequested()

    // ---- naming a branch on a row that has none --------------------
    // In-pane state, like the column widths: the page says where to
    // start and hears back only when there is a name to act on.
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

    // ---- a standing question ---------------------------------------
    // The bar comes down from the top of the pane and pushes the history
    // down (デザイン規約 §可否・警告の出し場所); the row it concerns is
    // marked rather than worded, so the question is written exactly once.

    /// The find bar's state, and its box, for the headless run — the key
    /// that opens it cannot be pressed from there.
    readonly property alias findOpen: findBar.open
    property alias findQuery: findBar.query

    /// Brings the find bar down over this list. Refused while a question
    /// is standing: two bars from one edge would leave neither readable,
    /// and the question is the one with something waiting on it.
    function startFind() {
        if (askBar.label !== "")
            return
        findBar.raise()
    }

    /// Raises the bar. `oidHex` is the row it is about ("" for none, and
    /// a row outside the loaded window simply goes unmarked — the bar
    /// stands either way).
    /// `hold` takes the answer as a press held down instead of a click,
    /// and `tip` is what the pill says on hover — the questions whose
    /// write leaves this machine ask that way (デザイン規約 §長押し).
    /// `form` is what the question needs in order to take an answer at all
    /// — a chooser, a name box. Most questions have none: they are
    /// answered by the pill and nothing else.
    function startAsking(oidHex, label, detail, accept, danger,
                         hold = false, tip = "", form = null) {
        graphList.namingOid = ""
        graphList.namingText = ""
        // Before the label, which is what opens the bar: the form has to
        // exist by the time opening decides where the focus goes.
        askBar.form = form
        askBar.answerable = true
        askBar.neutral = false
        askBar.label = label
        askBar.detail = detail
        askBar.accept = accept
        askBar.danger = danger
        askBar.hold = hold
        askBar.tip = tip
        graphList.askDanger = danger
        graphList.askOid = oidHex === undefined ? "" : oidHex
    }
    /// Whether the standing question can be answered yet. A question with
    /// a form turns this off until the form has something to send.
    property alias askAnswerable: askBar.answerable
    /// Whether it is asking for information rather than for consent, and
    /// the words on its pill and its tooltip — all three change under a
    /// publish question as the remote answers what the typed name means.
    property alias askNeutral: askBar.neutral
    property alias askAccept: askBar.accept
    property alias askHold: askBar.hold
    property alias askDetail: askBar.detail
    property alias askTip: askBar.tip
    /// The live form, so its owner can read what was typed into it.
    readonly property alias askForm: askBar.formItem
    function stopAsking() {
        askBar.label = ""
        graphList.askOid = ""
    }
    /// Automation: answer a held question the way a person does, by
    /// keeping the pill down to the end.
    function completeHold() {
        askBar.completeHold()
    }
    /// The bar's accept was clicked; the page runs what it guarded.
    signal askConfirmed()
    /// The question was walked away from.
    signal askCancelled()

    /// The list itself — for automation hooks (bench / scroll-to /
    /// screenshot flows) only; app code goes through the functions.
    readonly property alias view: graphList

    /// Moves the selection without scrolling.
    function setCurrentRow(row) {
        graphList.currentIndex = row
    }
    /// Moves the selection and centers the viewport on it.
    function jumpToRow(row) {
        graphList.currentIndex = row
        graphList.positionViewAtIndex(row, ListView.Center)
    }
    /// Re-centers on the current row a beat from now. Centering must
    /// outlive the ListView's own relayout: a model reset (tag swap /
    /// reload) zeroes contentY during the polish that runs after our
    /// handlers, so the anchor is applied a beat later.
    function anchorSoon() {
        anchorTimer.restart()
    }
    /// Puts the viewport back over the rows it was reading after `rows`
    /// commits arrived above them. A background rebuild replaces the graph
    /// by writing over the rows in place, so newcomers at the top slide
    /// everything below them down while the view stays where it is and
    /// quietly shows different commits — the further down someone is
    /// reading, the more that costs them.
    ///
    /// At the top of the list there is nothing to preserve: the newest
    /// commits are exactly what belongs there, so it stays pinned.
    /// Applied a beat later, for the same reason as the anchor above: the
    /// list has not laid the new rows out yet, so its content is still the
    /// old height and clamping against it would swallow the correction.
    function shiftRows(rows) {
        if (rows === 0)
            return
        if (graphList.contentY <= graphList.originY - graphList.topMargin)
            return
        shiftTimer.pending += rows
        shiftTimer.restart()
    }
    Timer {
        id: shiftTimer
        property int pending: 0
        interval: Metrics.anchorDelayMs
        onTriggered: {
            const rows = shiftTimer.pending
            shiftTimer.pending = 0
            graphList.contentY = graphList.clampY(
                graphList.contentY + rows * Theme.graphRowHeight)
        }
    }
    Timer {
        id: anchorTimer
        interval: Metrics.anchorDelayMs
        onTriggered: {
            if (graphList.currentIndex >= 0)
                graphList.positionViewAtIndex(graphList.currentIndex, ListView.Center)
        }
    }

    // Adjustable column widths (labels / graph); -1 = automatic. The
    // graph column starts at a default lane count and scrolls
    // horizontally when the full graph is wider.
    property real labelWManual: -1
    property real graphColWManual: -1
    // The narrowest the chip column goes, and the widest. `spaceXxl` is
    // where the divider has always refused to be dragged any further in.
    // The far end is where the two columns after it would stop being able
    // to say anything: one lane, and enough message column to show that
    // there is a message (規約 §グラフ列は最も広い所のレーンまで).
    readonly property real labelColWMin: Theme.spaceXxl
    readonly property real labelColWMax: Math.max(labelColWMin,
        width - graphColWMin - Metrics.messageMinW)
    // The chip column does not give when the pane narrows — the lanes do
    // (`graphColWMax`), and the window's floor holds this one's width in
    // reserve (規約 §窓の床). Squeezing it was tried and taken back out:
    // a column narrower than a chip draws a crushed one, and a column
    // that follows the pane changes width whenever the left menu folds,
    // which is a thing moving on screen that nobody asked to move
    // (2026-08-09 ユーザー報告 — 3 つの症状が全部これだった).
    //
    // A column somebody dragged is still held inside what the pane can
    // lay out: that one is as wide as a hand made it, and the message
    // column has to survive the window being narrowed afterwards.
    readonly property real labelW:
        labelWManual >= 0
        ? Math.max(labelColWMin, Math.min(labelWManual, labelColWMax))
        : Metrics.labelColW
    // The narrowest this pane can be laid out with all three columns still
    // saying something. The page's floor is built on it (RepoPage), which
    // is what keeps the window from being dragged past it.
    readonly property real contentMinW:
        Metrics.labelColW + graphColWMin + Metrics.messageMinW
    readonly property real graphFullW: Metrics.laneInset
                                       + Math.max(1, graphModel.maxLanes) * Metrics.laneW
                                       + Theme.spaceSm
    // The narrowest the column goes: one lane, with the rest sent
    // sideways. One place, so the display, the divider's clamp and the
    // question of whether there is a drag in it at all agree.
    readonly property real graphColWMin: Metrics.laneInset + Metrics.laneW
    // How far the divider may be pulled: as wide as the lanes ever get,
    // and no wider — a column past the last lane is emptiness taken from
    // the message column. A narrow window stops it earlier still, where
    // the message column would stop showing that a message is there
    // (規約 §グラフ列は最も広い所のレーンまで).
    readonly property real graphColWMax: Math.max(graphColWMin,
        Math.min(graphFullW, width - labelW - Metrics.messageMinW))
    /// Whether the column is the only width it can be. One lane and its
    /// gutter is the whole of what a linear history has to show, so the
    /// divider draws no line and turns the cursor away rather than
    /// taking drags that come to nothing
    /// (規約 §グラフ列は最も広い所のレーンまで).
    readonly property bool graphColWFixed: graphColWMax <= graphColWMin + Theme.spaceSm
    readonly property real graphColW: Math.min(graphColWMax,
        graphColWManual >= 0 ? Math.max(graphColWManual, graphColWMin)
                             : Metrics.laneInset + Metrics.graphDefaultLanes * Metrics.laneW
                               + Theme.spaceSm)
    property real graphX: 0
    readonly property real graphXMax: Math.max(0, graphFullW - graphColW)
    onGraphXMaxChanged: graphX = Math.min(graphX, graphXMax)

    color: Theme.bgSurface

    // Divider hover-lines live under the list so the message ticks and
    // lane strokes stay in front.
    Rectangle {
        x: labelDivider.x + Theme.borderWidth
        width: Theme.splitterWidth - 2 * Theme.borderWidth
        height: parent.height
        color: Theme.borderStrong
        visible: labelDivider.containsMouse || labelDivider.pressed
    }
    Rectangle {
        id: graphDividerLine
        x: graphDivider.x + Theme.borderWidth
        width: Theme.splitterWidth - 2 * Theme.borderWidth
        height: parent.height
        color: Theme.borderStrong
        // The line says "this moves". A column with one width does not,
        // so it stays out and the cursor speaks instead
        // (規約 §グラフ列は最も広い所のレーンまで).
        visible: !graphArea.graphColWFixed
                 && (graphArea.dividerPointed || graphDivider.pressed)
    }
    // The list starts under the bar — the graph moves down rather than
    // losing its top rows behind it.
    AskBar {
        id: askBar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        z: 3
        onConfirmed: graphArea.askConfirmed()
        onCancelled: graphArea.askCancelled()
    }
    // Hangs from the top-right corner, over the list rather than above it
    // — declared here rather than inside the view because a Flickable
    // adopts what is declared in it and scrolls it away. Only one of the
    // two is ever up: a question already standing keeps the place, since
    // it is one gesture from being over.
    FindBar {
        id: findBar
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceLg
        z: 4
    }
    ListView {
        id: graphList
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.top: askBar.bottom
        clip: true
        model: graphArea.graphModel
        reuseItems: true
        // Nothing but this pane's own functions move the view. Left on,
        // the list chases its current item: a background rebuild that
        // re-resolves the selection onto a row one further down drags a
        // reader parked in the history to wherever the selection is. The
        // graph has no key navigation to want the chase for.
        highlightFollowsCurrentItem: false
        boundsBehavior: Flickable.StopAtBounds
        flickDeceleration: 8000
        maximumFlickVelocity: 9000
        ScrollBar.vertical: AutoScrollBar {}
        // The graph is the one pane with no header band; this sliver of
        // margin drops the first row so its bottom line meets the
        // neighbouring bands' bottom edge when scrolled to the top.
        topMargin: Theme.headerHeight - Theme.graphRowHeight
        // A sliver of run-out at the end: without it the oldest row
        // sits flush on the pane edge and reads as clipped rather than
        // as the end of what is loaded. Just enough to see the break.
        bottomMargin: Theme.spaceSm
        // Bridge into the delegate (GraphRowDelegate reads its column
        // geometry off ListView.view).
        property real labelWidth: graphArea.labelW
        property real graphColWidth: graphArea.graphColW
        property real graphFullWidth: graphArea.graphFullW
        property real graphXOffset: graphArea.graphX
        property int wipCount: graphArea.worktreeModel.total
        // Which row's chip column is a name box, and what has been typed
        // into it. Held here rather than in the delegate: the delegate is
        // recycled the moment its row scrolls off.
        property string namingOid: ""
        property string namingText: ""
        // Which row the standing question is about, and in which tone —
        // held here for the same recycling reason. The words are on the
        // bar; the row only marks itself.
        property string askOid: ""
        property bool askDanger: false
        signal rowSelected(string oidHex)
        signal rowMenuRequested(string oidHex)
        signal rowSwitchRequested(string oidHex, string record)
        signal chipExpandRequested(var records, var anchor)
        signal chipCollapseRequested()
        signal rowHoverRequested(var row, bool inside)
        // Mirrored for the delegates, which can only see the view.
        readonly property var chipListAnchor: graphArea.chipListAnchor
        signal namingSubmitted(string oidHex, string name)
        signal namingCancelled()
        onRowMenuRequested: oidHex => graphArea.rowMenuOpenRequested(oidHex)
        onRowSelected: oidHex => graphArea.rowActivated(oidHex)
        onRowSwitchRequested: (oidHex, record) =>
            graphArea.rowSwitchRequested(oidHex, record)
        onChipExpandRequested: (records, anchor) =>
            graphArea.chipExpandRequested(records, anchor)
        onChipCollapseRequested: graphArea.chipCollapseRequested()
        onRowHoverRequested: (row, inside) =>
            graphArea.rowHoverRequested(row, inside)
        onNamingSubmitted: (oidHex, name) => {
            graphArea.stopNaming()
            // An empty box is the way out of the offer, not a branch
            // called nothing.
            if (name !== "")
                graphArea.createBranchRequested(oidHex, name)
        }
        onNamingCancelled: graphArea.stopNaming()
        delegate: GraphRowDelegate {}
        // Window cut: lanes keep running through the footer and the
        // message sits where subjects go. One row tall — the lanes carry
        // on for exactly one more commit's worth, which is all it takes
        // to read as "and it continues" — unless the message needs more
        // than that. It is the only thing explaining the cut, so a narrow
        // subject column grows the footer rather than eliding it.
        footer: Item {
            width: graphList.width
            height: graphArea.graphModel.truncated
                    ? Math.max(Theme.graphRowHeight,
                               tailMessage.contentHeight + 2 * Theme.spaceXs)
                    : 0
            visible: graphArea.graphModel.truncated
            Item {
                x: graphList.labelWidth
                width: graphList.graphColWidth
                height: parent.height
                clip: true
                Canvas {
                    id: tailCanvas
                    x: -graphList.graphXOffset
                    width: graphList.graphFullWidth
                    height: parent.height
                    onPaint: {
                        const ctx = getContext("2d")
                        ctx.clearRect(0, 0, width, height)
                        if (graphArea.graphModel.tailGeometry === "")
                            return
                        ctx.lineWidth = Metrics.laneStroke
                        ctx.globalAlpha = Metrics.tailFade
                        // Same tokens as a row's geometry (uppercase =
                        // dashed leash): a stash or WIP row whose target
                        // sits past the cut keeps dotting through here.
                        const toks = graphArea.graphModel.tailGeometry.split(";")
                        for (let n = 0; n < toks.length; n++) {
                            const t = toks[n]
                            const dot = t.indexOf(".")
                            const lane = parseInt(t.substring(1, dot))
                            const color = parseInt(t.substring(dot + 1))
                            const x = Metrics.laneInset + lane * Metrics.laneW + Metrics.laneW / 2
                            ctx.strokeStyle = Theme.graphLane[color % Theme.graphLane.length]
                            ctx.setLineDash(t[0] === t[0].toLowerCase()
                                            ? [] : Metrics.laneDash)
                            ctx.beginPath()
                            ctx.moveTo(x, 0)
                            ctx.lineTo(x, height)
                            ctx.stroke()
                        }
                        ctx.setLineDash([])
                    }
                    Connections {
                        target: graphArea.graphModel
                        function onStatsChanged() { tailCanvas.requestPaint() }
                    }
                }
            }
            // Bounded like a subject: the message stays in the subject
            // column rather than running under the next pane. Told in the
            // secondary colour, not a state one: the window is how the
            // graph is meant to work, and nothing is waiting on it
            // (デザイン規約 §状態).
            Label {
                id: tailMessage
                x: graphList.labelWidth + graphList.graphColWidth
                width: graphList.width - x
                // Its own laid-out height, centered in whatever the
                // footer ends up being: reading the footer's height back
                // here is the binding loop, since the footer is sized
                // from this. Nothing elides — the height follows the
                // wrap, so every line of the message is shown.
                height: contentHeight
                y: (parent.height - height) / 2
                leftPadding: Theme.spaceSm
                rightPadding: Theme.spaceSm
                wrapMode: Text.Wrap
                // The walk's own count, not the row count: the WIP row and
                // sifted stash parents move rows off the round window
                // limit, and this footer only stands when the walk hit it.
                text: qsTr("Only the first %L1 commits are loaded")
                      .arg(graphArea.graphModel.walkedTotal)
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMd
                font.weight: Font.DemiBold
            }
        }
        // Manual contentY math must respect originY: after
        // positionViewAtIndex jumps, the ListView shifts its coordinate
        // origin as item positions are fixed up, so
        // [0, contentHeight-height] no longer matches the real scroll
        // range (top rows become unreachable, the bottom overshoots the
        // truncation footer).
        function clampY(y) {
            // topMargin lives above the content origin — forgetting it
            // makes the top gap unreachable by wheel after any scroll.
            const minY = graphList.originY - graphList.topMargin
            const maxY = Math.max(minY, graphList.originY
                                        + graphList.contentHeight
                                        - graphList.height
                                        + graphList.bottomMargin)
            return Math.max(minY, Math.min(y, maxY))
        }
        // Mouse wheels scroll a fixed number of rows per notch;
        // touchpads keep native Flickable panning.
        WheelHandler {
            acceptedDevices: PointerDevice.Mouse
            onWheel: event => {
                // Wheel input exits middle-click autoscroll
                // (Chrome-like behavior).
                graphArea.autoScrolling = false
                graphList.cancelFlick()
                if (event.angleDelta.x !== 0)
                    graphArea.graphX = Math.max(0, Math.min(
                        graphArea.graphX - event.angleDelta.x / 2, graphArea.graphXMax))
                const step = (event.angleDelta.y / 120)
                           * Metrics.wheelRows * Theme.graphRowHeight
                graphList.contentY = graphList.clampY(graphList.contentY - step)
            }
        }
    }
    // Middle-click toggles autoscroll mode: the pointer distance from
    // the anchor sets the speed; any click exits.
    property bool autoScrolling: false
    property real autoAnchorX: 0
    property real autoAnchorY: 0
    property real autoCurrentX: 0
    property real autoCurrentY: 0
    /// Whether this autoscroll carries the lanes sideways as well. Decided
    /// by where the middle click landed, and kept for the whole gesture
    /// (デザイン規約 §グラフを横へ送る).
    property bool autoPanning: false
    /// Starts autoscroll from a point in this pane's frame. The press and
    /// the automation hook both come through here, so which column offers
    /// the sideways drift is answered in exactly one place.
    function startAutoScroll(x, y) {
        graphArea.autoAnchorX = x
        graphArea.autoAnchorY = y
        graphArea.autoCurrentX = x
        graphArea.autoCurrentY = y
        graphArea.autoPanning = x >= graphArea.labelW
                                && x < graphArea.labelW + graphArea.graphColW
        graphArea.autoScrolling = true
    }
    /// Where the pointer has drifted to since. Its distance from the
    /// anchor is what the ticker below reads as speed — the moving
    /// pointer and the automation hook write the same two values.
    function driftPointer(x, y) {
        graphArea.autoCurrentX = x
        graphArea.autoCurrentY = y
    }
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.MiddleButton
        onClicked: mouse => graphArea.startAutoScroll(mouse.x, mouse.y)
    }
    MouseArea {
        visible: graphArea.autoScrolling
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.AllButtons
        // The cursor says which ways this gesture goes, so a press that
        // did not land on the lanes does not look broken when the lanes
        // stay put under a sideways drift.
        cursorShape: graphArea.autoPanning ? Qt.SizeAllCursor : Qt.SizeVerCursor
        onPositionChanged: mouse => graphArea.driftPointer(mouse.x, mouse.y)
        onPressed: mouse => {
            graphArea.autoScrolling = false
            mouse.accepted = true
        }
        Timer {
            running: graphArea.autoScrolling
            interval: 16
            repeat: true
            onTriggered: {
                const delta = (graphArea.autoCurrentY - graphArea.autoAnchorY)
                            * Metrics.middleScrollGain
                graphList.contentY = graphList.clampY(graphList.contentY + delta)
                // Sideways drift pans the lanes — for a gesture that
                // started on them.
                if (graphArea.autoPanning && graphArea.graphXMax > 0) {
                    const dx = (graphArea.autoCurrentX - graphArea.autoAnchorX)
                             * Metrics.middleScrollGain
                    graphArea.graphX = Math.max(0, Math.min(graphArea.graphX + dx,
                                                            graphArea.graphXMax))
                }
            }
        }
        // Anchor marker
        Rectangle {
            x: graphArea.autoAnchorX - Theme.iconMd / 2
            y: graphArea.autoAnchorY - Theme.iconMd / 2
            width: Theme.iconMd
            height: Theme.iconMd
            radius: Theme.iconMd / 2
            color: "transparent"
            border.color: Theme.borderStrong
            border.width: Theme.borderWidth
            Rectangle {
                anchors.centerIn: parent
                width: Theme.spaceXs
                height: Theme.spaceXs
                radius: Theme.spaceXs / 2
                color: Theme.borderStrong
            }
        }
    }
    // Left-drag inside the lanes pans them horizontally when they
    // overflow; a motionless press-release still selects the row
    // underneath.
    MouseArea {
        id: lanePan
        x: graphArea.labelW
        width: graphArea.graphColW
        height: parent.height
        z: 1
        visible: graphArea.graphXMax > 0
        acceptedButtons: Qt.LeftButton
        property real pressX: 0
        property real startGX: 0
        property bool panning: false
        onPressed: mouse => {
            pressX = mouse.x
            startGX = graphArea.graphX
            panning = false
        }
        onPositionChanged: mouse => {
            if (!pressed)
                return
            if (!panning && Math.abs(mouse.x - pressX) > Theme.spaceXs)
                panning = true
            if (panning)
                graphArea.graphX = Math.max(0, Math.min(
                    startGX - (mouse.x - pressX), graphArea.graphXMax))
        }
        // The lanes are part of the row, so a double-click on them means
        // what it means anywhere else on it. Without this the gesture
        // would die in exactly the repositories wide enough to need
        // panning, and nothing on screen would say why.
        onDoubleClicked: mouse => {
            // mouse.y is in this MouseArea's frame, which starts at the
            // pane's top; the list starts below the ask bar. Map, or a
            // standing question makes every lane click land rows lower.
            const p = lanePan.mapToItem(graphList, mouse.x, mouse.y)
            const idx = graphList.indexAt(graphArea.labelW + 1,
                                          graphList.contentY + p.y)
            if (idx < 0)
                return
            // Asked of the row itself, so which chip a row leads to is
            // worked out in exactly one place.
            const row = graphList.itemAtIndex(idx)
            if (row && row.movable)
                graphList.rowSwitchRequested(graphArea.graphModel.oidAt(idx),
                                             row.primaryRecord)
        }
        onReleased: mouse => {
            if (panning)
                return
            // Same frame correction as the double-click above.
            const p = lanePan.mapToItem(graphList, mouse.x, mouse.y)
            const idx = graphList.indexAt(graphArea.labelW + 1,
                                          graphList.contentY + p.y)
            if (idx >= 0) {
                graphList.currentIndex = idx
                graphList.rowSelected(graphArea.graphModel.oidAt(idx))
            }
        }
    }

    // Draggable column dividers (labels | graph | message).
    MouseArea {
        id: labelDivider
        x: graphArea.labelW - Theme.splitterWidth / 2
        width: Theme.splitterWidth
        height: parent.height
        z: 2
        visible: !graphArea.blank
        hoverEnabled: true
        cursorShape: Qt.SplitHCursor
        preventStealing: true
        onPositionChanged: mouse => {
            if (!pressed)
                return
            const nx = mapToItem(graphArea, mouse.x, 0).x
            // The same pair the column itself is clamped to, so a drag
            // cannot leave a width the pane would not lay out.
            graphArea.labelWManual = Math.max(graphArea.labelColWMin,
                Math.min(nx, graphArea.labelColWMax))
        }
    }
    MouseArea {
        id: graphDivider
        // Sits behind the message tick column so the hover line
        // overlaps the ticks.
        x: graphArea.labelW + graphArea.graphColW + Theme.spaceSm
           + Theme.borderWidth - Theme.splitterWidth / 2
        width: Theme.splitterWidth
        height: parent.height
        z: 2
        visible: !graphArea.blank
        hoverEnabled: true
        // The cursor never changes: it is the platform's splitter shape
        // in both states, and a column that will not move says so with
        // the badge below instead (規約 §グラフ列は最も広い所のレーン
        // まで). Swapping in a drawn arrow made the refusal read as a
        // different tool from the divider one column over.
        cursorShape: Qt.SplitHCursor
        preventStealing: true
        onContainsMouseChanged: {
            graphArea.dividerPointed = containsMouse
            // Entering does not always bring a move with it, and the
            // mark is drawn where this says the hand is.
            if (containsMouse)
                graphArea.dividerPoint = Qt.point(graphDivider.x + mouseX, mouseY)
        }
        onPositionChanged: mouse => {
            graphArea.dividerPoint = Qt.point(graphDivider.x + mouse.x, mouse.y)
            if (!pressed || graphArea.graphColWFixed)
                return
            const nx = mapToItem(graphArea, mouse.x, 0).x
                          - Theme.spaceSm - Theme.borderWidth
            graphArea.graphColWManual = Math.max(graphArea.graphColWMin,
                Math.min(nx - graphArea.labelW, graphArea.graphColWMax))
        }
    }
    /// Whether the pointer is on the graph divider, and where. Real hover
    /// and the automation hook write the same two — hover cannot be
    /// injected (verify-ui), and both the line and the mark have to be
    /// provable.
    property bool dividerPointed: false
    property point dividerPoint: Qt.point(0, 0)
    /// Automation: the pointer resting on the divider, at its middle
    /// (`PG_AUTO_ACT=graph-divider`). Where that is stays here rather
    /// than in the hook — one answer, not a second one to keep in step.
    function restDividerPointer(inside) {
        graphArea.dividerPointed = inside
        if (inside)
            graphArea.dividerPoint = Qt.point(graphDivider.x + graphDivider.width / 2,
                                              graphArea.height / 2)
    }
    // What a column that will not move answers with: the platform's own
    // cursor, untouched, and this badge below and right of it (規約
    // §グラフ列は最も広い所のレーンまで). Tucked into the corner the
    // cursor leaves empty — the platform's own badged cursors sit that
    // close, and a badge held further out reads as a separate mark
    // rather than as something the cursor is wearing.
    NavIcon {
        id: refusedCursor
        kind: "no"
        tint: Theme.textPrimary
        width: Theme.iconMd
        height: Theme.iconMd
        x: graphArea.dividerPoint.x + Theme.spaceXs
        y: graphArea.dividerPoint.y + Theme.spaceXs
        z: 4
        visible: graphArea.graphColWFixed && graphArea.dividerPointed
        // A Canvas that was never visible was never asked to paint, and
        // the first thing this one does is appear (app-ui.md).
        onVisibleChanged: if (visible) requestPaint()
    }
    /// What is drawn, not what was asked for: the automation hook reports
    /// the line and the badge themselves, so a column that cannot be
    /// resized but still promises a drag cannot pass.
    readonly property alias graphDividerShown: graphDivider.visible
    readonly property alias graphDividerLineShown: graphDividerLine.visible
    readonly property alias graphDividerRefuses: refusedCursor.visible
    /// Whether the pointer is anywhere in this pane. A `HoverHandler`
    /// rather than a `MouseArea`: handlers are passive, so the rows',
    /// chips' and dividers' own hover does not take this one away. Real
    /// hover and the automation hook write the same property — hover
    /// cannot be injected (verify-ui).
    property bool pointerInside: false
    HoverHandler {
        onHoveredChanged: graphArea.pointerInside = hovered
    }
    /// Automation: the pointer resting in the pane, which is the only
    /// thing that puts the lane bar on screen (`PG_AUTO_ACT=graph-bar`).
    function restPointer(inside) {
        graphArea.pointerInside = inside
    }
    /// What is drawn, not what was asked for: the automation hook reports
    /// the bar itself so a broken binding cannot pass.
    readonly property alias laneBarShown: laneBar.visible
    // Horizontal scroll of the lanes when the full graph is wider than
    // its column. The bar lies over the lanes of the last row, so it
    // comes out only while the pointer is in the pane — and stays out
    // for as long as it is being dragged, wherever that has taken the
    // pointer (デザイン規約 §グラフを横へ送る).
    ScrollBar {
        id: laneBar
        visible: graphArea.graphXMax > 0
                 && (graphArea.pointerInside || pressed)
        // Fusion draws its handle only in the style's "active" state,
        // which for a bar that is not attached to a Flickable means while
        // the pointer is on the bar itself — a 6px strip on the pane's
        // bottom edge that nobody would find. When the bar is out it is
        // because this pane put it there, so the style stops deciding.
        policy: ScrollBar.AlwaysOn
        orientation: Qt.Horizontal
        x: graphArea.labelW
        width: graphArea.graphColW
        anchors.bottom: parent.bottom
        z: 2
        size: graphArea.graphFullW > 0 ? graphArea.graphColW / graphArea.graphFullW : 1
        position: graphArea.graphFullW > 0 ? graphArea.graphX / graphArea.graphFullW : 0
        onPositionChanged: {
            if (pressed)
                graphArea.graphX = Math.max(0, Math.min(
                    position * graphArea.graphFullW, graphArea.graphXMax))
        }
    }
    // First load, before any row exists. The drawn ring, not Fusion's
    // BusyIndicator — the window has one turning mark and this is it
    // (規約 §進行中・長押しの定数).
    NavIcon {
        anchors.centerIn: parent
        width: Theme.iconLg
        height: Theme.iconLg
        kind: "spinner"
        tint: Theme.textSecondary
        visible: graphArea.graphModel.loading && graphArea.graphModel.rowTotal === 0
        // On the render thread, so it keeps turning while the GUI
        // thread drains models.
        RotationAnimator on rotation {
            running: graphArea.graphModel.loading
                     && graphArea.graphModel.rowTotal === 0
                     && AppBackend.shotDir === ""
            loops: Animation.Infinite
            from: 0
            to: 360
            duration: Metrics.spinMs
        }
    }
    Label {
        anchors.centerIn: parent
        visible: graphArea.graphModel.error !== ""
        text: graphArea.graphModel.error
        color: Theme.danger
        width: parent.width - 2 * Theme.spaceXl
        wrapMode: Text.Wrap
        horizontalAlignment: Text.AlignHCenter
    }
    // Empty window: the one thing worth doing sits in the column that
    // will hold the history.
    Column {
        anchors.centerIn: parent
        visible: graphArea.blank
        spacing: Theme.spaceLg
        Label {
            text: qsTr("Platitude GG")
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
            anchors.horizontalCenter: parent.horizontalCenter
        }
        Label {
            text: qsTr("A thin, fast GUI over your installed git.")
            color: Theme.textSecondary
            // Its natural width while the column has room for it, and
            // wrapped inside the column when it has not: at the window's
            // floor this pane is narrower than the line asks for, and
            // unbounded it took the difference from the panes on either
            // side (規約 §窓の床). The same shape the error line above
            // already has.
            width: Math.min(implicitWidth,
                            graphArea.width - 2 * Theme.spaceXl)
            wrapMode: Text.Wrap
            horizontalAlignment: Text.AlignHCenter
            anchors.horizontalCenter: parent.horizontalCenter
        }
        HoverButton {
            text: qsTr("Open repository…")
            anchors.horizontalCenter: parent.horizontalCenter
            onClicked: graphArea.openRepositoryRequested()
        }
    }
}
