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
    /// Decided by platform, not read back from the hints: hints are read when the window is created, and a window
    /// that came up without a way to close it cannot be taken back. Only Windows is known to work; everywhere else
    /// keeps the platform's own title bar above an ordinary tab row (P3-確認事項 §ウィンドウ chrome). `PG_PLAIN_CHROME=1`
    /// asks for the other shape from here.
    readonly property bool captionMerged: Qt.platform.os === "windows" && !AppBackend.plainChrome

    // No frame at all: an expanded client area over the caption still leaves a real non-client frame — inflated past
    // the screen when maximised (measured: it covered eight columns of the next monitor), a white pixel no DWM
    // attribute moves, a remembered size coming back too wide. `FramelessWindowHint` deletes the whole area; the edge
    // is drawn in the scene (below), and the resize edges and grab run stay with the subclass
    // (`winframe::take_frame_hit_test` — Qt 6.10's own custom-chrome answer synthesises input from a poll and loses
    // track of it: the dead first click and the frozen hover). `keepWindowGestures` puts back the system's
    // minimise/maximise/menu style bits, and the subclass pins a maximise to the work area (`clamp_maximized`) because
    // Windows would otherwise maximise a frameless window over the whole monitor. No shadow: the drawn edge stands in.
    flags: root.captionMerged ? (Qt.Window | Qt.FramelessWindowHint) : Qt.Window
    title: Words.appName
    color: Theme.bgBase

    // ---- the floor the window may not be dragged under --------------------
    // Past the minimums, `SplitView` and the layouts here lay items out over their own edge, and nothing scrolls
    // sideways to reach what went over (measured 2026-08-09: at 640px the right pane had 32 of its 300 on screen).
    //
    // Read off what is on screen because the list folding and the command log opening both move it. Qt grows a
    // window when a floor rises under it, which is the way back from fold → shrink → unfold.
    //
    // `minimumWidth` alone only covers a dragged edge: `QWindow::resize` hands the size straight to the platform
    // without reading the hints (measured: asked for 200x150 against the floor, the window took it). So every size
    // this application sets itself goes through `holdFloor` below.
    /// The page the floor is read off. Not `curPage`: with no tab open that is null while the window is showing the
    /// blank page, which has the same three panes with the same minimums.
    readonly property var floorPage: root.curPage !== null ? root.curPage : blankPage.item
    readonly property real floorWidth:
        Math.max(topBar.floorWidth, root.floorPage !== null ? root.floorPage.floorWidth : 0)
    /// The same floor with the left list open whether or not it is — the width the band's three actions have finished
    /// giving their words up at (`TopBar.actionCap`). Folding the list lowers the real floor, and a schedule read off
    /// that would put the words back as the rail took the list's place (規約 §窓の床).
    readonly property real openFloorWidth:
        Math.max(topBar.floorWidth, root.floorPage !== null ? root.floorPage.openFloorWidth : 0)
    readonly property real floorHeight:
        // The band, the divider under it, and the line the window's bottom edge is drawn as — the three rows of
        // `mainUi` that are not the page (they carry their own heights; the page's is its own floor).
        Theme.toolbarHeight + Theme.splitterWidth + Theme.borderWidth
        + (root.floorPage !== null ? root.floorPage.floorHeight : 0)
    minimumWidth: Math.ceil(root.floorWidth)
    minimumHeight: Math.ceil(root.floorHeight)
    onFloorWidthChanged: windowShape.holdFloor()
    onFloorHeightChanged: windowShape.holdFloor()

    // The floor says how small the window may be; `WindowShape` says what it opens at, what its frame slop is, and
    // what the next launch is told. The way in keeps its name here: the harness calls `reportState()` on the window.
    WindowShape {
        id: windowShape
        window: root
    }
    function reportState() {
        windowShape.reportState()
    }

    // ---- what a title bar does, now that this band is one ----------------
    // Maximise, minimise, the grab-run's whereabouts and the window's dressing (`WindowChrome`). Only this file
    // calls them, so the names live on the chrome rather than being forwarded.
    WindowChrome {
        id: chrome
        window: root
        topBar: topBar
    }
    onWidthChanged: chrome.reportCaptionStrip()

    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd

    // A color written here without a group lands in *all three* groups — and, being a binding, it settles after the
    // groups' own bindings and overwrites them (measured: with `windowText` set both group-less and under `disabled`,
    // the disabled group kept the group-less color). So a role either never changes and is written once, or it changes
    // and is written out in every group. Never both.
    palette {
        // Same whatever state a control is in.
        window: Theme.bgBase
        base: Theme.bgBase
        button: Theme.bgElevated
        placeholderText: Theme.textMuted
        // The two roles Fusion paints a scroll bar's thumb with, and in this window they paint nothing else: every
        // other place the style reaches for `mid` (a Popup's border, a Dialog's, a SplitView's handle) is a background
        // this app supplies itself (`AppCardFace` / `SplitHandleBar`). So they are the thumb's own colors, and they
        // are translucent — the graph's rows run under the bar (デザイン規約 §スクロールバー).
        mid: Theme.scrollBarThumb
        dark: Theme.scrollBarThumbHeld // Fusion: a scroll bar's held handle
        light: Theme.borderDefault

        active {
            windowText: Theme.textPrimary
            text: Theme.textPrimary
            buttonText: Theme.textPrimary
            brightText: Theme.textOnAccent
            highlight: Theme.accent
            highlightedText: Theme.textOnAccent
        }
        // Losing the window's focus is not a state worth showing: the same colors as active.
        inactive {
            windowText: Theme.textPrimary
            text: Theme.textPrimary
            buttonText: Theme.textPrimary
            brightText: Theme.textOnAccent
            highlight: Theme.accent
            highlightedText: Theme.textOnAccent
        }
        // What cannot be pressed says so: labels drop to the muted text, and the accent face of a highlighted button
        // (Commit, Save) drops with them so it stops reading as the one thing to press.
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

    // Being on screen — not being focused — drives the periodic re-read: a window that only catches up when clicked
    // hides exactly what it is kept open to show.
    readonly property bool onScreen: root.visible
                                     && root.visibility !== Window.Minimized && root.visibility !== Window.Hidden

    FocusRelease {
        window: root
        // The left menu's name box is held open by nothing but the keyboard it took, so the press that took the
        // keyboard away is what walks away from it (デザイン規約 §左メニューの所作). Only the tab on screen has one: a
        // box in a tab nobody is looking at was not what the press landed away from.
        onPressedAway: scenePos => {
            if (root.curPage !== null)
                root.curPage.releasePressedAway(scenePos)
        }
    }

    // Identity: the dialog and the state that opens it live in the gate below (`IdentityGate`). The way in keeps its
    // name on the window — the harness calls it here (`WindowAutoActDriver`).
    function dismissIdentity() {
        identityGate.dismissIdentity()
    }

    Shortcut {
        sequence: "F5"
        enabled: root.curPage !== null
        onActivated: root.curPage.pageTab.refreshAll()
    }

    Shortcut {
        // The plural: some platforms give "find" more than one key, and `sequence` takes only the first and says so.
        sequences: [StandardKey.Find]
        enabled: root.curPage !== null
        onActivated: root.curPage.startFind()
    }

    // Frame counter for the scroll benchmark (PG_AUTO_SCROLL=1); the page's bench reads it through Window.window.
    property int frameCounter: 0
    onFrameSwapped: frameCounter++
    function claimAutoPageAct() { return autoShotDriver.claimPageAct() }
    function finishAutoAct() { autoShotDriver.finish() }

    WindowPerfDriver {
        window: root
        page: root.curPage
    }

    TabsModel {
        id: tabsModel
        onOpenRejected: (path, kind, message, near) =>
            openFailedDialog.show(path, kind, message, near)
    }

    // Only the picker's own answers come here: somebody there is still choosing a folder, so the way on is the picker
    // again. Every other way a repository fails to open (a restored tab, a worktree row, PG_AUTO_OPEN) keeps its tab
    // and its page-sized failure screen — a modal on startup is answered before it can be read.
    OpenFailedDialog {
        id: openFailedDialog
        onChooseAnother: near => root.openRepositoryPicker(near)
    }

    // The RepoPage of the active tab (the toolbar's right-side controls act on it). Only that tab has one
    // (`RepoPageStack`).
    readonly property var curPage: pages.curPage

    FolderDialog {
        id: folderDialog
        title: qsTr("Open repository")
        onAccepted: tabsModel.openRepositoryUrl(selectedFolder.toString())
    }

    // Every way in goes through here so the picker opens beside the repository that is already open — left to itself
    // the dialog reopens inside the folder it last accepted, and the next pick is always a level up from there.
    //
    // `nearUrl` names a folder to start at instead: a second try after a folder that was not a repository opens where
    // that one sits, which is where the one being looked for usually is.
    function openRepositoryPicker(nearUrl) {
        const near = (nearUrl !== undefined && nearUrl !== "")
                   ? nearUrl
                   : (root.curPage !== null ? root.curPage.pageTab.pickerFolderUrl : "")
        if (near !== "")
            folderDialog.currentFolder = near
        folderDialog.open()
        if (AppBackend.autoAct !== "" && AppBackend.autoAct !== "open-picker")
            AppBackend.report("picker folder=" + folderDialog.currentFolder)
    }
    // ---- smoke hooks -----------------------------------------------
    // The whole of the window's PG_AUTO_ACT harness, built only when a verb was given so an ordinary run carries none
    // of it. A file of its own cannot see this one's ids, so everything the verbs act on is handed over here — an
    // automation-only exposure, the same one `GraphPane.view` is (app-ui.md).
    Loader {
        id: autoActLoader
        active: AppBackend.autoAct !== ""
        sourceComponent: WindowAutoActDriver {
            window: root
            tabsModel: tabsModel
            pageRepeater: pages.seats
            topBar: topBar
            chrome: chrome
            mainUi: mainUi
            gate: gate
            openFailedDialog: openFailedDialog
            identityDialog: identityGate.dialog
            folderDialog: folderDialog
            settingsDialog: settingsDialog
        }
    }

    // Closing is the last chance: the timer will not come round again.
    onClosing: {
        if (!AppBackend.alreadyRunning)
            root.reportState()
    }

    SharedToolTip {
        id: sharedToolTip
        host: mainUi
        hand: hand
        // A run has no pointer to rest anywhere, and a tip that opens beside the hand has to be told where one would
        // have been. A quarter across the target: far enough off its middle that the picture says which of the two the
        // seat was read from (`SharedToolTip.handAcross`).
        handAcross: AppBackend.autoAct !== "" ? 0.25 : -1
    }

    Component.onCompleted: {
        sharedToolTip.dressToolTip()
        chrome.decorateWindow()
        // A window that is only here to say another process has the files does none of the rest.
        if (!AppBackend.alreadyRunning) {
            AppBackend.initialize()
            windowShape.applySavedWindow()
            if (AppBackend.autoOpen !== "") {
                // ';'-separated repositories open as tabs in order.
                const paths = AppBackend.autoOpen.split(";")
                for (let i = 0; i < paths.length; i++) {
                    if (paths[i] !== "")
                        tabsModel.openRepositoryPath(paths[i])
                }
            } else {
                tabsModel.restoreTabs()
            }
        }
        if (autoActLoader.item)
            autoActLoader.item.begin()
        autoShotDriver.begin()
    }
    // Popups (dialogs, menus) render in the window overlay, whose C++-created items grabToImage refuses ("no QML
    // engine"). This QML-declared mirror of the overlay is grabbable, which is what makes popups photographable on the
    // offscreen platform, where no OS window exists to shoot from outside. Loaded only while a shot directory is set,
    // and refreshed once at shot time: AutoShotDriver.
    Loader {
        id: overlayMirror
        active: AppBackend.shotDir !== ""
        anchors.fill: parent
        z: -10000
        sourceComponent: ShaderEffectSource {
            sourceItem: root.Overlay.overlay
            live: false
        }
    }
    // The two laid over each other, for the one question a pair of pictures cannot answer: *where* the thing that
    // opened stands against the thing it opened off. app.png has the row and no tooltip; overlay.png has the tooltip
    // over nothing. Neither says whether the hand could walk from one to the other, which is the whole of what a hover
    // seat is judged on.
    //
    // Two sources rather than one of the window's root item: this mirror is inside the content, so a mirror of
    // everything would be a mirror of itself. Refreshed only when the overlay was holding something — with nothing open
    // this picture is app.png again, and a board of doubles is a board nobody reads.
    Loader {
        id: sceneMirror
        active: AppBackend.shotDir !== ""
        anchors.fill: parent
        z: -10001
        sourceComponent: Item {
            id: sceneShot
            /// How many of the two textures are still owed. The refresh is asked for once the app's own picture is
            /// saved, so what is mirrored here is the scene that picture came out of — ink and all (`AutoShotDriver`).
            property int owed: 0
            signal ready()
            function refresh() {
                sceneShot.owed = 2
                uiSource.scheduleUpdate()
                overlaySource.scheduleUpdate()
            }
            function landed() {
                sceneShot.owed--
                if (sceneShot.owed === 0)
                    sceneShot.ready()
            }
            ShaderEffectSource {
                id: uiSource
                anchors.fill: parent
                live: false
                sourceItem: gate.visible ? gate : mainUi
                onScheduledUpdateCompleted: sceneShot.landed()
            }
            ShaderEffectSource {
                id: overlaySource
                anchors.fill: parent
                live: false
                sourceItem: root.Overlay.overlay
                onScheduledUpdateCompleted: sceneShot.landed()
            }
        }
    }
    AutoShotDriver {
        id: autoShotDriver
        window: root
        overlayMirror: overlayMirror
        sceneMirror: sceneMirror
        mainUi: mainUi
        gate: gate
    }

    // ---- the two ways the window has nothing to show ---------------------
    // No git to ask, or another process already has the files. The id stays here: the shot driver and the window's
    // harness both reach for it by name (`AutoShotDriver.gate`).
    StartupGate {
        id: gate
        anchors.fill: parent
        onCloseRequested: root.close()
    }

    // ---- identity dialog -------------------------------------------------
    IdentityGate {
        id: identityGate
        anchors.fill: parent
    }

    // ---- settings --------------------------------------------------------
    SettingsDialog {
        id: settingsDialog
        curPage: root.curPage
    }

    // The window's own edge: a frameless window has no non-client area for the platform to put a line around, so the
    // line the design asks for (規約 §ウィンドウの縁) is drawn in the client. Not while the window fills the screen — a line
    // there would separate the app from nothing. The bottom side is the floor rectangle's at the end of the column
    // below; windowed, the two coincide and the four sides read as one outline.
    Rectangle {
        anchors.fill: parent
        anchors.topMargin: -root.contentItem.y
        z: 9999
        visible: root.captionMerged && root.visibility !== Window.Maximized
        color: "transparent"
        border.width: Theme.borderWidth
        border.color: Theme.borderDefault
    }

    // ---- main ------------------------------------------------------------
    ColumnLayout {
        id: mainUi
        // ApplicationWindow keeps its content item inside the window's safe area, which with the client area expanded
        // starts below the title bar (measured on Windows: y = 31). The chrome reaches back up over that inset with a
        // negative top margin — the content item does not clip, so both painting and input carry. Not by reparenting
        // onto the window's root item: content outside the content item never wakes the render loop, so every change
        // waited for the next input event to be painted (measured: "the first click did nothing").
        anchors.fill: parent
        anchors.topMargin: -root.contentItem.y
        spacing: 0
        visible: AppBackend.gitState === "ok"

        // Where the hand is, for the shared tooltip to open beside. Declared here, on the parent of the whole content,
        // because that is the one place a window-wide handler costs the rows nothing (`PointerWatch`).
        PointerWatch {
            id: hand
        }

        TopBar {
            id: topBar
            Layout.fillWidth: true
            tabsModel: tabsModel
            curPage: root.curPage
            captionMerged: root.captionMerged
            windowMaximized: root.visibility === Window.Maximized
            // Standing on the floor is the one width with nothing left to share out, and the band's state group gives
            // up its words there (`TopBar.windowAtFloor`). Read here because the floor is the larger of the band's and
            // the page's.
            windowAtFloor: root.width <= Math.ceil(root.floorWidth)
            // …and the width the three actions have to be down to their marks by, which is that floor with the list
            // open (2026-08-25 ユーザー指示). Read here for the same reason: only this window has both halves of it.
            windowFloorWidth: root.openFloorWidth
            onOpenRepositoryRequested: root.openRepositoryPicker()
            onIdentityEditRequested: identityGate.identityEditing = true
            onSettingsRequested: settingsDialog.open()
            onMaximizeToggleRequested: chrome.toggleMaximized()
            onMinimizeRequested: chrome.minimizeWindow()
            onCloseRequested: root.close()
            onCaptionStripMoved: chrome.reportCaptionStrip()
        }
        // Divider under the tab toolbar — same look as the pane splitters.
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.splitterWidth
            color: Theme.borderSubtle
        }

        // Nothing open: the blank page. Built only while needed, so an app that starts with tabs never pays for it.
        Loader {
            id: blankPage
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

        // Repository pages — one row per tab, and a page only for the tab in front (`RepoPageStack`).
        RepoPageStack {
            id: pages
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: tabsModel.currentIndex >= 0
            tabsModel: tabsModel
            pageBand: topBar
            focusEpoch: root.focusEpoch
            onScreen: root.onScreen
            onOpenRepositoryPicker: root.openRepositoryPicker()
            onSettingsDialogRequested: settingsDialog.open()
            // The same card, told whom it was opened on before it opens (デザイン規約 §アバターを与える).
            onAvatarSettingsRequested: (name, email) => {
                settingsDialog.prefillName = name
                settingsDialog.prefillEmail = email
                settingsDialog.open()
            }
        }

        // The window's floor, and — while the edge above is drawn — its bottom side as well: one line in borderDefault
        // doing both. Unlike the edge, drawn while the window fills the screen too: this side still has the taskbar
        // under it.
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.borderWidth
            color: Theme.borderDefault
        }
    }
}
