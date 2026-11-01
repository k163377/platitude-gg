pragma ComponentBehavior: Bound

import QtQuick
// The app's style is Fusion by this import's order alone: nothing names a style (no `QQuickStyle`, no
// `QT_QUICK_CONTROLS_STYLE`, no `qtquickcontrols2.conf`). Imported before anything imports `QtQuick.Controls` bare, it
// is Qt's compile-time pick (qquickstyleplugin.cpp; the import trace says `QtQuick.Controls.Fusion imported before
// QtQuick.Controls; using compile-time style selection`), and the bare imports — `SplitHandleBar`, the dialogs Qt
// draws itself — read Fusion too. Do not let a bare `QtQuick.Controls` be processed first: on a Mac that asks for the
// default macOS style, which the bundle does not carry (`package::contents`, `OTHER_STYLE`).
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
    /// so "try it and see whether it took" is not open, and a window that came up without a way to close it cannot be
    /// taken back. Elsewhere the platform's title bar stands over the chrome's two rows (`TopBar`). The other shape
    /// comes up on this machine with `verify-ui --system-title-bar`, the one headless door: the variable set by hand
    /// never reaches the run — `app_env::clear_automation` drops the parent shell's `PGG_*` from every run xtask
    /// starts.
    readonly property bool captionMerged: Qt.platform.os === "windows" && !AppBackend.systemTitleBar

    // Frameless — window = client area = what is drawn, so maximised it is exactly the work area and not a pixel of the
    // app lands off screen (a framed window's frame runs 8px past every side when maximised, over the next monitor).
    // So the correct inset for it is none, from the start — and QML could not work one out:
    // `Screen.desktopAvailableWidth` spans the virtual desktop, `Screen.width` turns negative with a vertical taskbar,
    // and `root.width` leaves the frame out (`WindowShape.widthSlop`). Headless: `window-fill` (`fills=true`; no
    // picture shows it).
    // An expanded client area still leaves a real non-client frame (past the screen when maximised, a
    // white pixel no DWM attribute moves). The resize edges and grab run stay with the subclass
    // (`winframe::take_frame_hit_test`) — Qt's own custom chrome synthesises input from a poll and loses track of it.
    // `keepWindowGestures` puts back the system's minimise/maximise/menu style bits, and `clamp_maximized` pins a
    // maximise to the work area, which Windows would otherwise take to the whole monitor.
    flags: root.captionMerged ? (Qt.Window | Qt.FramelessWindowHint) : Qt.Window
    title: Words.appName
    color: Theme.bgBase

    /// Whether this app draws the window's edge (`windowEdge`, 1px `borderDefault` inside the client area): a frameless
    /// window has no frame for the platform to draw one on, and no shadow — the edge does the shadow's work. Maximised,
    /// only the bottom side stays (`WindowBody`): elsewhere the window's edge is the screen's, and a line there parts
    /// nothing and spends a row of the work area.
    readonly property bool edgeDrawn: root.captionMerged && root.visibility !== Window.Maximized
    /// How far the body stands in from the left and right edges, so the edge runs beside the band's end cells (☰ / ✕)
    /// rather than over their outer column (user decision — ask before changing): over it, the wash's outer column is
    /// the edge's colour, the ✕'s red alone comes out a pixel narrower than minimise and maximise beside it, and the
    /// marks sit half a pixel off the middle of what shows. Sideways only: the band's top row stays under the top
    /// edge. None while maximised (no edge). The window's width floor takes in these two columns (`floorWidth`).
    readonly property int edgeInset: root.edgeDrawn ? Theme.borderWidth : 0
    // The grab runs are reported in scene coordinates, which the inset moves without moving them inside the band.
    onEdgeInsetChanged: chrome.reportCaptionStrip()

    // ---- the floor the window may not be dragged under --------------------
    // Past the minimums, `SplitView` and the layouts lay items out over their own edge (デザイン規約 §窓の床).
    // `minimumWidth` only covers a dragged edge and a rising floor — `QWindow::resize` hands a size straight to the
    // platform without reading the hints — so every size this application sets goes through `holdFloor`. Measured in
    // `WindowBody`; named here, where the harness (`platitude.auto`) and `WindowShape` read them as `window.<name>` —
    // handing them the new owner instead is not open, the harness has one door (`seats`). The width's floors take in
    // the edge's two columns (`edgeInset`).
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
    // out in every group. Kept in this file though it is the largest block left: rules-refs/app-ui.md names this block
    // as where that trap is shown.
    palette {
        window: Theme.bgBase
        base: Theme.bgBase
        button: Theme.bgElevated
        placeholderText: Theme.textMuted
        // The two roles Fusion paints a scroll bar's thumb with (`pressed ? dark : mid`); everywhere else the style
        // reaches for them (Popup and Dialog borders, the SplitView handle — replaced by `SplitHandleBar`) this app
        // draws its own background (デザイン規約 §スクロールバー). So the thumb's inks live here, in these two roles. A
        // ground colour in `dark` would paint the held thumb in the groove's ink: it vanishes in the hand, and only
        // over rows whose ground does not change (it reads as "a sideways bar vanishes when grabbed").
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
        // `highlight` / `highlightedText` are also every field's and text area's selection colours, in every group.
        // No button is filled (`highlighted`), so this group's two are the only ones that reach nothing.
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
        if (!root.active) {
            root.everAway = true
            // A press in another window is one this one never hears (`Hand.acted`).
            Hand.acted()
            // Whatever took the window away — that press, the keyboard, another program — the menus go with it, as a
            // platform menu does. Qt does not: 6.12's `QQuickPopup` / `QQuickOverlay` / `QQuickMenu` ignore it.
            OutsidePress.closeAll()
        } else if (root.everAway) {
            root.focusEpoch++
        }
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
        Hand.acted()
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
    // go of (`SettingsAppPane`'s `Restart to apply`). Read here because `root.close()` is the one road out — the close
    // gate that waits a running write out stands on it.
    Connections {
        target: AppBackend
        function onRestartWantedChanged() {
            if (AppBackend.restartWanted)
                root.close()
        }
    }

    // The RepoPage of the active tab — the only tab that has one (`RepoPageStack`). Held by `WindowBody`, named here
    // for the floors' reason: the harness reads `window.curPage`.
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
    // reopens inside the folder it last accepted, the repository itself, where there is nothing to open. Set once per
    // press: the dialog writes `currentFolder` itself as the reader walks. `nearUrl` overrides: a retry opens beside
    // the refused folder. Headless: `open-picker` / `open-picker worktree` (`--preset nested-worktree`).
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
            windowTip: windowTip,
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

    // The window's one tooltip, which every target asks for through `TipAsk`.
    TipSeat {
        id: windowTip
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
        const ask = windowTip.current
        if (ask === null || !windowTip.shownNow || ask.place === "" || ask.href === "")
            return false
        windowTip.linkAsked(ask.href)
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
        chrome.decorateWindow()
        // A window that is only here to say another process has the files does none of the rest: its store is empty
        // (`main`'s `claim_store`), and the saved shape is the real window's to wear.
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
    // name (`AutoShotDriver.gate`): a sibling of the body, so the shot takes whichever stands — else a run that ended
    // at the gate pictures the app that never came up. Headless: `solo` (`blocked=true`).
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

    // The window's edge (`edgeDrawn` says why), windowed only; its bottom side coincides with the floor line at the end
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
        // outside the content item never wakes the render loop, so every change waits for the next input (it reads as
        // a tab's first click not taking).
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
