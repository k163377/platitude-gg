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

    // Graph-geometry / interaction constants still being tuned. This block
    // is their single source of truth for now (デザイン規約.md §運用の例外);
    // values follow the guideline principles (4px grid, token reuse) and
    // graduate into the document once they stabilize.
    readonly property int laneW: Theme.iconLg
    readonly property int laneInset: Theme.spaceSm
    readonly property int nodeIcon: Theme.iconLg
    readonly property int laneStroke: 2
    readonly property real iconStroke: 1.5
    readonly property real identiconFill: 0.72
    readonly property int wheelRows: 6
    readonly property real middleScrollGain: 0.12
    readonly property int labelColW: 144
    readonly property int detailsAvatar: 40

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
        if (AppBackend.autoOpen !== "") {
            // Multiple repositories separated by ';' open as tabs in order.
            const paths = AppBackend.autoOpen.split(";")
            for (let i = 0; i < paths.length; i++) {
                if (paths[i] !== "")
                    tabsModel.openRepositoryPath(paths[i])
            }
        }
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
        readonly property int graphAreaW: root.laneInset + shownLanes * root.laneW + Theme.spaceSm
        property string selectedOid: ""

        // Center area switches between the graph and a file diff.
        property bool diffShown: false
        property string diffKey: ""
        function toggleDiff(kind, path, origPath) {
            const key = kind + ":" + path
            if (page.diffShown && page.diffKey === key) {
                page.closeDiff()
                return
            }
            page.diffKey = key
            if (kind === "commit")
                diffModel.requestCommitFile(detailsModel.shaHex, detailsModel.parentHex,
                                            path, origPath)
            else
                diffModel.requestWorkTree(kind, path, origPath)
            page.diffShown = true
        }
        function closeDiff() {
            page.diffShown = false
            page.diffKey = ""
            diffModel.clear()
        }

        // Sidebar section expansion (filter reveals collapsed sections).
        property bool expBranches: true
        property bool expRemotes: true
        property bool expWorktree: true
        property bool expStashes: true
        property bool expTags: true

        RepoTab { id: repoTab }
        GraphModel { id: graphModel }
        WorkTreeModel { id: workTree }
        DetailsModel { id: detailsModel }
        DiffModel { id: diffModel }
        NavSectionModel { id: branchesModel }
        NavSectionModel { id: remotesModel }
        NavSectionModel { id: worktreeModel }
        NavSectionModel { id: stashesModel }
        NavSectionModel { id: tagsModel }

        Component.onCompleted: {
            repoTab.attach(page.tab_id)
            graphModel.attach(page.tab_id)
            workTree.attach(page.tab_id)
            detailsModel.attach(page.tab_id)
            diffModel.attach(page.tab_id)
            branchesModel.attachSection(page.tab_id, "branches")
            remotesModel.attachSection(page.tab_id, "remotes")
            worktreeModel.attachSection(page.tab_id, "worktree")
            stashesModel.attachSection(page.tab_id, "stashes")
            tagsModel.attachSection(page.tab_id, "tags")
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
                    page.toggleDiff("commit", detailsModel.filePathAt(0),
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

                // Navigation sidebar: fixed section headers, each section
                // scrolls inside its own list.
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
                            onTextChanged: {
                                branchesModel.setFilter(text)
                                remotesModel.setFilter(text)
                                worktreeModel.setFilter(text)
                                stashesModel.setFilter(text)
                                tagsModel.setFilter(text)
                            }
                        }

                        NavHeader {
                            caption: qsTr("BRANCHES")
                            iconKind: "branch"
                            iconTint: Theme.accent
                            count: branchesModel.total
                            expanded: page.expBranches || refFilter.text !== ""
                            onToggled: page.expBranches = !page.expBranches
                        }
                        NavList {
                            sectionModel: branchesModel
                            expanded: page.expBranches || refFilter.text !== ""
                            kindHint: "branch"
                            onRefActivated: oidHex => page.jumpToRef(oidHex)
                        }

                        NavHeader {
                            caption: qsTr("REMOTES")
                            iconKind: "remote"
                            iconTint: Theme.textLink
                            count: remotesModel.total
                            expanded: page.expRemotes || refFilter.text !== ""
                            onToggled: page.expRemotes = !page.expRemotes
                        }
                        NavList {
                            sectionModel: remotesModel
                            expanded: page.expRemotes || refFilter.text !== ""
                            kindHint: "remote"
                            onRefActivated: oidHex => page.jumpToRef(oidHex)
                        }

                        NavHeader {
                            caption: qsTr("WORKING TREE")
                            iconKind: "tree"
                            iconTint: Theme.success
                            count: worktreeModel.total
                            expanded: page.expWorktree || refFilter.text !== ""
                            onToggled: page.expWorktree = !page.expWorktree
                        }
                        NavList {
                            sectionModel: worktreeModel
                            expanded: page.expWorktree || refFilter.text !== ""
                            kindHint: "wt"
                            onFileActivated: (bucket, path, origPath) =>
                                page.toggleDiff(bucket, path, origPath)
                        }

                        NavHeader {
                            caption: qsTr("STASHES")
                            iconKind: "stash"
                            iconTint: Theme.textSecondary
                            count: stashesModel.total
                            expanded: page.expStashes || refFilter.text !== ""
                            onToggled: page.expStashes = !page.expStashes
                        }
                        NavList {
                            sectionModel: stashesModel
                            expanded: page.expStashes || refFilter.text !== ""
                            kindHint: "stash"
                        }

                        NavHeader {
                            caption: qsTr("TAGS")
                            iconKind: "tag"
                            iconTint: Theme.warning
                            count: tagsModel.total
                            expanded: page.expTags || refFilter.text !== ""
                            onToggled: page.expTags = !page.expTags
                            showTagToggle: true
                            tagsShown: repoTab.tagsShown
                            onTagsToggled: shown => repoTab.setTagsShown(shown)
                        }
                        NavList {
                            sectionModel: tagsModel
                            expanded: page.expTags || refFilter.text !== ""
                            kindHint: "tag"
                            onRefActivated: oidHex => page.jumpToRef(oidHex)
                        }

                        // Absorbs leftover space when sections are collapsed.
                        Item { Layout.fillHeight: true; Layout.minimumHeight: 0 }
                    }
                }

                // Center: commit graph ⇄ file diff
                StackLayout {
                    SplitView.fillWidth: true
                    SplitView.minimumWidth: 420
                    currentIndex: page.diffShown ? 1 : 0

                    // -- graph --
                    Rectangle {
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
                            // Bridge into the page scope for the shared
                            // delegate (inline components cannot see page ids).
                            property int graphAreaWidth: page.graphAreaW
                            signal rowSelected(string oidHex)
                            onRowSelected: oidHex => {
                                page.selectedOid = oidHex
                                detailsModel.request(oidHex)
                                page.closeDiff()
                            }
                            delegate: GraphRowDelegate {}
                            // Window cut: lanes keep running through the
                            // footer and the message sits where subjects go.
                            footer: Item {
                                width: graphList.width
                                height: graphModel.truncated ? 2 * Theme.rowHeight : 0
                                visible: graphModel.truncated
                                Canvas {
                                    id: tailCanvas
                                    x: root.labelColW + Theme.spaceSm
                                    width: page.graphAreaW
                                    height: parent.height
                                    onPaint: {
                                        const ctx = getContext("2d")
                                        ctx.clearRect(0, 0, width, height)
                                        if (graphModel.tailGeometry === "")
                                            return
                                        ctx.lineWidth = root.laneStroke
                                        ctx.globalAlpha = 0.45
                                        const toks = graphModel.tailGeometry.split(";")
                                        for (let n = 0; n < toks.length; n++) {
                                            const dot = toks[n].indexOf(".")
                                            const lane = parseInt(toks[n].substring(0, dot))
                                            const color = parseInt(toks[n].substring(dot + 1))
                                            const x = root.laneInset + lane * root.laneW + root.laneW / 2
                                            ctx.strokeStyle = Theme.graphLane[color % Theme.graphLane.length]
                                            ctx.beginPath()
                                            ctx.moveTo(x, 0)
                                            ctx.lineTo(x, height)
                                            ctx.stroke()
                                        }
                                    }
                                    Connections {
                                        target: graphModel
                                        function onStatsChanged() { tailCanvas.requestPaint() }
                                    }
                                }
                                Label {
                                    x: root.labelColW + Theme.spaceSm + page.graphAreaW + Theme.spaceSm
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: qsTr("Showing the first %L1 commits — older history is not loaded")
                                          .arg(graphModel.rowTotal)
                                    color: Theme.warning
                                    font.pixelSize: Theme.fontMd
                                    font.weight: Font.DemiBold
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
                        // Middle-click toggles autoscroll mode: the pointer
                        // distance from the anchor sets the speed; any click
                        // exits.
                        property bool autoScrolling: false
                        property real autoAnchorX: 0
                        property real autoAnchorY: 0
                        property real autoCurrentY: 0
                        id: graphArea
                        MouseArea {
                            anchors.fill: parent
                            acceptedButtons: Qt.MiddleButton
                            onClicked: mouse => {
                                graphArea.autoAnchorX = mouse.x
                                graphArea.autoAnchorY = mouse.y
                                graphArea.autoCurrentY = mouse.y
                                graphArea.autoScrolling = true
                            }
                        }
                        MouseArea {
                            visible: graphArea.autoScrolling
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.AllButtons
                            cursorShape: Qt.SizeVerCursor
                            onPositionChanged: mouse => graphArea.autoCurrentY = mouse.y
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
                                                * root.middleScrollGain
                                    const maxY = Math.max(0, graphList.contentHeight - graphList.height)
                                    graphList.contentY = Math.max(0, Math.min(graphList.contentY + delta, maxY))
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

                    // -- diff --
                    Rectangle {
                        color: Theme.bgSurface
                        ColumnLayout {
                            anchors.fill: parent
                            spacing: 0
                            Rectangle {
                                Layout.fillWidth: true
                                implicitHeight: Theme.headerHeight
                                color: Theme.bgElevated
                                RowLayout {
                                    anchors.fill: parent
                                    anchors.leftMargin: Theme.spaceSm
                                    anchors.rightMargin: Theme.spaceSm
                                    spacing: Theme.spaceSm
                                    Label {
                                        text: qsTr("DIFF · %1").arg(diffModel.title)
                                        font.pixelSize: Theme.fontSm
                                        font.weight: Font.DemiBold
                                        color: Theme.textSecondary
                                        elide: Text.ElideMiddle
                                        Layout.fillWidth: true
                                    }
                                    ToolButton {
                                        text: "×"
                                        implicitWidth: Theme.iconLg
                                        implicitHeight: Theme.iconLg
                                        padding: 0
                                        onClicked: page.closeDiff()
                                    }
                                }
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

                // Right side: commit details
                Rectangle {
                    SplitView.preferredWidth: 400
                    SplitView.minimumWidth: 300
                    color: Theme.bgSurface
                    ColumnLayout {
                        anchors.fill: parent
                        spacing: 0
                        PaneHeader { text: qsTr("COMMIT") }
                        ColumnLayout {
                            Layout.fillWidth: true
                            Layout.margins: Theme.spaceSm
                            spacing: Theme.spaceXs
                            visible: detailsModel.shaHex !== ""

                            // -- message first, like the commit editor:
                            // a prominent summary box and a dimmer
                            // description box (Phase 2 makes these editable
                            // for new commits and amend) --
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: subjectArea.implicitHeight + Theme.spaceSm
                                color: Theme.bgBase
                                radius: Theme.radiusMd
                                border.color: Theme.borderDefault
                                border.width: Theme.borderWidth
                                TextArea {
                                    id: subjectArea
                                    anchors.fill: parent
                                    anchors.margins: Theme.spaceXs
                                    readOnly: true
                                    wrapMode: TextArea.Wrap
                                    text: detailsModel.messageSubject
                                    font.pixelSize: Theme.fontLg
                                    font.weight: Font.DemiBold
                                    color: Theme.textPrimary
                                    background: null
                                    padding: 0
                                }
                            }
                            Rectangle {
                                visible: detailsModel.messageBody !== ""
                                Layout.fillWidth: true
                                Layout.preferredHeight: Math.min(bodyArea.implicitHeight + Theme.spaceSm, 120)
                                color: Theme.bgBase
                                radius: Theme.radiusMd
                                border.color: Theme.borderSubtle
                                border.width: Theme.borderWidth
                                ScrollView {
                                    anchors.fill: parent
                                    anchors.margins: Theme.spaceXs
                                    TextArea {
                                        id: bodyArea
                                        readOnly: true
                                        wrapMode: TextArea.Wrap
                                        text: detailsModel.messageBody
                                        font.pixelSize: Theme.fontMd
                                        color: Theme.textSecondary
                                        background: null
                                        padding: 0
                                    }
                                }
                            }
                            // -- author card: avatar + name/date on the
                            // left, own hash over parent hash on the right
                            // (rows aligned) --
                            RowLayout {
                                spacing: Theme.spaceSm
                                IdentIcon {
                                    code: detailsModel.avatar
                                    width: root.detailsAvatar
                                    height: root.detailsAvatar
                                }
                                ColumnLayout {
                                    spacing: 0
                                    Layout.fillWidth: true
                                    Label {
                                        id: authorLabel
                                        text: detailsModel.authorName
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                        color: Theme.textPrimary
                                        font.pixelSize: Theme.fontMd
                                        font.weight: Font.DemiBold
                                        ToolTip.visible: authorHover.containsMouse
                                        ToolTip.delay: 400
                                        ToolTip.text: qsTr("Author: %1 <%2>\nCommitter: %3")
                                                      .arg(detailsModel.authorName)
                                                      .arg(detailsModel.authorEmail)
                                                      .arg(detailsModel.committer)
                                        MouseArea {
                                            id: authorHover
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            acceptedButtons: Qt.NoButton
                                        }
                                    }
                                    Label {
                                        id: detailsDate
                                        text: Qt.formatDateTime(new Date(detailsModel.authorTime * 1000),
                                                                "yyyy-MM-dd HH:mm")
                                        color: Theme.textSecondary
                                        font.pixelSize: Theme.fontSm
                                    }
                                }
                                ColumnLayout {
                                    spacing: 0
                                    Layout.alignment: Qt.AlignRight
                                    RowLayout {
                                        spacing: Theme.spaceXs
                                        Layout.alignment: Qt.AlignRight
                                        Label {
                                            text: detailsModel.sha8
                                            font.family: Theme.monoFamily
                                            color: Theme.textPrimary
                                            font.pixelSize: Theme.fontMd
                                        }
                                        ToolButton {
                                            text: "⧉"
                                            padding: 0
                                            implicitWidth: Theme.iconMd
                                            implicitHeight: Theme.iconMd
                                            ToolTip.visible: hovered
                                            ToolTip.delay: 600
                                            ToolTip.text: qsTr("Copy full hash")
                                            onClicked: root.copyText(detailsModel.shaHex)
                                        }
                                    }
                                    Label {
                                        id: parentLink
                                        visible: detailsModel.parentHex !== ""
                                        Layout.alignment: Qt.AlignRight
                                        text: "← " + detailsModel.parentHex.substring(0, 8)
                                        font.family: Theme.monoFamily
                                        color: Theme.textLink
                                        font.pixelSize: Theme.fontSm
                                        font.underline: parentHover.containsMouse
                                        ToolTip.visible: parentHover.containsMouse
                                        ToolTip.delay: 600
                                        ToolTip.text: qsTr("Go to parent commit")
                                        MouseArea {
                                            id: parentHover
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            cursorShape: Qt.PointingHandCursor
                                            onClicked: page.jumpToRef(detailsModel.parentHex)
                                        }
                                    }
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
                        PaneHeader {
                            visible: detailsModel.shaHex !== ""
                            text: qsTr("CHANGES (%1)").arg(fileList.count)
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
                                    page.toggleDiff("commit", path, origPath)
                            }
                        }
                    }
                }
            }
        }

        function jumpToRef(oidHex) {
            const row = graphModel.rowOf(oidHex)
            if (row >= 0) {
                graphList.currentIndex = row
                graphList.positionViewAtIndex(row, ListView.Center)
            }
            // Details resolve even outside the window.
            page.selectedOid = oidHex
            detailsModel.request(oidHex)
            page.closeDiff()
        }
    }

    // ======================================================================
    // Commit-graph row: [branch/tag chips][lanes + identicon node][subject]
    // — fixed-width label and graph columns keep subjects aligned.
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
        // Chip records are separated by U+001F (see encode.rs). Branch-like
        // records (HEAD / local / remote) and tags get separate chips.
        readonly property var labelRecords: labels === "" ? [] : labels.split(String.fromCharCode(31))
        readonly property var branchRecords: labelRecords.filter(r => r[0] !== "T")
        readonly property var tagRecords: labelRecords.filter(r => r[0] === "T")

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
        onAvatarChanged: laneCanvas.requestPaint()

        RowLayout {
            anchors.fill: parent
            spacing: Theme.spaceSm

            // Branch / tag chips, right-aligned against the graph.
            Item {
                Layout.preferredWidth: root.labelColW
                Layout.fillHeight: true
                Row {
                    anchors.right: parent.right
                    anchors.rightMargin: Theme.spaceXs
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.spaceXs
                    RefChip {
                        records: rowItem.branchRecords
                        tagStyle: false
                        maxWidth: rowItem.tagRecords.length > 0
                                  ? (root.labelColW - Theme.spaceSm) / 2
                                  : root.labelColW - Theme.spaceSm
                    }
                    RefChip {
                        records: rowItem.tagRecords
                        tagStyle: true
                        maxWidth: rowItem.branchRecords.length > 0
                                  ? (root.labelColW - Theme.spaceSm) / 2
                                  : root.labelColW - Theme.spaceSm
                    }
                }
                MouseArea {
                    id: labelHover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }
                ToolTip.visible: labelHover.containsMouse && rowItem.labelRecords.length > 0
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
                    const cx = function (l) { return root.laneInset + l * root.laneW + root.laneW / 2 }
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
                    // The commit node is the author's identicon (5x5,
                    // mirrored; local substitute for network avatars). The
                    // pattern uses only the inner part of the circle so the
                    // clip cuts less of it.
                    const r = root.nodeIcon / 2
                    ctx.save()
                    ctx.beginPath()
                    ctx.arc(nodeX, midY, r, 0, 2 * Math.PI)
                    ctx.clip()
                    ctx.fillStyle = Theme.bgElevated
                    ctx.fillRect(nodeX - r, midY - r, 2 * r, 2 * r)
                    ctx.fillStyle = Theme.graphLane[(rowItem.avatar >> 15) & 0x7]
                    const inner = 2 * r * root.identiconFill
                    const cell = inner / 5
                    const ox = nodeX - inner / 2
                    const oy = midY - inner / 2
                    for (let row = 0; row < 5; row++) {
                        for (let col = 0; col < 3; col++) {
                            if ((rowItem.avatar >> (row * 3 + col)) & 1) {
                                ctx.fillRect(ox + col * cell, oy + row * cell,
                                             cell + 0.5, cell + 0.5)
                                if (col < 2)
                                    ctx.fillRect(ox + (4 - col) * cell, oy + row * cell,
                                                 cell + 0.5, cell + 0.5)
                            }
                        }
                    }
                    ctx.restore()
                    ctx.strokeStyle = Theme.borderStrong
                    ctx.lineWidth = Theme.borderWidth
                    ctx.beginPath()
                    ctx.arc(nodeX, midY, r, 0, 2 * Math.PI)
                    ctx.stroke()
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

    // One aggregated chip: primary name + "+N". Branch chips are outlined
    // (⎇ / ☁ / HEAD, blue), tag chips are filled (⚑, amber) so the two
    // kinds read differently at a glance.
    component RefChip: Rectangle {
        id: chip
        property var records: []
        property bool tagStyle: false
        property real maxWidth: 140

        visible: records.length > 0
        height: Theme.fontSmLine
        width: Math.min(chipContent.implicitWidth + 2 * Theme.spaceXs, maxWidth)
        radius: Theme.radiusSm
        clip: true

        readonly property string rec: records.length > 0 ? records[0] : "L00"
        readonly property string recKind: rec[0]
        readonly property bool recHead: rec[1] === "1"
        readonly property color chipColor: tagStyle ? Theme.warning
                                          : recKind === "R" ? Theme.textSecondary
                                          : recKind === "H" ? Theme.danger
                                          : Theme.accent

        color: tagStyle ? Theme.bgElevated : "transparent"
        border.color: chipColor
        border.width: Theme.borderWidth

        Row {
            id: chipContent
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: parent.left
            anchors.leftMargin: Theme.spaceXs
            spacing: Theme.spaceXs
            NavIcon {
                anchors.verticalCenter: parent.verticalCenter
                kind: chip.tagStyle ? "tag" : chip.recKind === "R" ? "remote" : "branch"
                tint: chip.chipColor
                width: Theme.iconSm
                height: Theme.iconSm
            }
            Label {
                text: chip.rec.substring(3)
                color: chip.chipColor
                font.pixelSize: Theme.fontSm
                font.weight: chip.recHead ? Font.DemiBold : Font.Normal
                elide: Text.ElideRight
                width: Math.min(implicitWidth,
                                chip.maxWidth - Theme.spaceLg
                                - (chip.records.length > 1 ? Theme.spaceLg : 0))
            }
            Label {
                visible: chip.records.length > 1
                text: "+" + (chip.records.length - 1)
                color: chip.chipColor
                font.pixelSize: Theme.fontSm
            }
        }
    }

    // ======================================================================
    // Sidebar building blocks
    // ======================================================================
    component NavHeader: Rectangle {
        id: header
        property string caption
        property string iconKind: "branch"
        property color iconTint: Theme.textSecondary
        property int count: 0
        property bool expanded: true
        property bool showTagToggle: false
        property bool tagsShown: true
        signal toggled()
        signal tagsToggled(bool shown)

        Layout.fillWidth: true
        implicitHeight: Theme.rowHeight
        color: Theme.bgElevated
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.spaceSm
            anchors.rightMargin: Theme.spaceXs
            spacing: Theme.spaceXs
            Label {
                text: header.expanded ? "▾" : "▸"
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
            NavIcon {
                kind: header.iconKind
                tint: header.iconTint
                width: Theme.iconSm + 2
                height: Theme.iconSm + 2
            }
            Label {
                text: header.caption
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
            }
            Label {
                text: "(" + header.count + ")"
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
            }
            Item { Layout.fillWidth: true }
            ToolButton {
                visible: header.showTagToggle
                checkable: true
                checked: header.tagsShown
                text: "⚑"
                opacity: checked ? 1.0 : 0.35
                padding: 0
                implicitWidth: Theme.iconLg
                implicitHeight: Theme.iconLg
                ToolTip.visible: hovered
                ToolTip.delay: 600
                ToolTip.text: qsTr("Show tags in the graph")
                onToggled: header.tagsToggled(checked)
            }
        }
        MouseArea {
            anchors.fill: parent
            // Leave the toggle button clickable.
            anchors.rightMargin: header.showTagToggle ? Theme.spaceXl : 0
            onClicked: header.toggled()
        }
    }

    component NavList: ListView {
        id: navList
        property var sectionModel
        property bool expanded: true
        property string kindHint: "branch"
        signal refActivated(string oidHex)
        signal fileActivated(string bucket, string path, string origPath)

        visible: expanded
        Layout.fillWidth: true
        Layout.fillHeight: expanded
        Layout.maximumHeight: expanded ? count * Theme.rowHeight + Theme.spaceXs : 0
        clip: true
        model: sectionModel
        reuseItems: true
        ScrollBar.vertical: ScrollBar {}
        delegate: NavItemDelegate {
            listWidth: navList.width
            kindHint: navList.kindHint
            onRefClicked: oidHex => navList.refActivated(oidHex)
            onFileClicked: (bucket, path, origPath) => navList.fileActivated(bucket, path, origPath)
            onFolderClicked: key => navList.sectionModel.toggleFolder(key)
        }
    }

    component NavItemDelegate: Item {
        id: navRow
        required property int index
        required property string name
        required property string full
        required property string oid_hex
        required property string change
        required property string bucket
        required property string orig_path
        required property bool is_head
        required property bool has_remote
        required property int depth
        required property bool folder
        required property bool collapsed
        property string kindHint: "branch"
        property real listWidth: 200

        signal refClicked(string oidHex)
        signal fileClicked(string bucket, string path, string origPath)
        signal folderClicked(string key)

        width: listWidth
        height: Theme.rowHeight

        Rectangle {
            anchors.fill: parent
            color: Theme.bgHover
            visible: itemMouse.containsMouse
        }
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.spaceMd + navRow.depth * Theme.spaceMd
            anchors.rightMargin: Theme.spaceSm
            spacing: Theme.spaceXs
            Label {
                visible: navRow.folder
                text: navRow.collapsed ? "▸" : "▾"
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
            ChangeIcon {
                visible: !navRow.folder && navRow.kindHint === "wt"
                change: navRow.change
                width: Theme.iconSm + 2
                height: Theme.iconSm + 2
                ToolTip.visible: wtHover.containsMouse
                ToolTip.delay: 600
                ToolTip.text: {
                    const c = navRow.change.length > 0 ? navRow.change[0] : ""
                    const what = navRow.change.length === 2 ? qsTr("Conflicted")
                               : c === "M" ? qsTr("Modified")
                               : c === "A" ? qsTr("Added")
                               : c === "D" ? qsTr("Deleted")
                               : c === "R" ? qsTr("Renamed")
                               : c === "C" ? qsTr("Copied")
                               : c === "T" ? qsTr("Type changed")
                               : c === "?" ? qsTr("Untracked") : navRow.change
                    const where = navRow.bucket === "staged" ? qsTr("staged")
                                : navRow.bucket === "unstaged" ? qsTr("unstaged")
                                : navRow.bucket === "untracked" ? qsTr("untracked")
                                : qsTr("conflict")
                    return what + " · " + where
                }
                MouseArea {
                    id: wtHover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }
            }
            Label {
                Layout.fillWidth: true
                text: navRow.name
                elide: Text.ElideMiddle
                font.weight: navRow.is_head ? Font.DemiBold : Font.Normal
                color: navRow.folder ? Theme.textSecondary
                       : navRow.is_head ? Theme.textLink : Theme.textPrimary
                font.pixelSize: Theme.fontMd
            }
            // Branch state badge: filled = has remote, hollow = local only
            // (PR state joins in Phase 4 as a third look).
            Rectangle {
                visible: !navRow.folder && navRow.kindHint === "branch"
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
                if (navRow.folder)
                    navRow.folderClicked(navRow.full)
                else if (navRow.kindHint === "wt")
                    navRow.fileClicked(navRow.bucket, navRow.name, navRow.orig_path)
                else if (navRow.oid_hex !== "")
                    navRow.refClicked(navRow.oid_hex)
            }
        }
        // Nested leaves show only their last segment; hover reveals the
        // full name.
        ToolTip.visible: itemMouse.containsMouse && !navRow.folder
                         && navRow.full !== "" && navRow.full !== navRow.name
        ToolTip.delay: 700
        ToolTip.text: navRow.full
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
            ChangeIcon {
                change: fileRow.changeText
                width: Theme.iconSm + 2
                height: Theme.iconSm + 2
                ToolTip.visible: changeHover.containsMouse
                ToolTip.delay: 600
                ToolTip.text: {
                    const c = fileRow.changeText.length > 0 ? fileRow.changeText[0] : ""
                    return c === "M" ? qsTr("Modified")
                         : c === "A" ? qsTr("Added")
                         : c === "D" ? qsTr("Deleted")
                         : c === "R" ? qsTr("Renamed")
                         : c === "C" ? qsTr("Copied")
                         : c === "T" ? qsTr("Type changed") : fileRow.changeText
                }
                MouseArea {
                    id: changeHover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }
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

    // Hand-drawn 16px-grid icons in the common git-client style (branch
    // fork, cloud remote, price-tag, archive box, tree, clock). Scaled by
    // the item size; single stroke color.
    component NavIcon: Canvas {
        id: icon
        property string kind: "branch"
        property color tint: Theme.textSecondary
        width: Theme.iconMd
        height: Theme.iconMd
        onKindChanged: requestPaint()
        onTintChanged: requestPaint()
        onPaint: {
            const ctx = getContext("2d")
            const s = width / 16
            ctx.clearRect(0, 0, width, height)
            ctx.strokeStyle = icon.tint
            ctx.fillStyle = icon.tint
            ctx.lineWidth = root.iconStroke
            ctx.lineCap = "round"
            if (icon.kind === "branch") {
                ctx.beginPath()
                ctx.moveTo(5 * s, 5 * s)
                ctx.lineTo(5 * s, 11 * s)
                ctx.stroke()
                ctx.beginPath()
                ctx.moveTo(11 * s, 7 * s)
                ctx.bezierCurveTo(11 * s, 9.5 * s, 8 * s, 9.5 * s, 5.8 * s, 10.2 * s)
                ctx.stroke()
                for (const c of [[5, 3.5], [5, 12.5], [11, 5]]) {
                    ctx.beginPath()
                    ctx.arc(c[0] * s, c[1] * s, 1.8 * s, 0, 2 * Math.PI)
                    ctx.stroke()
                }
            } else if (icon.kind === "remote") {
                ctx.beginPath()
                ctx.arc(6 * s, 9 * s, 3 * s, Math.PI * 0.5, Math.PI * 1.5)
                ctx.arc(8.5 * s, 6.8 * s, 3.2 * s, Math.PI * 0.95, Math.PI * 0.02, false)
                ctx.arc(11 * s, 9.4 * s, 2.6 * s, Math.PI * 1.55, Math.PI * 0.5)
                ctx.closePath()
                ctx.stroke()
            } else if (icon.kind === "tag") {
                ctx.save()
                ctx.translate(8 * s, 8.5 * s)
                ctx.rotate(Math.PI / 4)
                ctx.strokeRect(-3.6 * s, -3.6 * s, 7.2 * s, 7.2 * s)
                ctx.beginPath()
                ctx.arc(-1.4 * s, -1.4 * s, 1 * s, 0, 2 * Math.PI)
                ctx.fill()
                ctx.restore()
            } else if (icon.kind === "stash") {
                ctx.strokeRect(3 * s, 4 * s, 10 * s, 3 * s)
                ctx.strokeRect(4 * s, 7 * s, 8 * s, 6 * s)
                ctx.beginPath()
                ctx.moveTo(6.5 * s, 9.5 * s)
                ctx.lineTo(9.5 * s, 9.5 * s)
                ctx.stroke()
            } else if (icon.kind === "tree") {
                ctx.beginPath()
                ctx.arc(8 * s, 6.5 * s, 4 * s, 0, 2 * Math.PI)
                ctx.stroke()
                ctx.beginPath()
                ctx.moveTo(8 * s, 10.5 * s)
                ctx.lineTo(8 * s, 14 * s)
                ctx.stroke()
            } else if (icon.kind === "pen") {
                ctx.save()
                ctx.translate(8 * s, 8 * s)
                ctx.rotate(Math.PI / 4)
                ctx.strokeRect(-1.4 * s, -6 * s, 2.8 * s, 8.5 * s)
                ctx.beginPath()
                ctx.moveTo(-1.4 * s, 2.5 * s)
                ctx.lineTo(0, 5.5 * s)
                ctx.lineTo(1.4 * s, 2.5 * s)
                ctx.closePath()
                ctx.fill()
                ctx.restore()
            } else if (icon.kind === "plus") {
                ctx.beginPath()
                ctx.moveTo(8 * s, 3.5 * s)
                ctx.lineTo(8 * s, 12.5 * s)
                ctx.moveTo(3.5 * s, 8 * s)
                ctx.lineTo(12.5 * s, 8 * s)
                ctx.stroke()
            } else if (icon.kind === "minus") {
                ctx.beginPath()
                ctx.moveTo(3.5 * s, 8 * s)
                ctx.lineTo(12.5 * s, 8 * s)
                ctx.stroke()
            } else if (icon.kind === "arrow") {
                ctx.beginPath()
                ctx.moveTo(3.5 * s, 8 * s)
                ctx.lineTo(11.5 * s, 8 * s)
                ctx.moveTo(11.5 * s, 8 * s)
                ctx.lineTo(8.8 * s, 5.2 * s)
                ctx.moveTo(11.5 * s, 8 * s)
                ctx.lineTo(8.8 * s, 10.8 * s)
                ctx.stroke()
            } else if (icon.kind === "copyicon") {
                ctx.strokeRect(5.5 * s, 3.5 * s, 7 * s, 7 * s)
                ctx.strokeRect(3.5 * s, 5.5 * s, 7 * s, 7 * s)
            } else if (icon.kind === "bang") {
                ctx.beginPath()
                ctx.moveTo(8 * s, 3.5 * s)
                ctx.lineTo(8 * s, 10 * s)
                ctx.stroke()
                ctx.beginPath()
                ctx.arc(8 * s, 12.8 * s, 1 * s, 0, 2 * Math.PI)
                ctx.fill()
            } else if (icon.kind === "folder") {
                ctx.beginPath()
                ctx.moveTo(2.5 * s, 12.5 * s)
                ctx.lineTo(2.5 * s, 4.5 * s)
                ctx.lineTo(6.5 * s, 4.5 * s)
                ctx.lineTo(8 * s, 6 * s)
                ctx.lineTo(13.5 * s, 6 * s)
                ctx.lineTo(13.5 * s, 12.5 * s)
                ctx.closePath()
                ctx.stroke()
            } else if (icon.kind === "clock") {
                ctx.beginPath()
                ctx.arc(8 * s, 8 * s, 5.5 * s, 0, 2 * Math.PI)
                ctx.stroke()
                ctx.beginPath()
                ctx.moveTo(8 * s, 8 * s)
                ctx.lineTo(8 * s, 4.8 * s)
                ctx.moveTo(8 * s, 8 * s)
                ctx.lineTo(10.4 * s, 8 * s)
                ctx.stroke()
            }
        }
    }

    // Change-kind icon (pen = edit, + / − = add / delete, → = rename,
    // stacked squares = copy, ! = conflict). Edits are amber by request;
    // adds/deletes reuse the diff colors.
    component ChangeIcon: NavIcon {
        id: changeIcon
        property string change: ""
        readonly property string letter: change.length > 0 ? change[0] : ""
        readonly property bool conflict: change.length === 2
        kind: conflict ? "bang"
              : letter === "A" ? "plus"
              : letter === "D" ? "minus"
              : letter === "R" ? "arrow"
              : letter === "C" ? "copyicon"
              : letter === "?" ? "plus"
              : "pen"
        tint: conflict ? Theme.danger
              : letter === "A" ? Theme.diffAddedFg
              : letter === "D" ? Theme.diffRemovedFg
              : letter === "R" ? Theme.textLink
              : letter === "C" ? Theme.textSecondary
              : letter === "?" ? Theme.statusUntracked
              : Theme.warning
    }

    // Author identicon (same packed code as the graph nodes).
    component IdentIcon: Canvas {
        id: ident
        property int code: 0
        width: Theme.iconLg
        height: Theme.iconLg
        onCodeChanged: requestPaint()
        onPaint: {
            const ctx = getContext("2d")
            const r = width / 2
            ctx.clearRect(0, 0, width, height)
            ctx.save()
            ctx.beginPath()
            ctx.arc(r, r, r, 0, 2 * Math.PI)
            ctx.clip()
            ctx.fillStyle = Theme.bgElevated
            ctx.fillRect(0, 0, width, height)
            ctx.fillStyle = Theme.graphLane[(ident.code >> 15) & 0x7]
            const inner = width * root.identiconFill
            const cell = inner / 5
            const o = (width - inner) / 2
            for (let row = 0; row < 5; row++) {
                for (let col = 0; col < 3; col++) {
                    if ((ident.code >> (row * 3 + col)) & 1) {
                        ctx.fillRect(o + col * cell, o + row * cell, cell + 0.5, cell + 0.5)
                        if (col < 2)
                            ctx.fillRect(o + (4 - col) * cell, o + row * cell, cell + 0.5, cell + 0.5)
                    }
                }
            }
            ctx.restore()
            ctx.strokeStyle = Theme.borderStrong
            ctx.lineWidth = Theme.borderWidth
            ctx.beginPath()
            ctx.arc(r, r, r - 0.5, 0, 2 * Math.PI)
            ctx.stroke()
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
