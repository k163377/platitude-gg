pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import platitude
import platitude.ui

// The window's body, and the whole of what a headless picture is of (`AutoShotDriver.grabApp` grabs this item, so
// anything drawn outside it is drawn outside every screenshot). Almost all of it is the column below; the rest
// is the one mark that answers the pointer.
//
// Where this item sits inside the window, and whether there is a body at all, stay at the seat (`Main`): the inset
// it reaches back over is the window's to know.
Item {
    id: mainUi

    /// The window this body fills. The rows below reach back through it for the shape it is in and for the doors
    /// only the window has (`Main`).
    required property var window
    /// The two seats beside this one the band and the pages send their presses to (`WindowChrome` /
    /// `WindowDialogSeat`), and the strip of tabs the column below is a view of.
    required property var chrome
    required property var dialogSeat
    required property TabsModel tabsModel

    /// The RepoPage of the active tab (the toolbar's right-side controls act on it). Only that tab has one
    /// (`RepoPageStack`). The window carries the same name for everybody else (`Main.curPage`).
    readonly property var curPage: pages.curPage
    /// What the window hands on from here: the band (`WindowChrome` and the harness), the tabs' pages (the harness),
    /// where the pointer is (`SharedToolTip`) and the mark that stands beside it (`Main.holdWaitHand`).
    readonly property alias topBar: topBar
    readonly property alias pageRepeater: pages.seats
    readonly property alias hand: hand
    readonly property alias waitRing: waitRingSeat

    // ---- the floor the window may not be dragged under --------------------
    // Both halves of it are below — the band's and the page's — so the three numbers are measured here. What the
    // window does with them is the window's (`Main.minimumWidth` / `WindowShape.holdFloor`).
    /// The page the floor is read off. `curPage` is null with no tab open, while the window is showing the
    /// blank page, which has the same three panes with the same minimums.
    readonly property var floorPage: mainUi.curPage !== null ? mainUi.curPage : blankPage.item
    readonly property real floorWidth:
        Math.max(topBar.floorWidth, mainUi.floorPage !== null ? mainUi.floorPage.floorWidth : 0)
    /// The same floor with the left list open whether or not it is — the width the band's three actions have finished
    /// giving their words up at (`TopBar.actionCap`). Folding the list lowers the real floor, and a schedule read off
    /// that would put the words back as the rail took the list's place (規約 §窓の床).
    readonly property real openFloorWidth:
        Math.max(topBar.floorWidth, mainUi.floorPage !== null ? mainUi.floorPage.openFloorWidth : 0)
    readonly property real floorHeight:
        // The band, the divider under it, and the line the window's bottom edge is drawn as — the three rows of
        // `bodyColumn` that are not the page (they carry their own heights; the page's is its own floor).
        Theme.toolbarHeight + Theme.splitterWidth + Theme.borderWidth
        + (mainUi.floorPage !== null ? mainUi.floorPage.floorHeight : 0)

    // Where the hand is, for whoever has to open something beside it. Declared here, on the parent of the whole
    // content, because that is the one place a window-wide handler costs the rows nothing (`PointerWatch`).
    PointerWatch {
        id: hand
    }

    // ---- the wait the hand is given ----------------------------------
    // The mark that stands beside the pointer while a write replays history (`WindowWaitRing`). Over everything
    // and belonging to no pane, so its z and its fill are written here.
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
            // Standing on the floor is the one width with nothing left to share out, and the band's state
            // group gives up its words there (`TopBar.windowAtFloor`). Read here: the floor is the larger of
            // band's and page's.
            windowAtFloor: mainUi.window.width <= Math.ceil(mainUi.floorWidth)
            // …and the width the three actions have to be down to their marks by, which is that floor with the list
            // open. Read here for the same reason: only this body has both halves of it.
            windowFloorWidth: mainUi.openFloorWidth
            onOpenRepositoryRequested: mainUi.window.openRepositoryPicker()
            onCloneRepositoryRequested: mainUi.window.startClone()
            onIdentityEditRequested: mainUi.dialogSeat.openSettingsAt("git")
            onSettingsRequested: mainUi.dialogSeat.openSettingsAt("app")
            onMaximizeToggleRequested: mainUi.chrome.toggleMaximized()
            onMinimizeRequested: mainUi.chrome.minimizeWindow()
            onCloseRequested: mainUi.window.close()
            onCaptionStripMoved: mainUi.chrome.reportCaptionStrip()
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
            // The same card, told whom it was opened on before it opens (デザイン規約 §アバターを与える).
            onAvatarSettingsRequested: (name, email) => mainUi.dialogSeat.openAvatarSettings(name, email)
        }

        // The window's floor, and — while the edge above is drawn — its bottom side as well: one line in
        // borderDefault doing both. Drawn while the window fills the screen too: this side still has the
        // taskbar under it.
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.borderWidth
            color: Theme.borderDefault
        }
    }
}
