pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
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
    /// A stacked chip was hovered: unstack it under the chip.
    signal chipExpandRequested(var records, var anchor)
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
    // One place for all of them (デザイン規約 §可否・警告の出し場所): the
    // bar comes down from the top of the pane and pushes the history
    // down — a question that covered what it is about would hide the
    // very thing being judged — and the row it concerns is marked rather
    // than worded, so the question is written exactly once.
    property string askLabel: ""
    property string askDetail: ""
    property string askAccept: ""
    property bool askDanger: false
    /// Raises the bar. `oidHex` is the row it is about ("" for none, and
    /// a row outside the loaded window simply goes unmarked — the bar
    /// stands either way).
    function startAsking(oidHex, label, detail, accept, danger) {
        graphList.namingOid = ""
        graphList.namingText = ""
        graphArea.askLabel = label
        graphArea.askDetail = detail
        graphArea.askAccept = accept
        graphArea.askDanger = danger
        graphList.askDanger = danger
        graphList.askOid = oidHex === undefined ? "" : oidHex
    }
    function stopAsking() {
        graphArea.askLabel = ""
        graphList.askOid = ""
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
    // graph column caps at a default lane count and scrolls
    // horizontally when the full graph is wider.
    property real labelWManual: -1
    property real graphColWManual: -1
    readonly property real labelW: labelWManual >= 0 ? labelWManual : Metrics.labelColW
    readonly property real graphFullW: Metrics.laneInset
                                       + Math.max(1, graphModel.maxLanes) * Metrics.laneW
                                       + Theme.spaceSm
    readonly property real graphColW: Math.min(graphFullW,
        graphColWManual >= 0 ? Math.max(graphColWManual, Metrics.laneInset + Metrics.laneW)
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
        x: graphDivider.x + Theme.borderWidth
        width: Theme.splitterWidth - 2 * Theme.borderWidth
        height: parent.height
        color: Theme.borderStrong
        visible: graphDivider.containsMouse || graphDivider.pressed
    }
    // The question bar. Sized by its own words, opened and closed with
    // the standard 200ms, and the list starts under it — the graph moves
    // down rather than losing its top rows behind the bar.
    Rectangle {
        id: askBar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        z: 3
        clip: true
        readonly property bool open: graphArea.askLabel !== ""
        readonly property color tone: graphArea.askDanger ? Theme.danger
                                                          : Theme.warning
        height: open ? askRow.implicitHeight + 2 * Theme.spaceMd : 0
        Behavior on height {
            NumberAnimation { duration: 200 }
        }
        color: Theme.bgElevated
        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: Theme.borderWidth
            color: askBar.tone
        }
        RowLayout {
            id: askRow
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: Theme.spaceMd
            spacing: Theme.spaceMd
            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                Label {
                    text: graphArea.askLabel
                    color: askBar.tone
                    font.pixelSize: Theme.fontMd
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
                // What answering costs, in the one line §用語 allows it.
                Label {
                    text: graphArea.askDetail
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
            }
            // Clicking this is the answer. It is the only thing on the bar
            // that acts, so nothing else here can be hit by accident.
            Rectangle {
                Layout.alignment: Qt.AlignVCenter
                implicitWidth: acceptLabel.implicitWidth + 2 * Theme.spaceMd
                implicitHeight: Theme.controlHeight
                radius: Theme.radiusSm
                color: acceptMouse.containsMouse ? Theme.bgHover : "transparent"
                border.color: askBar.tone
                border.width: Theme.borderWidth
                Label {
                    id: acceptLabel
                    anchors.centerIn: parent
                    text: graphArea.askAccept
                    color: askBar.tone
                    font.pixelSize: Theme.fontMd
                }
                MouseArea {
                    id: acceptMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: graphArea.askConfirmed()
                }
            }
            // Escape and a click anywhere else walk away too; this is the
            // way out that can be seen, for a bar that stands until it is
            // answered.
            Label {
                Layout.alignment: Qt.AlignVCenter
                text: "✕"
                color: dismissMouse.containsMouse ? Theme.textPrimary
                                                  : Theme.textSecondary
                font.pixelSize: Theme.fontMd
                MouseArea {
                    id: dismissMouse
                    anchors.fill: parent
                    anchors.margins: -Theme.spaceXs
                    hoverEnabled: true
                    onClicked: graphArea.askCancelled()
                }
            }
        }
    }
    // The bar has no focus of its own — nothing in the graph takes any —
    // so Escape is heard as a shortcut while it stands.
    Shortcut {
        // `sequences` rather than `sequence`: Cancel is more than one key
        // on some platforms, and binding the single form takes only the
        // first of them (Qt warns about exactly this).
        sequences: [StandardKey.Cancel]
        enabled: askBar.open
        onActivated: graphArea.askCancelled()
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
        signal namingSubmitted(string oidHex, string name)
        signal namingCancelled()
        onRowMenuRequested: oidHex => graphArea.rowMenuOpenRequested(oidHex)
        onRowSelected: oidHex => graphArea.rowActivated(oidHex)
        onRowSwitchRequested: (oidHex, record) =>
            graphArea.rowSwitchRequested(oidHex, record)
        onChipExpandRequested: (records, anchor) =>
            graphArea.chipExpandRequested(records, anchor)
        onChipCollapseRequested: graphArea.chipCollapseRequested()
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
        // message sits where subjects go.
        footer: Item {
            width: graphList.width
            height: graphArea.graphModel.truncated ? 2 * Theme.graphRowHeight : 0
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
                        ctx.globalAlpha = 0.45
                        const toks = graphArea.graphModel.tailGeometry.split(";")
                        for (let n = 0; n < toks.length; n++) {
                            const dot = toks[n].indexOf(".")
                            const lane = parseInt(toks[n].substring(0, dot))
                            const color = parseInt(toks[n].substring(dot + 1))
                            const x = Metrics.laneInset + lane * Metrics.laneW + Metrics.laneW / 2
                            ctx.strokeStyle = Theme.graphLane[color % Theme.graphLane.length]
                            ctx.beginPath()
                            ctx.moveTo(x, 0)
                            ctx.lineTo(x, height)
                            ctx.stroke()
                        }
                    }
                    Connections {
                        target: graphArea.graphModel
                        function onStatsChanged() { tailCanvas.requestPaint() }
                    }
                }
            }
            // Bounded like a subject: the message is the only thing
            // that explains the cut, so it wraps over the footer's two
            // rows rather than running under the next pane. Told in the
            // secondary colour, not a state one: the window is how the
            // graph is meant to work, and nothing is waiting on it
            // (デザイン規約 §状態).
            Label {
                x: graphList.labelWidth + graphList.graphColWidth
                width: graphList.width - x
                height: parent.height
                verticalAlignment: Text.AlignVCenter
                leftPadding: Theme.spaceSm
                rightPadding: Theme.spaceSm
                wrapMode: Text.Wrap
                elide: Text.ElideRight
                text: qsTr("Showing the first %L1 commits — older history is not loaded")
                      .arg(graphArea.graphModel.rowTotal)
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
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.MiddleButton
        onClicked: mouse => {
            graphArea.autoAnchorX = mouse.x
            graphArea.autoAnchorY = mouse.y
            graphArea.autoCurrentX = mouse.x
            graphArea.autoCurrentY = mouse.y
            graphArea.autoScrolling = true
        }
    }
    MouseArea {
        visible: graphArea.autoScrolling
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.AllButtons
        cursorShape: Qt.SizeAllCursor
        onPositionChanged: mouse => {
            graphArea.autoCurrentX = mouse.x
            graphArea.autoCurrentY = mouse.y
        }
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
                // Sideways drift pans the lanes.
                if (graphArea.graphXMax > 0) {
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
            const idx = graphList.indexAt(graphArea.labelW + 1,
                                          graphList.contentY + mouse.y)
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
            const idx = graphList.indexAt(graphArea.labelW + 1,
                                          graphList.contentY + mouse.y)
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
            graphArea.labelWManual = Math.max(Theme.spaceXxl,
                Math.min(nx, graphArea.width - 2 * Theme.spaceXxl))
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
        cursorShape: Qt.SplitHCursor
        preventStealing: true
        onPositionChanged: mouse => {
            if (!pressed)
                return
            const nx = mapToItem(graphArea, mouse.x, 0).x
                          - Theme.spaceSm - Theme.borderWidth
            graphArea.graphColWManual = Math.max(Metrics.laneInset + Metrics.laneW,
                Math.min(nx - graphArea.labelW,
                         graphArea.width - graphArea.labelW - 2 * Theme.spaceXxl))
        }
    }
    // Horizontal scroll of the lanes when the full graph is wider than
    // its column.
    ScrollBar {
        visible: graphArea.graphXMax > 0
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
    BusyIndicator {
        anchors.centerIn: parent
        running: graphArea.graphModel.loading && graphArea.graphModel.rowTotal === 0
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
            text: qsTr("platitude-gg")
            font.pixelSize: Theme.fontXl
            font.weight: Font.DemiBold
            anchors.horizontalCenter: parent.horizontalCenter
        }
        Label {
            text: qsTr("A thin, fast GUI over your installed git.")
            color: Theme.textSecondary
            anchors.horizontalCenter: parent.horizontalCenter
        }
        HoverButton {
            text: qsTr("Open repository…")
            anchors.horizontalCenter: parent.horizontalCenter
            onClicked: graphArea.openRepositoryRequested()
        }
    }
}
