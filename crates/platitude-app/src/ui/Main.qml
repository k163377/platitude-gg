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
    // iconSm, lane stroke = 2, wheel step = 5 rows per notch.
    readonly property int laneW: Theme.spaceLg
    readonly property int nodeDiameter: Theme.iconSm
    readonly property int laneStroke: 2
    readonly property int wheelRows: 5

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

                // Sidebar: refs + filter
                Rectangle {
                    SplitView.preferredWidth: 240
                    SplitView.minimumWidth: 160
                    color: Theme.bgSurface
                    ColumnLayout {
                        anchors.fill: parent
                        spacing: 0
                        TextField {
                            id: refFilter
                            Layout.fillWidth: true
                            Layout.margins: Theme.spaceSm
                            implicitHeight: Theme.controlHeight
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
                            ScrollBar.vertical: ScrollBar {}
                            section.property: "group"
                            section.delegate: Rectangle {
                                required property string section
                                width: sidebarList.width
                                height: Theme.rowHeight
                                color: Theme.bgSurface
                                Label {
                                    anchors.verticalCenter: parent.verticalCenter
                                    x: Theme.spaceSm
                                    text: parent.section
                                    color: Theme.textSecondary
                                    font.pixelSize: Theme.fontSm
                                    font.weight: Font.DemiBold
                                }
                            }
                            delegate: Item {
                                id: refRow
                                required property string name
                                required property string oid_hex
                                required property string kind
                                required property bool is_head
                                required property bool has_remote
                                width: sidebarList.width
                                height: Theme.rowHeight
                                Rectangle {
                                    anchors.fill: parent
                                    color: Theme.bgHover
                                    visible: refMouse.containsMouse
                                }
                                RowLayout {
                                    anchors.fill: parent
                                    anchors.leftMargin: Theme.spaceSm
                                    anchors.rightMargin: Theme.spaceSm
                                    spacing: Theme.spaceXs
                                    Label {
                                        text: refRow.kind === "tag" ? "⚑"
                                              : refRow.kind === "remote" ? "☁" : "⎇"
                                        color: refRow.kind === "tag" ? Theme.warning
                                               : refRow.kind === "remote" ? Theme.textSecondary
                                               : Theme.accent
                                        font.pixelSize: Theme.fontSm
                                    }
                                    Label {
                                        Layout.fillWidth: true
                                        text: refRow.name
                                        elide: Text.ElideMiddle
                                        font.weight: refRow.is_head ? Font.DemiBold : Font.Normal
                                        color: refRow.is_head ? Theme.textLink : Theme.textPrimary
                                        font.pixelSize: Theme.fontMd
                                    }
                                    // Branch state badge: filled = has remote,
                                    // hollow = local-only (PR state joins in
                                    // Phase 4 as a third look).
                                    Rectangle {
                                        visible: refRow.kind === "branch"
                                        width: Theme.spaceSm
                                        height: Theme.spaceSm
                                        radius: Theme.spaceXs
                                        color: refRow.has_remote ? Theme.accent : "transparent"
                                        border.color: refRow.has_remote ? Theme.accent : Theme.textMuted
                                        border.width: Theme.borderWidth
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
                                        }
                                        // Outside the graph window the row is
                                        // absent, but details still resolve.
                                        detailsModel.request(refRow.oid_hex)
                                    }
                                }
                            }
                        }
                        PaneHeader {
                            text: qsTr("STASHES (%1)").arg(stashList.count)
                        }
                        ListView {
                            id: stashList
                            Layout.fillWidth: true
                            Layout.preferredHeight: Math.min(count * Theme.rowHeight + Theme.spaceXs, 120)
                            clip: true
                            model: stashModel
                            delegate: Item {
                                id: stashRow
                                required property string name
                                required property string message
                                width: stashList.width
                                height: Theme.rowHeight
                                Label {
                                    x: Theme.spaceSm
                                    anchors.verticalCenter: parent.verticalCenter
                                    width: parent.width - 2 * Theme.spaceSm
                                    elide: Text.ElideRight
                                    font.pixelSize: Theme.fontMd
                                    color: Theme.textPrimary
                                    text: stashRow.name + "  " + stashRow.message
                                }
                            }
                        }
                    }
                }

                // Commit graph
                Rectangle {
                    SplitView.fillWidth: true
                    SplitView.minimumWidth: 420
                    color: Theme.bgSurface
                    ColumnLayout {
                        anchors.fill: parent
                        spacing: 0
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
                        // Window-limit hint (GitKraken-style truncation).
                        Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: Theme.rowHeight
                            visible: graphModel.truncated
                            color: Theme.bgElevated
                            Label {
                                anchors.centerIn: parent
                                text: qsTr("Showing the first %L1 commits").arg(graphModel.rowTotal)
                                color: Theme.textSecondary
                                font.pixelSize: Theme.fontSm
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

                // Right side: working tree (persistent) + details + diff
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
                                    height: Theme.iconLg
                                    color: Theme.bgElevated
                                    Label {
                                        anchors.verticalCenter: parent.verticalCenter
                                        x: Theme.spaceSm
                                        font.pixelSize: Theme.fontSm
                                        font.weight: Font.DemiBold
                                        color: parent.section === "conflicts" ? Theme.statusConflict
                                               : parent.section === "staged" ? Theme.statusStaged
                                               : parent.section === "unstaged" ? Theme.statusUnstaged
                                               : Theme.textSecondary
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
                        }

                        // Commit details + file list
                        ColumnLayout {
                            SplitView.preferredHeight: 240
                            SplitView.minimumHeight: 120
                            spacing: 0
                            PaneHeader { text: qsTr("COMMIT") }
                            ColumnLayout {
                                Layout.fillWidth: true
                                Layout.margins: Theme.spaceSm
                                spacing: Theme.spaceXs
                                visible: detailsModel.shaHex !== ""
                                RowLayout {
                                    spacing: Theme.spaceSm
                                    Label {
                                        text: detailsModel.sha8
                                        font.family: Theme.monoFamily
                                        color: Theme.textLink
                                        font.pixelSize: Theme.fontSm
                                    }
                                    Label {
                                        text: detailsModel.author
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                        color: Theme.textSecondary
                                        font.pixelSize: Theme.fontSm
                                    }
                                    Label {
                                        text: Qt.formatDateTime(new Date(detailsModel.authorTime * 1000),
                                                                "yyyy-MM-dd HH:mm")
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
        height: Theme.rowHeight

        readonly property bool selected: ListView.isCurrentItem

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

        RowLayout {
            anchors.fill: parent
            spacing: Theme.spaceSm

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

            // Ref label chips (precomputed records: K + head + remote + text)
            Row {
                spacing: Theme.spaceXs
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
                        readonly property color chipColor: chipKind === "T" ? Theme.warning
                                                          : chipKind === "R" ? Theme.textSecondary
                                                          : chipKind === "H" ? Theme.danger
                                                          : Theme.accent
                        height: Theme.fontSmLine
                        width: chipRow.implicitWidth + 2 * Theme.spaceXs
                        radius: Theme.radiusSm
                        color: "transparent"
                        border.color: chipColor
                        border.width: Theme.borderWidth
                        Row {
                            id: chipRow
                            anchors.centerIn: parent
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
                                text: chip.chipText
                                color: chip.chipColor
                                font.pixelSize: Theme.fontSm
                                font.weight: chip.chipHead ? Font.DemiBold : Font.Normal
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
            }
            Label {
                Layout.preferredWidth: 130
                text: rowItem.author
                elide: Text.ElideRight
                font.pixelSize: Theme.fontSm
                color: Theme.textSecondary
            }
            Label {
                Layout.preferredWidth: 110
                text: Qt.formatDateTime(new Date(rowItem.atime * 1000), "yyyy-MM-dd HH:mm")
                font.pixelSize: Theme.fontSm
                color: Theme.textSecondary
            }
            Label {
                Layout.preferredWidth: 64
                text: rowItem.oid_hex.substring(0, 8)
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontSm
                color: Theme.textSecondary
                rightPadding: Theme.spaceSm
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
    component FileRowDelegate: Item {
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
                // Work-tree rows color by bucket (status tokens); commit
                // file rows only distinguish added/removed (diff tokens).
                color: fileRow.bucketText === "staged" ? Theme.statusStaged
                       : fileRow.bucketText === "unstaged" ? Theme.statusUnstaged
                       : fileRow.bucketText === "untracked" ? Theme.statusUntracked
                       : fileRow.bucketText === "conflicts" ? Theme.statusConflict
                       : fileRow.changeText.startsWith("A") ? Theme.diffAddedFg
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
            onClicked: fileRow.activated(fileRow.bucketText, fileRow.pathText, fileRow.origPathText)
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
