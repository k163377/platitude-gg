pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
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
    /// Right-click on a commit row.
    signal commitMenuRequested(string oidHex)
    /// The blank pane's "Open repository…" button.
    signal openRepositoryRequested()

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
    ListView {
        id: graphList
        anchors.fill: parent
        clip: true
        model: graphArea.graphModel
        reuseItems: true
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
        signal rowSelected(string oidHex)
        signal rowMenuRequested(string oidHex)
        onRowMenuRequested: oidHex => graphArea.commitMenuRequested(oidHex)
        onRowSelected: oidHex => graphArea.rowActivated(oidHex)
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
            // rows rather than running under the next pane.
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
                color: Theme.warning
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
