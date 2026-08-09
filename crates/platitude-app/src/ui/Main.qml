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
    ///
    /// `PG_PLAIN_CHROME=1` asks for the other shape from here: the two
    /// platforms that keep it cannot be run on this machine, so without a
    /// way to see their layout it is only ever exercised by the people who
    /// cannot report back.
    readonly property bool captionMerged: Qt.platform.os === "windows"
                                          && !AppBackend.plainChrome

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
    // puts those back without the drawing coming with them — and takes
    // `WM_NCHITTEST` away from Qt while it is at it, because Qt 6.10's
    // own answer for this flag set synthesises input from a poll and
    // loses track of it (the story is on `winframe::take_frame_hit_test`;
    // the symptom was the first click landing dead and hover freezing).
    flags: root.captionMerged
           ? (Qt.Window | Qt.CustomizeWindowHint
              | Qt.ExpandedClientAreaHint | Qt.NoTitleBarBackgroundHint)
           : Qt.Window
    title: qsTr("Platitude GG")
    color: Theme.bgBase

    // ---- what a title bar does, now that this band is one ----------------
    /// Goes through `visibility`, which is also where the saved shape is
    /// read from, so a window left maximised comes back that way.
    function toggleMaximized() {
        root.visibility = root.visibility === Window.Maximized
                          ? Window.Windowed : Window.Maximized
    }
    function minimizeWindow() {
        root.visibility = Window.Minimized
    }
    /// Tells the hit test where the band's grab-run is, in scene
    /// coordinates — the one stretch it answers HTCAPTION for, which is
    /// what makes it drag, snap, maximise on a double-click and open the
    /// window menu, all as the platform's own gestures. Called from the
    /// strip's own layout changes and from the shifts the strip cannot
    /// see: the maximised inset, and the window resizing.
    function reportCaptionStrip() {
        if (!root.captionMerged || topBar.grabRunItem === null)
            return
        const run = topBar.grabRunItem
        const at = run.mapToItem(null, 0, 0)
        AppBackend.setCaptionStrip(at.x, at.x + run.width, at.y + run.height)
    }
    onWidthChanged: root.reportCaptionStrip()
    onMaximizedInsetChanged: root.reportCaptionStrip()

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
        // The hairline Windows draws around the window, and the strip of
        // frame just inside it. Left to the system both are light, and
        // with no title bar to explain them they read as stray white
        // edges around the band. The strip takes the band's own colour so
        // it disappears into it; the hairline stays a line.
        AppBackend.setWindowBorder(Theme.borderDefault, Theme.bgElevated)
        // The hit test just installed reads the strip from here on; hand
        // it the shape the band settled into while loading.
        root.reportCaptionStrip()
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
        onOpenRejected: (path, kind, message, near) =>
            openFailedDialog.show(path, kind, message, near)
    }

    // Only the picker's own answers come here, and all three of them: a
    // folder somebody just chose is still in the middle of choosing one,
    // so the way on is the picker again — including when the check could
    // not say what was wrong, where the folder is no more openable for
    // not knowing why. Every other way a repository fails to open (a
    // restored tab, a worktree row, PG_AUTO_OPEN) keeps its tab and its
    // page-sized failure screen — nobody is standing at the picker for
    // those, and a modal on startup is answered before it can be read.
    OpenFailedDialog {
        id: openFailedDialog
        // Taking it back means what it says: no tab was opened and none
        // was closed, so the window is exactly where it started — with
        // the empty page's own way in, if there was nothing else open.
        onChooseAnother: near => root.openRepositoryPicker(near)
    }

    // Smoke hooks (PG_AUTO_ACT=open-not-a-repo / open-bare and the two
    // ways back out). The picker is the platform's own window, so the
    // run enters where its answer lands — the path it accepted — and
    // that is also the one place a folder gets checked, so nothing is
    // proved by a shorter cut. The argument is the folder; xtask makes
    // one when the verb needs it.
    readonly property bool pickAct: AppBackend.autoAct === "open-not-a-repo"
                                    || AppBackend.autoAct === "open-bare"
                                    || AppBackend.autoAct === "open-not-a-repo-retry"
                                    || AppBackend.autoAct === "open-not-a-repo-cancel"
    Timer {
        interval: 1200
        running: root.pickAct
        onTriggered: {
            tabsModel.openPickedPath(AppBackend.autoActArg)
            pickAnswerTimer.start()
        }
    }
    // The answer is one `git rev-parse` away (実測 30–36ms on Windows,
    // repository or not), so this is a beat rather than a wait.
    Timer {
        id: pickAnswerTimer
        interval: 400
        onTriggered: {
            if (AppBackend.autoAct === "open-not-a-repo-retry")
                openFailedDialog.retry()
            else if (AppBackend.autoAct === "open-not-a-repo-cancel")
                openFailedDialog.close()
            else {
                root.reportPick()
                return
            }
            // Both ways out end the dialog, and a closing popup is still
            // `opened` for a frame or two — the answer this reads is
            // whether it went, so it is read after it has had the time.
            pickSettleTimer.start()
        }
    }
    Timer {
        id: pickSettleTimer
        interval: 300
        onTriggered: root.reportPick()
    }

    // Smoke hooks (PG_AUTO_ACT=open-fail-tab / -bare / -log): the other
    // road, the one that keeps its tab — a tab put back from the last
    // session, a worktree row, a path named on the command line. Nothing
    // checks the folder first there, so the page itself is what says so,
    // in the same words the dialog would have used (`kind=` reports
    // which). The `-log` half goes on to open the command log the way
    // the toolbar's `>_` does, which on this screen used to do nothing.
    Timer {
        interval: 1200
        running: AppBackend.autoAct === "open-fail-tab"
                 || AppBackend.autoAct === "open-fail-tab-bare"
                 || AppBackend.autoAct === "open-fail-tab-log"
        onTriggered: {
            tabsModel.openRepositoryPath(AppBackend.autoActArg)
            failTabTimer.start()
        }
    }
    Timer {
        id: failTabTimer
        interval: 900
        onTriggered: {
            if (AppBackend.autoAct === "open-fail-tab-log" && root.curPage !== null)
                root.curPage.toggleCommands()
            AppBackend.report(
                "open_fail_tab tabs=" + pageRepeater.count
                + " state=" + (root.curPage !== null ? root.curPage.pageTab.state : "-")
                + " kind=" + (root.curPage !== null ? root.curPage.pageTab.errorKind : "-")
                + " commands=" + (root.curPage !== null ? root.curPage.commandsShown : "-"))
        }
    }
    /// What the run has to show for itself. `dialog=` is the dialog's own
    /// `opened` (reporting what was asked of it would go on passing with
    /// the binding cut), and `tabs=` says the refused folder never became
    /// one — which is the whole of what this verb is about.
    function reportPick() {
        AppBackend.report("open_failed kind=" + openFailedDialog.kind
                          + " dialog=" + openFailedDialog.opened
                          + " tabs=" + pageRepeater.count
                          + " active=" + tabsModel.currentIndex
                          + " near=" + openFailedDialog.near)
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
        const wantWidth = root.insideScreen(AppBackend.startWindowWidth(),
                                            Screen.width)
        const wantHeight = root.insideScreen(AppBackend.startWindowHeight(),
                                             Screen.height)
        root.width = wantWidth
        root.height = wantHeight
        root.askedWidth = wantWidth
        root.askedHeight = wantHeight
        settleTimer.restart()
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

    /// A remembered length, kept inside the screen the window comes up on.
    ///
    /// A shape saved on a display that is not there any more comes back to
    /// a smaller one, and so does anything that once got written down too
    /// wide — neither should open a window whose corners nobody can reach.
    /// The screen is `Screen.width`, the one this window is on, and *not*
    /// `Screen.desktopAvailableWidth`, which is the whole virtual desktop
    /// (measured on a three-monitor machine: 5760, so nothing is ever
    /// wider than it).
    ///
    /// Only a window somebody is at is fitted to a screen. A run that is
    /// being driven takes the size it was configured with — nobody is
    /// looking at it, and the offscreen platform the headless runs use
    /// reports an 800x800 screen that would cut every screenshot to fit.
    function insideScreen(saved, screen) {
        return AppBackend.automated ? saved : Math.min(saved, screen)
    }

    /// What this window adds to a size on the way in.
    ///
    /// It does not read back the way it is written. Measured on the merged
    /// chrome: asked for 1200 it comes up 1200 wide (client 1200, frame
    /// 1216) and then calls itself 1206 — so writing down what it says
    /// grew the window 6px on every launch (measured: 1200 → 1206 → 1212
    /// → 1218 → 1224 over four). What the two sides disagree about is the
    /// frame margins, which Qt takes from one place when it sets the
    /// geometry and another when it reads it back, and this window has no
    /// ordinary frame for them to agree on.
    ///
    /// So the difference is read off the window itself — like
    /// `maximizedInset`, whatever this window turns out to add is what
    /// comes back off — and taken away again on the way out, which keeps
    /// the *frame* where it was: the file loses the 6px that the frame
    /// gains.
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
            // Only ever measured against a size this window was just
            // handed, and only while nothing else has had a chance to
            // resize it. A window that came up maximised is not measured
            // at all — it reports the screen — and nor is one that is put
            // down later, because what puts it down (a snap to half the
            // screen, say) is free to resize it on the way, and a
            // difference read off that is not a frame margin. That leaves
            // the size such a run ends at 6px wide in the file, once; the
            // next launch starts windowed, measures, and stops it there.
            if (root.askedWidth <= 0 || root.visibility !== Window.Windowed)
                return
            root.widthSlop = root.width - root.askedWidth
            root.heightSlop = root.height - root.askedHeight
        }
    }

    /// Everything the next launch should come back to. One place, because
    /// what is worth writing is the shape the window settled into, not
    /// every value it passed through on the way (実装計画 §7).
    function reportState() {
        // A minimised window has nothing to say about the shape it will
        // come back as, so it says nothing and the file keeps what the
        // window last looked like. Measured on Windows: while it is down
        // the window reports neither its windowed nor its maximised
        // numbers (one maximised on a 1920x1032 work area calls itself
        // 1926x1032 at -3,3) and its visibility is no longer Maximized —
        // so a report from here wrote a window wider than the screen into
        // the file and cleared the flag that would have brought the
        // maximised one back.
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

    // PG_AUTO_ACT=band: the shape the title-bar band settled into. The
    // numbers rather than a screenshot, because the headless platform
    // draws no window buttons of its own — a band that lost the grab run
    // or pushed its buttons off the end looks fine in the picture.
    Timer {
        id: bandActTimer
        interval: 1200
        onTriggered: AppBackend.report(
            "band merged=" + root.captionMerged
            + " plain=" + AppBackend.plainChrome
            + " grabRun=" + topBar.bandGrabRun
            + " buttonsX=" + topBar.bandButtonsX
            + " width=" + topBar.width
            + " tabsW=" + topBar.bandTabsWidth
            + " rightMargin=" + topBar.bandRightMargin)
    }

    // PG_AUTO_ACT=tab-widths: what the tabs made of the run they share.
    // Numbers again, and for the same reason as the band's — a strip that
    // narrowed the wrong tabs, or narrowed them all when only the long
    // ones had to give, comes out looking like a strip that got it right.
    // `widths=` is the answer: the tabs left alone are the ones still at
    // their own length, and the ones that gave way all read alike.
    Timer {
        id: tabWidthActTimer
        interval: 1200
        onTriggered: AppBackend.report(
            "tab_widths tabs=" + topBar.bandTabCount
            + " run=" + Math.round(topBar.bandTabRun)
            + " cap=" + Math.round(topBar.tabTitleCap)
            + " floor=" + topBar.tabTitleMinW
            + " max=" + topBar.tabTitleMaxW
            + " content=" + Math.round(topBar.bandTabContent)
            + " view=" + Math.round(topBar.bandTabsWidth)
            + " scrolls=" + topBar.bandTabScrolls
            + " widths=" + topBar.tabWidths())
    }

    // PG_AUTO_ACT=tab-mark: the `✕` is out on the tab in front and on the
    // tab under the hand. The argument is which tab the hand is on — one
    // that is not in front, or the run says nothing the picture of any
    // other verb does not already say.
    Timer {
        id: tabMarkActTimer
        interval: 1200
        onTriggered: {
            topBar.pointAtTab(Number(AppBackend.autoActArg || 1))
            AppBackend.report(
                "tab_marks tabs=" + topBar.bandTabCount
                + " current=" + tabsModel.currentIndex
                + " pointed=" + Number(AppBackend.autoActArg || 1)
                + " marks=" + topBar.tabMarks())
        }
    }

    // PG_AUTO_ACT=solo: the window a run that was turned away puts up.
    // The harness has to be part of this one — it holds the real lock on
    // the config directory before it starts this process, so the picture
    // is of the mechanism and not of a flag that imitates it.
    Timer {
        id: soloActTimer
        interval: 1200
        onTriggered: AppBackend.report(
            "solo blocked=" + AppBackend.alreadyRunning
            + " held=" + (AppBackend.heldElsewhere !== "")
            + " gate=" + gate.visible
            + " main=" + mainUi.visible)
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
            // A run that ends with the window down. Up first, because the
            // window whose numbers go wrong while it is minimised is the
            // one that was maximised.
            if (AppBackend.autoActArg === "minimize")
                root.visibility = Window.Maximized
            stateReportTimer.start()
        }
    }
    // The splitters have to have taken the new sizes before they can be
    // read back off the panes.
    Timer {
        id: stateReportTimer
        interval: 400
        onTriggered: {
            // Up, then down, with a report from each: what the file holds
            // once the window is down has to be what it held while it was
            // up. Without the first report there is nothing for the
            // second one to leave alone, and the verb passes either way.
            if (AppBackend.autoActArg === "minimize") {
                root.reportState()
                root.visibility = Window.Minimized
            }
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
                // What the report above left in the store, which is what
                // the next launch comes back to. Read back rather than
                // repeated from the window, so a run that had nothing to
                // say (minimised) is told apart from one that said this.
                + " windowW=" + AppBackend.startWindowWidth()
                + " windowH=" + AppBackend.startWindowHeight()
                + " windowMax=" + AppBackend.startWindowMaximized()
                + " autoFetch=" + AppBackend.autoFetchMinutes)
            // Back up for the shot: `grabToImage` has nothing to hand
            // back from a window that is down. Windowed rather than
            // maximised, so the picture is the size every other verb's
            // is — the offscreen platform maximises to its own 800x800
            // screen, and a shot that shape is a shot of the harness.
            if (AppBackend.autoActArg === "minimize")
                root.visibility = Window.Windowed
        }
    }

    // Closing is the last chance: the timer will not come round again.
    onClosing: {
        if (!AppBackend.alreadyRunning)
            root.reportState()
    }

    Component.onCompleted: {
        root.decorateWindow()
        // A window that is only here to say another process has the files
        // does none of the rest: no git to ask about, no tabs to open, no
        // shape to take back — the window whose files these are is
        // already wearing it.
        if (!AppBackend.alreadyRunning) {
            AppBackend.initialize()
            root.applySavedWindow()
            if (AppBackend.autoOpen !== "") {
                // Multiple repositories separated by ';' open as tabs in
                // order.
                const paths = AppBackend.autoOpen.split(";")
                for (let i = 0; i < paths.length; i++) {
                    if (paths[i] !== "")
                        tabsModel.openRepositoryPath(paths[i])
                }
            } else {
                tabsModel.restoreTabs()
            }
        }
        if (AppBackend.autoAct === "state")
            stateActTimer.start()
        if (AppBackend.autoAct === "band")
            bandActTimer.start()
        if (AppBackend.autoAct === "tab-widths")
            tabWidthActTimer.start()
        if (AppBackend.autoAct === "tab-mark")
            tabMarkActTimer.start()
        if (AppBackend.autoAct === "solo")
            soloActTimer.start()
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
            // Whatever the window is actually showing. The gate is a
            // sibling of `mainUi`, not a child, so a run that ends on it
            // used to photograph the application it never became.
            const shown = gate.visible ? gate : mainUi
            const ok = shown.grabToImage(function (res) {
                const saved = res.saveToFile(path)
                console.warn("screenshot saved=" + saved + " path=" + path)
                if (AppBackend.autoQuitMs <= 0)
                    Qt.quit()
            })
            if (!ok)
                console.warn("grabToImage returned false")
        }
    }

    // ---- the two ways the window has nothing to show ---------------------
    // git is missing or too old, or another process already has the files
    // this one would have used. Both are "this window is not going to be
    // an application", and both wear the same shape.
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
            // The way out. There is no title bar of its own to close this
            // window by when the band is the title bar, and the band is
            // inside the part that stays hidden.
            // Not `highlighted`: the accent is for the button somebody
            // came to press (規約 §アクセント), and nobody came here. The
            // empty page's lone button is the same shape.
            HoverButton {
                visible: AppBackend.alreadyRunning
                text: qsTr("Close")
                anchors.horizontalCenter: parent.horizontalCenter
                onClicked: root.close()
            }
            // The drawn ring, not Fusion's BusyIndicator — the same
            // turning mark as everywhere else in the window
            // (規約 §進行中・長押しの定数).
            NavIcon {
                visible: AppBackend.gitState === "checking" && !AppBackend.alreadyRunning
                width: Theme.iconLg
                height: Theme.iconLg
                kind: "spinner"
                tint: Theme.textSecondary
                anchors.horizontalCenter: parent.horizontalCenter
                // On the render thread, so it keeps turning while the GUI
                // thread drains models.
                RotationAnimator on rotation {
                    running: AppBackend.gitState === "checking"
                             && !AppBackend.alreadyRunning
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
        // height). The chrome reaches back up over that inset with a
        // negative top margin — the content item does not clip, so both
        // painting and input carry.
        //
        // Not by reparenting onto the window's root item, though that
        // also lands the band at 0: content outside the content item
        // never wakes the render loop, so every change waited for the
        // next input event to be painted — the whole window ran one
        // frame behind the model, which reads as "the first click did
        // nothing" (measured: notify delivered synchronously, strip
        // painted one state late, and a lone mouse-move caught it up).
        anchors.fill: parent
        // Everything the band carries stays inside the screen when the
        // window is maximised (see `maximizedInset`); the top edge folds
        // the safe-area climb and that inset into one number.
        anchors.margins: root.maximizedInset
        anchors.topMargin: root.maximizedInset - root.contentItem.y
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
            onMaximizeToggleRequested: root.toggleMaximized()
            onMinimizeRequested: root.minimizeWindow()
            onCloseRequested: root.close()
            onCaptionStripMoved: root.reportCaptionStrip()
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

        // Smoke hook (PG_AUTO_ACT=open-again): asking for a repository the
        // strip already holds. The argument is the path to ask for; with
        // none it is the first tab's own, spelled the way that tab was
        // opened — passing git's spelling of the same folder instead is
        // how the other half of the answer gets read. Enters through the
        // same slot every way in uses, and reports the strip afterwards:
        // no new tab may appear, and `active` has to have moved to the one
        // already holding it (デザイン規約 §タブの所作).
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
