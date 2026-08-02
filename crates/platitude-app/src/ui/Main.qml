// platitude-gg main window. Presentation only: every model row arrives
// precomputed from Rust; the only JS here decodes compact draw/chip tokens
// and formats dates for display.
//
// All colors / fonts / dimensions come from the Theme singleton
// (internal-docs/デザイン規約.md is the source of truth).
pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import QtQuick.Dialogs
import platitude
import platitude.ui

ApplicationWindow {
    id: root
    width: 1440
    height: 900
    visible: true
    title: qsTr("platitude-gg")
    color: Theme.bgBase
    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd

    // Commit-graph geometry and interaction values not yet covered by the
    // design tokens; kept in one place and grid-aligned. Pending token
    // additions (do not tune): lane pitch = spaceLg, node diameter =
    // iconSm, lane stroke = 2, wheel step = 6 rows/notch, middle-drag
    // gain = 0.12, ref-label column width = 180.
    readonly property int laneW: Theme.spaceLg
    readonly property int nodeDiameter: Theme.iconSm
    readonly property int laneStroke: 2
    readonly property int wheelRows: 6
    readonly property real middleScrollGain: 0.12
    readonly property int labelColW: 180

    palette {
        window: Theme.bgBase
        windowText: Theme.textPrimary
        base: Theme.bgBase
        text: Theme.textPrimary
        button: Theme.bgElevated
        buttonText: Theme.textPrimary
        highlight: Theme.accent
        highlightedText: Theme.textOnAccent
        placeholderText: Theme.textMuted
        mid: Theme.borderDefault
        dark: Theme.bgBase
        light: Theme.borderDefault
    }

    // Window focus is a refresh trigger (refs/status/stash only).
    property int focusEpoch: 0
    onActiveChanged: if (active) focusEpoch++

    // Frame counter for the scroll benchmark (PG_AUTO_SCROLL=1).
    property int frameCounter: 0
    onFrameSwapped: frameCounter++

    // Clipboard access for copy buttons (QML has no direct clipboard API).
    function copyText(value) {
        clipboardEdit.text = value
        clipboardEdit.selectAll()
        clipboardEdit.copy()
    }
    TextEdit {
        id: clipboardEdit
        visible: false
    }

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
            spacing: Theme.spaceLg
            width: Math.min(640, root.width - 2 * Theme.spaceXxl)
            Label {
                text: qsTr("platitude-gg")
                font.pixelSize: Theme.fontXl
                font.weight: Font.DemiBold
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
                color: Theme.danger
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
            implicitHeight: Theme.toolbarHeight
            color: Theme.bgElevated
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
                                spacing: Theme.spaceXs
                                Label {
                                    text: tabButton.title
                                    elide: Text.ElideRight
                                    Layout.maximumWidth: 180
                                }
                                ToolButton {
                                    text: "×"
                                    padding: 0
                                    implicitWidth: Theme.iconLg
                                    implicitHeight: Theme.iconLg
                                    onClicked: tabsModel.closeTab(tabButton.tab_id)
                                }
                            }
                        }
                    }
                }
                ToolButton {
                    text: "+"
                    font.pixelSize: Theme.fontLg
                    onClicked: folderDialog.open()
                }
                Item { Layout.fillWidth: true }
                Label {
                    text: AppBackend.gitVersion === "" ? "" : qsTr("git %1").arg(AppBackend.gitVersion)
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontSm
                    rightPadding: Theme.spaceSm
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
        readonly property int graphAreaW: shownLanes * root.laneW + Theme.spaceSm
        property string selectedOid: ""

        RepoTab { id: repoTab }
        GraphModel { id: graphModel }
        SidebarModel { id: sidebarModel }
        WorkTreeModel { id: workTree }
        DetailsModel { id: detailsModel }
        DiffModel { id: diffModel }

        Component.onCompleted: {
            repoTab.attach(page.tab_id)
            graphModel.attach(page.tab_id)
            sidebarModel.attach(page.tab_id)
            workTree.attach(page.tab_id)
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

        // Restore the selected row after the tag-inclusive swap pass (the
        // model reset drops currentIndex).
        Connections {
            target: graphModel
            function onStatsChanged() {
                if (page.selectedOid !== "" && graphList.currentIndex < 0) {
                    const row = graphModel.rowOf(page.selectedOid)
                    if (row >= 0)
                        graphList.currentIndex = row
                }
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
            to: Math.min(3000, graphModel.rowTotal - 40) * Theme.rowHeight
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
            spacing: Theme.spaceMd
            width: Math.min(700, page.width - 2 * Theme.spaceXl)
            Label {
                text: qsTr("Could not open this folder as a git repository")
                font.pixelSize: Theme.fontLg
                font.weight: Font.DemiBold
                anchors.horizontalCenter: parent.horizontalCenter
            }
            Label {
                text: repoTab.error
                color: Theme.danger
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
                implicitHeight: Theme.toolbarHeight
                color: Theme.bgElevated
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.spaceMd
                    anchors.rightMargin: Theme.spaceMd
                    spacing: Theme.spaceSm

                    Label {
                        text: workTree.detached
                              ? qsTr("DETACHED HEAD")
                              : (workTree.branch === "" ? qsTr("(no branch)") : workTree.branch)
                        font.weight: Font.DemiBold
                        color: workTree.detached ? Theme.warning : Theme.textPrimary
                    }
                    Label {
                        visible: workTree.upstream !== ""
                        text: "↑" + workTree.ahead + " ↓" + workTree.behind
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontSm
                    }
                    Rectangle {
                        visible: workTree.opText !== ""
                        color: "transparent"
                        border.color: Theme.warning
                        border.width: Theme.borderWidth
                        radius: Theme.radiusSm
                        implicitHeight: Theme.iconLg
                        implicitWidth: opLabel.implicitWidth + 2 * Theme.spaceXs
                        Label {
                            id: opLabel
                            anchors.centerIn: parent
                            text: workTree.opText
                            color: Theme.warning
                            font.pixelSize: Theme.fontSm
                            font.weight: Font.DemiBold
                        }
                    }
                    Rectangle {
                        visible: workTree.hasConflicts
                        color: Theme.danger
                        radius: Theme.radiusSm
                        implicitHeight: Theme.iconLg
                        implicitWidth: conflictLabel.implicitWidth + 2 * Theme.spaceXs
                        Label {
                            id: conflictLabel
                            anchors.centerIn: parent
                            text: qsTr("CONFLICTS")
                            color: Theme.textOnAccent
                            font.pixelSize: Theme.fontSm
                            font.weight: Font.DemiBold
                        }
                    }
                    Label {
                        visible: repoTab.lastError !== ""
                        text: repoTab.lastError
                        color: Theme.danger
                        elide: Text.ElideRight
                        Layout.maximumWidth: 320
                        font.pixelSize: Theme.fontSm
                        MouseArea {
                            anchors.fill: parent
                            onClicked: repoTab.clearLastError()
                        }
                    }

                    Item { Layout.fillWidth: true }

                    // Reserved: auto-fetch indicator (Phase 4)
                    Label {
                        text: "↻"
                        color: Theme.borderDefault
                        font.pixelSize: Theme.fontMd
                    }
                    // Reserved: search box (backlog)
                    TextField {
                        enabled: false
                        opacity: 0.35
                        placeholderText: qsTr("Search")
                        implicitWidth: 160
                        implicitHeight: Theme.controlHeight
                    }
                    ToolButton {
                        text: qsTr("⚑ Tags")
                        checkable: true
                        checked: repoTab.tagsShown
                        onToggled: repoTab.setTagsShown(checked)
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
                handle: Rectangle {
                    implicitWidth: Theme.splitterWidth
                    implicitHeight: Theme.splitterWidth
                    color: Theme.borderSubtle
                }

                // Navigation sidebar: branches / remotes / working tree /
                // stashes / tags (collapsible sections, shared filter).
                Rectangle {
                    SplitView.preferredWidth: 260
                    SplitView.minimumWidth: 180
                    color: Theme.bgSurface
                    ColumnLayout {
                        anchors.fill: parent
                        spacing: 0
                        TextField {
                            id: refFilter
                            Layout.fillWidth: true
                            Layout.margins: Theme.spaceSm
                            implicitHeight: Theme.controlHeight
                            placeholderText: qsTr("Filter")
                            onTextChanged: sidebarModel.setFilter(text)
                        }
                        ListView {
                            id: sidebarList
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            clip: true
                            model: sidebarModel
                            reuseItems: true
                            ScrollBar.vertical: ScrollBar {}
                            delegate: SidebarRowDelegate {
                                listWidth: sidebarList.width
                                onSectionToggled: group => sidebarModel.toggleGroup(group)
                                onRefActivated: oidHex => {
                                    const row = graphModel.rowOf(oidHex)
                                    if (row >= 0) {
                                        graphList.currentIndex = row
                                        graphList.positionViewAtIndex(row, ListView.Center)
                                    }
                                    // Details resolve even outside the window.
                                    page.selectedOid = oidHex
                                    detailsModel.request(oidHex)
                                    diffModel.clear()
                                }
                                onFileActivated: (bucket, path, origPath) =>
                                    diffModel.requestWorkTree(bucket, path, origPath)
                            }
                        }
                    }
                }

                // Commit graph
                Rectangle {
                    SplitView.fillWidth: true
                    SplitView.minimumWidth: 420
                    color: Theme.bgSurface
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
                            page.selectedOid = oidHex
                            detailsModel.request(oidHex)
                            diffModel.clear()
                        }
                        delegate: GraphRowDelegate {}
                        // Window-limit hint appears only when scrolled to
                        // the very end (GitKraken-style truncation).
                        footer: Rectangle {
                            width: graphList.width
                            height: graphModel.truncated ? Theme.rowHeight : 0
                            visible: graphModel.truncated
                            color: Theme.bgElevated
                            Label {
                                anchors.centerIn: parent
                                text: qsTr("Showing the first %L1 commits").arg(graphModel.rowTotal)
                                color: Theme.textSecondary
                                font.pixelSize: Theme.fontSm
                            }
                        }
                        // Mouse wheels scroll a fixed number of rows per
                        // notch; touchpads keep native Flickable panning.
                        WheelHandler {
                            acceptedDevices: PointerDevice.Mouse
                            onWheel: event => {
                                graphList.cancelFlick()
                                const step = (event.angleDelta.y / 120)
                                           * root.wheelRows * Theme.rowHeight
                                const maxY = Math.max(0, graphList.contentHeight - graphList.height)
                                graphList.contentY = Math.max(0, Math.min(graphList.contentY - step, maxY))
                            }
                        }
                    }
                    // Middle-drag autoscroll: hold the middle button and
                    // drag vertically; speed follows the drag distance.
                    MouseArea {
                        id: midScroll
                        anchors.fill: parent
                        acceptedButtons: Qt.MiddleButton
                        property real anchorY: 0
                        property real currentY: 0
                        onPressed: mouse => {
                            anchorY = mouse.y
                            currentY = mouse.y
                            midScrollTimer.start()
                        }
                        onReleased: midScrollTimer.stop()
                        onCanceled: midScrollTimer.stop()
                        onPositionChanged: mouse => currentY = mouse.y
                        Timer {
                            id: midScrollTimer
                            interval: 16
                            repeat: true
                            onTriggered: {
                                const delta = (midScroll.currentY - midScroll.anchorY)
                                            * root.middleScrollGain
                                const maxY = Math.max(0, graphList.contentHeight - graphList.height)
                                graphList.contentY = Math.max(0, Math.min(graphList.contentY + delta, maxY))
                            }
                        }
                    }
                    BusyIndicator {
                        anchors.centerIn: parent
                        running: graphModel.loading && graphModel.rowTotal === 0
                    }
                    Label {
                        anchors.centerIn: parent
                        visible: graphModel.error !== ""
                        text: graphModel.error
                        color: Theme.danger
                        width: parent.width - 2 * Theme.spaceXl
                        wrapMode: Text.Wrap
                        horizontalAlignment: Text.AlignHCenter
                    }
                }

                // Right side: commit details + diff
                Rectangle {
                    SplitView.preferredWidth: 460
                    SplitView.minimumWidth: 320
                    color: Theme.bgSurface
                    SplitView {
                        anchors.fill: parent
                        orientation: Qt.Vertical
                        handle: Rectangle {
                            implicitWidth: Theme.splitterWidth
                            implicitHeight: Theme.splitterWidth
                            color: Theme.borderSubtle
                        }

                        // Commit details + file list
                        ColumnLayout {
                            SplitView.preferredHeight: 320
                            SplitView.minimumHeight: 160
                            spacing: 0
                            PaneHeader { text: qsTr("COMMIT") }
                            ColumnLayout {
                                Layout.fillWidth: true
                                Layout.margins: Theme.spaceSm
                                spacing: Theme.spaceXs
                                visible: detailsModel.shaHex !== ""
                                RowLayout {
                                    spacing: Theme.spaceXs
                                    Label {
                                        text: detailsModel.sha8
                                        font.family: Theme.monoFamily
                                        color: Theme.textLink
                                        font.pixelSize: Theme.fontSm
                                    }
                                    ToolButton {
                                        text: "⧉"
                                        padding: 0
                                        implicitWidth: Theme.iconLg
                                        implicitHeight: Theme.iconLg
                                        ToolTip.visible: hovered
                                        ToolTip.delay: 600
                                        ToolTip.text: qsTr("Copy full hash")
                                        onClicked: root.copyText(detailsModel.shaHex)
                                    }
                                    Label {
                                        id: detailsDate
                                        text: Qt.formatDateTime(new Date(detailsModel.authorTime * 1000),
                                                                "yyyy-MM-dd HH:mm")
                                        color: Theme.textSecondary
                                        font.pixelSize: Theme.fontSm
                                    }
                                    ToolButton {
                                        text: "⧉"
                                        padding: 0
                                        implicitWidth: Theme.iconLg
                                        implicitHeight: Theme.iconLg
                                        ToolTip.visible: hovered
                                        ToolTip.delay: 600
                                        ToolTip.text: qsTr("Copy date")
                                        onClicked: root.copyText(detailsDate.text)
                                    }
                                    Label {
                                        text: detailsModel.author
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                        color: Theme.textSecondary
                                        font.pixelSize: Theme.fontSm
                                    }
                                }
                                ScrollView {
                                    Layout.fillWidth: true
                                    Layout.preferredHeight: Math.min(messageArea.implicitHeight + Theme.spaceSm, 96)
                                    TextArea {
                                        id: messageArea
                                        readOnly: true
                                        wrapMode: TextArea.Wrap
                                        text: detailsModel.message
                                        font.pixelSize: Theme.fontMd
                                        color: Theme.textPrimary
                                        background: null
                                    }
                                }
                            }
                            Label {
                                visible: detailsModel.shaHex === ""
                                Layout.margins: Theme.spaceSm
                                text: qsTr("Select a commit to see its details")
                                color: Theme.textMuted
                                font.pixelSize: Theme.fontSm
                            }
                            ListView {
                                id: fileList
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                clip: true
                                model: detailsModel
                                reuseItems: true
                                ScrollBar.vertical: ScrollBar {}
                                delegate: FileRowDelegate {
                                    listWidth: fileList.width
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
                                Layout.margins: Theme.spaceSm
                                text: qsTr("Binary file — no text diff")
                                color: Theme.textMuted
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
                                    height: Theme.rowHeight
                                    color: kind === "add" ? Theme.diffAddedBg
                                           : kind === "del" ? Theme.diffRemovedBg
                                           : kind === "hunk" ? Theme.diffHunkHeaderBg
                                           : "transparent"
                                    Row {
                                        anchors.fill: parent
                                        spacing: 0
                                        Label {
                                            width: 42
                                            height: parent.height
                                            verticalAlignment: Text.AlignVCenter
                                            text: diffRow.old_no >= 0 ? diffRow.old_no : ""
                                            horizontalAlignment: Text.AlignRight
                                            rightPadding: Theme.spaceXs
                                            color: Theme.textMuted
                                            font.family: Theme.monoFamily
                                            font.pixelSize: Theme.fontSm
                                        }
                                        Label {
                                            width: 42
                                            height: parent.height
                                            verticalAlignment: Text.AlignVCenter
                                            text: diffRow.new_no >= 0 ? diffRow.new_no : ""
                                            horizontalAlignment: Text.AlignRight
                                            rightPadding: Theme.spaceXs
                                            color: Theme.textMuted
                                            font.family: Theme.monoFamily
                                            font.pixelSize: Theme.fontSm
                                        }
                                        Label {
                                            width: parent.width - 84
                                            height: parent.height
                                            verticalAlignment: Text.AlignVCenter
                                            text: diffRow.text
                                            elide: Text.ElideRight
                                            font.family: Theme.monoFamily
                                            font.pixelSize: Theme.fontMd
                                            color: diffRow.kind === "add" ? Theme.diffAddedFg
                                                   : diffRow.kind === "del" ? Theme.diffRemovedFg
                                                   : diffRow.kind === "hunk" ? Theme.diffHunkHeaderFg
                                                   : diffRow.kind === "meta" ? Theme.textMuted
                                                   : Theme.textPrimary
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
    // Commit-graph row: [ref label][lanes][avatar][subject] — fixed-width
    // label and graph columns keep every subject start aligned.
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
        required property int avatar
        required property string geometry
        required property string labels

        width: ListView.view.width
        height: Theme.rowHeight

        readonly property bool selected: ListView.isCurrentItem
        // Chip records are separated by U+001F (see encode.rs).
        readonly property var labelRecords: labels === "" ? [] : labels.split(String.fromCharCode(31))

        Rectangle {
            anchors.fill: parent
            color: Theme.bgSelected
            visible: rowItem.selected
        }
        Rectangle {
            anchors.fill: parent
            color: Theme.bgHover
            visible: rowMouse.containsMouse && !rowItem.selected
        }

        onGeometryChanged: laneCanvas.requestPaint()
        onNode_laneChanged: laneCanvas.requestPaint()
        onNode_colorChanged: laneCanvas.requestPaint()
        onAvatarChanged: avatarCanvas.requestPaint()

        RowLayout {
            anchors.fill: parent
            spacing: Theme.spaceSm

            // Ref label column: one chip per commit; hover lists them all.
            Item {
                Layout.preferredWidth: root.labelColW
                Layout.fillHeight: true
                Rectangle {
                    id: chip
                    visible: rowItem.labelRecords.length > 0
                    anchors.right: parent.right
                    anchors.rightMargin: Theme.spaceXs
                    anchors.verticalCenter: parent.verticalCenter
                    height: Theme.fontSmLine
                    width: Math.min(chipRow.implicitWidth + 2 * Theme.spaceXs,
                                    root.labelColW - Theme.spaceXs)
                    radius: Theme.radiusSm
                    color: "transparent"
                    clip: true
                    readonly property string rec: rowItem.labelRecords.length > 0
                                                  ? rowItem.labelRecords[0] : "L00"
                    readonly property string chipKind: rec[0]
                    readonly property bool chipHead: rec[1] === "1"
                    readonly property bool chipRemote: rec[2] === "1"
                    readonly property color chipColor: chipKind === "T" ? Theme.warning
                                                      : chipKind === "R" ? Theme.textSecondary
                                                      : chipKind === "H" ? Theme.danger
                                                      : Theme.accent
                    border.color: chipColor
                    border.width: Theme.borderWidth
                    Row {
                        id: chipRow
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.spaceXs
                        spacing: Theme.spaceXs
                        Rectangle {
                            visible: chip.chipKind === "L"
                            anchors.verticalCenter: parent.verticalCenter
                            width: Theme.spaceXs + 2
                            height: Theme.spaceXs + 2
                            radius: Theme.radiusSm + 1
                            color: chip.chipRemote ? chip.chipColor : "transparent"
                            border.color: chip.chipColor
                            border.width: Theme.borderWidth
                        }
                        Label {
                            text: chip.rec.substring(3)
                            color: chip.chipColor
                            font.pixelSize: Theme.fontSm
                            font.weight: chip.chipHead ? Font.DemiBold : Font.Normal
                            elide: Text.ElideRight
                            width: Math.min(implicitWidth,
                                            root.labelColW - 5 * Theme.spaceXs
                                            - (chip.chipKind === "L" ? Theme.spaceSm : 0)
                                            - (rowItem.labelRecords.length > 1 ? Theme.spaceLg : 0))
                        }
                        Label {
                            visible: rowItem.labelRecords.length > 1
                            text: "+" + (rowItem.labelRecords.length - 1)
                            color: chip.chipColor
                            font.pixelSize: Theme.fontSm
                        }
                    }
                    MouseArea {
                        id: chipMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.NoButton
                    }
                    ToolTip.visible: chipMouse.containsMouse
                    ToolTip.delay: 300
                    ToolTip.text: {
                        let lines = []
                        for (let i = 0; i < rowItem.labelRecords.length; i++) {
                            const r = rowItem.labelRecords[i]
                            const icon = r[0] === "T" ? "⚑" : r[0] === "R" ? "☁"
                                       : r[0] === "H" ? "HEAD" : "⎇"
                            lines.push(icon + " " + r.substring(3))
                        }
                        return lines.join("\n")
                    }
                }
            }

            Canvas {
                id: laneCanvas
                // graphAreaWidth is provided on the ListView (page scope).
                Layout.preferredWidth: rowItem.ListView.view ? rowItem.ListView.view.graphAreaWidth : 120
                Layout.fillHeight: true
                onPaint: {
                    const ctx = getContext("2d")
                    ctx.clearRect(0, 0, width, height)
                    ctx.lineWidth = root.laneStroke
                    const laneCount = Theme.graphLane.length
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
                            ctx.strokeStyle = Theme.graphLane[parseInt(t.substring(dot + 1)) % laneCount]
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
                    const r = root.nodeDiameter / 2
                    ctx.fillStyle = Theme.graphLane[rowItem.node_color % laneCount]
                    ctx.beginPath()
                    ctx.arc(nodeX, midY, r - 1, 0, 2 * Math.PI)
                    ctx.fill()
                    ctx.strokeStyle = Theme.bgSurface
                    ctx.lineWidth = Theme.borderWidth
                    ctx.beginPath()
                    ctx.arc(nodeX, midY, r - 1, 0, 2 * Math.PI)
                    ctx.stroke()
                }
            }

            // Local identicon (5x5, mirrored pattern from the avatar code;
            // fetching real avatars would require network access).
            Canvas {
                id: avatarCanvas
                Layout.preferredWidth: Theme.iconMd
                Layout.preferredHeight: Theme.iconMd
                Layout.alignment: Qt.AlignVCenter
                onPaint: {
                    const ctx = getContext("2d")
                    const cell = width / 5
                    ctx.clearRect(0, 0, width, height)
                    ctx.fillStyle = Theme.bgElevated
                    ctx.fillRect(0, 0, width, height)
                    ctx.fillStyle = Theme.graphLane[(rowItem.avatar >> 15) & 0x7]
                    for (let row = 0; row < 5; row++) {
                        for (let col = 0; col < 3; col++) {
                            if ((rowItem.avatar >> (row * 3 + col)) & 1) {
                                ctx.fillRect(col * cell, row * cell, cell + 0.5, cell + 0.5)
                                if (col < 2)
                                    ctx.fillRect((4 - col) * cell, row * cell, cell + 0.5, cell + 0.5)
                            }
                        }
                    }
                }
            }

            Label {
                Layout.fillWidth: true
                text: rowItem.subject
                elide: Text.ElideRight
                font.pixelSize: Theme.fontMd
                color: Theme.textPrimary
                rightPadding: Theme.spaceSm
            }
        }

        MouseArea {
            id: rowMouse
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.LeftButton
            onClicked: {
                rowItem.ListView.view.currentIndex = rowItem.index
                rowItem.ListView.view.rowSelected(rowItem.oid_hex)
            }
        }
        // Hover details: what the row no longer shows as columns.
        ToolTip.visible: rowMouse.containsMouse
        ToolTip.delay: 700
        ToolTip.text: rowItem.author + "\n"
                      + Qt.formatDateTime(new Date(rowItem.atime * 1000), "yyyy-MM-dd HH:mm") + "\n"
                      + rowItem.oid_hex.substring(0, 8)
    }

    // ======================================================================
    // Sidebar row: section header or item (ref / work-tree file / stash)
    // ======================================================================
    component SidebarRowDelegate: Item {
        id: navRow
        required property int index
        required property string kind
        required property string group
        required property string name
        required property string oid_hex
        required property string change
        required property string bucket
        required property string orig_path
        required property bool is_head
        required property bool has_remote
        required property bool collapsed
        required property int count
        property real listWidth: 200

        signal sectionToggled(string group)
        signal refActivated(string oidHex)
        signal fileActivated(string bucket, string path, string origPath)

        readonly property bool isHeader: kind === "header"

        width: listWidth
        height: Theme.rowHeight

        // ---- section header ----
        Rectangle {
            anchors.fill: parent
            visible: navRow.isHeader
            color: Theme.bgElevated
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.spaceSm
                anchors.rightMargin: Theme.spaceSm
                spacing: Theme.spaceXs
                Label {
                    text: navRow.collapsed ? "▸" : "▾"
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                }
                Label {
                    text: navRow.group === "branches" ? qsTr("BRANCHES")
                          : navRow.group === "remotes" ? qsTr("REMOTES")
                          : navRow.group === "worktree" ? qsTr("WORKING TREE")
                          : navRow.group === "stashes" ? qsTr("STASHES")
                          : qsTr("TAGS")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                    font.weight: Font.DemiBold
                }
                Label {
                    text: "(" + navRow.count + ")"
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontSm
                }
                Item { Layout.fillWidth: true }
            }
            MouseArea {
                anchors.fill: parent
                onClicked: navRow.sectionToggled(navRow.group)
            }
        }

        // ---- item row ----
        Item {
            anchors.fill: parent
            visible: !navRow.isHeader
            Rectangle {
                anchors.fill: parent
                color: Theme.bgHover
                visible: itemMouse.containsMouse
            }
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.spaceMd
                anchors.rightMargin: Theme.spaceSm
                spacing: Theme.spaceXs
                // Kind marker: ref icon or work-tree change letter.
                Label {
                    visible: navRow.kind !== "wt"
                    text: navRow.kind === "tag" ? "⚑"
                          : navRow.kind === "remote" ? "☁"
                          : navRow.kind === "stash" ? "☰" : "⎇"
                    color: navRow.kind === "tag" ? Theme.warning
                           : navRow.kind === "remote" ? Theme.textSecondary
                           : navRow.kind === "stash" ? Theme.textSecondary
                           : Theme.accent
                    font.pixelSize: Theme.fontSm
                }
                Label {
                    visible: navRow.kind === "wt"
                    text: navRow.change
                    font.family: Theme.monoFamily
                    font.pixelSize: Theme.fontSm
                    font.weight: Font.DemiBold
                    color: navRow.bucket === "staged" ? Theme.statusStaged
                           : navRow.bucket === "unstaged" ? Theme.statusUnstaged
                           : navRow.bucket === "untracked" ? Theme.statusUntracked
                           : Theme.statusConflict
                    Layout.preferredWidth: Theme.iconLg
                }
                Label {
                    Layout.fillWidth: true
                    text: navRow.name
                    elide: Text.ElideMiddle
                    font.weight: navRow.is_head ? Font.DemiBold : Font.Normal
                    color: navRow.is_head ? Theme.textLink : Theme.textPrimary
                    font.pixelSize: Theme.fontMd
                }
                // Branch state badge: filled = has remote, hollow = local
                // only (PR state joins in Phase 4 as a third look).
                Rectangle {
                    visible: navRow.kind === "branch"
                    width: Theme.spaceSm
                    height: Theme.spaceSm
                    radius: Theme.spaceXs
                    color: navRow.has_remote ? Theme.accent : "transparent"
                    border.color: navRow.has_remote ? Theme.accent : Theme.textMuted
                    border.width: Theme.borderWidth
                }
            }
            MouseArea {
                id: itemMouse
                anchors.fill: parent
                hoverEnabled: true
                onClicked: {
                    if (navRow.kind === "wt")
                        navRow.fileActivated(navRow.bucket, navRow.name, navRow.orig_path)
                    else if (navRow.oid_hex !== "")
                        navRow.refActivated(navRow.oid_hex)
                }
            }
        }
    }

    // ======================================================================
    // A changed-file row (commit file list)
    // ======================================================================
    component FileRowDelegate: Item {
        id: fileRow
        required property var model
        property real listWidth: 200

        readonly property string changeText: model.change ?? ""
        readonly property string pathText: model.path ?? ""
        readonly property string origPathText: model.orig_path ?? ""

        signal activated(string bucket, string path, string origPath)

        width: listWidth
        height: Theme.rowHeight

        Rectangle {
            anchors.fill: parent
            color: Theme.bgHover
            visible: fileMouse.containsMouse
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.spaceSm
            anchors.rightMargin: Theme.spaceSm
            spacing: Theme.spaceXs
            Label {
                text: fileRow.changeText
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
                color: fileRow.changeText.startsWith("A") ? Theme.diffAddedFg
                       : fileRow.changeText.startsWith("D") ? Theme.diffRemovedFg
                       : Theme.textSecondary
                Layout.preferredWidth: Theme.iconLg
            }
            Label {
                Layout.fillWidth: true
                text: fileRow.origPathText === ""
                      ? fileRow.pathText
                      : qsTr("%1 → %2").arg(fileRow.origPathText).arg(fileRow.pathText)
                elide: Text.ElideMiddle
                font.pixelSize: Theme.fontMd
                color: Theme.textPrimary
            }
        }
        MouseArea {
            id: fileMouse
            anchors.fill: parent
            hoverEnabled: true
            onClicked: fileRow.activated("", fileRow.pathText, fileRow.origPathText)
        }
    }

    component PaneHeader: Rectangle {
        property alias text: headerLabel.text
        Layout.fillWidth: true
        implicitHeight: Theme.headerHeight
        color: Theme.bgElevated
        Label {
            id: headerLabel
            anchors.verticalCenter: parent.verticalCenter
            x: Theme.spaceSm
            font.pixelSize: Theme.fontSm
            font.weight: Font.DemiBold
            color: Theme.textSecondary
        }
    }
}
