// platitude-gg main window: application chrome (tabs, toolbar, app
// dialogs) around one RepoPage per open repository. Presentation only —
// every model row arrives precomputed from Rust.
//
// All colors / fonts / dimensions come from the Theme and Metrics
// singletons (internal-docs/デザイン規約.md is the source of truth).
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

    // Frame counter for the scroll benchmark (PG_AUTO_SCROLL=1); the
    // page's bench reads it through Window.window.
    property int frameCounter: 0
    onFrameSwapped: frameCounter++

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

        TopBar {
            Layout.fillWidth: true
            tabsModel: tabsModel
            curPage: root.curPage
            onOpenRepositoryRequested: folderDialog.open()
            onIdentityEditRequested: root.identityEditing = true
            onSettingsRequested: settingsDialog.open()
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
                    onOpenRepositoryPicker: folderDialog.open()
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
                RepoPage {
                    focusEpoch: root.focusEpoch
                    onConfirmRequested: (heading, detail, acceptText, action) =>
                        root.confirm(heading, detail, acceptText, action)
                    onOpenRepositoryPicker: folderDialog.open()
                    onOpenRepositoryPathRequested: path => tabsModel.openRepositoryPath(path)
                    onSettingsDialogRequested: settingsDialog.open()
                    onCloseTabRequested: tabsModel.closeTab(tab_id)
                }
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
}
