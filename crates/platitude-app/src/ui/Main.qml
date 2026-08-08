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
import QtQuick.Window
import platitude
import platitude.ui

ApplicationWindow {
    id: root
    width: 1440
    height: 900
    visible: true
    /// Whether the tab row is the window's title bar.
    ///
    /// Named by platform rather than worked out from what the hints did:
    /// they are read when the window is created, and a window that came up
    /// without a way to close it cannot be taken back. Windows is the one
    /// that has been seen to work. Everywhere else keeps the platform's
    /// own title bar above an ordinary tab row — the layout this app
    /// already had, so the fallback is not new code (P3-確認事項
    /// §ウィンドウ chrome).
    readonly property bool captionMerged: Qt.platform.os === "windows"

    // Three hints make the band the title bar, and the fourth thing that
    // matters is what is *not* asked for:
    //
    //   ExpandedClientAreaHint    the client area reaches the top of the
    //                             window instead of starting under a
    //                             caption band (measured: the frame stops
    //                             reserving 31px for one)
    //   NoTitleBarBackgroundHint  the platform stops painting that band
    //   CustomizeWindowHint       said without WindowTitleHint, which is
    //                             what drops the title text and the icon
    //                             the platform would paint over the tabs
    //   no button hints           so the platform draws no buttons of its
    //                             own. The app draws them, because theirs
    //                             are a fixed 32px tall with their own
    //                             hover and their own glyphs, and none of
    //                             the three can be styled
    //
    // Dropping the button hints also drops the style bits that let the
    // system minimise and maximise the window at all; `keepWindowGestures`
    // puts those back without the drawing coming with them.
    flags: root.captionMerged
           ? (Qt.Window | Qt.CustomizeWindowHint
              | Qt.ExpandedClientAreaHint | Qt.NoTitleBarBackgroundHint)
           : Qt.Window
    title: qsTr("Platitude GG")
    color: Theme.bgBase

    // ---- what a title bar does, now that this band is one ----------------
    /// Drags the window. Only the platform can move a window, and only
    /// from inside the press that started it.
    function dragWindow() {
        root.startSystemMove()
    }
    /// Goes through `visibility`, which is also where the saved shape is
    /// read from, so a window left maximised comes back that way.
    function toggleMaximized() {
        root.visibility = root.visibility === Window.Maximized
                          ? Window.Windowed : Window.Maximized
    }
    function minimizeWindow() {
        root.visibility = Window.Minimized
    }

    /// How far a maximised window reaches past the screen.
    ///
    /// Windows inflates a maximised frame by the width of its invisible
    /// resize border on every side — the platform's own title bar used to
    /// absorb that at the top, and there is no title bar here to do it
    /// (measured: a 1936x1048 frame on a 1920x1032 work area, so 8 all
    /// round, which was taking the top of the band and the outer edge of
    /// the close button off screen with it). Read from the two sizes
    /// rather than from a metric, so whatever the border turns out to be
    /// on a given display is what comes back.
    readonly property real maximizedInset:
        root.captionMerged && root.visibility === Window.Maximized
        ? Math.max(0, (root.width - Screen.desktopAvailableWidth) / 2) : 0
    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd

    // Every control asks its palette for a role (`text`, `buttonText`,
    // …) and is handed the group that matches its own state, so this one
    // block is what tells menus, buttons and check boxes alike how being
    // switched off looks.
    //
    // A color written here without a group lands in *all three* groups —
    // and, being a binding, it settles after the groups' own bindings and
    // overwrites them (measured: with `windowText` set both group-less
    // and under `disabled`, the disabled group kept the group-less
    // color). So a role either never changes and is written once, or it
    // changes and is written out in every group. Never both.
    palette {
        // Same whatever state a control is in.
        window: Theme.bgBase
        base: Theme.bgBase
        button: Theme.bgElevated
        placeholderText: Theme.textMuted
        mid: Theme.borderDefault
        dark: Theme.bgBase
        light: Theme.borderDefault

        active {
            windowText: Theme.textPrimary
            text: Theme.textPrimary
            buttonText: Theme.textPrimary
            brightText: Theme.textOnAccent
            highlight: Theme.accent
            highlightedText: Theme.textOnAccent
        }
        // Losing the window's focus is not a state worth showing: the
        // same colors as active.
        inactive {
            windowText: Theme.textPrimary
            text: Theme.textPrimary
            buttonText: Theme.textPrimary
            brightText: Theme.textOnAccent
            highlight: Theme.accent
            highlightedText: Theme.textOnAccent
        }
        // What cannot be pressed says so: labels drop to the muted text,
        // and the accent face of a highlighted button (Commit, Save)
        // drops with them so it stops reading as the one thing to press.
        disabled {
            windowText: Theme.textMuted
            text: Theme.textMuted
            buttonText: Theme.textMuted
            brightText: Theme.textMuted
            highlight: Theme.accentMuted
            highlightedText: Theme.textMuted
        }
    }

    // Window focus is a refresh trigger (refs/status/stash only).
    property int focusEpoch: 0
    onActiveChanged: if (active) focusEpoch++

    // Being on screen — not being focused — is what drives the periodic
    // re-read. This window is usually the one sitting beside the editor,
    // terminal or agent that moves the repository, and a window that only
    // catches up when clicked hides exactly what it is kept open to show.
    readonly property bool onScreen: root.visible
                                     && root.visibility !== Window.Minimized
                                     && root.visibility !== Window.Hidden

    // QML never drops a text input's focus on its own: once the sidebar
    // filter or the commit editor was clicked, its caret kept blinking
    // until some other editor took focus. This watcher hands focus back
    // to the window whenever a press lands outside the focused editor.
    //
    // It must sit *above* every pane: press delivery visits items front
    // to back and stops at the first one that accepts, so a handler on
    // the window's own content item never hears clicks that land on a
    // row's MouseArea (measured: focus survived a graph click). And it
    // must be a PointHandler — a fronted TapHandler swallowed the press
    // and the control underneath never received it (measured: the
    // filter field stopped taking focus at all). PointHandler is the
    // one handler specified to take only passive grabs and accept
    // nothing, so everything below keeps working. Modal dialogs live in
    // the window overlay above this item and are unaffected.
    Item {
        anchors.fill: parent
        z: 10000
        PointHandler {
            acceptedButtons: Qt.AllButtons
            onActiveChanged: {
                if (!active)
                    return
                const item = root.activeFocusItem
                // Only text editors hold a caret worth releasing; list
                // views and buttons manage their own focus.
                if (!item || item.cursorPosition === undefined)
                    return
                const local = item.mapFromItem(null, point.scenePressPosition)
                if (local.x < 0 || local.y < 0
                        || local.x >= item.width || local.y >= item.height)
                    root.contentItem.forceActiveFocus()
            }
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

    // Re-read the open repository from disk. The page keeps itself
    // current on a tick, so this is the way out of the cases a tick
    // cannot cover rather than a thing to reach for — a key and a menu
    // row, not a button holding down toolbar room.
    Shortcut {
        sequence: "F5"
        enabled: root.curPage !== null
        onActivated: root.curPage.pageTab.refreshAll()
    }

    // Search comes down over the graph on the key rather than standing in
    // the toolbar: a box that is only wanted now and then does not earn a
    // permanent place in the band.
    Shortcut {
        // The plural: the platform's "find" is more than one key on some
        // of them, and `sequence` would take only the first and say so.
        sequences: [StandardKey.Find]
        enabled: root.curPage !== null
        onActivated: root.curPage.startFind()
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

    // Every way in goes through here so the picker opens beside the
    // repository that is already open. Left to itself the dialog comes
    // back up inside the folder it last accepted — the repository — and
    // the next one is always a level up from there.
    function openRepositoryPicker() {
        if (root.curPage !== null && root.curPage.pageTab.pickerFolderUrl !== "")
            folderDialog.currentFolder = root.curPage.pageTab.pickerFolderUrl
        folderDialog.open()
        if (AppBackend.autoAct !== "")
            AppBackend.report("picker folder=" + folderDialog.currentFolder)
    }

    // The size and place the window was left in. Assigned rather than
    // bound: from here on the window manager and the person dragging it
    // own these. An unsaved position stays unset so the platform gets to
    // place the window itself — a first run should not open at 0,0.
    function applySavedWindow() {
        root.width = AppBackend.startWindowWidth()
        root.height = AppBackend.startWindowHeight()
        const x = AppBackend.startWindowX()
        const y = AppBackend.startWindowY()
        if (x !== root.unplaced && y !== root.unplaced) {
            root.x = x
            root.y = y
        }
        if (AppBackend.startWindowMaximized())
            root.visibility = Window.Maximized
    }
    /// What the store sends for a coordinate it has never been told.
    readonly property int unplaced: -2147483648

    /// Everything the next launch should come back to. One place, because
    /// what is worth writing is the shape the window settled into, not
    /// every value it passed through on the way (実装計画 §7).
    function reportState() {
        AppBackend.saveWindow(root.x, root.y, root.width, root.height,
                              root.visibility === Window.Maximized)
        if (root.curPage !== null)
            root.curPage.reportLayout()
        AppBackend.flushState()
    }

    Timer {
        id: stateTimer
        interval: Metrics.stateFlushMs
        repeat: true
        running: true
        onTriggered: root.reportState()
    }

    // PG_AUTO_ACT=state: what a launch came back to, and (with the
    // argument "change") something for the next one to come back to.
    // Two runs sharing one --config-dir are what actually tests this —
    // a single run can only ever agree with itself.
    Timer {
        id: stateActTimer
        interval: 1200
        onTriggered: {
            if (AppBackend.autoActArg === "change" && root.curPage !== null) {
                root.curPage.sidebarCollapsed = true
                root.curPage.commandsOpen = true
                root.curPage.setDetailsWidth(520)
                root.curPage.setGraphColumns(190, 300)
                // The other file: a decision, written out at once rather
                // than on the state timer.
                AppBackend.setAutoFetchMinutes(7)
            }
            stateReportTimer.start()
        }
    }
    // The splitters have to have taken the new sizes before they can be
    // read back off the panes.
    Timer {
        id: stateReportTimer
        interval: 400
        onTriggered: {
            root.reportState()
            AppBackend.report(
                "state tabs=" + pageRepeater.count
                + " active=" + tabsModel.currentIndex
                + " opened=" + (root.curPage !== null ? root.curPage.pageTab.state : "-")
                + " collapsed=" + (root.curPage !== null ? root.curPage.sidebarCollapsed : "-")
                + " sidebar=" + AppBackend.startSidebarWidth()
                + " details=" + AppBackend.startDetailsWidth()
                + " graphLabels=" + AppBackend.startGraphLabelsWidth()
                + " graphLanes=" + AppBackend.startGraphLanesWidth()
                + " commands=" + AppBackend.startCommandsShown()
                + " maximized=" + (root.visibility === Window.Maximized)
                + " autoFetch=" + AppBackend.autoFetchMinutes)
        }
    }

    // Closing is the last chance: the timer will not come round again.
    onClosing: root.reportState()

    Component.onCompleted: {
        AppBackend.initialize()
        // Windows 11 rounds the window itself and leaves the corner pixels
        // transparent, so the desktop shows through them. The window is up
        // by now (`visible` is set above), which is all the switch needs.
        AppBackend.squareWindowCorners()
        // Without this the window wears the shell's generic icon, in the
        // title bar and on the taskbar button alike.
        AppBackend.setWindowIcon()
        // Asking for no drawn buttons took the system's own gestures with
        // them; this puts those back (see `flags` above).
        if (root.captionMerged)
            AppBackend.keepWindowGestures()
        root.applySavedWindow()
        if (AppBackend.autoOpen !== "") {
            // Multiple repositories separated by ';' open as tabs in order.
            const paths = AppBackend.autoOpen.split(";")
            for (let i = 0; i < paths.length; i++) {
                if (paths[i] !== "")
                    tabsModel.openRepositoryPath(paths[i])
            }
        } else {
            tabsModel.restoreTabs()
        }
        if (AppBackend.autoAct === "state")
            stateActTimer.start()
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
    // Popups (dialogs, menus) render in the window overlay, whose
    // C++-created items grabToImage refuses ("no QML engine"). This
    // QML-declared mirror of the overlay is grabbable, which makes
    // popups photographable on the offscreen platform, where no OS
    // window exists to shoot from outside. Loaded only while a shot
    // directory is set, so ordinary runs pay nothing for it.
    Loader {
        id: overlayMirror
        active: AppBackend.shotDir !== ""
        anchors.fill: parent
        z: -10000
        sourceComponent: ShaderEffectSource {
            sourceItem: root.Overlay.overlay
            live: true
        }
    }
    Timer {
        id: shotTimer
        interval: AppBackend.autoQuitMs > 800 ? AppBackend.autoQuitMs - 800 : 3500
        onTriggered: {
            const path = AppBackend.shotDir + "/app.png"
            if (overlayMirror.item)
                overlayMirror.item.grabToImage(function (res) {
                    const saved = res.saveToFile(AppBackend.shotDir + "/overlay.png")
                    console.warn("overlay saved=" + saved)
                })
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
                text: qsTr("Platitude GG")
                font.pixelSize: Theme.fontXl
                font.weight: Font.DemiBold
                anchors.horizontalCenter: parent.horizontalCenter
            }
            // The drawn ring, not Fusion's BusyIndicator — the same
            // turning mark as everywhere else in the window
            // (規約 §進行中・長押しの定数).
            NavIcon {
                visible: AppBackend.gitState === "checking"
                width: Theme.iconLg
                height: Theme.iconLg
                kind: "spinner"
                tint: Theme.textSecondary
                anchors.horizontalCenter: parent.horizontalCenter
                // On the render thread, so it keeps turning while the GUI
                // thread drains models.
                RotationAnimator on rotation {
                    running: AppBackend.gitState === "checking"
                             && AppBackend.shotDir === ""
                    loops: Animation.Infinite
                    from: 0
                    to: 360
                    duration: Metrics.spinMs
                }
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

    // ---- settings --------------------------------------------------------
    SettingsDialog {
        id: settingsDialog
        curPage: root.curPage
    }

    // ---- main ------------------------------------------------------------
    ColumnLayout {
        id: mainUi
        // ApplicationWindow keeps its content item inside the window's
        // safe area, and with the client area expanded that area starts
        // below the title bar (measured on Windows: y = 31, the band's own
        // height). So where the band is ours, the chrome hangs off the
        // window's root item instead, which starts at 0. Where it is not,
        // the content item is already the whole client area and
        // ApplicationWindow's own handling is the one to keep.
        parent: root.captionMerged ? root.contentItem.parent : root.contentItem
        anchors.fill: parent
        // Everything the band carries stays inside the screen when the
        // window is maximised (see `maximizedInset`).
        anchors.margins: root.maximizedInset
        spacing: 0
        visible: AppBackend.gitState === "ok"

        TopBar {
            id: topBar
            Layout.fillWidth: true
            tabsModel: tabsModel
            curPage: root.curPage
            captionMerged: root.captionMerged
            windowMaximized: root.visibility === Window.Maximized
            onOpenRepositoryRequested: root.openRepositoryPicker()
            onIdentityEditRequested: root.identityEditing = true
            onSettingsRequested: settingsDialog.open()
            onWindowDragRequested: root.dragWindow()
            onMaximizeToggleRequested: root.toggleMaximized()
            onMinimizeRequested: root.minimizeWindow()
            onCloseRequested: root.close()
        }
        // Smoke hook (PG_AUTO_ACT=middle-close): the gesture is on the tab
        // strip, which lives up here rather than on the page where most of
        // the verbs sit. The argument names the tab to aim at (the first
        // one by default); what is still open afterwards is the report,
        // since a closed tab leaves nothing of itself in the picture.
        Timer {
            interval: 1200
            running: AppBackend.autoAct === "middle-close"
            onTriggered: {
                topBar.middleClickTab(Number(AppBackend.autoActArg))
                AppBackend.report("middle_close tabs=" + pageRepeater.count
                                  + " active=" + tabsModel.currentIndex
                                  + " open=" + topBar.tabPaths())
            }
        }

        // Smoke hook (PG_AUTO_ACT=force-push-hold): the overwrite is only
        // reachable by holding the toolbar's button, which is here rather
        // than on the page where the other verbs live. Waits out the page's
        // own auto-act beat so the branch's standing is settled first —
        // the hold is armed only where the two histories have parted.
        Timer {
            interval: 2000
            running: AppBackend.autoAct === "force-push-hold"
            onTriggered: topBar.completePushHold()
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
                    onOpenRepositoryPicker: root.openRepositoryPicker()
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
                    onScreen: root.onScreen
                    pageCurrent: index === tabsModel.currentIndex
                    onOpenRepositoryPicker: root.openRepositoryPicker()
                    onOpenRepositoryPathRequested: path => tabsModel.openRepositoryPath(path)
                    onSettingsDialogRequested: settingsDialog.open()
                    // The same card, told whom it was opened on before it
                    // opens — pictures are managed in one list, and an
                    // avatar's badge is the way in that saves naming the
                    // person (デザイン規約 §アバターを与える).
                    onAvatarSettingsRequested: (name, email) => {
                        settingsDialog.prefillName = name
                        settingsDialog.prefillEmail = email
                        settingsDialog.open()
                    }
                    onCloseTabRequested: tabsModel.closeTab(tab_id)
                }
            }
        }

        // Bottom edge: the same splitter-style divider closes the window.
        // The git version used to float here; it moved into the page's
        // right pane, where the command log cannot open underneath it.
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.splitterWidth
            color: Theme.borderSubtle
        }
    }
}
