// platitude-gg main window. Presentation only: every model row arrives
// precomputed from Rust; the only JS here decodes compact draw/chip tokens
// and formats dates for display.
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import QtQuick.Dialogs
import platitude

ApplicationWindow {
    id: root
    width: 1440
    height: 900
    visible: true
    title: qsTr("platitude-gg")
    color: theme.window

    // ---- theme (dark blue only, per product requirement) ----------------
    QtObject {
        id: theme
        readonly property color window: "#0d1b2a"
        readonly property color base: "#0a1420"
        readonly property color panel: "#101d2e"
        readonly property color panelAlt: "#152538"
        readonly property color border: "#24344a"
        readonly property color text: "#c9d7e8"
        readonly property color dim: "#5f7387"
        readonly property color accent: "#4fc3f7"
        readonly property color highlight: "#2a6fb0"
        readonly property color danger: "#e57373"
        readonly property color warn: "#ffd54f"
        readonly property color ok: "#81c784"
        // diff palette (designed together with the dark theme)
        readonly property color diffAddBg: "#0d2b1d"
        readonly property color diffDelBg: "#33191f"
        readonly property color diffHunkBg: "#17293d"
        readonly property color diffAddText: "#8ce2b0"
        readonly property color diffDelText: "#ff9aa2"
        // graph lane palette — size must match core GRAPH_PALETTE_SIZE (12)
        readonly property var laneColors: [
            "#4fc3f7", "#81c784", "#ff8a65", "#ba68c8", "#ffd54f", "#64b5f6",
            "#e57373", "#4db6ac", "#f06292", "#aed581", "#7986cb", "#ffb74d"
        ]
    }

    palette {
        window: theme.window
        windowText: theme.text
        base: theme.base
        text: theme.text
        button: "#17293d"
        buttonText: theme.text
        highlight: theme.highlight
        highlightedText: "#ffffff"
        placeholderText: theme.dim
        mid: theme.border
        dark: "#091018"
        light: theme.border
    }

    readonly property int rowH: 24
    readonly property int laneW: 14

    // Window focus is a refresh trigger (refs/status/stash only).
    property int focusEpoch: 0
    onActiveChanged: if (active) focusEpoch++

    // Frame counter for the scroll benchmark (PG_AUTO_SCROLL=1).
    property int frameCounter: 0
    onFrameSwapped: frameCounter++

    TabsModel {
        id: tabsModel
    }

    FolderDialog {
        id: folderDialog
        title: qsTr("Open repository")
        onAccepted: tabsModel.openRepositoryUrl(selectedFolder.toString())
    }

    Component.onCompleted: {
        AppBackend.initialize()
        if (AppBackend.autoOpen !== "")
            tabsModel.openRepositoryPath(AppBackend.autoOpen)
        if (AppBackend.autoQuitMs > 0)
            quitTimer.start()
        if (AppBackend.shotDir !== "")
            shotTimer.start()
    }
    Timer {
        id: quitTimer
        interval: Math.max(AppBackend.autoQuitMs, 1)
        onTriggered: Qt.quit()
    }
    Timer {
        id: shotTimer
        interval: AppBackend.autoQuitMs > 800 ? AppBackend.autoQuitMs - 800 : 3500
        onTriggered: {
            const path = AppBackend.shotDir + "/app.png"
            const ok = mainUi.grabToImage(function (res) {
                const saved = res.saveToFile(path)
                console.warn("screenshot saved=" + saved + " path=" + path)
                if (AppBackend.autoQuitMs <= 0)
                    Qt.quit()
            })
            if (!ok)
                console.warn("grabToImage returned false")
        }
    }

    // ---- git gate --------------------------------------------------------
    Item {
        anchors.fill: parent
        visible: AppBackend.gitState !== "ok"
        Column {
            anchors.centerIn: parent
            spacing: 14
            width: Math.min(640, root.width - 80)
            Label {
                text: qsTr("platitude-gg")
                font.pixelSize: 28
                font.bold: true
                anchors.horizontalCenter: parent.horizontalCenter
            }
            BusyIndicator {
                visible: AppBackend.gitState === "checking"
                running: visible
                anchors.horizontalCenter: parent.horizontalCenter
            }
            Label {
                visible: AppBackend.gitState === "missing"
                text: qsTr("git was not found on PATH. Install git 2.43 or newer and restart.")
                wrapMode: Text.Wrap
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
            }
            Label {
                visible: AppBackend.gitState === "unsupported" || AppBackend.gitState === "error"
                text: AppBackend.gitError
                color: theme.danger
                wrapMode: Text.Wrap
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
            }
        }
    }

    // ---- main ------------------------------------------------------------
    ColumnLayout {
        id: mainUi
        anchors.fill: parent
        spacing: 0
        visible: AppBackend.gitState === "ok"

        // Tab strip
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 34
            color: theme.panel
            RowLayout {
                anchors.fill: parent
                spacing: 0
                TabBar {
                    id: tabBar
                    Layout.fillWidth: false
                    // Two-way sync without a binding loop: user clicks push
                    // into the model; model changes push back here.
                    onCurrentIndexChanged: tabsModel.setCurrentIndex(currentIndex)
                    Connections {
                        target: tabsModel
                        function onCurrentIndexChanged() {
                            tabBar.currentIndex = tabsModel.currentIndex
                        }
                    }
                    Repeater {
                        model: tabsModel
                        TabButton {
                            id: tabButton
                            required property int tab_id
                            required property string title
                            required property string repo_path
                            width: implicitWidth
                            contentItem: RowLayout {
                                spacing: 6
                                Label {
                                    text: tabButton.title
                                    elide: Text.ElideRight
                                    Layout.maximumWidth: 180
                                }
                                ToolButton {
                                    text: "×"
                                    padding: 0
                                    implicitWidth: 18
                                    implicitHeight: 18
                                    onClicked: tabsModel.closeTab(tabButton.tab_id)
                                }
                            }
                        }
                    }
                }
                ToolButton {
                    text: "+"
                    font.pixelSize: 16
                    onClicked: folderDialog.open()
                }
                Item { Layout.fillWidth: true }
                Label {
                    text: AppBackend.gitVersion === "" ? "" : qsTr("git %1").arg(AppBackend.gitVersion)
                    color: theme.dim
                    font.pixelSize: 11
                    rightPadding: 10
                }
            }
        }

        // Welcome page
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: tabsModel.currentIndex < 0
            Column {
                anchors.centerIn: parent
                spacing: 16
                Label {
                    text: qsTr("platitude-gg")
                    font.pixelSize: 30
                    font.bold: true
                    anchors.horizontalCenter: parent.horizontalCenter
                }
                Label {
                    text: qsTr("A thin, fast GUI over your installed git.")
                    color: theme.dim
                    anchors.horizontalCenter: parent.horizontalCenter
                }
                Button {
                    text: qsTr("Open repository…")
                    anchors.horizontalCenter: parent.horizontalCenter
                    onClicked: folderDialog.open()
                }
            }
        }

        // Repository pages (one per tab, kept alive for instant switching)
        StackLayout {
            id: pages
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: tabsModel.currentIndex >= 0
            currentIndex: Math.max(0, tabsModel.currentIndex)
            Repeater {
                model: tabsModel
                RepoPage {}
            }
        }
    }

    // ======================================================================
    // One repository page
    // ======================================================================
    component RepoPage: Item {
        id: page
        required property int index
        required property int tab_id

        readonly property int shownLanes: Math.max(1, Math.min(graphModel.maxLanes, 12))
        readonly property int graphAreaW: shownLanes * root.laneW + 8

        RepoTab { id: repoTab }
        GraphModel { id: graphModel }
        SidebarModel { id: sidebarModel }
        WorkTreeModel { id: workTree }
        StashModel { id: stashModel }
        DetailsModel { id: detailsModel }
        DiffModel { id: diffModel }

        Component.onCompleted: {
            repoTab.attach(page.tab_id)
            graphModel.attach(page.tab_id)
            sidebarModel.attach(page.tab_id)
            workTree.attach(page.tab_id)
            stashModel.attach(page.tab_id)
            detailsModel.attach(page.tab_id)
            diffModel.attach(page.tab_id)
        }

        Connections {
            target: root
            function onFocusEpochChanged() {
                if (page.visible && repoTab.state === "open")
                    repoTab.refreshQuick()
            }
        }

        // Scroll benchmark (PG_AUTO_SCROLL=1): after the stream finishes,
        // animate 3000 rows over 12s and report the measured fps.
        property bool benchStarted: false
        Connections {
            target: graphModel
            enabled: AppBackend.autoScroll
            function onStatsChanged() {
                if (!page.benchStarted && !graphModel.loading && graphModel.rowTotal > 0) {
                    page.benchStarted = true
                    benchPrep.start()
                }
            }
        }
        Timer {
            id: benchPrep
            interval: 800
            onTriggered: {
                page.benchT0 = Date.now()
                page.benchFrames0 = root.frameCounter
                benchAnim.start()
            }
        }
        property real benchT0: 0
        property int benchFrames0: 0
        NumberAnimation {
            id: benchAnim
            target: graphList
            property: "contentY"
            from: 0
            to: Math.min(3000, graphModel.rowTotal - 40) * root.rowH
            duration: 12000
            onStopped: {
                const secs = (Date.now() - page.benchT0) / 1000
                const frames = root.frameCounter - page.benchFrames0
                AppBackend.report("scroll_bench fps=" + (frames / secs).toFixed(1)
                                  + " rows=" + graphModel.rowTotal)
            }
        }

        // Automation (PG_AUTO_SELECT=1): select the newest row, then open
        // the first changed file's diff — exercises the full pipeline for
        // screenshot-based smoke tests.
        property bool autoSelected: false
        Connections {
            target: graphModel
            enabled: AppBackend.autoSelect
            function onStatsChanged() {
                if (!page.autoSelected && graphModel.rowTotal > 0) {
                    page.autoSelected = true
                    graphList.currentIndex = 0
                    graphList.rowSelected(graphModel.oidAt(0))
                }
            }
        }
        Connections {
            target: detailsModel
            enabled: AppBackend.autoSelect
            function onChanged() {
                if (detailsModel.shaHex !== "" && fileList.count > 0 && diffModel.title === "")
                    diffModel.requestCommitFile(detailsModel.shaHex, detailsModel.parentHex,
                                                detailsModel.filePathAt(0),
                                                detailsModel.fileOrigPathAt(0))
            }
        }

        // Open failed: show git's own message
        Column {
            anchors.centerIn: parent
            visible: repoTab.state === "error"
            spacing: 12
            width: Math.min(700, page.width - 60)
            Label {
                text: qsTr("Could not open this folder as a git repository")
                font.pixelSize: 18
                font.bold: true
                anchors.horizontalCenter: parent.horizontalCenter
            }
            Label {
                text: repoTab.error
                color: theme.danger
                wrapMode: Text.Wrap
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
            }
            Button {
                text: qsTr("Close tab")
                anchors.horizontalCenter: parent.horizontalCenter
                onClicked: tabsModel.closeTab(page.tab_id)
            }
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0
            visible: repoTab.state !== "error"

            // ---- always-on status header --------------------------------
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: 32
                color: theme.panelAlt
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 10
                    anchors.rightMargin: 10
                    spacing: 8

                    Label {
                        text: workTree.detached
                              ? qsTr("DETACHED HEAD")
                              : (workTree.branch === "" ? qsTr("(no branch)") : workTree.branch)
                        font.bold: true
                        color: workTree.detached ? theme.warn : theme.text
                    }
                    Label {
                        visible: workTree.upstream !== ""
                        text: "↑" + workTree.ahead + " ↓" + workTree.behind
                        color: theme.dim
                        font.pixelSize: 12
                    }
                    Rectangle {
                        visible: workTree.opText !== ""
                        color: "transparent"
                        border.color: theme.warn
                        radius: 4
                        implicitHeight: 20
                        implicitWidth: opLabel.implicitWidth + 12
                        Label {
                            id: opLabel
                            anchors.centerIn: parent
                            text: workTree.opText
                            color: theme.warn
                            font.pixelSize: 11
                            font.bold: true
                        }
                    }
                    Rectangle {
                        visible: workTree.hasConflicts
                        color: theme.danger
                        radius: 4
                        implicitHeight: 20
                        implicitWidth: conflictLabel.implicitWidth + 12
                        Label {
                            id: conflictLabel
                            anchors.centerIn: parent
                            text: qsTr("CONFLICTS")
                            color: "#000000"
                            font.pixelSize: 11
                            font.bold: true
                        }
                    }
                    Label {
                        visible: repoTab.lastError !== ""
                        text: repoTab.lastError
                        color: theme.danger
                        elide: Text.ElideRight
                        Layout.maximumWidth: 320
                        font.pixelSize: 11
                        MouseArea {
                            anchors.fill: parent
                            onClicked: repoTab.clearLastError()
                        }
                    }

                    Item { Layout.fillWidth: true }

                    // Reserved: auto-fetch indicator (Phase 4)
                    Label {
                        text: "↻"
                        color: theme.border
                        font.pixelSize: 14
                    }
                    // Reserved: search box (backlog)
                    TextField {
                        enabled: false
                        opacity: 0.35
                        placeholderText: qsTr("Search")
                        implicitWidth: 160
                        implicitHeight: 24
                    }
                    ToolButton {
                        text: qsTr("Refresh")
                        onClicked: repoTab.refreshAll()
                    }
                }
            }

            // ---- three-pane layout --------------------------------------
            SplitView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                orientation: Qt.Horizontal

                // Sidebar: refs + filter
                Rectangle {
                    SplitView.preferredWidth: 240
                    SplitView.minimumWidth: 160
                    color: theme.panel
                    ColumnLayout {
                        anchors.fill: parent
                        spacing: 0
                        TextField {
                            id: refFilter
                            Layout.fillWidth: true
                            Layout.margins: 6
                            placeholderText: qsTr("Filter refs")
                            onTextChanged: sidebarModel.setFilter(text)
                        }
                        ListView {
                            id: sidebarList
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            clip: true
                            model: sidebarModel
                            reuseItems: true
                            section.property: "group"
                            section.delegate: Rectangle {
                                required property string section
                                width: sidebarList.width
                                height: 22
                                color: theme.panel
                                Label {
                                    anchors.verticalCenter: parent.verticalCenter
                                    x: 8
                                    text: parent.section
                                    color: theme.dim
                                    font.pixelSize: 10
                                    font.bold: true
                                }
                            }
                            delegate: Rectangle {
                                id: refRow
                                required property string name
                                required property string oid_hex
                                required property string kind
                                required property bool is_head
                                required property bool has_remote
                                width: sidebarList.width
                                height: 24
                                color: refMouse.containsMouse ? theme.panelAlt : "transparent"
                                RowLayout {
                                    anchors.fill: parent
                                    anchors.leftMargin: 10
                                    anchors.rightMargin: 8
                                    spacing: 6
                                    Label {
                                        text: refRow.kind === "tag" ? "⚑"
                                              : refRow.kind === "remote" ? "☁" : "⎇"
                                        color: refRow.kind === "tag" ? theme.warn
                                               : refRow.kind === "remote" ? "#ba68c8" : theme.accent
                                        font.pixelSize: 11
                                    }
                                    Label {
                                        Layout.fillWidth: true
                                        text: refRow.name
                                        elide: Text.ElideMiddle
                                        font.bold: refRow.is_head
                                        color: refRow.is_head ? theme.accent : theme.text
                                        font.pixelSize: 12
                                    }
                                    // Branch state badge: filled = has remote,
                                    // hollow = local-only (PR state joins in
                                    // Phase 4 as a third look).
                                    Rectangle {
                                        visible: refRow.kind === "branch"
                                        width: 8
                                        height: 8
                                        radius: 4
                                        color: refRow.has_remote ? theme.ok : "transparent"
                                        border.color: refRow.has_remote ? theme.ok : theme.dim
                                        border.width: 1
                                    }
                                }
                                MouseArea {
                                    id: refMouse
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    onClicked: {
                                        const row = graphModel.rowOf(refRow.oid_hex)
                                        if (row >= 0) {
                                            graphList.currentIndex = row
                                            graphList.positionViewAtIndex(row, ListView.Center)
                                            detailsModel.request(refRow.oid_hex)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Commit graph
                Rectangle {
                    SplitView.fillWidth: true
                    SplitView.minimumWidth: 420
                    color: theme.base
                    ListView {
                        id: graphList
                        anchors.fill: parent
                        clip: true
                        model: graphModel
                        reuseItems: true
                        boundsBehavior: Flickable.StopAtBounds
                        flickDeceleration: 8000
                        maximumFlickVelocity: 9000
                        ScrollBar.vertical: ScrollBar {}
                        // Bridge into the page scope for the shared delegate
                        // component (inline components cannot see page ids).
                        property int graphAreaWidth: page.graphAreaW
                        signal rowSelected(string oidHex)
                        onRowSelected: oidHex => {
                            detailsModel.request(oidHex)
                            diffModel.clear()
                        }
                        delegate: GraphRowDelegate {}
                    }
                    BusyIndicator {
                        anchors.centerIn: parent
                        running: graphModel.loading && graphModel.rowTotal === 0
                    }
                    Label {
                        anchors.centerIn: parent
                        visible: graphModel.error !== ""
                        text: graphModel.error
                        color: theme.danger
                        width: parent.width - 40
                        wrapMode: Text.Wrap
                        horizontalAlignment: Text.AlignHCenter
                    }
                }

                // Right side: working tree (persistent) + details + diff
                Rectangle {
                    SplitView.preferredWidth: 460
                    SplitView.minimumWidth: 320
                    color: theme.panel
                    SplitView {
                        anchors.fill: parent
                        orientation: Qt.Vertical

                        // Working tree + stashes (always visible)
                        ColumnLayout {
                            SplitView.preferredHeight: 260
                            SplitView.minimumHeight: 140
                            spacing: 0
                            PaneHeader {
                                text: qsTr("WORKING TREE  ·  %1 staged  %2 unstaged  %3 untracked")
                                    .arg(workTree.stagedCount)
                                    .arg(workTree.unstagedCount)
                                    .arg(workTree.untrackedCount)
                            }
                            ListView {
                                id: workTreeList
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                clip: true
                                model: workTree
                                reuseItems: true
                                section.property: "bucket"
                                section.delegate: Rectangle {
                                    required property string section
                                    width: workTreeList.width
                                    height: 20
                                    color: theme.panelAlt
                                    Label {
                                        anchors.verticalCenter: parent.verticalCenter
                                        x: 8
                                        font.pixelSize: 10
                                        font.bold: true
                                        color: parent.section === "conflicts" ? theme.danger : theme.dim
                                        text: parent.section === "staged" ? qsTr("STAGED")
                                              : parent.section === "unstaged" ? qsTr("UNSTAGED")
                                              : parent.section === "untracked" ? qsTr("UNTRACKED")
                                              : qsTr("CONFLICTS")
                                    }
                                }
                                delegate: FileRowDelegate {
                                    listWidth: workTreeList.width
                                    onActivated: (bucket, path, origPath) =>
                                        diffModel.requestWorkTree(bucket, path, origPath)
                                }
                            }
                            PaneHeader {
                                text: qsTr("STASHES (%1)").arg(stashList.count)
                            }
                            ListView {
                                id: stashList
                                Layout.fillWidth: true
                                Layout.preferredHeight: Math.min(count * 22 + 4, 92)
                                clip: true
                                model: stashModel
                                delegate: Item {
                                    id: stashRow
                                    required property string name
                                    required property string message
                                    width: stashList.width
                                    height: 22
                                    Label {
                                        x: 8
                                        anchors.verticalCenter: parent.verticalCenter
                                        width: parent.width - 16
                                        elide: Text.ElideRight
                                        font.pixelSize: 11
                                        color: theme.text
                                        text: stashRow.name + "  " + stashRow.message
                                    }
                                }
                            }
                        }

                        // Commit details + file list
                        ColumnLayout {
                            SplitView.preferredHeight: 240
                            SplitView.minimumHeight: 120
                            spacing: 0
                            PaneHeader { text: qsTr("COMMIT") }
                            ColumnLayout {
                                Layout.fillWidth: true
                                Layout.margins: 8
                                spacing: 2
                                visible: detailsModel.shaHex !== ""
                                RowLayout {
                                    Label {
                                        text: detailsModel.sha8
                                        font.family: "Consolas"
                                        color: theme.accent
                                        font.pixelSize: 12
                                    }
                                    Label {
                                        text: detailsModel.author
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                        color: theme.dim
                                        font.pixelSize: 12
                                    }
                                    Label {
                                        text: Qt.formatDateTime(new Date(detailsModel.authorTime * 1000),
                                                                "yyyy-MM-dd HH:mm")
                                        color: theme.dim
                                        font.pixelSize: 12
                                    }
                                }
                                ScrollView {
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: Math.min(messageArea.implicitHeight + 8, 96)
                                    TextArea {
                                        id: messageArea
                                        readOnly: true
                                        wrapMode: TextArea.Wrap
                                        text: detailsModel.message
                                        font.pixelSize: 12
                                        background: null
                                    }
                                }
                            }
                            Label {
                                visible: detailsModel.shaHex === ""
                                Layout.margins: 8
                                text: qsTr("Select a commit to see its details")
                                color: theme.dim
                                font.pixelSize: 12
                            }
                            ListView {
                                id: fileList
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                clip: true
                                model: detailsModel
                                reuseItems: true
                                delegate: FileRowDelegate {
                                    listWidth: fileList.width
                                    commitMode: true
                                    onActivated: (bucket, path, origPath) =>
                                        diffModel.requestCommitFile(detailsModel.shaHex,
                                                                    detailsModel.parentHex,
                                                                    path, origPath)
                                }
                            }
                        }

                        // Unified diff
                        ColumnLayout {
                            SplitView.fillHeight: true
                            SplitView.minimumHeight: 140
                            spacing: 0
                            PaneHeader {
                                text: diffModel.title === ""
                                      ? qsTr("DIFF")
                                      : qsTr("DIFF · %1").arg(diffModel.title)
                            }
                            Label {
                                visible: diffModel.isBinary
                                Layout.margins: 8
                                text: qsTr("Binary file — no text diff")
                                color: theme.dim
                            }
                            ListView {
                                id: diffList
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                clip: true
                                model: diffModel
                                reuseItems: true
                                boundsBehavior: Flickable.StopAtBounds
                                ScrollBar.vertical: ScrollBar {}
                                delegate: Rectangle {
                                    id: diffRow
                                    required property string kind
                                    required property int old_no
                                    required property int new_no
                                    required property string text
                                    width: diffList.width
                                    height: 18
                                    color: kind === "add" ? theme.diffAddBg
                                           : kind === "del" ? theme.diffDelBg
                                           : kind === "hunk" ? theme.diffHunkBg
                                           : "transparent"
                                    Row {
                                        anchors.fill: parent
                                        spacing: 0
                                        Label {
                                            width: 42
                                            text: diffRow.old_no >= 0 ? diffRow.old_no : ""
                                            horizontalAlignment: Text.AlignRight
                                            rightPadding: 6
                                            color: theme.dim
                                            font.family: "Consolas"
                                            font.pixelSize: 11
                                        }
                                        Label {
                                            width: 42
                                            text: diffRow.new_no >= 0 ? diffRow.new_no : ""
                                            horizontalAlignment: Text.AlignRight
                                            rightPadding: 6
                                            color: theme.dim
                                            font.family: "Consolas"
                                            font.pixelSize: 11
                                        }
                                        Label {
                                            width: parent.width - 84
                                            text: diffRow.text
                                            elide: Text.ElideRight
                                            font.family: "Consolas"
                                            font.pixelSize: 11
                                            color: diffRow.kind === "add" ? theme.diffAddText
                                                   : diffRow.kind === "del" ? theme.diffDelText
                                                   : diffRow.kind === "hunk" ? theme.accent
                                                   : diffRow.kind === "meta" ? theme.dim
                                                   : theme.text
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ======================================================================
    // Commit-graph row: canvas lanes + node, chips, subject, meta columns
    // ======================================================================
    component GraphRowDelegate: Item {
        id: rowItem
        required property int index
        required property string oid_hex
        required property string author
        required property double atime
        required property string subject
        required property int node_lane
        required property int node_color
        required property string geometry
        required property string labels

        width: ListView.view.width
        height: root.rowH

        readonly property bool selected: ListView.isCurrentItem

        Rectangle {
            anchors.fill: parent
            color: rowItem.selected ? Qt.rgba(0.16, 0.44, 0.69, 0.35)
                                    : rowMouse.containsMouse ? Qt.rgba(1, 1, 1, 0.04)
                                                             : "transparent"
        }

        onGeometryChanged: laneCanvas.requestPaint()
        onNode_laneChanged: laneCanvas.requestPaint()
        onNode_colorChanged: laneCanvas.requestPaint()

        RowLayout {
            anchors.fill: parent
            spacing: 8

            Canvas {
                id: laneCanvas
                // graphAreaWidth is provided on the ListView (page scope).
                Layout.preferredWidth: rowItem.ListView.view ? rowItem.ListView.view.graphAreaWidth : 120
                Layout.fillHeight: true
                onPaint: {
                    const ctx = getContext("2d")
                    ctx.clearRect(0, 0, width, height)
                    ctx.lineWidth = 2
                    const cx = function (l) { return l * root.laneW + root.laneW / 2 }
                    const midY = height / 2
                    const nodeX = cx(rowItem.node_lane)
                    // decode precomputed draw tokens: t/i/o + lane + color
                    if (rowItem.geometry !== "") {
                        const toks = rowItem.geometry.split(";")
                        for (let n = 0; n < toks.length; n++) {
                            const t = toks[n]
                            const dot = t.indexOf(".")
                            const lane = parseInt(t.substring(1, dot))
                            const x = cx(lane)
                            ctx.strokeStyle = theme.laneColors[parseInt(t.substring(dot + 1)) % 12]
                            ctx.beginPath()
                            if (t[0] === "t") {
                                ctx.moveTo(x, 0)
                                ctx.lineTo(x, height)
                            } else if (t[0] === "i") {
                                ctx.moveTo(x, 0)
                                ctx.bezierCurveTo(x, midY * 0.66, nodeX, midY * 0.34, nodeX, midY)
                            } else {
                                ctx.moveTo(nodeX, midY)
                                ctx.bezierCurveTo(nodeX, height - midY * 0.34,
                                                  x, height - midY * 0.66, x, height)
                            }
                            ctx.stroke()
                        }
                    }
                    ctx.fillStyle = theme.laneColors[rowItem.node_color % 12]
                    ctx.beginPath()
                    ctx.arc(nodeX, midY, 4.5, 0, 2 * Math.PI)
                    ctx.fill()
                    ctx.strokeStyle = theme.base
                    ctx.lineWidth = 1
                    ctx.beginPath()
                    ctx.arc(nodeX, midY, 4.5, 0, 2 * Math.PI)
                    ctx.stroke()
                }
            }

            // Ref label chips (precomputed records: K + head + remote + text)
            Row {
                spacing: 4
                Layout.maximumWidth: 300
                Repeater {
                    // Chip records are separated by U+001F (see encode.rs).
                    model: rowItem.labels === "" ? [] : rowItem.labels.split(String.fromCharCode(31))
                    Rectangle {
                        id: chip
                        required property string modelData
                        readonly property string chipKind: modelData[0]
                        readonly property bool chipHead: modelData[1] === "1"
                        readonly property bool chipRemote: modelData[2] === "1"
                        readonly property string chipText: modelData.substring(3)
                        readonly property color chipColor: chipKind === "T" ? theme.warn
                                                          : chipKind === "R" ? "#ba68c8"
                                                          : chipKind === "H" ? theme.danger
                                                          : theme.accent
                        height: 16
                        width: chipRow.implicitWidth + 10
                        radius: 8
                        color: "transparent"
                        border.color: chipColor
                        border.width: chipHead ? 2 : 1
                        Row {
                            id: chipRow
                            anchors.centerIn: parent
                            spacing: 3
                            Rectangle {
                                visible: chip.chipKind === "L"
                                anchors.verticalCenter: parent.verticalCenter
                                width: 6
                                height: 6
                                radius: 3
                                color: chip.chipRemote ? chip.chipColor : "transparent"
                                border.color: chip.chipColor
                                border.width: 1
                            }
                            Label {
                                text: chip.chipText
                                color: chip.chipColor
                                font.pixelSize: 10
                                font.bold: chip.chipHead
                            }
                        }
                    }
                }
            }

            Label {
                Layout.fillWidth: true
                text: rowItem.subject
                elide: Text.ElideRight
                font.pixelSize: 12
                color: theme.text
            }
            Label {
                Layout.preferredWidth: 130
                text: rowItem.author
                elide: Text.ElideRight
                font.pixelSize: 11
                color: theme.dim
            }
            Label {
                Layout.preferredWidth: 110
                text: Qt.formatDateTime(new Date(rowItem.atime * 1000), "yyyy-MM-dd HH:mm")
                font.pixelSize: 11
                color: theme.dim
            }
            Label {
                Layout.preferredWidth: 64
                text: rowItem.oid_hex.substring(0, 8)
                font.family: "Consolas"
                font.pixelSize: 11
                color: theme.dim
                rightPadding: 8
            }
        }

        MouseArea {
            id: rowMouse
            anchors.fill: parent
            hoverEnabled: true
            onClicked: {
                rowItem.ListView.view.currentIndex = rowItem.index
                rowItem.ListView.view.rowSelected(rowItem.oid_hex)
            }
        }
    }

    // ======================================================================
    // A changed-file row (working tree pane and commit file list)
    // ======================================================================
    component FileRowDelegate: Rectangle {
        id: fileRow
        // `model` covers both lists: work-tree rows carry `bucket`, commit
        // file rows don't (undefined → "").
        required property var model
        property bool commitMode: false
        property real listWidth: 200

        readonly property string changeText: model.change ?? ""
        readonly property string pathText: model.path ?? ""
        readonly property string origPathText: model.orig_path ?? ""
        readonly property string bucketText: model.bucket ?? ""

        signal activated(string bucket, string path, string origPath)

        width: listWidth
        height: 22
        color: fileMouse.containsMouse ? theme.panelAlt : "transparent"

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 8
            anchors.rightMargin: 8
            spacing: 6
            Label {
                text: fileRow.changeText
                font.family: "Consolas"
                font.pixelSize: 11
                font.bold: true
                color: fileRow.changeText.startsWith("A") ? theme.ok
                       : fileRow.changeText.startsWith("D") ? theme.danger
                       : fileRow.changeText.startsWith("R") ? "#ba68c8"
                       : fileRow.changeText === "?" ? theme.dim
                       : fileRow.changeText.length === 2 ? theme.danger
                       : theme.warn
                Layout.preferredWidth: 18
            }
            Label {
                Layout.fillWidth: true
                text: fileRow.origPathText === ""
                      ? fileRow.pathText
                      : qsTr("%1 → %2").arg(fileRow.origPathText).arg(fileRow.pathText)
                elide: Text.ElideMiddle
                font.pixelSize: 11
                color: theme.text
            }
        }
        MouseArea {
            id: fileMouse
            anchors.fill: parent
            hoverEnabled: true
            onClicked: fileRow.activated(fileRow.bucketText, fileRow.pathText, fileRow.origPathText)
        }
    }

    component PaneHeader: Rectangle {
        property alias text: headerLabel.text
        Layout.fillWidth: true
        implicitHeight: 24
        color: theme.panelAlt
        Label {
            id: headerLabel
            anchors.verticalCenter: parent.verticalCenter
            x: 8
            font.pixelSize: 10
            font.bold: true
            color: theme.dim
        }
    }
}
