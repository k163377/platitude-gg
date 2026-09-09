// Phase 0 spike UI — throwaway. English-only strings (project rule).
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import QtQuick.Shapes
import pg_spike

ApplicationWindow {
    id: root
    width: 1100
    height: 720
    visible: true
    title: qsTr("platitude-gg spike (Phase 0)")
    color: "#0d1b2a"

    palette {
        window: "#0d1b2a"
        windowText: "#c9d7e8"
        base: "#0a1420"
        text: "#c9d7e8"
        button: "#17293d"
        buttonText: "#c9d7e8"
        highlight: "#2a6fb0"
        highlightedText: "#ffffff"
        placeholderText: "#5f7387"
        mid: "#24344a"
        dark: "#091018"
        light: "#24344a"
    }

    readonly property var laneColors: [
        "#4fc3f7", "#81c784", "#ff8a65", "#ba68c8", "#ffd54f",
        "#64b5f6", "#e57373", "#4db6ac", "#f06292", "#aed581"
    ]
    readonly property int laneW: 14
    readonly property int rowH: 24
    readonly property int graphAreaW: 8 * laneW + 8

    // ---- fps instrumentation -------------------------------------------
    property int frameCounter: 0
    property int fps: 0
    onFrameSwapped: frameCounter++
    Timer {
        interval: 1000
        running: true
        repeat: true
        property int last: 0
        onTriggered: {
            root.fps = root.frameCounter - last
            last = root.frameCounter
        }
    }

    // ---- scroll benchmark (S3) ------------------------------------------
    property bool benchRunning: false
    property var benchQueue: []
    property real benchT0: 0
    property int benchFrames0: 0

    function benchModeIndex(name) {
        return name === "canvas" ? 1 : name === "shape" ? 2 : 0
    }
    function startBench(queue) {
        benchQueue = queue
        tabBar.currentIndex = 2
        nextBench()
    }
    function nextBench() {
        if (benchQueue.length === 0) {
            if (SpikeConfig.autoBench !== "")
                Qt.quit()
            return
        }
        rendererCombo.currentIndex = benchQueue.shift()
        graphList.contentY = 0
        benchPrepTimer.start()
    }
    Timer {
        id: benchPrepTimer
        interval: 600
        onTriggered: {
            root.benchT0 = Date.now()
            root.benchFrames0 = root.frameCounter
            root.benchRunning = true
            benchAnim.start()
        }
    }
    NumberAnimation {
        id: benchAnim
        target: graphList
        property: "contentY"
        from: 0
        to: 3000 * root.rowH
        duration: 12000
        onStopped: {
            const secs = (Date.now() - root.benchT0) / 1000
            const frames = root.frameCounter - root.benchFrames0
            const f = (frames / secs).toFixed(1)
            const name = rendererCombo.currentText
            benchResult.text = "last run: " + name + " = " + f + " fps"
            SpikeConfig.report("PGG_SPIKE_BENCH renderer=" + name + " fps=" + f)
            root.benchRunning = false
            root.nextBench()
        }
    }

    // ---- automation entry points ----------------------------------------
    Component.onCompleted: {
        if (SpikeConfig.autoBench !== "") {
            startBench(SpikeConfig.autoBench === "all" ? [0, 1, 2] : [benchModeIndex(SpikeConfig.autoBench)])
        } else if (SpikeConfig.autoLog !== "") {
            tabBar.currentIndex = 3
            logModel.load(SpikeConfig.autoLog, SpikeConfig.logTopo)
        }
        if (SpikeConfig.startTab >= 0)
            tabBar.currentIndex = SpikeConfig.startTab
        if (SpikeConfig.shotDir !== "")
            prepareShot()
    }

    // ---- window-content screenshot mode (PGG_SPIKE_SHOTDIR) --------------
    function prepareShot() {
        SpikeConfig.report("SHOT prepare tab=" + tabBar.currentIndex + " dir=" + SpikeConfig.shotDir)
        if (tabBar.currentIndex === 3 && !logModel.loading && logModel.rowTotal === 0)
            logModel.load(SpikeConfig.defaultRepo, SpikeConfig.logTopo)
        shotTimer.interval = tabBar.currentIndex === 3 ? 4500 : 1500
        shotTimer.restart()
    }
    Timer {
        id: shotTimer
        onTriggered: {
            const path = SpikeConfig.shotDir + "/spike-tab" + tabBar.currentIndex + ".png"
            const ok = pages.grabToImage(function(res) {
                const saved = res.saveToFile(path)
                SpikeConfig.report("SHOT saved=" + saved + " " + path)
                if (tabBar.currentIndex < 4) {
                    tabBar.currentIndex++
                    root.prepareShot()
                } else {
                    Qt.quit()
                }
            })
            if (!ok)
                SpikeConfig.report("SHOT grabToImage returned false for tab " + tabBar.currentIndex)
        }
    }
    Connections {
        target: logModel
        function onFinished() {
            if (SpikeConfig.autoLog !== "" && !SpikeConfig.stay)
                quitTimer.start()
        }
    }
    Timer {
        id: quitTimer
        interval: 400
        onTriggered: Qt.quit()
    }

    // ---- backends ---------------------------------------------------------
    DemoModel { id: demoModel }
    WorkerBackend { id: worker }
    GraphModel { id: graphModel }
    LogModel { id: logModel }

    // ---- chrome -----------------------------------------------------------
    header: TabBar {
        id: tabBar
        TabButton { text: qsTr("S1 Model") }
        TabButton { text: qsTr("S4 Worker") }
        TabButton { text: qsTr("S3 Graph") }
        TabButton { text: qsTr("S5 Log") }
        TabButton { text: qsTr("S2 IME") }
    }

    footer: Frame {
        padding: 4
        RowLayout {
            anchors.fill: parent
            Label { text: "fps: " + root.fps }
            Item { Layout.fillWidth: true }
            Label { text: qsTr("Qt Bridges spike — all data below comes from Rust") }
        }
    }

    StackLayout {
        id: pages
        anchors.fill: parent
        currentIndex: tabBar.currentIndex

        // ============================ S1: model =========================
        ColumnLayout {
            spacing: 8
            RowLayout {
                Layout.margins: 10
                Layout.fillWidth: true
                TextField {
                    id: s1Input
                    Layout.preferredWidth: 280
                    placeholderText: qsTr("text for add / update")
                }
                Button {
                    text: qsTr("Add")
                    onClicked: demoModel.addItem(s1Input.text)
                }
                Button {
                    text: qsTr("Update selected")
                    onClicked: demoModel.updateItem(s1List.currentIndex, s1Input.text)
                }
                Button {
                    text: qsTr("Remove selected")
                    onClicked: demoModel.removeItem(s1List.currentIndex)
                }
                Button {
                    text: qsTr("Clear")
                    onClicked: demoModel.clearItems()
                }
                Item { Layout.fillWidth: true }
                Label { text: qsTr("counter: ") + demoModel.counter }
                Button {
                    text: qsTr("Bump")
                    onClicked: demoModel.bumpCounter()
                }
            }
            ListView {
                id: s1List
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.margins: 10
                clip: true
                model: demoModel
                delegate: ItemDelegate {
                    id: s1Delegate
                    required property var model
                    required property int index
                    width: s1List.width
                    highlighted: ListView.isCurrentItem
                    text: (index + 1) + ".  " + model.value
                    onClicked: s1List.currentIndex = index
                }
            }
        }

        // ============================ S4: worker ========================
        ColumnLayout {
            spacing: 12
            Item { Layout.preferredHeight: 20 }
            Label {
                Layout.alignment: Qt.AlignHCenter
                text: qsTr("Worker thread ticks every 16ms and calls back into the UI thread (QmlMethodInvoker)")
            }
            ProgressBar {
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: 500
                from: 0
                to: 300
                value: worker.progress
            }
            Label {
                Layout.alignment: Qt.AlignHCenter
                text: qsTr("progress: ") + worker.progress + (worker.running ? qsTr("  (running)") : qsTr("  (idle)"))
            }
            RowLayout {
                Layout.alignment: Qt.AlignHCenter
                Button {
                    text: qsTr("Start worker")
                    enabled: !worker.running
                    onClicked: worker.start()
                }
                Button {
                    text: qsTr("Stop")
                    enabled: worker.running
                    onClicked: worker.stop()
                }
            }
            Item { Layout.fillHeight: true }
        }

        // ============================ S3: graph =========================
        ColumnLayout {
            spacing: 4
            RowLayout {
                Layout.margins: 10
                Layout.fillWidth: true
                Label { text: qsTr("renderer:") }
                ComboBox {
                    id: rendererCombo
                    model: ["items", "canvas", "shape"]
                }
                Button {
                    text: qsTr("Run scroll bench")
                    enabled: !root.benchRunning
                    onClicked: root.startBench([rendererCombo.currentIndex])
                }
                Label { id: benchResult; text: qsTr("last run: —") }
                Item { Layout.fillWidth: true }
                Label { text: graphList.count.toLocaleString() + qsTr(" rows") }
            }
            ListView {
                id: graphList
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: graphModel
                reuseItems: true
                boundsBehavior: Flickable.StopAtBounds
                flickDeceleration: 8000
                maximumFlickVelocity: 9000
                delegate: [itemsDelegate, canvasDelegate, shapeDelegate][rendererCombo.currentIndex]
            }
        }

        // ============================ S5: log ===========================
        ColumnLayout {
            spacing: 4
            RowLayout {
                Layout.margins: 10
                Layout.fillWidth: true
                TextField {
                    id: repoField
                    Layout.fillWidth: true
                    text: SpikeConfig.defaultRepo
                }
                CheckBox {
                    id: topoCheck
                    text: qsTr("--topo-order")
                    checked: SpikeConfig.logTopo
                }
                Button {
                    text: qsTr("Load git log")
                    enabled: !logModel.loading
                    onClicked: logModel.load(repoField.text, topoCheck.checked)
                }
            }
            Label {
                Layout.leftMargin: 10
                text: qsTr("first chunk: ") + logModel.firstChunkMs + qsTr(" ms    total: ")
                      + logModel.totalMs + qsTr(" ms    rows: ") + logModel.rowTotal
                      + "    " + logModel.status
            }
            ListView {
                id: logList
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: logModel
                reuseItems: true
                delegate: Item {
                    id: logDelegate
                    required property string sha
                    required property string author
                    required property string subject
                    width: logList.width
                    height: root.rowH
                    Text {
                        x: 8
                        anchors.verticalCenter: parent.verticalCenter
                        text: logDelegate.sha
                        color: "#7f97ad"
                        font.family: "Consolas"
                        font.pixelSize: 12
                    }
                    Text {
                        x: 90
                        anchors.verticalCenter: parent.verticalCenter
                        width: 160
                        elide: Text.ElideRight
                        text: logDelegate.author
                        color: "#8fb0c9"
                        font.pixelSize: 12
                    }
                    Text {
                        x: 260
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width - x - 8
                        elide: Text.ElideRight
                        text: logDelegate.subject
                        color: "#c9d7e8"
                        font.pixelSize: 12
                    }
                }
            }
        }

        // ============================ S2: IME ===========================
        ColumnLayout {
            spacing: 8
            Label {
                Layout.margins: 10
                text: qsTr("Type Japanese here (conversion, commit, cursor position). Single-line field below too.")
            }
            TextArea {
                id: imeArea
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.margins: 10
                wrapMode: TextArea.Wrap
                font.pixelSize: 16
                placeholderText: qsTr("commit message editor stand-in")
                background: Rectangle { color: "#0a1420"; border.color: "#24344a" }
            }
            TextField {
                Layout.fillWidth: true
                Layout.margins: 10
                font.pixelSize: 14
                placeholderText: qsTr("single-line input (branch name stand-in)")
            }
            Label {
                Layout.margins: 10
                text: qsTr("length: ") + imeArea.length + qsTr("    cursor: ") + imeArea.cursorPosition
            }
        }
    }

    // ======================= graph row delegates ========================

    Component {
        id: itemsDelegate
        Item {
            id: rowItems
            required property int index
            required property string sha
            required property string subject
            required property int node_lane
            required property int through_mask
            required property int conn_from
            required property int color_idx
            width: graphList.width
            height: root.rowH

            Repeater {
                model: 8
                Rectangle {
                    required property int index
                    visible: (rowItems.through_mask >> index) & 1
                    x: index * root.laneW + root.laneW / 2 - 1.5
                    y: 0
                    width: 3
                    height: root.rowH
                    color: root.laneColors[index % 10]
                }
            }
            // connector approximation: horizontal elbow between lanes
            Rectangle {
                visible: rowItems.conn_from >= 0
                x: (Math.min(rowItems.conn_from, rowItems.node_lane) + 0.5) * root.laneW
                y: root.rowH / 2 - 1.5
                width: Math.abs(rowItems.conn_from - rowItems.node_lane) * root.laneW
                height: 3
                color: root.laneColors[(rowItems.conn_from < 0 ? 0 : rowItems.conn_from) % 10]
            }
            Rectangle {
                x: rowItems.node_lane * root.laneW + root.laneW / 2 - 5
                y: root.rowH / 2 - 5
                width: 10
                height: 10
                radius: 5
                color: root.laneColors[rowItems.color_idx]
                border.color: "#0d1b2a"
                border.width: 1
            }
            Text {
                x: root.graphAreaW
                anchors.verticalCenter: parent.verticalCenter
                text: rowItems.sha
                color: "#7f97ad"
                font.family: "Consolas"
                font.pixelSize: 12
            }
            Text {
                x: root.graphAreaW + 80
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - x - 8
                elide: Text.ElideRight
                text: rowItems.subject
                color: "#c9d7e8"
                font.pixelSize: 12
            }
        }
    }

    Component {
        id: canvasDelegate
        Item {
            id: rowCanvas
            required property int index
            required property string sha
            required property string subject
            required property int node_lane
            required property int through_mask
            required property int conn_from
            required property int color_idx
            width: graphList.width
            height: root.rowH

            onNode_laneChanged: cv.requestPaint()
            onThrough_maskChanged: cv.requestPaint()
            onConn_fromChanged: cv.requestPaint()

            Canvas {
                id: cv
                width: root.graphAreaW
                height: root.rowH
                onPaint: {
                    const ctx = getContext("2d")
                    ctx.clearRect(0, 0, width, height)
                    ctx.lineWidth = 3
                    for (let l = 0; l < 8; l++) {
                        if ((rowCanvas.through_mask >> l) & 1) {
                            ctx.strokeStyle = root.laneColors[l % 10]
                            ctx.beginPath()
                            ctx.moveTo(l * root.laneW + root.laneW / 2, 0)
                            ctx.lineTo(l * root.laneW + root.laneW / 2, height)
                            ctx.stroke()
                        }
                    }
                    if (rowCanvas.conn_from >= 0) {
                        ctx.strokeStyle = root.laneColors[rowCanvas.conn_from % 10]
                        ctx.beginPath()
                        ctx.moveTo(rowCanvas.conn_from * root.laneW + root.laneW / 2, 0)
                        ctx.lineTo(rowCanvas.node_lane * root.laneW + root.laneW / 2, height / 2)
                        ctx.stroke()
                    }
                    ctx.fillStyle = root.laneColors[rowCanvas.color_idx]
                    ctx.beginPath()
                    ctx.arc(rowCanvas.node_lane * root.laneW + root.laneW / 2, height / 2, 5, 0, 2 * Math.PI)
                    ctx.fill()
                }
            }
            Text {
                x: root.graphAreaW
                anchors.verticalCenter: parent.verticalCenter
                text: rowCanvas.sha
                color: "#7f97ad"
                font.family: "Consolas"
                font.pixelSize: 12
            }
            Text {
                x: root.graphAreaW + 80
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - x - 8
                elide: Text.ElideRight
                text: rowCanvas.subject
                color: "#c9d7e8"
                font.pixelSize: 12
            }
        }
    }

    Component {
        id: shapeDelegate
        Item {
            id: rowShape
            required property int index
            required property string sha
            required property string subject
            required property int node_lane
            required property int through_mask
            required property int conn_from
            required property int color_idx
            width: graphList.width
            height: root.rowH

            component LanePath : ShapePath {
                id: lp
                property int lane: 0
                strokeWidth: 3
                fillColor: "transparent"
                strokeColor: (rowShape.through_mask >> lp.lane) & 1 ? root.laneColors[lp.lane % 10] : "transparent"
                startX: lp.lane * root.laneW + root.laneW / 2
                startY: 0
                PathLine {
                    x: lp.lane * root.laneW + root.laneW / 2
                    y: root.rowH
                }
            }

            Shape {
                width: root.graphAreaW
                height: root.rowH
                LanePath { lane: 0 }
                LanePath { lane: 1 }
                LanePath { lane: 2 }
                LanePath { lane: 3 }
                LanePath { lane: 4 }
                LanePath { lane: 5 }
                LanePath { lane: 6 }
                LanePath { lane: 7 }
                ShapePath {
                    strokeWidth: 3
                    fillColor: "transparent"
                    strokeColor: rowShape.conn_from >= 0 ? root.laneColors[rowShape.conn_from % 10] : "transparent"
                    startX: (rowShape.conn_from < 0 ? 0 : rowShape.conn_from) * root.laneW + root.laneW / 2
                    startY: 0
                    PathLine {
                        x: rowShape.node_lane * root.laneW + root.laneW / 2
                        y: root.rowH / 2
                    }
                }
                ShapePath {
                    strokeColor: "transparent"
                    fillColor: root.laneColors[rowShape.color_idx]
                    PathAngleArc {
                        centerX: rowShape.node_lane * root.laneW + root.laneW / 2
                        centerY: root.rowH / 2
                        radiusX: 5
                        radiusY: 5
                        startAngle: 0
                        sweepAngle: 360
                    }
                }
            }
            Text {
                x: root.graphAreaW
                anchors.verticalCenter: parent.verticalCenter
                text: rowShape.sha
                color: "#7f97ad"
                font.family: "Consolas"
                font.pixelSize: 12
            }
            Text {
                x: root.graphAreaW + 80
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - x - 8
                elide: Text.ElideRight
                text: rowShape.subject
                color: "#c9d7e8"
                font.pixelSize: 12
            }
        }
    }
}
