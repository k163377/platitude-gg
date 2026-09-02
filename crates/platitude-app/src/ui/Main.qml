pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Dialogs
import QtQuick.Window
import platitude
import platitude.ui

ApplicationWindow {
    id: root
    width: 1440
    height: 900
    visible: true
    /// Whether the tab row is the window's title bar. Decided by platform, not read back from the hints: hints are read
    /// when the window is created, and a window that came up without a way to close it cannot be taken back. Only
    /// Windows is known to work; everywhere else keeps the platform's own title bar above an ordinary tab row
    /// (P3-確認事項 §ウィンドウ chrome). `PG_PLAIN_CHROME=1` asks for the other shape from here.
    readonly property bool captionMerged: Qt.platform.os === "windows" && !AppBackend.plainChrome

    // No frame at all: an expanded client area over the caption still leaves a real non-client frame — inflated past
    // the screen when maximised, a white pixel no DWM attribute moves, a remembered size coming back too wide.
    // `FramelessWindowHint` deletes the whole area; the edge is drawn in the scene (below), and the resize edges and
    // grab run stay with the subclass (`winframe::take_frame_hit_test` — Qt 6.10's own custom-chrome answer synthesises
    // input from a poll and loses track of it: the dead first click and the frozen hover). `keepWindowGestures` puts
    // back the system's minimise/maximise/menu style bits, and the subclass pins a maximise to the work area
    // (`clamp_maximized`), which Windows would otherwise take to the whole monitor. No shadow: the drawn edge stands in.
    flags: root.captionMerged ? (Qt.Window | Qt.FramelessWindowHint) : Qt.Window
    title: Words.appName
    color: Theme.bgBase

    // ---- the floor the window may not be dragged under --------------------
    // Past the minimums, `SplitView` and the layouts here lay items out over their own edge, and nothing scrolls
    // sideways to reach what went over. Read off what is on screen, because the list folding and the command log
    // opening both move it; Qt grows a window when a floor rises under it, which is the way back from
    // fold → shrink → unfold. `minimumWidth` alone only covers a dragged edge — `QWindow::resize` hands the size
    // straight to the platform without reading the hints — so every size this application sets itself goes through
    // `holdFloor` below. The numbers themselves are measured where both halves of the floor are (`WindowBody`); the
    // names stay on the window, which is where the harness and `WindowShape` read them.
    readonly property alias floorPage: body.floorPage
    readonly property alias floorWidth: body.floorWidth
    readonly property alias openFloorWidth: body.openFloorWidth
    readonly property alias floorHeight: body.floorHeight
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

    // What a title bar does, now that this band is one: maximise, minimise, the grab-run's whereabouts and the
    // window's dressing (`WindowChrome`). Only this file and the body it seats call them, so the names live on the
    // chrome.
    WindowChrome {
        id: chrome
        window: root
        topBar: body.topBar
    }
    onWidthChanged: chrome.reportCaptionStrip()

    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd

    // A color written here without a group lands in *all three* groups — and, being a binding, it settles after the
    // groups' own bindings and overwrites them. So a role either never changes and is written once, or it changes and
    // is written out in every group. Never both.
    palette {
        // Same whatever state a control is in.
        window: Theme.bgBase
        base: Theme.bgBase
        button: Theme.bgElevated
        placeholderText: Theme.textMuted
        // The two roles Fusion paints a scroll bar's thumb with, and in this window they paint nothing else: every
        // other place the style reaches for `mid` (a Popup's border, a Dialog's, a SplitView's handle) is a background
        // this app supplies itself. So they are the thumb's own colors, and they are translucent — the graph's rows
        // run under the bar (デザイン規約 §スクロールバー).
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
        // keyboard away is what walks away from it (デザイン規約 §左メニューの所作). Only the tab on screen has one.
        onPressedAway: scenePos => {
            if (root.curPage !== null)
                root.curPage.releasePressedAway(scenePos)
        }
    }

    /// Automation (`PG_AUTO_ACT=replay-running`): the hand stood at `x, y`, and what the mark beside it makes of
    /// that (`WindowWaitRing`). The two keep their names here — the harness calls them on the window.
    function holdWaitHand(x, y) {
        body.waitRing.holdWaitHand(x, y)
    }
    readonly property bool waitRingShown: body.waitRing.ringShown

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
    function claimAutoPageAct() {
        const acts = harness.ask()
        return acts !== null && acts.claimPageAct()
    }
    function finishAutoAct() {
        quitGate.yieldToRun()
        const acts = harness.ask()
        if (acts !== null)
            acts.finish()
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

    WindowQuitGate {
        id: quitGate
        window: root
        anchors.fill: parent
    }

    // The RepoPage of the active tab (the toolbar's right-side controls act on it). Only that tab has one
    // (`RepoPageStack`).
    readonly property alias curPage: body.curPage

    FolderDialog {
        id: folderDialog
        title: qsTr("Open repository")
        onAccepted: tabsModel.openRepositoryUrl(selectedFolder.toString())
    }

    // The settings screen and the clone box, with the model and folder chooser the clone owns — one seat, filling
    // the window because it is the popups' parent (`WindowDialogSeat`).
    WindowDialogSeat {
        id: dialogSeat
        anchors.fill: parent
        curPage: root.curPage
        tabsModel: tabsModel
    }
    function startClone() {
        dialogSeat.startClone()
    }

    // Every way in goes through here so the picker opens beside the repository that is already open — left to itself
    // the dialog reopens inside the folder it last accepted, and the next pick is always a level up from there.
    // `nearUrl` names a folder to start at instead: a second try after a folder that was not a repository opens where
    // that one sits, which is where the one being looked for usually is.
    function openRepositoryPicker(nearUrl) {
        const near = (nearUrl !== undefined && nearUrl !== "")
                   ? nearUrl
                   : (root.curPage !== null ? root.curPage.pageTab.pickerFolderUrl : "")
        if (near !== "")
            folderDialog.currentFolder = near
        folderDialog.open()
        root.pickerOpened()
    }
    /// Automation: the platform's folder chooser was put up, and where it was pointed. An automation-only exposure,
    /// the same one `GraphPane.view` is (app-ui.md) — the box belongs to the platform, so nothing on this side of it
    /// says afterwards that it opened.
    signal pickerOpened()
    // ---- the harness -----------------------------------------------
    // The whole of the window's verification harness, which a shipped build does not carry (`HarnessSeat`): the verbs,
    // the measurements, and the stand-ins a headless shot is taken from. Everything they act on is handed over here —
    // a module of its own cannot see this one's ids — and the `z` is for the stand-ins, which have to sit behind
    // everything the window draws.
    HarnessSeat {
        id: harness
        anchors.fill: parent
        z: -10000
        part: "WindowHarness.qml"
        wanted: AppBackend.automated
        seats: ({
            window: root,
            tabsModel: tabsModel,
            pageRepeater: body.pageRepeater,
            topBar: body.topBar,
            chrome: chrome,
            mainUi: body,
            gate: gate,
            windowShape: windowShape,
            sharedToolTip: sharedToolTip,
            openFailedDialog: openFailedDialog,
            identityGate: identityGate,
            identityDialog: identityGate.dialog,
            // The seat rather than the two screens in it: they are built on being asked for, and asking is the
            // harness's to do (`WindowDialogSeat.keepBuilt`).
            dialogSeat: dialogSeat,
            folderDialog: folderDialog,
            quitWaitDialog: quitGate.dialog
        })
    }
    // The tab in front is the one seat that moves under the harness, so it is written rather than handed over once.
    Binding {
        target: harness.driver
        property: "page"
        value: root.curPage
        when: harness.driver !== null
    }

    // While git is still writing, the close is put off rather than taken (`WindowQuitGate`); a close that passes
    // is the last chance — the timer will not come round again — so the state is reported only then.
    onClosing: close => {
        quitGate.gateClose(close)
        if (close.accepted && !AppBackend.alreadyRunning)
            root.reportState()
    }

    SharedToolTip {
        id: sharedToolTip
        host: body
        hand: body.hand
    }

    /// Something other than the strip's own memory is opening this window's tabs, so the ones that were left stay
    /// where they are. Answered on [`startingTabs`], which is the one moment it can be claimed in.
    property bool tabsClaimed: false
    /// The strip is ready for its tabs, and nothing has been put in it yet.
    signal startingTabs()

    Component.onCompleted: {
        // Asked for first, because what it seeds — the shape the window comes up at, the tabs the strip opens with —
        // is read further down this same handler (`HarnessSeat`).
        const acts = harness.ask()
        sharedToolTip.dressToolTip()
        chrome.decorateWindow()
        // A window that is only here to say another process has the files does none of the rest.
        if (!AppBackend.alreadyRunning) {
            AppBackend.initialize()
            windowShape.applySavedWindow()
            root.startingTabs()
            if (!root.tabsClaimed)
                tabsModel.restoreTabs()
        }
        if (acts !== null)
            acts.begin()
    }

    // The two ways the window has nothing to show: no git to ask, or another process already has the files. The id
    // stays here — the shot driver and the window's harness both reach for it by name (`AutoShotDriver.gate`).
    StartupGate {
        id: gate
        anchors.fill: parent
        onCloseRequested: root.close()
    }

    IdentityGate {
        id: identityGate
        anchors.fill: parent
        settingsOpen: dialogSeat.settingsOpened
        onSettingsAtGitRequested: dialogSeat.openSettingsAt("git")
    }

    // The window's own edge: a frameless window has no non-client area for the platform to put a line around, so the
    // line the design asks for (規約 §ウィンドウの縁) is drawn in the client. Not while the window fills the screen — a
    // line there would separate the app from nothing. The bottom side is the floor rectangle's at the end of the
    // body's column; windowed, the two coincide and the four sides read as one outline.
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
    // Everything the window shows, and the floor it stands on (`WindowBody`). What the rows in there reach back for
    // is handed over here — a module of its own cannot see this one's ids.
    WindowBody {
        id: body
        // ApplicationWindow keeps its content item inside the window's safe area, which with the client area expanded
        // starts below the title bar (Windows: y = 31). The chrome reaches back up over that inset with a negative top
        // margin — the content item does not clip, so both painting and input carry. Not by reparenting onto the
        // window's root item: content outside the content item never wakes the render loop, so every change waited for
        // the next input event to be painted.
        anchors.fill: parent
        anchors.topMargin: -root.contentItem.y
        visible: AppBackend.gitState === "ok"
        window: root
        chrome: chrome
        dialogSeat: dialogSeat
        tabsModel: tabsModel
    }
}
