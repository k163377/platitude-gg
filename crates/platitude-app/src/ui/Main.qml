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

    // QML never drops a text input's focus on its own: once the sidebar
    // filter or the commit editor was clicked, its caret kept blinking
    // until some other editor took focus. This passive watcher (it grabs
    // nothing, so every control underneath keeps working) hands focus
    // back to the window whenever a press lands outside the focused
    // editor. Judged on press, not on the tap: a tap is called off when
    // any item accepts the click, and most of the window is clickable.
    // Modal dialogs sit above the content item, so their own fields are
    // unaffected.
    TapHandler {
        acceptedButtons: Qt.AllButtons
        onPressedChanged: {
            if (!pressed)
                return
            const item = root.activeFocusItem
            // Only text editors hold a caret worth releasing; list views
            // and buttons manage their own focus.
            if (!item || !("cursorPosition" in item))
                return
            const local = item.mapFromItem(null, point.scenePressPosition)
            if (local.x < 0 || local.y < 0
                    || local.x >= item.width || local.y >= item.height)
                root.contentItem.forceActiveFocus()
        }
    }

    // Identity dialog: opens on startup when git has no name and email to
    // put on a commit, and on demand from the app menu or the toolbar
    // badge. "Not now" leaves the app fully usable — reading a repository
    // needs no identity.
    property bool identityDismissed: false
    property bool identityEditing: false
    readonly property bool identityWanted: AppBackend.gitState === "ok"
                                           && (identityEditing
                                               || (AppBackend.identityState === "missing"
                                                   && !identityDismissed))
    function dismissIdentity() {
        identityEditing = false
        identityDismissed = true
    }
    // Screenshot hook: PG_AUTO_IDENTITY="edit" opens the dialog on an
    // identity that is already set, which is otherwise a menu action.
    Connections {
        target: AppBackend
        function onIdentityChanged() {
            if (AppBackend.autoIdentity === "edit"
                    && AppBackend.identityState === "ready"
                    && !root.identityDismissed)
                root.identityEditing = true
        }
    }

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

    // The RepoPage of the active tab (the toolbar's right-side controls
    // act on it).
    readonly property var curPage: (pageRepeater.count > 0
                                    && tabsModel.currentIndex >= 0
                                    && tabsModel.currentIndex < pageRepeater.count)
                                   ? pageRepeater.itemAt(tabsModel.currentIndex) : null

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
            // Popups (the identity dialog) render in the window overlay,
            // outside this subtree: capturing those needs a window-level
            // screenshot from outside the process.
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

    // ---- identity dialog -------------------------------------------------
    // Opened and closed from the state above rather than by binding
    // `visible`: Escape closes a popup imperatively, which would overwrite
    // such a binding and leave the menu entry unable to open it again.
    // Closing for any reason answers the state, so the two stay in step.
    IdentityDialog {
        id: identityDialog
        editing: root.identityEditing
        onDismissed: root.dismissIdentity()
        Connections {
            target: root
            function onIdentityWantedChanged() {
                if (root.identityWanted)
                    identityDialog.open()
                else
                    identityDialog.close()
            }
        }
    }

    // ---- confirmation ----------------------------------------------------
    function confirm(heading, detail, acceptText, action) {
        confirmDialog.ask(heading, detail, acceptText, action)
    }
    ConfirmDialog {
        id: confirmDialog
    }

    // ---- settings --------------------------------------------------------
    SettingsDialog {
        id: settingsDialog
    }

    // ---- main ------------------------------------------------------------
    ColumnLayout {
        id: mainUi
        anchors.fill: parent
        spacing: 0
        visible: AppBackend.gitState === "ok"

        // Top toolbar: prominent tabs and the per-repository controls
        // share one row.
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.toolbarHeight
            color: Theme.bgElevated
            RowLayout {
                anchors.fill: parent
                anchors.rightMargin: Theme.spaceMd
                spacing: Theme.spaceSm
                // App menu (Claude-Desktop-style hamburger); most entries
                // are placeholders until their phases land.
                HoverToolButton {
                    id: menuButton
                    text: "☰"
                    font.pixelSize: Theme.fontLg
                    Layout.leftMargin: Theme.spaceXs
                    Layout.alignment: Qt.AlignVCenter
                    padding: 0
                    implicitWidth: Theme.spaceXl
                    implicitHeight: Theme.spaceXl
                    onClicked: appMenu.open()
                    Menu {
                        id: appMenu
                        y: menuButton.height
                        MenuItem {
                            text: qsTr("Open repository…")
                            onTriggered: folderDialog.open()
                        }
                        MenuItem {
                            text: qsTr("Clone repository…")
                            enabled: false
                        }
                        MenuSeparator {}
                        MenuItem {
                            text: qsTr("Identity…")
                            onTriggered: root.identityEditing = true
                        }
                        MenuItem {
                            text: qsTr("Settings…")
                            onTriggered: settingsDialog.open()
                        }
                        MenuItem {
                            text: qsTr("About platitude-gg")
                            enabled: false
                        }
                        MenuSeparator {}
                        MenuItem {
                            text: qsTr("Exit")
                            onTriggered: Qt.quit()
                        }
                    }
                }
                // Plain Row tabs (no TabBar): full control of the geometry
                // so the selected underline sits exactly on the toolbar's
                // bottom edge with no styling leftovers beneath it.
                Row {
                    id: tabRow
                    Layout.fillHeight: true
                    spacing: 0
                    Repeater {
                        model: tabsModel
                        Rectangle {
                            id: tabItem
                            required property int index
                            required property int tab_id
                            required property string title
                            required property string repo_path
                            readonly property bool current: tabsModel.currentIndex === index
                            width: tabContent.implicitWidth + 2 * Theme.spaceSm
                            height: tabRow.height
                            color: current ? Theme.bgSelected : "transparent"
                            MouseArea {
                                id: tabMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                onClicked: tabsModel.setCurrentIndex(tabItem.index)
                            }
                            Rectangle {
                                anchors.fill: parent
                                color: Theme.bgHover
                                visible: tabMouse.containsMouse && !tabItem.current
                            }
                            Rectangle {
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.bottom: parent.bottom
                                height: 2 * Theme.borderWidth
                                color: Theme.accent
                                visible: tabItem.current
                            }
                            RowLayout {
                                id: tabContent
                                anchors.fill: parent
                                anchors.leftMargin: Theme.spaceSm
                                anchors.rightMargin: Theme.spaceXs
                                spacing: Theme.spaceXs
                                Label {
                                    text: tabItem.title
                                    elide: Text.ElideRight
                                    Layout.maximumWidth: 180
                                    Layout.fillHeight: true
                                    verticalAlignment: Text.AlignVCenter
                                    font.weight: tabItem.current ? Font.DemiBold : Font.Normal
                                    color: tabItem.current ? Theme.textPrimary
                                                           : Theme.textSecondary
                                }
                                HoverToolButton {
                                    text: "×"
                                    padding: 0
                                    Layout.alignment: Qt.AlignVCenter
                                    implicitWidth: Theme.iconLg
                                    implicitHeight: Theme.iconLg
                                    onClicked: tabsModel.closeTab(tabItem.tab_id)
                                }
                            }
                        }
                    }
                }
                HoverToolButton {
                    text: "+"
                    font.pixelSize: Theme.fontLg
                    onClicked: folderDialog.open()
                }
                Item { Layout.fillWidth: true }

                // Transient state of the current repository.
                Rectangle {
                    visible: root.curPage !== null && root.curPage.pageWt.opText !== ""
                    color: "transparent"
                    border.color: Theme.warning
                    border.width: Theme.borderWidth
                    radius: Theme.radiusSm
                    implicitHeight: Theme.iconLg
                    implicitWidth: opLabel.implicitWidth + 2 * Theme.spaceXs
                    Label {
                        id: opLabel
                        anchors.centerIn: parent
                        text: root.curPage !== null ? root.curPage.pageWt.opText : ""
                        color: Theme.warning
                        font.pixelSize: Theme.fontSm
                        font.weight: Font.DemiBold
                    }
                }
                Rectangle {
                    visible: root.curPage !== null && root.curPage.pageWt.hasConflicts
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
                // Nothing to attribute commits to. Kept next to the other
                // repository-state badges so the way back to the setup
                // screen stays visible after "Not now".
                Rectangle {
                    visible: AppBackend.identityState === "missing"
                             || (root.curPage !== null
                                 && !root.curPage.pageTab.identityReady)
                    color: "transparent"
                    border.color: Theme.warning
                    border.width: Theme.borderWidth
                    radius: Theme.radiusSm
                    implicitHeight: Theme.iconLg
                    implicitWidth: identityBadge.implicitWidth + 2 * Theme.spaceXs
                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.radiusSm
                        color: Theme.bgHover
                        visible: identityBadgeMouse.containsMouse
                    }
                    Label {
                        id: identityBadge
                        anchors.centerIn: parent
                        text: qsTr("SET IDENTITY")
                        color: Theme.warning
                        font.pixelSize: Theme.fontSm
                        font.weight: Font.DemiBold
                    }
                    MouseArea {
                        id: identityBadgeMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: root.identityEditing = true
                        ToolTip.visible: containsMouse
                        ToolTip.delay: 600
                        ToolTip.text: qsTr("git has no name or email to record on commits")
                    }
                }
                Label {
                    visible: root.curPage !== null && root.curPage.pageTab.lastError !== ""
                    text: root.curPage !== null ? root.curPage.pageTab.lastError : ""
                    color: Theme.danger
                    elide: Text.ElideRight
                    Layout.maximumWidth: 320
                    font.pixelSize: Theme.fontSm
                    background: Rectangle {
                        color: Theme.bgHover
                        visible: errorClearMouse.containsMouse
                    }
                    MouseArea {
                        id: errorClearMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: if (root.curPage !== null) root.curPage.pageTab.clearLastError()
                    }
                }
                // Auto fetch: quiet by design. A machine that is simply
                // offline fails here once a minute, and that belongs in a
                // tooltip rather than in the error line.
                Label {
                    readonly property bool failing: root.curPage !== null
                                                    && root.curPage.pageTab.autoFetchError !== ""
                    text: "↻"
                    color: root.curPage !== null && root.curPage.pageTab.autoFetchRunning
                           ? Theme.accent
                           : failing ? Theme.warning
                           : AppBackend.autoFetchMinutes > 0 ? Theme.textMuted
                           : Theme.borderDefault
                    font.pixelSize: Theme.fontMd
                    background: Rectangle {
                        radius: Theme.radiusSm
                        color: Theme.bgHover
                        visible: fetchIndicatorMouse.containsMouse
                    }
                    MouseArea {
                        id: fetchIndicatorMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: settingsDialog.open()
                        ToolTip.visible: containsMouse
                        ToolTip.delay: 600
                        ToolTip.text: parent.failing
                                      ? root.curPage.pageTab.autoFetchError
                                      : AppBackend.autoFetchMinutes > 0
                                        ? qsTr("Fetching every %n minute(s)", "",
                                               AppBackend.autoFetchMinutes)
                                        : qsTr("Automatic fetching is off")
                    }
                }
                // Reserved: search box (backlog)
                SlimField {
                    enabled: false
                    opacity: 0.35
                    placeholderText: qsTr("Search")
                    implicitWidth: 160
                }
                ActionButton {
                    kind: "fetch"
                    text: qsTr("Fetch")
                    enabled: root.curPage !== null
                             && root.curPage.pageTab.remoteCount > 0
                    ToolTip.visible: hovered
                    ToolTip.delay: 600
                    ToolTip.text: qsTr("Fetch every remote now, pruning branches "
                                       + "they no longer have")
                    onClicked: root.curPage.pageTab.fetch("")
                }
                ActionButton {
                    kind: "push"
                    text: qsTr("Push")
                    enabled: root.curPage !== null && root.curPage.canPush
                    ToolTip.visible: hovered
                    ToolTip.delay: 600
                    ToolTip.text: root.curPage === null ? ""
                                  : root.curPage.pageWt.upstream !== ""
                                    ? qsTr("Push this branch to %1")
                                      .arg(root.curPage.pageWt.upstream)
                                    : qsTr("Publish this branch as %1")
                                      .arg(root.curPage.pushTargetLabel)
                    onClicked: root.curPage.pushNow()
                    // Overwriting a remote's history is the one push that
                    // needs asking about, so it lives behind its own entry.
                    MouseArea {
                        anchors.fill: parent
                        acceptedButtons: Qt.RightButton
                        onClicked: pushMenu.popup()
                    }
                    Menu {
                        id: pushMenu
                        MenuItem {
                            text: qsTr("Force push…")
                            enabled: root.curPage !== null && root.curPage.canPush
                            onTriggered: root.curPage.forcePushNow()
                        }
                    }
                }
                // Local re-read only (no network).
                HoverToolButton {
                    text: qsTr("Reload")
                    enabled: root.curPage !== null
                    ToolTip.visible: hovered
                    ToolTip.delay: 600
                    ToolTip.text: qsTr("Re-read this repository from disk (does not fetch)")
                    onClicked: root.curPage.pageTab.refreshAll()
                }
            }
        }

        // Divider under the tab toolbar — same look as the pane splitters.
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.splitterWidth
            color: Theme.borderSubtle
        }

        // Nothing open: the window keeps its usual three-pane shape with
        // every pane empty, and the way in sits where the graph goes.
        // Built only while it is needed, so an app that starts with tabs
        // never pays for it.
        Loader {
            Layout.fillWidth: true
            Layout.fillHeight: true
            active: tabsModel.currentIndex < 0
            visible: active
            sourceComponent: Component {
                RepoPage {
                    index: -1
                    tab_id: -1
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
                id: pageRepeater
                model: tabsModel
                RepoPage {}
            }
        }

        // Bottom edge: the same splitter-style divider closes the window;
        // the git version floats above it bottom-right as faint bare text.
        Item {
            Layout.fillWidth: true
            implicitHeight: Theme.splitterWidth
            Rectangle {
                anchors.fill: parent
                color: Theme.borderSubtle
            }
            Label {
                visible: AppBackend.gitVersion !== ""
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                anchors.rightMargin: Theme.spaceSm
                anchors.bottomMargin: Theme.splitterWidth + Theme.spaceXs
                text: qsTr("git %1").arg(AppBackend.gitVersion)
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
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
        // No repository behind this page (tab_id -1): the chrome renders
        // with empty models and the graph column offers the way in.
        readonly property bool blank: tab_id < 0

        property string selectedOid: ""

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

        // Right pane switches to the working-tree (WIP) view.
        property bool wipShown: false
        // Selected stash row's reflog selector ("" = not a stash).
        property string selectedStashRef: ""
        function showWip() {
            page.wipShown = true
            page.selectedOid = ""
            page.selectedStashRef = ""
            page.closeDiff()
            page.refreshHeadPublished()
        }

        // ---- commit editor -------------------------------------------
        // `amending` mirrors the checkbox so the page can act on it
        // without reaching into the delegate tree.
        property bool amending: false
        // Whether HEAD is already on a remote. Amending it rewrites
        // something other people may have, so that gets confirmed.
        property bool headPublished: false
        readonly property string headRange: "HEAD^!"
        function refreshHeadPublished() {
            if (repoTab.state === "open")
                repoTab.checkPublish(page.headRange)
        }
        Connections {
            target: repoTab
            function onChanged() {
                // One shared answer slot, so each consumer only reads the
                // reply to the range it asked about.
                if (repoTab.publishRange === page.headRange) {
                    page.headPublished = repoTab.publishPublished > 0
                } else if (page.menuOid !== ""
                           && repoTab.publishRange === page.menuOid + "^!") {
                    page.menuPublished = repoTab.publishPublished > 0
                    page.menuPublishKnown = true
                }
                page.absorbHeadMessage()
                page.absorbWriteResult()
            }
        }

        // Turning amend on starts the editor from HEAD's message; turning
        // it off empties it again, since the text belonged to that commit.
        property int seenHeadMessageSeq: 0
        property bool wantHeadMessage: false
        function amendToggled(on) {
            page.amending = on
            if (on) {
                page.wantHeadMessage = true
                repoTab.requestHeadMessage()
            } else {
                page.clearCommitEditor()
            }
        }
        function absorbHeadMessage() {
            if (repoTab.headMessageSeq === page.seenHeadMessageSeq)
                return
            page.seenHeadMessageSeq = repoTab.headMessageSeq
            if (!page.wantHeadMessage)
                return
            page.wantHeadMessage = false
            wipSubject.text = repoTab.headSubject
            wipBody.text = repoTab.headBody
        }
        function clearCommitEditor() {
            wipSubject.text = ""
            wipBody.text = ""
        }

        function commitNow() {
            if (page.amending && page.headPublished) {
                root.confirm(
                    qsTr("Rewrite a commit that is already on a remote?"),
                    qsTr("The last commit has been pushed. Amending replaces it "
                         + "with a different one, so anyone who already has it "
                         + "will be out of step until they reset."),
                    qsTr("Amend anyway"), page.doCommit)
                return
            }
            page.doCommit()
        }
        function doCommit() {
            repoTab.commit(wipSubject.text, wipBody.text, page.amending)
        }

        // ---- moving between branches and commits ----------------------
        // Terminology is deliberate: git runs `switch` / `restore`, and
        // the UI says "Switch to" (デザイン規約 §用語).
        //
        // A move with uncommitted changes asks what to do with them
        // instead of silently carrying them along: "leave them here"
        // stashes first (the default — arriving on another branch with
        // unexplained changes is how accidents start), "bring them along"
        // is git's own behaviour.
        readonly property bool treeDirty: workTree.stagedCount > 0
                                          || workTree.unstagedCount > 0
                                          || workTree.untrackedCount > 0
        // What the pending move is: kind is "branch" / "remote" / "commit".
        property string moveKind: ""
        property string moveTarget: ""
        property string moveLabel: ""

        function switchTo(kind, target, label) {
            page.moveKind = kind
            page.moveTarget = target
            page.moveLabel = label
            if (page.treeDirty)
                dirtySwitchDialog.open()
            else
                page.runSwitch(false)
        }
        function runSwitch(stashFirst) {
            if (page.moveKind === "branch")
                repoTab.checkoutBranch(page.moveTarget, stashFirst)
            else if (page.moveKind === "remote")
                repoTab.checkoutRemote(page.moveTarget,
                                       repoTab.localNameFor(page.moveTarget), stashFirst)
            else if (page.moveKind === "commit")
                repoTab.checkoutDetached(page.moveTarget, stashFirst)
            page.moveKind = ""
        }

        DirtySwitchDialog {
            id: dirtySwitchDialog
            moveLabel: page.moveLabel
            stayLabel: workTree.detached ? qsTr("this commit") : workTree.branch
            onResolved: stashFirst => page.runSwitch(stashFirst)
        }

        // ---- push ------------------------------------------------------
        readonly property string pushTargetLabel:
            workTree.upstream !== "" ? workTree.upstream
                                     : repoTab.defaultRemote + "/" + workTree.branch
        readonly property bool canPush: repoTab.state === "open"
                                        && !workTree.detached
                                        && workTree.branch !== ""
                                        && repoTab.remoteCount > 0
                                        && repoTab.busyCount === 0
        function pushNow() {
            repoTab.pushCurrent("", "")
        }
        function forcePushNow() {
            root.confirm(
                qsTr("Overwrite %1 with this branch?").arg(page.pushTargetLabel),
                qsTr("A force push replaces the remote branch's history with "
                     + "yours. Commits only the remote has are lost, and anyone "
                     + "who already pulled them keeps a history that no longer "
                     + "matches.\n\nThe push is refused if the remote moved since "
                     + "this window last saw it."),
                qsTr("Force push"),
                // A lease pinned to the commit actually on screen: a
                // background fetch must not turn this into a plain force.
                function () { repoTab.pushCurrent("lease", page.upstreamOid()) })
        }
        /// Commit the remote-tracking branch points at, as shown here.
        function upstreamOid() {
            return workTree.upstream !== ""
                   ? remotesModel.oidOfName(workTree.upstream) : ""
        }

        // ---- context menu on a branch row ------------------------------
        property string menuRefName: ""
        property string menuRefOid: ""
        property bool menuRefRemote: false
        function openRefMenu(name, oidHex, isRemote) {
            page.menuRefName = name
            page.menuRefOid = oidHex
            page.menuRefRemote = isRemote
            refMenu.popup()
        }
        Menu {
            id: refMenu
            MenuItem {
                text: qsTr("Switch to %1").arg(page.menuRefName)
                enabled: page.menuRefName !== workTree.branch
                onTriggered: page.switchTo(page.menuRefRemote ? "remote" : "branch",
                                           page.menuRefName, page.menuRefName)
            }
            MenuSeparator {}
            MenuItem {
                text: qsTr("Copy commit hash")
                onTriggered: root.copyText(page.menuRefOid)
            }
        }

        // ---- context menu on a commit row ------------------------------
        property string menuOid: ""
        readonly property string menuShort: page.menuOid.substring(0, 8)
        function openCommitMenu(oidHex) {
            page.menuOid = oidHex
            // Asked as the menu opens so the rewrite warnings inside it
            // know whether this commit has already left the machine. The
            // rewriting entries stay disabled until the answer lands —
            // one `rev-list --count`, so within a frame or two.
            page.menuPublished = false
            page.menuPublishKnown = false
            repoTab.checkPublish(oidHex + "^!")
            commitMenu.popup()
        }
        // Whether the commit the menu is about is already on a remote.
        property bool menuPublished: false
        property bool menuPublishKnown: false

        Menu {
            id: commitMenu
            MenuItem {
                text: qsTr("Copy this commit onto the current branch")
                enabled: repoTab.busyCount === 0
                onTriggered: repoTab.cherryPick(page.menuOid)
            }
            MenuItem {
                text: qsTr("Switch to this commit")
                enabled: repoTab.busyCount === 0
                onTriggered: page.switchTo("commit", page.menuOid, page.menuShort)
            }
            MenuSeparator {}
            MenuItem {
                text: qsTr("Edit message…")
                enabled: repoTab.busyCount === 0 && page.menuPublishKnown
                onTriggered: page.editMessage(page.menuOid)
            }
            MenuItem {
                text: qsTr("Fold into the commit before it")
                enabled: repoTab.busyCount === 0 && page.menuPublishKnown
                onTriggered: page.squashCommit(page.menuOid)
            }
            MenuSeparator {}
            MenuItem {
                text: qsTr("Copy commit hash")
                onTriggered: root.copyText(page.menuOid)
            }
        }

        // ---- rewriting one commit --------------------------------------
        // Both of these replay history when the commit is not the newest
        // one, so both warn once the commit has been pushed.
        function rewriteWarning(action, run) {
            if (!page.menuPublished) {
                run()
                return
            }
            root.confirm(
                qsTr("Rewrite a commit that is already on a remote?"),
                qsTr("%1 has been pushed. %2 replaces it, and every commit after "
                     + "it, with different ones — anyone who already has them will "
                     + "be out of step until they reset.")
                    .arg(page.menuShort).arg(action),
                qsTr("Rewrite anyway"), run)
        }
        function squashCommit(oidHex) {
            page.rewriteWarning(qsTr("Folding it in"),
                                function () { repoTab.squashIntoParent(oidHex) })
        }
        function editMessage(oidHex) {
            messageDialog.oid = oidHex
            messageDialog.published = page.menuPublished
            messageDialog.open()
        }

        RewordDialog {
            id: messageDialog
            details: detailsModel
            onSubmitted: (oid, subject, body, published) => {
                const run = function () {
                    repoTab.rewordCommit(oid, subject, body)
                }
                if (published)
                    page.rewriteWarning(qsTr("Changing its message"), run)
                else
                    run()
            }
        }

        // ---- smoke hook ------------------------------------------------
        // PG_AUTO_ACT runs one write operation through exactly the code
        // path a click takes, so the wiring can be proven headlessly. The
        // dispatch is equality on a bare verb; nothing here parses.
        Timer {
            id: autoActTimer
            interval: 1200
            onTriggered: page.runAutoAct()
        }
        // The diff has to arrive before a row of it can be staged.
        Timer {
            id: stageRowTimer
            interval: 800
            onTriggered: page.stageSelection(
                0, AppBackend.autoAct === "stage-line" ? 0 : -1)
        }
        function runAutoAct() {
            const act = AppBackend.autoAct
            const arg = AppBackend.autoActArg
            if (act === "commit") {
                repoTab.stageAll()
                wipSubject.text = arg
                page.commitNow()
            } else if (act === "amend") {
                // The message is supplied, so skip the prefill request
                // that would otherwise land on top of it.
                amendBox.checked = true
                page.amending = true
                wipSubject.text = arg
                page.commitNow()
            } else if (act === "switch") {
                page.switchTo("branch", arg, arg)
            } else if (act === "switch-leave") {
                // What the dirty-tree dialog's first button does.
                page.moveKind = "branch"
                page.moveTarget = arg
                page.runSwitch(true)
            } else if (act === "switch-remote") {
                page.switchTo("remote", arg, arg)
            } else if (act === "squash") {
                page.openCommitMenu(branchesModel.headOid)
                page.squashCommit(branchesModel.headOid)
            } else if (act === "reword") {
                page.openCommitMenu(branchesModel.headOid)
                repoTab.rewordCommit(branchesModel.headOid, arg, "")
            } else if (act === "cherry-pick") {
                repoTab.cherryPick(arg)
            } else if (act === "stage-hunk" || act === "stage-line") {
                page.toggleDiff("unstaged", arg, "")
                stageRowTimer.start()
            } else if (act === "push") {
                page.pushNow()
            } else if (act === "force-push") {
                repoTab.pushCurrent("lease", page.upstreamOid())
            } else if (act === "force-push-confirm") {
                // Goes through the confirmation, so nothing should be
                // pushed until someone answers it.
                page.forcePushNow()
            } else if (act === "fetch") {
                repoTab.fetch("")
            } else if (act === "preview") {
                page.toggleDiff("untracked", arg, "")
            } else if (act === "preview-unstaged") {
                page.toggleDiff("unstaged", arg, "")
            } else if (act === "preview-staged") {
                page.toggleDiff("staged", arg, "")
            } else if (act === "settings") {
                settingsDialog.open()
                AppBackend.setAutoFetchMinutes(Number(arg))
            }
            AppBackend.report("auto_act ran=" + act)
        }

        // A finished write the editor asked for: clear it only once git
        // says the commit landed, so a rejected one keeps its text.
        property int seenWriteSeq: 0
        function absorbWriteResult() {
            if (repoTab.writeSeq === page.seenWriteSeq)
                return
            page.seenWriteSeq = repoTab.writeSeq
            if (repoTab.lastWriteError !== "")
                return
            if (repoTab.lastWriteOp === "commit") {
                page.clearCommitEditor()
                amendBox.checked = false
                page.amending = false
            }
            if (repoTab.lastWriteOp === "stage" || repoTab.lastWriteOp === "unstage")
                page.reloadDiff()
            // Moving HEAD rewrites the working tree under the diff pane:
            // the file it holds may not even exist where the move landed,
            // so the center goes back to the graph that was moved through.
            if (repoTab.lastWriteOp === "checkout")
                page.closeDiff()
            page.refreshHeadPublished()
        }

        // Center area switches between the graph and a file diff. The
        // pieces are kept apart rather than parsed back out of the key:
        // a path may contain anything, colons included.
        property bool diffShown: false
        property string diffKey: ""
        property string diffKind: ""
        property string diffPath: ""
        property string diffOrigPath: ""
        // Whether the shown diff is a working-tree file (stageable).
        property bool diffFromWt: false
        readonly property bool diffStaged: page.diffKind === "staged"
        function toggleDiff(kind, path, origPath) {
            const key = kind + ":" + path
            if (page.diffShown && page.diffKey === key) {
                page.closeDiff()
                return
            }
            page.diffKey = key
            page.diffKind = kind
            page.diffPath = path
            page.diffOrigPath = origPath
            page.diffFromWt = kind !== "commit"
            if (kind === "commit")
                diffModel.requestCommitFile(detailsModel.shaHex, detailsModel.parentHex,
                                            path, origPath)
            else
                diffModel.requestWorkTree(kind, path, origPath)
            page.diffShown = true
        }
        // Stages (or unstages) one hunk, or one line of it. The indices
        // address the diff currently on screen, so the pane is reloaded
        // afterwards: once the patch is applied the rows have moved.
        function stageSelection(hunk, line) {
            repoTab.stageSelection(page.diffKind, page.diffPath, page.diffOrigPath, hunk, line)
            page.pendingDiffReload = true
        }
        property bool pendingDiffReload: false
        function reloadDiff() {
            if (!page.pendingDiffReload || !page.diffShown)
                return
            page.pendingDiffReload = false
            diffModel.requestWorkTree(page.diffKind, page.diffPath, page.diffOrigPath)
        }

        function closeDiff() {
            page.diffShown = false
            page.diffKey = ""
            page.diffKind = ""
            page.diffPath = ""
            page.diffOrigPath = ""
            page.diffFromWt = false
            diffModel.clear()
        }

        // Sidebar section expansion (filter reveals collapsed sections).
        property bool expBranches: true
        property bool expRemotes: true
        property bool expWorktree: true
        property bool expStashes: true
        property bool expTags: true

        // The last expanded section absorbs the leftover height so the
        // sidebar packs top to bottom.
        readonly property string lastOpen: refFilter.text !== "" ? "tags"
            : expTags ? "tags"
            : expStashes ? "stashes"
            : expWorktree ? "worktree"
            : expRemotes ? "remotes"
            : expBranches ? "branches" : ""

        // Exposed for the window toolbar (acts on the active tab).
        readonly property var pageTab: repoTab
        readonly property var pageWt: workTree

        RepoTab { id: repoTab }
        GraphModel { id: graphModel }
        WorkTreeModel { id: workTree }
        DetailsModel { id: detailsModel }
        DiffModel { id: diffModel }
        NavSectionModel { id: branchesModel }
        NavSectionModel { id: remotesModel }
        NavSectionModel { id: worktreeModel }
        NavSectionModel { id: worktreesModel }
        NavSectionModel { id: stashesModel }
        NavSectionModel { id: tagsModel }

        Component.onCompleted: {
            if (page.blank)
                return // no session to attach to; every model stays empty
            repoTab.attach(page.tab_id)
            graphModel.attach(page.tab_id)
            workTree.attach(page.tab_id)
            detailsModel.attach(page.tab_id)
            diffModel.attach(page.tab_id)
            branchesModel.attachSection(page.tab_id, "branches")
            remotesModel.attachSection(page.tab_id, "remotes")
            worktreeModel.attachSection(page.tab_id, "worktree")
            worktreesModel.attachSection(page.tab_id, "worktrees")
            stashesModel.attachSection(page.tab_id, "stashes")
            tagsModel.attachSection(page.tab_id, "tags")
            if (AppBackend.autoAct !== "")
                autoActTimer.start()
        }

        Connections {
            target: root
            function onFocusEpochChanged() {
                if (page.visible && repoTab.state === "open")
                    repoTab.refreshQuick()
            }
        }

        // Selection policy: restore across the tag-swap reset, and default
        // to the current branch's newest commit on first load so the
        // details pane always shows something.
        function trySelectDefault() {
            if (page.selectedOid !== "" || page.wipShown || AppBackend.autoSelect
                    || AppBackend.autoWip || graphModel.rowTotal === 0)
                return
            // Refs decide which commit is "current" — wait for them
            // instead of guessing the newest row too early.
            if (!branchesModel.refsLoaded)
                return
            let row = branchesModel.headOid !== ""
                      ? graphModel.rowOf(branchesModel.headOid) : -1
            if (row < 0) {
                if (graphModel.loading)
                    return // the head row may still be streaming in
                row = 0 // detached / head outside the window: newest commit
            }
            graphList.currentIndex = row
            anchorTimer.restart()
            graphList.rowSelected(graphModel.oidAt(row))
        }
        // Centering must outlive the ListView's own relayout: a model
        // reset (tag swap / reload) zeroes contentY during the polish that
        // runs after our handlers, so the anchor is applied a beat later.
        Timer {
            id: anchorTimer
            interval: Metrics.anchorDelayMs
            onTriggered: {
                if (graphList.currentIndex >= 0)
                    graphList.positionViewAtIndex(graphList.currentIndex, ListView.Center)
            }
        }
        // Each finished pass ends in one drain (the swap never shows a
        // loading edge), so watch the pass counter instead: re-resolve the
        // selection by oid and re-anchor the viewport on it.
        property int seenFinishCount: 0
        Connections {
            target: graphModel
            function onStatsChanged() {
                if (graphModel.finishCount !== page.seenFinishCount) {
                    page.seenFinishCount = graphModel.finishCount
                    if (page.selectedOid !== "") {
                        const row = graphModel.rowOf(page.selectedOid)
                        if (row >= 0) {
                            graphList.currentIndex = row
                            anchorTimer.restart()
                        }
                    }
                }
                page.trySelectDefault()
            }
        }
        Connections {
            target: branchesModel
            function onChanged() { page.trySelectDefault() }
        }
        Connections {
            target: worktreeModel
            function onChanged() {
                if (worktreeModel.total === 0 && page.wipShown)
                    page.wipShown = false
                // Smoke hook (PG_AUTO_WIP=1): open the WIP view once
                // uncommitted changes are known.
                if (AppBackend.autoWip && worktreeModel.total > 0 && !page.wipShown) {
                    graphList.currentIndex = 0
                    page.showWip()
                }
            }
        }

        // Smoke hook (PG_SCROLL_TO=top|bottom): jump the graph after the
        // final pass settles, using the same clamped math as the wheel.
        Timer {
            id: scrollToTimer
            interval: 600
            onTriggered: {
                graphList.contentY = graphList.clampY(
                    AppBackend.scrollTo === "bottom" ? 1e12 : -1e12)
            }
        }
        Connections {
            target: graphModel
            enabled: AppBackend.scrollTo !== ""
            function onStatsChanged() {
                if (graphModel.finishCount > 0)
                    scrollToTimer.restart()
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
            to: Math.min(3000, graphModel.rowTotal - 40) * Theme.graphRowHeight
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
            HoverButton {
                text: qsTr("Close tab")
                anchors.horizontalCenter: parent.horizontalCenter
                onClicked: tabsModel.closeTab(page.tab_id)
            }
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0
            visible: repoTab.state !== "error"

            // ---- three-pane layout --------------------------------------
            // (repository state / search / reload live in the window
            // toolbar, next to the tabs)
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
                        // Frameless, full-width filter: the sidebar is
                        // already enclosed by dividers, so the input only
                        // keeps a hairline underline (accent on focus).
                        // It is this pane's header band, so it takes the
                        // header height — the other panes' headers and the
                        // first row under each of them line up with it.
                        TextField {
                            id: refFilter
                            Layout.fillWidth: true
                            implicitHeight: Theme.headerHeight
                            font.pixelSize: Theme.fontMd
                            leftPadding: Theme.spaceSm
                            rightPadding: Theme.spaceSm
                            topPadding: 0
                            bottomPadding: 0
                            placeholderText: qsTr("Filter")
                            background: Rectangle {
                                color: "transparent"
                                Rectangle {
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.bottom: parent.bottom
                                    height: Theme.borderWidth
                                    color: refFilter.activeFocus ? Theme.borderFocus
                                                                 : Theme.borderSubtle
                                }
                            }
                            onTextChanged: {
                                branchesModel.setFilter(text)
                                remotesModel.setFilter(text)
                                worktreesModel.setFilter(text)
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
                        // Current branch pinned under the header (it stays
                        // in the list too, highlighted). Replaces the old
                        // top-left branch display.
                        Rectangle {
                            visible: (page.expBranches || refFilter.text !== "")
                                     && (branchesModel.headName !== "" || workTree.detached)
                            Layout.fillWidth: true
                            implicitHeight: Theme.rowHeight
                            color: Theme.bgElevated
                            Rectangle {
                                anchors.fill: parent
                                color: Theme.bgHover
                                visible: headRowMouse.containsMouse
                            }
                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: Theme.spaceMd
                                anchors.rightMargin: Theme.spaceSm
                                spacing: Theme.spaceXs
                                // "You are here" marker, sharing the fold
                                // arrows' column so the sidebar lines up.
                                NavIcon {
                                    kind: "check"
                                    tint: Theme.accent
                                    width: Theme.iconSm + 2
                                    height: Theme.iconSm + 2
                                }
                                Label {
                                    Layout.fillWidth: true
                                    text: workTree.detached ? qsTr("DETACHED HEAD")
                                                            : branchesModel.headName
                                    color: workTree.detached ? Theme.warning : Theme.textLink
                                    font.weight: Font.DemiBold
                                    font.pixelSize: Theme.fontMd
                                    elide: Text.ElideMiddle
                                }
                                Label {
                                    visible: !workTree.detached && workTree.upstream !== ""
                                    text: "↑" + workTree.ahead + " ↓" + workTree.behind
                                    color: Theme.textSecondary
                                    font.pixelSize: Theme.fontSm
                                }
                                NavIcon {
                                    visible: !workTree.detached
                                             && (branchesModel.headHasRemote
                                                 || branchesModel.headHasPr)
                                    kind: branchesModel.headHasPr ? "pr" : "remote"
                                    tint: branchesModel.headHasPr ? Theme.success
                                                                  : Theme.textSecondary
                                    width: Theme.iconSm + 2
                                    height: Theme.iconSm + 2
                                }
                            }
                            MouseArea {
                                id: headRowMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                enabled: branchesModel.headOid !== ""
                                onClicked: page.jumpToRef(branchesModel.headOid)
                            }
                        }
                        NavList {
                            sectionModel: branchesModel
                            expanded: page.expBranches || refFilter.text !== ""
                            kindHint: "branch"
                            stretch: page.lastOpen === "branches"
                            headTrack: workTree.upstream !== ""
                                       ? "↑" + workTree.ahead + " ↓" + workTree.behind : ""
                            onRefActivated: oidHex => page.jumpToRef(oidHex)
                            onRefMenuRequested: (name, oidHex) =>
                                page.openRefMenu(name, oidHex, false)
                        }

                        NavHeader {
                            caption: qsTr("REMOTES")
                            iconKind: "remote"
                            iconTint: Theme.textSecondary
                            count: remotesModel.total
                            expanded: page.expRemotes || refFilter.text !== ""
                            onToggled: page.expRemotes = !page.expRemotes
                        }
                        NavList {
                            sectionModel: remotesModel
                            expanded: page.expRemotes || refFilter.text !== ""
                            kindHint: "remote"
                            stretch: page.lastOpen === "remotes"
                            onRefActivated: oidHex => page.jumpToRef(oidHex)
                            onRefMenuRequested: (name, oidHex) =>
                                page.openRefMenu(name, oidHex, true)
                        }

                        // git worktrees (checkouts), GitKraken-style; the
                        // changed-file lists live in the right pane's WIP
                        // view. Clicking one opens it as a new tab.
                        NavHeader {
                            caption: qsTr("WORKTREES")
                            iconKind: "tree"
                            iconTint: Theme.success
                            count: worktreesModel.total
                            expanded: page.expWorktree || refFilter.text !== ""
                            onToggled: page.expWorktree = !page.expWorktree
                        }
                        NavList {
                            sectionModel: worktreesModel
                            expanded: page.expWorktree || refFilter.text !== ""
                            kindHint: "worktree"
                            stretch: page.lastOpen === "worktree"
                            onFileActivated: (bucket, path, origPath) =>
                                tabsModel.openRepositoryPath(path)
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
                            stretch: page.lastOpen === "stashes"
                            // A stash is a commit: clicking shows its
                            // stashed changes in the details pane.
                            onRefActivated: oidHex => page.jumpToRef(oidHex)
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
                            stretch: page.lastOpen === "tags"
                            onRefActivated: oidHex => page.jumpToRef(oidHex)
                        }
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
                        // Divider hover-lines live under the list so the
                        // message ticks and lane strokes stay in front.
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
                            model: graphModel
                            reuseItems: true
                            boundsBehavior: Flickable.StopAtBounds
                            flickDeceleration: 8000
                            maximumFlickVelocity: 9000
                            ScrollBar.vertical: AutoScrollBar {}
                            // The graph is the one pane with no header band;
                            // this sliver of margin drops the first row so
                            // its bottom line meets the neighbouring bands'
                            // bottom edge when scrolled to the top.
                            topMargin: Theme.headerHeight - Theme.graphRowHeight
                            // A sliver of run-out at the end: without it the
                            // oldest row sits flush on the pane edge and
                            // reads as clipped rather than as the end of
                            // what is loaded. Just enough to see the break.
                            bottomMargin: Theme.spaceSm
                            // Bridge into the page scope for the shared
                            // delegate (inline components cannot see page ids).
                            property real labelWidth: page.labelW
                            property real graphColWidth: page.graphColW
                            property real graphFullWidth: page.graphFullW
                            property real graphXOffset: page.graphX
                            property int wipCount: worktreeModel.total
                            signal rowSelected(string oidHex)
                            signal rowMenuRequested(string oidHex)
                            onRowMenuRequested: oidHex => page.openCommitMenu(oidHex)
                            onRowSelected: oidHex => {
                                // The all-zero id is the synthetic WIP row.
                                if (oidHex !== "" && !/[^0]/.test(oidHex)) {
                                    page.showWip()
                                    return
                                }
                                page.wipShown = false
                                page.selectedOid = oidHex
                                page.selectedStashRef = graphModel.stashRefOf(oidHex)
                                detailsModel.request(oidHex)
                                page.closeDiff()
                            }
                            delegate: GraphRowDelegate {}
                            // Window cut: lanes keep running through the
                            // footer and the message sits where subjects go.
                            footer: Item {
                                width: graphList.width
                                height: graphModel.truncated ? 2 * Theme.graphRowHeight : 0
                                visible: graphModel.truncated
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
                                            if (graphModel.tailGeometry === "")
                                                return
                                            ctx.lineWidth = Metrics.laneStroke
                                            ctx.globalAlpha = 0.45
                                            const toks = graphModel.tailGeometry.split(";")
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
                                            target: graphModel
                                            function onStatsChanged() { tailCanvas.requestPaint() }
                                        }
                                    }
                                }
                                // Bounded like a subject: the message is the
                                // only thing that explains the cut, so it
                                // wraps over the footer's two rows rather
                                // than running under the next pane.
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
                                          .arg(graphModel.rowTotal)
                                    color: Theme.warning
                                    font.pixelSize: Theme.fontMd
                                    font.weight: Font.DemiBold
                                }
                            }
                            // Manual contentY math must respect originY:
                            // after positionViewAtIndex jumps, the ListView
                            // shifts its coordinate origin as item positions
                            // are fixed up, so [0, contentHeight-height] no
                            // longer matches the real scroll range (top rows
                            // become unreachable, the bottom overshoots the
                            // truncation footer).
                            function clampY(y) {
                                // topMargin lives above the content origin —
                                // forgetting it makes the top gap
                                // unreachable by wheel after any scroll.
                                const minY = graphList.originY - graphList.topMargin
                                const maxY = Math.max(minY, graphList.originY
                                                            + graphList.contentHeight
                                                            - graphList.height
                                                            + graphList.bottomMargin)
                                return Math.max(minY, Math.min(y, maxY))
                            }
                            // Mouse wheels scroll a fixed number of rows per
                            // notch; touchpads keep native Flickable panning.
                            WheelHandler {
                                acceptedDevices: PointerDevice.Mouse
                                onWheel: event => {
                                    // Wheel input exits middle-click
                                    // autoscroll (Chrome-like behavior).
                                    graphArea.autoScrolling = false
                                    graphList.cancelFlick()
                                    if (event.angleDelta.x !== 0)
                                        page.graphX = Math.max(0, Math.min(
                                            page.graphX - event.angleDelta.x / 2, page.graphXMax))
                                    const step = (event.angleDelta.y / 120)
                                               * Metrics.wheelRows * Theme.graphRowHeight
                                    graphList.contentY = graphList.clampY(graphList.contentY - step)
                                }
                            }
                        }
                        // Middle-click toggles autoscroll mode: the pointer
                        // distance from the anchor sets the speed; any click
                        // exits.
                        property bool autoScrolling: false
                        property real autoAnchorX: 0
                        property real autoAnchorY: 0
                        property real autoCurrentX: 0
                        property real autoCurrentY: 0
                        id: graphArea
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
                                    if (page.graphXMax > 0) {
                                        const dx = (graphArea.autoCurrentX - graphArea.autoAnchorX)
                                                 * Metrics.middleScrollGain
                                        page.graphX = Math.max(0, Math.min(page.graphX + dx,
                                                                           page.graphXMax))
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
                        // Left-drag inside the lanes pans them horizontally
                        // when they overflow; a motionless press-release
                        // still selects the row underneath.
                        MouseArea {
                            id: lanePan
                            x: page.labelW
                            width: page.graphColW
                            height: parent.height
                            z: 1
                            visible: page.graphXMax > 0
                            acceptedButtons: Qt.LeftButton
                            property real pressX: 0
                            property real startGX: 0
                            property bool panning: false
                            onPressed: mouse => {
                                pressX = mouse.x
                                startGX = page.graphX
                                panning = false
                            }
                            onPositionChanged: mouse => {
                                if (!pressed)
                                    return
                                if (!panning && Math.abs(mouse.x - pressX) > Theme.spaceXs)
                                    panning = true
                                if (panning)
                                    page.graphX = Math.max(0, Math.min(
                                        startGX - (mouse.x - pressX), page.graphXMax))
                            }
                            onReleased: mouse => {
                                if (panning)
                                    return
                                const idx = graphList.indexAt(page.labelW + 1,
                                                              graphList.contentY + mouse.y)
                                if (idx >= 0) {
                                    graphList.currentIndex = idx
                                    graphList.rowSelected(graphModel.oidAt(idx))
                                }
                            }
                        }

                        // Draggable column dividers (labels | graph | message).
                        MouseArea {
                            id: labelDivider
                            x: page.labelW - Theme.splitterWidth / 2
                            width: Theme.splitterWidth
                            height: parent.height
                            z: 2
                            visible: !page.blank
                            hoverEnabled: true
                            cursorShape: Qt.SplitHCursor
                            preventStealing: true
                            onPositionChanged: mouse => {
                                if (!pressed)
                                    return
                                const nx = mapToItem(graphArea, mouse.x, 0).x
                                page.labelWManual = Math.max(Theme.spaceXxl,
                                    Math.min(nx, graphArea.width - 2 * Theme.spaceXxl))
                            }
                        }
                        MouseArea {
                            id: graphDivider
                            // Sits behind the message tick column so the
                            // hover line overlaps the ticks.
                            x: page.labelW + page.graphColW + Theme.spaceSm
                               + Theme.borderWidth - Theme.splitterWidth / 2
                            width: Theme.splitterWidth
                            height: parent.height
                            z: 2
                            visible: !page.blank
                            hoverEnabled: true
                            cursorShape: Qt.SplitHCursor
                            preventStealing: true
                            onPositionChanged: mouse => {
                                if (!pressed)
                                    return
                                const nx = mapToItem(graphArea, mouse.x, 0).x
                                              - Theme.spaceSm - Theme.borderWidth
                                page.graphColWManual = Math.max(Metrics.laneInset + Metrics.laneW,
                                    Math.min(nx - page.labelW,
                                             graphArea.width - page.labelW - 2 * Theme.spaceXxl))
                            }
                        }
                        // Horizontal scroll of the lanes when the full graph
                        // is wider than its column.
                        ScrollBar {
                            visible: page.graphXMax > 0
                            orientation: Qt.Horizontal
                            x: page.labelW
                            width: page.graphColW
                            anchors.bottom: parent.bottom
                            z: 2
                            size: page.graphFullW > 0 ? page.graphColW / page.graphFullW : 1
                            position: page.graphFullW > 0 ? page.graphX / page.graphFullW : 0
                            onPositionChanged: {
                                if (pressed)
                                    page.graphX = Math.max(0, Math.min(
                                        position * page.graphFullW, page.graphXMax))
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
                        // Empty window: the one thing worth doing sits in
                        // the column that will hold the history.
                        Column {
                            anchors.centerIn: parent
                            visible: page.blank
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
                                onClicked: folderDialog.open()
                            }
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
                                    HoverToolButton {
                                        visible: page.diffFromWt
                                        text: page.diffStaged ? qsTr("Unstage file")
                                                              : qsTr("Stage file")
                                        font.pixelSize: Theme.fontSm
                                        enabled: repoTab.busyCount === 0
                                        ToolTip.visible: hovered
                                        ToolTip.delay: 300
                                        ToolTip.text: page.diffStaged
                                            ? qsTr("Take this whole file out of the next commit")
                                            : qsTr("Put this whole file into the next commit")
                                        onClicked: {
                                            if (page.diffStaged)
                                                repoTab.unstagePath(page.diffPath)
                                            else
                                                repoTab.stagePath(page.diffPath)
                                        }
                                    }
                                    HoverToolButton {
                                        text: "×"
                                        implicitWidth: Theme.iconLg
                                        implicitHeight: Theme.iconLg
                                        padding: 0
                                        onClicked: page.closeDiff()
                                    }
                                }
                            }
                            // -- content preview: binaries summarized by
                            //    size, images rendered (added = After only,
                            //    deleted = Before only, modified = both).
                            Label {
                                visible: diffModel.previewKind === "binary"
                                         || (diffModel.isBinary
                                             && diffModel.previewKind === "")
                                Layout.margins: Theme.spaceSm
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                                text: {
                                    const oldS = diffModel.previewOldSize
                                    const newS = diffModel.previewNewSize
                                    if (oldS !== "" && newS !== "")
                                        return qsTr("Binary file · %1 → %2").arg(oldS).arg(newS)
                                    if (newS !== "")
                                        return qsTr("Binary file · %1").arg(newS)
                                    if (oldS !== "")
                                        return qsTr("Binary file removed · was %1").arg(oldS)
                                    return qsTr("Binary file — no text diff")
                                }
                                color: Theme.textMuted
                            }
                            RowLayout {
                                visible: diffModel.previewKind === "image"
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                Layout.margins: Theme.spaceSm
                                spacing: Theme.spaceSm
                                ImagePreviewCell {
                                    Layout.fillWidth: true
                                    Layout.fillHeight: true
                                    label: qsTr("Before · %1").arg(diffModel.previewOldSize)
                                    url: diffModel.previewOldUrl
                                    sizeText: diffModel.previewOldSize
                                }
                                ImagePreviewCell {
                                    Layout.fillWidth: true
                                    Layout.fillHeight: true
                                    label: qsTr("After · %1").arg(diffModel.previewNewSize)
                                    url: diffModel.previewNewUrl
                                    sizeText: diffModel.previewNewSize
                                }
                            }
                            ListView {
                                id: diffList
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                clip: true
                                model: diffModel
                                reuseItems: true
                                boundsBehavior: Flickable.StopAtBounds
                                ScrollBar.vertical: AutoScrollBar {}
                                // An image with no text rows hands its space
                                // to the preview (SVG edits keep both).
                                visible: diffModel.previewKind !== "image"
                                         || count > 0
                                delegate: Rectangle {
                                    id: diffRow
                                    required property string kind
                                    required property int old_no
                                    required property int new_no
                                    required property string text
                                    required property int hunk
                                    required property int line
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
                                    MouseArea {
                                        id: lineHover
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        acceptedButtons: Qt.NoButton
                                        enabled: page.diffFromWt
                                    }
                                    // Hunk-level staging. The row carries the
                                    // hunk index the patch builder needs, so
                                    // what is staged is exactly what is shown.
                                    HoverToolButton {
                                        visible: page.diffFromWt && diffRow.kind === "hunk"
                                        anchors.right: parent.right
                                        anchors.rightMargin: Theme.spaceSm
                                        anchors.verticalCenter: parent.verticalCenter
                                        text: page.diffStaged ? qsTr("Unstage hunk")
                                                              : qsTr("Stage hunk")
                                        font.pixelSize: Theme.fontSm
                                        enabled: repoTab.busyCount === 0
                                        onClicked: page.stageSelection(diffRow.hunk, -1)
                                    }
                                    // Line-level staging.
                                    Rectangle {
                                        visible: page.diffFromWt && lineHover.containsMouse
                                                 && (diffRow.kind === "add"
                                                     || diffRow.kind === "del")
                                        x: Theme.spaceXs
                                        anchors.verticalCenter: parent.verticalCenter
                                        width: Theme.iconMd
                                        height: Theme.iconMd
                                        radius: Theme.radiusSm
                                        color: Theme.bgElevated
                                        border.color: Theme.borderStrong
                                        border.width: Theme.borderWidth
                                        ToolTip.visible: stageLineHover.containsMouse
                                        ToolTip.delay: 300
                                        ToolTip.text: page.diffStaged ? qsTr("Unstage this line")
                                                                      : qsTr("Stage this line")
                                        Rectangle {
                                            anchors.fill: parent
                                            radius: Theme.radiusSm
                                            color: Theme.bgHover
                                            visible: stageLineHover.containsMouse
                                        }
                                        NavIcon {
                                            anchors.centerIn: parent
                                            width: Theme.iconSm
                                            height: Theme.iconSm
                                            kind: diffRow.kind === "add" ? "plus" : "minus"
                                            tint: Theme.textPrimary
                                        }
                                        MouseArea {
                                            id: stageLineHover
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            onClicked: page.stageSelection(diffRow.hunk,
                                                                           diffRow.line)
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

                        // ---- working-tree (WIP) mode: the commit-editor
                        // shape plus stage/unstage affordances. The
                        // controls are visual for now — the operations
                        // land in Phase 2. ----
                        Rectangle {
                            visible: page.wipShown
                            Layout.fillWidth: true
                            implicitHeight: Theme.headerHeight
                            color: Theme.bgElevated
                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: Theme.spaceSm
                                anchors.rightMargin: Theme.spaceXs
                                spacing: Theme.spaceXs
                                Label {
                                    text: qsTr("UNCOMMITTED CHANGES (%1)").arg(worktreeModel.total)
                                    font.pixelSize: Theme.fontSm
                                    font.weight: Font.DemiBold
                                    color: Theme.textSecondary
                                }
                                Item { Layout.fillWidth: true }
                                HoverToolButton {
                                    padding: 0
                                    implicitWidth: Theme.iconLg
                                    implicitHeight: Theme.iconLg
                                    ToolTip.visible: hovered
                                    ToolTip.delay: 600
                                    ToolTip.text: qsTr("Tree view")
                                    onClicked: worktreeModel.setTreeView(true)
                                    contentItem: NavIcon {
                                        kind: "hier"
                                        tint: worktreeModel.treeView ? Theme.accent
                                                                     : Theme.textMuted
                                    }
                                }
                                HoverToolButton {
                                    padding: 0
                                    implicitWidth: Theme.iconLg
                                    implicitHeight: Theme.iconLg
                                    ToolTip.visible: hovered
                                    ToolTip.delay: 600
                                    ToolTip.text: qsTr("Paths view")
                                    onClicked: worktreeModel.setTreeView(false)
                                    contentItem: NavIcon {
                                        kind: "list"
                                        tint: worktreeModel.treeView ? Theme.textMuted
                                                                     : Theme.accent
                                    }
                                }
                            }
                        }
                        // Message editor pinned on top — identical shape in
                        // commit details, amend and new-commit creation.
                        // Flush against the header band, like every other
                        // pane's first row.
                        ColumnLayout {
                            visible: page.wipShown
                            Layout.fillWidth: true
                            Layout.margins: Theme.spaceSm
                            Layout.topMargin: 0
                            spacing: Theme.spaceXs
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: wipSubject.implicitHeight + Theme.spaceSm
                                color: Theme.bgBase
                                radius: Theme.radiusMd
                                border.color: Theme.borderDefault
                                border.width: Theme.borderWidth
                                TextArea {
                                    id: wipSubject
                                    anchors.fill: parent
                                    anchors.margins: Theme.spaceXs
                                    wrapMode: TextArea.Wrap
                                    placeholderText: qsTr("Commit summary")
                                    font.pixelSize: Theme.fontLg
                                    font.weight: Font.DemiBold
                                    color: Theme.textPrimary
                                    background: null
                                    padding: 0
                                }
                            }
                            // Two lines tall from the start (matches the
                            // details pane's description box).
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: Math.min(Math.max(wipBody.implicitHeight,
                                                                          2 * Theme.fontMdLine)
                                                                 + Theme.spaceSm, 120)
                                color: Theme.bgBase
                                radius: Theme.radiusMd
                                border.color: Theme.borderSubtle
                                border.width: Theme.borderWidth
                                ScrollView {
                                    anchors.fill: parent
                                    anchors.margins: Theme.spaceXs
                                    TextArea {
                                        id: wipBody
                                        wrapMode: TextArea.Wrap
                                        placeholderText: qsTr("Description")
                                        font.pixelSize: Theme.fontMd
                                        color: Theme.textSecondary
                                        background: null
                                        padding: 0
                                    }
                                }
                            }
                            // Amend replaces the newest commit instead of
                            // adding one, so it starts from that commit's
                            // message rather than an empty editor.
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: Theme.spaceXs
                                CheckBox {
                                    id: amendBox
                                    text: qsTr("Amend the last commit")
                                    font.pixelSize: Theme.fontSm
                                    implicitHeight: Theme.controlHeight
                                    onToggled: page.amendToggled(checked)
                                }
                                Item { Layout.fillWidth: true }
                                // Warned about, not forbidden: git allows
                                // it and the confirmation says what it costs.
                                Label {
                                    visible: page.amending && page.headPublished
                                    text: qsTr("already pushed")
                                    color: Theme.warning
                                    font.pixelSize: Theme.fontSm
                                    ToolTip.visible: amendPushedHover.containsMouse
                                    ToolTip.delay: 400
                                    ToolTip.text: qsTr("The last commit is on a remote. "
                                                       + "Rewriting it would leave anyone "
                                                       + "who already has it out of step.")
                                    MouseArea {
                                        id: amendPushedHover
                                        anchors.fill: parent
                                        hoverEnabled: true
                                        acceptedButtons: Qt.NoButton
                                    }
                                }
                            }
                            HoverButton {
                                id: commitButton
                                Layout.fillWidth: true
                                highlighted: true
                                text: page.amending
                                      ? qsTr("Amend commit (%1 staged)").arg(workTree.stagedCount)
                                      : qsTr("Commit changes (%1 staged)").arg(workTree.stagedCount)
                                // An amend can stand on its own (message
                                // only); a new commit needs staged content
                                // and a summary, and git needs an identity
                                // to attribute either one to.
                                enabled: repoTab.busyCount === 0
                                         && repoTab.identityReady
                                         && wipSubject.text.trim() !== ""
                                         && (page.amending || workTree.stagedCount > 0)
                                onClicked: page.commitNow()
                                ToolTip.visible: commitHover.containsMouse && !enabled
                                ToolTip.delay: 300
                                ToolTip.text: !repoTab.identityReady
                                              ? qsTr("git has no name or email to record on commits")
                                              : wipSubject.text.trim() === ""
                                              ? qsTr("A commit needs a summary")
                                              : qsTr("Stage something to commit")
                                MouseArea {
                                    id: commitHover
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    acceptedButtons: Qt.NoButton
                                }
                            }
                        }
                        ListView {
                            id: wipList
                            visible: page.wipShown
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            clip: true
                            model: worktreeModel
                            reuseItems: true
                            ScrollBar.vertical: AutoScrollBar {}
                            // GitKraken grouping: unstaged (incl. untracked)
                            // above, staged below.
                            section.property: "group"
                            section.delegate: Rectangle {
                                id: bucketHeader
                                required property string section
                                width: wipList.width
                                height: Theme.rowHeight
                                color: Theme.bgElevated
                                RowLayout {
                                    anchors.fill: parent
                                    anchors.leftMargin: Theme.spaceSm
                                    anchors.rightMargin: Theme.spaceXs
                                    spacing: Theme.spaceXs
                                    Label {
                                        text: bucketHeader.section === "staged"
                                              ? qsTr("STAGED FILES (%1)").arg(workTree.stagedCount)
                                              : bucketHeader.section === "unstaged"
                                              ? qsTr("UNSTAGED FILES (%1)")
                                                .arg(workTree.unstagedCount + workTree.untrackedCount)
                                              : qsTr("CONFLICTS")
                                        font.pixelSize: Theme.fontSm
                                        font.weight: Font.DemiBold
                                        color: bucketHeader.section === "conflicts"
                                               ? Theme.danger : Theme.textSecondary
                                    }
                                    Item { Layout.fillWidth: true }
                                    HoverToolButton {
                                        visible: bucketHeader.section !== "conflicts"
                                        text: bucketHeader.section === "staged"
                                              ? qsTr("Unstage all") : qsTr("Stage all")
                                        font.pixelSize: Theme.fontSm
                                        ToolTip.visible: hovered
                                        ToolTip.delay: 300
                                        ToolTip.text: bucketHeader.section === "staged"
                                            ? qsTr("Empty the staging area")
                                            : qsTr("Stage every change, untracked files included")
                                        onClicked: {
                                            if (bucketHeader.section === "staged")
                                                repoTab.unstageAll()
                                            else
                                                repoTab.stageAll()
                                        }
                                    }
                                }
                            }
                            delegate: NavItemDelegate {
                                listWidth: wipList.width
                                kindHint: "wt"
                                showStage: true
                                onFileClicked: (bucket, path, origPath) =>
                                    page.toggleDiff(bucket, path, origPath)
                                onFolderClicked: key => worktreeModel.toggleFolder(key)
                                onStageClicked: (bucket, path) => {
                                    if (bucket === "staged")
                                        repoTab.unstagePath(path)
                                    else
                                        repoTab.stagePath(path)
                                }
                            }
                        }

                        // ---- commit-details mode ----
                        PaneHeader {
                            visible: !page.wipShown
                            text: qsTr("COMMIT")
                        }
                        // Stash actions when the selected row is a stash.
                        Rectangle {
                            visible: !page.wipShown && page.selectedStashRef !== ""
                            Layout.fillWidth: true
                            implicitHeight: Theme.headerHeight
                            color: Theme.bgElevated
                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: Theme.spaceSm
                                anchors.rightMargin: Theme.spaceXs
                                spacing: Theme.spaceXs
                                NavIcon {
                                    kind: "stash"
                                    tint: Theme.textSecondary
                                    width: Theme.iconSm + 2
                                    height: Theme.iconSm + 2
                                }
                                Label {
                                    text: page.selectedStashRef
                                    font.family: Theme.monoFamily
                                    font.pixelSize: Theme.fontSm
                                    color: Theme.textSecondary
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }
                                HoverToolButton {
                                    text: qsTr("Apply")
                                    font.pixelSize: Theme.fontSm
                                    ToolTip.visible: hovered
                                    ToolTip.delay: 600
                                    ToolTip.text: qsTr("Apply this stash, keeping it")
                                    onClicked: repoTab.applyStash(page.selectedStashRef)
                                }
                                HoverToolButton {
                                    text: qsTr("Pop")
                                    font.pixelSize: Theme.fontSm
                                    ToolTip.visible: hovered
                                    ToolTip.delay: 600
                                    ToolTip.text: qsTr("Apply this stash and drop it")
                                    onClicked: {
                                        repoTab.popStash(page.selectedStashRef)
                                        page.selectedStashRef = ""
                                    }
                                }
                            }
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            Layout.margins: Theme.spaceSm
                            Layout.topMargin: 0
                            spacing: Theme.spaceXs
                            visible: !page.wipShown && detailsModel.shaHex !== ""

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
                            // Always shown, even empty, and two lines tall
                            // from the start — the pair mirrors the commit
                            // editor's fields.
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: Math.min(Math.max(bodyArea.implicitHeight,
                                                                          2 * Theme.fontMdLine)
                                                                 + Theme.spaceSm, 120)
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
                                    width: Metrics.detailsAvatar
                                    height: Metrics.detailsAvatar
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
                                    // The hash is the button, not just the
                                    // icon beside it — a 16px glyph was too
                                    // small to aim at. Hovering underlines
                                    // the hash and lights the icon so the
                                    // whole plate reads as one control.
                                    // Not a HoverToolButton: the style's panel
                                    // would make the plate taller than one
                                    // line and drop this hash out of step with
                                    // the author name beside it, so it draws
                                    // the same wash over its own flat face.
                                    ToolButton {
                                        id: hashCopy
                                        Layout.alignment: Qt.AlignRight
                                        text: detailsModel.sha8
                                        leftPadding: Theme.spaceXs
                                        rightPadding: Theme.spaceXs
                                        topPadding: 0
                                        bottomPadding: 0
                                        readonly property bool lit: hovered || visualFocus
                                        ToolTip.visible: hovered
                                        ToolTip.delay: 600
                                        ToolTip.text: qsTr("Copy full hash")
                                        onClicked: root.copyText(detailsModel.shaHex)
                                        background: Rectangle {
                                            radius: Theme.radiusSm
                                            color: hashCopy.down ? Theme.bgPressed
                                                 : hashCopy.lit ? Theme.bgHover
                                                 : "transparent"
                                            MouseArea {
                                                anchors.fill: parent
                                                acceptedButtons: Qt.NoButton
                                                cursorShape: Qt.PointingHandCursor
                                            }
                                            // Drawn here rather than as the
                                            // label's font underline so the
                                            // rule runs under the icon too —
                                            // the hash and the icon are one
                                            // target, so they get one line.
                                            Rectangle {
                                                visible: hashCopy.lit
                                                color: Theme.textPrimary
                                                height: Theme.borderWidth
                                                anchors.left: parent.left
                                                anchors.right: parent.right
                                                anchors.bottom: parent.bottom
                                                anchors.leftMargin: hashCopy.leftPadding
                                                anchors.rightMargin: hashCopy.rightPadding
                                            }
                                        }
                                        contentItem: RowLayout {
                                            spacing: Theme.spaceXs
                                            Label {
                                                text: hashCopy.text
                                                font.family: Theme.monoFamily
                                                font.pixelSize: Theme.fontMd
                                                color: Theme.textPrimary
                                                Layout.alignment: Qt.AlignVCenter
                                            }
                                            NavIcon {
                                                kind: "copyicon"
                                                tint: hashCopy.lit ? Theme.textPrimary
                                                                   : Theme.textSecondary
                                                Layout.alignment: Qt.AlignVCenter
                                            }
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
                        // CHANGES header with the tree ⇄ path view toggle.
                        Rectangle {
                            visible: !page.wipShown && detailsModel.shaHex !== ""
                            Layout.fillWidth: true
                            implicitHeight: Theme.headerHeight
                            color: Theme.bgElevated
                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: Theme.spaceSm
                                anchors.rightMargin: Theme.spaceXs
                                spacing: Theme.spaceXs
                                Label {
                                    text: qsTr("CHANGES (%1)").arg(detailsModel.fileTotal)
                                    font.pixelSize: Theme.fontSm
                                    font.weight: Font.DemiBold
                                    color: Theme.textSecondary
                                }
                                Item { Layout.fillWidth: true }
                                HoverToolButton {
                                    padding: 0
                                    implicitWidth: Theme.iconLg
                                    implicitHeight: Theme.iconLg
                                    ToolTip.visible: hovered
                                    ToolTip.delay: 600
                                    ToolTip.text: qsTr("Tree view")
                                    onClicked: detailsModel.setTreeView(true)
                                    contentItem: NavIcon {
                                        kind: "hier"
                                        tint: detailsModel.treeView ? Theme.accent
                                                                    : Theme.textMuted
                                    }
                                }
                                HoverToolButton {
                                    padding: 0
                                    implicitWidth: Theme.iconLg
                                    implicitHeight: Theme.iconLg
                                    ToolTip.visible: hovered
                                    ToolTip.delay: 600
                                    ToolTip.text: qsTr("Paths view")
                                    onClicked: detailsModel.setTreeView(false)
                                    contentItem: NavIcon {
                                        kind: "list"
                                        tint: detailsModel.treeView ? Theme.textMuted
                                                                    : Theme.accent
                                    }
                                }
                            }
                        }
                        ListView {
                            id: fileList
                            visible: !page.wipShown
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            clip: true
                            model: detailsModel
                            reuseItems: true
                            ScrollBar.vertical: AutoScrollBar {}
                            delegate: FileRowDelegate {
                                listWidth: fileList.width
                                onActivated: (bucket, path, origPath) =>
                                    page.toggleDiff("commit", path, origPath)
                                onFolderToggled: key => detailsModel.toggleFolder(key)
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
            page.wipShown = false
            page.selectedOid = oidHex
            page.selectedStashRef = graphModel.stashRefOf(oidHex)
            detailsModel.request(oidHex)
            page.closeDiff()
        }
    }

}
