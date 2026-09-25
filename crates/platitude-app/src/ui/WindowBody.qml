pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import platitude
import platitude.ui

// The window's body: the band, the pages, and the mark beside the pointer. `AutoShotDriver.grabApp` grabs this
// item, so anything drawn outside it is outside every screenshot. Where it sits in the window stays with `Main`.
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
    /// Handed on by the window (to `WindowChrome`, `SharedToolTip`, `Main.holdWaitHand` and the harness).
    readonly property alias topBar: topBar
    readonly property alias pageRepeater: pages.seats
    readonly property alias hand: hand
    readonly property alias waitRing: waitRingSeat

    // ---- the floor the window may not be dragged under --------------------
    // Measured here, where both halves (band and page) are; the window applies it (`Main.minimumWidth` /
    // `WindowShape.holdFloor`).
    /// The page the floor is read off: the blank page with no tab open (same panes, same minimums).
    readonly property var floorPage: mainUi.curPage !== null ? mainUi.curPage : blankPage.item
    readonly property real floorWidth:
        Math.max(topBar.floorWidth, mainUi.floorPage !== null ? mainUi.floorPage.floorWidth : 0)
    /// The floor as if the left list were open — where the band's actions finish giving up their words
    /// (`TopBar.actionCap`). Read off the real floor, folding the list would put the words back (規約 §窓の床).
    readonly property real openFloorWidth:
        Math.max(topBar.floorWidth, mainUi.floorPage !== null ? mainUi.floorPage.openFloorWidth : 0)
    readonly property real floorHeight:
        // `bodyColumn`'s rows other than the page: the chrome's two rows and the bottom edge's line.
        Theme.toolbarHeight + Theme.opsBarHeight + Theme.borderWidth
        + (mainUi.floorPage !== null ? mainUi.floorPage.floorHeight : 0)

    // Where the pointer is. On the parent of all content: the one place a window-wide handler leaves the rows'
    // hover alone (`PointerWatch`).
    PointerWatch {
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
            // Read here: the floor is the larger of band's and page's (`TopBar.windowAtFloor`).
            windowAtFloor: mainUi.window.width <= Math.ceil(mainUi.floorWidth)
            onOpenRepositoryRequested: mainUi.window.openRepositoryPicker()
            onCloneRepositoryRequested: mainUi.window.startClone()
            onIdentityEditRequested: mainUi.dialogSeat.openSettingsAt("git")
            onSettingsRequested: mainUi.dialogSeat.openSettingsAt("app")
            onMaximizeToggleRequested: mainUi.chrome.toggleMaximized()
            onMinimizeRequested: mainUi.chrome.minimizeWindow()
            onCloseRequested: mainUi.window.close()
            onCaptionStripMoved: mainUi.chrome.reportCaptionStrip()
        }
        // No divider under the chrome (デザイン規約 §ウィンドウの縁).

        // Nothing open: the blank page.
        Loader {
            id: blankPage
            Layout.fillWidth: true
            Layout.fillHeight: true
            active: mainUi.tabsModel.currentIndex < 0
            visible: active
            sourceComponent: Component {
                RepoPage {
                    index: -1
                    tab_id: -1
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

        // The window's bottom edge, drawn maximised too: this side still has the taskbar under it.
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.borderWidth
            color: Theme.borderDefault
        }
    }
}
