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
    /// that has been seen to work; everywhere else keeps the platform's
    /// own title bar above an ordinary tab row (P3-確認事項 §ウィンドウ
    /// chrome). `PG_PLAIN_CHROME=1` asks for the other shape from here.
    readonly property bool captionMerged: Qt.platform.os === "windows"
                                          && !AppBackend.plainChrome

    // No frame at all, rather than a frame asked to behave: an expanded
    // client area over the caption still leaves a real non-client frame,
    // which is inflated past the screen when maximised (measured: it
    // covered eight columns of the next monitor), shows a white pixel no
    // DWM attribute moves, and makes a remembered size come back too
    // wide. `FramelessWindowHint` deletes the whole area — the edge is
    // drawn in the scene (below), and the resize edges and grab run are
    // the subclass's (`winframe::take_frame_hit_test`, which stays: Qt
    // 6.10's own custom-chrome answer synthesises input from a poll and
    // loses track of it — the dead first click and the frozen hover).
    // `keepWindowGestures` puts back the system's minimise/maximise/menu
    // style bits, and the subclass pins a maximise to the work area
    // (`clamp_maximized`) because Windows would otherwise maximise a
    // frameless window over the whole monitor. No shadow is owed: the
    // drawn edge stands in for it.
    flags: root.captionMerged
           ? (Qt.Window | Qt.FramelessWindowHint)
           : Qt.Window
    title: qsTr("Platitude GG")
    color: Theme.bgBase

    // ---- the floor the window may not be dragged under --------------------
    // `SplitView` and the layouts here stop shrinking their items at the
    // minimum and lay the rest out past their own edge, and nothing in
    // this window scrolls sideways to reach what went over (measured
    // 2026-08-09: at 640px the right pane had 32 of its 300 on screen).
    //
    // The number is read off what is on screen because two things move
    // it: the list folding and the command log opening. Qt grows a window
    // when a floor rises under it, which is the way back from
    // fold → shrink → unfold.
    //
    // `minimumWidth` alone only covers a person dragging an edge:
    // `QWindow::resize` hands the size straight to the platform without
    // looking at the hints (measured: asked for 200x150 against the
    // floor, the window took it). So every size this application sets
    // itself goes through `holdFloor` below.
    /// The page the floor is read off. Not `curPage`: with no tab open
    /// that is null while the window is showing the blank page, which has
    /// the same three panes with the same minimums.
    readonly property var floorPage:
        root.curPage !== null ? root.curPage : blankPage.item
    readonly property real floorWidth:
        Math.max(topBar.floorWidth,
                 root.floorPage !== null ? root.floorPage.floorWidth : 0)
    readonly property real floorHeight:
        // The band, the divider under it, and the line the window's bottom
        // edge is drawn as — the three rows of `mainUi` that are not the
        // page (they carry their own heights; the page's is its own floor).
        Theme.toolbarHeight + Theme.splitterWidth + Theme.borderWidth
        + (root.floorPage !== null ? root.floorPage.floorHeight : 0)
    minimumWidth: Math.ceil(root.floorWidth)
    minimumHeight: Math.ceil(root.floorHeight)
    /// Puts a window standing under its floor back on it. Only a window
    /// in its own shape: maximised and minimised ones are the platform's
    /// to size. A size this puts up is a size this asked for, so the
    /// frame slop keeps measuring the difference between the two —
    /// without that the growth itself reads as slop, and every launch
    /// after writes the window down that much smaller (measured: a window
    /// lifted from a 320-wide file stood at 704 and 510 went into the
    /// file).
    function holdFloor() {
        if (root.visibility !== Window.Windowed)
            return
        if (root.width < root.floorWidth) {
            root.width = Math.ceil(root.floorWidth)
            root.askedWidth = root.width
        }
        if (root.height < root.floorHeight) {
            root.height = Math.ceil(root.floorHeight)
            root.askedHeight = root.height
        }
    }
    onFloorWidthChanged: root.holdFloor()
    onFloorHeightChanged: root.holdFloor()

    // ---- what a title bar does, now that this band is one ----------------
    /// The button and the band's double-click have to mean the same
    /// thing, so both go to the platform.
    ///
    /// `visibility` alone does not: Qt maximises a frameless window by
    /// resizing it, and the platform is then holding no maximised state
    /// to put back — the button left the window large while the
    /// double-click, which the platform handles, restored it (reported
    /// 2026-08-09). `visibility` still says what the window *is*, and
    /// still carries the state everywhere the platform has no command of
    /// its own.
    function toggleMaximized() {
        const wanted = root.visibility !== Window.Maximized
        if (root.captionMerged)
            AppBackend.setWindowMaximized(wanted)
        else
            root.visibility = wanted ? Window.Maximized : Window.Windowed
    }
    function minimizeWindow() {
        root.visibility = Window.Minimized
    }
    /// Tells the hit test where the band's grab-run is, in scene
    /// coordinates — the one stretch it answers HTCAPTION for, which is
    /// what makes it drag, snap, maximise on a double-click and open the
    /// window menu, all as the platform's own gestures. Called from the
    /// strip's own layout changes and from the one shift the strip cannot
    /// see: the window resizing, which maximising is.
    function reportCaptionStrip() {
        if (!root.captionMerged || topBar.grabRunItem === null)
            return
        const run = topBar.grabRunItem
        const at = run.mapToItem(null, 0, 0)
        AppBackend.setCaptionStrip(at.x, at.x + run.width, at.y + run.height)
    }
    onWidthChanged: root.reportCaptionStrip()

    /// What Windows has to be told about this window, whatever the window
    /// turns out to be for. A run that was turned away gets a window too,
    /// and an undecorated one would be a second application on the taskbar
    /// wearing the shell's generic icon.
    function decorateWindow() {
        // Windows 11 rounds the window itself and leaves the corner pixels
        // transparent, so the desktop shows through them. The window is up
        // by now (`visible` is set above), which is all the switch needs.
        AppBackend.squareWindowCorners()
        // Without this the window wears the shell's generic icon, in the
        // title bar and on the taskbar button alike.
        AppBackend.setWindowIcon()
        // Asking for no drawn buttons took the system's own gestures with
        // them; this puts those back (see `flags` above).
        if (!root.captionMerged)
            return
        AppBackend.keepWindowGestures()
        // The hit test just installed reads the strip from here on; hand
        // it the shape the band settled into while loading.
        root.reportCaptionStrip()
    }

    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd

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

    // Being on screen — not being focused — drives the periodic re-read:
    // a window that only catches up when clicked hides exactly what it is
    // kept open to show.
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
    //
    // The top margin reaches up over the safe-area inset the same way
    // the chrome itself does, so presses on the band are heard too.
    Item {
        anchors.fill: parent
        anchors.topMargin: -root.contentItem.y
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
    // badge.
    property bool identityDismissed: false
    property bool identityEditing: false
    // A save that only half landed leaves an identity that *is* set, so
    // `missing` on its own takes the screen away at the one moment it has
    // something to say: measured, the state flipped to `ready` on the
    // name that did land and this window closed the dialog out from under
    // the answer. What holds it open is the flag the marks read.
    readonly property bool identityWanted: AppBackend.gitState === "ok"
                                           && (identityEditing
                                               || ((AppBackend.identityState === "missing"
                                                    || AppBackend.identityUnsaved)
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

    Shortcut {
        sequence: "F5"
        enabled: root.curPage !== null
        onActivated: root.curPage.pageTab.refreshAll()
    }

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
    function claimAutoPageAct() { return autoShotDriver.claimPageAct() }
    function finishAutoAct() { autoShotDriver.finish() }

    // Memory breakdown (PG_MEM_REPORT=1). A tick rather than a few chosen
    // moments: the harness samples the process every 100ms and takes the
    // largest reading, so the breakdown has to be a series to be lined up
    // against it at all — one report at the end would name a moment the
    // peak had already passed.
    Timer {
        interval: 500
        repeat: true
        running: AppBackend.memReport
        triggeredOnStart: true
        onTriggered: AppBackend.noteMemory(root.curPage === null ? "idle" : "open")
    }

    TabsModel {
        id: tabsModel
        onOpenRejected: (path, kind, message, near) =>
            openFailedDialog.show(path, kind, message, near)
    }

    // Only the picker's own answers come here: somebody there is still
    // choosing a folder, so the way on is the picker again. Every other
    // way a repository fails to open (a restored tab, a worktree row,
    // PG_AUTO_OPEN) keeps its tab and its page-sized failure screen — a
    // modal on startup is answered before it can be read.
    OpenFailedDialog {
        id: openFailedDialog
        onChooseAnother: near => root.openRepositoryPicker(near)
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
    //
    // `nearUrl` names a folder to start at instead: a second try after a
    // folder that turned out not to be a repository opens where that one
    // sits, which is where the one being looked for usually is.
    function openRepositoryPicker(nearUrl) {
        const near = (nearUrl !== undefined && nearUrl !== "")
                   ? nearUrl
                   : (root.curPage !== null ? root.curPage.pageTab.pickerFolderUrl : "")
        if (near !== "")
            folderDialog.currentFolder = near
        folderDialog.open()
        if (AppBackend.autoAct !== "")
            AppBackend.report("picker folder=" + folderDialog.currentFolder)
    }

    // The size and place the window was left in. Assigned rather than
    // bound: from here on the window manager and the person dragging it
    // own these. An unsaved position stays unset so the platform gets to
    // place the window itself — a first run should not open at 0,0.
    function applySavedWindow() {
        // Over the floor on the way in, not after: what is assigned here
        // is what `settleTimer` measures the frame slop from, and what a
        // maximise would come back to.
        const wantWidth = Math.max(
            root.insideScreen(AppBackend.startWindowWidth(), Screen.width),
            Math.ceil(root.floorWidth))
        const wantHeight = Math.max(
            root.insideScreen(AppBackend.startWindowHeight(), Screen.height),
            Math.ceil(root.floorHeight))
        root.width = wantWidth
        root.height = wantHeight
        root.askedWidth = wantWidth
        root.askedHeight = wantHeight
        const x = AppBackend.startWindowX()
        const y = AppBackend.startWindowY()
        if (x !== root.unplaced && y !== root.unplaced) {
            root.x = x
            root.y = y
        }
        // The *frame* has to fit, and it is wider than the window says it
        // is (measured 2026-08-09: a remembered 1920 came back as a
        // 1936-wide frame at x=-5 on a 1920 screen). `insideScreen`
        // cannot see either number; the platform side moves the window
        // back and says whether it had to. Before the maximise, not
        // after: the shape standing when a window is maximised is the
        // shape a restore comes back to.
        const moved = AppBackend.fitWindowToScreen()
        if (AppBackend.startWindowMaximized()) {
            // Let the platform do it, so the platform holds the shape to
            // come back to (`toggleMaximized`'s story). Where there is no
            // platform command, `visibility` still carries it.
            if (root.captionMerged)
                AppBackend.setWindowMaximized(true)
            else
                root.visibility = Window.Maximized
        } else if (!moved) {
            // Nothing is measured on a run that was moved or maximised:
            // the frame slop is the difference between what the window
            // was handed and what it says it is, and neither is that.
            settleTimer.restart()
        }
    }
    /// What the store sends for a coordinate it has never been told.
    readonly property int unplaced: -2147483648

    /// A remembered length, kept inside the screen the window comes up on.
    /// `Screen.width`, *not* `Screen.desktopAvailableWidth` — that is the
    /// whole virtual desktop (measured on a three-monitor machine: 5760,
    /// so nothing is ever wider than it). Automated runs are exempt: the
    /// offscreen platform reports an 800x800 screen that would cut every
    /// screenshot to fit.
    function insideScreen(saved, screen) {
        return AppBackend.automated ? saved : Math.min(saved, screen)
    }

    /// What this window adds to a size on the way in. It does not read
    /// back the way it is written (measured on the merged chrome: asked
    /// for 1200 it calls itself 1206, so writing down what it says grew
    /// the window 6px on every launch). Qt takes the frame margins from
    /// one place when it sets the geometry and another when it reads it
    /// back, so the difference is read off the window itself and taken
    /// away again on the way out.
    property int widthSlop: 0
    property int heightSlop: 0
    /// The size the window was asked for, which the slop is measured from.
    property int askedWidth: 0
    property int askedHeight: 0

    Timer {
        id: settleTimer
        // One beat, so the window has answered the size it was given: the
        // answer arrives as a queued platform event, not inside the
        // assignment.
        interval: Metrics.anchorDelayMs
        onTriggered: {
            // Only measured against a size this window was just handed,
            // and only while nothing else has had a chance to resize it —
            // a maximise or a snap resizes on the way, and a difference
            // read off that is not a frame margin.
            if (root.askedWidth <= 0 || root.visibility !== Window.Windowed)
                return
            root.widthSlop = root.width - root.askedWidth
            root.heightSlop = root.height - root.askedHeight
        }
    }

    /// Everything the next launch should come back to
    /// (rules-refs/core.md — settings.toml / state.toml).
    function reportState() {
        // A minimised window says nothing. Measured on Windows: while it
        // is down the window reports neither its windowed nor its
        // maximised numbers and its visibility is no longer Maximized —
        // a report from here wrote a window wider than the screen and
        // cleared the flag that would have restored the maximised one.
        if (root.visibility !== Window.Minimized)
            AppBackend.saveWindow(root.x, root.y,
                                  root.width - root.widthSlop,
                                  root.height - root.heightSlop,
                                  root.visibility === Window.Maximized)
        if (root.curPage !== null)
            root.curPage.reportLayout()
        AppBackend.flushState()
    }

    Timer {
        id: stateTimer
        interval: Metrics.stateFlushMs
        repeat: true
        // A run that was turned away holds an empty store, so its reports
        // would reach no file — it does not make them all the same.
        running: !AppBackend.alreadyRunning
        onTriggered: root.reportState()
    }

    // ---- smoke hooks -----------------------------------------------
    // The whole of the window's PG_AUTO_ACT harness, built only when a
    // verb was given so an ordinary run carries none of it. A file of its
    // own cannot see this one's ids, so everything the verbs act on is
    // named here — an automation-only exposure, the same one
    // `GraphPane.view` is (app-ui.md).
    Loader {
        id: autoActLoader
        active: AppBackend.autoAct !== ""
        sourceComponent: WindowAutoActDriver {
            window: root
            tabsModel: tabsModel
            pageRepeater: pageRepeater
            topBar: topBar
            mainUi: mainUi
            gate: gate
            openFailedDialog: openFailedDialog
            identityDialog: identityDialog
        }
    }

    // Closing is the last chance: the timer will not come round again.
    onClosing: {
        if (!AppBackend.alreadyRunning)
            root.reportState()
    }

    /// The shared tooltip — the one popup in this app nobody declares.
    /// The attached property builds it from the style, so it arrives in
    /// Fusion's own clothes: a pale yellow ground, a frame that reads the
    /// *text* role (so the palette cannot separate the two), and a drawn
    /// shadow. Reaching it is the only way to dress it, and dressing it
    /// once reaches every `ToolTip.text` in the tree.
    readonly property var sharedTip: mainUi.ToolTip.toolTip
    /// Puts the app's own card on it (デザイン規約 §背景 names
    /// `bgElevated` as the tooltip's ground).
    function dressToolTip() {
        root.sharedTip.background = tipGround.createObject(root.sharedTip)
        root.sharedTip.contentItem = tipWord.createObject(root.sharedTip)
        root.sharedTip.padding = Theme.spaceSm
    }
    Component {
        id: tipGround
        Rectangle {
            color: Theme.bgElevated
            radius: Theme.radiusMd
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth
        }
    }
    Component {
        id: tipWord
        Text {
            text: root.sharedTip.text
            font: root.sharedTip.font
            color: Theme.textPrimary
            wrapMode: Text.Wrap
        }
    }

    Component.onCompleted: {
        root.dressToolTip()
        root.decorateWindow()
        // A window that is only here to say another process has the files
        // does none of the rest.
        if (!AppBackend.alreadyRunning) {
            AppBackend.initialize()
            root.applySavedWindow()
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
    AutoShotDriver {
        id: autoShotDriver
        window: root
        overlayMirror: overlayMirror
        mainUi: mainUi
        gate: gate
    }

    // ---- the two ways the window has nothing to show ---------------------
    // No git to ask, or another process already has the files. A git
    // older than the supported minimum is not one of them: it answers, so
    // the app runs and wears the band's `OLD GIT` badge instead
    // (規約 §ウィンドウの縁).
    Item {
        id: gate
        anchors.fill: parent
        visible: AppBackend.gitState !== "ok" || AppBackend.alreadyRunning
        // Its own ground rather than the window's, so that a grab of this
        // item is a picture of the screen (`shotTimer`). Nothing under it
        // is drawn while it is up — `mainUi` is hidden — so the colour is
        // the one the window would have shown anyway.
        Rectangle {
            anchors.fill: parent
            color: Theme.bgBase
        }
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
            Label {
                visible: AppBackend.alreadyRunning
                text: qsTr("Platitude GG is already open. Its window is the one to use.")
                wrapMode: Text.Wrap
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
            }
            // Which one, since two builds are alike on screen and the
            // taskbar's launch entry does not say which it started.
            Label {
                visible: AppBackend.alreadyRunning
                text: AppBackend.heldElsewhere
                color: Theme.textSecondary
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontSm
                elide: Text.ElideMiddle
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
            }
            // The way out: the band is the title bar and it is inside the
            // part that stays hidden. A plain frame (規約 §アクセント).
            ActionButton {
                visible: AppBackend.alreadyRunning
                implicitHeight: Theme.controlHeight
                text: qsTr("Close")
                frameColor: Theme.borderDefault
                activeFocusOnTab: true
                anchors.horizontalCenter: parent.horizontalCenter
                onActivated: root.close()
            }
            // The drawn ring, not Fusion's BusyIndicator
            // (規約 §進行中・長押しの定数).
            SpinnerIcon {
                spinning: AppBackend.gitState === "checking"
                          && !AppBackend.alreadyRunning
                width: Theme.iconLg
                height: Theme.iconLg
                anchors.horizontalCenter: parent.horizontalCenter
            }
            Label {
                visible: AppBackend.gitState === "missing"
                text: qsTr("git was not found on PATH. Install git %1 or newer and restart.")
                          .arg(AppBackend.minimumGit)
                wrapMode: Text.Wrap
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
            }
            Label {
                visible: AppBackend.gitState === "error"
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

    // The window's own edge: a frameless window has no non-client area
    // for the platform to put a line around, so the line the design asks
    // for (規約 §ウィンドウの縁) is drawn in the client. Not while the
    // window fills the screen — a line there would separate the app from
    // nothing. The bottom side is the floor rectangle's at the end of the
    // column below; windowed, the two coincide and the four sides read as
    // one outline.
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
        // ApplicationWindow keeps its content item inside the window's
        // safe area, which with the client area expanded starts below the
        // title bar (measured on Windows: y = 31). The chrome reaches
        // back up over that inset with a negative top margin — the
        // content item does not clip, so both painting and input carry.
        // Not by reparenting onto the window's root item: content outside
        // the content item never wakes the render loop, so every change
        // waited for the next input event to be painted — "the first
        // click did nothing" (measured).
        anchors.fill: parent
        anchors.topMargin: -root.contentItem.y
        spacing: 0
        visible: AppBackend.gitState === "ok"

        TopBar {
            id: topBar
            Layout.fillWidth: true
            tabsModel: tabsModel
            curPage: root.curPage
            captionMerged: root.captionMerged
            windowMaximized: root.visibility === Window.Maximized
            // Standing on the floor is the one width with nothing left to
            // share out, and the band's state group gives up its words
            // there (`TopBar.windowAtFloor`). Read here because the floor
            // is the larger of the band's and the page's.
            windowAtFloor: root.width <= Math.ceil(root.floorWidth)
            onOpenRepositoryRequested: root.openRepositoryPicker()
            onIdentityEditRequested: root.identityEditing = true
            onSettingsRequested: settingsDialog.open()
            onMaximizeToggleRequested: root.toggleMaximized()
            onMinimizeRequested: root.minimizeWindow()
            onCloseRequested: root.close()
            onCaptionStripMoved: root.reportCaptionStrip()
        }
        // Smoke hook (PG_AUTO_ACT=middle-close): what is still open
        // afterwards is the report, since a closed tab leaves nothing of
        // itself in the picture.
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

        // Smoke hook (PG_AUTO_ACT=open-again): asking for a repository
        // the strip already holds. No new tab may appear, and `active`
        // has to have moved to the one already holding it (デザイン規約
        // §タブの所作).
        Timer {
            interval: 1200
            running: AppBackend.autoAct === "open-again"
            onTriggered: {
                const asked = AppBackend.autoActArg !== ""
                            ? AppBackend.autoActArg : topBar.tabPathAt(0)
                tabsModel.openRepositoryPath(asked)
                AppBackend.report("open_again tabs=" + pageRepeater.count
                                  + " active=" + tabsModel.currentIndex
                                  + " asked=" + asked
                                  + " open=" + topBar.tabPaths())
            }
        }

        // Smoke hook (PG_AUTO_ACT=force-push-hold): waits out the page's
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

        // Nothing open: the blank page. Built only while it is needed, so
        // an app that starts with tabs never pays for it.
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
                    // opens (デザイン規約 §アバターを与える).
                    onAvatarSettingsRequested: (name, email) => {
                        settingsDialog.prefillName = name
                        settingsDialog.prefillEmail = email
                        settingsDialog.open()
                    }
                    onCloseTabRequested: tabsModel.closeTab(tab_id)
                }
            }
        }

        // The window's floor, and — while the edge above is drawn — the
        // bottom side of it as well: one line in borderDefault doing
        // both. Drawn while the window fills the screen too, unlike the
        // edge: this side still has the taskbar under it.
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.borderWidth
            color: Theme.borderDefault
        }
    }
}
