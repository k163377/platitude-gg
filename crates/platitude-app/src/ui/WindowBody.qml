pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import platitude
import platitude.ui

// The window's body: the band, the pages, and the mark beside the pointer. `AutoShotDriver.app` grabs this
// item's subtree, so anything drawn outside it — a surface put straight under the `ApplicationWindow` — is outside
// every screenshot, and the report line still goes green. Whatever fills the window goes in here, beside
// `bodyColumn` rather than in it: every child of a Layout is laid out. The one exception is the window's edge
// (`Main.windowEdge`), which `WindowShotMirrors` lays between this body and the overlay in scene.png alone; app.png
// is this body only, so on Windows it is 2px narrower than the window (1440 → 1438). Where it sits in the window
// stays with `Main`.
Item {
    id: mainUi

    /// The window this body fills, for its shape and the doors only it has (`Main`).
    required property var window
    /// Where the band's and the pages' presses go (`WindowChrome` / `WindowDialogSeat`), and the tabs shown below.
    required property var chrome
    required property var dialogSeat
    required property TabsModel tabsModel

    /// The active tab's RepoPage — the only tab with one (`RepoPageStack`). Everybody else reads `Main.curPage`.
    readonly property var curPage: pages.curPage
    /// Handed on by the window (to `WindowChrome`, `TipSeat`, `Main.holdWaitHand` and the harness).
    readonly property alias topBar: topBar
    readonly property alias pageRepeater: pages.seats
    readonly property alias hand: hand
    readonly property alias waitRing: waitRingSeat

    /// The strip has been given what it starts with — the saved tabs, or a run's own (`Main`). Before that, no tab
    /// in front is not yet "nothing open".
    property bool tabsStarted: false
    /// The strip has started and waits on no folder (`TabsModel.opening`).
    readonly property bool stripAtRest: mainUi.tabsStarted && !mainUi.tabsModel.opening
    /// Whether the blank page may stand: from the first moment the strip is at rest. Before it, a blank page would be
    /// built whole only to be taken down a turn later, under the page of the tab that was on its way — a start that
    /// opens a tab would pay for two pages and keep the first one's memory as the process's high-water mark.
    /// Latched: a folder asked for later (the picker, with no tab open) leaves the blank page standing while git
    /// places it.
    property bool blankDue: false

    // ---- the floor the window may not be dragged under --------------------
    // Measured here, where both halves (band and page) are; the window adds its edge and applies it
    // (`Main.floorWidth` / `WindowShape.holdFloor`).
    /// The page the floor is read off: the blank page with no tab open (same panes, same minimums). Never `curPage`
    /// alone: with no tab it is null, and only that window would shrink to the band's floor. Null while the strip is
    /// starting (`blankDue`) and between a tab switch's two steps (`RepoPageStack.curPage`); the floor rises with the
    /// page that arrives (`Main.onFloorWidthChanged`).
    /// Headless: `window-floor --restore` (`tabs=0` answers `pageW=821`; `floorW=` adds the edge, 823 on Windows).
    readonly property var floorPage: mainUi.curPage !== null ? mainUi.curPage : blankPage.item
    readonly property real floorWidth:
        Math.max(topBar.floorWidth, mainUi.floorPage !== null ? mainUi.floorPage.floorWidth : 0)
    /// The floor as if the left list were open, for the panel's runs to report (`openW=`).
    readonly property real openFloorWidth:
        Math.max(topBar.floorWidth, mainUi.floorPage !== null ? mainUi.floorPage.openFloorWidth : 0)
    readonly property real floorHeight:
        // `bodyColumn`'s rows other than the page: the chrome's two rows and the bottom edge's line.
        Theme.toolbarHeight + Theme.opsBarHeight + Theme.borderWidth
        + (mainUi.floorPage !== null ? mainUi.floorPage.floorHeight : 0)

    onStripAtRestChanged: if (mainUi.stripAtRest) mainUi.blankDue = true

    // Where the pointer is. On the parent of all content: the one place a window-wide handler leaves the rows'
    // hover alone (`HandWatch`) — a `HoverHandler` takes hover over its item's whole area but does not kill that item's
    // own children, where one on a sheet laid over the content would kill every row under it. Not a `PointHandler`: it
    // goes active only with a press, and a hovering pointer is never reported. Under a popup `known` falls, which is
    // the right answer there.
    HandWatch {
        id: hand
    }

    // The ring beside the pointer during a replay (`WindowWaitRing`); over every pane, so its z and fill are here.
    WindowWaitRing {
        id: waitRingSeat
        anchors.fill: parent
        z: 10001
        hand: hand
        page: mainUi.curPage
    }

    ColumnLayout {
        id: bodyColumn
        anchors.fill: parent
        spacing: 0

        TopBar {
            id: topBar
            Layout.fillWidth: true
            tabsModel: mainUi.tabsModel
            curPage: mainUi.curPage
            captionMerged: mainUi.window.captionMerged
            windowMaximized: mainUi.window.visibility === Window.Maximized
            // Read here: the floor is the larger of band's and page's (`TopBar.windowAtFloor`). The body's own width
            // against the body's own floor — the window's carries the edge beside them (`Main.edgeInset`).
            windowAtFloor: mainUi.width <= Math.ceil(mainUi.floorWidth)
            onOpenRepositoryRequested: mainUi.window.openRepositoryPicker()
            onCloneRepositoryRequested: mainUi.window.startClone()
            onIdentityEditRequested: mainUi.dialogSeat.openSettingsAt("git")
            onSettingsRequested: mainUi.dialogSeat.openSettingsAt("app")
            onMaximizeToggleRequested: mainUi.chrome.toggleMaximized()
            onMinimizeRequested: mainUi.chrome.minimizeWindow()
            onCloseRequested: mainUi.window.close()
            onCaptionStripMoved: mainUi.chrome.reportCaptionStrip()
        }
        // No divider under the chrome: the panel closes itself with its own hairline (`TopBar`'s header).

        // Nothing open: the blank page (`blankDue` says from when). The seat is in the column whenever no tab is in
        // front, built or not: with nothing here to take the height, the column would share it out between the band
        // and the foot line and stand the band down the window.
        Loader {
            id: blankPage
            Layout.fillWidth: true
            Layout.fillHeight: true
            active: mainUi.blankDue && mainUi.tabsModel.currentIndex < 0
            visible: mainUi.tabsModel.currentIndex < 0
            sourceComponent: Component {
                RepoPage {
                    index: -1
                    tab_id: undefined
                    onOpenRepositoryPicker: mainUi.window.openRepositoryPicker()
                }
            }
        }

        // Repository pages — one row per tab, and a page only for the tab in front (`RepoPageStack`).
        RepoPageStack {
            id: pages
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: mainUi.tabsModel.currentIndex >= 0
            tabsModel: mainUi.tabsModel
            pageBand: topBar
            focusEpoch: mainUi.window.focusEpoch
            onScreen: mainUi.window.onScreen
            onOpenRepositoryPicker: mainUi.window.openRepositoryPicker()
            onSettingsDialogRequested: mainUi.dialogSeat.openSettingsAt("app")
            onGitSettingsRequested: mainUi.dialogSeat.openSettingsAt("git")
            onAvatarSettingsRequested: (name, email) => mainUi.dialogSeat.openAvatarSettings(name, email)
        }

        // The window's bottom edge, drawn maximised too: this side still has the taskbar under it, and content touching
        // it looks as if the window ran on down (`Main.edgeDrawn` has the other sides). The other sides' ink and width,
        // so windowed the four come out one even 1px outline. One line: `splitterWidth`'s 4px is a split handle's,
        // there is nothing to split at the window's foot, and over the edge the bottom alone would read 4px thick.
        // Here, not on `Main.windowEdge`: this line stands on every OS (the floor counts it), the edge only where the
        // window is frameless, so the two conditions part.
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.borderWidth
            color: Theme.borderDefault
        }
    }
}
