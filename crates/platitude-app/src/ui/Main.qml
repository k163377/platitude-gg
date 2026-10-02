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
    /// Whether the tab row is the window's title bar: fixed at creation, and named by platform — hints are read then,
    /// and a window that came up without a way to close it cannot be taken back
    /// (デザイン規約 §ウィンドウの縁「兼ねるかはプラットフォームを名指しで決める」).
    readonly property bool captionMerged: Qt.platform.os === "windows" && !AppBackend.systemTitleBar

    // Frameless: an expanded client area still leaves a real non-client frame (past the screen when maximised, a
    // white pixel no DWM attribute moves). The resize edges and grab run stay with the subclass
    // (`winframe::take_frame_hit_test`) — Qt's own custom chrome synthesises input from a poll and loses track of it.
    // `keepWindowGestures` puts back the system's minimise/maximise/menu style bits, and `clamp_maximized` pins a
    // maximise to the work area, which Windows would otherwise take to the whole monitor.
    flags: root.captionMerged ? (Qt.Window | Qt.FramelessWindowHint) : Qt.Window
    title: Words.appName
    color: Theme.bgBase
    // Every tip waits only the delay its site names (`Metrics.tipDelayMs`) and stays until its site lets it go: Qt's
    // automatic policy would add the platform's wake-up delay and a timeout to each. A popup inherits nothing from the
    // window, so the popup bases say it again (rules-refs/app-ui.md「ToolTip.policy」).
    ToolTip.policy: ToolTip.Manual

    /// Whether this app draws the window's edge: a frameless window has no frame for the platform to draw one on, and
    /// a maximised one's edge is the screen's (デザイン規約 §ウィンドウの縁).
    readonly property bool edgeDrawn: root.captionMerged && root.visibility !== Window.Maximized
    /// How far the body stands in from the left and right edges, so the edge runs beside the band's end cells (☰ / ✕)
    /// rather than over their outer column. Sideways only: the band's top row stays under the top edge
    /// (規約 §ウィンドウの縁「縁は中身を食わない — 左右だけ」).
    readonly property int edgeInset: root.edgeDrawn ? Theme.borderWidth : 0
    // The grab runs are reported in scene coordinates, which the inset moves without moving them inside the band.
    onEdgeInsetChanged: chrome.reportCaptionStrip()

    // ---- the floor the window may not be dragged under --------------------
    // Past the minimums, `SplitView` and the layouts lay items out over their own edge (デザイン規約 §窓の床).
    // `minimumWidth` only covers a dragged edge and a rising floor — `QWindow::resize` hands a size straight to the
    // platform without reading the hints — so every size this application sets goes through `holdFloor`. Measured in
    // `WindowBody`; named here, where the harness and `WindowShape` read them. The width's floors take in the edge's
    // two columns (`edgeInset`).
    readonly property alias floorPage: body.floorPage
    readonly property real floorWidth: body.floorWidth + 2 * root.edgeInset
    readonly property real openFloorWidth: body.openFloorWidth + 2 * root.edgeInset
    readonly property alias floorHeight: body.floorHeight
    minimumWidth: Math.ceil(root.floorWidth)
    minimumHeight: Math.ceil(root.floorHeight)
    onFloorWidthChanged: windowShape.holdFloor()
    onFloorHeightChanged: windowShape.holdFloor()

    // `reportState()` stays on the window: the harness calls it there.
    WindowShape {
        id: windowShape
        window: root
    }
    function reportState() {
        windowShape.reportState()
    }

    // Only this file and the body call it, so the names live on the chrome.
    WindowChrome {
        id: chrome
        window: root
        topBar: body.topBar
    }
    onWidthChanged: chrome.reportCaptionStrip()

    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd

    // A color written here without a group lands in all three groups — and, being a binding, settles after the
    // groups' own bindings and overwrites them. So a role either never changes and is written once, or is written
    // out in every group.
    palette {
        window: Theme.bgBase
        base: Theme.bgBase
        button: Theme.bgElevated
        placeholderText: Theme.textMuted
        // The two roles Fusion paints a scroll bar's thumb with; everywhere else the style reaches for `mid` (Popup and
        // Dialog borders, the SplitView handle) this app draws its own background (デザイン規約 §スクロールバー).
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
        // As active: the window losing focus keeps its look.
        inactive {
            windowText: Theme.textPrimary
            text: Theme.textPrimary
            buttonText: Theme.textPrimary
            brightText: Theme.textOnAccent
            highlight: Theme.accent
            highlightedText: Theme.textOnAccent
        }
        // A highlighted button's accent face (Commit, Save) drops too, or it still reads as the thing to press.
        disabled {
            windowText: Theme.textMuted
            text: Theme.textMuted
            buttonText: Theme.textMuted
            brightText: Theme.textMuted
            highlight: Theme.accentMuted
            highlightedText: Theme.textMuted
        }
    }

    // Window focus is a refresh trigger (refs/status/stash only). Only a window coming back counts: the first
    // activation lands while the repository is first being read (`RepoSession::open`), and counting it queues a second
    // whole `git status` behind that one (`session::ReadFlight`).
    property int focusEpoch: 0
    /// Read off the losing of focus, so a window already active before this handler exists still counts its first
    /// real return.
    property bool everAway: false
    onActiveChanged: {
        if (!root.active)
            root.everAway = true
        else if (root.everAway)
            root.focusEpoch++
    }

    // Drives the periodic re-read.
    readonly property bool onScreen: root.visible
                                     && root.visibility !== Window.Minimized && root.visibility !== Window.Hidden

    /// Automation only (rules-refs/app-ui.md「自動化フック専用の露出」): a run presses through `caretHand.pressedAt`,
    /// the road the hand's own press takes (`PGG_AUTO_ACT=edit-message-away`).
    readonly property alias caretHand: caretHand
    FocusRelease {
        id: caretHand
        window: root
        // Over the safe-area inset the chrome also covers, so presses on the band are heard too.
        reachUp: root.contentItem.y
        // So an Escape after the caret was walked away from still reaches the page's handler.
        home: root.curPage
        // The left menu's name box is held open by nothing but the keyboard it took, so the press that took the
        // keyboard away walks away from it (デザイン規約 §左メニューの所作).
        onPressedAway: scenePos => {
            if (root.curPage !== null)
                root.curPage.releasePressedAway(scenePos)
        }
        onPressedAnywhere: root.pressLandedAnywhere()
    }
    /// A press landed, whatever it was on; a run enters the same road. What the window put up for the reader stands
    /// until their next press (デザイン規約 §hover のツールチップ).
    function pressLandedAnywhere() {
        MiddleHand.pressLanded()
        body.topBar.pressLanded()
        if (root.curPage !== null)
            root.curPage.notePress()
    }

    /// Automation (`PGG_AUTO_ACT=replay-running`), on the window because the harness calls it here.
    function holdWaitHand(x, y) {
        body.waitRing.holdWaitHand(x, y)
    }
    readonly property bool waitRingShown: body.waitRing.ringShown

    // On the window because the harness calls it here (`WindowAutoActDriver`).
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

    // Frames swapped, for automation (`WindowPerfDriver`, and page verbs through `Window.window`).
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
    // What the page's run has its write barrier holding, for the line a stalled run leaves: the ceiling lives in the
    // window's harness seat, out of the page's reach.
    function noteAutoActWrite(state) {
        const acts = harness.ask()
        if (acts !== null)
            acts.noteWriteState(state)
    }

    TabsModel {
        id: tabsModel
        onOpenRejected: (path, kind, message, near) =>
            openFailedDialog.show(path, kind, message, near)
    }

    // Only the picker's own failures come here, and the way on is the picker again. Every other failed open (a
    // restored tab, a worktree row, PGG_AUTO_OPEN) keeps its tab and its failure screen — a modal on startup is
    // answered before it can be read.
    OpenFailedDialog {
        id: openFailedDialog
        onChooseAnother: near => root.openRepositoryPicker(near)
    }

    WindowQuitGate {
        id: quitGate
        window: root
        anchors.fill: parent
    }

    // A different git was chosen: this window closes, and `main` starts the successor once the settings files are let
    // go of (規約 §設定の画面). Read here because `root.close()` is the one road out — the close gate that waits a
    // running write out stands on it.
    Connections {
        target: AppBackend
        function onRestartWantedChanged() {
            if (AppBackend.restartWanted)
                root.close()
        }
    }

    // The RepoPage of the active tab — the only tab that has one (`RepoPageStack`).
    readonly property alias curPage: body.curPage

    FolderDialog {
        id: folderDialog
        title: qsTr("Open repository")
        onAccepted: tabsModel.openRepositoryUrl(selectedFolder.toString())
    }

    // The settings screen and the clone box — filling the window because it is the popups' parent.
    WindowDialogSeat {
        id: dialogSeat
        anchors.fill: parent
        curPage: root.curPage
        tabsModel: tabsModel
    }
    function startClone() {
        dialogSeat.startClone()
    }

    // Every way in goes through here so the picker opens beside the open repository — left to itself the dialog
    // reopens inside the folder it last accepted. `nearUrl` overrides: a retry opens beside the refused folder.
    function openRepositoryPicker(nearUrl) {
        const near = (nearUrl !== undefined && nearUrl !== "")
                   ? nearUrl
                   : (root.curPage !== null ? root.curPage.pageTab.pickerFolderUrl : "")
        if (near !== "")
            folderDialog.currentFolder = near
        folderDialog.open()
        root.pickerOpened()
    }
    /// Automation only: the platform's folder chooser was put up — nothing on this side can say so afterwards.
    signal pickerOpened()
    // ---- the harness -----------------------------------------------
    // Absent from a shipped build. Everything the verbs act on is handed over here — a module of its own cannot see
    // this one's ids — and the `z` puts the headless shot's stand-ins behind everything the window draws.
    HarnessSeat {
        id: harness
        anchors.fill: parent
        z: -10000
        part: "WindowHarness.qml"
        wanted: AppBackend.harnessPresent
        seats: ({
            window: root,
            tabsModel: tabsModel,
            pageRepeater: body.pageRepeater,
            topBar: body.topBar,
            chrome: chrome,
            mainUi: body,
            gate: gate,
            windowEdge: windowEdge,
            windowShape: windowShape,
            sharedToolTip: sharedToolTip,
            openFailedDialog: openFailedDialog,
            identityGate: identityGate,
            identityDialog: identityGate.dialog,
            // The seat, not its screens: they are built on being asked for (`WindowDialogSeat.keepBuilt`).
            dialogSeat: dialogSeat,
            folderDialog: folderDialog,
            quitWaitDialog: quitGate.dialog
        })
    }
    // The tab in front is the one seat that moves under the harness.
    Binding {
        target: harness.driver
        property: "page"
        value: root.curPage
        when: harness.driver !== null
    }

    // A close that passes the gate is the last chance to report the state: the timer will not come round again.
    onClosing: close => {
        quitGate.gateClose(close)
        if (close.accepted && !AppBackend.alreadyRunning)
            root.reportState()
    }

    SharedToolTip {
        id: sharedToolTip
        host: body
        hand: body.hand
        // One tip for the whole window, so a press inside it is the page's to answer (`RepoPage.tipLinkAsked`).
        onLinkAsked: href => {
            if (root.curPage !== null)
                root.curPage.tipLinkAsked(href)
        }
    }
    /// Automation (`PGG_AUTO_ACT=fetch-tip-link`): presses the tip's link word at the tip's own signal — the glyphs are
    /// the one part a run cannot press (verify-ui スキル). Answers whether there was a link, so a run cannot pass on a
    /// tip that offered nothing to press.
    function pressTipLink() {
        if (sharedToolTip.tipPlace === "" || sharedToolTip.tipHref === "")
            return false
        sharedToolTip.linkAsked(sharedToolTip.tipHref)
        return true
    }

    /// Something other than the strip's own memory opens this window's tabs, so the saved ones are not restored. Set
    /// only on [`startingTabs`], the one moment it can be claimed in.
    property bool tabsClaimed: false
    /// The strip is ready for its tabs, and nothing has been put in it yet.
    signal startingTabs()

    Component.onCompleted: {
        // First: what it seeds (the window's opening shape, the strip's first tabs) is read further down.
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

    // No git to ask, or another process already has the files. The shot driver and the harness reach for the id by
    // name (`AutoShotDriver.gate`).
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

    // The window's edge (規約 §ウィンドウの縁), windowed only; its bottom side coincides with the floor line at the end
    // of `WindowBody`'s column, which alone stays when maximised.
    Rectangle {
        id: windowEdge
        anchors.fill: parent
        anchors.topMargin: -root.contentItem.y
        z: 9999
        visible: root.edgeDrawn
        color: "transparent"
        border.width: Theme.borderWidth
        border.color: Theme.borderDefault
    }

    // ---- main ------------------------------------------------------------
    WindowBody {
        id: body
        // ApplicationWindow keeps its content item inside the window's safe area; a negative top margin reaches back
        // over that inset, and the content item does not clip. Not reparenting onto the window's root item: content
        // outside the content item never wakes the render loop, so every change waits for the next input.
        anchors.fill: parent
        anchors.topMargin: -root.contentItem.y
        anchors.leftMargin: root.edgeInset
        anchors.rightMargin: root.edgeInset
        visible: AppBackend.gitState === "ok"
        window: root
        chrome: chrome
        dialogSeat: dialogSeat
        tabsModel: tabsModel
        // Under everything the window draws, so every request nothing above took reaches it (`keyMenuAsked`).
        ContextMenu.onRequested: position => root.keyMenuAsked(body.mapToItem(null, position))
    }
    /// Qt's context-menu request that nothing it was handed to took, at `scenePoint`: one with no place — Windows'
    /// Shift+F10 — is answered where the keyboard is (`KeyMenu.answer`); a placed one (a right-click nothing opened a
    /// menu for, xcb's menu key at the pointer) is turned away. Says whether a menu was asked.
    function keyMenuAsked(scenePoint) {
        return KeyMenu.placeless(scenePoint) && KeyMenu.answer(root.activeFocusItem)
    }
}
